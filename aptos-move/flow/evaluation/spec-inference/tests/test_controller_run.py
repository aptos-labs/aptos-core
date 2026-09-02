"""End-to-end controller runs driven by the scripted fake agent.

These exercise the real turn loop, judge invocation, and terminal
classification. Judge commands are replaced with deterministic stub programs so
the tests need no Move toolchain, and `move-flow` resolution is patched so they
do not depend on an installed binary.
"""

from __future__ import annotations

import asyncio
import json
import tempfile
import unittest
from dataclasses import asdict
from pathlib import Path
from unittest.mock import patch

from harness.artifacts import tree_hash
from harness.config import ExperimentConfig, ResolvedRunSpec, RunSpec
from harness.controller import Controller


COMMIT = "a" * 40
COMPILE_STUB = "import sys; sys.exit(0)"
# Judge command templates pass through `str.format_map`, so a stub program must
# not contain literal braces. The stub accepts any candidate: these tests cover
# the controller's turn loop, not the acceptance rules themselves.
CANDIDATE_STUB = (
    "import json,pathlib,sys;"
    "report=pathlib.Path(sys.argv[2]);"
    "report.parent.mkdir(parents=True,exist_ok=True);"
    "report.write_text(json.dumps(dict(schema_version=1,"
    "state='candidate_accepted',accepted=True,diagnostics='')))"
)
PROVE_STUB = "import sys; sys.exit(0)"
# Accepts only a candidate whose specification carries the marker the second
# scripted turn writes, so one run exercises the rejection, the follow-up, and
# the recovery. The template forbids literal braces.
REJECT_UNTIL_REPAIRED_STUB = (
    "import json,pathlib,sys;"
    "package=pathlib.Path(json.load(open(sys.argv[1]))['package']);"
    "repaired='repaired' in (package/'sources'/'example.move').read_text();"
    "report=pathlib.Path(sys.argv[2]);"
    "report.parent.mkdir(parents=True,exist_ok=True);"
    "report.write_text(json.dumps(dict(schema_version=1,"
    "state='candidate_accepted' if repaired else 'prover_failure',"
    "accepted=repaired,"
    "diagnostics='' if repaired else 'post-condition does not hold')))"
)

BASELINE_SOURCE = "module 0x42::example { fun f() {} }\n"
SPEC_EDIT = {
    "sources/example.move": BASELINE_SOURCE
    + "spec 0x42::example { spec f { ensures true == true; } }\n"
}



def _stub(program: str, *arguments: str) -> list[str]:
    return ["python3", "-c", program, *arguments]


def _config(**overrides: object) -> ExperimentConfig:
    fields: dict[str, object] = {
        "schema_version": 1,
        "source_commit": COMMIT,
        "claude_agent_sdk_version": "0.2.139",
        "claude_code_version": "2.1.241",
        "model": "fake",
        "provider_base_url": "https://api.z.ai/api/anthropic",
        "effort": "max",
        "feedback_level": "acceptance",
        "max_controller_turns": 3,
        "max_model_turns_per_controller_turn": 60,
        "max_wall_seconds": 2700,
        "max_output_tokens": 100000,
        "operational_timeout_seconds": 10,
        "eventual_timeout_seconds": 40,
        "infrastructure_retries": 1,
        "allowed_builtin_tools": [],
        "denied_builtin_tools": [],
        "compile_command": _stub(COMPILE_STUB),
        "prove_command": _stub(PROVE_STUB),
        "inference_command": _stub(COMPILE_STUB),
        "check_candidate_command": _stub(CANDIDATE_STUB, "{config}", "{output}"),
    }
    fields.update(overrides)
    return ExperimentConfig(**fields)  # type: ignore[arg-type]


def _session_start(
    identifier: str,
    restart: bool,
    feedback_level: str | None = "acceptance",
    candidate_check_configured: bool = True,
) -> dict[str, object]:
    return {
        "event": "session_start",
        "session_id": identifier,
        "restart": restart,
        "inference_tactic": "agent_only",
        "evaluation_mode": True,
        "flow_source_commit": COMMIT,
        **(
            {}
            if feedback_level is None
            else {
                "feedback_level": feedback_level,
                "candidate_check_configured": candidate_check_configured,
            }
        ),
    }


class ControllerRunTest(unittest.TestCase):
    def _run(
        self,
        script: list[dict[str, object]],
        config_overrides: dict[str, object] | None = None,
    ) -> dict[str, object]:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            package = root / "package"
            (package / "sources").mkdir(parents=True)
            (package / "Move.toml").write_text(
                '[package]\nname = "Example"\nversion = "1.0.0"\n', encoding="utf-8"
            )
            (package / "sources" / "example.move").write_text(
                BASELINE_SOURCE, encoding="utf-8"
            )
            patch_file = root / "task.patch"
            patch_file.write_text("", encoding="utf-8")

            plugin = root / "plugin"
            plugin.mkdir()
            (plugin / "move-flow-manifest.json").write_text(
                json.dumps(
                    {
                        "inference_tactic": "agent_only",
                        "flow_source_commit": COMMIT,
                        "evaluation_mode": True,
                        "feedback_level": "acceptance",
                        "mcp_tools": [],
                    }
                ),
                encoding="utf-8",
            )
            (plugin / ".mcp.json").write_text(
                json.dumps({"mcpServers": {"move-flow": {"command": "move-flow"}}}),
                encoding="utf-8",
            )

            prompts = root / "prompts"
            prompts.mkdir()
            (prompts / "initial.txt").write_text("Infer {target} in {package}.", encoding="utf-8")
            (prompts / "followups.json").write_text(
                json.dumps(
                    {
                        "compile_failure": "Fix {target}: {diagnostics}",
                        "prover_failure": "Prove {target}: {diagnostics}",
                        "prover_timeout": "Simplify {target}: {diagnostics}",
                        "no_progress": "Continue with {target}.",
                        "forbidden_weakening": "Restore {target}: {diagnostics}",
                    }
                ),
                encoding="utf-8",
            )

            script_path = root / "script.json"
            script_path.write_text(json.dumps(script), encoding="utf-8")

            config = _config(**(config_overrides or {}))
            spec = RunSpec(
                schema_version=2,
                run_id="test-run",
                task_id="test-task",
                target="0x42::example::f",
                arm="agent_only",
                replicate=1,
                shared_package=str(package),
                task_patch=str(patch_file),
                plugin_dir=str(plugin),
                plugin_manifest_sha256="0" * 64,
                initial_tree_sha256=tree_hash(package),
                mutant_manifest_sha256="0" * 64,
                required_contract_categories=("normal-result",),
                allowed_edit_paths=("sources/**/*.move",),
            )
            resolved = ResolvedRunSpec(spec, package, patch_file, plugin)
            controller = Controller(
                config,
                resolved,
                root / "artifacts" / spec.run_id,
                prompts,
                "fake",
                script_path,
                None,
            )
            # The plugin hash is checked against the run manifest; the fixture
            # manifest is generated here rather than pinned.
            with (
                patch(
                    "harness.controller.sha256_file",
                    return_value=spec.plugin_manifest_sha256,
                ),
                patch("harness.controller.shutil.which", return_value="/usr/bin/true"),
                patch(
                    "harness.controller._binary_identity",
                    return_value={"path": "/usr/bin/true"},
                ),
            ):
                result = asyncio.run(controller.run_experiment())
            events = [
                json.loads(line)
                for line in (
                    controller.artifact_dir / "controller-events.jsonl"
                ).read_text(encoding="utf-8").splitlines()
            ]
            return {"result": result, "events": events}

    def test_clean_session_reaches_operational_success(self) -> None:
        outcome = self._run(
            [
                {
                    "result": "done",
                    "writes": SPEC_EDIT,
                    "flow_events": [_session_start("flow-1", False)],
                }
            ]
        )

        self.assertEqual("operational_success", outcome["result"]["terminal_status"])
        self.assertEqual(1, outcome["result"]["attempts"])

    def test_a_rejected_candidate_draws_a_follow_up_and_recovers(self) -> None:
        """The controller's follow-up path, which no real round has reached.

        Every session of `corpus-v3-pilot-002` finished in one turn, so the
        rejection-to-follow-up cycle has never run outside a unit test. Here the
        first turn writes a specification the judge rejects and the second
        repairs it, which is the only way to exercise the prompt the policy
        selects for the failure kind.
        """
        rejected = {
            "result": "done",
            "writes": SPEC_EDIT,
            "flow_events": [_session_start("flow-1", False)],
        }
        repaired = {
            "result": "done",
            "writes": {
                "sources/example.move": BASELINE_SOURCE
                + "spec 0x42::example { spec f { ensures true == true; /* repaired */ } }\n"
            },
            "flow_events": [_session_start("flow-1", False)],
        }

        outcome = self._run(
            [rejected, repaired],
            config_overrides={
                "check_candidate_command": _stub(
                    REJECT_UNTIL_REPAIRED_STUB, "{config}", "{output}"
                )
            },
        )

        self.assertEqual("operational_success", outcome["result"]["terminal_status"])
        agent_results = [e for e in outcome["events"] if e["event"] == "agent_result"]
        self.assertEqual(2, len(agent_results), "the run should take a second turn")
        prompts = [e for e in outcome["events"] if e["event"] == "prompt"]
        self.assertEqual(2, len(prompts))
        # The follow-up is the one the policy selects for a prover failure, and
        # it names neither the arm nor the tactic.
        follow_up = json.dumps(prompts[1])
        self.assertIn("Prove", follow_up)
        self.assertNotIn("agent_only", follow_up)

    def test_a_turn_interrupted_at_the_wall_deadline_keeps_its_usage(self) -> None:
        hanging = {
            "result": "interrupted",
            "hang": True,
            "writes": SPEC_EDIT,
            "usage": {"input_tokens": 11, "output_tokens": 7},
            "total_cost_usd": 0.5,
            "flow_events": [_session_start("flow-1", False)],
        }

        outcome = self._run([hanging], config_overrides={"max_wall_seconds": 1})

        self.assertEqual("wall_budget_exhausted", outcome["result"]["terminal_status"])
        results = [
            event for event in outcome["events"] if event["event"] == "agent_result"
        ]
        self.assertEqual(1, len(results))
        self.assertTrue(results[0]["interrupted_at_wall_deadline"])
        self.assertEqual(7, results[0]["result"]["usage"]["output_tokens"])
        self.assertEqual(0.5, results[0]["result"]["total_cost_usd"])

    def test_supervisor_restart_retries_then_invalidates_the_run(self) -> None:
        restarted = {
            "result": "done",
            "writes": SPEC_EDIT,
            "flow_events": [_session_start("flow-1", False), _session_start("flow-2", True)],
        }

        outcome = self._run([restarted, restarted, restarted, restarted])

        self.assertEqual(
            "invalid_infrastructure_failure", outcome["result"]["terminal_status"]
        )
        self.assertEqual(2, outcome["result"]["attempts"])
        restarts = [
            event
            for event in outcome["events"]
            if event["event"] == "flow_supervisor_restart"
        ]
        self.assertEqual(2, len(restarts))
        self.assertIn("restarted session 'flow-2'", restarts[0]["detail"])


if __name__ == "__main__":
    unittest.main()


class FlowConfigurationTest(unittest.TestCase):
    """An arm must not run without the apparatus it was scheduled for."""

    def test_a_missing_candidate_check_invalidates_the_run(self) -> None:
        # The exact defect from feedback-ablation-002: the server started with
        # neither the feedback level nor the candidate check, and its default
        # level matched what the plugin declared, so nothing noticed.
        outcome = ControllerRunTest()._run(
            [
                {
                    "result": "done",
                    "writes": SPEC_EDIT,
                    "flow_events": [
                        _session_start("flow-1", False, candidate_check_configured=False)
                    ],
                }
            ]
            * 4
        )

        self.assertEqual(
            "invalid_infrastructure_failure", outcome["result"]["terminal_status"]
        )

    def test_a_mismatched_feedback_level_invalidates_the_run(self) -> None:
        outcome = ControllerRunTest()._run(
            [
                {
                    "result": "done",
                    "writes": SPEC_EDIT,
                    "flow_events": [_session_start("flow-1", False, feedback_level="baseline")],
                }
            ]
            * 4
        )

        self.assertEqual(
            "invalid_infrastructure_failure", outcome["result"]["terminal_status"]
        )

    def test_a_build_without_the_telemetry_is_still_runnable(self) -> None:
        outcome = ControllerRunTest()._run(
            [
                {
                    "result": "done",
                    "writes": SPEC_EDIT,
                    "flow_events": [_session_start("flow-1", False, feedback_level=None)],
                }
            ]
        )

        self.assertEqual("operational_success", outcome["result"]["terminal_status"])


class ApparatusIdentityTest(unittest.TestCase):
    """A round must not change beneath its own observations."""

    @staticmethod
    def _controller(**overrides: object) -> Controller:
        spec = RunSpec(
            schema_version=2,
            run_id="test-run",
            task_id="test-task",
            target="0x42::example::f",
            arm="agent_only",
            replicate=1,
            shared_package=".",
            task_patch=".",
            plugin_dir=".",
            plugin_manifest_sha256="0" * 64,
            initial_tree_sha256="0" * 64,
            mutant_manifest_sha256="0" * 64,
            required_contract_categories=("normal-result",),
            **overrides,  # type: ignore[arg-type]
        )
        controller = Controller.__new__(Controller)
        controller.run = ResolvedRunSpec(spec, Path("."), Path("."), Path("."))
        return controller

    def test_an_edited_harness_stops_the_run(self) -> None:
        controller = self._controller(controller_harness_sha256="f" * 64)

        with self.assertRaisesRegex(ValueError, "controller harness changed"):
            controller._validate_apparatus_identity("0" * 64, {"sha256": "0" * 64})

    def test_a_rebuilt_binary_stops_the_run(self) -> None:
        controller = self._controller(move_flow_sha256="f" * 64)

        with self.assertRaisesRegex(ValueError, "move-flow binary changed"):
            controller._validate_apparatus_identity("0" * 64, {"sha256": "a" * 64})

    def test_a_round_that_pins_nothing_is_still_allowed(self) -> None:
        # Rounds scheduled before the apparatus was pinned stay runnable.
        self._controller()._validate_apparatus_identity("0" * 64, {"sha256": "0" * 64})
