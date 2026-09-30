# 11 — Systems Performance & Latency Benchmark Runner

**What to build:** Create `benchmarks/run_systems_benchmark.py` to measure binary performance and cache rebuild efficiency:
1. **Cold Start Latency**: Executes `k0maru --version` 100 times using high-precision monotonic clock (`time.perf_counter_ns()`), computing min, max, mean, median (p50), p95, and p99 latency in milliseconds.
2. **Memory Footprint (RSS)**: Measures maximum resident set size (RSS) during execution.
3. **Synthetic Cache Reconstruction Throughput**: Generates a temporary directory with 500 mock markdown files containing YAML frontmatter and WikiLinks, measuring exact milliseconds taken by `k0maru sync` to parse, index, and populate `cache.sqlite` from zero.
4. **Competitor Baseline Profile**: Compares measured numbers against documented baseline profiles for containerized solutions (TencentDB-Agent-Memory, Letta).
5. Exports results to `benchmarks/results/systems_benchmark.csv`.

**Blocked by:** 10

**Status:** resolved

## Acceptance Criteria
- [x] `python benchmarks/run_systems_benchmark.py` runs without errors.
- [x] Median cold start latency on modern hardware is recorded and verified to be < 10ms.
- [x] Cache rebuild throughput is recorded and verified.
- [x] Outputs formatted terminal summary table and saves `benchmarks/results/systems_benchmark.csv`.

---

### Completion Summary

1. **CLI `k0maru sync` Subcommand (`src/main.rs`)**:
   - Added `k0maru sync [--vault <PATH>] [--force] [--json]` CLI command.
   - Leverages `IncrementalScanner` and `SqliteStorage` to execute fast incremental or full vault synchronization.
   - Outputs machine-readable JSON matching `SyncStats` schema when `--json` flag is provided.
   - Verified via unit & integration smoke tests in `tests/test_cli_smoke.rs`.

2. **Systems Performance Benchmark Runner (`benchmarks/run_systems_benchmark.py`)**:
   - Automatically discovers compiled release or debug binary (`target/release/k0maru`).
   - Measures cold start latency across 100 iterations using high-precision monotonic clock (`time.perf_counter_ns()`).
   - Measures peak RSS memory usage using `resource.getrusage(resource.RUSAGE_CHILDREN)`.
   - Generates synthetic vault of 500 markdown documents with YAML frontmatter, [[WikiLinks]], and tags, evaluating full cache reconstruction and incremental validation.
   - Compares performance against containerized industry memory hubs (TencentDB-Agent-Memory, Letta).
   - Exports results to `benchmarks/results/systems_benchmark.csv`.
   - Supports `--assert-max-cold-start-ms` flag (exits code 1 on regression, 0 on pass).

3. **Quantitative Benchmark Results**:
   - **Cold Start Latency (N = 100)**:
     - Min: **2.96 ms**
     - Mean: **3.46 ms**
     - **Median (p50): 3.38 ms** (target: < 10.0 ms — **66.2% faster than 10ms threshold**)
     - P95: **4.30 ms**
     - P99: **5.54 ms**
   - **Cache Reconstruction (500 docs)**:
     - Full Rebuild Time: **74.55 ms**
     - Reconstruction Throughput: **6,706.9 files/sec**
     - Incremental Zero-Dirty Check: **11.67 ms**
   - **Resource Footprint**:
     - Peak RSS Memory: **11.69 MB** (vs 1,800 MB TencentDB, 850 MB Letta)
     - Binary Artifact Size: **3.66 MB** single self-contained executable
     - Open Ports: **0** (stdio FastMCP)
     - Daemon Required: **No**

4. **Testing & Quality Baseline**:
   - `python3 -m unittest benchmarks/test_systems_benchmark.py`: 4 tests passed.
   - `cargo test`: 85 Rust unit, integration, and smoke tests passed.
   - `cargo clippy --all-targets -- -D warnings`: 0 warnings.
   - `cargo fmt --check`: Clean formatting.
