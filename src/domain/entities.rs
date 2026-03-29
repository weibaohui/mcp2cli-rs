//! Domain entities - Core business objects with identity

use super::value_objects::{AuthConfig, TransportType};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Server configuration entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transport: Option<String>,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub type_: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth: Option<AuthConfig>,
}

impl ServerConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_url(mut self, url: impl Into<String>) -> Self {
        self.url = Some(url.into());
        self
    }

    pub fn with_command(mut self, command: impl Into<String>) -> Self {
        self.command = Some(command.into());
        self
    }

    pub fn with_args(mut self, args: Vec<String>) -> Self {
        self.args = Some(args);
        self
    }

    pub fn with_transport(mut self, transport: impl Into<String>) -> Self {
        self.transport = Some(transport.into());
        self
    }

    pub fn with_timeout(mut self, timeout: u64) -> Self {
        self.timeout = Some(timeout);
        self
    }

    pub fn with_headers(mut self, headers: HashMap<String, String>) -> Self {
        self.headers = Some(headers);
        self
    }

    pub fn with_auth(mut self, auth: AuthConfig) -> Self {
        self.auth = Some(auth);
        self
    }

    /// Get the inferred transport type
    pub fn get_transport_type(&self) -> TransportType {
        TransportType::infer(
            self.transport.as_deref().or(self.type_.as_deref()),
            self.url.as_deref(),
            self.command.as_deref(),
        )
    }

    /// Get timeout in milliseconds (default: 30000)
    pub fn get_timeout_ms(&self) -> u64 {
        self.timeout.unwrap_or(30000)
    }

    /// Build command string for display
    pub fn build_command_string(&self) -> Option<String> {
        self.command.as_ref().map(|cmd| {
            if let Some(args) = &self.args {
                format!("{} {}", cmd, args.join(" "))
            } else {
                cmd.clone()
            }
        })
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            transport: None,
            type_: None,
            url: None,
            command: None,
            args: None,
            env: None,
            timeout: None,
            headers: None,
            auth: None,
        }
    }
}

/// MCP configuration containing all servers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPConfig {
    #[serde(rename = "mcpServers")]
    pub mcp_servers: HashMap<String, ServerConfig>,
}

impl MCPConfig {
    pub fn new() -> Self {
        Self {
            mcp_servers: HashMap::new(),
        }
    }

    pub fn with_server(mut self, name: impl Into<String>, config: ServerConfig) -> Self {
        self.mcp_servers.insert(name.into(), config);
        self
    }

    pub fn get_server(&self, name: &str) -> Option<&ServerConfig> {
        self.mcp_servers.get(name)
    }

    pub fn merge(&mut self, other: MCPConfig) {
        for (name, config) in other.mcp_servers {
            self.mcp_servers.insert(name, config);
        }
    }
}

impl Default for MCPConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// MCP Tool entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(rename = "inputSchema", skip_serializing_if = "Option::is_none")]
    pub input_schema: Option<serde_json::Value>,
}

impl Tool {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: None,
            input_schema: None,
        }
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn with_input_schema(mut self, schema: serde_json::Value) -> Self {
        self.input_schema = Some(schema);
        self
    }
}

/// MCP Call Tool Result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallToolResult {
    pub content: Vec<ToolContent>,
    #[serde(rename = "isError", skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
}

impl CallToolResult {
    pub fn new() -> Self {
        Self {
            content: Vec::new(),
            is_error: None,
        }
    }

    pub fn with_content(mut self, content: ToolContent) -> Self {
        self.content.push(content);
        self
    }

    /// Extract text content from the result
    pub fn extract_text(&self) -> Vec<String> {
        self.content
            .iter()
            .filter_map(|c| match c {
                ToolContent::Text { text } => Some(text.clone()),
                _ => None,
            })
            .collect()
    }
}

impl Default for CallToolResult {
    fn default() -> Self {
        Self::new()
    }
}

/// Tool content variants
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ToolContent {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image")]
    Image { data: String, mime_type: String },
    #[serde(rename = "resource")]
    Resource { resource: EmbeddedResource },
}

/// Embedded resource
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddedResource {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uri: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blob: Option<String>,
}

/// List tools result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListToolsResult {
    pub tools: Vec<Tool>,
    #[serde(rename = "nextCursor", skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

/// JSON-RPC request for MCP protocol
#[derive(Debug, Clone, Serialize)]
pub struct JsonRpcRequest<T> {
    pub jsonrpc: String,
    pub id: u64,
    pub method: String,
    pub params: T,
}

impl<T: Serialize> JsonRpcRequest<T> {
    pub fn new(id: u64, method: impl Into<String>, params: T) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            method: method.into(),
            params,
        }
    }
}

/// JSON-RPC response for MCP protocol
#[derive(Debug, Clone, Deserialize)]
pub struct JsonRpcResponse<T> {
    pub jsonrpc: String,
    pub id: Option<u64>,
    #[serde(flatten)]
    pub result: JsonRpcResult<T>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum JsonRpcResult<T> {
    Result(T),
    Error(JsonRpcError),
}

/// JSON-RPC error
#[derive(Debug, Clone, Deserialize)]
pub struct JsonRpcError {
    pub code: i64,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

/// Call tool parameters
#[derive(Debug, Clone, Serialize)]
pub struct CallToolParams {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<serde_json::Value>,
}

impl CallToolParams {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            arguments: None,
        }
    }

    pub fn with_arguments(mut self, args: serde_json::Value) -> Self {
        self.arguments = Some(args);
        self
    }
}

/// List tools parameters
#[derive(Debug, Clone, Default, Serialize)]
pub struct ListToolsParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_config_builder() {
        let config = ServerConfig::new()
            .with_url("http://example.com")
            .with_timeout(5000);

        assert_eq!(config.url, Some("http://example.com".to_string()));
        assert_eq!(config.timeout, Some(5000));
        assert_eq!(config.get_timeout_ms(), 5000);
    }

    #[test]
    fn test_server_config_default_timeout() {
        let config = ServerConfig::new();
        assert_eq!(config.get_timeout_ms(), 30000);
    }

    #[test]
    fn test_mcp_config_merge() {
        let mut config1 = MCPConfig::new().with_server(
            "server1",
            ServerConfig::new().with_url("http://server1.com"),
        );
        let config2 = MCPConfig::new().with_server(
            "server2",
            ServerConfig::new().with_url("http://server2.com"),
        );

        config1.merge(config2);

        assert!(config1.mcp_servers.contains_key("server1"));
        assert!(config1.mcp_servers.contains_key("server2"));
    }

    #[test]
    fn test_call_tool_result_extract_text() {
        let result = CallToolResult::new()
            .with_content(ToolContent::Text {
                text: "Hello".to_string(),
            })
            .with_content(ToolContent::Text {
                text: "World".to_string(),
            });

        let texts = result.extract_text();
        assert_eq!(texts.len(), 2);
        assert_eq!(texts[0], "Hello");
        assert_eq!(texts[1], "World");
    }
}
