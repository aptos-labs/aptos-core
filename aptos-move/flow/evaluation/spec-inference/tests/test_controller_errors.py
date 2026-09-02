from __future__ import annotations

import asyncio
import json
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace

from harness.agent import AgentTurn
from harness.controller import (
    Controller,
    _agent_infrastructure_error,
    _count_lines,
    _send_with_wall_deadline,
    _within,
)


def turn(**overrides: object) -> AgentTurn:
    fields: dict[str, object] = {
        "result": "",
        "is_error": False,
        "session_id": "session",
        "duration_ms": 1,
        "duration_api_ms": 1,
        "num_turns": 1,
        "usage": {},
        "model_usage": {},
        "total_cost_usd": 0.0,
        "system_init": None,
        "terminal_reason": "completed",
        "api_error_status": None,
    }
    fields.update(overrides)
    return AgentTurn(**fields)  # type: ignore[arg-type]


class ControllerErrorTest(unittest.TestCase):
    def test_initial_prompt_activates_plugin_skill_before_task(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            prompts = root / "prompts"
            prompts.mkdir()
            (prompts / "initial.txt").write_text(
                "Infer {target} in {package}.", encoding="utf-8"
            )
            controller = Controller.__new__(Controller)
            controller.prompts_dir = prompts
            controller.run = SimpleNamespace(
                spec=SimpleNamespace(target="0x42::example::f")
            )

            prompt = controller._initial_prompt()

        self.assertTrue(prompt.startswith("/move-inf\n\n"))
        self.assertIn("Infer 0x42::example::f in .", prompt)

    def test_statusless_api_error_is_infrastructure(self) -> None:
        error = _agent_infrastructure_error(
            turn(
                is_error=True,
                terminal_reason="api_error",
                result="Not logged in · Please run /login",
            )
        )
        self.assertEqual(
            "provider API error: Not logged in · Please run /login",
            error,
        )

    def test_normal_result_is_not_infrastructure(self) -> None:
        self.assertIsNone(_agent_infrastructure_error(turn(result="done")))

    def test_http_status_is_preserved(self) -> None:
        self.assertEqual(
            "provider API error 429",
            _agent_infrastructure_error(
                turn(is_error=True, terminal_reason="api_error", api_error_status=429)
            ),
        )


class PackageRelpathTest(unittest.TestCase):
    """`package_relpath` comes from the run manifest and roots both trees.

    A value that climbs out would aim the pristine baseline at the agent's own
    workspace, and the authoritative candidate check would then compare the
    workspace to itself.
    """

    def test_a_relative_package_resolves_under_its_tree(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary) / "workspace"
            self.assertEqual(root / "framework", _within(root, "framework"))
            self.assertEqual(root, _within(root, "."))

    def test_a_package_may_not_reach_outside_its_tree(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary) / "baseline"
            for relative in ("../workspace", "/etc", "a/../../workspace"):
                with self.subTest(relative=relative):
                    with self.assertRaises(ValueError) as raised:
                        _within(root, relative)
                    self.assertIn("escapes", str(raised.exception))


class ControllerFlowRestartTest(unittest.TestCase):
    @staticmethod
    def _controller(events: Path) -> Controller:
        controller = Controller.__new__(Controller)
        controller.flow_events = events
        return controller

    @staticmethod
    def _session(sequence: int, restart: bool) -> str:
        return json.dumps(
            {
                "event": "session_start",
                "session_id": f"flow-{sequence}",
                "restart": restart,
            }
        )

    def test_restart_after_offset_invalidates_the_observation(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            events = Path(temporary) / "flow-events.jsonl"
            events.write_text(
                self._session(1, False) + "\n" + self._session(2, True) + "\n",
                encoding="utf-8",
            )

            error = self._controller(events)._flow_restart_error(0)

        self.assertEqual("Flow MCP supervisor restarted session 'flow-2'", error)

    def test_restart_before_offset_does_not_invalidate_a_retry(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            events = Path(temporary) / "flow-events.jsonl"
            events.write_text(
                self._session(1, True) + "\n", encoding="utf-8"
            )
            offset = _count_lines(events)
            with events.open("a", encoding="utf-8") as stream:
                stream.write(self._session(2, False) + "\n")

            controller = self._controller(events)

        self.assertEqual(1, offset)
        self.assertIsNone(controller._flow_restart_error(offset))

    def test_clean_session_and_partial_line_are_not_failures(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            events = Path(temporary) / "flow-events.jsonl"
            events.write_text(
                self._session(1, False) + "\n{\"event\": \"tool_start\"",
                encoding="utf-8",
            )

            self.assertIsNone(self._controller(events)._flow_restart_error(0))

    def test_missing_telemetry_is_not_a_failure(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            events = Path(temporary) / "absent.jsonl"

            self.assertEqual(0, _count_lines(events))
            self.assertIsNone(self._controller(events)._flow_restart_error(0))


class ControllerWallDeadlineTest(unittest.IsolatedAsyncioTestCase):
    async def test_in_flight_agent_turn_is_cancelled_at_wall_deadline(self) -> None:
        class NeverReturns:
            async def send(self, prompt: str) -> AgentTurn:
                del prompt
                await asyncio.Event().wait()
                raise AssertionError("unreachable")

        with self.assertRaises(TimeoutError):
            await _send_with_wall_deadline(NeverReturns(), "prompt", 0.01)  # type: ignore[arg-type]

    async def test_expired_deadline_does_not_start_agent_turn(self) -> None:
        class MustNotRun:
            async def send(self, prompt: str) -> AgentTurn:
                raise AssertionError(f"unexpected prompt: {prompt}")

        with self.assertRaises(TimeoutError):
            await _send_with_wall_deadline(MustNotRun(), "prompt", 0)  # type: ignore[arg-type]


if __name__ == "__main__":
    unittest.main()
