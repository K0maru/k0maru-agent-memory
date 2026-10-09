# 项目介绍与设计哲学

K0maru 是专为现代 AI 编码智能体打造的**单静态二进制、零常驻守护进程、轻量级长期记忆核心**。

## 核心设计哲学

- **Bring Your Own Markdown (BYOM)**：文件系统即唯一真理源。
- **Zero-Daemon**：进程即用即弃，冷启动耗时低于 5ms。
- **Disposable SQLite Index**：数据库仅作为快速检索缓存，损坏或删除后随时自动重建。
