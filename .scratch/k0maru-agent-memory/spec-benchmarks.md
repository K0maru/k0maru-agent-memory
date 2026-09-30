# Spec: K0maru-Agent-Memory Quantitative Benchmark Suite & CI Pipeline

Status: ready-for-agent

## Problem Statement

To prove that `k0maru-agent-memory` delivers substantial and measurable value to AI coding agents, the project needs reproducible, publication-grade quantitative benchmarks with verifiable data and charts. Specifically:
1. **Token & Cost Savings**: Quantify how much token budget is saved when using `k0maru offload` compared to dumping raw terminal/test logs into agent prompts (targeting 60%–80% reduction).
2. **System Efficiency & Zero-Daemon Advantage**: Quantify the lightweight advantages of the single static Rust binary (<5ms cold start, ~5MB RSS, 0 network ports) against heavyweight Docker/microservice competitors (TencentDB-Agent-Memory, Letta).
3. **Cache Reconstruction Speed**: Quantify the disposable cache's self-healing speed (<100ms full re-index).
4. **Continuous Quality Gate**: Automate non-flaky, deterministic benchmark runs in CI/CD without incurring paid API calls.

## Proposed Solution: In-Repo `benchmarks/` Suite

We establish a self-contained `benchmarks/` evaluation suite in the repository and wire it to `.github/workflows/benchmark.yml`.

### Directory Layout
```text
benchmarks/
├── README.md                     # Methodological documentation & one-command reproduction guide
├── fixtures/                     # Deterministic, sanitized real-world error traces
│   ├── cargo_build_error.log     # ~500 lines Rust compilation errors
│   ├── pytest_failures.log       # ~800 lines Python test failure tracebacks
│   ├── jest_test_failures.log    # ~1200 lines JS/TS test failure traces
│   └── multithread_crash.log     # ~2500 lines complex runtime stack dump
├── run_token_benchmark.py        # Token reduction evaluator (using tiktoken cl100k_base / o200k_base)
├── run_systems_benchmark.py      # Measures binary cold start, memory RSS, and cache rebuild throughput
├── generate_charts.py            # Generates publication-grade Matplotlib comparison charts
└── requirements.txt              # Minimal benchmark runner dependencies (tiktoken, matplotlib)
```

## User Stories

1. As an open-source evaluator or researcher, I want to run `python benchmarks/run_token_benchmark.py` and immediately inspect the Token Reduction Ratio (TRR %) across all standard fixtures.
2. As a maintainer preparing release documentation, I want to run `python benchmarks/generate_charts.py` to output publication-ready SVG/PNG comparison charts saved to `benchmarks/charts/` or `docs/assets/`.
3. As a CI/CD pipeline, I want `.github/workflows/benchmark.yml` to automatically verify that token savings remain >= 60% and cold start remains <= 20ms on every PR into `dev`, gating regressions.
4. As a systems reviewer, I want to run `python benchmarks/run_systems_benchmark.py` to measure binary launch latency and memory consumption side-by-side with Docker-based baseline profiles.

## Measurement Metrics & Formulas

1. **Token Reduction Ratio (TRR %)**:
   $$TRR = \frac{\text{Tokens}_{\text{raw}} - \text{Tokens}_{\text{offloaded}}}{\text{Tokens}_{\text{raw}}} \times 100\%$$
   Where tokens are measured using `tiktoken` (standard `cl100k_base` for GPT-4/Claude 3.5, and `o200k_base` for GPT-4o).
2. **Cold Start Latency**:
   Measured via high-precision timestamps (monotonic clock) for executing `k0maru --version` or `k0maru loadout --help` over 100 warm/cold runs.
3. **Cache Rebuild Throughput**:
   Measuring time in milliseconds to scan and index 1,000 markdown files into a fresh `cache.sqlite`.

## CI/CD Gating Rules
- **Assert TRR >= 60%**: If any fixture compression ratio drops below 60%, fail the step.
- **Assert Cold Start <= 25ms**: If median startup exceeds 25ms on GitHub runner, emit warning or fail.
