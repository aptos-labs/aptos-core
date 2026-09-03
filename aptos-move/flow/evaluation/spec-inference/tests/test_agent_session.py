from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path
from typing import Any

from claude_agent_sdk import ResultMessage, SystemMessage

from harness.agent import ClaudeAgentSession
from harness.artifacts import JsonlWriter


def _result(session_id: str = "session-1") -> ResultMessage:
    return ResultMessage(
        subtype="success",
        duration_ms=1,
        duration_api_ms=1,
        is_error=False,
        num_turns=1,
        session_id=session_id,
        total_cost_usd=0.0,
        usage={"output_tokens": 1},
        result="done",
    )


class _ClientAnnouncingInitOnce:
    """The SDK's actual shape: `system/init` is emitted once, at startup."""

    def __init__(self) -> None:
        self.responses = 0

    async def query(self, prompt: str) -> None:
        self.responses += 1

    async def receive_response(self):
        if self.responses == 1:
            yield SystemMessage(
                subtype="init",
                data={"model": "glm-5.3[1m]", "claude_code_version": "2.1.258"},
            )
        yield _result()

    async def get_mcp_status(self) -> dict[str, Any]:
        return {"mcpServers": [{"name": "move-flow", "status": "connected", "tools": []}]}


class RuntimeIdentityAcrossTurnsTest(unittest.IsolatedAsyncioTestCase):
    """Runtime identity has to survive past the first turn.

    The controller validates identity after every turn, and multi-turn cells
    are the whole point of the follow-up policy. Reading `system/init` from the
    current response alone makes every follow-up look like a session with no
    identity, which the controller classifies as an infrastructure failure.
    The fake-agent tests cannot see this: identity validation is skipped for
    them.
    """

    async def test_a_follow_up_turn_still_reports_the_session_identity(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            log = Path(temporary) / "events.jsonl"
            with JsonlWriter(log) as event_log:
                session = ClaudeAgentSession.__new__(ClaudeAgentSession)
                session._client = _ClientAnnouncingInitOnce()
                session._event_log = event_log
                session._mcp_status = None
                session._system_init = None

                first = await session.send("infer the specification")
                second = await session.send("the candidate was rejected; repair it")

        for label, turn in (("first", first), ("follow-up", second)):
            with self.subTest(turn=label):
                self.assertIsInstance(turn.system_init["system"], dict, label)
                self.assertEqual(
                    "glm-5.3[1m]", turn.system_init["system"]["model"], label
                )
                self.assertEqual(
                    "connected",
                    turn.system_init["mcp_status"]["mcpServers"][0]["status"],
                    label,
                )

    async def test_the_settled_mcp_status_is_fetched_only_when_init_arrives(
        self,
    ) -> None:
        # The refresh exists to re-read a server the SDK reported as `pending`
        # at startup; repeating it every turn would be a needless round trip.
        with tempfile.TemporaryDirectory() as temporary:
            log = Path(temporary) / "events.jsonl"
            with JsonlWriter(log) as event_log:
                session = ClaudeAgentSession.__new__(ClaudeAgentSession)
                session._client = _ClientAnnouncingInitOnce()
                session._event_log = event_log
                session._mcp_status = None
                session._system_init = None
                await session.send("first")
                await session.send("second")
            settled = [
                line
                for line in log.read_text(encoding="utf-8").splitlines()
                if json.loads(line).get("event") == "mcp_status_settled"
            ]

        self.assertEqual(1, len(settled))


if __name__ == "__main__":
    unittest.main()
