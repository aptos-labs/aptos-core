from __future__ import annotations

import asyncio
from dataclasses import replace
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

from harness.agent import CodexAgentSession
from harness.artifacts import JsonlWriter
from harness.config import ExperimentConfig


ROOT = Path(__file__).resolve().parent.parent


class CodexAgentSessionTest(unittest.TestCase):
    def test_executes_and_resumes_jsonl_thread(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            workspace = root / "workspace"
            plugin = root / "plugin"
            codex_home = root / "codex-home"
            workspace.mkdir()
            (plugin / "skills/move-inf").mkdir(parents=True)
            (plugin / "skills/move-inf/SKILL.md").write_text(
                "---\nname: move-inf\ndescription: test\n---\n", encoding="utf-8"
            )
            (plugin / "move-flow-manifest.json").write_text(
                json.dumps({"mcp_tools": ["move_spec_check"]}), encoding="utf-8"
            )
            mcp = root / "mcp.json"
            mcp.write_text(
                json.dumps({"mcpServers": {"move-flow": {"command": "unused"}}}),
                encoding="utf-8",
            )
            executable = root / "codex"
            executable.write_text(
                "#!/bin/sh\n"
                "if [ \"${1:-}\" = --version ]; then echo 'codex-cli 0.153.2'; exit; fi\n"
                "printf '%s\\n' '{\"type\":\"thread.started\",\"thread_id\":\"thread-1\"}'\n"
                "printf '%s\\n' '{\"type\":\"item.completed\",\"item\":{\"type\":\"agent_message\",\"text\":\"done\"}}'\n"
                "printf '%s\\n' '{\"type\":\"turn.completed\",\"usage\":{\"input_tokens\":3,\"cached_input_tokens\":1,\"output_tokens\":2}}'\n",
                encoding="utf-8",
            )
            executable.chmod(0o755)
            config = replace(
                ExperimentConfig.load(ROOT / "config/default.json"),
                agent_runtime="codex",
                codex_cli_version="0.153.2",
                model="gpt-5.6-sol",
                provider_base_url="https://chatgpt.com/backend-api",
                effort="high",
            )
            stderr: list[str] = []

            async def exercise() -> None:
                with JsonlWriter(root / "events.jsonl") as events:
                    async with CodexAgentSession(
                        config,
                        workspace,
                        plugin,
                        mcp,
                        events,
                        stderr.append,
                        root / "mcp.sock",
                    ) as session:
                        first = await session.send("first")
                        second = await session.send("second")
                self.assertEqual("done", first.result)
                self.assertEqual("thread-1", second.session_id)
                self.assertEqual(2, second.usage["output_tokens"])

            environment = {
                "PATH": f"{root}:{os.environ['PATH']}",
                "CODEX_HOME": str(codex_home),
            }
            with patch.dict(os.environ, environment, clear=True):
                asyncio.run(exercise())
            generated = (codex_home / "config.toml").read_text(encoding="utf-8")
            self.assertIn('model = "gpt-5.6-sol"', generated)
            self.assertIn('model_reasoning_effort = "high"', generated)
            self.assertIn('command = "/opt/bin/move-flow-mcp"', generated)
            self.assertEqual([], stderr)


if __name__ == "__main__":
    unittest.main()
