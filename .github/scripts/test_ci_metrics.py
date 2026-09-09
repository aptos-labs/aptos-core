import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from ci_metrics import (
    cache_error_counts,
    cgroup_peak,
    check_partitions,
    execution_mode,
    host_memory_used,
    inventory,
    measure,
    run_report,
)
from targeted_tests import package_args


class MetricsTests(unittest.TestCase):
    def test_cache_diagnostics_export_counts_without_log_contents(self):
        counts = cache_error_counts(
            "storage write check failed: RateLimited url=SECRET\nGot unexpected error: 429\n"
        )
        self.assertEqual(counts["rate_limited_lines"], 2)
        self.assertEqual(counts["write_probe_failure_lines"], 1)
        self.assertEqual(counts["backend_read_error_lines"], 1)
        self.assertNotIn("SECRET", json.dumps(counts))

    def test_report_retains_attempts_without_double_counting_reused_jobs(self):
        def job(identifier, name=None, start="2026-09-09T00:00:00Z"):
            return {
                "id": identifier,
                "name": name or str(identifier),
                "started_at": start,
                "completed_at": "2026-09-09T00:01:00Z",
                "runner_id": 1,
                "conclusion": "success",
                "labels": [
                    "2cpu-gh-ubuntu24-x64" if identifier == 3 else "runs-on,cpu=32"
                ],
            }

        with patch(
            "ci_metrics.gh_json",
            side_effect=[
                {"run_attempt": 2},
                {"jobs": [job(1), job(2)]},
                {"jobs": [job(2), job(20, name="2"), job(3)]},
                {},
                {},
            ],
        ):
            report = run_report("owner/repo", 1)
        self.assertEqual(
            [job["measurement_run_attempt"] for job in report["jobs"]], [1, 1, 2]
        )
        self.assertEqual(
            sum(job["allocated_vcpu_minutes"] for job in report["jobs"]), 66
        )
        self.assertEqual(report["jobs"][1]["reused_job_ids"], [20])

    def test_report_counts_a_real_rerun_of_the_same_job(self):
        job = {
            "id": 1,
            "name": "test",
            "runner_id": 1,
            "conclusion": "failure",
            "started_at": "2026-09-09T00:00:00Z",
            "completed_at": "2026-09-09T00:01:00Z",
            "labels": ["cpu=32"],
        }
        retry = {
            **job,
            "id": 2,
            "runner_id": 2,
            "conclusion": "success",
            "started_at": "2026-09-09T00:02:00Z",
            "completed_at": "2026-09-09T00:03:00Z",
        }
        with patch(
            "ci_metrics.gh_json",
            side_effect=[
                {"run_attempt": 2},
                {"jobs": [job]},
                {"jobs": [retry]},
                {},
                {},
            ],
        ):
            report = run_report("owner/repo", 1)
        self.assertEqual(sum(j["allocated_vcpu_minutes"] for j in report["jobs"]), 64)

    def test_report_does_not_bill_skipped_jobs_with_reversed_timestamps(self):
        job = {
            "id": 1,
            "name": "skipped",
            "runner_id": 0,
            "conclusion": "skipped",
            "started_at": "2026-09-09T01:00:00Z",
            "completed_at": "2026-09-09T00:00:00Z",
            "labels": ["cpu=64"],
        }
        with patch(
            "ci_metrics.gh_json",
            side_effect=[
                {"run_attempt": 1},
                {"jobs": [job]},
                {},
                {},
            ],
        ):
            report = run_report("owner/repo", 1)
        self.assertEqual(report["jobs"][0]["runner_minutes"], 0)
        self.assertNotIn("allocated_vcpu_minutes", report["jobs"][0])

    def test_host_memory_sample(self):
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "meminfo"
            self.assertIsNone(host_memory_used(path))
            path.write_text(
                "MemTotal: 1000 kB\nMemFree: 100 kB\nMemAvailable: 600 kB\n"
            )
            self.assertEqual(host_memory_used(path), 400 * 1024)
            path.write_text("MemTotal: 1000 kB\n")
            self.assertIsNone(host_memory_used(path))

    def test_cgroup_peak_versions_and_missing_counter(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            self.assertEqual(cgroup_peak(root), {})
            legacy = root / "memory" / "memory.max_usage_in_bytes"
            legacy.parent.mkdir()
            legacy.write_text("123\n")
            self.assertEqual(
                cgroup_peak(root),
                {
                    "cgroup_peak_bytes": 123,
                    "cgroup_peak_source": str(legacy),
                },
            )
            current = root / "memory.peak"
            current.write_text("456\n")
            self.assertEqual(cgroup_peak(root)["cgroup_peak_bytes"], 456)
            current.write_text("max\n")
            self.assertEqual(cgroup_peak(root)["cgroup_peak_bytes"], 123)

    def test_benchmark_selection_uses_names_not_versions(self):
        metadata = {
            "packages": [{"name": "test", "manifest_path": "/repo/crate/Cargo.toml"}]
        }
        self.assertEqual(
            package_args(["test"], metadata), ["-p", "file:///repo/crate#test"]
        )
        with self.assertRaises(KeyError):
            package_args(["missing"], metadata)

    def test_threshold_boundaries(self):
        for count, expected in [
            (0, "inline"),
            (2341, "inline"),
            (2999, "inline"),
            (3000, "inline"),
            (3001, "shard"),
        ]:
            self.assertEqual(execution_mode(count, 3000), expected)
        for count in [-1, "1000", None, True]:
            with self.assertRaises(ValueError):
                execution_mode(count, 3000)

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
        self.assertEqual(
            inventory(listing), [("one", "same", False), ("two", "same", True)]
        )

    def test_measurement_preserves_failures(self):
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            self.assertEqual(
                measure(
                    "failure", [sys.executable, "-c", "raise SystemExit(7)"], directory
                ),
                7,
            )
            self.assertEqual(
                measure("missing", [str(directory / "missing")], directory), 127
            )
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
