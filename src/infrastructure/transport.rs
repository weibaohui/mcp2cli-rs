//! Transport implementations for MCP protocol

use crate::domain::{
    entities::{
        CallToolParams, CallToolResult, JsonRpcRequest, JsonRpcResponse, JsonRpcResult,
        ListToolsParams, ListToolsResult, ServerConfig,
    },
    errors::{MCPError, MCPResult},
    services::EnvResolver,
    value_objects::TransportType,
};
use reqwest::{header::HeaderMap, Client as HttpClient};
use serde::{de::DeserializeOwned, Serialize};
use std::collections::HashMap;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;
use std::sync::Arc;

/// Transport enum for different transport types
pub enum Transport {
    Http(HttpTransport),
    Stdio(StdioTransport),
}

impl Transport {
    /// Send a request and receive a response
    pub async fn request<Req, Res>(&mut self, method: &str, params: Req) -> MCPResult<Res>
    where
        Req: Serialize + Send,
        Res: DeserializeOwned + Send,
    {
        match self {
            Transport::Http(t) => t.request(method, params).await,
            Transport::Stdio(t) => t.request(method, params).await,
        }
    }

    /// Initialize the transport
    pub async fn initialize(&mut self) -> MCPResult<()> {
        match self {
            Transport::Http(t) => t.initialize().await,
            Transport::Stdio(t) => t.initialize().await,
        }
    }

    /// Close the transport
    pub async fn close(&mut self) {
        match self {
            Transport::Http(t) => t.close().await,
            Transport::Stdio(t) => t.close().await,
        }
    }

    /// Check if connected
    pub fn is_connected(&self) -> bool {
        match self {
            Transport::Http(t) => t.is_connected(),
            Transport::Stdio(t) => t.is_connected(),
        }
    }
}

/// HTTP-based transport for Streamable and SSE
pub struct HttpTransport {
    client: HttpClient,
    endpoint: String,
    transport_type: TransportType,
    headers: HeaderMap,
    request_id: u64,
}

impl HttpTransport {
    pub fn new(
        endpoint: String,
        transport_type: TransportType,
        headers: Option<HashMap<String, String>>,
    ) -> Self {
        let mut header_map = HeaderMap::new();
        if let Some(h) = headers {
            for (key, value) in h {
                if let Ok(header_name) = key.parse::<reqwest::header::HeaderName>() {
                    if let Ok(header_value) = value.parse::<reqwest::header::HeaderValue>() {
                        header_map.insert(header_name, header_value);
                    }
                }
            }
        }

        Self {
            client: HttpClient::new(),
            endpoint,
            transport_type,
            headers: header_map,
            request_id: 0,
        }
    }

    fn next_id(&mut self) -> u64 {
        self.request_id += 1;
        self.request_id
    }

    pub async fn request<Req, Res>(&mut self, method: &str, params: Req) -> MCPResult<Res>
    where
        Req: Serialize + Send,
        Res: DeserializeOwned + Send,
    {
        let id = self.next_id();
        let request = JsonRpcRequest::new(id, method, params);

        let url = if self.transport_type == TransportType::Sse {
            format!("{}/message", self.endpoint)
        } else {
            self.endpoint.clone()
        };

        let response = self
            .client
            .post(&url)
            .headers(self.headers.clone())
            .json(&request)
            .timeout(Duration::from_secs(60))
            .send()
            .await
            .map_err(|e| MCPError::transport_error(format!("HTTP request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response
                .text()
                .await
                .unwrap_or_default();
            return Err(MCPError::transport_error(format!(
                "HTTP error {}: {}",
                status, body
            )));
        }

        let rpc_response: JsonRpcResponse<Res> = response.json().await.map_err(|e| {
            MCPError::transport_error(format!("Failed to parse response: {}", e))
        })?;

        match rpc_response.result {
            JsonRpcResult::Result(result) => Ok(result),
            JsonRpcResult::Error(err) => Err(MCPError::transport_error(format!(
                "RPC error {}: {}",
                err.code, err.message
            ))),
        }
    }

    pub async fn initialize(&mut self) -> MCPResult<()> {
        // For HTTP transports, connection is stateless
        Ok(())
    }

    pub async fn close(&mut self) {
        // HTTP is stateless, nothing to close
    }

    pub fn is_connected(&self) -> bool {
        true // HTTP is always "connected"
    }
}

/// Stdio transport for command-based MCP servers
pub struct StdioTransport {
    child: Option<Child>,
    stdin: Option<Arc<Mutex<tokio::process::ChildStdin>>>,
    stdout: Option<Arc<Mutex<BufReader<tokio::process::ChildStdout>>>>,
    request_id: u64,
    command: String,
    args: Vec<String>,
    env: HashMap<String, String>,
}

impl StdioTransport {
    pub fn new(command: String, args: Vec<String>, env: HashMap<String, String>) -> Self {
        Self {
            child: None,
            stdin: None,
            stdout: None,
            request_id: 0,
            command,
            args,
            env,
        }
    }

    fn next_id(&mut self) -> u64 {
        self.request_id += 1;
        self.request_id
    }

    pub async fn initialize(&mut self) -> MCPResult<()> {
        let mut cmd = Command::new(&self.command);
        cmd.args(&self.args)
            .envs(&self.env)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());

        let mut child = cmd.spawn().map_err(|e| {
            MCPError::transport_error(format!(
                "Failed to spawn command '{}': {}",
                self.command, e
            ))
        })?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| MCPError::transport_error("Failed to get stdin"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| MCPError::transport_error("Failed to get stdout"))?;

        self.stdin = Some(Arc::new(Mutex::new(stdin)));
        self.stdout = Some(Arc::new(Mutex::new(BufReader::new(stdout))));
        self.child = Some(child);

        Ok(())
    }

    pub async fn request<Req, Res>(&mut self, method: &str, params: Req) -> MCPResult<Res>
    where
        Req: Serialize + Send,
        Res: DeserializeOwned + Send,
    {
        let id = self.next_id();
        let request = JsonRpcRequest::new(id, method, params);
        let request_json = serde_json::to_string(&request).map_err(|e| {
            MCPError::transport_error(format!("Failed to serialize request: {}", e))
        })?;

        // Send request
        if let Some(stdin) = &self.stdin {
            let mut stdin = stdin.lock().await;
            stdin
                .write_all(request_json.as_bytes())
                .await
                .map_err(|e| MCPError::transport_error(format!("Failed to write: {}", e)))?;
            stdin
                .write_all(b"\n")
                .await
                .map_err(|e| MCPError::transport_error(format!("Failed to write newline: {}", e)))?;
            stdin
                .flush()
                .await
                .map_err(|e| MCPError::transport_error(format!("Failed to flush: {}", e)))?;
        }

        // Read response
        if let Some(stdout) = &self.stdout {
            let mut stdout = stdout.lock().await;
            let mut line = String::new();
            stdout
                .read_line(&mut line)
                .await
                .map_err(|e| MCPError::transport_error(format!("Failed to read: {}", e)))?;

            let response: JsonRpcResponse<Res> = serde_json::from_str(&line).map_err(|e| {
                MCPError::transport_error(format!(
                    "Failed to parse response: {}. Response was: {}",
                    e, line
                ))
            })?;

            match response.result {
                JsonRpcResult::Result(result) => Ok(result),
                JsonRpcResult::Error(err) => Err(MCPError::transport_error(format!(
                    "RPC error {}: {}",
                    err.code, err.message
                ))),
            }
        } else {
            Err(MCPError::transport_error("Not connected"))
        }
    }

    pub async fn close(&mut self) {
        if let Some(child) = &mut self.child {
            let _ = child.kill().await;
        }
        self.child = None;
        self.stdin = None;
        self.stdout = None;
    }

    pub fn is_connected(&self) -> bool {
        self.child.is_some()
    }
}

/// Transport factory
pub struct TransportFactory;

impl TransportFactory {
    pub fn create(config: &ServerConfig) -> MCPResult<Transport> {
        let transport_type = config.get_transport_type();

        match transport_type {
            TransportType::Stdio => {
                let command = config
                    .command
                    .clone()
                    .ok_or_else(|| MCPError::transport_error("stdio transport requires command"))?;
                let args = config.args.clone().unwrap_or_default();
                let env = config.env.clone().unwrap_or_default();
                Ok(Transport::Stdio(StdioTransport::new(command, args, env)))
            }
            TransportType::Sse | TransportType::Streamable => {
                let url = config
                    .url
                    .clone()
                    .ok_or_else(|| MCPError::transport_error("HTTP transport requires url"))?;
                
                // Resolve environment variables in headers
                let headers = config.headers.as_ref().map(|h| {
                    h.iter()
                        .map(|(k, v)| (k.clone(), EnvResolver::resolve(v)))
                        .collect()
                });

                Ok(Transport::Http(HttpTransport::new(
                    url,
                    transport_type,
                    headers,
                )))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http_transport_new() {
        let transport = HttpTransport::new(
            "http://example.com".to_string(),
            TransportType::Streamable,
            None,
        );
        assert!(transport.is_connected());
    }
}
