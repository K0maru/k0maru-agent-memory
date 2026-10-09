# 核心算法与关键模块深度实现

本章深度剖析 K0maru 的关键工程实现细节与核心算法。

## 核心实现主题

- **xxh3 增量扫描**：0-I/O 脏文件判定与 mtime 边界缓存。
- **RRF 混合检索与图谱加权**：BM25、ONNX 向量与 1-Hop 双向 WikiLinks 图谱跃迁。
- **FastMCP Stdio**：零常驻进程通信与自然语言意图对齐。
- **符号化日志压缩**：纯 Rust 状态机提取与脱敏。
- **ConventionSniffer**：异构知识库拓扑嗅探。
- **FlushEngine**：原子写与防碰撞重命名机制。
- **Trace-to-Skill**：编译器报错到动态技能启发式合成。
