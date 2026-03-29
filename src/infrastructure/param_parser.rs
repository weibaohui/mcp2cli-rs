//! Parameter parsing implementation

use crate::application::ports::ParamParserPort;
use crate::domain::{
    errors::{MCPError, MCPResult},
    services::ParamParser,
};
use async_trait::async_trait;
use std::collections::HashMap;

/// Parameter parser implementation
pub struct CliParamParser;

impl CliParamParser {
    pub fn new() -> Self {
        Self
    }
}

impl Default for CliParamParser {
    fn default() -> Self {
        Self::new()
    }
}

impl ParamParserPort for CliParamParser {
    fn parse_kv_args(&self, args: &[String]) -> MCPResult<HashMap<String, serde_json::Value>> {
        ParamParser::parse_kv_args(args)
    }

    fn parse_yaml(&self, yaml: &str) -> MCPResult<HashMap<String, serde_json::Value>> {
        ParamParser::parse_yaml(yaml)
    }

    fn has_piped_input(&self) -> bool {
        is_piped_input()
    }

    fn read_stdin_yaml(&self) -> MCPResult<HashMap<String, serde_json::Value>> {
        read_stdin_yaml()
    }

    async fn read_yaml_file(&self, path: &str) -> MCPResult<HashMap<String, serde_json::Value>> {
        let data = tokio::fs::read_to_string(path).await.map_err(|e| {
            MCPError::param_invalid(format!("Cannot read YAML file {:?}: {}", path, e))
        })?;
        ParamParser::parse_yaml(&data)
    }
}

/// Check if stdin is a pipe (piped input)
pub fn is_piped_input() -> bool {
    #[cfg(unix)]
    {
        use std::os::fd::AsRawFd;
        use nix::unistd::isatty;
        
        // Check if stdin is a tty
        return isatty(std::io::stdin().as_raw_fd()) != Ok(true);
    }
    
    #[cfg(not(unix))]
    {
        false
    }
}

/// Read YAML from stdin
pub fn read_stdin_yaml() -> MCPResult<HashMap<String, serde_json::Value>> {
    use std::io::Read;

    let mut data = String::new();
    std::io::stdin()
        .read_to_string(&mut data)
        .map_err(|e| MCPError::param_invalid(format!("Failed to read stdin: {}", e)))?;

    if data.trim().is_empty() {
        return Err(MCPError::param_invalid("empty stdin input"));
    }

    ParamParser::parse_yaml(&data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_param_parser() {
        let parser = CliParamParser::new();
        
        let args = vec![
            "name=John".to_string(),
            "age:number=30".to_string(),
        ];

        let result = parser.parse_kv_args(&args).unwrap();
        assert_eq!(result.get("name"), Some(&serde_json::json!("John")));
        assert_eq!(result.get("age"), Some(&serde_json::json!(30)));
    }

    #[test]
    fn test_parse_yaml() {
        let parser = CliParamParser::new();
        let yaml = "name: John\nage: 30";

        let result = parser.parse_yaml(yaml).unwrap();
        assert_eq!(result.get("name"), Some(&serde_json::json!("John")));
        assert_eq!(result.get("age"), Some(&serde_json::json!(30)));
    }
}
