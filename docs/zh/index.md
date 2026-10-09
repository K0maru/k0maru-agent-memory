---
layout: home

hero:
  name: "K0maru"
  text: "AI 编码智能体长期记忆核心"
  tagline: "单静态二进制、零常驻守护进程、透明挂载 Markdown 知识库与 LLM-Wiki"
  actions:
    - theme: brand
      text: 🚀 3 分钟极速起步
      link: /zh/guide/quickstart
    - theme: alt
      text: 🏗️ 架构与内核总览
      link: /zh/architecture/overview
    - theme: alt
      text: 📊 A100 实测看板
      link: /zh/benchmarks/a100-evaluation

features:
  - icon: ⚡
    title: "单静态二进制 · 零常驻"
    details: "基于 Rust 极致编译，内存开销 <15MB，冷启动 <5ms。无须 Docker、无须 Python、无须启动后台守护进程。"
  - icon: 🧠
    title: "BYOM 哲学 · 拒绝数据锁死"
    details: "坚持 \"Bring Your Own Markdown\"。纯文本文件系统即唯一真理源，SQLite 仅作为随时可抛弃的瞬态索引缓存。"
  - icon: 🎯
    title: "混合检索 · 图谱加权"
    details: "BM25 词法全文检索 + ONNX FastEmbed 向量语义 + 1-Hop 双向 WikiLinks 图谱跃迁，通过 RRF (k=60) 融合排序。"
  - icon: 🛡️
    title: "终端日志状态机离线压缩"
    details: "自动识别并提炼 1,000+ 行编译器报错与测试堆栈，提炼为 15 行 Mermaid 状态机图谱，Token 消耗骤降 97%。"
  - icon: 🔌
    title: "FastMCP 开箱即用"
    details: "零配置无缝接入 Claude Code、Cursor、Windsurf、Gemini CLI、Cline、Hermes Agent 与 OpenClaw。"
  - icon: 📈
    title: "实测工业级表现"
    details: "NVIDIA A100 80GB 实测，Pass@1 提高 40%，响应耗时缩减 5.90x，Token 消耗节省 23.2%。"
---
