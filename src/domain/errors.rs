//! Domain errors - Error types for the MCP domain

use std::fmt;
use thiserror::Error;

/// Error codes for MCP operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    ConfigNotFound,
    ConnectFailed,
    ServerNotFound,
    MethodNotFound,
    MethodAmbiguous,
    CallFailed,
    ParamInvalid,
    AuthFailed,
    TransportError,
    ParseError,
    InternalError,
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorCode::ConfigNotFound => write!(f, "MCP_CONFIG_NOT_FOUND"),
            ErrorCode::ConnectFailed => write!(f, "MCP_CONNECT_FAILED"),
            ErrorCode::ServerNotFound => write!(f, "MCP_SERVER_NOT_FOUND"),
            ErrorCode::MethodNotFound => write!(f, "MCP_METHOD_NOT_FOUND"),
            ErrorCode::MethodAmbiguous => write!(f, "MCP_METHOD_AMBIGUOUS"),
            ErrorCode::CallFailed => write!(f, "MCP_CALL_FAILED"),
            ErrorCode::ParamInvalid => write!(f, "MCP_PARAM_INVALID"),
            ErrorCode::AuthFailed => write!(f, "MCP_AUTH_FAILED"),
            ErrorCode::TransportError => write!(f, "MCP_TRANSPORT_ERROR"),
            ErrorCode::ParseError => write!(f, "MCP_PARSE_ERROR"),
            ErrorCode::InternalError => write!(f, "MCP_INTERNAL_ERROR"),
        }
    }
}

/// Main MCP error type
#[derive(Debug, Error, Clone)]
#[error("[{code}] {message}")]
pub struct MCPError {
    pub code: ErrorCode,
    pub message: String,
    pub details: Option<serde_json::Map<String, serde_json::Value>>,
}

impl MCPError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            details: None,
        }
    }

    pub fn with_details(
        code: ErrorCode,
        message: impl Into<String>,
        details: serde_json::Map<String, serde_json::Value>,
    ) -> Self {
        Self {
            code,
            message: message.into(),
            details: Some(details),
        }
    }

    pub fn config_not_found(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::ConfigNotFound, message)
    }

    pub fn connect_failed(server: impl Into<String>, message: impl Into<String>) -> Self {
        let server = server.into();
        let mut details = serde_json::Map::new();
        details.insert("server".to_string(), server.into());
        Self::with_details(
            ErrorCode::ConnectFailed,
            message,
            details,
        )
    }

    pub fn server_not_found(server: impl Into<String>) -> Self {
        let server = server.into();
        let mut details = serde_json::Map::new();
        details.insert("server".to_string(), server.clone().into());
        Self::with_details(
            ErrorCode::ServerNotFound,
            format!("Server {:?} not found in config", server),
            details,
        )
    }

    pub fn method_not_found(method: impl Into<String>, server: impl Into<String>) -> Self {
        let method = method.into();
        let server = server.into();
        let mut details = serde_json::Map::new();
        details.insert("method".to_string(), method.clone().into());
        details.insert("server".to_string(), server.clone().into());
        Self::with_details(
            ErrorCode::MethodNotFound,
            format!("Tool {:?} not found on server {:?}", method, server),
            details,
        )
    }

    pub fn call_failed(
        method: impl Into<String>,
        server: impl Into<String>,
        err: impl fmt::Display,
    ) -> Self {
        let method = method.into();
        let server = server.into();
        let mut details = serde_json::Map::new();
        details.insert("method".to_string(), method.clone().into());
        details.insert("server".to_string(), server.clone().into());
        Self::with_details(
            ErrorCode::CallFailed,
            format!("Failed to call tool {:?} on server {:?}: {}", method, server, err),
            details,
        )
    }

    pub fn param_invalid(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::ParamInvalid, message)
    }

    pub fn auth_failed(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::AuthFailed, message)
    }

    pub fn transport_error(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::TransportError, message)
    }

    pub fn parse_error(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::ParseError, message)
    }

    /// Check if this is a retryable error
    pub fn is_retryable(&self) -> bool {
        match self.code {
            ErrorCode::ConnectFailed | ErrorCode::TransportError => {
                let msg = self.message.to_lowercase();
                let retryable_patterns = [
                    "connection refused",
                    "connection reset",
                    "connection timed out",
                    "timeout",
                    "temporary failure",
                    "i/o timeout",
                    "network unreachable",
                    "no such host",
                    "use of closed network connection",
                ];
                retryable_patterns.iter().any(|p| msg.contains(p))
            }
            _ => false,
        }
    }
}

/// Result type alias for MCP operations
pub type MCPResult<T> = Result<T, MCPError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_code_display() {
        assert_eq!(
            ErrorCode::ConfigNotFound.to_string(),
            "MCP_CONFIG_NOT_FOUND"
        );
        assert_eq!(
            ErrorCode::ConnectFailed.to_string(),
            "MCP_CONNECT_FAILED"
        );
    }

    #[test]
    fn test_mcp_error_format() {
        let err = MCPError::config_not_found("test config");
        assert!(err.to_string().contains("MCP_CONFIG_NOT_FOUND"));
        assert!(err.to_string().contains("test config"));
    }

    #[test]
    fn test_is_retryable() {
        let retryable = MCPError::connect_failed("test", "connection refused");
        assert!(retryable.is_retryable());

        let not_retryable = MCPError::param_invalid("invalid");
        assert!(!not_retryable.is_retryable());
    }
}
