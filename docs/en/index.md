---
layout: home

hero:
  name: "K0maru"
  text: "Long-Term Memory Hub for AI Coding Agents"
  tagline: "Single static binary, zero-daemon, transparently mounting Obsidian vaults and LLM-Wikis"
  actions:
    - theme: brand
      text: 🚀 3-Minute Quickstart
      link: /en/guide/quickstart
    - theme: alt
      text: 🏗️ Architecture Overview
      link: /en/architecture/overview
    - theme: alt
      text: 📊 A100 Benchmark Dashboard
      link: /en/benchmarks/a100-evaluation

features:
  - icon: ⚡
    title: "Single Static Binary · Zero Daemon"
    details: "Crafted in high-performance Rust with <15MB RAM footprint and <5ms cold-start. Zero Docker, zero Python runtime, zero background daemons."
  - icon: 🧠
    title: "BYOM Philosophy · Zero Vendor Lock-in"
    details: "Bring Your Own Markdown. Human-readable Markdown is the single source of truth; SQLite acts solely as a disposable index cache."
  - icon: 🎯
    title: "Hybrid RRF Retrieval & Graph Boost"
    details: "Fuses BM25 lexical match, ONNX FastEmbed semantic vectors, and 1-hop bidirectional WikiLinks graph boost via Reciprocal Rank Fusion."
  - icon: 🛡️
    title: "Symbolic Log Offloading"
    details: "Compresses 1,000+ lines of raw compiler traces into a 15-line Mermaid state machine, slashing token overhead by up to 97%."
  - icon: 🔌
    title: "FastMCP Stdio Ready"
    details: "Out-of-the-box integration for Claude Code, Cursor, Windsurf, Gemini CLI, Cline, Hermes Agent, and OpenClaw."
  - icon: 📈
    title: "Proven Production Benchmarks"
    details: "Evaluated on NVIDIA A100 80GB: +40% Pass@1 gain, 5.90x latency reduction, and 23.2% Token Reduction Ratio."
---
