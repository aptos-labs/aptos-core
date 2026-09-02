from __future__ import annotations

import dataclasses
import unittest
import os
from pathlib import Path
import subprocess
import sys
import tempfile
from unittest.mock import patch

from harness.pilot_sandbox import (
    AGENT_BOOGIE,
    BOOGIE_PROXY_SOCKET,
    _missing_parent_directories,
    _reject_development_options,
    _require_inside_round,
    _required_solver,
    _round_directory,
    agent_landlock_paths,
    build_bwrap_command,
    Launch,
    preflight,
)


def _example_launch(root: Path) -> Launch:
    return Launch(
        command=(sys.executable, "-m", "harness.controller"),
        config=root / "config.json",
        run_manifest=root / "run.json",
        artifacts=root / "artifacts",
        run_id="test-run",
        shared_package=root / "snapshot",
        task_patch=root / "task.patch",
        plugin=root / "plugin",
        evaluation_root=root / "evaluation",
        python_root=None,
        move_flow=root / "move-flow",
        claude=root / "claude",
        boogie=root / "boogie",
        boogie_client=root / "boogie-proxy-client.py",
        z3=root / "z3",
        landlock=root / "landlock-exec",
        feedback_level="acceptance",
    )


class PilotSandboxTest(unittest.TestCase):
    def test_parent_directories_do_not_recreate_system_mounts(self) -> None:
        directories = _missing_parent_directories(
            [Path("/home/test/study/harness"), Path("/etc/resolv.conf")]
        )
        self.assertIn(Path("/home/test/study"), directories)
        self.assertIn(Path("/etc"), directories)
        self.assertNotIn(Path("/usr"), directories)
        self.assertNotIn(Path("/lib"), directories)

    def test_the_agent_writes_only_its_workspace_and_telemetry(self) -> None:
        # The run directory holds the pristine baseline the judge compares
        # against and the run's own record. Write access there would let a
        # session rewrite the baseline to match an edited implementation, or
        # forge `run.json`; only the workspace and the telemetry file are the
        # agent's to write.
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            launch = _example_launch(root)
            readable, writable = agent_landlock_paths(launch)
            run_dir = root / "artifacts" / "test-run"
            self.assertIn(run_dir / "workspace", writable)
            self.assertIn(run_dir / "flow-events.jsonl", writable)
            self.assertNotIn(run_dir, writable)
            for controller_owned in ("baseline", "run.json", "judge.json", "candidate-check.json"):
                self.assertNotIn(run_dir / controller_owned, writable)
            for needed in ("baseline", "candidate-check.json", "mcp.runtime.json", "plugin"):
                self.assertIn(run_dir / needed, readable)
            # A baseline cell is not given the task's criteria at all.
            baseline_launch = dataclasses.replace(launch, feedback_level="baseline")
            readable, _ = agent_landlock_paths(baseline_launch)
            self.assertNotIn(run_dir / "candidate-check.json", readable)

    def test_manifest_paths_may_not_escape_the_round(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve()
            round_dir = root / "evaluation-artifacts" / "round-1"
            inside = round_dir / "schedule" / "snapshots" / "corpus"
            inside.mkdir(parents=True)
            # The scheduler writes these relative to the manifest, so the
            # legitimate shape climbs within the round and must be accepted.
            _require_inside_round(round_dir, (("shared_package", inside),))
            _require_inside_round(
                round_dir,
                (("shared_package", round_dir / "schedule" / "runs" / ".." / ".." / "plugins"),),
            )

            outside = root / "elsewhere"
            outside.mkdir()
            for escape in (
                outside,
                round_dir / "schedule" / ".." / ".." / "other-round",
            ):
                with self.assertRaises(SystemExit) as raised:
                    _require_inside_round(round_dir, (("task_patch", escape),))
                self.assertIn("escapes the round directory", str(raised.exception))

    def test_preflight_hides_host_paths_and_confines_the_agent_proc(self) -> None:
        # The security property, asserted unconditionally: this is the only
        # test that runs the real `landlock-exec`, and nothing in CI runs it.
        result = preflight()
        self.assertEqual(3, result["policy_version"])
        self.assertTrue(result["isolation"], result["detail"])
        self.assertIn("host-path-and-agent-proc-isolation=passed", result["detail"])

    def test_preflight_runs_the_prover_chain_as_an_agent_grandchild(self) -> None:
        # Whether this host can run the solver chain is a separate question,
        # and one a host without Boogie or Z3 cannot answer. A round still
        # needs both -- `passed` is their conjunction -- but reporting a
        # missing solver as a failing sandbox policy would be untrue.
        result = preflight()
        if result["prover_pipeline"] is None:
            self.skipTest(f"sandbox isolation probe did not run: {result['detail']}")
        try:
            _required_solver("BOOGIE_EXE", "boogie")
            _required_solver("Z3_EXE", "z3")
        except SystemExit as error:
            self.skipTest(f"no solver toolchain on this host: {error}")
        self.assertTrue(result["prover_pipeline"], result["detail"])
        self.assertTrue(result["passed"], result["detail"])
        self.assertIn("prover-pipeline-as-agent-grandchild=passed", result["detail"])

    def test_wrapper_sets_sandbox_marker_inside_cleared_environment(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            evaluation_root = root / "evaluation"
            artifacts = root / "artifacts"
            staging = root / "staging"
            for directory in (
                evaluation_root / "harness",
                evaluation_root / "prompts",
                artifacts,
                staging,
                root / "snapshot",
                root / "plugin",
            ):
                directory.mkdir(parents=True, exist_ok=True)
            paths = {
                name: root / name
                for name in (
                    "config",
                    "run",
                    "move-flow",
                    "claude",
                    "boogie",
                    "boogie-proxy-client.py",
                    "z3",
                    "landlock",
                )
            }
            for path in paths.values():
                path.touch()
            launch = Launch(
                command=(sys.executable, "-m", "harness.controller"),
                config=paths["config"],
                run_manifest=paths["run"],
                artifacts=artifacts,
                run_id="test-run",
                shared_package=root / "snapshot",
                task_patch=paths["run"],
                plugin=root / "plugin",
                evaluation_root=evaluation_root,
                python_root=None,
                move_flow=paths["move-flow"],
                claude=paths["claude"],
                boogie=paths["boogie"],
                boogie_client=paths["boogie-proxy-client.py"],
                z3=paths["z3"],
                landlock=paths["landlock"],
                feedback_level="acceptance",
            )
            with (
                patch("harness.pilot_sandbox._required_executable", return_value=Path("/usr/bin/bwrap")),
                patch.dict(os.environ, {"MOVE_INFERENCE_EVAL_SANDBOXED": "host-value"}),
            ):
                command = build_bwrap_command(launch, staging)

        clearenv = command.index("--clearenv")
        marker = command.index("MOVE_INFERENCE_EVAL_SANDBOXED", clearenv)
        self.assertEqual("--setenv", command[marker - 1])
        self.assertEqual("1", command[marker + 1])
        self.assertNotIn("host-value", command)

    def test_the_agent_is_confined_more_narrowly_than_the_sandbox(self) -> None:
        launch = _example_launch(Path("/eval"))
        readable, writable = agent_landlock_paths(launch)

        # The agent edits its workspace and reads its plugin and toolchain.
        self.assertIn(launch.plugin, readable)
        self.assertIn(launch.move_flow, readable)
        self.assertIn(launch.artifacts / launch.run_id / "workspace", writable)
        # The apparatus it is measured against is mounted for the controller
        # and stays out of the agent's reach.
        self.assertNotIn(launch.evaluation_root / "harness", readable + writable)
        self.assertNotIn(launch.evaluation_root / "prompts", readable + writable)
        self.assertNotIn(launch.shared_package, readable + writable)
        self.assertNotIn(launch.task_patch, readable + writable)
        self.assertNotIn(launch.config, readable + writable)
        # Boogie runs on the controller's side of the proxy: the agent gets the
        # client under `/opt`, never the executable. Z3 it needs itself, for
        # `move-flow`'s version probe.
        self.assertNotIn(launch.boogie, readable + writable)
        self.assertIn(launch.z3, readable)
        self.assertIn(Path("/opt"), readable)

    def test_the_agent_boogie_is_the_proxy_client(self) -> None:
        launch = _example_launch(Path("/eval"))
        with (
            tempfile.TemporaryDirectory() as staging,
            patch("harness.pilot_sandbox._required_executable", return_value=Path("/usr/bin/bwrap")),
        ):
            command = build_bwrap_command(launch, Path(staging))
        client = command.index(str(launch.boogie_client))
        self.assertEqual("--ro-bind", command[client - 1])
        self.assertEqual(str(AGENT_BOOGIE), command[client + 1])
        for name, value in (
            ("BOOGIE_EXE", str(launch.boogie)),
            ("MOVE_INFERENCE_BOOGIE_REAL", str(launch.boogie)),
            ("MOVE_INFERENCE_BOOGIE_AGENT", str(AGENT_BOOGIE)),
            ("MOVE_INFERENCE_BOOGIE_PROXY", str(BOOGIE_PROXY_SOCKET)),
        ):
            index = command.index(name)
            self.assertEqual("--setenv", command[index - 1])
            self.assertEqual(value, command[index + 1])

    def test_production_wrapper_rejects_fake_agent(self) -> None:
        with self.assertRaises(SystemExit) as raised:
            _reject_development_options(
                [sys.executable, "-m", "harness.controller", "--agent", "fake"]
            )
        self.assertIn("permits only --agent claude", str(raised.exception))

    def test_production_wrapper_rejects_development_options(self) -> None:
        for option in ("--fake-script", "--allow-unsandboxed"):
            with self.assertRaises(SystemExit) as raised:
                _reject_development_options(
                    [sys.executable, "-m", "harness.controller", option]
                )
            self.assertIn("rejects development option", str(raised.exception))


if __name__ == "__main__":
    unittest.main()


class RoundDirectoryTest(unittest.TestCase):
    """Every mounted path must belong to one round under evaluation-artifacts."""

    ROOT = Path("/eval")

    def test_a_round_path_resolves_to_its_round(self) -> None:
        self.assertEqual(
            Path("/eval/evaluation-artifacts/round-002"),
            _round_directory(
                self.ROOT,
                Path("/eval/evaluation-artifacts/round-002/schedule/runs/a.json"),
            ),
        )

    def test_a_path_outside_evaluation_artifacts_has_no_round(self) -> None:
        self.assertIsNone(_round_directory(self.ROOT, Path("/eval/config/default.json")))
        self.assertIsNone(_round_directory(self.ROOT, Path("/etc/passwd")))

    def test_the_artifacts_root_itself_is_not_a_round(self) -> None:
        self.assertIsNone(
            _round_directory(self.ROOT, Path("/eval/evaluation-artifacts"))
        )
