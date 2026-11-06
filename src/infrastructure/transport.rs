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
    session_id: Option<String>,
    protocol_version: Option<String>,
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

        // Streamable HTTP servers require the client to accept both
        // content types; without this they answer 406 Not Acceptable.
        if !header_map.contains_key(reqwest::header::ACCEPT) {
            header_map.insert(
                reqwest::header::ACCEPT,
                reqwest::header::HeaderValue::from_static(
                    "application/json, text/event-stream",
                ),
            );
        }

        Self {
            client: HttpClient::new(),
            endpoint,
            transport_type,
            headers: header_map,
            request_id: 0,
            session_id: None,
            protocol_version: None,
        }
    }

    fn next_id(&mut self) -> u64 {
        self.request_id += 1;
        self.request_id
    }

    fn request_url(&self) -> String {
        if self.transport_type == TransportType::Sse {
            format!("{}/message", self.endpoint)
        } else {
            self.endpoint.clone()
        }
    }

    /// Send one JSON-RPC payload, echoing the session id and the
    /// negotiated protocol version once known.
    async fn send(&self, body: &serde_json::Value) -> MCPResult<reqwest::Response> {
        let mut req = self
            .client
            .post(self.request_url())
            .headers(self.headers.clone())
            .json(body)
            .timeout(Duration::from_secs(60));

        if let Some(sid) = &self.session_id {
            req = req.header("Mcp-Session-Id", sid);
        }

        if let Some(version) = &self.protocol_version {
            req = req.header("MCP-Protocol-Version", version);
        }

        req.send()
            .await
            .map_err(|e| MCPError::transport_error(format!("HTTP request failed: {}", e)))
    }

    /// Remember the session id handed out by the server, if any.
    fn update_session(&mut self, response: &reqwest::Response) {
        if let Some(value) = response.headers().get("mcp-session-id") {
            if let Ok(sid) = value.to_str() {
                if !sid.is_empty() {
                    self.session_id = Some(sid.to_string());
                }
            }
        }
    }

    /// Extract the JSON-RPC payload from a response body, which may be
    /// plain JSON or a `text/event-stream` (SSE) stream.
    fn extract_payload(text: &str) -> String {
        let trimmed = text.trim();
        if trimmed.starts_with('{') {
            return trimmed.to_string();
        }

        // SSE: collect the `data:` lines. A stream may carry server
        // notifications before the actual response, so prefer the last
        // payload that looks like a JSON-RPC result/error message.
        let payloads: Vec<&str> = text
            .lines()
            .filter_map(|line| line.trim().strip_prefix("data:"))
            .map(|data| data.trim())
            .filter(|data| !data.is_empty())
            .collect();

        if let Some(payload) = payloads
            .iter()
            .rev()
            .find(|data| data.contains("\"result\"") || data.contains("\"error\""))
        {
            return payload.to_string();
        }

        payloads
            .last()
            .map(|data| data.to_string())
            .unwrap_or_else(|| trimmed.to_string())
    }

    fn is_session_not_ready(body: &str) -> bool {
        let lower = body.to_lowercase();
        lower.contains("missing session id") || lower.contains("session not found")
    }

    /// Read the protocol version the server agreed to in its initialize
    /// result, if the response is parseable and carries one.
    fn negotiated_version(payload: &str) -> Option<String> {
        serde_json::from_str::<serde_json::Value>(payload)
            .ok()?
            .pointer("/result/protocolVersion")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }

    pub async fn request<Req, Res>(&mut self, method: &str, params: Req) -> MCPResult<Res>
    where
        Req: Serialize + Send,
        Res: DeserializeOwned + Send,
    {
        let id = self.next_id();
        let request = JsonRpcRequest::new(id, method, params);
        let body = serde_json::to_value(&request).map_err(|e| {
            MCPError::transport_error(format!("Failed to serialize request: {}", e))
        })?;

        // A session may be rejected because it is still being registered
        // or has expired. Per spec a 404 means the client must
        // re-initialize, so re-run the handshake to get a fresh session
        // instead of retrying with a session the server already lost.
        const MAX_ATTEMPTS: u32 = 3;
        for attempt in 0..MAX_ATTEMPTS {
            let response = self.send(&body).await?;
            self.update_session(&response);

            let status = response.status();
            let text = response.text().await.unwrap_or_default();

            if !status.is_success() {
                if Self::is_session_not_ready(&text) && attempt + 1 < MAX_ATTEMPTS {
                    self.session_id = None;
                    self.initialize().await?;
                    tokio::time::sleep(Duration::from_millis(200)).await;
                    continue;
                }
                return Err(MCPError::transport_error(format!(
                    "HTTP error {}: {}",
                    status, text
                )));
            }

            let payload = Self::extract_payload(&text);
            let rpc_response: JsonRpcResponse<Res> =
                serde_json::from_str(&payload).map_err(|e| {
                    MCPError::transport_error(format!("Failed to parse response: {}", e))
                })?;

            return match rpc_response.result {
                JsonRpcResult::Result(result) => Ok(result),
                JsonRpcResult::Error(err) => Err(MCPError::transport_error(format!(
                    "RPC error {}: {}",
                    err.code, err.message
                ))),
            };
        }

        Err(MCPError::transport_error(
            "Session did not become ready after re-initialization attempts",
        ))
    }

    pub async fn initialize(&mut self) -> MCPResult<()> {
        // Only Streamable HTTP has a session lifecycle. SSE keeps the
        // previous behaviour and stdio only spawns the process.
        if self.transport_type != TransportType::Streamable {
            return Ok(());
        }

        // 1) initialize must be the first interaction; the server hands
        //    out the session id in a response header. Streamable HTTP was
        //    introduced in protocol revision 2025-03-26, so announce that
        //    version — 2024-11-05 predates it and a strict server may
        //    reject the handshake.
        let id = self.next_id();
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-03-26",
                "capabilities": {},
                "clientInfo": {
                    "name": "rmcp",
                    "version": env!("CARGO_PKG_VERSION"),
                },
            },
        });

        let response = self.send(&body).await?;
        self.update_session(&response);

        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(MCPError::transport_error(format!(
                "HTTP error {}: {}",
                status, text
            )));
        }

        let payload = Self::extract_payload(&text);
        if !payload.is_empty() {
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(&payload) {
                if let Some(err) = value.get("error") {
                    let code = err.get("code").and_then(|c| c.as_i64()).unwrap_or(0);
                    let message = err
                        .get("message")
                        .and_then(|m| m.as_str())
                        .unwrap_or("unknown error");
                    return Err(MCPError::transport_error(format!(
                        "RPC error {}: {}",
                        code, message
                    )));
                }

                // Remember the version the server agreed to and echo it on
                // later requests (required by revision 2025-06-18, ignored
                // by older servers). A server that omits it is
                // non-conformant but tolerated.
                if let Some(version) = value
                    .pointer("/result/protocolVersion")
                    .and_then(|v| v.as_str())
                {
                    self.protocol_version = Some(version.to_string());
                }
            }
        }

        // 2) Tell the server the client is ready (notification, no id),
        //    echoing the session id via send().
        let notification = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized",
        });
        let response = self.send(&notification).await?;
        self.update_session(&response);
        // Servers answer 202 Accepted with an empty body; the exact
        // status is not significant, so just drain the body.
        let _ = response.text().await;

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
        assert!(transport.session_id.is_none());
    }

    #[test]
    fn test_http_transport_adds_accept_header() {
        let transport = HttpTransport::new(
            "http://example.com".to_string(),
            TransportType::Streamable,
            None,
        );
        assert_eq!(
            transport.headers.get(reqwest::header::ACCEPT).unwrap(),
            "application/json, text/event-stream"
        );
    }

    #[test]
    fn test_http_transport_keeps_configured_accept_header() {
        let mut headers = HashMap::new();
        headers.insert("Accept".to_string(), "application/json".to_string());
        let transport = HttpTransport::new(
            "http://example.com".to_string(),
            TransportType::Streamable,
            Some(headers),
        );
        assert_eq!(
            transport.headers.get(reqwest::header::ACCEPT).unwrap(),
            "application/json"
        );
    }

    #[test]
    fn test_extract_payload_plain_json() {
        let payload = HttpTransport::extract_payload("  {\"jsonrpc\":\"2.0\"}  ");
        assert_eq!(payload, "{\"jsonrpc\":\"2.0\"}");
    }

    #[test]
    fn test_extract_payload_sse() {
        let body = "event: message\ndata: {\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{}}\n\n";
        let payload = HttpTransport::extract_payload(body);
        assert_eq!(payload, "{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{}}");
    }

    #[test]
    fn test_extract_payload_sse_skips_notifications() {
        let body = "data: {\"jsonrpc\":\"2.0\",\"method\":\"notifications/message\"}\n\ndata: {\"jsonrpc\":\"2.0\",\"id\":2,\"result\":{\"tools\":[]}}\n\n";
        let payload = HttpTransport::extract_payload(body);
        assert!(payload.contains("\"result\""));
        assert!(payload.contains("\"id\":2"));
    }

    #[test]
    fn test_is_session_not_ready() {
        assert!(HttpTransport::is_session_not_ready(
            "Bad Request: Missing session ID"
        ));
        assert!(HttpTransport::is_session_not_ready("Session not found"));
        assert!(!HttpTransport::is_session_not_ready("some other error"));
    }

    #[test]
    fn test_negotiated_version() {
        let payload = r#"{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":"2025-03-26","capabilities":{},"serverInfo":{"name":"mock","version":"1.0"}}}"#;
        assert_eq!(
            HttpTransport::negotiated_version(payload).as_deref(),
            Some("2025-03-26")
        );
        // No protocolVersion in the result: tolerated, no version echoed.
        assert!(HttpTransport::negotiated_version(r#"{"jsonrpc":"2.0","id":1,"result":{}}"#).is_none());
        // Unparseable payload: no version echoed.
        assert!(HttpTransport::negotiated_version("not json").is_none());
    }
}
