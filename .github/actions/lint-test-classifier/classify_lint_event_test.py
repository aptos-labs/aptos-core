import unittest

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
RELEASE_BASES = (
    "devnet",
    "testnet",
    "mainnet",
    "aptos-release-v1.49",
    "aptos-node-v1.50.0-rc",
)


class ClassifyLintEventTest(unittest.TestCase):
    def test_release_destinations_run_full_suite(self):
        for base in RELEASE_BASES:
            self.assertEqual(
                classify_event("pull_request", base, "1/merge"), FULL_SUITE, base
            )

    def test_development_prs_keep_freshness_and_targeted_tests(self):
        for base in ("main", "my-release-work"):
            self.assertEqual(
                classify_event("pull_request", base, "1/merge"), DEVELOPMENT_PR, base
            )

    def test_push_and_dispatch_run_full_suite(self):
        for event in ("push", "workflow_dispatch"):
            self.assertEqual(classify_event(event, "", "feature"), FULL_SUITE, event)

    def test_invalid_inputs_fail_closed(self):
        with self.assertRaises(ValueError):
            classify_event("schedule", "main", "main")
        with self.assertRaises(ValueError):
            classify_event("pull_request", "", "1/merge")


if __name__ == "__main__":
    unittest.main()
