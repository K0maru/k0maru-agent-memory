# Issue 12: Publication-Grade Chart Generator & CI/CD Pipeline

Status: ready-for-agent
Blocked by: 10, 11

## Description

Create the chart rendering script and wire the benchmark into GitHub Actions CI:
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
3. `benchmarks/README.md`:
   - Comprehensive documentation with quickstart commands to reproduce all charts and CSVs.

## Acceptance Criteria
- [ ] `python benchmarks/generate_charts.py` successfully exports all 3 PNG charts with >= 300 DPI into `benchmarks/charts/`.
- [ ] `.github/workflows/benchmark.yml` is valid YAML and passes linting.
- [ ] `benchmarks/README.md` clearly guides users on running the suite in one command.
