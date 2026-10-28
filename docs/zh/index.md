---
layout: home

hero:
  name: "K0maru Agent Memory"
  text: "AI 编码智能体长期记忆中枢"
  tagline: "单静态二进制 · 零常驻守护进程 · 透明挂载 Markdown 知识库与本地 LLM-Wiki"
  actions:
    - theme: brand
      text: 🚀 3 分钟极速起步
      link: /zh/guide/quickstart
    - theme: alt
      text: 🏗️ 架构与内核总览
      link: /zh/architecture/overview
    - theme: alt
      text: 📦 安装与配置
      link: /zh/guide/installation
    - theme: alt
      text: 📊 A100 实测看板
      link: /zh/benchmarks/a100-evaluation

features:
  - icon: ⚡
    title: "单静态二进制 · 零守护进程"
    details: "纯 Rust 静态编译，仅 3.66 MB 单文件。无须 Docker、无须 Python、无须常驻后台进程，冷启动仅需 3.38 ms，执行完毕即刻释放全部内存。"
  - icon: 📝
    title: "自备纯文本 Markdown (BYOM)"
    details: "本地 Markdown 笔记（Obsidian Vault、Karpathy LLM-Wiki）即唯一真理源。零格式锁死，SQLite 仅作为随时可删除重建的瞬态缓存。"
  - icon: 🎯
    title: "多路混合检索 · 双链图谱加权"
    details: "BM25 词法倒排全文检索 + ONNX FastEmbed 向量语义 + 1-Hop WikiLinks 双向图谱拓扑加权（+0.05 Boost），通过 RRF (k=60) 融合排序。"
  - icon: 🛡️
    title: "终端日志符号化卸载 (99.4% 压缩)"
    details: "Unix 管道过滤器自动识别并拦截千行编译器崩溃与测试回溯，提炼为 15 行 Mermaid 状态机时序图，彻底终结上下文爆炸与注意力漂移。"
  - icon: 🔌
    title: "FastMCP 官方协议开箱即用"
    details: "一键挂载 Claude Code、Cursor、Windsurf、Gemini CLI、Cline、Nous Research Hermes Agent 与 OpenClaw，AI 自然语言意图自主唤醒。"
  - icon: 📈
    title: "工业级真实 A100 实测验证"
    details: "NVIDIA A100 80GB 云端实测：Qwen2.5-Coder-32B Pass@1 提升 40% 并提速 5.90 倍；DeepSeek-R1 推理延迟腰斩 48.2%。"
---

<div class="developer-hero-section" style="margin-top: 2.5rem; text-align: center;">

## ⚡ 终端极速体验

只需一条命令即可完成安装并挂载你的私有 Markdown 知识库：

```bash
# 1. 官方脚本一键安装二进制
curl -fsSL https://raw.githubusercontent.com/K0maru/k0maru-agent-memory/main/install.sh | bash

# 2. 一键挂载本地知识库至 Claude Code / Cursor / Windsurf
k0maru install --vault ~/Documents/MyVault

# 3. 运行环境健康自检
k0maru doctor
```

</div>

<div class="developer-cards-grid" style="display: grid; grid-template-columns: repeat(auto-fit, minmax(240px, 1fr)); gap: 1.25rem; margin-top: 2rem;">

<div style="background: #1B2336; border: 1px solid #334155; border-radius: 8px; padding: 1.25rem;">
<h3 style="margin-top: 0; color: #22C55E;">🚀 快速起步</h3>
<p style="color: #94A3B8; font-size: 0.95rem;">3 分钟完成从环境自检、架构记忆提取、终端日志卸载到决策结晶落盘的全流程闭环。</p>
<a href="/zh/guide/quickstart" style="color: #22C55E; font-weight: 600; text-decoration: none;">进入快速起步 →</a>
</div>

<div style="background: #1B2336; border: 1px solid #334155; border-radius: 8px; padding: 1.25rem;">
<h3 style="margin-top: 0; color: #22C55E;">🏗️ 架构设计</h3>
<p style="color: #94A3B8; font-size: 0.95rem;">四层解耦模型、知识库 L0–L3 语义分级机制与 xxh3 增量扫描状态机底层剖析。</p>
<a href="/zh/architecture/overview" style="color: #22C55E; font-weight: 600; text-decoration: none;">深入架构内核 →</a>
</div>

<div style="background: #1B2336; border: 1px solid #334155; border-radius: 8px; padding: 1.25rem;">
<h3 style="margin-top: 0; color: #22C55E;">🤖 智能体生态</h3>
<p style="color: #94A3B8; font-size: 0.95rem;">全面兼容 Claude Code、Cursor、Windsurf、Hermes Agent 与 OpenClaw，附系统提示词最佳范式。</p>
<a href="/zh/workflows/ecosystem" style="color: #22C55E; font-weight: 600; text-decoration: none;">查看生态整合 →</a>
</div>

<div style="background: #1B2336; border: 1px solid #334155; border-radius: 8px; padding: 1.25rem;">
<h3 style="margin-top: 0; color: #22C55E;">📊 A100 实测看板</h3>
<p style="color: #94A3B8; font-size: 0.95rem;">基于 NVIDIA A100-80GB 的 Qwen2.5-Coder、DeepSeek-R1 等 4 大开源大模型端到端实测数据。</p>
<a href="/zh/benchmarks/a100-evaluation" style="color: #22C55E; font-weight: 600; text-decoration: none;">查阅实测看板 →</a>
</div>

</div>
