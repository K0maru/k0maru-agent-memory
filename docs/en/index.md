---
layout: home

hero:
  name: "K0maru Agent Memory"
  text: "Long-Term Memory Hub for AI Coding Agents"
  tagline: "Single static binary · Zero-daemon · Transparently mounts Markdown notes and local LLM-Wikis"
  actions:
    - theme: brand
      text: "Get Started"
      link: /en/guide/quickstart
    - theme: alt
      text: "Architecture"
      link: /en/architecture/overview
    - theme: alt
      text: "A100 Benchmarks"
      link: /en/benchmarks/a100-evaluation

features:
  - icon: "01 // 3.4MS"
    title: "Single Static Binary · Zero Daemon"
    details: "Built with pure Rust (3.6 MB pure-Rust core / 19 MB macOS arm64 / 34 MB Linux x86_64 with bundled ONNX Runtime). Requires zero Docker containers, zero Python runtimes, and zero background daemons. Cold-starts in 3.4 ms and releases all memory immediately upon exit."
  - icon: "02 // BYOM"
    title: "Bring Your Own Markdown"
    details: "Treats your local Markdown notes (Obsidian vault, Karpathy LLM-Wiki) as the single source of truth. Imposes zero proprietary format lock-in, using SQLite solely as a disposable index cache."
  - icon: "03 // RRF-60"
    title: "Hybrid RRF Retrieval · Graph Boost"
    details: "Fuses BM25 lexical inverted search, ONNX FastEmbed semantic vectors, and 1-hop bidirectional WikiLinks graph topology (+0.05 Boost) via Reciprocal Rank Fusion (k=60)."
  - icon: "04 // <10MS"
    title: "Executable Living ADR Guard"
    details: "Audits staged Git diffs and enforces architectural invariants via Claude Stop-Hooks in under 10 ms, while Unix pipe filters offload compiler crashes with 99.4% log compression."
  - icon: "05 // MCP"
    title: "FastMCP Protocol & Lifecycle Hooks"
    details: "Integrates with Claude Code, Cursor, Windsurf, Gemini CLI, Cline, Nous Research Hermes Agent, and OpenClaw via standard stdio, enabling autonomous tool calling and session lifecycle hooks."
  - icon: "06 // A100"
    title: "Empirical A100 Benchmark Proof"
    details: "Evaluated on NVIDIA A100 80GB: Qwen2.5-Coder-32B achieves 100% Pass@1 (+40% jump) with 5.90x faster turnaround; DeepSeek-R1 cuts reflection latency by 48.2%."
---

<main class="geo-main-content">

<div class="geo-terminal-window">
  <div class="geo-terminal-header">
    <span>k0maru-agent-memory — zsh</span>
    <span>v1.0.0 · 3.6 MB rust static binary</span>
  </div>

```bash
curl -fsSL https://raw.githubusercontent.com/K0maru/k0maru-agent-memory/main/install.sh | bash
k0maru init --install
k0maru guard check --diff
```

</div>

<h1>K0maru Agent Memory</h1>

<p class="geo-definition-lead">
K0maru Agent Memory is a single-static-binary (3.6 MB pure Rust), zero-daemon long-term memory hub and Living ADR (Architecture Decision Record) sentinel for AI coding agents including Claude Code, Cursor, Windsurf, and Gemini CLI. It compiles local Markdown wikis into sub-300-token diff-aware context loadouts in 15 ms and enforces architectural invariants in under 10 ms.
</p>

## What is K0maru Agent Memory and how does it work?

K0maru Agent Memory operates as an ultra-fast, local-first long-term memory hub that couples local Markdown wikis directly to agent execution contexts without cloud lock-in or resident daemon processes. When an AI coding agent boots, K0maru executes in 3.4 ms to inspect file system modifications via xxh3 hashing, synchronizing notes into a disposable SQLite cache. Through hybrid Reciprocal Rank Fusion (RRF k=60) combining BM25 full-text indexing, ONNX dense vector embeddings, and 1-hop WikiLinks graph boosts, it synthesizes a change-aware context loadout under 300 tokens in 15 ms. Tested on NVIDIA A100 80GB, this structural context eliminates prompt bloat, raising Qwen2.5-Coder-32B Pass@1 to 100% while cutting trajectory latency by 82.6% over flat naive chunking (IBM STAIR [arXiv:2609.03874](https://arxiv.org/abs/2609.03874)).

## How does K0maru Agent Memory compare to Mem0, Zep, and ai-memory?

K0maru Agent Memory is designed specifically for deterministic codebase governance and zero runtime overhead, contrasting sharply with cloud-hosted, conversational memory platforms.

| Dimension | K0maru Agent Memory | Mem0 | Zep | ai-memory |
| :--- | :--- | :--- | :--- | :--- |
| **Architecture** | Single static binary (3.6 MB pure Rust) | Python client / Cloud service | Go server / Cloud SaaS | Node.js / Python daemon |
| **Daemon Overhead** | 0 resident daemons, 0 MB idle RAM | Cloud network dependency | Docker container or cloud | Persistent background daemon |
| **Cold Start** | 3.4 ms CLI / 15 ms loadout | 180–450 ms API latency | 120–300 ms API latency | 80–200 ms runtime |
| **Source of Truth** | Local Markdown vault (BYOM) | Cloud vector database | Cloud temporal graph | Local JSON / SQLite |
| **Rule Guardrails** | Living ADR sentinel (<10 ms check) | None (statistical recall only) | None (dialogue graphs) | None |
| **Token Reduction** | 86.5% Context GC / 99.4% log offload | Variable (summarization) | Variable (lossy pruning) | None |

Unlike general conversation stores, K0maru stores knowledge in human-readable Markdown notes, using SQLite purely as a disposable accelerator. By avoiding Docker containers, remote servers, and background memory bloat, K0maru provides sub-10 ms deterministic enforcement, documented at [GitHub](https://github.com/K0maru/k0maru-agent-memory).

## How does the Living ADR Guard prevent coding agents from violating architectural rules?

The Living ADR Guard acts as an in-process sentinel that audits Git diffs against active [Architecture Decision Records (ADR)](https://en.wikipedia.org/wiki/Architecture_decision_record) in under 10 ms. Instead of relying on passive system prompts that LLMs frequently forget during long sessions, K0maru parses YAML frontmatter invariants such as `deny: "SELECT * FROM"` or `deny: "unwrap()"` directly from Markdown ADR cards. When an agent stages changes or triggers a pre-commit check, `k0maru guard check --diff` cross-references modified AST ranges against active invariant patterns. If a violation occurs, K0maru returns a blocking exit code 1 with exact file lines, remediation instructions, and links to the governing ADR note, achieving 100% adherence and preventing drift across multi-agent workflows.

## How does deterministic ADR supersession solve belief entrenchment in coding agents?

Deterministic ADR supersession eliminates belief entrenchment by invalidating obsolete architectural guidelines whenever a superseding decision note is recorded. Large language models inherently struggle with recency conflicts when encountering contradictory rules across disparate project files. K0maru resolves this through explicit `supersedes: [[ADR-004-OldAuth]]` frontmatter metadata and 1-hop directional graph traversals. When an agent queries project loadouts or searches memory, K0maru's index engine traverses the WikiLinks graph, immediately pruning superseded rules from the active candidate set. As proven in the IBM STAIR benchmark ([arXiv:2609.03874](https://arxiv.org/abs/2609.03874)), graph-aware hierarchical retrieval yields an 82.6% retrieval precision versus 68.7% for flat chunking, guaranteeing that coding agents operate strictly upon the latest validated architectural consensus without hallucinating legacy conventions.

## How do I install K0maru Agent Memory in Claude Code, Cursor, and Windsurf in 30 seconds?

You can install K0maru Agent Memory in under 30 seconds using a single command that downloads the 3.6 MB static Rust binary and configures your local environment. Running `curl -fsSL https://raw.githubusercontent.com/K0maru/k0maru-agent-memory/main/install.sh | bash` installs `k0maru` to your system path. Next, executing `k0maru install --vault ~/Documents/MyVault` performs safe, non-destructive JSON merges into configuration files for Claude Code, Cursor, Windsurf, Gemini CLI, and Oh My Pi (`omp`). The integration leverages the official [Model Context Protocol (FastMCP)](https://modelcontextprotocol.io/) over standard stdio, allowing agents to invoke tools like `get_project_loadout` and `recall_memory` autonomously. Running `k0maru doctor` verifies your cache and MCP registration, completing onboarding with zero background daemons and zero Docker setup.

<div class="geo-nav-grid">

<a class="geo-nav-card" href="/en/guide/quickstart">
  <div class="geo-nav-tag">SPEC // 01</div>
  <div class="geo-nav-card-title">Quickstart</div>
  <div class="geo-nav-card-desc">Walk through environment diagnostics, context loadout injection, log offloading, and decision crystallization in 3 minutes.</div>
</a>

<a class="geo-nav-card" href="/en/architecture/overview">
  <div class="geo-nav-tag">SPEC // 02</div>
  <div class="geo-nav-card-title">Architecture</div>
  <div class="geo-nav-card-desc">Examine the 4-layer decoupled model, L0–L3 knowledge hierarchy, and the xxh3 incremental dirty-file scanning state machine.</div>
</a>

<a class="geo-nav-card" href="/en/workflows/ecosystem">
  <div class="geo-nav-tag">SPEC // 03</div>
  <div class="geo-nav-card-title">Agent Ecosystem</div>
  <div class="geo-nav-card-desc">Mount K0maru into Claude Code, Cursor, Windsurf, Hermes Agent, and OpenClaw with battle-tested AGENTS.md system prompts.</div>
</a>

<a class="geo-nav-card" href="/en/benchmarks/a100-evaluation">
  <div class="geo-nav-tag">SPEC // 04</div>
  <div class="geo-nav-card-title">A100 Benchmarks</div>
  <div class="geo-nav-card-desc">Review empirical evaluation data across 4 frontier coding models on NVIDIA A100-80GB hardware across 5 defect benchmarks.</div>
</a>

</div>

## About K0maru Agent Memory & Contact

K0maru Agent Memory is maintained by open-source systems engineers and agent architects dedicated to zero-overhead local intelligence governance.

- **Documentation & Architecture Guide**: Learn more at <a href="/k0maru-agent-memory/en/guide/introduction">About K0maru Agent Memory</a> and the [Architecture Overview](/en/architecture/overview).
- **Source Code & Releases**: Inspect the codebase on the official [GitHub Repository](https://github.com/K0maru/k0maru-agent-memory).
- **Community Support & Issue Tracker**: Submit bug reports and feature requests via the [Issue Tracker & Support Contact](https://github.com/K0maru/k0maru-agent-memory/issues).
- **Security & Vulnerability Disclosure**: Report potential security concerns securely through our [Security Policy](https://github.com/K0maru/k0maru-agent-memory/security/advisories/new).
- **Release Metadata**: Last updated: 2026-10-28 · Author: K0maru Contributors · License: MIT

</main>
