#!/usr/bin/env python3
"""
Zero-Dependency, Zero-API-Cost OpenAI-compatible Mock Server.
Listens on 127.0.0.1:8000.
Simulates two-turn tool calling:
  Turn 1: Returns a tool_call targeting k0maru-memory flush_session.
  Turn 2: Acknowledges tool execution output and finishes.
"""

import json
import sys
from http.server import HTTPServer, BaseHTTPRequestHandler

PORT = 8000

class MockOpenAIHandler(BaseHTTPRequestHandler):
    def log_message(self, format, *args):
        # Silence standard HTTP logs or print to stderr
        sys.stderr.write(f"[MockLLM] {format % args}\n")
        sys.stderr.flush()

    def do_GET(self):
        if self.path in ("/v1/models", "/models"):
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.end_headers()
            resp = {
                "object": "list",
                "data": [
                    {"id": "hermes-3-mock", "object": "model", "owned_by": "nousresearch"}
                ]
            }
            self.wfile.write(json.dumps(resp).encode("utf-8"))
        elif self.path == "/health":
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.end_headers()
            self.wfile.write(json.dumps({"status": "ok"}).encode("utf-8"))
        else:
            self.send_response(404)
            self.end_headers()

    def do_POST(self):
        if self.path in ("/v1/chat/completions", "/chat/completions"):
            content_length = int(self.headers.get("Content-Length", 0))
            body = self.rfile.read(content_length)
            try:
                data = json.loads(body.decode("utf-8"))
            except Exception as e:
                self.send_response(400)
                self.end_headers()
                self.wfile.write(json.dumps({"error": str(e)}).encode("utf-8"))
                return

            messages = data.get("messages", [])
            has_tool_response = any(m.get("role") == "tool" for m in messages)

            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.end_headers()

            if not has_tool_response:
                # Turn 1: Trigger Hermes to execute k0maru flush_session
                resp = {
                    "id": "chatcmpl-mock-turn-1",
                    "object": "chat.completion",
                    "created": 1728390000,
                    "model": "hermes-3-mock",
                    "choices": [
                        {
                            "index": 0,
                            "message": {
                                "role": "assistant",
                                "content": None,
                                "tool_calls": [
                                    {
                                        "id": "call_hermes_flush_001",
                                        "type": "function",
                                        "function": {
                                            "name": "flush_session",
                                            "arguments": json.dumps({
                                                "title": "Hermes Docker E2E Architecture Decision",
                                                "content": "Verified in isolated Docker container with mock LLM server",
                                                "category": "decision",
                                                "summary": "Zero host pollution and zero API cost verified",
                                                "tags": ["hermes", "docker", "e2e"]
                                            })
                                        }
                                    }
                                ]
                            },
                            "finish_reason": "tool_calls"
                        }
                    ]
                }
            else:
                # Turn 2: Receive tool execution result and conclude
                resp = {
                    "id": "chatcmpl-mock-turn-2",
                    "object": "chat.completion",
                    "created": 1728390001,
                    "model": "hermes-3-mock",
                    "choices": [
                        {
                            "index": 0,
                            "message": {
                                "role": "assistant",
                                "content": "Knowledge crystallization successful. Verified k0maru-agent-memory integration in Hermes."
                            },
                            "finish_reason": "stop"
                        }
                    ]
                }

            self.wfile.write(json.dumps(resp).encode("utf-8"))
        else:
            self.send_response(404)
            self.end_headers()

def run():
    server = HTTPServer(("0.0.0.0", PORT), MockOpenAIHandler)
    sys.stderr.write(f"[MockLLM] Server started on 0.0.0.0:{PORT} (OpenAI Compatible)\n")
    sys.stderr.flush()
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        pass
    server.server_close()

if __name__ == "__main__":
    run()
