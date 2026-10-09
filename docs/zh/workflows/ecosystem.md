# 智能体生态挂载与配置

K0maru-Agent-Memory 遵循标准的 FastMCP Stdio 协议规范，能够无缝挂载至当前业界主流的各种 AI 编码智能体客户端与自主智能体框架中。

---

## ⚡ 一键自动化挂载配置（`k0maru install`）

无需手动翻找各 IDE 隐藏在系统深处的配置文件，K0maru 提供了全自动的配置注入器：

```bash
# 一键自动发现并挂载所有已安装的客户端，指向您的 Markdown 知识库
k0maru install --vault ~/Documents/SecondBrain
```

### 预览模式（Dry Run）
在实际写盘之前，可先使用 `--dry-run` 预览将要进行的修改与差异对比：

```bash
k0maru install --vault ~/Documents/SecondBrain --dry-run
```

### 指定特定目标客户端（`--target`）
若只想为特定智能体配置，可通过 `--target` 单独指定：

```bash
# 仅配置 Claude Code
k0maru install --vault ~/Documents/SecondBrain --target claude

# 仅配置 Cursor
k0maru install --vault ~/Documents/SecondBrain --target cursor

# 仅配置 Windsurf
k0maru install --vault ~/Documents/SecondBrain --target windsurf
```

---

## 🛠️ 各客户端配置文件路径与手动配置指南

K0maru 的配置注入引擎采用**非破坏性 JSON 合并策略**，保留已有其他 MCP 服务与配置项。如果你更倾向于手动维护配置，可参考下表中各客户端的配置文件路径与模板。

| 智能体客户端 | 配置文件标准系统路径 | 注入协议类型 |
| :--- | :--- | :--- |
| **Claude Code** | `~/.claude.json` | FastMCP Stdio |
| **Cursor** | `~/.cursor/mcp.json` | FastMCP Stdio |
| **Antigravity / Gemini CLI** | `~/.gemini/antigravity-cli/mcp_config.json`<br/>`~/.gemini/config/mcp_config.json` | FastMCP Stdio |
| **Windsurf** | `~/.codeium/windsurf/mcp_config.json` | FastMCP Stdio |
| **Cline / Roo Code** | macOS: `~/Library/Application Support/Code/User/globalStorage/.../settings/cline_mcp_settings.json`<br/>Linux: `~/.config/Code/User/globalStorage/.../settings/cline_mcp_settings.json`<br/>Windows: `%APPDATA%\Code\User\globalStorage\...\settings\cline_mcp_settings.json` | FastMCP Stdio |
| **Hermes Agent** | `~/.hermes/mcp.json`（或环境变量 `$HERMES_HOME/mcp.json`） | FastMCP Stdio |
| **OpenClaw** | `~/.openclaw/mcp.json`（或环境变量 `$OPENCLAW_HOME/mcp.json`） | FastMCP Stdio |

### 手动标准配置代码块

在上述各 JSON 配置文件的 `mcpServers` 对象中追加如下键值：

```json
{
  "mcpServers": {
    "k0maru-memory": {
      "command": "k0maru",
      "args": [
        "mcp",
        "--vault",
        "/Users/username/Documents/MyVault"
      ]
    }
  }
}
```

::: tip 动态查找知识库
若在 `args` 中省略 `--vault` 参数，K0maru 会在启动时自动自底向上递归查找当前终端工作目录及父级目录中的 `.k0maru` 或 `.obsidian` 标识。
:::

---

## 🔍 JSON-RPC 通信与排错调试技巧

如果智能体未成功调用工具或遇到连接问题，可在终端直接测试与 K0maru 的 FastMCP 通信。

### 1. 手动启动 Stdio 服务并发送初始化报文
直接在终端运行 `k0maru mcp`，随后手动粘贴 JSON-RPC 握手报文：

```bash
k0maru mcp --vault ~/Documents/MyVault
```

在终端输入以下 JSON-RPC 初始化请求（回车）：

```json
{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test-client","version":"1.0.0"}}}
```

### 预期返回：
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "protocolVersion": "2024-11-05",
    "capabilities": { "tools": {} },
    "serverInfo": { "name": "k0maru", "version": "0.8.0" }
  }
}
```

### 2. 测试工具列表查询
继续在终端输入：
```json
{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}
```

K0maru 将返回完整的 6 大原生工具列表（`get_project_loadout`、`recall_memory`、`offload_context`、`inspect_log_node`、`flush_session`、`distill_session_skill`）及其 JSON Schema 契约。

---

## 📜 `AGENTS.md` 系统提示词最佳实践范式

为了让 AI 智能体在日常编码中具备 100% 的主动意识去查阅知识库，而非仅凭训练语料中的通用印象随缘编码，推荐在你的项目根目录创建或更新 `AGENTS.md`（或在全局 System Prompt 中追加）：

```markdown
# AGENTS.md — 智能体架构纪律与长程记忆治理契约

本仓库挂载了 `k0maru-memory` 长期记忆中枢。在执行任何开发、重构或排障任务时，请严格遵守以下行为准则：

## 1. 编码前主动读档（Pre-flight Inspection）
在开辟新特性或开始阅读代码前，优先调用 `get_project_loadout(project_name)` 加载当前项目的愿景、架构依赖与已声明的 L3 核心不变量。

## 2. 查阅架构决策与踩坑教训（Active Recall）
在设计数据表、引入并发锁、编写敏感状态机流转或排查复杂故障时，必须主动调用 `recall_memory(query)` 翻阅既往决策（ADR）与事故事后复盘（Postmortem），严禁凭空臆造方案。

## 3. 终端长日志治理（Symbolic Log Governance）
当运行测试或编译产生大量输出时，使用终端管道过滤 `| k0maru offload`。智能体只需研读精炼的 Mermaid 状态机图与关键报错代号；仅在确实需要深入调用栈时才调用 `inspect_log_node(node_id)`。

## 4. 经验与决策自愈沉淀（Continuous Crystallization）
任务交付或复杂 Bug 解决后，必须调用 `flush_session` 将核心决策持久化至知识库，并通过 `related_notes` 编织双向 WikiLinks，形成经验沉淀。
```

当把上述规约沉淀至 `AGENTS.md` 后，即便用户只向模型输入简单的一句话指令（例如：*“帮我实现支付回调处理逻辑”*），模型也会自动执行如下四部曲：
1. 主动调用 `get_project_loadout` 获取支付服务的架构约束；
2. 主动调用 `recall_memory` 查阅防重幂等与分布式锁规约；
3. 输出完全符合业务不变量的代码；
4. 调用 `flush_session` 结晶当前设计决策。
