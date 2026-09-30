import importlib.util
import json
import pathlib
import shutil
import subprocess
import sys
import tempfile
import unittest


MODULE_PATH = pathlib.Path(__file__).with_name("compare.py")
SPEC = importlib.util.spec_from_file_location("compare", MODULE_PATH)
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)

# Tests may import the trusted validator; compare.py itself must not.
sys.path.insert(0, str(MODULE_PATH.parents[6] / ".github" / "ci"))
from ci_actions.report_schema import parse_and_validate_report  # noqa: E402

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

    def test_builds_typed_percent_rows(self):
        report = self.build([self.result()])
        self.assertEqual(report["producer"], "mono-move-micro-bench")
        self.assertEqual(report["metrics"], [{
            "name": "fib/mono",
            "verdict": "regression",
            "mean_percent": 4.2,
            "ci_low_percent": None,
            "ci_high_percent": None,
            "base_median_ns": 1000.0,
            "pr_median_ns": 1042.0,
        }])

    def test_json_lines_and_report_use_the_same_percent_conversion(self):
        self.assertEqual(MODULE.as_percent(0.042), 4.2)
        self.assertIsNone(MODULE.as_percent(None))

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


if __name__ == "__main__":
    unittest.main()
