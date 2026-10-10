"""Summarize Codex exec JSONL events retained by the provider adapter."""

from __future__ import annotations

import json
from collections import Counter
from pathlib import Path
from typing import Any

from .artifacts import write_json


def write_codex_metrics(
    events_path: Path, output_path: Path, *, allow_incomplete_tail: bool = False
) -> None:
    counts: Counter[str] = Counter()
    tokens: Counter[str] = Counter()
    sessions: list[str] = []
    completed = 0
    truncated_tail_bytes = 0
    lines = events_path.read_bytes().splitlines(keepends=True)
    for index, line in enumerate(lines):
        try:
            record = json.loads(line)
        except (ValueError, UnicodeDecodeError):
            if allow_incomplete_tail and index == len(lines) - 1 and not line.endswith(b"\n"):
                truncated_tail_bytes = len(line)
                break
            raise
        if record.get("event") != "codex_event":
            continue
        event: dict[str, Any] = record.get("payload") or {}
        kind = str(event.get("type") or "unknown")
        counts[kind] += 1
        if kind == "thread.started" and isinstance(event.get("thread_id"), str):
            sessions.append(event["thread_id"])
        if kind == "turn.completed":
            completed += 1
            for name, value in (event.get("usage") or {}).items():
                if isinstance(value, int):
                    tokens[name] += value
    write_json(
        output_path,
        {
            "schema_version": 1,
            "source": events_path.name,
            "coverage": "Codex exec --json stdout",
            "event_counts": dict(counts),
            "sessions": sessions,
            "turn_count": completed,
            "usage_totals": dict(tokens),
            "truncated_tail_bytes": truncated_tail_bytes,
            "totals_complete": bool(completed) and not truncated_tail_bytes,
        },
    )
