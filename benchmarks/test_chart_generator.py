#!/usr/bin/env python3
"""
Unit test for benchmark chart generator.
"""

import os
import subprocess
import sys
import unittest

_REPO_ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
CHARTS_DIR = os.path.join(_REPO_ROOT, "benchmarks", "charts")


class TestChartGenerator(unittest.TestCase):
    def test_charts_generation_and_validity(self):
        script_path = os.path.join(_REPO_ROOT, "benchmarks", "generate_charts.py")
        proc = subprocess.run([sys.executable, script_path], capture_output=True, text=True)
        self.assertEqual(proc.returncode, 0, f"Chart generation failed:\n{proc.stderr}")

        expected_charts = [
            "token_savings_bar.png",
            "cumulative_token_curve.png",
            "systems_log_comparison.png",
        ]

        png_magic = b"\x89PNG\r\n\x1a\n"

        for chart_name in expected_charts:
            chart_path = os.path.join(CHARTS_DIR, chart_name)
            self.assertTrue(os.path.isfile(chart_path), f"Missing chart: {chart_name}")
            file_size = os.path.getsize(chart_path)
            self.assertGreater(file_size, 10000, f"Chart {chart_name} is too small ({file_size} bytes)")

            with open(chart_path, "rb") as f:
                header = f.read(8)
                self.assertEqual(header, png_magic, f"{chart_name} is not a valid PNG file")


if __name__ == "__main__":
    unittest.main()
