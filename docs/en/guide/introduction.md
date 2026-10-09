# Introduction & Philosophy

K0maru is a **cleanroom, single-static-binary, zero-daemon long-term memory hub** specifically engineered for AI coding agents.

## Core Design Principles

- **Bring Your Own Markdown (BYOM)**: Your filesystem is the sole source of truth.
- **Zero-Daemon Architecture**: Ephemeral execution with cold-start latency under 5ms.
- **Disposable SQLite Index**: The database is an auxiliary index cache that can be reconstructed on demand.
