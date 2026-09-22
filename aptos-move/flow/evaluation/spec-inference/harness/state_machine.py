"""Arm-blind controller transition policy."""

from __future__ import annotations

import json
from dataclasses import dataclass
from pathlib import Path

from .judge import JudgeResult


@dataclass(frozen=True)
class Decision:
    action: str
    prompt: str | None
    terminal_status: str | None


class ConversationPolicy:
    def __init__(self, prompts: Path):
        value = json.loads(prompts.read_text(encoding="utf-8"))
        if not isinstance(value, dict):
            raise ValueError("follow-up prompts must be a JSON object")
        self.prompts: dict[str, str] = value

    def decide(
        self,
        result: JudgeResult,
        target: str,
        forbidden_repairs: int,
    ) -> Decision:
        state = result.state
        if state == "infrastructure_failure":
            return Decision("retry_snapshot", None, None)
        if state == "operational_success":
            return Decision("stop", None, "operational_success")
        if state == "forbidden_weakening" and forbidden_repairs >= 1:
            return Decision("stop", None, "repeated_forbidden_weakening")
        if state not in self.prompts:
            raise ValueError(f"no pre-registered transition for judge state `{state}`")
        prompt = self.prompts[state].format(
            diagnostics=result.diagnostics,
            target=target,
        )
        return Decision("continue", prompt, None)
