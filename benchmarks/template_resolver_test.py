#!/usr/bin/env python3
"""
Template for Colab Project Resolver & Scoped FTS Test.
Verifies Phase 9.1: First-Class Project Catalog & Multi-Tier Scoped Resolver on Google Colab.
"""

import os
import sys
import time
import json
import base64
import tarfile
import io
import shutil
import subprocess
from pathlib import Path

# ==========================================
# 0. Zero-Burn Guard for Colab Instance
# ==========================================
def force_release_colab_instance():
    print("\n[GUARD] Checking runtime teardown to prevent CCU drain...")
    try:
        from google.colab import runtime
        runtime.unassign()
        print("[GUARD] runtime.unassign() successfully requested.")
    except ImportError:
        print("[GUARD] Running outside Google Colab environment. Clean exit.")
    except Exception as err:
        print(f"[GUARD ERROR] runtime.unassign() error: {err}", file=sys.stderr)

# ==========================================
# 1. Environment & Hardware Probing
# ==========================================
def probe_hardware():
    print("=" * 70)
    print("  COLAB VERIFICATION: FIRST-CLASS PROJECT RESOLVER & SCOPED FTS")
    print("=" * 70)
    try:
        import platform
        print(f"[SYSTEM] OS: {platform.system()} {platform.release()}, Arch: {platform.machine()}")
        print(f"[SYSTEM] Python: {platform.python_version()}")
        try:
            import torch
            if torch.cuda.is_available():
                print(f"[HARDWARE] GPU: {torch.cuda.get_device_name(0)}")
            else:
                print("[HARDWARE] Pure CPU environment")
        except ImportError:
            print("[HARDWARE] CPU environment (Torch not loaded)")
    except Exception as e:
        print(f"[SYSTEM PROBE ERROR] {e}")

# ==========================================
# 2. Source Extraction & Cargo Compilation
# ==========================================
EMBEDDED_K0MARU_TAR_B64 = """%%EMBEDDED_B64%%"""

def install_and_build_k0maru():
    print("\n[BUILD] Unpacking embedded k0maru source code...")
    src_dir = Path("/tmp/k0maru_src")
    if src_dir.exists():
        shutil.rmtree(src_dir)
    src_dir.mkdir(parents=True, exist_ok=True)
    
    tar_bytes = base64.b64decode(EMBEDDED_K0MARU_TAR_B64.strip())
    with tarfile.open(fileobj=io.BytesIO(tar_bytes), mode="r:gz") as tar:
        tar.extractall(path=src_dir)
    
    print("[BUILD] Checking Rust toolchain...")
    cargo_bin = shutil.which("cargo")
    if not cargo_bin:
        rustup_cargo = Path(os.path.expanduser("~/.cargo/bin/cargo"))
        if rustup_cargo.exists():
            cargo_bin = str(rustup_cargo)
            os.environ["PATH"] = f"{os.path.expanduser('~/.cargo/bin')}:{os.environ.get('PATH', '')}"
        else:
            print("[BUILD] Installing Rust toolchain via rustup...")
            cmd = "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable --profile minimal"
            subprocess.run(cmd, shell=True, check=True)
            cargo_bin = os.path.expanduser("~/.cargo/bin/cargo")
            os.environ["PATH"] = f"{os.path.expanduser('~/.cargo/bin')}:{os.environ.get('PATH', '')}"
    
    # 2.1 Compile release binary first
    print(f"\n[BUILD] Compiling k0maru release binary...")
    t0 = time.time()
    compile_cmd = [cargo_bin, "build", "--release", "--bin", "k0maru"]
    subprocess.run(compile_cmd, cwd=src_dir, check=True)
    compile_duration = time.time() - t0
    print(f"[BUILD] k0maru release binary compiled in {compile_duration:.2f}s!")
    
    k0maru_bin = src_dir / "target" / "release" / "k0maru"
    dest_bin = Path("/usr/local/bin/k0maru")
    try:
        shutil.copy(k0maru_bin, dest_bin)
        dest_bin.chmod(0o755)
        installed_bin = str(dest_bin)
    except Exception:
        installed_bin = str(k0maru_bin)
    
    ver = subprocess.run([installed_bin, "--version"], capture_output=True, text=True).stdout.strip()
    print(f"[BUILD] Installed k0maru: {ver} at {installed_bin}")

    # 2.2 Run unit and integration tests natively on Linux (sharing release cache)
    print(f"\n[TEST SUITE] Running native cargo tests on Linux using {cargo_bin}...")
    t_test = time.time()
    test_cmd = [cargo_bin, "test", "--release", "--test", "test_project_resolver_tiers"]
    proc_test = subprocess.run(test_cmd, cwd=src_dir, capture_output=True, text=True)
    if proc_test.returncode != 0:
        print("[TEST SUITE FAILED]:", proc_test.stderr)
        print(proc_test.stdout)
        raise RuntimeError("Integration tests failed on Linux!")
    print(f"[TEST SUITE PASS] All test_project_resolver_tiers passed in {time.time() - t_test:.2f}s!")
    for line in proc_test.stdout.splitlines():
        if "test test_" in line:
            print(f"  {line}")

    return installed_bin

# ==========================================
# 3. Setup Adversarial Vault (Recreating Real-World Issue)
# ==========================================
def setup_adversarial_vault(vault_path: Path):
    if vault_path.exists():
        shutil.rmtree(vault_path)
    vault_path.mkdir(parents=True, exist_ok=True)

    # 10_Projects
    p_dir = vault_path / "10_Projects"
    p_dir.mkdir(parents=True, exist_ok=True)
    
    # 1. Target Chinese Project with English Slug and Aliases
    (p_dir / "分布式网关限流与幂等结算系统.md").write_text("""---
title: 分布式网关限流与幂等结算系统
date: 2026-09-04
status: active
slug: distributed-gateway-settlement
aliases:
  - gateway-settlement
  - distributed-gateway
tags:
  - type/project
  - topic/distributed-systems
  - topic/gateway-settlement
  - topic/rate-limiting
  - topic/idempotency
  - topic/gateway-api
---

# 分布式网关限流与幂等结算系统

> 依托现代分布式一致性原理与高吞吐异步事件驱动架构。

## 🧭 核心目标与工程铁律 (Core Objectives & Iron Laws)
1. 彻底消除重复结算与幂等穿透风险 (Zero Idempotency Leaks)。
2. 网关边界前置分布式租约令牌校验。
3. 滑动窗口限流门禁 (Sliding Window Gate): Token Count <= Limit。
4. 租约锁定期铁律: 至少锁定结算幂等键 180 秒避免重放。

架构设计见 [[20_Cards/分布式租约令牌与自适应限流架构演进]] 与 [[20_Cards/滑动窗口令牌桶与令牌过期数学模型]]。
""", encoding="utf-8")

    # 2. Second Project to verify separation
    (p_dir / "payment-settlement-engine.md").write_text("""---
title: Payment Settlement Engine
slug: payment-settlement-engine
status: active
tags: [project, payment]
---
# Payment Settlement Engine
Distributed locking and idempotency invariants.
""", encoding="utf-8")

    # 20_Cards
    c_dir = vault_path / "20_Cards"
    c_dir.mkdir(parents=True, exist_ok=True)
    (c_dir / "分布式租约令牌与自适应限流架构演进.md").write_text("""---
title: 分布式租约令牌与自适应限流架构演进
category: evergreen
tags: [distributed, rate-limiting, architecture]
---
# 分布式租约令牌与自适应限流架构演进
## 核心概念
> 符号化确定性分布式租约拦截层 + 自适应滑动窗口令牌桶算法。
""", encoding="utf-8")

    (c_dir / "滑动窗口令牌桶与令牌过期数学模型.md").write_text("""---
title: 滑动窗口令牌桶与令牌过期数学模型
category: evergreen
tags: [distributed, math-model]
---
# 滑动窗口令牌桶与令牌过期数学模型
## 核心概念
> 基于滑动时间窗口建模租约令牌消耗速率与过期半衰期。
""", encoding="utf-8")
    # 01_AI_Logs / 00_Logs: 30 High-Frequency Cluttered Logs
    # In the old code, these 30 logs with "distributed-gateway-settlement" completely monopolized
    # SQLite FTS5 BM25 top 5, causing project resolution to fail!
    l_dir = vault_path / "01_AI_Logs"
    l_dir.mkdir(parents=True, exist_ok=True)
    for i in range(1, 31):
        (l_dir / f"2026-10-{i:02d}-distributed-gateway-settlement-phase-5-live-tui-monitor.md").write_text(f"""---
title: 2026-10-{i:02d}-distributed-gateway-settlement-phase-5-live-tui-monitor
category: log
hierarchy: L2Log
tags: [gateway, settlement, live-monitor]
---
# 2026-10-{i:02d}-distributed-gateway-settlement-phase-5-live-tui-monitor

Session log #{i}: distributed gateway settlement live execution.
Checking [[10_Projects/分布式网关限流与幂等结算系统]] invariants.
Detailed execution traces and telemetry dumps:
distributed gateway settlement distributed gateway settlement distributed gateway settlement.
""", encoding="utf-8")

    # .k0maru/rules.md
    k_dir = vault_path / ".k0maru"
    k_dir.mkdir(parents=True, exist_ok=True)
    (k_dir / "rules.md").write_text("""decisions: 20_Cards
logs: 01_AI_Logs
naming_style: slug_only
""", encoding="utf-8")

    print(f"[VAULT] Adversarial vault successfully scaffolded at {vault_path} (30 cluttered logs + Chinese project)")

# ==========================================
# 4. Live Verification Suite
# ==========================================
def run_resolver_live_tests(k0maru_bin: str) -> bool:
    vault_path = Path("/tmp/colab_adversarial_vault")
    setup_adversarial_vault(vault_path)
    
    # 4.1 Sync Cache
    print("\n[SYNC] Indexing adversarial vault...")
    t0_sync = time.time()
    subprocess.run([k0maru_bin, "sync", "--vault", str(vault_path)], check=True)
    print(f"[SYNC] Indexing completed in {(time.time() - t0_sync)*1000:.2f} ms")

    all_passed = True

    # 4.2 Test 1: Query by Kebab-case Slug (distributed-gateway-settlement) under 30-log clutter
    print("\n[TEST 1: SLUG RESOLUTION]")
    t0 = time.time()
    proc1 = subprocess.run([k0maru_bin, "loadout", "--vault", str(vault_path), "distributed-gateway-settlement"], capture_output=True, text=True)
    lat1 = (time.time() - t0) * 1000.0
    pass1 = (proc1.returncode == 0 and "分布式网关限流与幂等结算系统" in proc1.stdout and "分布式租约令牌" in proc1.stdout)
    print(f"  -> Result: {'PASS ✓' if pass1 else 'FAIL ✗'} (Latency: {lat1:.2f}ms)")
    if not pass1:
        print(f"     Output: {proc1.stdout}\n     Error: {proc1.stderr}")
        all_passed = False

    # 4.3 Test 2: Query by English Alias (gateway-settlement)
    print("\n[TEST 2: ALIAS RESOLUTION]")
    t0 = time.time()
    proc2 = subprocess.run([k0maru_bin, "loadout", "--vault", str(vault_path), "gateway-settlement"], capture_output=True, text=True)
    lat2 = (time.time() - t0) * 1000.0
    pass2 = (proc2.returncode == 0 and "分布式网关限流与幂等结算系统" in proc2.stdout)
    print(f"  -> Result: {'PASS ✓' if pass2 else 'FAIL ✗'} (Latency: {lat2:.2f}ms)")
    if not pass2:
        print(f"     Output: {proc2.stdout}\n     Error: {proc2.stderr}")
        all_passed = False

    # 4.4 Test 3: Query by Chinese Substring (幂等结算)
    print("\n[TEST 3: CHINESE SUBSTRING RESOLUTION]")
    t0 = time.time()
    proc3 = subprocess.run([k0maru_bin, "loadout", "--vault", str(vault_path), "幂等结算"], capture_output=True, text=True)
    lat3 = (time.time() - t0) * 1000.0
    pass3 = (proc3.returncode == 0 and "分布式网关限流与幂等结算系统" in proc3.stdout)
    print(f"  -> Result: {'PASS ✓' if pass3 else 'FAIL ✗'} (Latency: {lat3:.2f}ms)")
    if not pass3:
        print(f"     Output: {proc3.stdout}\n     Error: {proc3.stderr}")
        all_passed = False

    # 4.5 Test 4: Query by Prefix (分布式)
    print("\n[TEST 4: PREFIX RESOLUTION]")
    t0 = time.time()
    proc4 = subprocess.run([k0maru_bin, "loadout", "--vault", str(vault_path), "分布式"], capture_output=True, text=True)
    lat4 = (time.time() - t0) * 1000.0
    pass4 = (proc4.returncode == 0 and "分布式网关限流与幂等结算系统" in proc4.stdout)
    print(f"  -> Result: {'PASS ✓' if pass4 else 'FAIL ✗'} (Latency: {lat4:.2f}ms)")
    if not pass4:
        print(f"     Output: {proc4.stdout}\n     Error: {proc4.stderr}")
        all_passed = False
    # 4.6 Test 5: MCP JSON-RPC Stdio Tool Call
    print("\n[TEST 5: MCP STDIO get_project_loadout]")
    t0 = time.time()
    mcp_call = json.dumps({
        "jsonrpc": "2.0",
        "id": 42,
        "method": "tools/call",
        "params": {
            "name": "get_project_loadout",
            "arguments": {"project_name": "distributed-gateway-settlement"}
        }
    })
    proc5 = subprocess.run([k0maru_bin, "mcp", "--vault", str(vault_path)], input=mcp_call, capture_output=True, text=True)
    lat5 = (time.time() - t0) * 1000.0
    try:
        lines = [l for l in proc5.stdout.strip().splitlines() if l.strip()]
        last_line = lines[-1]
        resp = json.loads(last_line)
        content_text = resp.get("result", {}).get("content", [{}])[0].get("text", "")
        is_err = resp.get("result", {}).get("isError", False)
        pass5 = ("分布式网关限流与幂等结算系统" in content_text) and not is_err and ("分布式租约令牌" in content_text)
    except Exception as e:
        print(f"     MCP Parse Error: {e}")
        pass5 = False
    print(f"  -> Result: {'PASS ✓' if pass5 else 'FAIL ✗'} (Latency: {lat5:.2f}ms)")
    if not pass5:
        print(f"     Raw Output: {proc5.stdout}\n     Error: {proc5.stderr}")
        all_passed = False

    # 4.7 Test 6: Verify Token Budget Invariant (<300 tokens)
    print("\n[TEST 6: LOADOUT TOKEN BUDGET INVARIANT]")
    loadout_text = proc1.stdout
    approx_tokens = len(loadout_text) // 4
    pass6 = (approx_tokens < 300)
    print(f"  -> Loadout Length: {len(loadout_text)} chars, Approx: {approx_tokens} tokens")
    print(f"  -> Result: {'PASS ✓' if pass6 else 'FAIL ✗'} (Target: <300 tokens)")
    if not pass6:
        all_passed = False

    print("\n" + "=" * 70)
    if all_passed:
        print("  🎉 ALL ADVERSARIAL PROJECT RESOLUTION TESTS PASSED ON COLAB!")
        print("     Zero log-clutter truncation, 100% slug & alias accuracy.")
    else:
        print("  ❌ ONE OR MORE TESTS FAILED ON COLAB!")
    print("=" * 70)
    return all_passed

def main():
    probe_hardware()
    success = False
    try:
        k0maru_bin = install_and_build_k0maru()
        success = run_resolver_live_tests(k0maru_bin)
    except Exception as e:
        import traceback
        print(f"[FATAL ERROR] {e}", file=sys.stderr)
        traceback.print_exc()
    finally:
        time.sleep(1)
        force_release_colab_instance()
    
    sys.exit(0 if success else 1)

if __name__ == "__main__":
    main()
