# ⚡ K0maru-Agent-Memory 极速上手指南 (Quick Start)

本文档带你在 **1 分钟** 内掌握 `k0maru` 的日常核心实操！

---

## 🛠️ 第一步：一键编译与安装

`k0maru` 是一个**单静态二进制文件（Single Static Binary）**，体积仅 3.6MB，无须配置 Python 虚拟环境或安装外部数据库。

```bash
# 1. 在仓库根目录下编译优化版本
cargo build --release

# 2. 安装至你的 PATH 路径（例如 ~/.local/bin 或 /usr/local/bin）
# macOS 建议先移除旧文件再拷贝签名，避免 inode 代码页缓存冲突：
rm -f ~/.local/bin/k0maru && cp target/release/k0maru ~/.local/bin/ && codesign -s - --force ~/.local/bin/k0maru

# 3. 验证安装
k0maru --version
# 输出: k0maru 0.4.2
```

---

## 🎒 第二步：开局秒级给 AI 读档 (`loadout`)

当你开启一个新的 Claude Code、Cursor、ChatGPT 窗口准备写代码时，别再自己手动交代项目背景了。

### 1. 查看知识库里当前有哪些项目：
```bash
k0maru loadout -l --vault ~/wiki
```
> 输出你的活跃项目清单（如 `my-project`、`auth-service` 等）。

### 2. 提取项目记忆并直接注入系统剪贴板：
```bash
k0maru loadout my-project --vault ~/wiki --copy
```
> 终端显示：`📋 Copied loadout (<300 tokens) to clipboard!`

### 3. 在 AI 会话中直接粘贴 (`Cmd + V`)：
AI 将在 0 额外开销下立刻获得：
- 💡 **项目愿景与范围**；
- 🧠 **5 条核心常青卡片（L3）的一句话概念**（如风控铁律、套利数学模型）；
- 📝 **前天写下的研发日志（L2）双链**；
- 🛡️ **知识库操作规约**。

整个过程消耗 **不到 300 Token**，耗时仅 **5 毫秒**！

---

## 🛡️ 第三步：终端长日志防撑爆 (`offload` & `inspect`)

AI 在终端跑测试或构建时经常刷出好几百行日志，直接导致注意力漂移与大模型降智。

### 1. 在任何长命令后面加上管道 `| k0maru offload`：
```bash
# 示例：运行测试并自动拦截过长日志
cargo test | k0maru offload
# 或者：
pytest -v | k0maru offload
```
- **输出不超过 50 行**：正常输出，不做处理；
- **输出超过 50 行**：全量日志被瞬间截断存入 `.scratch/refs/`，终端只输出极简的 Mermaid 状态图：
  ```text
  stateDiagram-v2
      [*] --> Running
      Running --> Offloaded : >50 lines (342 lines total)
  💡 Inspect trace: k0maru inspect node_54697ed3
  ```

### 2. 局部精准排查错误 (`inspect`)：
AI 不需要重新跑命令，直接按编号调取该段报错堆栈：
```bash
k0maru inspect node_54697ed3
```
只看那一段报错，**立省 50%~80% 的无用 Token**！

---

## 🤖 第四步：在 IDE 中配置 FastMCP（让 AI 自主调用）

配置后，**你连命令行都不需要敲**，AI 在跟你的对话中会自己翻看你的笔记库！

### 1. Claude Code 配置 (`~/.claude.json`)
```json
{
  "mcpServers": {
    "k0maru-memory": {
      "command": "k0maru",
      "args": ["mcp", "--vault", "/path/to/your/wiki"]
    }
  }
}
```

### 2. Antigravity / agy 配置 (`~/.gemini/config/mcp_config.json`)
```json
{
  "mcpServers": {
    "k0maru-memory": {
      "command": "k0maru",
      "args": ["mcp", "--vault", "/path/to/your/wiki"]
    }
  }
}
```

### 3. 配置完成后的自然语言体验：
在聊天窗口中直接用大白话提问，AI 会自动命中 MCP 工具：
- *“我想了解一下当前项目的背景与架构核心，请帮我查阅相关笔记”*  
  👉 AI 自动调用 `k0maru-memory/recall_memory` 翻阅你的知识库；
- *“准备开始开发 my-project，帮我读档”*  
  👉 AI 自动调用 `k0maru-memory/get_project_loadout`，自主装配紧凑背包。

---

## 🖥️ 第五步：按需唤醒本地极客控制台 (`ui`)

如果你想直观探索混合检索打分、透视长日志状态机或监看 Token 节约情况：

```bash
# 按需唤醒极客控制台（自动打开浏览器）
k0maru ui --vault ~/wiki --open
```

- 🌐 **中英双语即时切换**：顶栏自带 `🌐 中文 / EN` 按钮，即点即切无须刷新，本地持久化并自动识别系统语言；
- 🔍 **混合检索调试台**：实时直观查看 BM25 排名、Vector 距离与 `+0.05 Graph Boost` 拓扑升权；
- 📜 **长日志切片透视**：原生脱机渲染 Mermaid 流程图与带错误行号的折叠堆栈，一键复制错误切片；
- 📊 **Token 计分板**：实时直观监看上下文治理节约量（TRR 98.8%）与缓存覆盖度，提供一键增量同步；
- *(注：WikiLinks 拓扑图谱功能底层模型与 API 就绪，界面默认保持精简收起)*；
- 退出只需在终端按下 `Ctrl + C`，**立即释放端口与内存，绝不留存任何后台守护进程**！

> 📖 **详尽界面实操与高阶用法请参阅**：[docs/UI_TUTORIAL.md](docs/UI_TUTORIAL.md)

---

## 📖 CLI 常用指令速查手册

| 命令 | 关键参数 | 功能说明 |
| :--- | :--- | :--- |
| `k0maru ui` | `--vault <path>` | 启动本地内嵌极客控制台（默认 `127.0.0.1:3721`） |
| | `--port <port>` | 自定义绑定端口号（默认 3721） |
| | `--open` | 启动后自动在系统默认浏览器中打开控制台 |
| `k0maru loadout <query>` | `--vault <path>` | 指定笔记库根目录（默认探测当前及父目录） |
| | `--copy` | 将装配好的背包直接写入系统剪贴板 |
| | `--json` | 输出机器无损解析的 JSON 数据包 |
| | `-l`, `--list` | 列出当前笔记库中的所有活跃工程项目 |
| `k0maru search <query>` | `--vault <path>` | 指定笔记库根目录 |
| | `--mode <hybrid\|bm25\|vector>` | 检索模式（默认 hybrid 混合融合） |
| | `--limit <N>` | 结果返回上限（默认 5 篇） |
| | `--json` | 输出机器结构化 JSON 检索结果 |
| `k0maru sync` | `--vault <path>` | 增量扫描并刷新 SQLite 索引与双链拓扑 |
| | `--vector` | 同步生成/增量刷新本地向量嵌入缓存 |
| | `--json` | 输出增量同步统计状态 |
| `k0maru offload` | `--threshold <N>` | 自定义截断行数阈值（默认 50 行） |
| | `--task-id <id>` | 绑定任务标识（便于日志命名归档） |
| `k0maru inspect <id>` | `<node_id>` | 提取被截断保存的长日志原文 |
| `k0maru mcp` | `--vault <path>` | 启动标准 stdio MCP 服务，供 IDE 无缝挂载 |
