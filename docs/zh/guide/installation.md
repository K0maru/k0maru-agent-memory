# 安装与环境配置

K0maru-Agent-Memory 采用纯 Rust 编写并进行静态编译分发。你可以根据自己的操作系统环境与偏好选择最合适的安装方式。

---

## 📦 5 种安装方式

### 方式一：官方一键安装脚本（macOS 与 Linux - 强烈推荐）

该脚本会自动检测你的 CPU 架构（Apple Silicon `aarch64` 或 Intel/AMD `x86_64`）与操作系统，从 GitHub Release 自动下载对应的预编译二进制，校验 SHA-256 完整性哈希，并自动将其安装至系统的可执行路径（默认 `~/.local/bin` 或 `/usr/local/bin`）：

```bash
curl -fsSL https://raw.githubusercontent.com/K0maru/k0maru-agent-memory/main/install.sh | bash
```

::: tip 自动注入 PATH
安装脚本会自动检测你当前使用的 Shell（`zsh`、`bash` 或 `fish`），并在 `~/.zshrc` 或 `~/.bashrc` 中追加 `export PATH="$HOME/.local/bin:$PATH"`。若安装完成后终端无法识别 `k0maru`，只需执行 `source ~/.zshrc` 或重启终端即可。
:::

---

### 方式二：Homebrew Tap（macOS 与 Linux）

如果你习惯使用 Homebrew 进行包管理，可以通过官方 Tap 快速安装与更新：

```bash
# 添加官方 Tap 并安装
brew tap K0maru/k0maru-agent-memory
brew install k0maru

# 或者一条命令直接安装
brew install K0maru/tap/k0maru
```

升级至最新版本：
```bash
brew update && brew upgrade k0maru
```

---

### 方式三：Cargo 安装（通过 crates.io 或 Git）

如果本地已具备 Rust 开发环境（Rust 1.75+），可直接通过 `cargo` 进行安装：

```bash
# 从 crates.io 安装官方稳定版本
cargo install k0maru-agent-memory

# 或直接拉取 GitHub 仓库的最新 dev 分支代码编译安装
cargo install --git https://github.com/K0maru/k0maru-agent-memory --branch dev
```

安装完成后，可执行文件通常位于 `~/.cargo/bin/k0maru`。请确保 `~/.cargo/bin` 已加入你的系统环境变量 `$PATH`。

---

### 方式四：预编译 GitHub Release 手动下载

你可以直接前往 [GitHub Releases 页面](https://github.com/K0maru/k0maru-agent-memory/releases) 下载适合当前平台与 CPU 架构的预编译归档包：

| 平台与芯片架构 | Target Triple 标识 | 适用设备 |
| :--- | :--- | :--- |
| **macOS (Apple Silicon)** | `aarch64-apple-darwin` | M1 / M2 / M3 / M4 系列 Mac |
| **macOS (Intel)** | `x86_64-apple-darwin` | Intel 处理器 Mac |
| **Linux (x86_64)** | `x86_64-unknown-linux-gnu` | 主流 Linux 发行版（Ubuntu、Debian、CentOS、Arch） |
| **Linux (ARM64)** | `aarch64-unknown-linux-gnu` | ARM64 Linux 虚拟机或服务器 |
| **Windows (x64)** | `x86_64-pc-windows-msvc` | Windows 10 / 11 64 位系统 |

以 macOS Apple Silicon 为例的手动安装步骤：

```bash
# 1. 下载对应架构的压缩包
curl -LO https://github.com/K0maru/k0maru-agent-memory/releases/latest/download/k0maru-aarch64-apple-darwin.tar.gz

# 2. 解压归档文件
tar -xzf k0maru-aarch64-apple-darwin.tar.gz

# 3. 赋予执行权限并移入用户可执行目录
chmod +x k0maru
mv k0maru ~/.local/bin/
```

---

### 方式五：从源码编译构建（开发与高级定制）

如果你需要针对特定的微架构进行激进优化，或者对 K0maru 进行二次开发，可以自行克隆仓库并编译。

#### 1. 系统构建依赖准备
- **Rust 工具链**：MSRV 1.75+（推荐通过 `rustup update stable` 保持最新）；
- **C/C++ 编译工具**：用于编译 `rusqlite` 与 `sqlite-vec` 扩展（macOS 需安装 Xcode Command Line Tools，Linux 需安装 `build-essential` 与 `cmake`）。

在 Ubuntu / Debian 上的环境准备命令：
```bash
sudo apt-get update
sudo apt-get install -y build-essential cmake clang
```

在 macOS 上的环境准备命令：
```bash
xcode-select --install
```

#### 2. 克隆与标准发布编译
```bash
# 克隆仓库
git clone https://github.com/K0maru/k0maru-agent-memory.git
cd k0maru-agent-memory

# 运行标准 Release 优化编译（默认包含 FTS5、sqlite-vec 与 ONNX fastembed）
cargo build --release

# 查看编译产物（产物体积通常约 3.66 MB）
ls -lh target/release/k0maru

# 安装至本地 PATH
cp target/release/k0maru ~/.local/bin/
```

#### 3. 极简轻量模式编译（关闭 ONNX 向量引擎）
如果你的运行目标环境极其受限（例如 512 MB 内存的小型 VPS、嵌入式板卡或 CI 临时容器），可以关闭默认的稠密向量模型特性。此时二进制体积将从 3.66 MB 进一步压低至 **~2.8 MB**，完全依赖 BM25 词法倒排与图谱拓扑：

```bash
cargo build --release --no-default-features
```

---

## 🩺 验证与诊断：`k0maru doctor`

安装完成后，在终端运行 `k0maru doctor` 命令对本地运行环境、系统依赖、知识库规范与智能体配置进行全方位自检：

```bash
k0maru doctor
```

预期终端输出示例：

```text
K0maru Health & Diagnostic Report
=================================
[✓] Binary in System PATH: /Users/k0maru3/.local/bin/k0maru (v0.8.0)
[✓] Target Vault Detected: /Users/k0maru3/workspace/SecondBrain
[✓] SQLite Cache Integrity: .k0maru/cache.sqlite (Healthy, 74ms rebuildable)
[✓] FTS5 BM25 Inverted Index: 521 documents indexed
[✓] Vector Embeddings: all-MiniLM-L6-v2 (384-dim, FastEmbed ready)
[✓] Claude Code MCP Config: Mounted (/Users/k0maru3/.claude.json)
[✓] Cursor MCP Config: Mounted (/Users/k0maru3/.cursor/mcp.json)
[✓] Windsurf MCP Config: Mounted (/Users/k0maru3/.codeium/windsurf/mcp_config.json)

Status: ALL CHECKS PASSED. Ready for agent orchestration.
```

若 `doctor` 检测到任何未就绪的项目（例如缺少环境变量、知识库未初始化或客户端配置缺失），会打印出高亮的黄色警告，并给出对应的精准一键修复命令。
