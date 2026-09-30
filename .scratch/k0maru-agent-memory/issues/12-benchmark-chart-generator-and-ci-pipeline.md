# 12 — Publication-Grade Chart Generator & CI/CD Pipeline

**What to build:** Create the chart rendering script and wire the benchmark into GitHub Actions CI:
1. `benchmarks/generate_charts.py`:
   - Uses `matplotlib` with publication/academic style (clean typography, clear annotations, high-DPI export).
   - Generates three essential visual assets in `benchmarks/charts/`:
     - `token_savings_bar.png` (Raw vs. Offloaded tokens by workload with percentage reduction badges).
     - `cumulative_token_curve.png` (Simulated 10-turn debugging trajectory showing the widening token/cost gap).
     - `systems_log_comparison.png` (Logarithmic bar chart comparing Cold Start, RSS Memory, and Package Size between K0maru and Docker microservices).
2. `.github/workflows/benchmark.yml`:
   - Triggers on PR to `dev` and push to `main` / `dev`.
   - Builds `k0maru` in release mode.
   - Runs `python benchmarks/run_token_benchmark.py --assert-min-reduction 60.0`.
   - Validates that token reduction remains >= 60%.
   - Validates cold start latency <= 25ms in CI.
   - Preserves benchmark charts and CSVs as artifacts.
3. `benchmarks/README.md`:
   - Comprehensive documentation with quickstart commands to reproduce all charts and CSVs.

**Blocked by:** 10, 11

**Status:** resolved

## Acceptance Criteria
- [x] `python benchmarks/generate_charts.py` successfully exports all 3 PNG charts with >= 300 DPI into `benchmarks/charts/`.
- [x] `.github/workflows/benchmark.yml` is valid YAML and passes linting.
- [x] `benchmarks/README.md` clearly guides users on running the suite in one command.

---

### Completion Summary

1. **Publication Chart Generator (`benchmarks/generate_charts.py`)**:
   - Engineered using `matplotlib` with non-interactive `Agg` backend and publication typography (sans-serif, clean borders, custom palette).
   - Exports 3 high-resolution (300 DPI) figures to `benchmarks/charts/`:
     - `token_savings_bar.png` (187 KB): Grouped logarithmic bar chart with percentage reduction badges (-97.3% to -99.8% TRR).
     - `cumulative_token_curve.png` (300 KB): 10-turn trajectory showing vanilla agent exceeding the 128k context wall at Turn 6 (162.5k tokens) vs K0maru (1,505 tokens, 99.1% cumulative savings).
     - `systems_log_comparison.png` (208 KB): Logarithmic scale comparison of Cold Start Latency, Memory RSS, and Artifact Size across K0maru, TencentDB, and Letta.

2. **Automated Unit Tests (`benchmarks/test_chart_generator.py`)**:
   - Validates automated execution, PNG magic headers, and file size integrity.
   - Total test suite now runs 9 benchmark unit tests in 1.58s (`python3 -m unittest discover -s benchmarks`).

3. **CI/CD Performance Regression Gate (`.github/workflows/benchmark.yml`)**:
   - Triggered on PRs to `dev` and pushes to `main` / `dev`.
   - Automatically builds release binary with Rust cache.
   - Enforces Token Reduction Gate (`--assert-min-reduction 60.0`).
   - Enforces Systems Latency Gate (`--assert-max-cold-start-ms 25.0`).
   - Generates publication assets and uploads artifacts.

4. **Comprehensive Documentation (`benchmarks/README.md`)**:
   - Explains mathematical definitions (TRR %, latency percentiles, throughput files/sec, RSS).
   - Provides one-command reproduction guide and full architecture breakdown.
