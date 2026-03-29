//! Ports (interfaces) - Define what the application needs from infrastructure

use crate::domain::{
    entities::{CallToolResult, ListToolsResult, MCPConfig, ServerConfig},
    errors::MCPResult,
};
use async_trait::async_trait;
use std::collections::HashMap;

/// Port for configuration operations
#[async_trait]
pub trait ConfigPort: Send + Sync {
    /// Load the MCP configuration
    async fn load_config(&self) -> MCPResult<(MCPConfig, Vec<String>)>;

    /// Get a specific server configuration
    fn get_server_config(&self, config: &MCPConfig, name: &str) -> Option<ServerConfig>;

    /// List all server names
    fn list_servers(&self, config: &MCPConfig) -> Vec<String>;
}

/// Port for MCP client operations
#[async_trait]
pub trait MCPClientPort: Send + Sync {
    /// Connect to a server
    async fn connect(&mut self, server_name: &str, config: &ServerConfig) -> MCPResult<()>;

    /// Disconnect from the current server
    async fn disconnect(&mut self);

    /// List tools from the connected server
    async fn list_tools(&mut self) -> MCPResult<ListToolsResult>;

    /// Call a tool on the connected server
    async fn call_tool(
        &mut self,
        tool_name: &str,
        params: HashMap<String, serde_json::Value>,
    ) -> MCPResult<CallToolResult>;

    /// Check if connected to a server
    fn is_connected(&self) -> bool;

    /// Get current server name
    fn current_server(&self) -> Option<&str>;
}

/// Port for output formatting
pub trait OutputPort: Send + Sync {
    /// Output success response as JSON
    fn output_json<T: serde::Serialize>(&self, data: &T);

    /// Output success response as YAML
    fn output_yaml<T: serde::Serialize>(&self, data: &T);

    /// Output success response as compact text
    fn output_text<T: serde::Serialize>(&self, data: &T);

    /// Output error response
    fn output_error(&self, code: &str, message: &str, details: Option<serde_json::Value>);

    /// Output streaming content
    fn output_stream(&self, content: &str);

    /// Flush output
    fn flush(&self);
}

/// Port for parameter parsing
pub trait ParamParserPort: Send + Sync {
    /// Parse key=value arguments
    fn parse_kv_args(&self, args: &[String]) -> MCPResult<HashMap<String, serde_json::Value>>;

    /// Parse YAML string
    fn parse_yaml(&self, yaml: &str) -> MCPResult<HashMap<String, serde_json::Value>>;

    /// Check if stdin has piped input
    fn has_piped_input(&self) -> bool;

    /// Read YAML from stdin
    fn read_stdin_yaml(&self) -> MCPResult<HashMap<String, serde_json::Value>>;

    /// Read YAML from file
    async fn read_yaml_file(&self, path: &str) -> MCPResult<HashMap<String, serde_json::Value>>;
}

/// Port for OAuth operations
#[async_trait]
pub trait OAuthPort: Send + Sync {
    /// Perform OAuth authentication and get token
    async fn authenticate(&self, config: &crate::domain::value_objects::OAuthConfig) -> MCPResult<String>;
}

/// Port for environment variable resolution
pub trait EnvPort: Send + Sync {
    /// Resolve environment variables in a string
    fn resolve(&self, input: &str) -> String;

    /// Resolve environment variables in headers
    fn resolve_headers(&self, headers: &HashMap<String, String>) -> HashMap<String, String>;

    /// Get environment variable
    fn get_var(&self, name: &str) -> Option<String>;
}
