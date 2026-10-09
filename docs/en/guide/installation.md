# Installation & Environment Setup

K0maru-Agent-Memory is written in pure Rust and distributed as a self-contained static binary. Choose the installation method that best fits your operating system and workflow.

---

## 📦 5 Installation Methods

### Method 1: Official One-Line Script (macOS & Linux - Recommended)

The automated installation script detects your CPU architecture (Apple Silicon `aarch64` or Intel/AMD `x86_64`) and operating system, downloads the matching precompiled binary from the latest GitHub Release, verifies its SHA-256 integrity checksum, and places it into your executable path (`~/.local/bin` or `/usr/local/bin`):

```bash
curl -fsSL https://raw.githubusercontent.com/K0maru/k0maru-agent-memory/main/install.sh | bash
```

::: tip Automatic PATH Configuration
The installer detects your active shell (`zsh`, `bash`, or `fish`) and appends `export PATH="$HOME/.local/bin:$PATH"` to `~/.zshrc` or `~/.bashrc` if needed. If your shell does not recognize `k0maru` immediately after installation, run `source ~/.zshrc` or restart your terminal.
:::

---

## Method 2: Homebrew Tap (macOS & Linux)

If you manage CLI tools with Homebrew, install and update K0maru through the official tap:

```bash
# Add the official tap and install the formula
brew tap K0maru/k0maru-agent-memory
brew install k0maru

# Alternatively, install via the shorthand invocation
brew install K0maru/tap/k0maru
```

Upgrade to the latest release at any time:
```bash
brew update && brew upgrade k0maru
```

---

## Method 3: Cargo Installation (crates.io or Git)

If you maintain a local Rust development environment with Rust 1.75+, install K0maru directly using `cargo`:

```bash
# Install the official stable release from crates.io
cargo install k0maru-agent-memory

# Alternatively, compile directly from the latest dev branch on GitHub
cargo install --git https://github.com/K0maru/k0maru-agent-memory --branch dev
```

Cargo installs the binary to `~/.cargo/bin/k0maru`. Ensure that `~/.cargo/bin` is included in your `$PATH`.

---

## Method 4: Pre-Compiled GitHub Release Binaries

Download pre-built archive packages directly from the [GitHub Releases page](https://github.com/K0maru/k0maru-agent-memory/releases) for your operating system and CPU architecture:

| Platform & Architecture | Target Triple Identifier | Supported Systems |
| :--- | :--- | :--- |
| **macOS (Apple Silicon)** | `aarch64-apple-darwin` | Apple M1 / M2 / M3 / M4 Macs |
| **macOS (Intel)** | `x86_64-apple-darwin` | Intel-based Macs |
| **Linux (x86_64)** | `x86_64-unknown-linux-gnu` | Ubuntu, Debian, CentOS, Fedora, Arch Linux |
| **Linux (ARM64)** | `aarch64-unknown-linux-gnu` | ARM64 Linux servers and virtual machines |
| **Windows (x64)** | `x86_64-pc-windows-msvc` | Windows 10 / 11 64-bit systems |

Example manual installation steps for macOS Apple Silicon:

```bash
# 1. Download the matching architecture archive
curl -LO https://github.com/K0maru/k0maru-agent-memory/releases/latest/download/k0maru-aarch64-apple-darwin.tar.gz

# 2. Extract the archive
tar -xzf k0maru-aarch64-apple-darwin.tar.gz

# 3. Grant executable permissions and move to your local bin directory
chmod +x k0maru
mv k0maru ~/.local/bin/
```

---

## Method 5: Building from Source (Development & Customization)

Compile K0maru from source if you plan to modify internal algorithms, benchmark specific target architectures, or audit the implementation.

### 1. Prerequisites
- **Rust toolchain**: MSRV 1.75+ (install or update via `rustup update stable`).
- **C/C++ compiler and build tools**: Required to compile embedded `rusqlite` and `sqlite-vec` extensions.

On Ubuntu / Debian:
```bash
sudo apt-get update
sudo apt-get install -y build-essential cmake clang
```

On macOS:
```bash
xcode-select --install
```

### 2. Clone and Compile Standard Release
```bash
# Clone the repository
git clone https://github.com/K0maru/k0maru-agent-memory.git
cd k0maru-agent-memory

# Build the optimized release binary (includes FTS5, sqlite-vec, and ONNX FastEmbed)
cargo build --release

# Inspect the compiled binary (typically ~3.66 MB)
ls -lh target/release/k0maru

# Copy to your system PATH
cp target/release/k0maru ~/.local/bin/
```

### 3. Ultra-Lightweight Build (Disabling ONNX Embedding Engine)
If you deploy K0maru to resource-constrained environments (such as a 512 MB RAM VPS, an embedded IoT device, or a temporary CI runner), disable default dense vector embedding features. This strips the ONNX runtime and shrinks the binary to **~2.8 MB**, relying exclusively on BM25 lexical inverted search and 1-hop graph boost:

```bash
cargo build --release --no-default-features
```

---

## 🩺 Environment Diagnostics: `k0maru doctor`

Verify your binary installation, local vault status, SQLite cache integrity, and agent client integrations by running `k0maru doctor`:

```bash
k0maru doctor
```

Expected terminal output:

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

If `doctor` detects any missing prerequisites (such as an uninitialized vault directory or missing client configuration files), it prints highlighted diagnostic warnings with actionable remediation commands.
