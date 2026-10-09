# Agent Ecosystem Integration

K0maru-Agent-Memory implements the standard FastMCP Stdio protocol, integrating out of the box with leading AI coding assistants and autonomous agent frameworks.

---

## ⚡ Automated Configuration Injection (`k0maru install`)

Instead of searching through hidden application configuration files, use K0maru's built-in installer to mount your knowledge base:

```bash
# Automatically discover installed agent clients and mount your Markdown vault
k0maru install --vault ~/Documents/SecondBrain
```

### Dry-Run Preview
Inspect proposed changes before writing them to disk:

```bash
k0maru install --vault ~/Documents/SecondBrain --dry-run
```

### Target a Specific Client (`--target`)
Target individual clients using the `--target` flag:

```bash
# Configure Claude Code only
k0maru install --vault ~/Documents/SecondBrain --target claude

# Configure Cursor only
k0maru install --vault ~/Documents/SecondBrain --target cursor

# Configure Windsurf only
k0maru install --vault ~/Documents/SecondBrain --target windsurf
```

---

## 🛠️ Configuration File Paths & Manual Setup

K0maru's injector uses a **non-destructive JSON merge strategy**, preserving existing MCP servers and user settings. If you prefer to manage client configurations manually, reference the paths and JSON snippet below.

| AI Agent Client | Standard Configuration File Path | Protocol Transport |
| :--- | :--- | :--- |
| **Claude Code** | `~/.claude.json` | FastMCP Stdio |
| **Cursor** | `~/.cursor/mcp.json` | FastMCP Stdio |
| **Antigravity / Gemini CLI** | `~/.gemini/antigravity-cli/mcp_config.json`<br/>`~/.gemini/config/mcp_config.json` | FastMCP Stdio |
| **Windsurf** | `~/.codeium/windsurf/mcp_config.json` | FastMCP Stdio |
| **Cline / Roo Code** | macOS: `~/Library/Application Support/Code/User/globalStorage/.../settings/cline_mcp_settings.json`<br/>Linux: `~/.config/Code/User/globalStorage/.../settings/cline_mcp_settings.json`<br/>Windows: `%APPDATA%\Code\User\globalStorage\...\settings\cline_mcp_settings.json` | FastMCP Stdio |
| **Hermes Agent** | `~/.hermes/mcp.json` (or `$HERMES_HOME/mcp.json`) | FastMCP Stdio |
| **OpenClaw** | `~/.openclaw/mcp.json` (or `$OPENCLAW_HOME/mcp.json`) | FastMCP Stdio |

### Standard Configuration Snippet
Add the following entry to the `mcpServers` object in your client's JSON configuration:

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

::: tip Dynamic Vault Resolution
If you omit the `--vault` flag from `args`, K0maru traverses parent directories from the current working terminal to locate `.k0maru` or `.obsidian` vault root markers automatically.
:::

---

## 🔍 JSON-RPC Protocol Testing & Diagnostics

If an agent client fails to invoke tools, verify FastMCP communication directly in your terminal.

### 1. Launch the Server and Send Initialization Payload
Start K0maru in MCP mode:

```bash
k0maru mcp --vault ~/Documents/MyVault
```

Paste the following JSON-RPC handshake request and press `Enter`:

```json
{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test-client","version":"1.0.0"}}}
```

### Expected Handshake Response:
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

### 2. Inspect Available Tools
Query registered tools by sending:

```json
{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}
```

K0maru returns JSON Schema declarations for all 6 native tools (`get_project_loadout`, `recall_memory`, `offload_context`, `inspect_log_node`, `flush_session`, and `distill_session_skill`).

---

## 📜 `AGENTS.md` System Prompt Best Practices

To ensure AI coding agents consult your knowledge base proactively without requiring manual prompt reminders, create an `AGENTS.md` file in your repository root (or append to your global agent instructions):

```markdown
# AGENTS.md — Agent Governance & Long-Term Memory Protocol

This repository mounts the `k0maru-memory` long-term memory hub. Adhere to the following rules when executing development, refactoring, or debugging tasks:

## 1. Pre-Flight Inspection (Bootstrap Loadout)
Before writing code or designing components, call `get_project_loadout(project_name)` to load project vision, technology dependencies, and declared L3 architecture invariants.

## 2. Active Recall of Architectural Decisions (Active Recall)
When modifying database schemas, designing concurrency controls, altering state machines, or resolving complex bugs, call `recall_memory(query)` to review past Architectural Decision Records (ADRs) and incident postmortems. Never invent patterns that contradict established invariants.

## 3. Symbolic Log Governance (Noise Filtering)
When running tests or build commands that produce verbose output, pipe terminal streams through `| k0maru offload`. Review the summarized Mermaid state diagram and error codes. Only call `inspect_log_node(node_id)` when you must inspect specific call frames.

## 4. Continuous Crystallization (Knowledge Compounding)
After delivering a feature, passing integration tests, or resolving a production defect, call `flush_session` to persist the solution into the knowledge base. Link related cards using `related_notes` to maintain the [[WikiLinks]] graph.
```

With these instructions in place, an agent that receives a prompt like *"Implement the payment callback handler"* executes the full 4-step workflow automatically:
1. Calls `get_project_loadout` to retrieve project constraints;
2. Calls `recall_memory` to check distributed lock and idempotency invariants;
3. Generates invariant-compliant application code;
4. Calls `flush_session` to persist the implementation decision.
