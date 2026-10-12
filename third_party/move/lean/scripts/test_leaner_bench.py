# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

import importlib.util
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location("leaner_bench", Path(__file__).with_name("leaner-bench.py"))
bench = importlib.util.module_from_spec(spec)
spec.loader.exec_module(bench)


class BenchmarkOutcomeTest(unittest.TestCase):
    problem = {"expected_rejections": ["amm::negative"]}

    def result(self, outcomes, errors=2):
        return {"status": "failed", "outcomes": outcomes, "errors": errors}

    def test_only_declared_rejection_is_expected(self):
        result = self.result([
            {"target": "amm::positive", "status": "verified", "errors": 0},
            {"target": "amm::negative", "status": "rejected", "errors": 2},
        ])
        self.assertEqual(bench.expected_outcome(self.problem, result)["status"], "expected rejection")

    def test_timeout_is_not_expected_rejection(self):
        result = self.result([{"target": "amm::negative", "status": "timeout", "errors": 2}])
        self.assertEqual(bench.expected_outcome(self.problem, result)["status"], "timeout")

    def test_unrelated_rejection_is_not_hidden(self):
        result = self.result([
            {"target": "amm::positive", "status": "rejected", "errors": 1},
            {"target": "amm::negative", "status": "rejected", "errors": 2},
        ], errors=3)
        self.assertEqual(bench.expected_outcome(self.problem, result)["status"], "failed")

    def test_unattributed_error_is_not_hidden(self):
        result = self.result([{"target": "amm::negative", "status": "rejected", "errors": 2}], errors=3)
        self.assertEqual(bench.expected_outcome(self.problem, result)["status"], "failed")

    def test_inconsistent_success_cannot_satisfy_expected_rejection(self):
        result = {"status": "verified", "errors": 0, "outcomes": [
            {"target": "amm::negative", "status": "rejected", "errors": 1},
        ]}
        self.assertEqual(bench.expected_outcome(self.problem, result)["status"], "failed")

    def test_missing_target_and_accepted_negative_are_failures(self):
        self.assertEqual(bench.expected_outcome(self.problem, self.result([]))["status"], "failed")
        result = self.result([{"target": "amm::negative", "status": "verified", "errors": 0}], errors=0)
        self.assertEqual(bench.expected_outcome(self.problem, result)["status"], "unexpected acceptance")

    def test_suite_includes_rejections_and_available_measurements(self):
        point = {"problems": [
            {"status": "verified", "heartbeats": {"total": 10}},
            {"status": "expected rejection", "heartbeats": {"total": 20}},
            {"status": "failed", "heartbeats": {"total": 30}},
            {"status": "timeout", "heartbeats": {"total": 40}},
            {"status": "crashed"},
        ]}
        self.assertEqual(bench.suite_value(point, "heartbeats"), 100)
        self.assertIsNone(bench.suite_value({**point, "subset": True}, "heartbeats"))

    def test_comparison_includes_rejections_and_displays_status(self):
        before = {"problems": [
            {"name": "amm", "status": "failed", "wall_ms": {"total": 100},
             "heartbeats": {"total": 1500}},
            {"name": "missing", "status": "crashed"},
        ]}
        after = {"problems": [
            {"name": "amm", "status": "expected rejection", "wall_ms": {"total": 20},
             "heartbeats": {"total": 100}},
            {"name": "missing", "status": "verified", "wall_ms": {"total": 5},
             "heartbeats": {"total": 50}},
        ]}
        rows = bench.local_comparison(before, after)
        self.assertEqual(rows, [("amm", 1500, 100, 1.0, 1.0, "failed", "expected rejection")])
        changes = bench.latest_changes([before, after], "amm")
        self.assertAlmostEqual(changes["heartbeats"], -93.33333333333333)


if __name__ == "__main__":
    unittest.main()
