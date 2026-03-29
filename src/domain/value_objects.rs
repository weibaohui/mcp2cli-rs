//! Value objects - Immutable objects defined by their attributes

use serde::{Deserialize, Serialize};
use std::fmt;

/// Transport type for MCP connections
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TransportType {
    Sse,
    Streamable,
    Stdio,
}

impl TransportType {
    /// Infer transport type from various hints
    pub fn infer(transport: Option<&str>, url: Option<&str>, command: Option<&str>) -> Self {
        // 1. Check explicit transport field
        if let Some(t) = transport {
            return Self::from_str(t);
        }

        // 2. Has command → stdio
        if command.is_some() && !command.unwrap().is_empty() {
            return Self::Stdio;
        }

        // 3. URL contains "sse" (case insensitive)
        if let Some(u) = url {
            let u_lower = u.to_lowercase();
            if u_lower.contains("sse") {
                return Self::Sse;
            }
            if u_lower.contains("stream") {
                return Self::Streamable;
            }
        }

        // 4. Default streamable
        Self::Streamable
    }

    fn from_str(s: &str) -> Self {
        let lower = s.to_lowercase();
        if lower.contains("stream") {
            Self::Streamable
        } else if lower.contains("sse") {
            Self::Sse
        } else if lower.contains("command") || lower.contains("stdio") {
            Self::Stdio
        } else {
            Self::Streamable
        }
    }
}

impl fmt::Display for TransportType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TransportType::Sse => write!(f, "sse"),
            TransportType::Streamable => write!(f, "streamable"),
            TransportType::Stdio => write!(f, "stdio"),
        }
    }
}

impl Default for TransportType {
    fn default() -> Self {
        Self::Streamable
    }
}

/// OAuth configuration for authentication
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OAuthConfig {
    #[serde(rename = "accessToken", skip_serializing_if = "Option::is_none")]
    pub access_token: Option<String>,
    #[serde(rename = "clientId", skip_serializing_if = "Option::is_none")]
    pub client_id: Option<String>,
    #[serde(rename = "clientSecret", skip_serializing_if = "Option::is_none")]
    pub client_secret: Option<String>,
    #[serde(rename = "authorizationURL", skip_serializing_if = "Option::is_none")]
    pub authorization_url: Option<String>,
    #[serde(rename = "tokenURL", skip_serializing_if = "Option::is_none")]
    pub token_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scopes: Option<String>,
    #[serde(rename = "redirectURL", skip_serializing_if = "Option::is_none")]
    pub redirect_url: Option<String>,
    #[serde(rename = "clientIdMetadataUrl", skip_serializing_if = "Option::is_none")]
    pub client_id_metadata_url: Option<String>,
}

/// Authentication configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AuthConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oauth: Option<OAuthConfig>,
}

/// Parameter information extracted from tool schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParamInfo {
    pub name: String,
    pub param_type: String,
    pub required: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl ParamInfo {
    pub fn new(name: impl Into<String>, param_type: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            param_type: param_type.into(),
            required: false,
            description: None,
        }
    }

    pub fn with_required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }
}

/// Output format for CLI results
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutputFormat {
    #[default]
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

impl fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OutputFormat::Json => write!(f, "json"),
            OutputFormat::Yaml => write!(f, "yaml"),
            OutputFormat::Text => write!(f, "text"),
        }
    }
}

/// Tool call result wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallResult {
    pub server: String,
    pub method: String,
    pub result: serde_json::Value,
}

/// Server information summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
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

/// Tool information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInfo {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_schema: Option<serde_json::Value>,
}

/// Tool match with server name
#[derive(Debug, Clone)]
pub struct ToolMatch {
    pub server_name: String,
    pub tool: ToolInfo,
}

/// Suggested tool match with reasoning
#[derive(Debug, Clone)]
pub struct SuggestMatch {
    pub server_name: String,
    pub tool: ToolInfo,
    pub reason: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transport_type_infer() {
        assert_eq!(
            TransportType::infer(Some("sse"), None, None),
            TransportType::Sse
        );
        assert_eq!(
            TransportType::infer(None, Some("http://example.com/sse"), None),
            TransportType::Sse
        );
        assert_eq!(
            TransportType::infer(None, None, Some("mycommand")),
            TransportType::Stdio
        );
        assert_eq!(
            TransportType::infer(None, Some("http://example.com/stream"), None),
            TransportType::Streamable
        );
        assert_eq!(
            TransportType::infer(None, None, None),
            TransportType::Streamable
        );
    }

    #[test]
    fn test_output_format_from_str() {
        assert_eq!(OutputFormat::from_str("json"), Some(OutputFormat::Json));
        assert_eq!(OutputFormat::from_str("yaml"), Some(OutputFormat::Yaml));
        assert_eq!(OutputFormat::from_str("YAML"), Some(OutputFormat::Yaml));
        assert_eq!(OutputFormat::from_str("text"), Some(OutputFormat::Text));
        assert_eq!(OutputFormat::from_str("invalid"), None);
    }
}
