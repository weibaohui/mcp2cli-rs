//! Repository traits - Define interfaces for data access
//!
//! Following the Repository pattern from DDD, these traits define
//! the contracts that infrastructure implementations must fulfill.

use super::{
    entities::{CallToolResult, ListToolsResult, MCPConfig, ServerConfig, Tool},
    errors::MCPResult,
    value_objects::{ServerInfo, ToolInfo, ToolMatch},
};
use async_trait::async_trait;
use std::collections::HashMap;

/// Configuration repository for loading and saving MCP configs
#[async_trait]
pub trait ConfigRepository: Send + Sync {
    /// Load configuration from all search paths
    async fn load_config(&self) -> MCPResult<(MCPConfig, Vec<String>)>;

    /// Load configuration from specific paths
    async fn load_config_from_paths(&self, paths: Vec<String>) -> MCPResult<(MCPConfig, Vec<String>)>;

    /// Save configuration to a specific path
    async fn save_config(&self, config: &MCPConfig, path: &str) -> MCPResult<()>;

    /// Get config search paths for the current platform
    fn get_search_paths(&self) -> Vec<String>;
}

/// MCP client for interacting with a server
#[async_trait]
pub trait MCPClient: Send + Sync {
    /// Connect to the MCP server
    async fn connect(&mut self) -> MCPResult<()>;

    /// Disconnect from the server
    async fn disconnect(&mut self);

    /// Check if connected
    fn is_connected(&self) -> bool;

    /// List all available tools
    async fn list_tools(&mut self) -> MCPResult<ListToolsResult>;

    /// Call a tool with parameters
    async fn call_tool(
        &mut self,
        name: &str,
        params: Option<HashMap<String, serde_json::Value>>,
    ) -> MCPResult<CallToolResult>;

    /// Get server name
    fn server_name(&self) -> &str;

    /// Get transport type
    fn transport_type(&self) -> &str;
}

/// Factory for creating MCP clients
#[async_trait]
pub trait MCPClientFactory: Send + Sync {
    /// Create a new MCP client for the given server configuration
    async fn create_client(&self, name: &str, config: &ServerConfig) -> MCPResult<Box<dyn MCPClient>>;
}

/// Tool cache for storing tool information
#[async_trait]
pub trait ToolCache: Send + Sync {
    /// Cache tools for a server
    async fn cache_tools(&self, server: &str, tools: Vec<Tool>);

    /// Get cached tools for a server
    async fn get_cached_tools(&self, server: &str) -> Option<Vec<Tool>>;

    /// Clear cache for a server
    async fn clear_cache(&self, server: &str);

    /// Clear all caches
    async fn clear_all(&self);
}

/// OAuth service for authentication
#[async_trait]
pub trait OAuthService: Send + Sync {
    /// Perform OAuth flow and return access token
    async fn authenticate(&self, config: &super::value_objects::OAuthConfig) -> MCPResult<String>;

    /// Check if token is valid (may refresh if needed)
    async fn validate_token(&self, token: &str) -> MCPResult<bool>;
}

/// Transport layer abstraction
#[async_trait]
pub trait Transport: Send + Sync {
    /// Send a request and receive a response
    async fn send_request<Req, Res>(
        &mut self,
        method: &str,
        params: Req,
    ) -> MCPResult<Res>
    where
        Req: serde::Serialize + Send,
        Res: serde::de::DeserializeOwned + Send;

    /// Check if transport is connected
    fn is_connected(&self) -> bool;

    /// Close the transport connection
    async fn close(&mut self);
}

/// Server discovery service
#[async_trait]
pub trait ServerDiscovery: Send + Sync {
    /// Get information about a specific server
    async fn get_server_info(&self, server_name: &str) -> MCPResult<ServerInfo>;

    /// List all configured servers
    async fn list_servers(&self) -> MCPResult<Vec<ServerInfo>>;

    /// Find tool across all servers
    async fn find_tool(&self, tool_name: &str) -> MCPResult<Vec<ToolMatch>>;

    /// Search tools by query
    async fn search_tools(&self, query: &str) -> MCPResult<Vec<ToolMatch>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    // These tests just verify the traits compile correctly
    // Actual implementations are tested in the infrastructure layer

    #[tokio::test]
    async fn test_config_repository_trait_object() {
        // This test just ensures the trait is object-safe
        fn _assert_object_safe(_: &dyn ConfigRepository) {}
    }

    #[tokio::test]
    async fn test_mcp_client_trait_object() {
        fn _assert_object_safe(_: &dyn MCPClient) {}
    }
}
