# syntax=docker/dockerfile:1
# ------------------------------------------------------------------------------
# Stage 1: Build k0maru binary in Linux container
# ------------------------------------------------------------------------------
FROM rust:1.80-slim AS builder

WORKDIR /build
# Install build dependencies for sqlite and compilation
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config libssl-dev gcc libc6-dev \
    && rm -rf /var/lib/apt/lists/*

COPY . .
RUN cargo build --release --bin k0maru

# ------------------------------------------------------------------------------
# Stage 2: Isolated Hermes Agent Black-Box Runtime
# ------------------------------------------------------------------------------
FROM python:3.11-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    curl \
    procps \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Attempt installation of hermes-agent inside container (zero pollution to host)
RUN pip install --no-cache-dir hermes-agent || true

# Inject compiled k0maru binary
COPY --from=builder /build/target/release/k0maru /usr/local/bin/k0maru
RUN chmod +x /usr/local/bin/k0maru

# Setup workspace and test scripts
WORKDIR /tests/ecosystem/hermes
COPY tests/ecosystem/hermes/ .
RUN chmod +x run_e2e.sh mock_llm.py

ENTRYPOINT ["/tests/ecosystem/hermes/run_e2e.sh"]
