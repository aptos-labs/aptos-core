# Copyright (c) Aptos Foundation
# Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

import contextlib
import io
import json
import os
import re
import shutil
from pathlib import Path
import subprocess
import sys
import tempfile
import tomllib
import unittest
from unittest.mock import patch

from select_tests import main, planner_inputs_changed, REGISTRY, selections


def plan(mode, tests):
    return {
        "schema_version": 1,
        "mode": mode,
        "explicit_packages": False,
        "e2e_tests": {test: ["reason"] for test in tests},
    }


class E2eSelectionTest(unittest.TestCase):
    @staticmethod
    def workflow_job(workflow, job):
        return re.search(
            r"^  " + re.escape(job) + r":\n(.*?)(?=^  [a-z][a-z0-9-]*:|\Z)",
            workflow,
            re.M | re.S,
        ).group(1)

    def test_registry_has_real_workflow_jobs_and_required_nightly_coverage(self):
        root = Path(__file__).resolve().parents[3]
        nightly = (root / ".github/workflows/nightly-full-suite.yaml").read_text()
        nightly_jobs = set(re.findall(r"^  ([a-z][a-z0-9-]*):$", nightly, re.M))
        required = (
            re.search(r"  result:\n.*?    needs: \[(.*?)\]", nightly, re.S)
            .group(1)
            .split(", ")
        )
        for name, runner in REGISTRY.items():
            with self.subTest(name=name):
                workflow = (root / runner["workflow"]).read_text()
                self.assertRegex(
                    workflow, r"(?m)^  " + re.escape(runner["job"]) + r":$"
                )
                self.assertTrue(runner["nightly_jobs"])
                for job in runner["nightly_jobs"]:
                    self.assertIn(job, nightly_jobs)
                    self.assertIn(job, required)
        self.assertNotIn("flow-evaluation", REGISTRY)

    def test_manual_suites_bypass_selection_but_keep_trigger_gates(self):
        root = Path(__file__).resolve().parents[3]
        for filename, label in (
            ("mono-move-tests-parity.yaml", "mono-move-e2e-tests"),
            ("mono-move-e2e-perf.yaml", "mono-move-e2e-perf"),
            ("faucet-tests-prod.yaml", "CICD:non-required-tests"),
        ):
            workflow = (root / ".github/workflows" / filename).read_text()
            with self.subTest(workflow=filename):
                self.assertNotIn("e2e-selection", workflow)
                self.assertIn(label, workflow)
                if filename.startswith("mono-move"):
                    self.assertIn("head.repo.full_name == github.repository", workflow)
                else:
                    self.assertEqual(workflow.count("needs: [permission-check]"), 2)
        workflow = (root / ".github/workflows/docker-build-test.yaml").read_text()
        for job, label in (
            ("forge-framework-upgrade-test", "CICD:run-framework-upgrade-test"),
            ("forge-consensus-only-perf-test", "CICD:run-consensus-only-perf-test"),
            ("forge-multiregion-test", "CICD:run-multiregion-test"),
        ):
            body = re.search(r"^  " + job + r":\n(.*?)(?=^  [a-z][a-z0-9-]*:|\Z)",
                             workflow, re.M | re.S).group(1)
            with self.subTest(job=job):
                self.assertNotIn("e2e-test-determinator", body)
                self.assertNotIn("SELECTION_RESULT", body)
                self.assertIn("- permission-check", body)
                self.assertIn(label, body)
                if job == "forge-framework-upgrade-test":
                    self.assertIn("SKIP_JOB: ${{ !contains(github.event.pull_request.labels.*.name, 'CICD:run-framework-upgrade-test') }}", body)
        self.assertTrue({"mono-move-parity", "mono-move-performance",
                         "forge-framework-upgrade", "forge-consensus-only-performance",
                         "forge-multiregion"}.isdisjoint(REGISTRY))

    def test_full_run_label_reaches_compat_prerequisite(self):
        root = Path(__file__).resolve().parents[3]
        workflow = (root / ".github/workflows/docker-build-test.yaml").read_text()
        fetch = self.workflow_job(workflow, "fetch-last-released-docker-image-tag")
        self.assertIn("CICD:run-all-e2e-tests", fetch)

    def test_explicit_forge_label_forces_legacy_e2e_selection(self):
        root = Path(__file__).resolve().parents[3]
        selection = (root / ".github/workflows/e2e-test-selection.yaml").read_text()
        self.assertIn("CICD:run-forge-e2e-perf", selection)

    def test_selected_reusable_jobs_reach_their_work(self):
        root = Path(__file__).resolve().parents[3]
        faucet = (
            root / ".github/workflows/faucet-tests-main.yaml"
        ).read_text()
        faucet_job = self.workflow_job(faucet, "run-tests-main")
        faucet_gate = faucet_job.split("    runs-on:", 1)[0]
        self.assertIn("inputs.SELECTION_RESULT != 'success'", faucet_gate)
        self.assertIn("!inputs.SKIP_JOB", faucet_gate)

        caller = (root / ".github/workflows/docker-build-test.yaml").read_text()
        faucet_call = self.workflow_job(caller, "faucet-tests-main")
        skip = next(line for line in faucet_call.splitlines() if "SKIP_JOB:" in line)
        self.assertIn("needs.e2e-test-determinator.outputs.mode != 'subsystem'", skip)
        self.assertIn("CICD:non-required-tests", skip)
        self.assertIn("CICD:run-all-e2e-tests", skip)

        performance = (
            root / ".github/workflows/workflow-run-execution-performance.yaml"
        ).read_text()
        performance_job = self.workflow_job(performance, "single-node-performance")
        gate = performance_job.split("    concurrency:", 1)[0]
        self.assertIn("needs.test-target-determinator.result != 'success'", gate)
        self.assertIn(
            "needs.test-target-determinator.outputs.run_execution_performance_test == 'true'",
            gate,
        )
        determinator = self.workflow_job(performance, "test-target-determinator")
        self.assertNotIn("mode != 'subsystem'", determinator)
        self.assertNotIn("mode == 'subsystem'", determinator)

    def test_pr_generated_directories_are_archived_before_artifact_upload(self):
        root = Path(__file__).resolve().parents[3]
        trusted_event_guard = (
            "github.event_name != 'pull_request' && "
            "github.event_name != 'pull_request_target'"
        )
        for filename, step, archive, raw_path in (
            (
                "cli-e2e-tests.yaml",
                "Preserve CLI test output",
                "cli-e2e-output.tar.gz",
                "aptos-e2e-tests-*/out/",
            ),
            (
                "node-api-compatibility-tests.yaml",
                "Preserve generated API specs",
                "api-compatibility-specs.tar.gz",
                "specs/",
            ),
        ):
            workflow = (root / ".github/workflows" / filename).read_text()
            with self.subTest(workflow=filename):
                self.assertIn("tar --create --gzip", workflow)
                self.assertGreaterEqual(workflow.count(trusted_event_guard), 2)
                upload = re.search(
                    r"- name: " + re.escape(step) + r"\n(.*?)(?=\n      - |\Z)",
                    workflow,
                    re.S,
                ).group(1)
                self.assertIn(f"path: ${{{{ runner.temp }}}}/{archive}", upload)
                self.assertNotIn(raw_path, upload)

    def test_checked_in_config_uses_only_registered_names(self):
        root = Path(__file__).resolve().parents[3]
        config = tomllib.loads((root / ".config/test-subsystems.toml").read_text())
        definitions = set(config["e2e_tests"])
        self.assertEqual(definitions, set(REGISTRY))
        for subsystem in config["subsystems"].values():
            self.assertLessEqual(set(subsystem.get("e2e_tests", [])), definitions)

    def test_selected_and_empty_plans(self):
        self.assertEqual(
            selections(plan("subsystem", ["cli-e2e"]), "subsystem"), ["cli-e2e"]
        )
        self.assertEqual(selections(plan("subsystem", []), "subsystem"), [])

    def test_compare_preserves_full_execution_even_on_comparison_error(self):
        value = plan("compare", REGISTRY)
        value["comparison_error"] = "Invalid subsystem config"
        self.assertEqual(selections(value, "compare"), sorted(REGISTRY))
        with self.assertRaises(ValueError):
            selections(plan("compare", ["cli-e2e"]), "compare")

    def test_invalid_plans_fail_instead_of_skipping(self):
        for change in (
            {"schema_version": 2},
            {"mode": "legacy"},
            {"explicit_packages": True},
            {"e2e_tests": None},
            {"e2e_tests": []},
            {"e2e_tests": {"typo": []}},
        ):
            with self.subTest(change=change), self.assertRaises(ValueError):
                selections({**plan("subsystem", []), **change}, "subsystem")

    def run_action(self, mode, cargo_result=None, error=None, changed_paths=""):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            outputs, summary, artifact = (
                root / name for name in ("outputs", "summary", "plan.json")
            )
            with patch.dict(
                os.environ,
                {"GITHUB_OUTPUT": str(outputs), "GITHUB_STEP_SUMMARY": str(summary)},
            ), patch(
                "sys.argv",
                [
                    "select_tests.py",
                    "--mode",
                    mode,
                    "--base",
                    "origin/release",
                    "--plan-file",
                    str(artifact),
                ],
            ), patch(
                "select_tests.subprocess.run",
                return_value=cargo_result,
                side_effect=error,
            ) as cargo, patch(
                "select_tests.subprocess.check_output",
                return_value=changed_paths,
            ) as diff, contextlib.redirect_stdout(
                io.StringIO()
            ):
                if error:
                    with self.assertRaises(subprocess.CalledProcessError):
                        main()
                    self.assertFalse(outputs.exists())
                    self.assertFalse(artifact.exists())
                else:
                    main()
                    recorded = json.loads(artifact.read_text())
                    self.assertIn('"e2e_tests"', summary.read_text())
                    self.assertEqual(
                        outputs.read_text(),
                        "selected_e2e_tests="
                        + json.dumps(selections(recorded, mode))
                        + "\n",
                    )
                if mode == "legacy":
                    diff.assert_not_called()
                    cargo.assert_not_called()
                elif changed_paths:
                    diff.assert_called_once()
                    cargo.assert_not_called()
                else:
                    diff.assert_called_once()
                    self.assertEqual(
                        cargo.call_args.args[0],
                        [
                            "cargo",
                            "x",
                            "--determinator",
                            mode,
                            "--base",
                            "origin/release",
                            "test-plan",
                            "--format",
                            "json",
                        ],
                    )

    def test_legacy_action_does_not_require_cargo_or_config(self):
        self.run_action("legacy")

    def test_legacy_planner_runs_against_workspace_without_new_ci_files(self):
        # Model an older PR checkout: execute the workflow revision's consumer
        # with no action, registry, subsystem config or Cargo in the workspace.
        consumer = Path(__file__).with_name("select_tests.py").resolve()
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            outputs = root / "outputs"
            subprocess.run(
                [
                    sys.executable, str(consumer), "--mode", "legacy", "--base",
                    "origin/main", "--plan-file", str(root / "plan.json"),
                ],
                cwd=root,
                env={
                    **os.environ,
                    "PATH": "",
                    "GITHUB_OUTPUT": str(outputs),
                    "GITHUB_STEP_SUMMARY": str(root / "summary"),
                },
                check=True,
                stdout=subprocess.PIPE,
                text=True,
            )
            selected = json.loads(outputs.read_text().splitlines()[0].split("=", 1)[1])
            self.assertEqual(set(selected), set(REGISTRY))

    def test_action_consumes_plan_and_preserves_base_branch(self):
        self.run_action(
            "subsystem",
            subprocess.CompletedProcess(
                [], 0, json.dumps(plan("subsystem", ["cli-e2e"]))
            ),
        )

    def test_planner_input_changes_select_all_before_parsing(self):
        for path in (
            ".config/test-subsystems.toml",
            ".github/actions/e2e-test-determinator/registry.json",
            "devtools/aptos-cargo-cli/src/test_selection.rs",
        ):
            with self.subTest(path=path):
                self.run_action("subsystem", changed_paths=path + "\n")

    def test_planner_input_matching_is_exact(self):
        with patch(
            "select_tests.subprocess.check_output",
            return_value="docs/devtools/aptos-cargo-cli/readme.md\n",
        ):
            self.assertFalse(planner_inputs_changed("origin/main"))

    def test_prebuilt_planner_bypasses_pr_cargo_alias(self):
        consumer = Path(__file__).with_name("select_tests.py").resolve()
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            planner = root / "trusted-planner"
            planner.write_text(
                f"#!{sys.executable}\nimport json\nprint(json.dumps({plan('subsystem', ['cli-e2e'])!r}))\n"
            )
            planner.chmod(0o755)
            subprocess.run(["git", "init", "-q"], cwd=root, check=True)
            subprocess.run(
                ["git", "-c", "user.name=Test", "-c", "user.email=test@example.invalid",
                 "commit", "--allow-empty", "-qm", "baseline"],
                cwd=root, check=True,
            )
            # A PR-controlled Cargo alias must never execute during selection.
            (root / ".cargo").mkdir()
            (root / ".cargo/config.toml").write_text('[alias]\nx = "!exit 99"\n')
            subprocess.run(
                [sys.executable, str(consumer), "--mode", "subsystem",
                "--base", "HEAD", "--plan-file", str(root / "plan.json"),
                 "--planner-bin", str(planner)],
                cwd=root,
                env={**os.environ, "PATH": str(Path(shutil.which("git")).parent),
                     "GITHUB_OUTPUT": str(root / "outputs"),
                     "GITHUB_STEP_SUMMARY": str(root / "summary")},
                check=True, stdout=subprocess.PIPE, text=True,
            )
            selected = json.loads((root / "outputs").read_text().splitlines()[0].split("=", 1)[1])
            self.assertEqual(selected, ["cli-e2e"])

    def test_cargo_failure_cannot_produce_skip_outputs(self):
        self.run_action("subsystem", error=subprocess.CalledProcessError(1, ["cargo"]))


if __name__ == "__main__":
    unittest.main()
