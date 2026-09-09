import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

from ci_metrics import check_partitions, execution_mode, inventory, measure
from targeted_tests import package_args


class MetricsTests(unittest.TestCase):
    def test_benchmark_selection_uses_names_not_versions(self):
        metadata = {"packages": [{"name": "test", "manifest_path": "/repo/crate/Cargo.toml"}]}
        self.assertEqual(package_args(["test"], metadata), ["-p", "file:///repo/crate#test"])
        with self.assertRaises(KeyError):
            package_args(["missing"], metadata)

    def test_threshold_boundaries(self):
        for count, expected in [(0, "inline"), (999, "inline"), (1000, "inline"), (1001, "shard")]:
            self.assertEqual(execution_mode(count, 1000), expected)
        for count in [-1, "1000", None, True]:
            with self.assertRaises(ValueError):
                execution_mode(count, 1000)

    def test_partition_coverage_and_duplicates(self):
        first, second = ("binary", "a", False), ("binary", "b", False)
        check_partitions([first, second], [[first], [], [second]])
        check_partitions([], [[], []])
        for partitions in [
            [[first]],
            [[first], [first, second]],
            [[first, second, ("other", "a", False)]],
        ]:
            with self.assertRaises(ValueError):
                check_partitions([first, second], partitions)

    def test_inventory_keeps_binary_identity_and_ignore_status(self):
        case = {"ignored": False, "filter-match": {"status": "matches"}}
        listing = {
            "rust-suites": {
                "one": {"testcases": {"same": case}},
                "two": {
                    "testcases": {
                        "same": {**case, "ignored": True},
                        "filtered": {**case, "filter-match": {"status": "mismatch"}},
                    }
                },
            }
        }
        self.assertEqual(inventory(listing), [("one", "same", False), ("two", "same", True)])

    def test_measurement_preserves_failures(self):
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            self.assertEqual(
                measure("failure", [sys.executable, "-c", "raise SystemExit(7)"], directory),
                7,
            )
            self.assertEqual(measure("missing", [str(directory / "missing")], directory), 127)
            rows = [
                json.loads(line)
                for line in (directory / "phases.jsonl").read_text().splitlines()
            ]
            self.assertEqual([row["exit_code"] for row in rows], [7, 127])
            self.assertTrue(all(row["elapsed_seconds"] >= 0 for row in rows))

    def test_missing_archive_is_a_failure(self):
        with tempfile.TemporaryDirectory() as temporary:
            result = subprocess.run(
                [
                    sys.executable,
                    str(Path(__file__).with_name("ci_metrics.py")),
                    "plan",
                    str(Path(temporary) / "missing.json"),
                ],
                capture_output=True,
            )
            self.assertNotEqual(result.returncode, 0)


if __name__ == "__main__":
    unittest.main()
