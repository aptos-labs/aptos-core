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
from ci_actions.report_schema import parse_and_validate_report  # noqa: E402
from tests.helpers import SHA  # noqa: E402
from tests.property_support import POSITIVE_ID, SAFE_COMPONENT, SHA as SHA_STRATEGY, configure_profiles  # noqa: E402
from tests.report_support import CONTRACTS  # noqa: E402

configure_profiles()

TPS = st.one_of(st.none(), st.integers(min_value=0, max_value=10**12))
RATE = st.floats(min_value=0, max_value=1e6, allow_nan=False, allow_infinity=False)
E2E_INPUTS = st.lists(st.fixed_dictionaries({
    "name": SAFE_COMPONENT.map(lambda name: "a" + name),
    "verdict": st.sampled_from(CONTRACTS["mono-move-e2e-perf"]["verdicts"]),
    "v1": TPS, "mono": TPS, "speedup": st.one_of(st.none(), RATE),
    "execution": RATE, "inner_block_executor": RATE,
}), max_size=10)
FAILURES = st.lists(st.tuples(SAFE_COMPONENT.map(lambda name: "a" + name), st.text(max_size=64)), max_size=10)
ENV = {"PR_CI_RUN_ID": "1234", "PR_CI_PR_NUMBER": "99", "PR_CI_HEAD_SHA": SHA}


def result_from(item):
    return types.SimpleNamespace(
        workload=types.SimpleNamespace(name=item["name"]), verdict=item["verdict"],
        v1_runs=[types.SimpleNamespace(tps=item["v1"])],
        mono_runs=[types.SimpleNamespace(tps=item["mono"])],
        speedup={"execution": item["speedup"], "unrelated": 999},
        spread={"execution": item["execution"], "inner_block_executor": item["inner_block_executor"],
                "unrelated": 1e9},
    )


class ProducerReportTests(unittest.TestCase):
    def test_fixed_result_projects_to_hand_computed_rows(self):
        result = result_from({"name": "apt-fa-transfer", "verdict": "ok", "v1": 100.0, "mono": 125.0,
                              "speedup": 1.25, "execution": 0.02, "inner_block_executor": 0.04})
        report = MODULE.build_pr_ci_report(
            [result], [("no-op", "reason")], run_id=1234, pr_number=99, head_sha=SHA, status="failed",
            summarize=lambda runs, metric: (runs[0].tps, None), verdict_metrics=("execution", "inner_block_executor"))
        self.assertEqual([
            {"name": "apt-fa-transfer", "verdict": "ok", "v1_tps": 100.0, "mono_tps": 125.0,
             "execution_speedup": 1.25, "max_execution_spread": 0.04},
            {"name": "no-op", "verdict": "failed", "v1_tps": None, "mono_tps": None,
             "execution_speedup": None, "max_execution_spread": None},
        ], report["metrics"])

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

    def test_absent_report_path_has_no_environment_or_writer_effects(self):
        for path in (None, ""):
            summarize = Mock()
            with patch.dict(os.environ, {}, clear=True), patch.object(MODULE, "write_pr_ci_report") as writer:
                MODULE.write_ci_report_if_requested([], [], "passed", report_path=path, summarize=summarize,
                                                    verdict_metrics=("execution",))
                summarize.assert_not_called()
                writer.assert_not_called()

    def test_each_environment_binding_failure_occurs_before_write(self):
        cases = [({key: value for key, value in ENV.items() if key != missing}, KeyError) for missing in ENV]
        cases.append(({**ENV, "PR_CI_RUN_ID": "bad"}, ValueError))
        for env, error in cases:
            with self.subTest(env=env), patch.dict(os.environ, env, clear=True), \
                    patch.object(pathlib.Path, "write_text") as write:
                with self.assertRaises(error):
                    MODULE.write_ci_report_if_requested([], [], "passed", report_path="unused.json",
                                                        summarize=Mock(), verdict_metrics=("execution",))
                write.assert_not_called()


class ProducerReportProperties(unittest.TestCase):
    @given(inputs=E2E_INPUTS, failures=FAILURES, run_id=POSITIVE_ID, pr_number=POSITIVE_ID,
           head_sha=SHA_STRATEGY, status=st.sampled_from(("passed", "failed")),
           verdict_metrics=st.sampled_from((("execution",), ("inner_block_executor",),
                                           ("execution", "inner_block_executor"))))
    def test_e2e_projection_writer_and_consumer_share_binding(
        self, inputs, failures, run_id, pr_number, head_sha, status, verdict_metrics
    ):
        results = [result_from(item) for item in inputs]
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
        expected_calls = [(runs, "execution") for result in results for runs in (result.v1_runs, result.mono_runs)]
        with tempfile.TemporaryDirectory() as directory:
            path = pathlib.Path(directory) / "report.json"
            with patch.dict(os.environ, {"PR_CI_RUN_ID": str(run_id), "PR_CI_PR_NUMBER": str(pr_number),
                                        "PR_CI_HEAD_SHA": head_sha}, clear=True):
                MODULE.write_ci_report_if_requested(results, failures, status, report_path=path,
                                                    summarize=summarize, verdict_metrics=verdict_metrics)
            payload = path.read_bytes()
        self.assertEqual(expected_calls, [call.args for call in summarize.call_args_list])
        self.assertTrue(payload.endswith(b"\n"))
        self.assertNotIn(b"\n", payload[:-1])
        self.assertEqual(expected, parse_and_validate_report(payload, binding))
        changed_reasons = [(name, "@team [link](unsafe)\n" + reason) for name, reason in failures]
        self.assertEqual(expected, MODULE.build_pr_ci_report(
            results, changed_reasons, run_id=run_id, pr_number=pr_number, head_sha=head_sha, status=status,
            summarize=lambda runs, metric: (runs[0].tps, None), verdict_metrics=verdict_metrics))


if __name__ == "__main__":
    unittest.main()
