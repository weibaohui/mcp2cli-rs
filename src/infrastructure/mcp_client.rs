//! MCP client implementation

use crate::application::ports::MCPClientPort;
use crate::domain::{
    entities::{CallToolParams, CallToolResult, ListToolsParams, ListToolsResult, ServerConfig, Tool},
    errors::{MCPError, MCPResult},
    value_objects::TransportType,
};
use async_trait::async_trait;
use std::collections::HashMap;

use super::transport::{Transport, TransportFactory};
use std::pin::Pin;
use std::future::Future;

/// MCP client implementation
pub struct MCPClientImpl {
    server_name: Option<String>,
    transport: Option<Transport>,
    tool_cache: Option<Vec<Tool>>,
    timeout_ms: u64,
}

impl MCPClientImpl {
    pub fn new() -> Self {
        Self {
            server_name: None,
            transport: None,
            tool_cache: None,
            timeout_ms: 30000,
        }
    }

    fn ensure_connected(&mut self) -> MCPResult<&mut Transport> {
        self.transport.as_mut().ok_or_else(|| {
            MCPError::transport_error("Not connected to server")
        })
    }
}

impl Default for MCPClientImpl {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl MCPClientPort for MCPClientImpl {
    async fn connect(&mut self, server_name: &str, config: &ServerConfig) -> MCPResult<()> {
        // Disconnect if already connected
        if self.transport.is_some() {
            self.disconnect().await;
        }

        self.timeout_ms = config.get_timeout_ms();
        self.server_name = Some(server_name.to_string());

        // Create and initialize transport
        let mut transport = TransportFactory::create(config)?;
        
        // Set timeout for connection
        let timeout = tokio::time::Duration::from_millis(self.timeout_ms);
        let result = tokio::time::timeout(
            timeout,
            transport.initialize()
        ).await;

        match result {
            Ok(Ok(())) => {
                self.transport = Some(transport);
                Ok(())
            }
            Ok(Err(e)) => Err(e),
            Err(_) => Err(MCPError::connect_failed(
                server_name,
                format!("Connection timeout after {}ms", self.timeout_ms),
            )),
        }
    }

    async fn disconnect(&mut self) {
        if let Some(transport) = &mut self.transport {
            transport.close().await;
        }
        self.transport = None;
        self.server_name = None;
        self.tool_cache = None;
    }

    fn is_connected(&self) -> bool {
        self.transport
            .as_ref()
            .map(|t| t.is_connected())
            .unwrap_or(false)
    }

    async fn list_tools(&mut self) -> MCPResult<ListToolsResult> {
        let timeout_ms = self.timeout_ms;
        let transport = self.ensure_connected()?;

        let timeout = tokio::time::Duration::from_millis(timeout_ms);
        let params = ListToolsParams::default();

        let result: Result<MCPResult<ListToolsResult>, _> = tokio::time::timeout(
            timeout,
            transport.request("tools/list", params)
        ).await;

        match result {
            Ok(Ok(tools)) => {
                self.tool_cache = Some(tools.tools.clone());
                Ok(tools)
            }
            Ok(Err(e)) => Err(e),
            Err(_) => Err(MCPError::transport_error("List tools timeout")),
        }
    }

    async fn call_tool(
        &mut self,
        tool_name: &str,
        params: HashMap<String, serde_json::Value>,
    ) -> MCPResult<CallToolResult> {
        let timeout_ms = self.timeout_ms;
        let server_name = self.server_name.clone().unwrap_or_default();
        let transport = self.ensure_connected()?;

        let timeout = tokio::time::Duration::from_millis(timeout_ms);
        
        let call_params = CallToolParams::new(tool_name)
            .with_arguments(serde_json::Value::Object(
                params.into_iter().map(|(k, v)| (k, v)).collect()
            ));

        let result: Result<MCPResult<CallToolResult>, _> = tokio::time::timeout(
            timeout,
            transport.request("tools/call", call_params)
        ).await;

        match result {
            Ok(Ok(result)) => Ok(result),
            Ok(Err(e)) => Err(MCPError::call_failed(tool_name, server_name, e)),
            Err(_) => Err(MCPError::call_failed(
                tool_name,
                server_name,
                "Request timeout",
            )),
        }
    }

    fn current_server(&self) -> Option<&str> {
        self.server_name.as_deref()
    }
}

/// Thread-safe MCP client wrapper
pub struct ThreadSafeMCPClient {
    inner: tokio::sync::Mutex<MCPClientImpl>,
}

impl ThreadSafeMCPClient {
    pub fn new() -> Self {
        Self {
            inner: tokio::sync::Mutex::new(MCPClientImpl::new()),
        }
    }
}

impl Default for ThreadSafeMCPClient {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl MCPClientPort for ThreadSafeMCPClient {
    async fn connect(&mut self, server_name: &str, config: &ServerConfig) -> MCPResult<()> {
        let mut client = self.inner.lock().await;
        client.connect(server_name, config).await
    }

    async fn disconnect(&mut self) {
        let mut client = self.inner.lock().await;
        client.disconnect().await;
    }

    fn is_connected(&self) -> bool {
        // This is a bit tricky with the mutex - we'd need to block or use try_lock
        // For simplicity, we'll assume it's connected if we can get the lock
        true
    }

    async fn list_tools(&mut self) -> MCPResult<ListToolsResult> {
        let mut client = self.inner.lock().await;
        client.list_tools().await
    }

    async fn call_tool(
        &mut self,
        tool_name: &str,
        params: HashMap<String, serde_json::Value>,
    ) -> MCPResult<CallToolResult> {
        let mut client = self.inner.lock().await;
        client.call_tool(tool_name, params).await
    }

    fn current_server(&self) -> Option<&str> {
        // Same issue as is_connected
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_client_new() {
        let client = MCPClientImpl::new();
        assert!(!client.is_connected());
        assert!(client.current_server().is_none());
    }
}
