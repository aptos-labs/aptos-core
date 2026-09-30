import importlib.util
import json
import pathlib
import sys
import tempfile
import types
import unittest


sys.modules.setdefault("tabulate", types.SimpleNamespace(tabulate=lambda *args, **kwargs: ""))
MODULE_PATH = pathlib.Path(__file__).with_name("run_e2e_perf_test.py")
SPEC = importlib.util.spec_from_file_location("run_e2e_perf_test", MODULE_PATH)
MODULE = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = MODULE
SPEC.loader.exec_module(MODULE)

# Tests may import the trusted validator; run_e2e_perf_test.py itself must not.
sys.path.insert(0, str(MODULE_PATH.parents[5] / ".github" / "ci"))
from ci_actions.report_schema import parse_and_validate_report, validate_report  # noqa: E402

SHA = "0123456789abcdef0123456789abcdef01234567"
BINDING = {"producer": "mono-move-e2e-perf", "run_id": 1234, "pr_number": 99, "head_sha": SHA}


class ProducerReportTests(unittest.TestCase):
    def result(self):
        workload = MODULE.Workload("apt-fa-transfer", 500, "transfer")
        stages_v1 = {name: 100.0 for name in MODULE.METRICS if name not in ("total", "output_bytes_per_txn")}
        stages_mono = {name: 125.0 for name in MODULE.METRICS if name not in ("total", "output_bytes_per_txn")}
        result = MODULE.WorkloadResult(workload=workload, verdict="ok")
        result.v1_runs = [MODULE.RunStats(100.0, stages_v1, 10.0, False)]
        result.mono_runs = [MODULE.RunStats(125.0, stages_mono, 9.0, True)]
        result.speedup = {name: 1.25 for name in MODULE.METRICS}
        result.spread = {name: 0.02 for name in MODULE.METRICS}
        return result

    def build(self, results, failures=(), **overrides):
        arguments = {"pr_number": 99, "head_sha": SHA, "run_id": 1234, "status": "passed", **overrides}
        return MODULE.build_pr_ci_report(results, list(failures), **arguments)

    def test_builds_bounded_typed_rows_without_failure_text(self):
        report = self.build(
            [self.result()], [("no-op", "attacker-controlled [markdown] @team")], status="failed"
        )
        self.assertEqual(set(report), {
            "schema", "producer", "run_id", "pr_number", "head_sha", "status", "metrics"
        })
        self.assertEqual(report["producer"], "mono-move-e2e-perf")
        self.assertEqual(report["metrics"][0], {
            "name": "apt-fa-transfer",
            "verdict": "ok",
            "v1_tps": 100.0,
            "mono_tps": 125.0,
            "execution_speedup": 1.25,
            "max_execution_spread": 0.02,
        })
        self.assertEqual(report["metrics"][1], {
            "name": "no-op",
            "verdict": "failed",
            "v1_tps": None,
            "mono_tps": None,
            "execution_speedup": None,
            "max_execution_spread": None,
        })
        self.assertNotIn("attacker-controlled", json.dumps(report))
        validate_report(report, BINDING)

    def test_writer_rejects_non_finite_numbers(self):
        result = self.result()
        result.speedup["execution"] = float("nan")
        with tempfile.TemporaryDirectory() as directory:
            with self.assertRaises(ValueError):
                MODULE.write_pr_ci_report(pathlib.Path(directory) / "report.json", self.build([result]))

    def test_trusted_validator_rejects_invalid_binding_status_and_row_count(self):
        cases = [
            ({"pr_number": 0}, {"pr_number": 0}),
            ({"run_id": 2**53}, {"run_id": 2**53}),
            ({"head_sha": "A" * 40}, {"head_sha": "A" * 40}),
            ({"status": "unknown"}, {}),
        ]
        for override, binding in cases:
            with self.subTest(override=override), self.assertRaises(ValueError):
                validate_report(self.build([self.result()], **override), {**BINDING, **binding})
        with self.assertRaisesRegex(ValueError, "at most 100"):
            validate_report(self.build([self.result()] * 101), BINDING)

    def test_actual_output_is_deterministic_and_accepted_by_trusted_validator(self):
        with tempfile.TemporaryDirectory() as directory:
            path = pathlib.Path(directory) / "report.json"
            MODULE.write_pr_ci_report(path, self.build([self.result()]))
            payload = path.read_bytes()
        self.assertTrue(payload.endswith(b"\n"))
        self.assertNotIn(b"\n", payload[:-1])
        self.assertEqual(parse_and_validate_report(payload, BINDING), json.loads(payload))


if __name__ == "__main__":
    unittest.main()
