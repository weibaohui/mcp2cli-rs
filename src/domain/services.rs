//! Domain services - Business logic that doesn't belong to a single entity

use super::{
    entities::{CallToolResult, ServerConfig, Tool},
    errors::{MCPError, MCPResult},
    value_objects::{ParamInfo, ToolInfo, ToolMatch, TransportType},
};
use std::collections::HashMap;

/// Parameter parsing service
pub struct ParamParser;

impl ParamParser {
    /// Parse key=value or key:type=value format arguments
    pub fn parse_kv_args(args: &[String]) -> MCPResult<HashMap<String, serde_json::Value>> {
        let mut result = HashMap::new();

        for arg in args {
            let parts: Vec<&str> = arg.splitn(2, '=').collect();
            if parts.len() != 2 {
                return Err(MCPError::param_invalid(format!(
                    "invalid argument format: {:?} (expected key=value or key:type=value)",
                    arg
                )));
            }

            let key_part = parts[0];
            let val_str = parts[1];

            // Parse key and optional type annotation
            let (key, type_hint) = Self::parse_key_with_type(key_part)?;

            // Convert value based on type
            let converted_val = Self::convert_value(val_str, &type_hint).map_err(|e| {
                MCPError::param_invalid(format!(
                    "argument {:?} value {:?} cannot be converted to type {:?}: {}",
                    key, val_str, type_hint, e
                ))
            })?;

            result.insert(key, converted_val);
        }

        Ok(result)
    }

    /// Parse "key" or "key:type"
    fn parse_key_with_type(key_part: &str) -> MCPResult<(String, String)> {
        if let Some(colon_idx) = key_part.find(':') {
            let key = &key_part[..colon_idx];
            let type_hint = &key_part[colon_idx + 1..];

            if key.is_empty() {
                return Err(MCPError::param_invalid("argument key cannot be empty"));
            }

            // Validate type
            let valid_types = ["string", "number", "int", "float", "bool", "boolean"];
            if !valid_types.contains(&type_hint) {
                return Err(MCPError::param_invalid(format!(
                    "argument {:?} uses unsupported type {:?}, supported types: string, number, int, float, bool",
                    key, type_hint
                )));
            }

            Ok((key.to_string(), type_hint.to_string()))
        } else {
            if key_part.is_empty() {
                return Err(MCPError::param_invalid("argument key cannot be empty"));
            }
            Ok((key_part.to_string(), "string".to_string()))
        }
    }

    /// Convert a string value based on type hint
    fn convert_value(val_str: &str, type_hint: &str) -> anyhow::Result<serde_json::Value> {
        match type_hint {
            "string" => Ok(serde_json::Value::String(val_str.to_string())),
            "number" | "int" => {
                // Try integer first
                if let Ok(int_val) = val_str.parse::<i64>() {
                    Ok(serde_json::Value::Number(int_val.into()))
                } else {
                    // Try float
                    let float_val: f64 = val_str.parse()?;
                    Ok(serde_json::Number::from_f64(float_val)
                        .map(serde_json::Value::Number)
                        .unwrap_or_else(|| serde_json::Value::String(val_str.to_string())))
                }
            }
            "float" => {
                let float_val: f64 = val_str.parse()?;
                Ok(serde_json::Number::from_f64(float_val)
                    .map(serde_json::Value::Number)
                    .unwrap_or_else(|| serde_json::Value::String(val_str.to_string())))
            }
            "bool" | "boolean" => {
                let lower = val_str.to_lowercase();
                let bool_val = matches!(lower.as_str(), "true" | "1" | "yes");
                Ok(serde_json::Value::Bool(bool_val))
            }
            _ => Ok(serde_json::Value::String(val_str.to_string())),
        }
    }

    /// Parse YAML string to parameters
    pub fn parse_yaml(yaml_str: &str) -> MCPResult<HashMap<String, serde_json::Value>> {
        let trimmed = yaml_str.trim();
        if trimmed.is_empty() {
            return Err(MCPError::param_invalid("empty YAML input"));
        }

        let result: HashMap<String, serde_json::Value> =
            serde_yaml::from_str(trimmed).map_err(|e| {
                MCPError::param_invalid(format!("invalid YAML: {}", e))
            })?;

        if result.is_empty() {
            return Err(MCPError::param_invalid("empty YAML input"));
        }

        Ok(result)
    }
}

/// Schema formatting service
pub struct SchemaFormatter;

impl SchemaFormatter {
    /// Format input schema to human-readable parameter format
    pub fn format_input_schema(schema: &serde_json::Value) -> Option<Vec<String>> {
        let schema_map = schema.as_object()?;
        let properties = schema_map.get("properties")?.as_object()?;

        let mut result = Vec::new();
        for (key, prop) in properties {
            let prop_map = prop.as_object()?;
            let json_type = prop_map
                .get("type")
                .and_then(|t| t.as_str())
                .unwrap_or("string");
            let type_hint = Self::json_type_to_type_hint(json_type);

            // Format: key:type={value} // description
            let mut line = format!("{}:{}={{", key, type_hint);
            line.push('}');
            if let Some(desc) = prop_map.get("description").and_then(|d| d.as_str()) {
                line.push_str(" // ");
                line.push_str(desc);
            }
            result.push(line);
        }

        Some(result)
    }

    /// Map JSON types to type hints
    fn json_type_to_type_hint(json_type: &str) -> &str {
        match json_type {
            "number" => "number",
            "integer" => "int",
            "boolean" => "bool",
            "array" => "array",
            "object" => "object",
            "string" => "string",
            _ => "string",
        }
    }

    /// Get required parameter names from schema
    pub fn get_required_params(schema: &serde_json::Value) -> Vec<String> {
        let schema_map = match schema.as_object() {
            Some(m) => m,
            None => return Vec::new(),
        };

        schema_map
            .get("required")
            .and_then(|r| r.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Extract structured parameter information from schema
    pub fn get_param_info_list(schema: &serde_json::Value) -> Vec<ParamInfo> {
        let schema_map = match schema.as_object() {
            Some(m) => m,
            None => return Vec::new(),
        };

        let properties = match schema_map.get("properties").and_then(|p| p.as_object()) {
            Some(p) => p,
            None => return Vec::new(),
        };

        // Build required set
        let required_set: std::collections::HashSet<_> = schema_map
            .get("required")
            .and_then(|r| r.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .collect()
            })
            .unwrap_or_default();

        let mut result = Vec::new();
        for (key, prop) in properties {
            let prop_map = match prop.as_object() {
                Some(m) => m,
                None => continue,
            };

            let json_type = prop_map
                .get("type")
                .and_then(|t| t.as_str())
                .unwrap_or("string");

            let mut info = ParamInfo::new(key, Self::json_type_to_type_hint(json_type))
                .with_required(required_set.contains(key.as_str()));

            if let Some(desc) = prop_map.get("description").and_then(|d| d.as_str()) {
                info = info.with_description(desc);
            }

            result.push(info);
        }

        result
    }

    /// Build a call example string from parameter info
    pub fn build_call_example(params: &[ParamInfo]) -> String {
        if params.is_empty() {
            return String::new();
        }

        params
            .iter()
            .map(|p| format!("{}:{}={{", p.name, p.param_type))
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// Tool matching service
pub struct ToolMatcher;

impl ToolMatcher {
    /// Find a tool by name in a list of tools
    pub fn find_tool_by_name<'a>(tools: &'a [Tool], name: &str) -> Option<&'a Tool> {
        tools.iter().find(|t| t.name == name)
    }

    /// Search tools by query (matches name or description)
    pub fn search_tools<'a>(tools: &'a [Tool], query: &str) -> Vec<&'a Tool> {
        let query_lower = query.to_lowercase();
        tools
            .iter()
            .filter(|t| {
                let name_match = t.name.to_lowercase().contains(&query_lower);
                let desc_match = t
                    .description
                    .as_ref()
                    .map(|d| d.to_lowercase().contains(&query_lower))
                    .unwrap_or(false);
                name_match || desc_match
            })
            .collect()
    }

    /// Suggest tools based on task keywords
    pub fn suggest_tools<'a>(tools: &'a [Tool], task: &str) -> Vec<(&'a Tool, Vec<String>)> {
        let task_lower = task.to_lowercase();
        let task_words: Vec<_> = task_lower
            .split_whitespace()
            .filter(|w| w.len() >= 3)
            .collect();

        let mut results = Vec::new();
        for tool in tools {
            let name_lower = tool.name.to_lowercase();
            let desc_lower = tool
                .description
                .as_ref()
                .map(|d| d.to_lowercase())
                .unwrap_or_default();

            let matched_words: Vec<String> = task_words
                .iter()
                .filter(|&&word| name_lower.contains(word) || desc_lower.contains(word))
                .map(|&w| w.to_string())
                .collect();

            if !matched_words.is_empty() {
                results.push((tool, matched_words));
            }
        }

        results
    }
}

/// Environment variable resolver
pub struct EnvResolver;

impl EnvResolver {
    /// Resolve environment variables in a string (${VAR} or $VAR syntax)
    pub fn resolve(input: &str) -> String {
        let re = regex::Regex::new(r"\$\{([^}]+)\}|\$([A-Za-z_][A-Za-z0-9_]*)").unwrap();

        re.replace_all(input, |caps: &regex::Captures| {
            let var_name = caps
                .get(1)
                .or_else(|| caps.get(2))
                .map(|m| m.as_str())
                .unwrap_or("");

            std::env::var(var_name).unwrap_or_else(|_| caps.get(0).unwrap().as_str().to_string())
        })
        .into_owned()
    }

    /// Resolve environment variables in header values
    pub fn resolve_headers(
        headers: &HashMap<String, String>,
    ) -> HashMap<String, String> {
        headers
            .iter()
            .map(|(k, v)| (k.clone(), Self::resolve(v)))
            .collect()
    }
}

/// Retry service with exponential backoff
pub struct RetryService {
    max_retries: u32,
    base_delay_ms: u64,
}

impl RetryService {
    pub fn new(max_retries: u32, base_delay_ms: u64) -> Self {
        Self {
            max_retries,
            base_delay_ms,
        }
    }

    pub async fn execute<T, F, Fut>(&self, mut operation: F) -> MCPResult<T>
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = MCPResult<T>>,
    {
        let mut last_error = None;

        for attempt in 1..=self.max_retries {
            match operation().await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    if !e.is_retryable() || attempt == self.max_retries {
                        return Err(e);
                    }
                    last_error = Some(e);

                    // Exponential backoff
                    let delay = self.base_delay_ms * attempt as u64;
                    tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
                }
            }
        }

        Err(last_error.unwrap_or_else(|| {
            MCPError::internal_error("Retry loop exited without result or error")
        }))
    }
}

// Helper extension trait for creating internal errors
trait InternalErrorExt {
    fn internal_error(msg: impl Into<String>) -> Self;
}

impl InternalErrorExt for MCPError {
    fn internal_error(msg: impl Into<String>) -> Self {
        MCPError::new(super::errors::ErrorCode::InternalError, msg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_kv_args() {
        let args = vec![
            "name=John".to_string(),
            "age:number=30".to_string(),
            "enabled:bool=true".to_string(),
        ];

        let result = ParamParser::parse_kv_args(&args).unwrap();
        assert_eq!(result.get("name"), Some(&serde_json::json!("John")));
        assert_eq!(result.get("age"), Some(&serde_json::json!(30)));
        assert_eq!(result.get("enabled"), Some(&serde_json::json!(true)));
    }

    #[test]
    fn test_parse_kv_args_invalid() {
        let args = vec!["invalid".to_string()];
        assert!(ParamParser::parse_kv_args(&args).is_err());
    }

    #[test]
    fn test_parse_yaml() {
        let yaml = "name: John\nage: 30";
        let result = ParamParser::parse_yaml(yaml).unwrap();
        assert_eq!(result.get("name"), Some(&serde_json::json!("John")));
        assert_eq!(result.get("age"), Some(&serde_json::json!(30)));
    }

    #[test]
    fn test_schema_formatter() {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "name": { "type": "string", "description": "User name" },
                "age": { "type": "integer" }
            },
            "required": ["name"]
        });

        let formatted = SchemaFormatter::format_input_schema(&schema);
        assert!(formatted.is_some());
        let formatted = formatted.unwrap();
        assert_eq!(formatted.len(), 2);

        let required = SchemaFormatter::get_required_params(&schema);
        assert_eq!(required, vec!["name"]);

        let params = SchemaFormatter::get_param_info_list(&schema);
        assert_eq!(params.len(), 2);
    }

    #[test]
    fn test_tool_matcher() {
        let tools = vec![
            Tool::new("search")
                .with_description("Search for items"),
            Tool::new("list")
                .with_description("List all items"),
        ];

        let found = ToolMatcher::find_tool_by_name(&tools, "search");
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "search");

        let search_results = ToolMatcher::search_tools(&tools, "list");
        assert_eq!(search_results.len(), 1);

        let suggestions = ToolMatcher::suggest_tools(&tools, "search items");
        assert!(!suggestions.is_empty());
    }

    #[test]
    fn test_env_resolver() {
        // SAFETY: This is test code and we're only setting a test variable
        unsafe { std::env::set_var("TEST_VAR", "test_value"); }
        
        assert_eq!(EnvResolver::resolve("$TEST_VAR"), "test_value");
        assert_eq!(EnvResolver::resolve("${TEST_VAR}"), "test_value");
        assert_eq!(EnvResolver::resolve("prefix-$TEST_VAR-suffix"), "prefix-test_value-suffix");
        assert_eq!(EnvResolver::resolve("$NONEXISTENT"), "$NONEXISTENT");
    }
}
