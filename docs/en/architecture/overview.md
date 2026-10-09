# Architectural Overview & Data Flow

K0maru is engineered around a four-layer decoupled architecture:

1. **Presentation & Protocol Layer**: CLI subcommands, FastMCP stdio server, Axum WebUI console.
2. **Domain & Processing Layer**: Incremental scanner state machine, Convention sniffer, Heuristic distill engine, Flush writer.
3. **Retrieval & Fusion Layer**: BM25 FTS5 lexical engine, ONNX FastEmbed semantic embeddings, and 1-hop WikiLinks graph boost fused via Reciprocal Rank Fusion ($k=60$).
4. **Storage Layer**: Human-readable Markdown repository (single source of truth) with disposable SQLite cache.
