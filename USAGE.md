# rmcp 使用说明

## 1. 安装

### 从源码构建

```bash
# 克隆仓库
git clone https://github.com/weibaohui/rmcp.git
cd rmcp

# 构建发布版本
cargo build --release

# 复制到系统路径（可选）
cp target/release/rmcp /usr/local/bin/
# 或
cp target/release/rmcp ~/bin/
```

### 验证安装

```bash
rmcp --version
# 输出: rmcp 0.4.0
```

## 2. 配置文件

rmcp 会自动按以下顺序查找配置文件：

1. `~/.config/modelcontextprotocol/mcp.json`
2. `~/.config/mcp/config.json`
3. `./mcp.json` （当前目录）
4. `./.mcp/config.json`
5. `/etc/mcp/config.json`

### 创建配置

```bash
# 创建配置目录
mkdir -p ~/.config/mcp

# 创建配置文件
cat > ~/.config/mcp/config.json << 'EOF'
{
  "mcpServers": {
    "openDeepWiki": {
      "url": "https://opendeepwiki.k8m.site/mcp/streamable"
    }
  }
}
EOF
```

### 配置示例

#### HTTP Streamable 服务器
```json
{
  "mcpServers": {
    "my-api": {
      "url": "https://api.example.com/mcp"
    }
  }
}
```

#### SSE 服务器
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

#### Stdio 服务器（本地命令）
```json
{
  "mcpServers": {
    "local-server": {
      "command": "mcp-server",
      "args": ["--stdio"],
      "env": {
        "API_KEY": "your-api-key"
      }
    }
  }
}
```

#### 带认证的服务器
```json
{
  "mcpServers": {
    "secure-server": {
      "url": "https://api.example.com/mcp",
      "headers": {
        "Authorization": "Bearer ${API_TOKEN}"
      }
    }
  }
}
```

> 注：支持 `${VAR}` 或 `$VAR` 语法引用环境变量

## 3. 基本使用

### 3.1 列出所有服务器

```bash
rmcp
```

输出示例：
```json
{
  "success": true,
  "data": {
    "configFiles": [
      "/Users/username/.config/mcp/config.json"
    ],
    "servers": [
      {
        "name": "openDeepWiki",
        "transport": "streamable",
        "url": "https://opendeepwiki.k8m.site/mcp/streamable"
      }
    ]
  },
  "meta": {
    "timestamp": "2026-03-29T12:00:00Z",
    "version": "0.4.0"
  }
}
```

### 3.2 列出服务器上的所有工具

```bash
rmcp <服务器名>
```

示例：
```bash
rmcp openDeepWiki
```

### 3.3 查看工具详情

```bash
rmcp <服务器名> <工具名>
```

示例：
```bash
rmcp openDeepWiki list_repositories
```

输出示例：
```json
{
  "success": true,
  "data": {
    "configFiles": ["/Users/username/.config/mcp/config.json"],
    "server": "openDeepWiki",
    "tool": {
      "name": "list_repositories",
      "description": "List all repositories",
      "required": "repoOwner",
      "param_format": "key:type=value (type: string/number/bool)",
      "param_example": [
        "repoOwner:string={value}",
        "limit:number={value}"
      ],
      "call_example": "rmcp openDeepWiki list_repositories repoOwner:string={value} limit:number={value}"
    }
  },
  "meta": {
    "timestamp": "2026-03-29T12:00:00Z",
    "version": "0.4.0"
  }
}
```

### 3.4 调用工具

#### 方式一：Key=Value 参数

```bash
# 字符串参数（默认）
rmcp openDeepWiki list_repositories repoOwner=github

# 带类型的参数
rmcp openDeepWiki list_repositories repoOwner:string=github limit:number=10

# 布尔类型
rmcp openDeepWiki search enabled:bool=true
```

支持的类型：
- `string` - 字符串
- `number` / `int` - 整数或浮点数
- `float` - 浮点数
- `bool` / `boolean` - 布尔值（true/false, 1/0, yes/no）

#### 方式二：内联 YAML

```bash
rmcp openDeepWiki create_repository --yaml '
name: my-repo
description: My new repository
private: false
'
```

或单行：
```bash
rmcp openDeepWiki create_repository --yaml 'name: my-repo private: false'
```

#### 方式三：YAML 文件

```bash
# 创建参数文件
cat > params.yaml << 'EOF'
name: my-repo
description: My new repository
private: false
EOF

# 使用文件
rmcp openDeepWiki create_repository -f params.yaml
```

#### 方式四：管道输入

```bash
cat params.yaml | rmcp openDeepWiki create_repository

# 或使用 echo
echo 'name: my-repo' | rmcp openDeepWiki create_repository
```

## 4. 输出格式

默认输出格式为 JSON，支持以下格式：

### JSON（默认）
```bash
rmcp openDeepWiki list_repositories
# 或
rmcp --output json openDeepWiki list_repositories
```

### YAML
```bash
rmcp --output yaml openDeepWiki list_repositories
```

### 紧凑文本（适合管道）
```bash
rmcp --output text openDeepWiki list_repositories
```

## 5. 交互模式

启动交互式 REPL：

```bash
rmcp interactive
```

交互命令：
- `servers` - 列出所有服务器
- `use <服务器名>` - 设置默认服务器
- `tool` 或 `tools` - 列出当前服务器的工具
- `tool <工具名>` - 查看工具详情
- `help` 或 `?` - 显示帮助
- `exit` 或 `quit` - 退出

交互示例：
```
$ rmcp interactive
MCP Interactive Mode
Type 'help' for available commands, 'exit' to quit

mcp> servers
...服务器列表...

mcp> use openDeepWiki
Default server set to: openDeepWiki

mcp> tool
...工具列表...

mcp> list_repositories repoOwner=github limit=5
...调用结果...

mcp> exit
Goodbye!
```

## 6. 高级功能

### 6.1 流式输出

对于返回大量文本的工具，使用 `--stream` 选项：

```bash
rmcp --stream openDeepWiki get_repo_structure repoOwner=github repoName=vscode
```

### 6.2 参数优先级

当多种参数输入方式同时使用时，优先级如下：

1. `-f <文件>` （最高优先级）
2. `--yaml <内容>`
3. 管道输入
4. `key=value` 参数（最低优先级）

### 6.3 完整示例

```bash
# 1. 查看配置的服务器
rmcp

# 2. 查看 openDeepWiki 服务器上的所有工具
rmcp openDeepWiki

# 3. 查看 list_repositories 工具的详细信息
rmcp openDeepWiki list_repositories

# 4. 调用工具获取 github/vscode 仓库的文件结构
rmcp openDeepWiki get_repo_structure repoOwner=github repoName=vscode

# 5. 以 YAML 格式输出
rmcp --output yaml openDeepWiki list_repositories repoOwner=github limit=5

# 6. 使用复杂参数
rmcp openDeepWiki search --yaml '
query: "rust async"
filters:
  language: rust
  stars: ">1000"
sort: updated
'
```

## 7. 故障排查

### 找不到配置文件

```bash
# 检查配置文件路径
ls -la ~/.config/mcp/config.json

# 如果没有，创建一个示例
echo '{"mcpServers":{}}' > ~/.config/mcp/config.json
```

### 连接超时

```bash
# 在配置中设置超时时间（毫秒）
{
  "mcpServers": {
    "my-server": {
      "url": "https://api.example.com/mcp",
      "timeout": 60000
    }
  }
}
```

### 调试模式

```bash
RUST_LOG=debug rmcp openDeepWiki list_repositories
```

## 8. 与 AI 助手配合使用

rmcp 的主要用途是让 AI 助手通过简单的 bash 命令调用 MCP 工具，而不需要在上下文中维护完整的工具定义。

示例对话：

```
用户: 帮我查看 github/vscode 仓库的文件结构

AI: 我来帮你查看。首先让我看看有哪些工具可用：
    $ rmcp openDeepWiki
    
    然后调用 get_repo_structure 工具：
    $ rmcp openDeepWiki get_repo_structure repoOwner=github repoName=vscode
    
    [显示结果...]
```

## 9. 帮助信息

随时查看帮助：

```bash
# 主帮助
rmcp --help

# 简短帮助
rmcp -h

# 版本信息
rmcp --version
```
