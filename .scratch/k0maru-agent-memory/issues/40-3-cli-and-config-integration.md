# Ticket 40-3: CLI Flags, Config/Env Var Resolution & Doctor Integration

**Status**: Completed
**Blocked by**: Ticket 40-1, Ticket 40-2

## Goal
Integrate `--embed-model` flag into CLI commands (`sync`, `search`, `mcp`, `ui`), wire environment variable and configuration file resolution, and update `k0maru doctor` to inspect and report active embedding model status.

## Acceptance Criteria
1. CLI Argument Updates:
   - Add `--embed-model <MODEL>` (alias `--embedder`) to `SyncArgs`, `SearchArgs`, and `McpArgs` in `src/main.rs`.
   - Update `run_sync` to call `resolve_embedding_engine(args.embed_model.as_deref(), model_cache)`.
   - In `run_sync`, before calling `VectorSyncEngine::sync`, invoke `storage.ensure_vector_dimension(engine.dimension())` and log an informative notification if a migration was performed.
2. Update `k0maru doctor`:
   - Probe configured/active embedding model name and output dimensions.
   - Check if model cache directory exists and contains downloaded model weights.
   - Output structured check in terminal and `--json` diagnostic report.
3. Integration tests in `tests/test_embed_cli.rs`:
   - `k0maru sync --vector --embed-model mock` succeeds and writes vectors.
   - `k0maru doctor --json` reports embedding engine metadata.
   - `k0maru search --vector "query" --embed-model mock` executes without crashing.
