# rmcp

> A CLI-native MCP client that loads tool schemas on-demand and keeps them out of your context window.

[![Rust](https://img.shields.io/badge/rust-1.85%2B-blue.svg)](https://www.rust-lang.org)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![GitHub stars](https://img.shields.io/github/stars/weibaohui/rmcp?style=social)](https://github.com/weibaohui/rmcp)

`rmcp` is a Rust implementation of [mcp2cli](https://github.com/weibaohui/mcp2cli), providing a lightweight, token-efficient way to interact with [Model Context Protocol](https://modelcontextprotocol.io/) servers directly from your terminal.

## Why rmcp?

When using MCP servers directly, tool schemas consume valuable context tokens — even when you're not using most of them. `rmcp` solves this by **discover-then-call** in a single invocation, loading tool definitions on-demand and discarding them after use.

```
┌─────────────────────────────────────────────────────────────┐
│  Direct MCP Usage                                           │
│  ┌─────────────┐                                            │
│  │ Tool Schema │  ← Always in context (~500-1000 tokens)     │
│  │ Tool Schema │  ← Always in context                       │
│  │ Tool Schema │  ← Always in context                       │
│  │ Tool Schema │  ← Always in context                       │
│  └─────────────┘                                            │
│                           vs                                 │
│  rmcp Usage                                                 │
│  ┌─────────────┐                                            │
│  │ Call Tool  │  ← Schema loaded only when called (~100 tokens)│
│  └─────────────┘                                            │
└─────────────────────────────────────────────────────────────┘
```

| Scenario | Direct MCP | rmcp | Savings |
|----------|-----------|------|---------|
| Discover 1 tool | ~500 tokens | ~100 tokens | **80%** |
| Call 1 tool | ~300 tokens | ~130 tokens | **57%** |
| 10-tool server | ~10,000 tokens | 0 tokens | **100%** |
| Full workflow | ~2,000 tokens | ~230 tokens | **89%** |

## Features

| Feature | Description |
|---------|-------------|
| 🚀 **Multi-Transport** | SSE, Streamable HTTP, and Stdio transports |
| 🔐 **OAuth 2.1 + PKCE** | Built-in authentication support |
| 📝 **Flexible Params** | `key=value`, inline YAML, YAML file, or stdin pipe |
| 🖥️ **Interactive REPL** | Explore servers and call tools interactively |
| 📤 **Multiple Outputs** | JSON, YAML, and human-readable text formats |
| ⚡ **Streaming** | Stream text content for long-running operations |
| 🎯 **Token Efficient** | 80-90% fewer tokens than direct MCP usage |

## Quick Start

### Installation

```bash
# Build from source
git clone https://github.com/weibaohui/rmcp.git
cd rmcp
cargo build --release
sudo cp target/release/rmcp /usr/local/bin/
```

### Configuration

Create `~/.config/modelcontextprotocol/mcp.json`:

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
    }
  }
}
```

Configuration file search order (first found wins):
1. `~/.config/modelcontextprotocol/mcp.json`
2. `~/.config/mcp/config.json`
3. `./mcp.json`
4. `./.mcp/config.json`
5. `/etc/mcp/config.json`

### Usage

```bash
# List all configured servers
rmcp

# List tools on a server
rmcp openDeepWiki

# Show tool details
rmcp openDeepWiki list_repositories

# Call a tool with simple args (strings by default)
rmcp openDeepWiki list_repositories limit=3

# Call with typed arguments
rmcp openDeepWiki list_repositories limit:number=3 enabled:bool=true

# Call with inline YAML
rmcp openDeepWiki list_repositories --yaml 'limit: 3 repoOwner: github'

# Call with YAML from file
rmcp openDeepWiki create_issue -f issue.yaml

# Pipe YAML to stdin
cat issue.yaml | rmcp openDeepWiki create_issue

# Stream long-running tool output
rmcp --stream openDeepWiki long_running_tool

# Output in different formats
rmcp --output yaml openDeepWiki list_repositories
rmcp --output text openDeepWiki list_repositories
```

### Interactive Mode

```bash
rmcp interactive
```

```
MCP Interactive Mode
Type 'help' for available commands, 'exit' to quit

> servers                                    # List all servers
> use openDeepWiki                           # Set default server
> tool                                       # List tools on default server
> list_repositories limit=5                  # Call tool directly
> tool list_repositories                     # Show tool details
> exit
```

## Architecture

`rmcp` follows **Domain-Driven Design (DDD)** principles:

```
rmcp/src/
├── domain/           # Core business logic
│   ├── entities/     # Server, Tool, etc.
│   ├── value_objects/# TransportType, etc.
│   ├── errors/       # Domain errors
│   ├── services/     # Domain services
│   └── repositories/ # Repository traits
├── application/      # Application layer
│   ├── dto/          # Data Transfer Objects
│   ├── ports/        # Interface definitions
│   └── use_cases/    # Business use cases
├── infrastructure/   # External implementations
│   ├── config/       # File-based config
│   ├── transport/    # HTTP/Stdio transports
│   ├── mcp_client/   # MCP protocol client
│   ├── oauth/        # OAuth 2.1 + PKCE
│   ├── output/       # JSON/YAML/Text formatting
│   └── param_parser/ # CLI argument parsing
└── presentation/     # CLI interface
    ├── cli/          # Clap argument parsing
    ├── commands/     # Command handlers
    └── interactive/  # REPL implementation
```

## Transport Types

### Streamable HTTP (Default)
```json
{ "mcpServers": { "my-server": { "url": "https://example.com/mcp" } } }
```

### SSE (Server-Sent Events)
```json
{ "mcpServers": { "sse-server": { "url": "https://example.com/mcp/sse", "transport": "sse" } } }
```

### Stdio (Command-based)
```json
{
  "mcpServers": {
    "stdio-server": {
      "command": "mcp-server",
      "args": ["--stdio"],
      "env": { "API_KEY": "secret" }
    }
  }
}
```

### OAuth 2.1 + PKCE
```json
{
  "mcpServers": {
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

## Development

```bash
# Build
cargo build --release

# Run tests
cargo test

# Run with debug logging
RUST_LOG=debug cargo run -- openDeepWiki list_repositories

# Auto-fix warnings
cargo fix --bin rmcp -p rmcp
```

## License

MIT License - see [LICENSE](LICENSE) for details.

## Acknowledgments

- [mcp2cli](https://github.com/weibaohui/mcp2cli) - Original Go implementation by [weibaohui](https://github.com/weibaohui)
- [Model Context Protocol](https://modelcontextprotocol.io/) - The protocol specification
