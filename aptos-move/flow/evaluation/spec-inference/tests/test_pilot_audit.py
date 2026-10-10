from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

from harness.pilot_audit import _attempt_count, _audit_flow_telemetry, audit_pilot


def _config() -> dict[str, object]:
    return {
        "schema_version": 1,
        "source_commit": "a" * 40,
        "claude_agent_sdk_version": "0.2.139",
        "claude_code_version": "2.1.241",
        "model": "glm-5.3[1m]",
        "provider_base_url": "https://api.z.ai/api/anthropic",
        "effort": "max",
        "feedback_level": "acceptance",
        "max_controller_turns": 6,
        "max_model_turns_per_controller_turn": 60,
        "max_wall_seconds": 2700,
        "max_output_tokens": 100000,
        "operational_timeout_seconds": 10,
        "eventual_timeout_seconds": 40,
        "infrastructure_retries": 1,
        "allowed_builtin_tools": [],
        "denied_builtin_tools": [],
        "compile_command": [],
        "prove_command": [],
        "inference_command": [],
        "check_candidate_command": [],
    }


SESSION = {
    "session_id": "flow-session",
    "flow_source_commit": "a" * 40,
    "inference_tactic": "agent_only",
    "evaluation_mode": True,
}


class AttemptCountTest(unittest.TestCase):
    """Where the attempt count lives decides whether a retry is an audit failure.

    A recovered run holds one provider session per attempt. Reading the count
    from the scheduler's manifest instead of the controller's result yields 1
    for every run, so the first genuine infrastructure retry is rejected.
    """

    def test_the_controller_result_supplies_the_count(self) -> None:
        self.assertEqual(2, _attempt_count({"attempts": 2}))
        self.assertEqual(1, _attempt_count({"attempts": 1}))

    def test_a_run_manifest_does_not_supply_it(self) -> None:
        # This is the shape that made the earlier fix ineffective: `attempts`
        # sits under `result`, and the top level has no such key.
        manifest = {"run_id": "r", "arm": "agent_only", "result": {"attempts": 2}}
        self.assertEqual(1, _attempt_count(manifest))

    def test_a_missing_or_absurd_count_falls_back_to_one(self) -> None:
        for record in ({}, {"attempts": None}, {"attempts": "2"}, {"attempts": 0}):
            with self.subTest(record=record):
                self.assertEqual(1, _attempt_count(record))


class PilotAuditTest(unittest.TestCase):
    def test_flow_telemetry_uses_normalized_tactic_and_balanced_session(self) -> None:
        issues: list[dict[str, str]] = []
        invalid: list[dict[str, str]] = []
        _audit_flow_telemetry(
            [
                {"event": "session_start", "restart": False, **SESSION},
                {"event": "session_end", **SESSION},
            ],
            "run",
            "agent_only",
            "a" * 40,
            {},
            issues,
            invalid,
        )
        self.assertEqual([], issues)
        self.assertEqual([], invalid)

    def test_supervisor_restart_is_an_infrastructure_invalidation(self) -> None:
        issues: list[dict[str, str]] = []
        invalid: list[dict[str, str]] = []
        _audit_flow_telemetry(
            [
                {"event": "session_start", "restart": False, **SESSION},
                {"event": "session_start", "restart": True, **SESSION},
                {"event": "session_end", **SESSION},
            ],
            "run",
            "agent_only",
            "a" * 40,
            {},
            issues,
            invalid,
        )
        self.assertEqual([], issues)
        self.assertEqual(2, len(invalid))
        self.assertIn("restarted session", invalid[0]["detail"])
        self.assertIn("start/end mismatch", invalid[1]["detail"])

    def test_reports_missing_runs_without_claiming_completion(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "schedule" / "runs").mkdir(parents=True)
            (root / "artifacts").mkdir()
            config_path = root / "config.json"
            config_path.write_text(json.dumps(_config()), encoding="utf-8")
            (root / "schedule" / "pilot-manifest.json").write_text(
                json.dumps(
                    {
                        "runs": 72,
                        "blocks": 24,
                        "replicates": 3,
                        "task_count": 8,
                        "controller_harness_sha256": "b" * 64,
                        "controller_prompts_sha256": "c" * 64,
                        "experiment_config_sha256": "d" * 64,
                    }
                ),
                encoding="utf-8",
            )
            result = audit_pilot(root / "schedule", root / "artifacts", config_path)
            self.assertFalse(result["complete"])
            self.assertEqual(0, result["audited_runs"])
            self.assertEqual(72, result["expected_runs"])
            self.assertIn("expected 72", result["issues"][0]["detail"])

    def test_expected_run_count_follows_the_round_manifest(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "schedule" / "runs").mkdir(parents=True)
            (root / "artifacts").mkdir()
            config_path = root / "config.json"
            config_path.write_text(json.dumps(_config()), encoding="utf-8")
            (root / "schedule" / "pilot-manifest.json").write_text(
                json.dumps(
                    {
                        "runs": 24,
                        "blocks": 8,
                        "replicates": 1,
                        "task_count": 8,
                        "controller_harness_sha256": "b" * 64,
                        "controller_prompts_sha256": "c" * 64,
                        "experiment_config_sha256": "d" * 64,
                    }
                ),
                encoding="utf-8",
            )
            result = audit_pilot(root / "schedule", root / "artifacts", config_path)
            self.assertEqual(24, result["expected_runs"])
            self.assertIn("expected 24", result["issues"][0]["detail"])


if __name__ == "__main__":
    unittest.main()
