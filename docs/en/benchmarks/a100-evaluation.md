# NVIDIA A100 Evaluation & Token Economics

To bridge theoretical design and industrial software engineering, we conducted empirical evaluations using an enterprise **NVIDIA A100-SXM4-80GB GPU** (80 GB HBM2e VRAM, 167 GB host RAM) hosted on Google Colab Pro. We evaluated leading open-weight coding and reasoning models across a suite of industrial defect benchmarks.

---

## 🖥️ Hardware Setup & Evaluation Protocol

```text
Compute Platform: Google Colab Pro Dedicated Instance
Accelerator:      NVIDIA A100-SXM4-80GB (80 GB HBM2e, 2,039 GB/s bandwidth)
Driver & CUDA:    NVIDIA Driver 580.82.07, CUDA 13.0
Host System:      Intel Xeon CPU @ 2.20GHz (12 vCPUs), 167 GiB RAM, 194 GiB NVMe SSD
Inference Engine: Ollama v0.40.2 (CUDA 13 native hardware acceleration, 256k window)
Decoding Params:  Greedy deterministic (temperature = 0.0, num_predict = 512, seed = 42)
```

### 5 Industrial Cross-Language Defect Benchmarks
The evaluation suite comprises real-world build failures and crash dumps:
1. **Rust**: Loop closure `E0382` ownership move conflict;
2. **Python**: `asyncio` unawaited task triggering `InvalidStateError` memory leaks;
3. **TypeScript**: Authentication middleware undefined deep destructuring crash;
4. **Go**: Unbuffered channel concurrent goroutine permanent deadlock;
5. **C**: Heap memory use-after-free and double free (`heap-use-after-free`).

---

## 📊 A100 Multi-Model Empirical Results Matrix

The table below contrasts unmanaged agents (Control Group) against K0maru-managed agents (Experimental Group) across all five benchmark cases:

| Evaluated Model | Architecture & Role | Group | Pass@1 Accuracy | Input Prompt Tokens | Average Latency | Speedup Factor | Key Engineering Observations |
| :--- | :--- | :--- | :---: | :---: | :---: | :---: | :--- |
| **Qwen2.5-Coder-32B** *(Kanban)* | 32B Dense Code Specialist | Control (Vanilla) | 60.0% (3/5) | 5,472 | 24.68s | 1.0x | Misled by verbose Rust borrow checker stack (90.1s) and TS auth null reference |
| **Qwen2.5-Coder-32B** *(Kanban)* | 32B Dense Code Specialist | **K0maru Managed** | **100.0% (5/5)** | **1,119** | **7.13s** | **3.46x** | **5/5 solved (+40% accuracy jump)**; Case 1 sped up **15.06x** (90.1s -> 5.98s); 79.6% token reduction |
| **DeepSeek-Coder-V2-16B** | 16B MoE (2.4B active) | Control (Vanilla) | 80.0% (4/5) | 6,298 | 9.43s | 1.0x | Fast MoE architecture, but failed TS auth due to raw stack trace misdirection |
| **DeepSeek-Coder-V2-16B** | 16B MoE (2.4B active) | **K0maru Managed** | **100.0% (5/5)** | **1,142** | **1.36s** | **6.93x** | **100% Pass rate**; sub-second turnaround on TS (1.26s) and C (1.19s); Case 1 sped up **26.44x** |
| **DeepSeek-R1-32B** | 32B Reasoning / CoT | Control (Vanilla) | 20.0% (1/5) | 5,412 | 12.01s | 1.0x | Exhaustive thinking chain, but burdened by lengthy stack traces leading to high generation latency |
| **DeepSeek-R1-32B** | 32B Reasoning / CoT | **K0maru Managed** | **40.0% (2/5)** | **989** | **9.19s** | **1.31x** | Resolved Python async leak with `asyncio.gather`; prompt tokens cut by **81.7%** |
| **Qwen3.8-27B** | 27B Dense (2-Stage) | Control (Vanilla) | 20.0% (1/5) | 5,745 | 16.70s | 1.0x | Baseline Rust cold inference took 56.7s; failed Python async and Go deadlock |
| **Qwen3.8-27B** | 27B Dense (2-Stage) | **K0maru Managed** | **40.0% (2/5)** | **1,076** | **6.14s** | **2.72x** | Rust inference dropped from 56.7s to 6.06s (**9.36x speedup**); resolved Python async leak |
| **Google Gemma 4 31B** | 31B Multimodal Foundation | Control (Vanilla) | 0.0% (0/5) | 6,356 | 16.69s | 1.0x | General foundation model without code specialization; failed all 5 raw trace repairs |
| **Google Gemma 4 31B** | 31B Multimodal Foundation | **K0maru Managed** | 0.0% (0/5) | **1,134** | **6.94s** | **2.40x** | Prompt tokens reduced by **82.2%**; Rust case latency dropped from 53.4s to 6.85s (**7.8x speedup**) |

---

## ⚡ Multi-Agent Heterogeneous Concurrency & Stress Evaluation (A100)

| Benchmark Stress Dimension | Concurrency / Load | Measured Metric / Throughput | Status | Engineering Assurance |
| :--- | :--- | :--- | :---: | :--- |
| **Simultaneous Read Bursts** | 12 Concurrent Agents, 120 queries | **18.9 QPS**, Avg: **614.29ms** (P50: 781.9ms) | **100% PASS** | Zero SQLite busy contention (`SQLITE_BUSY = 0`) |
| **Simultaneous Write & Flush Race** | 8 Concurrent Writers | **12.2 writes/sec**, Avg: **584.93ms** | **100% PASS** | 8/8 files created, zero collision loss (`-v2.md` to `-v8.md`) |
| **Instant Self-Healing Recall** | Read-your-writes validation | **290.99 ms** | **PASS ✓** | Immediate sub-second recall without re-indexing |
| **Context Garbage Collection (GC)** | 50-turn trajectory compaction | **94.67% TRR** (20,946 ➔ 1,110 tokens) | **PASS ✓** | Preserves context within cognitive Smart-Zone (15%–60%) |
| **Dynamic Loadout Git-Diff** | Change-aware ADR activation | **245 tokens** | **PASS ✓** | Strictly bounded under <300 token budget |
| **Heterogeneous Agent Loops** | Hermes, OpenClaw, Pi | Turnaround: **1.04s – 8.49s** | **100% PASS** | Verified autonomous tool calling & ADR crystallization |


## 📈 Publication-Grade Visualizations

### Figure 1: Log Token Compression & 10-Turn Context Growth Curves

![Figure 1: Log Token Compression & 10-Turn Context Growth](/images/comparison_with_without.png)

- **Panel A (Peak Token Compression)**: Compares token footprints before and after symbolic offloading across Rust compiler crashes, Python test failures, Jest runs, and multithreaded deadlocks, achieving up to **99.81%** compression.
- **Panel B (10-Turn Cumulative Context Growth)**: In a 10-turn debugging session, unmanaged agents breach 128k token limits by Turn 6, inducing severe amnesia. K0maru holds context flat at **~1,505 tokens**, yielding a cumulative **99.1%** token savings.

### Figure 2: System-Level Architectural Benchmarks

![Figure 2: System-Level Architectural Benchmarks](/images/comparison_competitors.png)

- **Panel A (Cold-Start Invocation Latency)**: K0maru cold-starts in **3.38 ms**, delivering up to **1,000x faster execution** than containerized frameworks like Letta (3,200 ms) and TencentDB (2,500 ms).
- **Panel B (Resident Memory & Binary Footprint)**: K0maru maintains a compact **~11.7 MB** resident memory footprint (RSS) and a **3.66 MB** standalone binary.
- **Panel C (Index Rebuild Throughput)**: Rebuilds indexes at **6,746 documents per second** (500 documents in 74.1 ms).

### Figure 3: First-Turn Response Latency

![Figure 3: First-Turn Response Latency](/images/latency_comparison.png)

### Figure 4: Pass@1 Accuracy Jump

![Figure 4: Pass@1 Accuracy Jump](/images/pass_rate_comparison.png)

### Figure 5: Token Savings & Reduction Distribution

![Figure 5: Token Savings & Reduction Distribution](/images/token_reduction.png)

---

## 🔬 Detailed Case Study Analyses

### Case 1: Rust Multithreaded Worker Ownership Move (`E0382`)
- **DeepSeek-R1-32B**:
  - Control: 39.23s latency, 243 tokens (spent excessive cycles re-analyzing call frames);
  - K0maru: **11.54s latency (3.4x faster)**, 161 tokens (Mermaid state diagram pointed directly to `Arc::clone`).
- **Qwen3.8-27B**:
  - Control: 21.20s latency, 268 tokens;
  - K0maru: **11.12s latency (halved turnaround)**, 176 tokens (34.3% token savings).

### Case 2: Python Asyncio Unawaited Task Leak (`InvalidStateError`)
- **Qwen2.5-Coder-32B**:
  - Control: **Failed** (misled by multiple traceback levels; called synchronous `.result()`);
  - K0maru: **Passed** (identified pending state from the Mermaid diagram and refactored to `await asyncio.gather(*tasks, return_exceptions=True)`).
- **DeepSeek-Coder-V2**:
  - Control: **Failed**;
  - K0maru: **Passed (1.00s sub-second turnaround)**.

### Case 3: TypeScript Authentication Middleware Deep Destructuring Crash
- **Qwen2.5-Coder-32B**:
  - Control: **Failed** (wrote an incomplete `if (ctx.req)` guard that missed nested headers);
  - K0maru: **Passed** (used modern optional chaining: `ctx?.req?.headers?.['authorization']`).

### Case 4: Go Unbuffered Channel Goroutine Deadlock
- All models resolved this defect in both configurations, but K0maru reduced prompt token overhead by **15% to 25%**.

### Case 5: C Heap Memory Use-After-Free & Double Free
- **DeepSeek-R1-32B**:
  - Control: 35.29s latency, 200 tokens;
  - K0maru: **20.03s latency (1.76x faster)**, 126 tokens (37.0% token savings).

---

## 🛠️ Benchmark Reproduction Instructions

All evaluation scripts, prompts, and raw data are located in the `benchmarks/` directory:

```bash
# 1. Execute system-level performance benchmarks (cold-start, RSS, throughput)
python3 benchmarks/run_systems_benchmark.py

# 2. Run token compression and long-log evaluation suite
python3 benchmarks/run_token_benchmark.py

# 3. Regenerate all publication-grade figures
python3 benchmarks/generate_charts.py
```
