//! CLI argument parsing with clap

use clap::{Parser, Subcommand, ValueEnum};

/// Output format options
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum OutputFormat {
    Json,
    Yaml,
    Text,
}

/// rmcp - A CLI-native MCP client
#[derive(Debug, Parser)]
#[command(
    name = "rmcp",
    about = "A CLI-native MCP client that loads tool schemas on-demand",
    long_about = r#"
rmcp - A CLI-native MCP client that loads tool schemas on-demand

Config file search paths (by priority):
  1. ~/.config/modelcontextprotocol/mcp.json
  2. ~/.config/mcp/config.json
  3. ./mcp.json
  4. ./.mcp/config.json
  5. /etc/mcp/config.json

Usage examples:

  # List all configured servers
  rmcp

  # List tools for a specific server
  rmcp openDeepWiki

  # View details of a specific tool
  rmcp openDeepWiki list_repositories

  # Call a tool (args format: key=value or key:type=value)
  rmcp openDeepWiki list_repositories limit=3

  # Call a tool with streaming output
  rmcp --stream openDeepWiki list_repositories limit=3

  # Call with YAML parameters (inline)
  rmcp openDeepWiki list_repositories --yaml 'limit: 3 repoOwner: github'

  # Call with YAML from file
  rmcp openDeepWiki create_issue -f issue.yaml

  # Pipe YAML to stdin
  cat issue.yaml | rmcp openDeepWiki create_issue

  # Output in different formats
  rmcp --output yaml openDeepWiki list_repositories
  rmcp --output text openDeepWiki list_repositories
"#,
    version = env!("CARGO_PKG_VERSION")
)]
pub struct Cli {
    /// Output format (json, yaml, text)
    #[arg(short, long, value_enum, default_value = "json")]
    pub output: OutputFormat,

    /// Enable streaming output
    #[arg(short, long, default_value = "false")]
    pub stream: bool,

    /// YAML parameters (inline)
    #[arg(short = 'y', long = "yaml")]
    pub yaml_params: Option<String>,

    /// YAML file with parameters
    #[arg(short = 'f', long = "file")]
    pub yaml_file: Option<String>,

    /// Subcommand
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Positional arguments for server/tool/params
    #[arg(value_name = "ARGS")]
    pub args: Vec<String>,
}

/// CLI subcommands
#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Start interactive REPL mode
    #[command(name = "interactive")]
    Interactive,
}

/// Parsed command arguments
#[derive(Debug)]
pub enum ParsedCommand {
    /// List all servers
    ListServers,
    /// Show server info
    ShowServerInfo { server: String },
    /// Show tool info
    ShowToolInfo { server: String, tool: String },
    /// Call a tool
    CallTool {
        server: String,
        tool: String,
        args: Vec<String>,
    },
    /// Interactive mode
    Interactive,
}

impl Cli {
    /// Parse and determine the command based on arguments
    pub fn parse_command(&self) -> ParsedCommand {
        // Check for interactive subcommand
        if let Some(Commands::Interactive) = &self.command {
            return ParsedCommand::Interactive;
        }

        // Check for interactive mode via args
        if self.args.len() == 1 && self.args[0] == "interactive" {
            return ParsedCommand::Interactive;
        }

        // Parse positional args
        match self.args.len() {
            0 => ParsedCommand::ListServers,
            1 => ParsedCommand::ShowServerInfo {
                server: self.args[0].clone(),
            },
            2 => {
                // Check if we have YAML input mode
                let has_yaml_input = self.yaml_params.is_some()
                    || self.yaml_file.is_some()
                    || (is_piped_input() && self.args.len() <= 2);

                if has_yaml_input {
                    // This is actually a call with YAML params
                    ParsedCommand::CallTool {
                        server: self.args[0].clone(),
                        tool: self.args[1].clone(),
                        args: vec![],
                    }
                } else {
                    ParsedCommand::ShowToolInfo {
                        server: self.args[0].clone(),
                        tool: self.args[1].clone(),
                    }
                }
            }
            _ => ParsedCommand::CallTool {
                server: self.args[0].clone(),
                tool: self.args[1].clone(),
                args: self.args[2..].to_vec(),
            },
        }
    }
}

/// Check if stdin has piped input
fn is_piped_input() -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::FileTypeExt;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_default() {
        let cli = Cli::parse_from(["rmcp"]);
        assert!(cli.yaml_params.is_none());
        assert!(cli.yaml_file.is_none());
        assert!(!cli.stream);
    }

    #[test]
    fn test_parse_command_list_servers() {
        let cli = Cli::parse_from(["rmcp"]);
        match cli.parse_command() {
            ParsedCommand::ListServers => {}
            _ => panic!("Expected ListServers"),
        }
    }

    #[test]
    fn test_parse_command_show_server() {
        let cli = Cli::parse_from(["rmcp", "myserver"]);
        match cli.parse_command() {
            ParsedCommand::ShowServerInfo { server } => {
                assert_eq!(server, "myserver");
            }
            _ => panic!("Expected ShowServerInfo"),
        }
    }
}
