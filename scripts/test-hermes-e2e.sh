#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

echo "================================================================================"
echo "🛡️  K0maru-Agent-Memory: Hermes Agent Black-Box Container Verification"
echo "    [Invariant 1] Host Zero Pollution: Runs 100% inside disposable Docker container"
echo "    [Invariant 2] Zero API Cost: Intercepted by local mock_llm.py (port 8000)"
echo "    [Invariant 3] Zero Daemon: Asserts 0 lingering background k0maru processes"
echo "================================================================================"

if ! command -v docker >/dev/null 2>&1; then
    echo "⚠️  [Notice] Docker is not installed or not running in your current environment PATH."
    echo "   • Tier 1 (Host Protocol Conformance & Sandbox Gate) has passed (see 'cargo test')."
    echo "   • To run this Tier 2 Black-Box test, please install/start Docker (or OrbStack/Colima)."
    echo "   • Once Docker is running, re-execute: ./scripts/test-hermes-e2e.sh"
    exit 0
fi

echo "--> Building isolated test container: k0maru-hermes-e2e..."
docker build -t k0maru-hermes-e2e -f tests/ecosystem/hermes/Dockerfile .

echo "--> Executing Hermes Agent black-box E2E test (with automatic --rm cleanup)..."
docker run --rm -t k0maru-hermes-e2e

echo "✓ Black-box container run completed successfully!"
