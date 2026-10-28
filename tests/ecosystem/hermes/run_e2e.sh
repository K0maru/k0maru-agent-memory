#!/usr/bin/env bash
set -euo pipefail

echo "================================================================================"
echo "🚀 [Hermes E2E] Running Zero-API, Zero-Pollution Black-Box Container Test"
echo "================================================================================"

# 1. Start Mock LLM server in background
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
python3 "$SCRIPT_DIR/mock_llm.py" &
MOCK_PID=$!

cleanup() {
    echo "--> Cleaning up background mock server (PID: $MOCK_PID)..."
    kill "$MOCK_PID" 2>/dev/null || true
}
trap cleanup EXIT

# 2. Wait for mock server healthcheck
echo "--> Waiting for Mock OpenAI server on 127.0.0.1:8000..."
for i in {1..30}; do
    if curl -s http://127.0.0.1:8000/health | grep -q "ok"; then
        echo "✓ Mock LLM server is ready on port 8000!"
        break
    fi
    sleep 0.2
done

# 3. Setup ephemeral test vault
VAULT_DIR="/workspace/vault"
rm -rf "$VAULT_DIR"
mkdir -p "$VAULT_DIR/decisions" "$VAULT_DIR/skills"

# 4. Execute k0maru install targeting Hermes
echo "--> Running: k0maru install --target hermes --vault $VAULT_DIR"
k0maru install --target hermes --vault "$VAULT_DIR"

HERMES_CFG="$HOME/.hermes/mcp.json"
if [ ! -f "$HERMES_CFG" ]; then
    echo "❌ ERROR: Hermes configuration file was not created at $HERMES_CFG"
    exit 1
fi
echo "✓ Hermes MCP config successfully created: $HERMES_CFG"
cat "$HERMES_CFG"

# 5. Simulate Hermes Agent tool execution loop
export OPENAI_API_BASE="http://127.0.0.1:8000/v1"
export OPENAI_BASE_URL="http://127.0.0.1:8000/v1"
export OPENAI_API_KEY="mock-placeholder-key"
export HERMES_MODEL="hermes-3-mock"

echo "--> Triggering Hermes Agent interaction with k0maru-memory MCP server..."

# Check if hermes-agent CLI binary exists, otherwise invoke standard hermes runner
if command -v hermes-agent >/dev/null 2>&1; then
    hermes-agent --prompt "Please document our Docker E2E testing architecture decision" || true
elif command -v hermes >/dev/null 2>&1; then
    hermes chat --prompt "Please document our Docker E2E testing architecture decision" || true
else
    # Direct hermes runner simulation via standard MCP stdio client bridge
    python3 -c "
import json, os, subprocess, urllib.request

# 1. Query Mock LLM to get tool_calls
req = urllib.request.Request(
    'http://127.0.0.1:8000/v1/chat/completions',
    data=json.dumps({
        'model': 'hermes-3-mock',
        'messages': [{'role': 'user', 'content': 'Record decision'}]
    }).encode('utf-8'),
    headers={'Content-Type': 'application/json'}
)
resp = urllib.request.urlopen(req)
result = json.loads(resp.read().decode('utf-8'))
tool_call = result['choices'][0]['message']['tool_calls'][0]
fn_name = tool_call['function']['name']
fn_args = json.loads(tool_call['function']['arguments'])

print(f'[Hermes Agent] Autonomously dispatching tool call: {fn_name}')

# 2. Dispatch to k0maru stdio MCP server as configured in ~/.hermes/mcp.json
with open(os.path.expanduser('~/.hermes/mcp.json')) as f:
    cfg = json.load(f)
server_cmd = cfg['mcpServers']['k0maru-memory']
proc = subprocess.Popen(
    [server_cmd['command']] + server_cmd['args'],
    stdin=subprocess.PIPE,
    stdout=subprocess.PIPE,
    stderr=subprocess.PIPE,
    text=True
)

init_req = json.dumps({'jsonrpc': '2.0', 'id': 1, 'method': 'initialize', 'params': {'protocolVersion': '2024-11-05', 'capabilities': {}, 'clientInfo': {'name': 'hermes-agent', 'version': '0.1.0'}}})
tool_req = json.dumps({'jsonrpc': '2.0', 'id': 2, 'method': 'tools/call', 'params': {'name': fn_name, 'arguments': fn_args}})

stdout_out, _ = proc.communicate(input=f'{init_req}\n{tool_req}\n')
lines = [l for l in stdout_out.splitlines() if l.strip()]
print(f'[Hermes Agent] MCP Response received: {lines[-1]}')
"
fi

# 6. Strong Assertion: Verify note written to vault
echo "--> Checking vault contents for crystallized decision..."
ls -la "$VAULT_DIR/decisions"
NOTE_COUNT=$(find "$VAULT_DIR/decisions" -type f -name "*Hermes*" | wc -l)
if [ "$NOTE_COUNT" -eq 0 ]; then
    echo "❌ ERROR: Expected crystallized decision note in $VAULT_DIR/decisions, but none found!"
    exit 1
fi
echo "✓ Confirmed: Note written into vault ($NOTE_COUNT file(s) found)!"

# 7. Strong Assertion: Zero-Daemon Process Tree Check
echo "--> Checking for orphaned k0maru background processes (Zero-Daemon Invariant)..."
if pgrep -x k0maru >/dev/null 2>&1; then
    echo "❌ ERROR: Detected lingering k0maru background processes! Violates Zero-Daemon rule."
    ps aux | grep k0maru
    exit 1
fi
echo "✓ Zero-Daemon Invariant passed: 0 background k0maru processes running."

echo "================================================================================"
echo "🎉 ALL TESTS PASSED: Hermes Agent E2E black-box verification successful!"
echo "================================================================================"
