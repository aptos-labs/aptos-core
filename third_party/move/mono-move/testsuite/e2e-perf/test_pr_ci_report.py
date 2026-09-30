import json
import math
import os
import pathlib
import shutil
import subprocess
import sys
import tempfile
import types
import unittest
from unittest.mock import Mock, patch

from hypothesis import given, strategies as st

MODULE_PATH = pathlib.Path(__file__).with_name("e2e_ci_report.py")
sys.path.insert(0, str(MODULE_PATH.parent))
sys.path.insert(0, str(MODULE_PATH.parent.parent))
import e2e_ci_report as MODULE  # noqa: E402

# Tests may import the trusted validator; the producer adapter must not.
sys.path.insert(0, str(MODULE_PATH.parents[5] / ".github" / "ci"))
from ci_actions.report_schema import parse_and_validate_report, validate_report  # noqa: E402
from tests.property_support import POSITIVE_ID, SAFE_COMPONENT, SHA as SHA_STRATEGY, configure_profiles  # noqa: E402

configure_profiles()

TPS = st.one_of(st.none(), st.integers(min_value=0, max_value=10**12))
RATE = st.floats(min_value=0, max_value=1e6, allow_nan=False, allow_infinity=False)
E2E_INPUTS = st.lists(st.fixed_dictionaries({
    "name": SAFE_COMPONENT.map(lambda name: "a" + name), "verdict": st.sampled_from(
        ("ok", "improvement", "regression", "noisy", "uncalibrated", "failed", "self-compare")),
    "v1": TPS, "mono": TPS, "speedup": st.one_of(st.none(), RATE),
    "execution": RATE, "inner_block_executor": RATE,
}), max_size=10)
FAILURES = st.lists(st.tuples(SAFE_COMPONENT.map(lambda name: "a" + name), st.text(max_size=64)), max_size=10)

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


class ProducerReportProperties(unittest.TestCase):
    def result_from(self, item):
        return types.SimpleNamespace(
            workload=types.SimpleNamespace(name=item["name"]), verdict=item["verdict"],
            v1_runs=[types.SimpleNamespace(tps=item["v1"])],
            mono_runs=[types.SimpleNamespace(tps=item["mono"])],
            speedup={"execution": item["speedup"], "unrelated": 999},
            spread={"execution": item["execution"], "inner_block_executor": item["inner_block_executor"],
                    "unrelated": 1e9},
        )

    @given(inputs=E2E_INPUTS, failures=FAILURES, run_id=POSITIVE_ID, pr_number=POSITIVE_ID,
           head_sha=SHA_STRATEGY, status=st.sampled_from(("passed", "failed")),
           verdict_metrics=st.sampled_from((("execution",), ("inner_block_executor",),
                                           ("execution", "inner_block_executor"))))
    def test_e2e_projection_writer_and_consumer_share_binding(
        self, inputs, failures, run_id, pr_number, head_sha, status, verdict_metrics
    ):
        results = [self.result_from(item) for item in inputs]
        summarize = Mock(side_effect=lambda runs, metric: (runs[0].tps, "@discarded"))
        expected_rows = [{
            "name": item["name"], "verdict": item["verdict"], "v1_tps": item["v1"],
            "mono_tps": item["mono"], "execution_speedup": item["speedup"],
            "max_execution_spread": max(item[metric] for metric in verdict_metrics),
        } for item in inputs] + [{
            "name": name, "verdict": "failed", "v1_tps": None, "mono_tps": None,
            "execution_speedup": None, "max_execution_spread": None,
        } for name, _reason in failures]
        binding = {"producer": "mono-move-e2e-perf", "run_id": run_id,
                   "pr_number": pr_number, "head_sha": head_sha}
        expected = {**binding, "schema": "pr-ci-report-v1", "status": status, "metrics": expected_rows}
        actual = MODULE.build_pr_ci_report(results, failures, run_id=run_id, pr_number=pr_number,
                                          head_sha=head_sha, status=status, summarize=summarize,
                                          verdict_metrics=verdict_metrics)
        self.assertEqual(expected, actual)
        expected_calls = [(runs, "execution") for result in results for runs in (result.v1_runs, result.mono_runs)]
        self.assertEqual(expected_calls, [call.args for call in summarize.call_args_list])
        summarize.reset_mock()
        with patch.dict(os.environ, {"PR_CI_RUN_ID": str(run_id), "PR_CI_PR_NUMBER": str(pr_number),
                                    "PR_CI_HEAD_SHA": head_sha}, clear=True), \
                patch.object(pathlib.Path, "write_text", autospec=True) as write:
            MODULE.write_ci_report_if_requested(results, failures, status, report_path="unused.json",
                                                summarize=summarize, verdict_metrics=verdict_metrics)
            text = write.call_args.args[1]
            self.assertEqual("utf-8", write.call_args.kwargs["encoding"])
            MODULE.write_pr_ci_report("unused.json", dict(reversed(list(actual.items()))))
            self.assertEqual(text, write.call_args.args[1])
        self.assertTrue(text.endswith("\n"))
        self.assertNotIn("\n", text[:-1])
        self.assertEqual(json.dumps(expected, allow_nan=False, separators=(",", ":"), sort_keys=True) + "\n", text)
        self.assertEqual(sorted(actual), list(json.loads(text)))
        self.assertEqual(expected, parse_and_validate_report(text.encode(), binding))
        self.assertEqual(expected_calls, [call.args for call in summarize.call_args_list])
        for field, value in {"run_id": run_id + 1, "pr_number": pr_number + 1,
                             "head_sha": ("1" if head_sha[0] == "0" else "0") + head_sha[1:],
                             "producer": "mono-move-micro-bench"}.items():
            with self.assertRaisesRegex(ValueError, "trusted workflow run metadata"):
                parse_and_validate_report(text.encode(), {**binding, field: value})
        changed_reasons = [(name, "@team [link](unsafe)\n" + reason) for name, reason in failures]
        self.assertEqual(expected, MODULE.build_pr_ci_report(
            results, changed_reasons, run_id=run_id, pr_number=pr_number, head_sha=head_sha, status=status,
            summarize=lambda runs, metric: (runs[0].tps, None), verdict_metrics=verdict_metrics))

    @given(inputs=E2E_INPUTS, failures=FAILURES)
    def test_absent_report_path_has_no_environment_or_writer_effects(self, inputs, failures):
        for path in (None, ""):
            summarize = Mock()
            with patch.dict(os.environ, {}, clear=True), patch.object(MODULE, "write_pr_ci_report") as writer:
                MODULE.write_ci_report_if_requested([self.result_from(item) for item in inputs], failures,
                                                    "passed", report_path=path, summarize=summarize,
                                                    verdict_metrics=("execution",))
                summarize.assert_not_called()
                writer.assert_not_called()

    def test_each_environment_binding_failure_occurs_before_write(self):
        env = {"PR_CI_RUN_ID": "1234", "PR_CI_PR_NUMBER": "99", "PR_CI_HEAD_SHA": SHA}
        for field in env:
            with self.subTest(missing=field), patch.dict(os.environ, {
                key: value for key, value in env.items() if key != field
            }, clear=True), patch.object(pathlib.Path, "write_text") as write:
                with self.assertRaises(KeyError):
                    MODULE.write_ci_report_if_requested([], [], "passed", report_path="unused.json",
                                                        summarize=Mock(), verdict_metrics=("execution",))
                write.assert_not_called()
        for field in ("PR_CI_RUN_ID", "PR_CI_PR_NUMBER"):
            for value in ("", "bad", "1.0"):
                with self.subTest(field=field, value=value), patch.dict(os.environ, {**env, field: value}, clear=True), \
                        patch.object(pathlib.Path, "write_text") as write:
                    with self.assertRaises(ValueError):
                        MODULE.write_ci_report_if_requested([], [], "passed", report_path="unused.json",
                                                            summarize=Mock(), verdict_metrics=("execution",))
                    write.assert_not_called()

    def test_each_non_finite_writer_value_rejects_before_write(self):
        for field in ("v1_tps", "mono_tps", "execution_speedup", "max_execution_spread"):
            for value in (math.nan, math.inf, -math.inf):
                report = ProducerReportTests().build([ProducerReportTests().result()])
                report["metrics"][0][field] = value
                with self.subTest(field=field, value=value), patch.object(pathlib.Path, "write_text") as write:
                    with self.assertRaises(ValueError):
                        MODULE.write_pr_ci_report("unused.json", report)
                    write.assert_not_called()

    def test_writer_serializes_finite_invalid_binding_but_consumer_rejects(self):
        env = {"PR_CI_RUN_ID": "1234", "PR_CI_PR_NUMBER": "99", "PR_CI_HEAD_SHA": SHA}
        for field, value in (("PR_CI_RUN_ID", "0"), ("PR_CI_PR_NUMBER", "-1"), ("PR_CI_HEAD_SHA", "bad")):
            with self.subTest(field=field), patch.dict(os.environ, {**env, field: value}, clear=True), \
                    patch.object(pathlib.Path, "write_text", autospec=True) as write:
                MODULE.write_ci_report_if_requested([], [], "passed", report_path="unused.json",
                                                    summarize=Mock(), verdict_metrics=("execution",))
                with self.assertRaises(ValueError):
                    parse_and_validate_report(write.call_args.args[1].encode(), BINDING)

    def test_adapter_numeric_endpoints_and_consumer_row_limit(self):
        for tps, speedup, spread in ((0, 0, 0), (10**12, 1e6, 1e6), (None, None, 0)):
            item = {"name": "metric", "verdict": "ok", "v1": tps, "mono": tps, "speedup": speedup,
                    "execution": spread, "inner_block_executor": spread}
            result = self.result_from(item)
            report = MODULE.build_pr_ci_report(
                [result] * 50, [("failure", "@reason")] * 50, run_id=1234, pr_number=99, head_sha=SHA,
                status="failed", summarize=lambda runs, metric: (runs[0].tps, None), verdict_metrics=("execution",))
            self.assertEqual(report, parse_and_validate_report(json.dumps(report).encode(), BINDING))
            report["metrics"].append(dict(report["metrics"][-1]))
            with self.assertRaisesRegex(ValueError, "at most 100"):
                parse_and_validate_report(json.dumps(report).encode(), BINDING)


if __name__ == "__main__":
    unittest.main()
