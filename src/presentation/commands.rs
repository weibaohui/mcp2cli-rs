//! Command handlers

use crate::application::{
    dto::*,
    ports::{ConfigPort, MCPClientPort, OutputPort, ParamParserPort},
    use_cases::*,
};
use crate::domain::errors::MCPResult;
use std::sync::Arc;

/// Command executor
pub struct CommandExecutor<CP, CP2, OP, PP>
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

impl<CP, CP2, OP, PP> CommandExecutor<CP, CP2, OP, PP>
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

    /// Execute list servers command
    pub async fn list_servers(&self) -> MCPResult<()> {
        let use_case = ListServersUseCase::new(
            self.config_port.clone(),
            self.output_port.clone(),
        );
        use_case.execute(ListServersRequest::default()).await
    }

    /// Execute show server info command
    pub async fn show_server_info(&self, server: &str) -> MCPResult<()> {
        let use_case = GetServerInfoUseCase::new(
            self.config_port.clone(),
            self.client_port.clone(),
            self.output_port.clone(),
        );
        use_case
            .execute(GetServerInfoRequest {
                server_name: server.to_string(),
            })
            .await
    }

    /// Execute show tool info command
    pub async fn show_tool_info(&self, server: &str, tool: &str) -> MCPResult<()> {
        let use_case = GetToolInfoUseCase::new(
            self.config_port.clone(),
            self.client_port.clone(),
            self.output_port.clone(),
        );
        use_case
            .execute(GetToolInfoRequest {
                server_name: server.to_string(),
                tool_name: tool.to_string(),
            })
            .await
    }

    /// Execute call tool command
    pub async fn call_tool(
        &self,
        server: &str,
        tool: &str,
        args: Vec<String>,
        yaml_params: Option<String>,
        yaml_file: Option<String>,
        stream: bool,
    ) -> MCPResult<()> {
        // Parse parameters
        let params = if let Some(file) = yaml_file {
            self.param_parser.read_yaml_file(&file).await?
        } else if let Some(yaml) = yaml_params {
            self.param_parser.parse_yaml(&yaml)?
        } else if self.param_parser.has_piped_input() && args.is_empty() {
            self.param_parser.read_stdin_yaml()?
        } else {
            self.param_parser.parse_kv_args(&args)?
        };

        let use_case = CallToolUseCase::new(
            self.config_port.clone(),
            self.client_port.clone(),
            self.output_port.clone(),
            self.param_parser.clone(),
        );

        use_case
            .execute(CallToolRequest {
                server_name: server.to_string(),
                tool_name: tool.to_string(),
                params,
                stream,
            })
            .await
    }

    /// Execute find tool command
    pub async fn find_tool(&self, tool_name: &str) -> MCPResult<()> {
        let use_case = FindToolUseCase::new(
            self.config_port.clone(),
            self.client_port.clone(),
            self.output_port.clone(),
        );
        use_case
            .execute(FindToolRequest {
                tool_name: tool_name.to_string(),
            })
            .await
    }

    /// Execute search tools command
    pub async fn search_tools(&self, query: &str) -> MCPResult<()> {
        let use_case = SearchToolsUseCase::new(
            self.config_port.clone(),
            self.client_port.clone(),
            self.output_port.clone(),
        );
        use_case
            .execute(SearchToolsRequest {
                query: query.to_string(),
            })
            .await
    }
}

/// Handle errors and output them
pub fn handle_error<OP: OutputPort>(output: &OP, err: &crate::domain::errors::MCPError) {
    let details = err.details.clone().map(serde_json::Value::Object);
    output.output_error(&err.code.to_string(), &err.message, details);
}
