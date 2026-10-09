---
layout: home

hero:
  name: "K0maru Agent Memory"
  text: "Long-Term Memory Hub for AI Coding Agents"
  tagline: "Single static binary · Zero-daemon · Transparently mounts Markdown notes and local LLM-Wikis"
  actions:
    - theme: brand
      text: 🚀 3-Minute Quickstart
      link: /en/guide/quickstart
    - theme: alt
      text: 🏗️ Architecture Overview
      link: /en/architecture/overview
    - theme: alt
      text: 📦 Installation & Setup
      link: /en/guide/installation
    - theme: alt
      text: 📊 A100 Benchmark Dashboard
      link: /en/benchmarks/a100-evaluation

features:
  - icon: ⚡
    title: "Single Static Binary · Zero Daemon"
    details: "Built with pure Rust in a 3.66 MB single file. Requires zero Docker containers, zero Python runtimes, and zero background daemons. Cold-starts in 3.38 ms and releases all memory immediately upon exit."
  - icon: 📝
    title: "Bring Your Own Markdown (BYOM)"
    details: "Treats your local Markdown notes (Obsidian vault, Karpathy LLM-Wiki) as the single source of truth. Imposes zero proprietary format lock-in, using SQLite solely as a disposable index cache."
  - icon: 🎯
    title: "Hybrid RRF Retrieval · Graph Boost"
    details: "Fuses BM25 lexical inverted search, ONNX FastEmbed semantic vectors, and 1-hop bidirectional WikiLinks graph topology (+0.05 Boost) via Reciprocal Rank Fusion (k=60)."
  - icon: 🛡️
    title: "Symbolic Log Offloading (99.4% Compression)"
    details: "Intercepts 1,000+ line compiler crashes and test traces through standard Unix pipes, compressing them into 15-line Mermaid state diagrams to eliminate context window blowouts."
  - icon: 🔌
    title: "FastMCP Protocol Ready Out-of-the-Box"
    details: "Integrates with Claude Code, Cursor, Windsurf, Gemini CLI, Cline, Nous Research Hermes Agent, and OpenClaw via standard stdio, enabling autonomous tool calling from natural language intent."
  - icon: 📈
    title: "Empirical Production A100 Benchmarks"
    details: "Evaluated on NVIDIA A100 80GB: Qwen2.5-Coder-32B achieves 100% Pass@1 (+40% jump) with 5.90x faster turnaround; DeepSeek-R1 cuts reflection latency by 48.2%."
---

<div class="developer-hero-section" style="margin-top: 2.5rem; text-align: center;">

## ⚡ Instant Terminal Experience

Install the binary and mount your local Markdown vault with three commands:

```bash
# 1. Install the binary with the official installer script
curl -fsSL https://raw.githubusercontent.com/K0maru/k0maru-agent-memory/main/install.sh | bash

# 2. Mount your local vault into Claude Code, Cursor, and Windsurf
k0maru install --vault ~/Documents/MyVault

# 3. Verify your environment with the health check
k0maru doctor
```

</div>

<div class="developer-cards-grid" style="display: grid; grid-template-columns: repeat(auto-fit, minmax(240px, 1fr)); gap: 1.25rem; margin-top: 2rem;">

<div style="background: #1B2336; border: 1px solid #334155; border-radius: 8px; padding: 1.25rem;">
<h3 style="margin-top: 0; color: #22C55E;">🚀 Quickstart</h3>
<p style="color: #94A3B8; font-size: 0.95rem;">Walk through environment diagnostics, context loadout injection, log offloading, and decision crystallization in 3 minutes.</p>
<a href="/en/guide/quickstart" style="color: #22C55E; font-weight: 600; text-decoration: none;">Explore Quickstart →</a>
</div>

<div style="background: #1B2336; border: 1px solid #334155; border-radius: 8px; padding: 1.25rem;">
<h3 style="margin-top: 0; color: #22C55E;">🏗️ Architecture</h3>
<p style="color: #94A3B8; font-size: 0.95rem;">Examine the 4-layer decoupled model, L0–L3 knowledge hierarchy, and the xxh3 incremental dirty-file scanning state machine.</p>
<a href="/en/architecture/overview" style="color: #22C55E; font-weight: 600; text-decoration: none;">Inspect Architecture →</a>
</div>

<div style="background: #1B2336; border: 1px solid #334155; border-radius: 8px; padding: 1.25rem;">
<h3 style="margin-top: 0; color: #22C55E;">🤖 Agent Ecosystem</h3>
<p style="color: #94A3B8; font-size: 0.95rem;">Mount K0maru into Claude Code, Cursor, Windsurf, Hermes Agent, and OpenClaw with battle-tested AGENTS.md system prompts.</p>
<a href="/en/workflows/ecosystem" style="color: #22C55E; font-weight: 600; text-decoration: none;">Integrate Ecosystem →</a>
</div>

<div style="background: #1B2336; border: 1px solid #334155; border-radius: 8px; padding: 1.25rem;">
<h3 style="margin-top: 0; color: #22C55E;">📊 A100 Benchmarks</h3>
<p style="color: #94A3B8; font-size: 0.95rem;">Review empirical evaluation data across 4 frontier coding models on NVIDIA A100-80GB hardware across 5 defect benchmarks.</p>
<a href="/en/benchmarks/a100-evaluation" style="color: #22C55E; font-weight: 600; text-decoration: none;">Review Benchmarks →</a>
</div>

</div>
