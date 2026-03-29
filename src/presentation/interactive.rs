//! Interactive REPL mode

use crate::application::ports::{ConfigPort, MCPClientPort, OutputPort, ParamParserPort};
use crate::presentation::commands::CommandExecutor;
use reedline::{DefaultPrompt, Reedline, Signal};

/// Interactive REPL
pub struct InteractiveREPL<CP, CP2, OP, PP>
where
    CP: ConfigPort,
    CP2: MCPClientPort,
    OP: OutputPort,
    PP: ParamParserPort,
{
    executor: CommandExecutor<CP, CP2, OP, PP>,
    current_server: Option<String>,
}

impl<CP, CP2, OP, PP> InteractiveREPL<CP, CP2, OP, PP>
where
    CP: ConfigPort,
    CP2: MCPClientPort,
    OP: OutputPort,
    PP: ParamParserPort,
{
    pub fn new(executor: CommandExecutor<CP, CP2, OP, PP>) -> Self {
        Self {
            executor,
            current_server: None,
        }
    }

    pub async fn run(&mut self) -> anyhow::Result<()> {
        println!("MCP Interactive Mode");
        println!("Type 'help' for available commands, 'exit' to quit");
        println!();

        let mut line_editor = Reedline::create();
        let prompt = DefaultPrompt::default();

        loop {
            let sig = line_editor.read_line(&prompt);

            match sig {
                Ok(Signal::Success(line)) => {
                    let line = line.trim();
                    if line.is_empty() {
                        continue;
                    }

                    if let Err(e) = self.process_command(line).await {
                        eprintln!("Error: {}", e);
                    }
                }
                Ok(Signal::CtrlC) => {
                    println!("\nUse 'exit' to quit");
                }
                Ok(Signal::CtrlD) => {
                    println!("\nGoodbye!");
                    break;
                }
                Err(e) => {
                    eprintln!("Error: {}", e);
                    break;
                }
            }
        }

        Ok(())
    }

    async fn process_command(&mut self, input: &str) -> anyhow::Result<()> {
        let parts = parse_input(input);
        if parts.is_empty() {
            return Ok(());
        }

        match parts[0].as_str() {
            "exit" | "quit" | "q" => {
                println!("Goodbye!");
                std::process::exit(0);
            }
            "help" | "?" => {
                self.print_help();
            }
            "servers" | "list" => {
                if let Err(_) = self.executor.list_servers().await {
                    // Error is already handled by output port
                }
            }
            "use" => {
                if parts.len() < 2 {
                    println!("Usage: use <server>");
                } else {
                    self.current_server = Some(parts[1].clone());
                    println!("Default server set to: {}", parts[1]);
                }
            }
            "tool" | "tools" => {
                if parts.len() < 2 {
                    if let Some(server) = &self.current_server {
                        if let Err(_) = self.executor.show_server_info(server).await {
                            // Error handled by output port
                        }
                    } else {
                        println!("No default server. Use 'use <server>' or 'servers' to list available servers");
                    }
                } else {
                    if let Some(server) = &self.current_server {
                        if let Err(_) = self.executor.show_tool_info(server, &parts[1]).await {
                            // Error handled by output port
                        }
                    } else {
                        println!("No default server. Use 'use <server>' first");
                    }
                }
            }
            _ => {
                self.handle_tool_call(&parts).await?;
            }
        }

        Ok(())
    }

    async fn handle_tool_call(&mut self, parts: &[String]) -> anyhow::Result<()> {
        let (server, tool, args) = if parts.len() >= 2 {
            if self.current_server.is_some() {
                let server = self.current_server.clone().unwrap();
                let tool = parts[0].clone();
                let args: Vec<String> = parts[1..].to_vec();
                (server, tool, args)
            } else {
                println!("No default server. Use 'use <server>' first");
                return Ok(());
            }
        } else if parts.len() == 1 && self.current_server.is_some() {
            let server = self.current_server.clone().unwrap();
            let tool = parts[0].clone();
            (server, tool, vec![])
        } else {
            println!("Invalid command. Type 'help' for available commands");
            return Ok(());
        };

        if let Err(_) = self
            .executor
            .call_tool(&server, &tool, args, None, None, false)
            .await
        {
            // Error handled by output port
        }

        Ok(())
    }

    fn print_help(&self) {
        println!("Available commands:");
        println!("  <server> <tool> [args...]  Call a tool on a specific server");
        println!("  use <server>               Set default server for short commands");
        println!("  servers                    List all configured servers");
        println!("  tool [name]                List tools on default server or show tool details");
        println!("  help                       Show this help");
        println!("  exit                       Exit interactive mode");
        println!();
        println!("Shortcut: If no server prefix, uses the default server (set with 'use')");
        println!();
        println!("Examples:");
        println!("  openDeepWiki list_repositories limit=3");
        println!("  use openDeepWiki");
        println!("  list_repositories limit=5");
    }
}

/// Parse input line respecting quotes
fn parse_input(input: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut current = String::new();
    let mut in_quote = false;

    for ch in input.chars() {
        match ch {
            '"' => {
                in_quote = !in_quote;
            }
            ' ' | '\t' => {
                if !in_quote {
                    if !current.is_empty() {
                        result.push(current.clone());
                        current.clear();
                    }
                } else {
                    current.push(ch);
                }
            }
            _ => {
                current.push(ch);
            }
        }
    }

    if !current.is_empty() {
        result.push(current);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_input() {
        let input = "server tool arg1 arg2";
        let parts = parse_input(input);
        assert_eq!(parts.len(), 4);
        assert_eq!(parts[0], "server");

        let input_quoted = r#"server tool "arg with spaces""#;
        let parts = parse_input(input_quoted);
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[2], "arg with spaces");
    }
}
