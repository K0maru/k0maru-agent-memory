# Ticket 32-4: Zero-API Hermes Black-Box Docker E2E Test Suite

## Context & Goal
Provide a non-polluting, zero-API-cost black-box verification suite for Nous Research Hermes Agent. Runs inside an ephemeral Docker container (`docker run --rm`) to ensure host safety. A lightweight Python standard library Mock HTTP Server (`mock_llm.py`) intercepts `/v1/chat/completions` and emits `tool_calls` for `k0maru-memory`, triggering genuine Hermes Agent tool invocations.

## Blocked by
32-3

## Scope of Work
1. **Mock OpenAI Server (`tests/ecosystem/hermes/mock_llm.py`)**:
   - Written with Python standard library (`http.server`, `urllib`, `json`);
   - Endpoints:
     - `POST /v1/chat/completions`
     - `GET /v1/models`
   - State machine:
     - Turn 1: Return OpenAI format response with `tool_calls` targeting `flush_session` (or `remember`);
     - Turn 2: Return final message acknowledging tool output;
2. **Container Definition (`tests/ecosystem/hermes/Dockerfile`)**:
   - Base image: `python:3.11-slim`;
   - Install `hermes-agent` (or `pip install hermes-agent`);
   - Multi-stage build or copy pre-built `k0maru` binary into `/usr/local/bin/k0maru`;
   - Copy `mock_llm.py`, `run_e2e.sh`;
3. **E2E Execution Script (`tests/ecosystem/hermes/run_e2e.sh`)**:
   - Spin up `mock_llm.py` on `127.0.0.1:8000`;
   - Run `k0maru install --target hermes --vault /workspace/vault`;
   - Run Hermes Agent chat execution targeting mock server;
   - Assert note written in `/workspace/vault/`;
   - Check process tree to assert zero lingering `k0maru` daemon processes;
4. **Host Driver Script (`./scripts/test-hermes-e2e.sh`)**:
   - Checks if `docker` is available;
   - If not available, prints diagnostic guidance;
   - If available, runs `docker build` and `docker run --rm`.

## Acceptance Criteria
- [x] `mock_llm.py` provides OpenAI compatible `tool_calls` response without external dependencies;
- [x] Docker container is self-contained and leaves zero footprint on host;
- [x] Hermes Agent executes tool calling against `k0maru` and writes to vault;
- [x] Process table confirms Zero-Daemon after Hermes exits;
- [x] Host driver script `./scripts/test-hermes-e2e.sh` is executable and documented.
