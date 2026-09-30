import importlib.util
import io
import json
import math
import pathlib
import shutil
import subprocess
import sys
import tempfile
import unittest
from contextlib import redirect_stdout
from unittest.mock import patch

from hypothesis import example, given, strategies as st

MODULE_PATH = pathlib.Path(__file__).with_name("compare.py")
SPEC = importlib.util.spec_from_file_location("compare", MODULE_PATH)
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)

# Tests may import the trusted validator; compare.py itself must not.
sys.path.insert(0, str(MODULE_PATH.parents[6] / ".github" / "ci"))
from ci_actions.report_schema import parse_and_validate_report  # noqa: E402
from tests.property_support import POSITIVE_ID, SAFE_COMPONENT, SHA as SHA_STRATEGY, configure_profiles  # noqa: E402

configure_profiles()

MICRO_RESULTS = st.lists(st.fixed_dictionaries({
    "id": SAFE_COMPONENT.map(lambda name: "a" + name),
    "verdict": st.sampled_from(("regression", "improvement", "ok", "notable", "noise", "new", "absent",
                               "workload changed", "incomplete")),
    "delta": st.one_of(st.none(), st.floats(min_value=-1e4, max_value=1e4, allow_nan=False, allow_infinity=False)),
    "base_median_ns": st.one_of(st.none(), st.integers(min_value=0, max_value=10**18)),
    "head_median_ns": st.one_of(st.none(), st.integers(min_value=0, max_value=10**18)),
    "base_processes": st.integers(min_value=0, max_value=100),
    "head_processes": st.integers(min_value=0, max_value=100),
}), max_size=20)

SHA = "0123456789abcdef0123456789abcdef01234567"
BINDING = {"producer": "mono-move-micro-bench", "run_id": 1234, "pr_number": 99, "head_sha": SHA}


class ProducerReportTests(unittest.TestCase):
    def result(self):
        return {
            "id": "fib/mono",
            "verdict": MODULE.REGRESSION,
            "delta": 0.042,
            "base_median_ns": 1000.0,
            "head_median_ns": 1042.0,
        }

    def build(self, results, **overrides):
        arguments = {"pr_number": 99, "head_sha": SHA, "run_id": 1234, "status": "failed", **overrides}
        return MODULE.build_pr_ci_report(results, **arguments)

    def test_writer_rejects_non_finite_numbers(self):
        result = self.result()
        result["delta"] = float("inf")
        with tempfile.TemporaryDirectory() as directory:
            with self.assertRaises(ValueError):
                MODULE.write_pr_ci_report(pathlib.Path(directory) / "report.json", self.build([result]))

    def test_actual_output_is_deterministic_and_accepted_by_trusted_validator(self):
        with tempfile.TemporaryDirectory() as directory:
            path = pathlib.Path(directory) / "report.json"
            MODULE.write_pr_ci_report(path, self.build([self.result()]))
            payload = path.read_bytes()
        self.assertTrue(payload.endswith(b"\n"))
        self.assertNotIn(b"\n", payload[:-1])
        self.assertEqual(parse_and_validate_report(payload, BINDING), json.loads(payload))

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


class ProducerReportProperties(unittest.TestCase):
    @example(
        results=[{"id": "fib/mono", "verdict": "regression", "delta": 0.042,
                  "base_median_ns": 1000.0, "head_median_ns": 1042.0,
                  "base_processes": 1, "head_processes": 1}],
        run_id=1234, pr_number=99, head_sha=SHA, status="passed",
    )
    @given(results=MICRO_RESULTS, run_id=POSITIVE_ID, pr_number=POSITIVE_ID, head_sha=SHA_STRATEGY,
           status=st.sampled_from(("passed", "failed")))
    def test_micro_projection_and_json_lines_agree_with_independent_percent(
        self, results, run_id, pr_number, head_sha, status
    ):
        results = [dict(result) for result in results]
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
        output = io.StringIO()
        with redirect_stdout(output):
            MODULE.emit_json_lines(results, {"threshold_percent": 3})
        lines = [json.loads(line) for line in output.getvalue().splitlines()]
        self.assertEqual(len(results), len(lines))
        for result, line, row in zip(results, lines, expected_rows):
            self.assertEqual(result["id"], line["id"])
            self.assertEqual(row["mean_percent"], line["delta_pct"])
            self.assertEqual(result["base_median_ns"], line["base_median_ns"])
            self.assertEqual(result["head_median_ns"], line["head_median_ns"])
        with patch.object(pathlib.Path, "write_text", autospec=True) as write:
            MODULE.write_pr_ci_report("unused.json", actual)
            text = write.call_args.args[1]
            self.assertEqual("utf-8", write.call_args.kwargs["encoding"])
            MODULE.write_pr_ci_report("unused.json", dict(reversed(list(actual.items()))))
            self.assertEqual(text, write.call_args.args[1])
        self.assertTrue(text.endswith("\n"))
        self.assertNotIn("\n", text[:-1])
        self.assertEqual(json.dumps(expected, allow_nan=False, separators=(",", ":"), sort_keys=True) + "\n", text)
        self.assertEqual(sorted(actual), list(json.loads(text)))
        self.assertEqual(expected, parse_and_validate_report(text.encode(), binding))
        for field, value in {"run_id": run_id + 1, "pr_number": pr_number + 1,
                             "head_sha": ("1" if head_sha[0] == "0" else "0") + head_sha[1:],
                             "producer": "mono-move-e2e-perf"}.items():
            with self.assertRaisesRegex(ValueError, "trusted workflow run metadata"):
                parse_and_validate_report(text.encode(), {**binding, field: value})

    def test_each_non_finite_writer_value_rejects_before_write(self):
        for field in ("mean_percent", "ci_low_percent", "ci_high_percent", "base_median_ns", "pr_median_ns"):
            for value in (math.nan, math.inf, -math.inf):
                report = MODULE.build_pr_ci_report([ProducerReportTests().result()], run_id=1234,
                                                  pr_number=99, head_sha=SHA, status="failed")
                report["metrics"][0][field] = value
                with self.subTest(field=field, value=value), patch.object(pathlib.Path, "write_text") as write:
                    with self.assertRaises(ValueError):
                        MODULE.write_pr_ci_report("unused.json", report)
                    write.assert_not_called()

    def test_writer_serializes_finite_invalid_envelopes_but_consumer_rejects(self):
        report = MODULE.build_pr_ci_report([], run_id=1234, pr_number=99, head_sha=SHA, status="passed")
        for field, value in (("status", "unknown"), ("run_id", True), ("head_sha", "bad"), ("schema", "unknown")):
            with self.subTest(field=field), patch.object(pathlib.Path, "write_text", autospec=True) as write:
                MODULE.write_pr_ci_report("unused.json", {**report, field: value})
                with self.assertRaises(ValueError):
                    parse_and_validate_report(write.call_args.args[1].encode(), BINDING)

    def test_adapter_numeric_endpoints_and_consumer_row_limit(self):
        result = ProducerReportTests().result()
        for delta in (-1e4, 0, 1e4, None):
            for median in (0, 10**18, None):
                row = {**result, "delta": delta, "base_median_ns": median, "head_median_ns": median}
                actual = MODULE.build_pr_ci_report([row] * 100, run_id=1234, pr_number=99, head_sha=SHA, status="passed")
                self.assertEqual(actual, parse_and_validate_report(json.dumps(actual).encode(), BINDING))
        actual = MODULE.build_pr_ci_report([result] * 101, run_id=1234, pr_number=99, head_sha=SHA, status="passed")
        with self.assertRaisesRegex(ValueError, "at most 100"):
            parse_and_validate_report(json.dumps(actual).encode(), BINDING)


if __name__ == "__main__":
    unittest.main()
