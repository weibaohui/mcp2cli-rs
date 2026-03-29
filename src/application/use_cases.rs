//! Use cases - Application-specific business rules

use super::{
    dto::*,
    ports::{ConfigPort, MCPClientPort, OutputPort, ParamParserPort},
};
use crate::domain::{
    errors::{MCPError, MCPResult},
    services::{ParamParser, SchemaFormatter, ToolMatcher},
    value_objects::{ServerInfo, SuggestMatch, ToolInfo, ToolMatch},
};
use chrono::Utc;
use std::sync::Arc;

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// List servers use case
pub struct ListServersUseCase<CP, OP>
where
    CP: ConfigPort,
    OP: OutputPort,
{
    config_port: Arc<CP>,
    output_port: Arc<OP>,
}

impl<CP, OP> ListServersUseCase<CP, OP>
where
    CP: ConfigPort,
    OP: OutputPort,
{
    pub fn new(config_port: Arc<CP>, output_port: Arc<OP>) -> Self {
        Self {
            config_port,
            output_port,
        }
    }

    pub async fn execute(&self, req: ListServersRequest) -> MCPResult<()> {
        let (config, loaded_paths) = self.config_port.load_config().await?;

        let servers: Vec<ServerDto> = config
            .mcp_servers
            .iter()
            .map(|(name, cfg)| {
                let transport = cfg.get_transport_type().to_string();
                ServerDto {
                    name: name.clone(),
                    transport,
                    url: cfg.url.clone(),
                    command: cfg.build_command_string(),
                    tools: None,
                    error: None,
                }
            })
            .collect();

        let response = ServerListResponse {
            servers,
            config_files: loaded_paths,
        };

        self.output_success(&response);
        Ok(())
    }

    fn output_success<T: serde::Serialize>(&self, data: &T) {
        let response = SuccessResponse {
            success: true,
            data,
            meta: ResponseMeta {
                timestamp: Utc::now().to_rfc3339(),
                version: VERSION.to_string(),
            },
        };
        self.output_port.output_json(&response);
    }
}

/// Get server info use case
pub struct GetServerInfoUseCase<CP, CP2, OP>
where
    CP: ConfigPort,
    CP2: MCPClientPort,
    OP: OutputPort,
{
    config_port: Arc<CP>,
    client_port: Arc<tokio::sync::Mutex<CP2>>,
    output_port: Arc<OP>,
}

impl<CP, CP2, OP> GetServerInfoUseCase<CP, CP2, OP>
where
    CP: ConfigPort,
    CP2: MCPClientPort,
    OP: OutputPort,
{
    pub fn new(
        config_port: Arc<CP>,
        client_port: Arc<tokio::sync::Mutex<CP2>>,
        output_port: Arc<OP>,
    ) -> Self {
        Self {
            config_port,
            client_port,
            output_port,
        }
    }

    pub async fn execute(&self, req: GetServerInfoRequest) -> MCPResult<()> {
        let (config, loaded_paths) = self.config_port.load_config().await?;

        let server_config = config
            .mcp_servers
            .get(&req.server_name)
            .ok_or_else(|| MCPError::server_not_found(&req.server_name))?;

        // Connect and get tools
        let mut client = self.client_port.lock().await;
        client.connect(&req.server_name, server_config).await?;

        let tools_result = client.list_tools().await?;
        let tools: Vec<String> = tools_result.tools.iter().map(|t| t.name.clone()).collect();

        let transport = server_config.get_transport_type().to_string();
        let server_dto = ServerDto {
            name: req.server_name.clone(),
            transport,
            url: server_config.url.clone(),
            command: server_config.build_command_string(),
            tools: Some(tools),
            error: None,
        };

        let response = serde_json::json!({
            "configFiles": loaded_paths,
            "server": server_dto,
        });

        let wrapped = SuccessResponse {
            success: true,
            data: response,
            meta: ResponseMeta {
                timestamp: Utc::now().to_rfc3339(),
                version: VERSION.to_string(),
            },
        };

        self.output_port.output_json(&wrapped);
        Ok(())
    }
}

/// Get tool info use case
pub struct GetToolInfoUseCase<CP, CP2, OP>
where
    CP: ConfigPort,
    CP2: MCPClientPort,
    OP: OutputPort,
{
    config_port: Arc<CP>,
    client_port: Arc<tokio::sync::Mutex<CP2>>,
    output_port: Arc<OP>,
}

impl<CP, CP2, OP> GetToolInfoUseCase<CP, CP2, OP>
where
    CP: ConfigPort,
    CP2: MCPClientPort,
    OP: OutputPort,
{
    pub fn new(
        config_port: Arc<CP>,
        client_port: Arc<tokio::sync::Mutex<CP2>>,
        output_port: Arc<OP>,
    ) -> Self {
        Self {
            config_port,
            client_port,
            output_port,
        }
    }

    pub async fn execute(&self, req: GetToolInfoRequest) -> MCPResult<()> {
        let (config, loaded_paths) = self.config_port.load_config().await?;

        let server_config = config
            .mcp_servers
            .get(&req.server_name)
            .ok_or_else(|| MCPError::server_not_found(&req.server_name))?;

        let mut client = self.client_port.lock().await;
        client.connect(&req.server_name, server_config).await?;

        let tools = client.list_tools().await?;

        let tool = tools
            .tools
            .iter()
            .find(|t| t.name == req.tool_name)
            .ok_or_else(|| MCPError::method_not_found(&req.tool_name, &req.server_name))?;

        // Format tool information
        let formatted_params = tool
            .input_schema
            .as_ref()
            .and_then(SchemaFormatter::format_input_schema);
        let required_params = tool
            .input_schema
            .as_ref()
            .map(SchemaFormatter::get_required_params)
            .unwrap_or_default();
        let param_info_list = tool
            .input_schema
            .as_ref()
            .map(SchemaFormatter::get_param_info_list)
            .unwrap_or_default();

        let mut tool_data = serde_json::Map::new();
        tool_data.insert("name".to_string(), tool.name.clone().into());
        if let Some(desc) = &tool.description {
            tool_data.insert("description".to_string(), desc.clone().into());
        }

        if !required_params.is_empty() {
            tool_data.insert(
                "required".to_string(),
                required_params.join(" OR ").into(),
            );
        }

        if let Some(formatted) = formatted_params {
            tool_data.insert(
                "param_format".to_string(),
                "key:type=value (type: string/number/bool)".into(),
            );
            tool_data.insert("param_example".to_string(), formatted.into());
            tool_data.insert(
                "call_example".to_string(),
                format!(
                    "rmcp {} {} {}",
                    req.server_name,
                    tool.name,
                    SchemaFormatter::build_call_example(&param_info_list)
                )
                .into(),
            );
        } else if let Some(schema) = &tool.input_schema {
            tool_data.insert("inputSchema".to_string(), schema.clone());
        }

        let response = serde_json::json!({
            "configFiles": loaded_paths,
            "server": req.server_name,
            "tool": tool_data,
        });

        let wrapped = SuccessResponse {
            success: true,
            data: response,
            meta: ResponseMeta {
                timestamp: Utc::now().to_rfc3339(),
                version: VERSION.to_string(),
            },
        };

        self.output_port.output_json(&wrapped);
        Ok(())
    }
}

/// Call tool use case
pub struct CallToolUseCase<CP, CP2, OP, PP>
where
    CP: ConfigPort,
    CP2: MCPClientPort,
    OP: OutputPort,
    PP: ParamParserPort,
{
    config_port: Arc<CP>,
    client_port: Arc<tokio::sync::Mutex<CP2>>,
    output_port: Arc<OP>,
    param_parser: Arc<PP>,
}

impl<CP, CP2, OP, PP> CallToolUseCase<CP, CP2, OP, PP>
where
    CP: ConfigPort,
    CP2: MCPClientPort,
    OP: OutputPort,
    PP: ParamParserPort,
{
    pub fn new(
        config_port: Arc<CP>,
        client_port: Arc<tokio::sync::Mutex<CP2>>,
        output_port: Arc<OP>,
        param_parser: Arc<PP>,
    ) -> Self {
        Self {
            config_port,
            client_port,
            output_port,
            param_parser,
        }
    }

    pub async fn execute(&self, req: CallToolRequest) -> MCPResult<()> {
        let (config, _loaded_paths) = self.config_port.load_config().await?;

        let server_config = config
            .mcp_servers
            .get(&req.server_name)
            .ok_or_else(|| MCPError::server_not_found(&req.server_name))?;

        // Connect with retry logic
        let result = self
            .call_with_retry(&req.server_name, server_config, &req.tool_name, &req.params)
            .await;

        match result {
            Ok(call_result) => {
                // Format result
                let output = self.format_call_result(&call_result);

                let response = ToolCallResponse {
                    server: req.server_name.clone(),
                    method: req.tool_name.clone(),
                    result: output,
                };

                let wrapped = SuccessResponse {
                    success: true,
                    data: response,
                    meta: ResponseMeta {
                        timestamp: Utc::now().to_rfc3339(),
                        version: VERSION.to_string(),
                    },
                };

                if req.stream {
                    // Output streaming content
                    for text in call_result.extract_text() {
                        self.output_port.output_stream(&text);
                    }
                    self.output_port.flush();
                } else {
                    self.output_port.output_json(&wrapped);
                }

                Ok(())
            }
            Err(e) => {
                self.output_port.output_error(
                    &e.code.to_string(),
                    &e.message,
                    e.details.clone().map(serde_json::Value::Object),
                );
                Err(e)
            }
        }
    }

    async fn call_with_retry(
        &self,
        server_name: &str,
        server_config: &crate::domain::entities::ServerConfig,
        tool_name: &str,
        params: &std::collections::HashMap<String, serde_json::Value>,
    ) -> MCPResult<crate::domain::entities::CallToolResult> {
        let max_retries = 3;
        let mut last_error = None;

        for attempt in 1..=max_retries {
            let mut client = self.client_port.lock().await;

            if let Err(e) = client.connect(server_name, server_config).await {
                if !e.is_retryable() || attempt == max_retries {
                    return Err(e);
                }
                last_error = Some(e);
                drop(client); // Release lock before sleep
                tokio::time::sleep(tokio::time::Duration::from_millis(500 * attempt as u64))
                    .await;
                continue;
            }

            match client.call_tool(tool_name, params.clone()).await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    if !e.is_retryable() || attempt == max_retries {
                        return Err(e);
                    }
                    last_error = Some(e);
                    drop(client);
                    tokio::time::sleep(tokio::time::Duration::from_millis(500 * attempt as u64))
                        .await;
                }
            }
        }

        Err(last_error.unwrap_or_else(|| {
            MCPError::new(
                crate::domain::errors::ErrorCode::InternalError,
                "Retry loop exited without result",
            )
        }))
    }

    fn format_call_result(
        &self,
        result: &crate::domain::entities::CallToolResult,
    ) -> serde_json::Value {
        // Extract text content as structured data
        let content_data: Vec<serde_json::Map<String, serde_json::Value>> = result
            .content
            .iter()
            .filter_map(|item| match item {
                crate::domain::entities::ToolContent::Text { text } => {
                    let mut map = serde_json::Map::new();
                    map.insert("type".to_string(), "text".into());
                    map.insert("text".to_string(), text.clone().into());
                    Some(map)
                }
                _ => None,
            })
            .collect();

        if !content_data.is_empty() {
            serde_json::Value::Array(
                content_data.into_iter().map(serde_json::Value::Object).collect(),
            )
        } else {
            serde_json::to_value(result).unwrap_or_default()
        }
    }
}

/// Find tool use case
pub struct FindToolUseCase<CP, CP2, OP>
where
    CP: ConfigPort,
    CP2: MCPClientPort,
    OP: OutputPort,
{
    config_port: Arc<CP>,
    client_port: Arc<tokio::sync::Mutex<CP2>>,
    output_port: Arc<OP>,
}

impl<CP, CP2, OP> FindToolUseCase<CP, CP2, OP>
where
    CP: ConfigPort,
    CP2: MCPClientPort,
    OP: OutputPort,
{
    pub fn new(
        config_port: Arc<CP>,
        client_port: Arc<tokio::sync::Mutex<CP2>>,
        output_port: Arc<OP>,
    ) -> Self {
        Self {
            config_port,
            client_port,
            output_port,
        }
    }

    pub async fn execute(&self, req: FindToolRequest) -> MCPResult<()> {
        let (config, _loaded_paths) = self.config_port.load_config().await?;

        let mut matches = Vec::new();

        for (server_name, server_config) in &config.mcp_servers {
            let mut client = self.client_port.lock().await;

            if let Err(_) = client.connect(server_name, server_config).await {
                continue;
            }

            if let Ok(tools) = client.list_tools().await {
                for tool in tools.tools {
                    if tool.name == req.tool_name {
                        matches.push(ToolMatchDto {
                            server: server_name.clone(),
                            tool: ToolSummaryDto {
                                name: tool.name,
                                description: tool.description,
                            },
                        });
                    }
                }
            }
        }

        let response = ToolSearchResponse { matches };

        let wrapped = SuccessResponse {
            success: true,
            data: response,
            meta: ResponseMeta {
                timestamp: Utc::now().to_rfc3339(),
                version: VERSION.to_string(),
            },
        };

        self.output_port.output_json(&wrapped);
        Ok(())
    }
}

/// Search tools use case
pub struct SearchToolsUseCase<CP, CP2, OP>
where
    CP: ConfigPort,
    CP2: MCPClientPort,
    OP: OutputPort,
{
    config_port: Arc<CP>,
    client_port: Arc<tokio::sync::Mutex<CP2>>,
    output_port: Arc<OP>,
}

impl<CP, CP2, OP> SearchToolsUseCase<CP, CP2, OP>
where
    CP: ConfigPort,
    CP2: MCPClientPort,
    OP: OutputPort,
{
    pub fn new(
        config_port: Arc<CP>,
        client_port: Arc<tokio::sync::Mutex<CP2>>,
        output_port: Arc<OP>,
    ) -> Self {
        Self {
            config_port,
            client_port,
            output_port,
        }
    }

    pub async fn execute(&self, req: SearchToolsRequest) -> MCPResult<()> {
        let (config, _loaded_paths) = self.config_port.load_config().await?;

        let query_lower = req.query.to_lowercase();
        let mut matches = Vec::new();

        for (server_name, server_config) in &config.mcp_servers {
            let mut client = self.client_port.lock().await;

            if let Err(_) = client.connect(server_name, server_config).await {
                continue;
            }

            if let Ok(tools) = client.list_tools().await {
                for tool in tools.tools {
                    let name_match = tool.name.to_lowercase().contains(&query_lower);
                    let desc_match = tool
                        .description
                        .as_ref()
                        .map(|d| d.to_lowercase().contains(&query_lower))
                        .unwrap_or(false);

                    if name_match || desc_match {
                        matches.push(ToolMatchDto {
                            server: server_name.clone(),
                            tool: ToolSummaryDto {
                                name: tool.name,
                                description: tool.description,
                            },
                        });
                    }
                }
            }
        }

        let response = ToolSearchResponse { matches };

        let wrapped = SuccessResponse {
            success: true,
            data: response,
            meta: ResponseMeta {
                timestamp: Utc::now().to_rfc3339(),
                version: VERSION.to_string(),
            },
        };

        self.output_port.output_json(&wrapped);
        Ok(())
    }
}
