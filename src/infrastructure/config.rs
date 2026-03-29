//! Configuration repository implementation

use crate::application::ports::ConfigPort;
use crate::domain::{
    entities::{MCPConfig, ServerConfig},
    errors::{MCPError, MCPResult},
};
use async_trait::async_trait;
use std::collections::HashMap;
use std::path::PathBuf;

/// Configuration repository implementation
pub struct FileConfigRepository {
    search_paths: Vec<String>,
}

impl FileConfigRepository {
    pub fn new() -> Self {
        Self {
            search_paths: Self::get_default_search_paths(),
        }
    }

    pub fn with_search_paths(paths: Vec<String>) -> Self {
        Self { search_paths: paths }
    }

    /// Get default config search paths based on platform
    fn get_default_search_paths() -> Vec<String> {
        #[cfg(target_os = "windows")]
        {
            Self::get_windows_paths()
        }
        #[cfg(not(target_os = "windows"))]
        {
            Self::get_unix_paths()
        }
    }

    #[cfg(not(target_os = "windows"))]
    fn get_unix_paths() -> Vec<String> {
        let mut paths = Vec::new();

        if let Ok(home) = std::env::var("HOME") {
            paths.push(format!("{}/.config/modelcontextprotocol/mcp.json", home));
            paths.push(format!("{}/.config/mcp/config.json", home));
        }

        // Current directory paths
        paths.push("./mcp.json".to_string());
        paths.push("./.mcp/config.json".to_string());

        // System-level path
        paths.push("/etc/mcp/config.json".to_string());

        paths
    }

    #[cfg(target_os = "windows")]
    fn get_windows_paths() -> Vec<String> {
        let mut paths = Vec::new();

        if let Ok(app_data) = std::env::var("APPDATA") {
            paths.push(format!("{}\\modelcontextprotocol\\mcp.json", app_data));
            paths.push(format!("{}\\mcp\\config.json", app_data));
        }

        if let Ok(user_profile) = std::env::var("USERPROFILE") {
            paths.push(format!("{}\\.mcp\\config.json", user_profile));
        }

        // Current directory paths
        paths.push(".\\mcp.json".to_string());
        paths.push(".\\.mcp\\config.json".to_string());

        // System-level path
        if let Ok(program_data) = std::env::var("ProgramData") {
            paths.push(format!("{}\\mcp\\config.json", program_data));
        }

        paths
    }

    /// Expand ~ in paths
    fn expand_home(path: &str) -> String {
        if !path.starts_with("~") {
            return path.to_string();
        }

        if let Ok(home) = std::env::var("HOME") {
            if path == "~" {
                return home;
            }
            format!("{}{}", home, &path[1..])
        } else {
            path.to_string()
        }
    }

    /// Load configuration from specified paths
    async fn load_config_from_paths(&self, paths: Vec<String>) -> MCPResult<(MCPConfig, Vec<String>)> {
        use std::path::PathBuf;
        use crate::domain::errors::MCPError;
        
        let mut result = MCPConfig::new();
        let mut loaded_paths = Vec::new();

        for path in paths {
            let expanded = Self::expand_home(&path);
            let path_buf = PathBuf::from(&expanded);

            if !path_buf.exists() {
                continue;
            }

            let data = match tokio::fs::read_to_string(&expanded).await {
                Ok(d) => d,
                Err(_) => continue,
            };

            let cfg: MCPConfig = match serde_json::from_str(&data) {
                Ok(c) => c,
                Err(_) => continue,
            };

            // Merge servers
            for (name, server_cfg) in cfg.mcp_servers {
                result.mcp_servers.insert(name, server_cfg);
            }

            loaded_paths.push(expanded);
        }

        if loaded_paths.is_empty() {
            return Err(MCPError::config_not_found("no config files found"));
        }

        Ok((result, loaded_paths))
    }
}

#[async_trait]
impl ConfigPort for FileConfigRepository {
    async fn load_config(&self) -> MCPResult<(MCPConfig, Vec<String>)> {
        self.load_config_from_paths(self.search_paths.clone()).await
    }



    fn get_server_config(&self, config: &MCPConfig, name: &str) -> Option<ServerConfig> {
        config.mcp_servers.get(name).cloned()
    }

    fn list_servers(&self, config: &MCPConfig) -> Vec<String> {
        config.mcp_servers.keys().cloned().collect()
    }
}

impl Default for FileConfigRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_load_config() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("mcp.json");

        let config = r#"{
            "mcpServers": {
                "test-server": {
                    "url": "http://localhost:8080",
                    "timeout": 5000
                }
            }
        }"#;

        tokio::fs::write(&config_path, config).await.unwrap();

        let repo = FileConfigRepository::with_search_paths(vec![config_path.to_string_lossy().to_string()]);
        let (cfg, paths) = repo.load_config().await.unwrap();

        assert_eq!(paths.len(), 1);
        assert!(cfg.mcp_servers.contains_key("test-server"));
        
        let server = cfg.mcp_servers.get("test-server").unwrap();
        assert_eq!(server.url, Some("http://localhost:8080".to_string()));
    }

    #[tokio::test]
    async fn test_load_config_not_found() {
        let repo = FileConfigRepository::with_search_paths(vec!["/nonexistent/path.json".to_string()]);
        let result = repo.load_config().await;
        assert!(result.is_err());
    }

    #[test]
    fn test_expand_home() {
        // SAFETY: This is test code and we're only setting a test variable
        unsafe { std::env::set_var("HOME", "/home/testuser"); }
        assert_eq!(
            FileConfigRepository::expand_home("~/.config/mcp.json"),
            "/home/testuser/.config/mcp.json"
        );
        assert_eq!(
            FileConfigRepository::expand_home("/absolute/path"),
            "/absolute/path"
        );
    }
}
