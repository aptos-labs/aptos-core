import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

from classify_lint_event import classify_event


DEVELOPMENT_PR = {
    "run_freshness": True,
    "run_targeted_tests": True,
    "run_full_tests": False,
}
FULL_SUITE = {
    "run_freshness": False,
    "run_targeted_tests": False,
    "run_full_tests": True,
}
RELEASE_BRANCHES = (
    "devnet",
    "testnet",
    "mainnet",
    "aptos-release-v1.49",
    "aptos-release-v1.50",
    "aptos-node-v1.49.6-hotfix",
    "aptos-node-v1.50.0-rc",
)


class ClassifyLintEventTest(unittest.TestCase):
    def test_release_destination_prs_run_full_suite(self):
        for branch in RELEASE_BRANCHES:
            with self.subTest(branch=branch):
                self.assertEqual(
                    classify_event("pull_request", branch, "123/merge"), FULL_SUITE
                )

    def test_development_prs_keep_freshness_and_targeted_tests(self):
        for branch in ("main", "feature", "my-release-work", "devnet-fix"):
            with self.subTest(branch=branch):
                self.assertEqual(
                    classify_event("pull_request", branch, "123/merge"), DEVELOPMENT_PR
                )

    def test_pr_destination_takes_precedence_over_ref_name(self):
        for ref_name in RELEASE_BRANCHES:
            with self.subTest(ref_name=ref_name):
                self.assertEqual(
                    classify_event("pull_request", "main", ref_name), DEVELOPMENT_PR
                )

    def test_push_and_dispatch_always_run_full_suite(self):
        for event in ("push", "workflow_dispatch"):
            for branch in (*RELEASE_BRANCHES, "main", "feature", "devnet-fix"):
                for base_ref in ("", "main", "devnet"):
                    with self.subTest(event=event, branch=branch, base_ref=base_ref):
                        self.assertEqual(
                            classify_event(event, base_ref, branch), FULL_SUITE
                        )

    def test_unsupported_events_fail(self):
        for event in ("", "schedule", "pull_request_target", "merge_group", "unknown"):
            with self.subTest(event=event):
                with self.assertRaisesRegex(ValueError, "Unsupported"):
                    classify_event(event, "main", "main")

    def test_missing_branch_fails_instead_of_using_other_ref(self):
        for event, base_ref, ref_name in (
            ("pull_request", "", "123/merge"),
            ("push", "main", ""),
            ("workflow_dispatch", "main", ""),
        ):
            with self.subTest(event=event):
                with self.assertRaisesRegex(ValueError, "Missing branch"):
                    classify_event(event, base_ref, ref_name)

    def run_script(self, output_path, event, base_ref, ref_name):
        return subprocess.run(
            [sys.executable, str(Path(__file__).with_name("classify_lint_event.py"))],
            env={
                "CI_EVENT_NAME": event,
                "CI_BASE_REF": base_ref,
                "CI_REF_NAME": ref_name,
                "GITHUB_OUTPUT": str(output_path),
            },
            capture_output=True,
            text=True,
            check=False,
        )

    def test_cli_appends_github_boolean_outputs(self):
        for event, base_ref, ref_name, expected in (
            ("pull_request", "main", "123/merge", DEVELOPMENT_PR),
            ("pull_request", "devnet", "123/merge", FULL_SUITE),
            ("push", "", "devnet", FULL_SUITE),
            ("workflow_dispatch", "", "aptos-release-v1.49", FULL_SUITE),
        ):
            with (
                self.subTest(event=event, base_ref=base_ref),
                tempfile.TemporaryDirectory() as tmp,
            ):
                output_path = Path(tmp) / "output"
                output_path.write_text("existing=value\n")
                result = self.run_script(output_path, event, base_ref, ref_name)
                self.assertEqual(result.returncode, 0, result.stderr)
                output = "".join(
                    f"{key}={str(value).lower()}\n" for key, value in expected.items()
                )
                self.assertEqual(output_path.read_text(), "existing=value\n" + output)
                self.assertEqual(result.stdout, output)

    def test_invalid_cli_input_fails_without_writing_outputs(self):
        for event, base_ref, ref_name in (
            ("unknown", "main", "main"),
            ("pull_request", "", "123/merge"),
        ):
            with self.subTest(event=event), tempfile.TemporaryDirectory() as tmp:
                output_path = Path(tmp) / "output"
                result = self.run_script(output_path, event, base_ref, ref_name)
                self.assertNotEqual(result.returncode, 0)
                self.assertFalse(output_path.exists())
                self.assertEqual(result.stdout, "")


if __name__ == "__main__":
    unittest.main()
