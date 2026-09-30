# Issue 11: Systems Performance & Latency Benchmark Runner

Status: ready-for-agent
Blocked by: 10

## Description

Create `benchmarks/run_systems_benchmark.py` to measure binary performance and cache rebuild efficiency:
1. **Cold Start Latency**:
   - Executes `k0maru --version` 100 times using high-precision monotonic clock (`time.perf_counter_ns()`).
   - Computes min, max, median (p50), p95, and p99 latency in milliseconds.
2. **Memory Footprint (RSS)**:
   - Measures maximum resident set size (RSS) during execution.
3. **Synthetic Cache Reconstruction Throughput**:
   - Generates a temporary directory with 500 mock markdown files containing YAML frontmatter and WikiLinks.
   - Measures exact milliseconds taken by `k0maru sync` to parse, index, and populate `cache.sqlite` from zero.
4. **Competitor Baseline Profile**:
   - Compares measured numbers against documented baseline profiles for containerized solutions (TencentDB-Agent-Memory, Letta).
   - Exports results to `benchmarks/results/systems_benchmark.csv`.

## Acceptance Criteria
- [ ] `python benchmarks/run_systems_benchmark.py` runs without errors.
- [ ] Median cold start latency on modern hardware is recorded and verified to be < 10ms.
- [ ] Cache rebuild throughput is recorded and verified.
- [ ] Outputs formatted terminal summary table and saves `benchmarks/results/systems_benchmark.csv`.
