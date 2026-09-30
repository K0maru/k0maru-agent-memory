# Issue 10: Benchmark Fixtures & Token Reduction Evaluator

Status: ready-for-agent
Blocked by: None

## Description

Create the core benchmark fixtures and token evaluation engine in `benchmarks/`:
1. `benchmarks/fixtures/`:
   - `cargo_build_error.log` (~500 lines of typical Rust compiler errors and warnings)
   - `pytest_failures.log` (~800 lines of Python tracebacks with assertion details)
   - `jest_test_failures.log` (~1200 lines of JavaScript/TypeScript test runner logs)
   - `multithread_crash.log` (~2500 lines of multi-thread panic and stack traces)
2. `benchmarks/requirements.txt`: Minimal dependencies (`tiktoken>=0.7`, `matplotlib>=3.8`).
3. `benchmarks/run_token_benchmark.py`:
   - Feeds each fixture into `k0maru offload` via stdin.
   - Calculates exact token counts for raw text vs. Mermaid-offloaded text using `tiktoken` (`cl100k_base` and `o200k_base`).
   - Computes Token Reduction Ratio ($TRR = (Tokens_{raw} - Tokens_{offloaded}) / Tokens_{raw} \times 100\%$).
   - Exports results to `benchmarks/results/token_savings.csv`.
   - Supports CLI flags: `--assert-min-reduction <float>` (e.g. `60.0`), exit code 0 on pass, 1 on regression.

## Acceptance Criteria
- [ ] All 4 fixture logs exist with realistic error traces.
- [ ] `python benchmarks/run_token_benchmark.py` runs cleanly against the compiled `target/release/k0maru` (or `target/debug/k0maru`).
- [ ] Generated `token_savings.csv` contains columns: `fixture`, `raw_lines`, `raw_tokens_cl100k`, `offloaded_tokens_cl100k`, `token_reduction_ratio_pct`.
- [ ] Average Token Reduction Ratio across fixtures is >= 65%.
- [ ] Passes `--assert-min-reduction 60.0`.
