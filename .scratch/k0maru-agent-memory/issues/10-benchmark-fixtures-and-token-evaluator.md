# 10 — Benchmark Fixtures & Token Reduction Evaluator

**What to build:** Create the core benchmark fixtures and token evaluation engine in `benchmarks/`: 4 realistic error/panic log fixtures, Python dependencies in `requirements.txt`, and an automated evaluation runner `benchmarks/run_token_benchmark.py` calculating token reduction ratios with `tiktoken` against `k0maru offload`.

**Blocked by:** None

**Status:** resolved

## Acceptance Criteria
- [x] All 4 fixture logs exist with realistic error traces.
- [x] `python benchmarks/run_token_benchmark.py` runs cleanly against the compiled `target/release/k0maru` (or `target/debug/k0maru`).
- [x] Generated `token_savings.csv` contains columns: `fixture`, `raw_lines`, `raw_tokens_cl100k`, `offloaded_tokens_cl100k`, `token_reduction_ratio_pct`.
- [x] Average Token Reduction Ratio across fixtures is >= 65%.
- [x] Passes `--assert-min-reduction 60.0`.

---

### Completion Summary

1. **Benchmark Fixtures (`benchmarks/fixtures/`)**:
   - `cargo_build_error.log`: 500 lines of realistic Rust compiler output (`cargo build`) with dependency compiles, borrow checker errors (`E0502`), type mismatches (`E0308`), trait bounds (`E0277`), and unused variable warnings.
   - `pytest_failures.log`: 800 lines of detailed Python tracebacks with assertion diffs, session summary, and captured stderr/stdout.
   - `jest_test_failures.log`: 1200 lines of TypeScript / Jest test runner logs with asynchronous failure traces, component snapshot assertions, and network timeouts.
   - `multithread_crash.log`: 2500 lines of multi-threaded panic stack traces across 16 tokio/database threads, crash register dumps, and memory maps.
   - Generator script provided in `benchmarks/generate_fixtures.py` for deterministic reproducibility.

2. **Benchmark Engine & Evaluator (`benchmarks/run_token_benchmark.py`)**:
   - Discovers `k0maru` binary (`target/release/k0maru` fallback `target/debug/k0maru` or custom `--bin`).
   - Evaluates raw log files against `k0maru offload` standard input pipe.
   - Calculates exact token counts using `tiktoken` with `cl100k_base` and `o200k_base`.
   - Computes Token Reduction Ratio ($TRR = (Tokens_{raw} - Tokens_{offloaded}) / Tokens_{raw} \times 100\%$).
   - Exports results to `benchmarks/results/token_savings.csv`.
   - Implements `--assert-min-reduction <float>` CLI assertion flag (exits with code 1 on regression, 0 on pass).
   - Auto-discovers local repository virtualenv site-packages if run with system python.

3. **Quantitative Results**:
   | Fixture | Raw Lines | Raw (cl100k) | Offloaded | TRR (cl100k %) | Raw (o200k) | Offloaded | TRR (o200k %) |
   |---|---:|---:|---:|---:|---:|---:|---:|
   | `cargo_build_error.log` | 500 | 5,455 | 148 | **97.29%** | 5,409 | 151 | **97.21%** |
   | `pytest_failures.log` | 800 | 10,621 | 152 | **98.57%** | 10,686 | 155 | **98.55%** |
   | `jest_test_failures.log` | 1,200 | 12,362 | 151 | **98.78%** | 12,616 | 154 | **98.78%** |
   | `multithread_crash.log` | 2,500 | 78,943 | 151 | **99.81%** | 79,523 | 154 | **99.81%** |
   | **OVERALL AVERAGE** | - | 107,381 | 602 | **99.44%** | 108,234 | 614 | **99.43%** |

4. **Testing & Verification**:
   - `python3 -m unittest benchmarks/test_token_benchmark.py`: 4 unit tests passing 100%.
   - `cargo test`: 84 Rust unit/integration tests passing 100%.
   - `cargo clippy --all-targets -- -D warnings`: 0 warnings.
   - `cargo fmt --check`: Clean formatting.
