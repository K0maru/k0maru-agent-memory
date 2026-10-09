#!/usr/bin/env python3
"""
Generate realistic benchmark log fixtures for k0maru token reduction evaluation:
1. cargo_build_error.log (~500 lines)
2. pytest_failures.log (~800 lines)
3. jest_test_failures.log (~1200 lines)
4. multithread_crash.log (~2500 lines)
"""

import os
import random

FIXTURES_DIR = os.path.join(os.path.dirname(__file__), "fixtures")
os.makedirs(FIXTURES_DIR, exist_ok=True)

def generate_cargo_build_log(target_lines=500):
    lines = [
        "    Updating crates.io index",
        " Downloading crates ...",
        "  Downloaded tokio v1.38.0",
        "  Downloaded serde v1.0.203",
        "  Downloaded hyper v1.3.1",
        "  Downloaded rusqlite v0.31.0",
        "  Downloaded tracing v0.1.40",
        "   Compiling libc v0.2.155",
        "   Compiling cfg-if v1.0.0",
        "   Compiling proc-macro2 v1.0.86",
        "   Compiling unicode-ident v1.0.12",
        "   Compiling quote v1.0.36",
        "   Compiling syn v2.0.66",
        "   Compiling version_check v0.9.4",
        "   Compiling once_cell v1.19.0",
        "   Compiling memchr v2.7.4",
        "   Compiling parking_lot_core v0.9.10",
        "   Compiling lock_api v0.4.12",
        "   Compiling smallvec v1.13.2",
        "   Compiling scopeguard v1.2.0",
        "   Compiling signal-hook-registry v1.4.2",
        "   Compiling mio v0.8.11",
        "   Compiling socket2 v0.5.7",
        "   Compiling bytes v1.6.0",
        "   Compiling pin-project-lite v0.2.14",
        "   Compiling log v0.4.21",
        "   Compiling tracing-core v0.1.32",
        "   Compiling slab v0.4.9",
        "   Compiling futures-core v0.3.30",
        "   Compiling futures-sink v0.3.30",
        "   Compiling futures-channel v0.3.30",
        "   Compiling futures-task v0.3.30",
        "   Compiling futures-util v0.3.30",
        "   Compiling tokio-macros v2.3.0",
        "   Compiling serde_derive v1.0.203",
        "   Compiling serde v1.0.203",
        "   Compiling tokio v1.38.0",
        "   Compiling itoa v1.0.11",
        "   Compiling fnv v1.0.7",
        "   Compiling http v1.1.0",
        "   Compiling httparse v1.9.4",
        "   Compiling indexmap v2.2.6",
        "   Compiling bitflags v2.5.0",
        "   Compiling tracing v0.1.40",
        "   Compiling k0maru-cluster v0.1.0 (/workspace/k0maru-cluster)",
    ]

    error_templates = [
        (
            "warning: unused import: `std::sync::atomic::AtomicBool`",
            "  --> src/consensus/raft.rs:14:24\n"
            "   |\n"
            "14 | use std::sync::atomic::{AtomicBool, AtomicU64};\n"
            "   |                         ^^^^^^^^^^\n"
            "   |\n"
            "   = note: `#[warn(unused_imports)]` on by default"
        ),
        (
            "warning: variable `stale_term` is assigned to but never used",
            "  --> src/consensus/state.rs:88:17\n"
            "   |\n"
            "88 |         let mut stale_term = current_term - 1;\n"
            "   |                 ^^^^^^^^^^\n"
            "   |\n"
            "   = note: `#[warn(unused_variables)]` on by default\n"
            "   = help: maybe prefix with an underscore: `_stale_term`"
        ),
        (
            "error[E0308]: mismatched types",
            "   --> src/storage/wal.rs:142:31\n"
            "    |\n"
            "140 |     pub async fn append_entry(&mut self, entry: LogEntry) -> Result<u64, WalError> {\n"
            "141 |         let serialized = bincode::serialize(&entry)?;\n"
            "142 |         self.writer.write_all(serialized).await?;\n"
            "    |                     --------- ^^^^^^^^^^ expected `&[u8]`, found `Vec<u8>`\n"
            "    |                     |\n"
            "    |                     arguments to this method are incorrect\n"
            "    |\n"
            "help: consider borrowing here\n"
            "    |\n"
            "142 |         self.writer.write_all(&serialized).await?;\n"
            "    |                               +"
        ),
        (
            "error[E0502]: cannot borrow `*self` as mutable because it is also borrowed as immutable",
            "  --> src/network/peer.rs:95:9\n"
            "   |\n"
            "91 |         let peer_info = self.get_peer_info(peer_id)?;\n"
            "   |                         ---------------------------- immutable borrow occurs here\n"
            "...\n"
            "95 |         self.reconnect_peer(peer_id).await?;\n"
            "   |         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^ mutable borrow occurs here\n"
            "96 |         info!(\"Reconnected peer: {}\", peer_info.address);\n"
            "   |                                       ----------------- immutable borrow later used here"
        ),
        (
            "error[E0382]: use of moved value: `tx_payload`",
            "   --> src/consensus/executor.rs:215:28\n"
            "    |\n"
            "209 |     let tx_payload = envelope.into_payload();\n"
            "    |         ---------- move occurs because `tx_payload` has type `TransactionPayload`, which does not implement the `Copy` trait\n"
            "210 |     self.verify_signature(&tx_payload)?;\n"
            "211 |     self.ledger.apply_transaction(tx_payload)?;\n"
            "    |                                   ---------- `tx_payload` moved here\n"
            "...\n"
            "215 |     self.metrics.record_tx(tx_payload.tx_id, tx_payload.size_bytes());\n"
            "    |                            ^^^^^^^^^^^^^^^^ value used here after move\n"
            "    |\n"
            "help: consider cloning the value if the performance cost is acceptable\n"
            "    |\n"
            "211 |     self.ledger.apply_transaction(tx_payload.clone())?;\n"
            "    |                                             ++++++++"
        ),
        (
            "error[E0277]: `*const u8` cannot be shared between threads safely",
            "   --> src/server/dispatcher.rs:64:17\n"
            "    |\n"
            "64  |     tokio::spawn(async move {\n"
            "    |     ^^^^^^^^^^^^ `*const u8` cannot be shared between threads safely\n"
            "    |\n"
            "    = help: within `RequestContext`, the trait `Sync` is not implemented for `*const u8`\n"
            "    = note: required because it appears within the type `RequestContext`\n"
            "    = note: required for `tokio::task::JoinHandle<()>` to implement `Send`\n"
            "note: required by a bound in `tokio::spawn`\n"
            "   --> /home/developer/.cargo/registry/src/index.crates.io-6f17d22bba15001f/tokio-1.38.0/src/task/spawn.rs:166:21\n"
            "    |\n"
            "166 |         T: Future + Send + 'static,\n"
            "    |                     ^^^^ required by this bound in `spawn`"
        ),
        (
            "error[E0599]: no method named `into_segmented_buffer` found for struct `RingQueue<T>` in the current scope",
            "   --> src/buffer/pool.rs:182:35\n"
            "    |\n"
            "182 |         let segmented = self.queue.into_segmented_buffer();\n"
            "    |                                    ^^^^^^^^^^^^^^^^^^^^^^ method not found in `RingQueue<T>`\n"
            "    |\n"
            "    = help: items from traits can only be used if the trait is in scope\n"
            "help: trait `BufferSegmentation` which provides `into_segmented_buffer` is implemented but not in scope; consider adding an import\n"
            "    |\n"
            "4   + use crate::buffer::traits::BufferSegmentation;\n"
            "    |"
        ),
    ]

    idx = 0
    while len(lines) < target_lines - 10:
        header, body = error_templates[idx % len(error_templates)]
        lines.append(header)
        lines.extend(body.split("\n"))
        lines.append("")
        idx += 1

    lines.append(f"error: could not compile `k0maru-cluster` (lib) due to {idx} previous errors; 8 warnings emitted")
    lines.append("warning: `k0maru-cluster` (lib) generated 8 warnings (run `cargo fix --lib -p k0maru-cluster` to apply 4 suggestions)")
    
    # Adjust to exactly target_lines
    if len(lines) > target_lines:
        lines = lines[:target_lines]
    while len(lines) < target_lines:
        lines.append(f"   Compiling k0maru-cluster-submodule-shard-{len(lines)} v0.1.0")

    return "\n".join(lines) + "\n"


def generate_pytest_log(target_lines=800):
    lines = [
        "============================= test session starts ==============================",
        "platform darwin -- Python 3.11.8, pytest-8.1.1, pluggy-1.4.0",
        "rootdir: /workspace/k0maru-agent-memory",
        "configfile: pyproject.toml",
        "testpaths: tests",
        "plugins: anyio-4.3.0, asyncio-0.23.6, cov-5.0.0, mock-3.14.0, xdist-3.5.0",
        "asyncio: mode=Mode.STRICT",
        "collected 164 items",
        "",
        "tests/unit/test_adapters.py .....................                       [ 12%]",
        "tests/unit/test_markdown_parser.py ................................     [ 32%]",
        "tests/unit/test_sqlite_storage.py .............F........                [ 45%]",
        "tests/unit/test_loadout_builder.py ..........F.F....                   [ 55%]",
        "tests/unit/test_offload_engine.py .....................                 [ 68%]",
        "tests/integration/test_mcp_server.py ........F...F....                  [ 80%]",
        "tests/integration/test_vault_sync.py ...E.F....F...                     [ 90%]",
        "tests/integration/test_e2e_flow.py .F...F......                        [100%]",
        "",
        "=================================== FAILURES ===================================",
    ]

    failure_templates = [
        (
            "test_sqlite_wal_transaction_rollback",
            "tests/unit/test_sqlite_storage.py:184: in test_sqlite_wal_transaction_rollback\n"
            "    storage.execute_batch(corrupted_tx_payload)\n"
            "src/storage/sqlite.py:92: in execute_batch\n"
            "    cursor.executescript(script)\n"
            "E   sqlite3.OperationalError: database table is locked\n"
            "----------------------------- Captured stdout call -----------------------------\n"
            "[DEBUG] Acquiring write lock for transaction tx_0092182\n"
            "[WARN] Lock acquisition waited 5000ms before timeout\n"
            "----------------------------- Captured stderr call -----------------------------\n"
            "Traceback (most recent call last):\n"
            "  File \"/workspace/k0maru-agent-memory/src/storage/sqlite.py\", line 88, in execute_batch\n"
            "    with self.conn.transaction():\n"
            "LockTimeout: Timeout acquiring database lock on /tmp/k0maru_test_vault/.k0maru/cache.sqlite"
        ),
        (
            "test_loadout_builder_token_budget_exceeded",
            "tests/unit/test_loadout_builder.py:64: in test_loadout_builder_token_budget_exceeded\n"
            "    assert loadout.token_count <= 300, f\"Token count {loadout.token_count} exceeds limit\"\n"
            "E   AssertionError: Token count 482 exceeds limit\n"
            "E   assert 482 <= 300\n"
            "E    +  where 482 = LoadoutResult(name='AtlasCluster', tokens=482).token_count\n"
            "----------------------------- Captured stdout call -----------------------------\n"
            "Loadout generated:\n"
            "# Project: AtlasCluster\n"
            "Status: active\n"
            "Cards included: 14 cards (CardA, CardB, CardC, CardD, CardE, CardF)\n"
            "Summary length: 1824 characters"
        ),
        (
            "test_mcp_inspect_node_missing_reference",
            "tests/integration/test_mcp_server.py:210: in test_mcp_inspect_node_missing_reference\n"
            "    res = client.call_tool(\"inspect_log_node\", {\"node_id\": \"node_invalid_9999\"})\n"
            "src/mcp/client.py:45: in call_tool\n"
            "    raise ToolExecutionError(res[\"error\"][\"message\"])\n"
            "E   src.mcp.exceptions.ToolExecutionError: Log node 'node_invalid_9999' not found in .scratch/refs\n"
            "----------------------------- Captured log call --------------------------------\n"
            "2026-09-30 14:10:02 [INFO] FastMCP stdio server listening\n"
            "2026-09-30 14:10:02 [ERROR] MCP tool inspect_log_node failed: node_invalid_9999 not found"
        ),
        (
            "test_vault_sync_concurrent_file_modification",
            "tests/integration/test_vault_sync.py:315: in test_vault_sync_concurrent_file_modification\n"
            "    assert synced_count == 25, f\"Expected 25 synced cards, got {synced_count}\"\n"
            "E   AssertionError: Expected 25 synced cards, got 19\n"
            "E   assert 19 == 25\n"
            "----------------------------- Captured stderr call -----------------------------\n"
            "FileNotFoundError: [Errno 2] No such file or directory: '/tmp/test_vault/20_Cards/ActiveTask.md.tmp.1284'\n"
            "Worker thread 3 encountered race condition during atomic rename."
        ),
        (
            "test_markdown_parser_wikilink_aliases",
            "tests/unit/test_markdown_parser.py:112: in test_markdown_parser_wikilink_aliases\n"
            "    assert parsed_links == expected_links\n"
            "E   AssertionError: assert [{'target': 'CardA', 'alias': 'Alpha'}] == [{'target': 'CardA', 'alias': 'Alpha'}, {'target': 'CardB', 'alias': None}]\n"
            "E     Right contains one more item: {'target': 'CardB', 'alias': None}\n"
            "E     Full diff:\n"
            "E       [\n"
            "E         {\n"
            "E           'alias': 'Alpha',\n"
            "E           'target': 'CardA',\n"
            "E         },\n"
            "E     +   {\n"
            "E     +     'alias': None,\n"
            "E     +     'target': 'CardB',\n"
            "E     +   },\n"
            "E       ]"
        ),
    ]

    idx = 0
    while len(lines) < target_lines - 40:
        name, body = failure_templates[idx % len(failure_templates)]
        header = f"___________________________ {name} [{idx}] ___________________________"
        lines.append(header)
        lines.extend(body.split("\n"))
        lines.append("")
        idx += 1

    lines.append("=========================== short test summary info ============================")
    for i in range(min(idx, 15)):
        lines.append(f"FAILED tests/test_cases.py::test_case_{i} - AssertionError: check failed at iteration {i}")
    lines.append(f"================== {idx} failed, 150 passed, 2 errors in 24.18s ===================")

    if len(lines) > target_lines:
        lines = lines[:target_lines]
    while len(lines) < target_lines:
        lines.append(f"PASSED tests/generated/test_shard_{len(lines)}.py::test_ok")

    return "\n".join(lines) + "\n"


def generate_jest_log(target_lines=1200):
    lines = [
        "$ jest --colors --maxWorkers=4 --verbose",
        "  console.log",
        "    [INFO] Starting Jest Test Runner in workspace /workspace/frontend-hub",
        "",
        "  console.warn",
        "    [WARN] React.createFactory() is deprecated and will be removed in next major release.",
        "      at node_modules/react-dom/cjs/react-dom.development.js:1248:15",
        "",
    ]

    suite_templates = [
        (
            "FAIL src/components/Dashboard/GraphViewer.test.tsx",
            "  ● GraphViewer › renders mermaid SVG diagram asynchronously\n"
            "\n"
            "    expect(received).toContain(expected)\n"
            "\n"
            "    Expected substring: \"<svg id=\\\"mermaid-diagram\\\"\"\n"
            "    Received string:    \"<div class=\\\"loading-spinner\\\">Rendering graph...</div>\"\n"
            "\n"
            "      38 |     render(<GraphViewer chartDefinition={mockMermaid} />);\n"
            "      39 |     const rendered = screen.getByTestId('mermaid-container').innerHTML;\n"
            "    > 40 |     expect(rendered).toContain('<svg id=\"mermaid-diagram\"');\n"
            "         |                      ^\n"
            "      41 |   });\n"
            "      42 |\n"
            "      at Object.<anonymous> (src/components/Dashboard/GraphViewer.test.tsx:40:22)\n"
            "      at Promise.then.completed (node_modules/jest-circus/build/utils.js:298:28)\n"
            "      at new Promise (<anonymous>)\n"
            "      at callAsyncCircusFn (node_modules/jest-circus/build/utils.js:231:10)\n"
            "      at _callCircusTest (node_modules/jest-circus/build/run.js:316:40)\n"
            "      at processTicksAndRejections (node:internal/process/task_queues:95:5)"
        ),
        (
            "FAIL src/api/k0maruClient.test.ts",
            "  ● k0maruClient › offloadLogStream › streams large payload above 50 lines\n"
            "\n"
            "    AxiosError: timeout of 5000ms exceeded\n"
            "\n"
            "      at createError (node_modules/axios/lib/core/createError.js:16:15)\n"
            "      at RedirectableRequest.handleTimeout (node_modules/axios/lib/adapters/http.js:304:16)\n"
            "      at RedirectableRequest.emit (node:events:517:28)\n"
            "      at Timeout.<anonymous> (node_modules/follow-redirects/index.js:199:12)\n"
            "      at listOnTimeout (node:internal/timers:573:17)\n"
            "      at processTimers (node:internal/timers:514:7)\n"
            "\n"
            "    Cause:\n"
            "      Error: socket hang up\n"
            "          at connResetException (node:internal/errors:720:14)\n"
            "          at Socket.socketOnEnd (node:_http_client:543:23)\n"
            "          at Socket.emit (node:events:529:35)"
        ),
        (
            "FAIL src/features/memory/CardMetadataEditor.test.tsx",
            "  ● CardMetadataEditor › updates YAML frontmatter correctly\n"
            "\n"
            "    expect(received).toEqual(expected) // deep equality\n"
            "\n"
            "    - Expected  - 2\n"
            "    + Received  + 2\n"
            "\n"
            "      Object {\n"
            "        \"archived\": false,\n"
            "    -   \"tags\": Array [\n"
            "    -     \"project/core\",\n"
            "    -     \"status/active\",\n"
            "    +   \"tags\": Array [\n"
            "    +     \"project/core\",\n"
            "        ],\n"
            "        \"updated_at\": \"2026-09-30T12:00:00Z\",\n"
            "      }\n"
            "\n"
            "      72 |     fireEvent.click(saveButton);\n"
            "      73 |     await waitFor(() => expect(onSaveMock).toHaveBeenCalled());\n"
            "    > 74 |     expect(onSaveMock).toHaveBeenCalledWith(expectedPayload);\n"
            "         |                        ^\n"
            "      at src/features/memory/CardMetadataEditor.test.tsx:74:24"
        ),
        (
            "FAIL src/state/slices/projectSlice.test.ts",
            "  ● projectSlice › reducers › handleLoadoutFetchRejected\n"
            "\n"
            "    TypeError: Cannot read properties of undefined (reading 'error')\n"
            "\n"
            "      at projectSlice.ts:89:32\n"
            "      at node_modules/@reduxjs/toolkit/src/createReducer.ts:284:18\n"
            "      at Object.<anonymous> (src/state/slices/projectSlice.test.ts:44:17)\n"
            "      at Promise.then.completed (node_modules/jest-circus/build/utils.js:298:28)"
        ),
    ]

    idx = 0
    while len(lines) < target_lines - 25:
        title, body = suite_templates[idx % len(suite_templates)]
        lines.append(f"{title} (run {idx + 1})")
        lines.extend(body.split("\n"))
        lines.append("")
        idx += 1

    lines.extend([
        "",
        "Test Suites: 18 failed, 42 passed, 60 total",
        "Tests:       45 failed, 320 passed, 365 total",
        "Snapshots:   0 total",
        "Time:        34.582 s",
        "Ran all test suites.",
        "error Command failed with exit code 1.",
    ])

    if len(lines) > target_lines:
        lines = lines[:target_lines]
    while len(lines) < target_lines:
        lines.append(f"PASS src/generated/shard_{len(lines)}.test.ts")

    return "\n".join(lines) + "\n"


def generate_multithread_crash_log(target_lines=2500):
    lines = [
        "*** CRASH REPORT: FATAL RUNTIME TERMINATION ***",
        "Timestamp: 2026-09-30 14:32:08.192841 UTC",
        "Binary: target/release/k0maru-cluster-node",
        "PID: 84920 (Thread Group Leader)",
        "Signal: SIGSEGV (Address boundary error / Segmentation fault)",
        "Fault address: 0x0000000000000028 (null pointer dereference + offset 40)",
        "",
        "Registers (Thread tokio-worker-3):",
        "  rax: 0x0000000000000000  rbx: 0x000070000c14b8a0  rcx: 0x0000000109f29100",
        "  rdx: 0x0000000000000008  rsi: 0x000070000c14b980  rdi: 0x0000000000000000",
        "  rbp: 0x000070000c14b7e0  rsp: 0x000070000c14b7c0   r8: 0x0000000000000001",
        "   r9: 0x0000000000000010  r10: 0x0000000109e4a300  r11: 0x00007fff6c3b9990",
        "  r12: 0x0000000109f00000  r13: 0x0000000000000000  r14: 0x000070000c14b980",
        "  r15: 0x00007fff8e923180  rip: 0x00000001094038a4  rflags: 0x0000000000010206",
        "",
        "============================ ACTIVE THREAD DUMPS ============================",
    ]

    thread_names = [
        "tokio-runtime-worker-0",
        "tokio-runtime-worker-1",
        "tokio-runtime-worker-2",
        "tokio-runtime-worker-3",
        "tokio-runtime-worker-4",
        "tokio-runtime-worker-5",
        "tokio-runtime-worker-6",
        "tokio-runtime-worker-7",
        "wal-flusher-thread",
        "storage-compaction-worker-0",
        "storage-compaction-worker-1",
        "network-listener-epoll",
        "heartbeat-monitor-timer",
        "garbage-collector-reaper",
        "mcp-stdio-reader",
        "fastmcp-handler-pool-0",
    ]

    frame_pool = [
        ("0x00007fff6c3b88d8", "futex_wait_cancelable", "from /lib64/libpthread.so.0"),
        ("0x00007fff6c3b9990", "__pthread_cond_wait_common", "from /lib64/libpthread.so.0"),
        ("0x0000000109520112", "tokio::runtime::park::Parker::park", "at /rustc/library/tokio/src/park.rs:88"),
        ("0x0000000109521340", "tokio::runtime::scheduler::multi_thread::worker::Context::run", "at /rustc/library/tokio/src/worker.rs:452"),
        ("0x0000000109522100", "tokio::runtime::task::core::CoreStage::poll", "at /rustc/library/tokio/src/task.rs:218"),
        ("0x0000000109488a10", "k0maru::consensus::raft::RaftNode::step", "at src/consensus/raft.rs:341"),
        ("0x00000001094892c4", "k0maru::consensus::wal::LogAppender::commit_index", "at src/consensus/wal.rs:189"),
        ("0x000000010948a990", "k0maru::storage::sqlite::SqliteStorage::execute_batch", "at src/storage/sqlite.rs:104"),
        ("0x000000010948b432", "rusqlite::Connection::prepare_cached", "at /cargo/registry/rusqlite/src/lib.rs:832"),
        ("0x000000010948c188", "sqlite3_step", "from /usr/lib/libsqlite3.dylib"),
        ("0x000000010948d390", "sqlite3_wal_checkpoint_v2", "from /usr/lib/libsqlite3.dylib"),
        ("0x0000000109490104", "k0maru::network::peer_connection::PeerSession::receive_frame", "at src/network/peer.rs:215"),
        ("0x0000000109491028", "bytes::bytes_mut::BytesMut::split_to", "at /cargo/registry/bytes/src/bytes_mut.rs:412"),
        ("0x0000000109492440", "crossbeam_channel::internal::channel::Sender::send", "at /cargo/registry/crossbeam/src/channel.rs:230"),
        ("0x00000001094931a2", "parking_lot::mutex::Mutex::lock_contended", "at /cargo/registry/parking_lot/src/mutex.rs:125"),
        ("0x0000000109494a80", "std::panicking::rust_panic_with_hook", "at library/std/src/panicking.rs:720"),
        ("0x0000000109495110", "core::option::unwrap_failed", "at library/core/src/option.rs:198"),
    ]

    t_idx = 0
    while len(lines) < target_lines - 15:
        tname = thread_names[t_idx % len(thread_names)]
        lines.append(f"--- Thread {t_idx + 1} ({tname}, LWP {50000 + t_idx}) ---")
        lines.append("  Thread State: BLOCKED on Mutex / IO Wait")
        lines.append("  Stack Trace (most recent call first):")
        
        # 10 to 18 frames per thread
        num_frames = random.randint(12, 18)
        for f_idx in range(num_frames):
            addr, func, loc = frame_pool[(t_idx * 3 + f_idx) % len(frame_pool)]
            lines.append(f"    #{f_idx:<2} {addr} in {func} () {loc}")
        lines.append("")
        t_idx += 1

    lines.extend([
        "============================ MEMORY MAP SUMMARY ============================",
        "0000000109400000-0000000109800000 r-xp 00000000 01:04 18294821  /workspace/target/release/k0maru",
        "0000000109800000-0000000109900000 r--p 00400000 01:04 18294821  /workspace/target/release/k0maru",
        "0000000109900000-0000000109950000 rw-p 00500000 01:04 18294821  /workspace/target/release/k0maru",
        "000070000c000000-000070000c200000 rw-p 00000000 00:00 0         [stack: tokio-worker-3]",
        "000070000c200000-000070000c400000 rw-p 00000000 00:00 0         [stack: tokio-worker-4]",
        "*** END OF CRASH LOG (Core dumped to /var/cores/core.k0maru.84920) ***",
    ])

    if len(lines) > target_lines:
        lines = lines[:target_lines]
    while len(lines) < target_lines:
        lines.append(f"    #99 0x00007fff00000000 in thread_start () from /usr/lib/libsystem_pthread.dylib")

    return "\n".join(lines) + "\n"


def main():
    fixtures = [
        ("cargo_build_error.log", generate_cargo_build_log(500)),
        ("pytest_failures.log", generate_pytest_log(800)),
        ("jest_test_failures.log", generate_jest_log(1200)),
        ("multithread_crash.log", generate_multithread_crash_log(2500)),
    ]

    for fname, content in fixtures:
        path = os.path.join(FIXTURES_DIR, fname)
        with open(path, "w", encoding="utf-8") as f:
            f.write(content)
        line_count = len(content.splitlines())
        print(f"Generated {fname}: {line_count} lines ({len(content)} bytes)")

if __name__ == "__main__":
    main()
