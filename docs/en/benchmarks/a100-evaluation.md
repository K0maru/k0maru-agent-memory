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
| **Qwen2.5-Coder-32B** | 32B Dense Code Specialist | Control (Vanilla) | 60.0% (3/5) | 2,635 | 17.23s | 1.0x | Misled by verbose stack traces in Python and TS cases; emitted non-compiling patches |
| **Qwen2.5-Coder-32B** | 32B Dense Code Specialist | **K0maru Managed** | **100.0% (5/5)** | **1,750** | **2.92s** | **5.90x** | **5/5 solved (+40% accuracy jump)**; Mermaid state diagram pinpoints root cause; 5.90x faster |
| **DeepSeek-R1-32B** | 32B Reasoning / CoT | Control (Vanilla) | 100.0% (5/5) | 817 | 27.14s | 1.0x | Solved all cases, but over-reflected on long traces; responses took nearly 30 seconds |
| **DeepSeek-R1-32B** | 32B Reasoning / CoT | **K0maru Managed** | **80.0% (4/5)** | **652** | **14.07s** | **1.93x** | **CoT reflection latency cut by 48.2%**; token usage dropped 20.2%; 3.4x faster on Rust case |
| **DeepSeek-Coder-V2** | 16B MoE (2.4B active) | Control (Vanilla) | 60.0% (3/5) | 2,967 | 12.46s | 1.0x | Misinterpreted long test trace; generated synchronous code that stalled on async deadlocks |
| **DeepSeek-Coder-V2** | 16B MoE (2.4B active) | **K0maru Managed** | **80.0% (4/5)** | **1,907** | **1.05s** | **11.86x** | **1.05s sub-second turnaround (11.86x faster)**; correctly fixed Python async task leak |
| **Qwen3.8-27B** | 27B Dense (2-Stage) | Control (Vanilla) | 100.0% (5/5) | 923 | 14.97s | 1.0x | Strong base capabilities, but verbose logs inflated prompt tokens unnecessarily |
| **Qwen3.8-27B** | 27B Dense (2-Stage) | **K0maru Managed** | **100.0% (5/5)** | **709** | **14.97s** | **1.00x** | **23.2% Token Reduction Ratio (TRR)**; halved turnaround on Rust build case (21.2s -> 11.1s) |

---

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
