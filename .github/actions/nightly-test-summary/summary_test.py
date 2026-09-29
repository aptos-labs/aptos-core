# Copyright (c) Aptos Foundation
# Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

from pathlib import Path
import re
import shlex
import unittest
from summary import CANCELLED, GREEN, GREY, RED, YELLOW, build_history, build_summary


class NightlySummaryTest(unittest.TestCase):
    def summary(self, needs, **kwargs):
        return build_summary(
            needs,
            "main",
            "abc123",
            "https://github.com/org/repo/actions/runs/1",
            **kwargs
        )

    def test_nightly_rust_matrix_preserves_legacy_eligibility(self):
        root = Path(__file__).resolve().parents[3]
        legacy = (root / ".github/actions/rust-unit-tests/action.yaml").read_text()
        nightly = (root / ".github/workflows/nightly-full-suite.yaml").read_text()
        workspace = re.search(r"command: (cargo nextest run .*?)\n", nightly).group(1)
        self.assertEqual(
            set(re.findall(r"--exclude (\S+)", workspace)),
            set(re.findall(r"--exclude (\S+)", legacy)),
        )
        # Cargo test suites must already run in legacy CI, too.
        def cargo_test_args(text):
            return {
                frozenset(shlex.split(command))
                for command in re.findall(r"(?:run|command): (cargo test .*?)\n", text)
            }
        self.assertEqual(cargo_test_args(nightly), cargo_test_args(legacy))
        self.assertIn("FORGE_NAMESPACE: forge-nightly-", nightly)

    def test_history_bar_links_each_night_oldest_first_and_ends_with_this_run(self):
        def run(day, conclusion, status="completed"):
            return {
                "createdAt": f"2026-09-{day:02d}T02:00:00Z",
                "conclusion": conclusion,
                "status": status,
                "url": f"https://github.com/org/repo/actions/runs/{day}",
            }

        previous = [run(3, "failure"), run(1, "success"), run(2, "skipped")]
        line = build_history(previous, "https://github.com/org/repo/actions/runs/9", False)
        self.assertEqual(
            line,
            "Last 4 nights: "
            f"<https://github.com/org/repo/actions/runs/1|{GREEN}>"
            f"<https://github.com/org/repo/actions/runs/2|{GREY}>"
            f"<https://github.com/org/repo/actions/runs/3|{RED}>"
            f"<https://github.com/org/repo/actions/runs/9|{GREEN}>",
        )
        # Only the newest six completed nights precede this run; an in-progress
        # run is not a night, and this run's own colour follows its result.
        previous = [run(day, "success") for day in range(1, 10)]
        previous.append(run(11, None, status="in_progress"))
        line = build_history(previous, "https://github.com/org/repo/actions/runs/9", True)
        self.assertEqual(line.count("<"), 7)
        self.assertNotIn("/runs/3|", line)
        self.assertNotIn("/runs/11", line)
        self.assertTrue(line.endswith(f"<https://github.com/org/repo/actions/runs/9|{RED}>"))
        self.assertEqual(
            build_history(None, "https://github.com/org/repo/actions/runs/9", False),
            f"Last 1 nights: <https://github.com/org/repo/actions/runs/9|{GREEN}>",
        )
        # Every nightly message carries the bar.
        _, payload = self.summary({"workspace": {"result": "success"}})
        self.assertIn("Last 1 nights: <https://github.com/org/repo/actions/runs/1|", payload["text"])

    def test_passing_only_on_retry_is_yellow_and_names_recovered_jobs(self):
        previous = [
            {
                "createdAt": "2026-09-01T02:00:00Z",
                "status": "completed",
                "conclusion": "success",
                "attempt": 2,
                "url": "https://github.com/org/repo/actions/runs/1",
            }
        ]
        failed, payload = self.summary(
            {"workspace": {"result": "success"}},
            previous_runs=previous,
            attempt=2,
            jobs=[{"name": "flaky", "conclusion": "success"}],
            first_attempt_jobs=[
                {"name": "flaky", "conclusion": "failure"},
                {"name": "steady", "conclusion": "success"},
            ],
        )
        self.assertFalse(failed)
        self.assertIn("Nightly full-suite passed after retry", payload["text"])
        self.assertIn(
            f"<https://github.com/org/repo/actions/runs/1|{YELLOW}>"
            f"<https://github.com/org/repo/actions/runs/1|{YELLOW}>",
            payload["text"],
        )
        self.assertIn("Passed on retry: flaky\n", payload["text"] + "\n")
        # Failing again after the retry stays red.
        failed, payload = self.summary(
            {"workspace": {"result": "failure"}},
            attempt=2,
            jobs=[{"name": "broken", "conclusion": "failure", "steps": []}],
            first_attempt_jobs=[{"name": "broken", "conclusion": "failure"}],
        )
        self.assertTrue(failed)
        self.assertIn(f"|{RED}>", payload["text"])
        self.assertNotIn("Passed on retry", payload["text"])

    def test_failed_first_attempt_retries_instead_of_posting(self):
        root = Path(__file__).resolve().parents[3]
        nightly = (root / ".github/workflows/nightly-full-suite.yaml").read_text()
        retry = re.search(r"^  retry:\n(.*?)(?=^  [a-z-]+:$)", nightly, re.M | re.S).group(1)
        notify = re.search(r"^  notify:\n(.*?)(?=^  [a-z-]+:$|\Z)", nightly, re.M | re.S).group(1)
        self.assertIn(
            "!cancelled() && needs.result.result != 'success' && github.run_attempt == '1'",
            retry,
        )
        result = re.search(r"^  result:\n(.*?)(?=^  [a-z-]+:$)", nightly, re.M | re.S).group(1)
        self.assertIn("if: cancelled()\n        run: echo RUN_CANCELLED=true", result)
        self.assertIn("gh workflow run nightly-full-suite-retry.yaml", retry)
        self.assertIn("needs.retry.result != 'success'", notify)
        self.assertIn("errors: true", notify)
        rerun = (root / ".github/workflows/nightly-full-suite-retry.yaml").read_text()
        self.assertIn(".github/workflows/nightly-full-suite.yaml", rerun)
        self.assertIn('--jq .run_attempt)" = 1', rerun)
        self.assertIn("--failed", rerun)

    def test_cancelled_runs_are_crossed_out_but_timeouts_stay_red(self):
        def night(day, conclusion, job_conclusion):
            return {
                "createdAt": f"2026-09-{day:02d}T02:00:00Z",
                "status": "completed",
                "conclusion": conclusion,
                "url": f"https://github.com/org/repo/actions/runs/{day}",
                "jobs": [{"name": "forge", "conclusion": job_conclusion}],
            }

        previous = [night(1, "cancelled", "cancelled"), night(2, "failure", "cancelled")]
        _, payload = self.summary(
            {"forge": {"result": "failure"}},
            previous_runs=previous,
            jobs=[{"name": "forge", "conclusion": "cancelled", "steps": []}],
        )
        self.assertIn(
            f"<https://github.com/org/repo/actions/runs/1|{CANCELLED}>"
            f"<https://github.com/org/repo/actions/runs/2|{RED}>",
            payload["text"],
        )
        # The cancelled night crosses out; the timed-out job of a failed night is red.
        self.assertIn(f"{CANCELLED}{RED}{RED}  forge", payload["text"])
        _, payload = self.summary(
            {"forge": {"result": "cancelled"}},
            previous_runs=previous,
            jobs=[{"name": "forge", "conclusion": "cancelled", "steps": []}],
            cancelled=True,
        )
        self.assertIn("Nightly full-suite CANCELLED", payload["text"])
        self.assertIn(f"|{CANCELLED}>\nBranch:", payload["text"])
        self.assertIn(f"{CANCELLED}{RED}{CANCELLED}  forge", payload["text"])

    def test_green_run_is_not_failed(self):
        failed, _ = self.summary({"workspace": {"result": "success"}})
        self.assertFalse(failed)

    def test_failure_outside_move_alerts_with_context(self):
        failed, payload = self.summary(
            {"storage": {"result": "failure"}},
            jobs=[
                {
                    "name": "cargo (workspace)",
                    "conclusion": "failure",
                    "steps": [{"name": "Run tests", "conclusion": "failure"}],
                }
            ],
            previous_sha="base123",
        )
        self.assertTrue(failed)
        self.assertIn(f"{RED}  cargo (workspace) \u2014 Run tests", payload["text"])
        self.assertIn("base123...abc123", payload["text"])
        # Without job details the incomplete suites are named instead.
        _, payload = self.summary({"storage": {"result": "failure"}})
        self.assertIn("Required suites: storage: failure", payload["text"])

    def test_failed_jobs_show_their_seven_night_history(self):
        def night(day, jobs, first_attempt_jobs=None):
            return {
                "createdAt": f"2026-09-{day:02d}T02:00:00Z",
                "status": "completed",
                "conclusion": "success",
                "attempt": 2 if first_attempt_jobs else 1,
                "url": f"https://github.com/org/repo/actions/runs/{day}",
                "jobs": jobs,
                "first_attempt_jobs": first_attempt_jobs,
            }

        passed = [{"name": "parity", "conclusion": "success"}]
        previous = [night(day, passed) for day in range(1, 5)]
        previous.append(night(5, passed, [{"name": "parity", "conclusion": "failure"}]))
        previous.append(night(6, [{"name": "parity", "conclusion": "failure"}]))
        previous.append(night(7, []))
        _, payload = self.summary(
            {"mono-move-parity": {"result": "failure"}, "cli-e2e": {"result": "skipped"}},
            previous_runs=previous,
            jobs=[
                {
                    "name": "parity",
                    "conclusion": "failure",
                    "html_url": "https://github.com/org/repo/actions/runs/1/job/9",
                    "steps": [{"name": "Check parity", "conclusion": "failure"}],
                },
                {"name": "cli", "conclusion": "skipped"},
            ],
        )
        # Six prior nights (the oldest drops out), then tonight linked to the job log.
        self.assertIn(
            f"{GREEN * 3}{YELLOW}{RED}{GREY}"
            f"<https://github.com/org/repo/actions/runs/1/job/9|{RED}>"
            "  parity \u2014 Check parity",
            payload["text"],
        )
        self.assertIn("Skipped suites: cli-e2e", payload["text"])
        self.assertNotIn("Required suites", payload["text"])

    def test_skipped_jobs_are_omitted_from_failure_details(self):
        _, payload = self.summary(
            {"storage": {"result": "failure"}},
            jobs=[
                {"name": "failed", "conclusion": "failure", "steps": []},
                {"name": "expected skip", "conclusion": "skipped", "steps": []},
            ],
        )
        self.assertIn(f"{RED}  failed", payload["text"])
        self.assertNotIn("expected skip", payload["text"])

    def test_skips_timeouts_cancellations_and_missing_results_are_not_green(self):
        for result in ("skipped", "cancelled", "failure", "timed_out"):
            with self.subTest(result=result):
                self.assertTrue(self.summary({"setup": {"result": result}})[0])
        self.assertTrue(self.summary({})[0])

    def test_enrichment_is_optional_and_dynamic_text_is_escaped(self):
        failed, payload = build_summary(
            {"cargo": {"result": "failure"}},
            "feature/<@everyone>'quoted'",
            "sha",
            "https://github.com/org/repo/actions/runs/1",
        )
        self.assertTrue(failed)
        self.assertNotIn("<@everyone>", payload["text"])
        self.assertIn("'quoted'", payload["text"])
        self.assertIn("actions/runs/1", payload["text"])


if __name__ == "__main__":
    unittest.main()
