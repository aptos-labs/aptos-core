import importlib.util
import math
import pathlib
import shutil
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

from hypothesis import example, given, strategies as st

MODULE_PATH = pathlib.Path(__file__).with_name("compare.py")
SPEC = importlib.util.spec_from_file_location("compare", MODULE_PATH)
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)

# Tests may import the trusted validator; compare.py itself must not.
sys.path.insert(0, str(MODULE_PATH.parents[6] / ".github" / "ci"))
from ci_actions.report_schema import parse_and_validate_report  # noqa: E402
from tests.helpers import SHA  # noqa: E402
from tests.property_support import POSITIVE_ID, SAFE_COMPONENT, SHA as SHA_STRATEGY, configure_profiles  # noqa: E402
from tests.report_support import CONTRACTS  # noqa: E402

configure_profiles()

MICRO_RESULTS = st.lists(st.fixed_dictionaries({
    "id": SAFE_COMPONENT.map(lambda name: "a" + name),
    "verdict": st.sampled_from(CONTRACTS["mono-move-micro-bench"]["verdicts"]),
    "delta": st.one_of(st.none(), st.floats(min_value=-1e4, max_value=1e4, allow_nan=False, allow_infinity=False)),
    "base_median_ns": st.one_of(st.none(), st.integers(min_value=0, max_value=10**18)),
    "head_median_ns": st.one_of(st.none(), st.integers(min_value=0, max_value=10**18)),
    "base_processes": st.integers(min_value=0, max_value=100),
    "head_processes": st.integers(min_value=0, max_value=100),
}), max_size=20)

RESULT = {"id": "fib/mono", "verdict": "regression", "delta": 0.042, "base_median_ns": 1000.0, "head_median_ns": 1042.0}


class ProducerReportTests(unittest.TestCase):
    def test_fixed_result_projects_to_a_hand_computed_row(self):
        report = MODULE.build_pr_ci_report([RESULT], run_id=1234, pr_number=99, head_sha=SHA, status="failed")
        self.assertEqual([{"name": "fib/mono", "verdict": "regression", "mean_percent": 4.2, "ci_low_percent": None,
                           "ci_high_percent": None, "base_median_ns": 1000.0, "pr_median_ns": 1042.0}],
                         report["metrics"])

    def test_compare_imports_the_writer_in_repository_and_staged_layouts(self):
        helper_path = MODULE_PATH.parents[2] / "pr_ci_report.py"
        with tempfile.TemporaryDirectory() as directory:
            staged = pathlib.Path(directory)
            shutil.copy2(MODULE_PATH, staged / "compare.py")
            shutil.copy2(helper_path, staged / "pr_ci_report.py")
            for script in (MODULE_PATH, staged / "compare.py"):
                result = subprocess.run(
                    [sys.executable, "-I", str(script), "--help"],
                    cwd=staged, capture_output=True, text=True, check=False,
                )
                with self.subTest(script=str(script)):
                    self.assertEqual(result.returncode, 0, result.stderr)

    def test_non_finite_writer_value_rejects_before_write(self):
        for value in (math.nan, math.inf):
            report = MODULE.build_pr_ci_report([{**RESULT, "delta": value}], run_id=1234, pr_number=99,
                                              head_sha=SHA, status="failed")
            with self.subTest(value=value), patch.object(pathlib.Path, "write_text") as write:
                with self.assertRaises(ValueError):
                    MODULE.write_pr_ci_report("unused.json", report)
                write.assert_not_called()


class ProducerReportProperties(unittest.TestCase):
    @example(results=[{**RESULT, "base_processes": 1, "head_processes": 1}],
             run_id=1234, pr_number=99, head_sha=SHA, status="passed")
    @given(results=MICRO_RESULTS, run_id=POSITIVE_ID, pr_number=POSITIVE_ID, head_sha=SHA_STRATEGY,
           status=st.sampled_from(("passed", "failed")))
    def test_micro_projection_is_written_and_accepted_by_the_trusted_validator(
        self, results, run_id, pr_number, head_sha, status
    ):
        expected_rows = [{
            "name": result["id"], "verdict": result["verdict"],
            "mean_percent": None if result["delta"] is None else result["delta"] * 100.0,
            "ci_low_percent": None, "ci_high_percent": None,
            "base_median_ns": result["base_median_ns"], "pr_median_ns": result["head_median_ns"],
        } for result in results]
        binding = {"producer": "mono-move-micro-bench", "run_id": run_id,
                   "pr_number": pr_number, "head_sha": head_sha}
        expected = {**binding, "schema": "pr-ci-report-v1", "status": status, "metrics": expected_rows}
        actual = MODULE.build_pr_ci_report(results, run_id=run_id, pr_number=pr_number,
                                          head_sha=head_sha, status=status)
        self.assertEqual(expected, actual)
        with tempfile.TemporaryDirectory() as directory:
            path = pathlib.Path(directory) / "report.json"
            MODULE.write_pr_ci_report(path, actual)
            payload = path.read_bytes()
        self.assertTrue(payload.endswith(b"\n"))
        self.assertNotIn(b"\n", payload[:-1])
        self.assertEqual(expected, parse_and_validate_report(payload, binding))


if __name__ == "__main__":
    unittest.main()
