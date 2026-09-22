# Copyright (c) Aptos Foundation
# Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

from pathlib import Path
import re
import shlex
import unittest
from summary import build_summary


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

    def test_green_run_does_not_alert(self):
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
        self.assertIn("storage: failure", payload["text"])
        self.assertIn("cargo (workspace): Run tests", payload["text"])
        self.assertIn("base123...abc123", payload["text"])

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
