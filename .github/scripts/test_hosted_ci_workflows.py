import json
import subprocess
import unittest
from pathlib import Path

import yaml

ROOT = Path(__file__).resolve().parents[2]


class WorkflowTests(unittest.TestCase):
    def setUp(self):
        self.parent = yaml.load(
            (ROOT / ".github/workflows/lint-test.yaml").read_text(),
            Loader=yaml.BaseLoader,
        )
        self.reusable = yaml.load(
            (ROOT / ".github/workflows/hosted-rust-unit-tests.yaml").read_text(),
            Loader=yaml.BaseLoader,
        )

    def test_provider_selection(self):
        script = self.parent["jobs"]["rust-ci-provider"]["steps"][0]["run"]
        import os
        import tempfile

        for configured, manual, expected in [
            ("", "default", "hosted"),
            ("aws", "default", "aws"),
            ("hosted", "aws", "aws"),
            ("aws", "hosted", "hosted"),
        ]:
            with tempfile.NamedTemporaryFile() as output:
                env = dict(
                    os.environ,
                    CONFIGURED_PROVIDER=configured,
                    MANUAL_PROVIDER=manual,
                    GITHUB_OUTPUT=output.name,
                )
                subprocess.run(
                    ["sh", "-eu", "-c", script],
                    env=env,
                    check=True,
                    capture_output=True,
                )
                self.assertEqual(
                    Path(output.name).read_text(), f"provider={expected}\n"
                )
        result = subprocess.run(
            ["sh", "-eu", "-c", script],
            env=dict(os.environ, CONFIGURED_PROVIDER="typo", MANUAL_PROVIDER="default"),
            capture_output=True,
        )
        self.assertNotEqual(result.returncode, 0)

    def test_provider_gates_and_shard_cache(self):
        for name, job in self.reusable["jobs"].items():
            self.assertIn(
                (
                    "inputs.provider == 'aws'"
                    if name == "aws-unit-tests"
                    else "inputs.provider == 'hosted'"
                ),
                job["if"],
            )
        shards = self.reusable["jobs"]["unit-test-shards"]
        self.assertEqual(shards["strategy"]["matrix"]["shard"], ["1", "2", "3", "4"])
        self.assertEqual(shards["strategy"]["max-parallel"], "4")
        setup = next(
            step
            for step in shards["steps"]
            if step.get("uses") == "./.github/actions/rust-prebuilt-setup"
        )
        self.assertEqual(setup["with"]["save-cache"], "false")
        self.assertEqual(set(self.reusable["on"]), {"workflow_call"})

    def test_migrated_actions_use_node24_release_pins(self):
        forbidden = [
            "actions/checkout@v4",
            "actions/upload-artifact@v4",
            "actions/download-artifact@v4",
        ]
        for workflow in ["lint-test.yaml", "hosted-rust-unit-tests.yaml"]:
            text = (ROOT / ".github/workflows" / workflow).read_text()
            for action in forbidden:
                self.assertNotIn(action, text)

    def test_full_suite_checks_and_aggregate(self):
        jobs = self.reusable["jobs"]
        self.assertEqual(
            set(jobs["verify-results"]["needs"]),
            {"vm-and-doc-tests", "build-archive", "unit-test-shards"},
        )
        checks = "\n".join(
            step.get("run", "") for step in jobs["vm-and-doc-tests"]["steps"]
        )
        self.assertIn("--features check-vm-features -p aptos-node", checks)
        self.assertIn("--doc --workspace", checks)
        archive = "\n".join(
            step.get("run", "") for step in jobs["build-archive"]["steps"]
        )
        for package in [
            "smoke-test",
            "aptos-testcases",
            "aptos-keyless-circuit",
            "aptos-batch-encryption",
        ]:
            self.assertIn(f"--exclude {package}", archive)
        self.assertIn("rust-targeted-unit-tests", self.parent["jobs"])
        self.assertIn(
            "runs-on,cpu=64", self.parent["jobs"]["rust-targeted-unit-tests"]["runs-on"]
        )


if __name__ == "__main__":
    unittest.main()
