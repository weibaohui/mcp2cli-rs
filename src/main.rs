//! rmcp - A CLI-native MCP client
//!
//! This is a Rust implementation of mcp2cli, following DDD architecture.

mod application;
mod domain;
mod infrastructure;
mod presentation;

use application::{
    dto::OutputFormat as AppOutputFormat,
    ports::{ConfigPort, MCPClientPort, OutputPort, ParamParserPort},
    use_cases::*,
};
use clap::Parser;
use infrastructure::{
    CliParamParser, ConsoleOutput, FileConfigRepository, MCPClientImpl,
};
use presentation::{
    cli::{Cli, Commands, OutputFormat, ParsedCommand},
    commands::CommandExecutor,
    interactive::InteractiveREPL,
};
use std::sync::Arc;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[tokio::main]
async fn main() {
    // Initialize tracing for logging
    tracing_subscriber::fmt::init();

    // Parse CLI arguments
    let cli = Cli::parse();

    // Create infrastructure components
    let config_repo = Arc::new(FileConfigRepository::new());
    let client = Arc::new(tokio::sync::Mutex::new(MCPClientImpl::new()));

    // Convert output format
    let output_format = match cli.output {
        OutputFormat::Json => AppOutputFormat::Json,
        OutputFormat::Yaml => AppOutputFormat::Yaml,
        OutputFormat::Text => AppOutputFormat::Text,
    };
    let output = Arc::new(ConsoleOutput::new(output_format));
    let param_parser = Arc::new(CliParamParser::new());

    // Create command executor
    let executor = CommandExecutor::new(
        config_repo.clone(),
        client.clone(),
        output.clone(),
        param_parser.clone(),
    );

    // Parse and execute command
    let command = cli.parse_command();

    match command {
        ParsedCommand::Interactive => {
            let mut repl = InteractiveREPL::new(executor);
            if let Err(e) = repl.run().await {
                eprintln!("Interactive mode error: {}", e);
                std::process::exit(1);
            }
        }
        _ => {
            if let Err(e) = execute_command(&cli, command, &executor).await {
                presentation::commands::handle_error(&*output, &e);
                std::process::exit(1);
            }
        }
    }
}

async fn execute_command<CP, CP2, OP, PP>(
    cli: &Cli,
    command: ParsedCommand,
    executor: &CommandExecutor<CP, CP2, OP, PP>,
) -> crate::domain::errors::MCPResult<()>
where
    CP: ConfigPort,
    CP2: MCPClientPort,
    OP: OutputPort,
    PP: ParamParserPort,
{
    match command {
        ParsedCommand::ListServers => {
            executor.list_servers().await?;
        }
        ParsedCommand::ShowServerInfo { server } => {
            executor.show_server_info(&server).await?;
        }
        ParsedCommand::ShowToolInfo { server, tool } => {
            executor.show_tool_info(&server, &tool).await?;
        }
        ParsedCommand::CallTool {
            server,
            tool,
            args,
        } => {
            executor
                .call_tool(
                    &server,
                    &tool,
                    args,
                    cli.yaml_params.clone(),
                    cli.yaml_file.clone(),
                    cli.stream,
                )
                .await?;
        }
        ParsedCommand::Interactive => {
            // Should not reach here
            unreachable!()
        }
    }

    Ok(())
}
