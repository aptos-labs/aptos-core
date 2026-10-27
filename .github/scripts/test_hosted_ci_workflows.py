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
        self.assertEqual(len(shards["strategy"]["matrix"]["include"]), 4)
        self.assertEqual(shards["strategy"]["max-parallel"], "4")
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
            {"vm-and-doc-tests", "unit-test-shards"},
        )
        checks = "\n".join(
            step.get("run", "") for step in jobs["vm-and-doc-tests"]["steps"]
        )
        self.assertIn("--features check-vm-features -p aptos-node", checks)
        self.assertIn("--doc --workspace", checks)
        run = "\n".join(
            step.get("run", "") for step in jobs["unit-test-shards"]["steps"]
        )
        for package in [
            "smoke-test",
            "aptos-testcases",
            "aptos-keyless-circuit",
            "aptos-batch-encryption",
        ]:
            self.assertIn(f"--exclude {package}", run)
        groups = jobs["unit-test-shards"]["strategy"]["matrix"]["include"]
        import shlex

        assigned = []
        remaining = []
        for group in groups:
            args = shlex.split(group["packages"])
            if group["group"] == "remaining":
                remaining = [
                    args[i + 1] for i, arg in enumerate(args) if arg == "--exclude"
                ]
                self.assertIn("--workspace", args)
            else:
                assigned.extend(
                    args[i + 1] for i, arg in enumerate(args) if arg == "-p"
                )
        self.assertEqual(len(assigned), len(set(assigned)))
        self.assertEqual(set(assigned), set(remaining))
        self.assertNotIn("--partition", run)
        self.assertIn("rust-targeted-unit-tests", self.parent["jobs"])
        self.assertIn(
            "runs-on,cpu=64", self.parent["jobs"]["rust-targeted-unit-tests"]["runs-on"]
        )

    def test_remaining_jobs_use_provider_with_aws_fallback(self):
        for name in ["rust-targeted-unit-tests", "rust-check-merge-base"]:
            job = self.parent["jobs"][name]
            self.assertEqual(
                set(job["needs"]), {"file_change_determinator", "rust-ci-provider"}
            )
            self.assertIn("64cpu-aptos-image-ubuntu22-x64", job["runs-on"])
            self.assertIn("runs-on,cpu=64", job["runs-on"])
            prebuilt = next(
                s
                for s in job["steps"]
                if s.get("uses") == "./.github/actions/rust-prebuilt-setup"
            )
            self.assertEqual(
                prebuilt["if"],
                "${{ needs.rust-ci-provider.outputs.provider == 'hosted' }}",
            )
        targeted = self.parent["jobs"]["rust-targeted-unit-tests"]
        setup = next(
            s for s in targeted["steps"] if s.get("name") == "Run dev_setup.sh"
        )
        self.assertEqual(
            setup["if"], "${{ needs.rust-ci-provider.outputs.provider == 'aws' }}"
        )
        action = next(
            s
            for s in targeted["steps"]
            if s.get("uses") == "./.github/actions/rust-targeted-unit-tests"
        )
        self.assertEqual(
            action["with"]["BASE_REF"], "origin/${{ github.base_ref || 'main' }}"
        )
        self.assertEqual(
            action["with"]["DETERMINATOR_MODE"],
            "${{ vars.APTOS_TEST_DETERMINATOR || 'legacy' }}",
        )

    def test_rerun_artifacts_and_failure_analytics(self):
        shards = self.reusable["jobs"]["unit-test-shards"]
        upload = next(
            s
            for s in shards["steps"]
            if str(s.get("uses", "")).startswith("actions/upload-artifact@")
        )
        self.assertEqual(upload["with"]["overwrite"], "true")
        self.assertEqual(upload["if"], "${{ always() }}")
        verify = self.reusable["jobs"]["verify-results"]
        download = next(
            s
            for s in verify["steps"]
            if str(s.get("uses", "")).startswith("actions/download-artifact@")
        )
        self.assertEqual(download["if"], "${{ !cancelled() }}")
        gate = next(
            s
            for s in verify["steps"]
            if s.get("name") == "Require every phase to succeed"
        )
        self.assertIn('test "$SHARDS_RESULT" = success', gate["run"])

    def test_phase_a_followup_workflows_route_hosted_with_aws_fallback(self):
        HOSTED = "64cpu-aptos-image-ubuntu22-x64"
        AWS = "runs-on,cpu=64"
        # (workflow, job, expect prebuilt-setup on hosted)
        routed = {
            "nightly-full-suite.yaml": ["cargo-tests", "batch-encryption"],
            "coverage.yaml": ["rust-unit-coverage", "rust-smoke-coverage"],
            "lint-test-gated.yaml": [
                "rust-cryptohasher-domain-separation-check",
                "rust-doc-tests",
                "rust-batch-encryption-tests",
                "rust-all-unit-tests",
                "rust-build-cached-packages",
                "rust-consensus-only-unit-test",
                "rust-consensus-only-smoke-test",
            ],
        }
        for workflow, jobs in routed.items():
            doc = yaml.load(
                (ROOT / ".github/workflows" / workflow).read_text(),
                Loader=yaml.BaseLoader,
            )
            self.assertIn("rust-ci-provider", doc["jobs"], workflow)
            for name in jobs:
                job = doc["jobs"][name]
                # Single needs key, and it must include the provider (else runs-on
                # cannot reference it). Guard against the duplicate-needs regression.
                needs = job["needs"]
                needs = [needs] if isinstance(needs, str) else needs
                self.assertIn(
                    "rust-ci-provider", needs, f"{workflow}:{name} missing provider in needs"
                )
                runs_on = job["runs-on"]
                self.assertIn(HOSTED, runs_on, f"{workflow}:{name}")
                self.assertIn(AWS, runs_on, f"{workflow}:{name}")
                self.assertIn(
                    "needs.rust-ci-provider.outputs.provider == 'hosted'",
                    runs_on,
                    f"{workflow}:{name}",
                )

    def test_nightly_workspace_shards_cover_full_suite(self):
        doc = yaml.load(
            (ROOT / ".github/workflows/nightly-full-suite.yaml").read_text(),
            Loader=yaml.BaseLoader,
        )
        groups = doc["jobs"]["cargo-tests"]["strategy"]["matrix"]["include"]
        commands = {g["suite"]: g["command"] for g in groups}
        # The old single workspace command's exclusions must be preserved across shards.
        import shlex

        assigned = []
        remaining_excludes = []
        for suite, command in commands.items():
            args = shlex.split(command)
            if suite == "workspace-remaining":
                self.assertIn("--workspace", args)
                remaining_excludes = [
                    args[i + 1] for i, a in enumerate(args) if a == "--exclude"
                ]
            elif suite.startswith("workspace-"):
                assigned.extend(args[i + 1] for i, a in enumerate(args) if a == "-p")
        self.assertEqual(len(assigned), len(set(assigned)))
        # Every sharded package is excluded from the remaining shard, and the heavy
        # packages excluded by the original workspace command stay excluded there.
        for package in assigned + [
            "smoke-test",
            "aptos-testcases",
            "aptos-keyless-circuit",
            "aptos-batch-encryption",
        ]:
            self.assertIn(package, remaining_excludes)


if __name__ == "__main__":
    unittest.main()
