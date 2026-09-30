# K0maru Benchmark Suite: Token Reduction & Systems Performance

Comprehensive evaluation suite measuring context window efficiency, cold start latency, memory footprint, and cache reconstruction throughput for **K0maru-Agent-Memory** compared to raw agent context retention and containerized microservices.

---

## 📊 Executive Summary of Results

| Evaluation Metric | K0maru-Agent-Memory | Industry Baseline (Vanilla / Docker) | Improvement / Advantage |
|:---|---:|---:|:---|
| **Average Token Reduction (TRR)** | **99.44%** | 0.0% (raw logs retained) | **~178× fewer tokens** |
| **Median Cold Start Latency** | **3.38 ms** | 2,500 – 3,200 ms (TencentDB / Letta) | **~750× faster boot** |
| **Peak Memory Footprint (RSS)** | **11.69 MB** | 850 – 1,800 MB | **~98.7% less memory** |
| **Cache Reconstruction Throughput** | **6,706.9 files/sec** | 119 – 138 files/sec | **~50× higher throughput** |
| **Open Network Ports** | **0** | 1 – 4 HTTP / DB ports | **Attack surface eliminated** |
| **Daemon Requirement** | **No (Zero-Daemon)** | Yes (Docker / Postgres / Background) | **Zero process management** |
| **Binary / Artifact Size** | **3.66 MB** | 1.2 – 2.5 GB Docker Image | **~400× smaller footprint** |

---

## 🖼️ Visual Assets

### 1. Context Window Footprint: Raw Logs vs. K0maru Symbolic Offload
Across four realistic error and panic workloads, K0maru reduces log token consumption by **97.29% to 99.81%** using symbolic Mermaid state graphs while preserving full diagnostic inspectability via zero-daemon seams (`k0maru inspect <node_id>`).

![Token Savings Bar](charts/token_savings_bar.png)

### 2. 10-Turn Debugging Trajectory: Cumulative Token Escalation
In a simulated 10-turn debugging loop, an un-offloaded vanilla agent exceeds standard 128k context limits at Turn 6 and accumulates **162,516 tokens**, driving massive inference costs and context pollution. K0maru maintains a compact context footprint of only **1,505 tokens** across all 10 turns (**99.1% cumulative savings**).

![Cumulative Token Curve](charts/cumulative_token_curve.png)

### 3. Systems Footprint: Native Binary vs. Containerized Microservices
Comparing cold start latency (ms), peak resident set size (MB), and artifact distribution size (MB) against TencentDB-Agent-Memory and Letta (MemGPT) on a logarithmic scale.

![Systems Log Comparison](charts/systems_log_comparison.png)

---

## 📂 Directory Structure

```text
benchmarks/
├── README.md                      # Comprehensive benchmark documentation
├── requirements.txt               # Minimal Python dependencies (tiktoken, matplotlib)
├── generate_fixtures.py           # Deterministic test fixture generator
├── run_token_benchmark.py         # Token reduction ratio (TRR %) evaluation engine
├── run_systems_benchmark.py       # Cold start, RSS memory, and cache throughput runner
├── generate_charts.py             # 300+ DPI publication figure rendering script
├── test_token_benchmark.py        # Automated unit tests for token benchmark engine
├── test_systems_benchmark.py      # Automated unit tests for systems performance runner
├── test_chart_generator.py        # Automated unit tests for chart generation pipeline
├── fixtures/                      # Realistic log fixtures
│   ├── cargo_build_error.log      # 500 lines (compiler passes, E0502, E0308, E0277)
│   ├── pytest_failures.log        # 800 lines (tracebacks, assertion diffs, logs)
│   ├── jest_test_failures.log     # 1,200 lines (async timeouts, component diffs)
│   └── multithread_crash.log      # 2,500 lines (16-thread panics, register dumps)
├── results/                       # Exported benchmark CSV datasets
│   ├── token_savings.csv          # Raw vs offloaded token counts and TRR %
│   └── systems_benchmark.csv      # System comparisons and latency distributions
└── charts/                        # Publication figures (>= 300 DPI)
    ├── token_savings_bar.png
    ├── cumulative_token_curve.png
    └── systems_log_comparison.png
```

---

## 🚀 Quickstart & One-Command Reproduction

### 1. Compile the Release Binary
Ensure Rust 1.75+ is installed:
```bash
cargo build --release
```

### 2. Install Python Dependencies
```bash
pip install -r benchmarks/requirements.txt
```

### 3. Run All Benchmarks in One Command
```bash
# 1. Run Token Reduction Evaluation (Strict gate: TRR >= 60%)
python3 benchmarks/run_token_benchmark.py --assert-min-reduction 60.0

# 2. Run Systems Performance & Latency Evaluation (Gate: Cold Start <= 10ms)
python3 benchmarks/run_systems_benchmark.py --assert-max-cold-start-ms 10.0

# 3. Render Publication-Grade Charts
python3 benchmarks/generate_charts.py
```

### 4. Run Benchmark Test Suite
```bash
python3 -m unittest discover -s benchmarks
```

---

## 📐 Benchmark Methodology

### 1. Token Reduction Ratio (TRR %)
- **Formula:**
  $$\text{TRR} = \frac{\text{Tokens}_{\text{raw}} - \text{Tokens}_{\text{offloaded}}}{\text{Tokens}_{\text{raw}}} \times 100\%$$
- **Tokenizers Tested:**
  - `cl100k_base` (GPT-4 / Claude / standard frontier LLM baseline)
  - `o200k_base` (GPT-4o)
- **Workloads:**
  - `cargo_build_error.log` (500 lines): Typical multi-crate compilation failure.
  - `pytest_failures.log` (800 lines): Unit and integration test failure tracebacks.
  - `jest_test_failures.log` (1,200 lines): Front-end/Node.js testing suite errors.
  - `multithread_crash.log` (2,500 lines): High-concurrency runtime abort and thread dumps.

### 2. Cold Start Latency
- Measured via high-precision monotonic clock (`time.perf_counter_ns()`).
- Executes `k0maru --version` 100 iterations.
- Tracks `min`, `mean`, `median (p50)`, `p95`, `p99`, and `max`.

### 3. Synthetic Cache Reconstruction Throughput
- Synthesizes 500 isolated Markdown documents with YAML frontmatter, [[WikiLinks]], and tags.
- Measures wall-clock execution of `k0maru sync --vault <dir> --json` from an empty cache state.
- Reports throughput in **files processed per second** and subsequent incremental zero-dirty check latency.

### 4. Memory Resident Set Size (RSS)
- Captures maximum resident set size of child execution processes using `resource.getrusage(resource.RUSAGE_CHILDREN)` across platform conventions (bytes on macOS Darwin, KB on Linux).

---

## 🛡️ CI/CD Regression Gate Policy

All benchmarks are wired into GitHub Actions (`.github/workflows/benchmark.yml`):
- **Pull Requests to `dev`** and **Pushes to `main`/`dev`** trigger automated benchmark validation.
- Pull requests fail automatically if:
  1. Token Reduction Ratio falls below **60.0%** for any fixture.
  2. Cold start median latency exceeds **25.0 ms** on virtualized runners.
  3. Any of the 8 automated benchmark unit tests fail.
- Benchmark CSVs and generated charts are preserved as downloadable artifacts for 14 days.
