# 四层架构与核心数据流

K0maru 采用高度解耦的四层微内核架构：

1. **协议与表现层 (Presentation & Protocol)**：CLI、FastMCP Stdio、Axum WebUI。
2. **领域处理层 (Domain & Processing)**：增量扫描器、拓扑嗅探器、技能提炼器、结晶写入器。
3. **检索融合层 (Retrieval & Fusion)**：BM25 词法全文检索 + ONNX FastEmbed + 1-Hop 图谱跃迁加权 RRF 排序。
4. **物理存储层 (Storage)**：本地 Markdown 库（唯一真理源）与 SQLite 瞬态缓存。
