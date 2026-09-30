import json
import os
import pathlib
import shutil
import subprocess
import sys
import tempfile
import types
import unittest
from unittest.mock import patch


MODULE_PATH = pathlib.Path(__file__).with_name("e2e_ci_report.py")
sys.path.insert(0, str(MODULE_PATH.parent))
sys.path.insert(0, str(MODULE_PATH.parent.parent))
import e2e_ci_report as MODULE  # noqa: E402

# Tests may import the trusted validator; the producer adapter must not.
sys.path.insert(0, str(MODULE_PATH.parents[5] / ".github" / "ci"))
from ci_actions.report_schema import parse_and_validate_report, validate_report  # noqa: E402

SHA = "0123456789abcdef0123456789abcdef01234567"
BINDING = {"producer": "mono-move-e2e-perf", "run_id": 1234, "pr_number": 99, "head_sha": SHA}


class ProducerReportTests(unittest.TestCase):
    def result(self):
        return types.SimpleNamespace(
            workload=types.SimpleNamespace(name="apt-fa-transfer"),
            verdict="ok",
            v1_runs=[types.SimpleNamespace(tps=100.0)],
            mono_runs=[types.SimpleNamespace(tps=125.0)],
            speedup={"execution": 1.25},
            spread={"execution": 0.02, "inner_block_executor": 0.04},
        )

    def summarize(self, runs, metric):
        self.assertEqual(metric, "execution")
        return runs[0].tps, 0.01

    def build(self, results, failures=(), **overrides):
        arguments = {
            "pr_number": 99,
            "head_sha": SHA,
            "run_id": 1234,
            "status": "passed",
            "summarize": self.summarize,
            "verdict_metrics": ("execution", "inner_block_executor"),
            **overrides,
        }
        return MODULE.build_pr_ci_report(results, list(failures), **arguments)

    def test_builds_bounded_rows_from_explicit_summarizer_and_verdict_metrics(self):
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
            "max_execution_spread": 0.04,
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

    def test_writer_skips_environment_binding_when_no_path_is_given(self):
        with patch.dict(os.environ, {}, clear=True):
            MODULE.write_ci_report_if_requested(
                [self.result()], [], "passed", report_path=None,
                summarize=self.summarize, verdict_metrics=("execution", "inner_block_executor"),
            )

    def test_writer_uses_existing_environment_binding_and_emits_valid_json(self):
        with tempfile.TemporaryDirectory() as directory:
            path = pathlib.Path(directory) / "report.json"
            with patch.dict(os.environ, {
                "PR_CI_PR_NUMBER": "99",
                "PR_CI_HEAD_SHA": SHA,
                "PR_CI_RUN_ID": "1234",
            }, clear=True):
                MODULE.write_ci_report_if_requested(
                    [self.result()], [], "passed", report_path=path,
                    summarize=self.summarize,
                    verdict_metrics=("execution", "inner_block_executor"),
                )
            payload = path.read_bytes()
        self.assertTrue(payload.endswith(b"\n"))
        self.assertNotIn(b"\n", payload[:-1])
        self.assertEqual(parse_and_validate_report(payload, BINDING), json.loads(payload))

    def test_writer_rejects_non_finite_numbers(self):
        result = self.result()
        result.speedup["execution"] = float("nan")
        with tempfile.TemporaryDirectory() as directory:
            with patch.dict(os.environ, {
                "PR_CI_PR_NUMBER": "99",
                "PR_CI_HEAD_SHA": SHA,
                "PR_CI_RUN_ID": "1234",
            }, clear=True), self.assertRaises(ValueError):
                MODULE.write_ci_report_if_requested(
                    [result], [], "passed", report_path=pathlib.Path(directory) / "report.json",
                    summarize=self.summarize,
                    verdict_metrics=("execution", "inner_block_executor"),
                )

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

    def test_adapter_imports_without_a_runner_checkout(self):
        helper_path = MODULE_PATH.parent.parent / "pr_ci_report.py"
        with tempfile.TemporaryDirectory() as directory:
            staged = pathlib.Path(directory)
            shutil.copy2(MODULE_PATH, staged / "e2e_ci_report.py")
            shutil.copy2(helper_path, staged / "pr_ci_report.py")
            result = subprocess.run(
                [
                    sys.executable,
                    "-I",
                    "-c",
                    "import sys; sys.path.insert(0, sys.argv[1]); import e2e_ci_report; print('ready')",
                    str(staged),
                ],
                cwd=staged,
                capture_output=True,
                text=True,
                check=False,
            )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout.strip(), "ready")


if __name__ == "__main__":
    unittest.main()
