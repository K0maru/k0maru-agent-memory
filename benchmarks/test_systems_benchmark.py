#!/usr/bin/env python3
"""
Unit tests for the systems performance & latency benchmark runner.
"""

import os
import subprocess
import sys
import tempfile
import unittest

_REPO_ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))

from benchmarks.run_systems_benchmark import (
    find_k0maru_binary,
    get_binary_size_mb,
    measure_cache_rebuild,
    measure_cold_start,
)


class TestSystemsBenchmark(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.k0maru_bin = find_k0maru_binary()

    def test_binary_discovery_and_size(self):
        self.assertTrue(os.path.isfile(self.k0maru_bin))
        self.assertTrue(os.access(self.k0maru_bin, os.X_OK))
        size_mb = get_binary_size_mb(self.k0maru_bin)
        self.assertGreater(size_mb, 1.0)
        self.assertLess(size_mb, 50.0)

    def test_cold_start_measurement(self):
        result = measure_cold_start(self.k0maru_bin, runs=10)
        self.assertEqual(result["runs"], 10)
        self.assertGreater(result["min_ms"], 0.0)
        self.assertGreaterEqual(result["max_ms"], result["min_ms"])
        self.assertGreater(result["median_ms"], 0.0)
        # Verify median cold start is under 15ms (robust across test runners)
        self.assertLess(result["median_ms"], 15.0)

    def test_cache_rebuild_measurement(self):
        result = measure_cache_rebuild(self.k0maru_bin, num_docs=25)
        self.assertEqual(result["num_docs"], 25)
        self.assertGreater(result["sync_wall_ms"], 0.0)
        self.assertGreater(result["throughput_fps"], 0.0)
        self.assertGreater(result["peak_rss_mb"], 0.0)
        self.assertGreater(result["incremental_wall_ms"], 0.0)

    def test_cli_execution_and_assertion(self):
        script_path = os.path.join(_REPO_ROOT, "benchmarks", "run_systems_benchmark.py")
        with tempfile.TemporaryDirectory() as tmp_dir:
            csv_path = os.path.join(tmp_dir, "test_systems.csv")

            # Passing assertion
            proc_pass = subprocess.run(
                [
                    sys.executable,
                    script_path,
                    "--runs",
                    "5",
                    "--mock-docs",
                    "15",
                    "--output-csv",
                    csv_path,
                    "--assert-max-cold-start-ms",
                    "50.0",
                ],
                capture_output=True,
                text=True,
            )
            self.assertEqual(proc_pass.returncode, 0, f"Benchmark failed: {proc_pass.stderr}")
            self.assertTrue(os.path.isfile(csv_path))
            self.assertIn("ARCHITECTURAL & RESOURCE COMPARISON", proc_pass.stdout)
            self.assertIn("TencentDB-Agent-Memory", proc_pass.stdout)
            self.assertIn("PASSED: Cold start median latency", proc_pass.stdout)

            # Failing assertion
            proc_fail = subprocess.run(
                [
                    sys.executable,
                    script_path,
                    "--runs",
                    "5",
                    "--mock-docs",
                    "10",
                    "--output-csv",
                    csv_path,
                    "--assert-max-cold-start-ms",
                    "0.01",
                ],
                capture_output=True,
                text=True,
            )
            self.assertEqual(proc_fail.returncode, 1)
            self.assertIn("ASSERTION FAILED", proc_fail.stderr)


if __name__ == "__main__":
    unittest.main()
