//! Output formatting implementation

use crate::application::ports::OutputPort;
use std::io::Write;

/// Console output implementation
pub struct ConsoleOutput {
    format: crate::application::dto::OutputFormat,
}

impl ConsoleOutput {
    pub fn new(format: crate::application::dto::OutputFormat) -> Self {
        Self { format }
    }

    pub fn with_format(mut self, format: crate::application::dto::OutputFormat) -> Self {
        self.format = format;
        self
    }
}

impl Default for ConsoleOutput {
    fn default() -> Self {
        Self::new(crate::application::dto::OutputFormat::Json)
    }
}

impl OutputPort for ConsoleOutput {
    fn output_json<T: serde::Serialize>(&self, data: &T) {
        match serde_json::to_string_pretty(data) {
            Ok(json) => println!("{}", json),
            Err(e) => eprintln!("Failed to serialize JSON: {}", e),
        }
    }

    fn output_yaml<T: serde::Serialize>(&self, data: &T) {
        match serde_yaml::to_string(data) {
            Ok(yaml) => println!("{}", yaml),
            Err(e) => eprintln!("Failed to serialize YAML: {}", e),
        }
    }

    fn output_text<T: serde::Serialize>(&self, data: &T) {
        // Output as compact JSON for text mode
        match serde_json::to_string(data) {
            Ok(json) => println!("{}", json),
            Err(e) => eprintln!("Failed to serialize: {}", e),
        }
    }

    fn output_error(&self, code: &str, message: &str, details: Option<serde_json::Value>) {
        let error_obj = serde_json::json!({
            "success": false,
            "error": {
                "code": code,
                "message": message,
                "details": details
            },
            "meta": {
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "version": env!("CARGO_PKG_VERSION")
            }
        });

        match self.format {
            crate::application::dto::OutputFormat::Yaml => {
                if let Ok(yaml) = serde_yaml::to_string(&error_obj) {
                    eprintln!("{}", yaml);
                }
            }
            _ => {
                if let Ok(json) = serde_json::to_string_pretty(&error_obj) {
                    eprintln!("{}", json);
                }
            }
        }
    }

    fn output_stream(&self, content: &str) {
        print!("{}", content);
        std::io::stdout().flush().ok();
    }

    fn flush(&self) {
        std::io::stdout().flush().ok();
    }
}

/// Output handler with format switching
pub struct OutputHandler {
    format: crate::application::dto::OutputFormat,
}

impl OutputHandler {
    pub fn new() -> Self {
        Self {
            format: crate::application::dto::OutputFormat::Json,
        }
    }

    pub fn with_format(format: crate::application::dto::OutputFormat) -> impl OutputPort {
        ConsoleOutput::new(format)
    }

    pub fn output<T: serde::Serialize>(&self, data: &T) {
        let output = ConsoleOutput::new(self.format);
        match self.format {
            crate::application::dto::OutputFormat::Json => output.output_json(data),
            crate::application::dto::OutputFormat::Yaml => output.output_yaml(data),
            crate::application::dto::OutputFormat::Text => output.output_text(data),
        }
    }
}

impl Default for OutputHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::dto::OutputFormat;

    #[test]
    fn test_console_output_new() {
        let output = ConsoleOutput::new(OutputFormat::Json);
        // Just verify it doesn't panic
    }

    #[test]
    fn test_output_json() {
        let output = ConsoleOutput::new(OutputFormat::Json);
        let data = serde_json::json!({"test": "value"});
        output.output_json(&data);
        // Output goes to stdout, hard to test directly
    }
}
