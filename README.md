# rmcp

[![Rust](https://img.shields.io/badge/rust-1.85%2B-blue.svg)](https://www.rust-lang.org)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A CLI-native MCP (Model Context Protocol) client that loads tool schemas on-demand and resolves discover-then-call into a single invocation — keeping tool definitions out of your context window.

This is a Rust implementation of [mcp2cli](https://github.com/weibaohui/mcp2cli), following **DDD (Domain-Driven Design)** architecture principles.

## Features

- 🔧 **Multi-Transport Support**: SSE, Streamable HTTP, and Stdio transports
- 🔐 **OAuth 2.1 + PKCE**: Built-in authentication support
- 📝 **Flexible Parameter Input**: Key=value, YAML inline, YAML file, or stdin pipe
- 🖥️ **Interactive REPL Mode**: Explore and call tools interactively
- 📤 **Multiple Output Formats**: JSON, YAML, and compact text
- 🔍 **Tool Discovery**: List tools, show tool details with parameter schemas
- ⚡ **Streaming Support**: Stream text content for long-running tools
- 🎯 **Token Efficient**: 80-90% fewer tokens compared to direct MCP usage

## Architecture

This project follows **Domain-Driven Design (DDD)** architecture:

```
rmcp/
├── src/
│   ├── domain/           # Core business logic
│   │   ├── entities/     # Domain entities (Server, Tool, etc.)
│   │   ├── value_objects/# Value objects (TransportType, etc.)
│   │   ├── errors/       # Domain errors
│   │   ├── services/     # Domain services
│   │   └── repositories/ # Repository interfaces
│   ├── application/      # Application layer
│   │   ├── dto/          # Data Transfer Objects
│   │   ├── ports/        # Interface definitions
│   │   └── use_cases/    # Use cases / Interactors
│   ├── infrastructure/   # Infrastructure implementations
│   │   ├── config/       # Configuration repository
│   │   ├── transport/    # HTTP/Stdio transports
│   │   ├── mcp_client/   # MCP client implementation
│   │   ├── oauth/        # OAuth service
│   │   ├── output/       # Output formatting
│   │   └── param_parser/ # Parameter parsing
│   └── presentation/     # CLI presentation layer
│       ├── cli/          # CLI argument parsing
│       ├── commands/     # Command handlers
│       └── interactive/  # REPL mode
```

## Installation

### From Source

```bash
git clone https://github.com/weibaohui/rmcp.git
cd rmcp
cargo build --release

# Binary will be at target/release/rmcp
cp target/release/rmcp /usr/local/bin/
```

### Prerequisites

- Rust 1.85 or higher
- For OAuth support: A web browser for authentication flow

## Configuration

Create a configuration file at one of these locations (in order of priority):

1. `~/.config/modelcontextprotocol/mcp.json`
2. `~/.config/mcp/config.json`
3. `./mcp.json`
4. `./.mcp/config.json`
5. `/etc/mcp/config.json`

### Example Configuration

```json
{
  "mcpServers": {
    "openDeepWiki": {
      "url": "https://opendeepwiki.k8m.site/mcp/streamable"
    },
    "local-server": {
      "command": "mcp-server",
      "args": ["--stdio"]
    },
    "secure-server": {
      "url": "https://api.example.com/mcp",
      "headers": {
        "Authorization": "Bearer ${API_TOKEN}"
      }
    },
    "oauth-server": {
      "url": "https://api.example.com/mcp",
      "auth": {
        "oauth": {
          "clientId": "your-client-id",
          "authorizationURL": "https://auth.example.com/authorize",
          "tokenURL": "https://auth.example.com/token",
          "scopes": "read write"
        }
      }
    }
  }
}
```

## Usage

### List Servers

```bash
rmcp
```

### List Tools on a Server

```bash
rmcp <server>
```

### Show Tool Details

```bash
rmcp <server> <tool>
```

### Call a Tool

```bash
# Simple key=value (string by default)
rmcp server tool name=John age=30

# Typed key:type=value
rmcp server tool name:string=John age:number=30 enabled:bool=true

# Inline YAML
rmcp server tool --yaml 'name: John details: {age: 30, city: NYC}'

# YAML from file
rmcp server tool -f params.yaml

# Pipe YAML to stdin
cat params.yaml | rmcp server tool
```

### Output Formats

```bash
# JSON (default)
rmcp server tool

# YAML
rmcp --output yaml server tool

# Compact text
rmcp --output text server tool
```

### Interactive Mode

```bash
rmcp interactive
```

Or simply:

```bash
rmcp interactive
```

Interactive commands:
- `servers` - List all configured servers
- `use <server>` - Set default server
- `tool [name]` - List tools or show tool details
- `help` - Show help
- `exit` - Exit interactive mode

## Transport Types

### Streamable HTTP (Default)

```json
{
  "mcpServers": {
    "my-server": {
      "url": "https://example.com/mcp"
    }
  }
}
```

### SSE (Server-Sent Events)

```json
{
  "mcpServers": {
    "sse-server": {
      "url": "https://example.com/mcp/sse",
      "transport": "sse"
    }
  }
}
```

### Stdio (Command-based)

```json
{
  "mcpServers": {
    "stdio-server": {
      "command": "mcp-server",
      "args": ["--stdio"],
      "env": {
        "API_KEY": "secret"
      }
    }
  }
}
```

## Environment Variables

Environment variables can be used in configuration:

```json
{
  "mcpServers": {
    "my-server": {
      "url": "https://api.example.com",
      "headers": {
        "Authorization": "Bearer ${API_TOKEN}"
      }
    }
  }
}
```

Both `${VAR}` and `$VAR` syntax are supported.

## Comparison with Direct MCP

| Scenario | Direct MCP | rmcp | Saving |
|----------|-----------|------|--------|
| Discover 1 tool | ~500 tokens | ~100 tokens | 80% |
| Call 1 tool | ~300 tokens | ~130 tokens | 57% |
| 10-tool server in context | ~10,000 tokens | 0 tokens | 100% |
| Full workflow (discover + call) | ~2,000 tokens | ~230 tokens | 89% |

## Development

### Build

```bash
cargo build --release
```

### Test

```bash
cargo test
```

### Run with Logging

```bash
RUST_LOG=debug cargo run
```

## License

MIT License - see [LICENSE](LICENSE) for details.

## Acknowledgments

This project is a Rust reimplementation of [mcp2cli](https://github.com/weibaohui/mcp2cli) by [weibaohui](https://github.com/weibaohui).
