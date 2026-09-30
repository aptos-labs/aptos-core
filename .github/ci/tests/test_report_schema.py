import json
import math
import unittest

from ci_actions import report_schema
from tests.helpers import SHA

BINDING = {"producer": "mono-move-e2e-perf", "run_id": 1234, "pr_number": 99, "head_sha": SHA}
MICRO_BINDING = {**BINDING, "producer": "mono-move-micro-bench"}


def e2e_report(**overrides):
    return {
        "schema": "pr-ci-report-v1", "producer": "mono-move-e2e-perf", "run_id": 1234, "pr_number": 99,
        "head_sha": SHA, "status": "passed",
        "metrics": [{"name": "apt-fa-transfer", "verdict": "ok", "v1_tps": 100.0, "mono_tps": 125.0,
                     "execution_speedup": 1.25, "max_execution_spread": 0.02}],
        **overrides,
    }


def micro_report(**overrides):
    return {
        "schema": "pr-ci-report-v1", "producer": "mono-move-micro-bench", "run_id": 1234, "pr_number": 99,
        "head_sha": SHA, "status": "failed",
        "metrics": [{"name": "fib/mono", "verdict": "regression", "mean_percent": 4.2, "ci_low_percent": 3.1,
                     "ci_high_percent": 5.3, "base_median_ns": 1000.0, "pr_median_ns": 1042.0}],
        **overrides,
    }


class ReportSchemaTests(unittest.TestCase):
    def validate(self, report, binding=BINDING):
        return report_schema.parse_and_validate_report(json.dumps(report).encode(), binding)

    def test_valid_reports_for_both_producers_render_from_fixed_templates(self):
        e2e = report_schema.render_report(self.validate(e2e_report()))
        micro = report_schema.render_report(self.validate(micro_report(), MICRO_BINDING))
        self.assertRegex(e2e, r"^### MonoMove E2E performance")
        self.assertIn("| `apt-fa-transfer` | ok | 100.00 | 125.00 | 1.25x | 2.00% |", e2e)
        self.assertRegex(micro, r"^### MonoMove micro-benchmark")
        self.assertIn("| `fib/mono` | regression | 4.20% | [3.10%, 5.30%] | 1000.00ns | 1042.00ns |", micro)

    def test_renders_null_metrics_as_n_a(self):
        report = micro_report()
        report["metrics"][0].update(mean_percent=None, ci_low_percent=None, base_median_ns=None)
        self.assertIn("| `fib/mono` | regression | n/a | n/a | n/a | 1042.00ns |",
                      report_schema.render_report(self.validate(report, MICRO_BINDING)))

    def test_rejects_forged_producer_pr_sha_and_run_id(self):
        for key, value in [("producer", "mono-move-micro-bench"), ("pr_number", 100),
                           ("head_sha", "f" * 40), ("run_id", 1235)]:
            with self.subTest(key=key), self.assertRaisesRegex(ValueError, "trusted workflow run metadata"):
                self.validate(e2e_report(), {**BINDING, key: value})

    def test_rejects_unknown_keys_invalid_enums_and_non_finite_values(self):
        with self.assertRaisesRegex(ValueError, "unknown key"):
            self.validate(e2e_report(extra=True))
        with self.assertRaisesRegex(ValueError, "missing key"):
            self.validate({key: value for key, value in e2e_report().items() if key != "status"})
        with self.assertRaisesRegex(ValueError, "invalid report schema"):
            self.validate(e2e_report(schema="pr-ci-report-v2"))
        with self.assertRaisesRegex(ValueError, "invalid report status"):
            self.validate(e2e_report(status="maybe"))
        report = e2e_report()
        report["metrics"][0]["verdict"] = "fast"
        with self.assertRaisesRegex(ValueError, "invalid verdict"):
            self.validate(report)
        report = e2e_report()
        report["metrics"][0]["v1_tps"] = math.inf
        with self.assertRaisesRegex(ValueError, "finite"):
            report_schema.validate_report(report, BINDING)
        for field, value in [("producer", []), ("status", []), ("run_id", True), ("pr_number", 0),
                             ("head_sha", "A" * 40), ("metrics", {})]:
            with self.subTest(field=field), self.assertRaises(ValueError):
                self.validate(e2e_report(**{field: value}))
        report = e2e_report()
        report["metrics"][0]["verdict"] = []
        with self.assertRaises(ValueError):
            self.validate(report)

    def test_rejects_extreme_finite_metric_magnitudes(self):
        for field, value in [("v1_tps", -1.0), ("mono_tps", 1e308), ("execution_speedup", 1e308),
                             ("max_execution_spread", 1e308), ("max_execution_spread", 10**400)]:
            report = e2e_report()
            report["metrics"][0][field] = value
            with self.subTest(field=field), self.assertRaisesRegex(ValueError, "bounded range"):
                self.validate(report)
        report = micro_report()
        report["metrics"][0]["mean_percent"] = 1e308
        with self.assertRaisesRegex(ValueError, "bounded range"):
            self.validate(report, MICRO_BINDING)

    def test_rejects_excess_rows_long_strings_mentions_controls_and_markdown(self):
        with self.assertRaisesRegex(ValueError, "at most 100"):
            self.validate(e2e_report(metrics=e2e_report()["metrics"] * 101))
        for name in ["a" * 129, "hello@team", "line\nbreak", "[link](bad)", "tick`code", "-leading", ""]:
            report = e2e_report()
            report["metrics"][0]["name"] = name
            with self.subTest(name=name), self.assertRaisesRegex(ValueError, "safe plain text"):
                self.validate(report)

    def test_rejects_malformed_json_non_standard_constants_and_oversized_reports(self):
        with self.assertRaisesRegex(ValueError, "malformed JSON"):
            report_schema.parse_and_validate_report(b"{", BINDING)
        with self.assertRaisesRegex(ValueError, "malformed JSON"):
            report_schema.parse_and_validate_report(b"\xff", BINDING)
        with self.assertRaisesRegex(ValueError, "finite"):
            report_schema.parse_and_validate_report(json.dumps(e2e_report()).replace("100.0", "NaN").encode(), BINDING)
        with self.assertRaisesRegex(ValueError, "one MiB"):
            report_schema.parse_and_validate_report(b" " * (1024 * 1024 + 1), BINDING)

    def test_rejects_duplicate_keys_at_every_object_level(self):
        root_duplicate = json.dumps(e2e_report())[:-1] + ',"status":"failed"}'
        row_duplicate = json.dumps(e2e_report()).replace('"verdict": "ok"', '"verdict": "ok", "verdict": "failed"')
        for payload in (root_duplicate, row_duplicate):
            with self.assertRaisesRegex(ValueError, "duplicate key"):
                report_schema.parse_and_validate_report(payload.encode(), BINDING)

    def test_missing_report_is_fixed_from_the_trusted_binding(self):
        report = report_schema.missing_report(BINDING)
        self.assertEqual(report, e2e_report(status="failed", metrics=[]))
        rendered = report_schema.render_report(report)
        self.assertIn("Validated result: **failed**", rendered)
        self.assertIn("No metric rows were produced", rendered)


if __name__ == "__main__":
    unittest.main()
