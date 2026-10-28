#!/usr/bin/env python3
"""
Unit tests for the token reduction benchmark suite.
"""

import glob
import os
import subprocess
import sys
import unittest

# Discover virtualenv site packages
_REPO_ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
for _pattern in [
    os.path.join(_REPO_ROOT, ".venv", "lib", "python*", "site-packages"),
    os.path.join(_REPO_ROOT, "benchmarks", ".venv", "lib", "python*", "site-packages"),
]:
    for _site_dir in glob.glob(_pattern):
        if _site_dir not in sys.path:
            sys.path.insert(0, _site_dir)

from benchmarks.run_token_benchmark import (
    evaluate_fixture,
    find_k0maru_binary,
    offload_log,
)
import tiktoken


class TestTokenBenchmark(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.k0maru_bin = find_k0maru_binary()
        cls.fixtures_dir = os.path.join(_REPO_ROOT, "benchmarks", "fixtures")
        cls.enc_cl100k = tiktoken.get_encoding("cl100k_base")
        cls.enc_o200k = tiktoken.get_encoding("o200k_base")

    def test_fixtures_exist_and_sized(self):
        expected = {
            "cargo_build_error.log": 500,
            "pytest_failures.log": 800,
            "jest_test_failures.log": 1200,
            "multithread_crash.log": 2500,
        }
        for fname, min_lines in expected.items():
            path = os.path.join(self.fixtures_dir, fname)
            self.assertTrue(os.path.isfile(path), f"Missing fixture {fname}")
            with open(path, "r", encoding="utf-8") as f:
                lines = f.readlines()
            self.assertGreaterEqual(
                len(lines),
                min_lines - 20,
                f"Fixture {fname} has too few lines ({len(lines)})",
            )

    def test_offload_produces_mermaid_graph(self):
        cargo_fixture = os.path.join(self.fixtures_dir, "cargo_build_error.log")
        with open(cargo_fixture, "r", encoding="utf-8") as f:
            raw = f.read()

        offloaded = offload_log(self.k0maru_bin, raw)
        self.assertIn("stateDiagram-v2", offloaded)
        self.assertIn("Offloaded", offloaded)
        self.assertIn("NodeID node_", offloaded)
        self.assertIn("k0maru inspect node_", offloaded)

    def test_evaluate_fixture_reduction_ratio(self):
        pytest_fixture = os.path.join(self.fixtures_dir, "pytest_failures.log")
        res = evaluate_fixture(
            pytest_fixture,
            self.k0maru_bin,
            self.enc_cl100k,
            self.enc_o200k,
        )
        self.assertEqual(res["fixture"], "pytest_failures.log")
        self.assertGreaterEqual(res["raw_lines"], 750)
        self.assertGreater(res["raw_tokens_cl100k"], 5000)
        self.assertLess(res["offloaded_tokens_cl100k"], 300)
        self.assertGreaterEqual(res["token_reduction_ratio_pct"], 65.0)
        self.assertGreaterEqual(res["reduction_o200k_pct"], 65.0)

    def test_cli_execution_and_assertion(self):
        script_path = os.path.join(_REPO_ROOT, "benchmarks", "run_token_benchmark.py")

        # Test successful run with assertion
        proc_pass = subprocess.run(
            [sys.executable, script_path, "--assert-min-reduction", "60.0"],
            capture_output=True,
            text=True,
        )
        self.assertEqual(proc_pass.returncode, 0, f"Benchmark failed: {proc_pass.stderr}")
        self.assertIn("OVERALL AVERAGE", proc_pass.stdout)
        self.assertIn("PASSED: All fixtures achieved >= 60.0%", proc_pass.stdout)

        # Test regression assertion exit code 1
        proc_fail = subprocess.run(
            [sys.executable, script_path, "--assert-min-reduction", "99.99"],
            capture_output=True,
            text=True,
        )
        self.assertEqual(proc_fail.returncode, 1)
        self.assertIn("ASSERTION FAILED", proc_fail.stderr)


if __name__ == "__main__":
    unittest.main()
