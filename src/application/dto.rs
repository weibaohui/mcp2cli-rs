//! Data Transfer Objects - For communication between layers

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Request DTO for listing servers
#[derive(Debug, Clone)]
pub struct ListServersRequest {
    pub include_tools: bool,
}

impl Default for ListServersRequest {
    fn default() -> Self {
        Self {
            include_tools: false,
        }
    }
}

/// Request DTO for getting server info
#[derive(Debug, Clone)]
pub struct GetServerInfoRequest {
    pub server_name: String,
}

/// Request DTO for listing tools
#[derive(Debug, Clone)]
pub struct ListToolsRequest {
    pub server_name: String,
}

/// Request DTO for getting tool info
#[derive(Debug, Clone)]
pub struct GetToolInfoRequest {
    pub server_name: String,
    pub tool_name: String,
}

/// Request DTO for calling a tool
#[derive(Debug, Clone)]
pub struct CallToolRequest {
    pub server_name: String,
    pub tool_name: String,
    pub params: HashMap<String, serde_json::Value>,
    pub stream: bool,
}

/// Request DTO for finding a tool
#[derive(Debug, Clone)]
pub struct FindToolRequest {
    pub tool_name: String,
}

/// Request DTO for searching tools
#[derive(Debug, Clone)]
pub struct SearchToolsRequest {
    pub query: String,
}

/// Response DTO for server list
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerListResponse {
    pub servers: Vec<ServerDto>,
    pub config_files: Vec<String>,
}

/// Server DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerDto {
    pub name: String,
    pub transport: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Tool list response DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolListResponse {
    pub server: String,
    pub tools: Vec<ToolSummaryDto>,
}

/// Tool summary DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSummaryDto {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Tool info response DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInfoResponse {
    pub server: String,
    pub tool: ToolDetailDto,
}

/// Tool detail DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDetailDto {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required_params: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub param_format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub param_example: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub call_example: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_schema: Option<serde_json::Value>,
}

/// Tool call response DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallResponse {
    pub server: String,
    pub method: String,
    pub result: serde_json::Value,
}

/// Tool search response DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSearchResponse {
    pub matches: Vec<ToolMatchDto>,
}

/// Tool match DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolMatchDto {
    pub server: String,
    pub tool: ToolSummaryDto,
}

/// Tool suggestion response DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSuggestResponse {
    pub suggestions: Vec<ToolSuggestionDto>,
}

/// Tool suggestion DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSuggestionDto {
    pub server: String,
    pub tool: ToolSummaryDto,
    pub reason: String,
}

/// Error response DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Map<String, serde_json::Value>>,
}

/// Success response wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuccessResponse<T> {
    pub success: bool,
    pub data: T,
    pub meta: ResponseMeta,
}

/// Response metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseMeta {
    pub timestamp: String,
    pub version: String,
}

/// Error response wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponseWrapper {
    pub success: bool,
    pub error: ErrorResponse,
    pub meta: ResponseMeta,
}

/// CLI output format options
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Json,
    Yaml,
    Text,
}

impl OutputFormat {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "json" => Some(Self::Json),
            "yaml" | "yml" => Some(Self::Yaml),
            "text" => Some(Self::Text),
            _ => None,
        }
    }
}

impl Default for OutputFormat {
    fn default() -> Self {
        Self::Json
    }
}

/// Configuration info DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigInfoDto {
    pub paths: Vec<String>,
    pub server_count: usize,
}
