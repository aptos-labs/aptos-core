"""Identity of each workflow whose report the trusted pr-ci-report action accepts.

`.github/ci/pr-ci-report-producers.json` is the only list of producers. The
reporter reads it to bind runs and to find each benchmark step. The Ruby
workflow tests read it to check the producer and consumer workflows."""

from __future__ import annotations

import json
from dataclasses import dataclass, fields
from pathlib import Path
from typing import Any

from ci_actions.github import ActionError
from ci_actions.run_binding import Producer

MANIFEST_PATH = Path(__file__).resolve().parent.parent / "pr-ci-report-producers.json"


@dataclass(frozen=True)
class ReportProducer:
    key: str             # report "producer" value and sticky comment header
    workflow_path: str   # ".github/workflows/<file>.yaml"
    workflow_name: str   # must equal both workflow.name and run.name
    job: str             # benchmark job name in the run's job list
    benchmark_step: str  # name of the step that runs the benchmark
    label: str           # PR label that approves a run; read by the Ruby tests

    def binding(self) -> Producer:
        return Producer(self.key, self.workflow_path, self.workflow_name)


FIELDS = frozenset(field.name for field in fields(ReportProducer))


def _reject_duplicate_keys(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    keys = [key for key, _ in pairs]
    if len(set(keys)) != len(keys):
        raise ActionError("producer manifest contains a duplicate JSON key")
    return dict(pairs)


def parse_report_producers(text: str) -> dict[str, ReportProducer]:
    try:
        value = json.loads(text, object_pairs_hook=_reject_duplicate_keys)
    except json.JSONDecodeError as error:
        raise ActionError(f"producer manifest must be valid JSON: {error}") from None
    if not isinstance(value, dict) or set(value) != {"producers"}:
        raise ActionError("producer manifest must be an object with only a producers key")
    entries = value["producers"]
    if not isinstance(entries, list) or not entries:
        raise ActionError("producer manifest producers must be a non-empty list")
    producers: dict[str, ReportProducer] = {}
    for index, entry in enumerate(entries):
        if not isinstance(entry, dict) or set(entry) != FIELDS:
            raise ActionError(f"producer[{index}] must have exactly the fields {sorted(FIELDS)}")
        if not all(isinstance(entry[name], str) and entry[name] for name in FIELDS):
            raise ActionError(f"producer[{index}] fields must be non-empty strings")
        if entry["key"] in producers:
            raise ActionError(f"producer[{index}] key is duplicated")
        producers[entry["key"]] = ReportProducer(**entry)
    return producers


def load_report_producers() -> dict[str, ReportProducer]:
    return parse_report_producers(MANIFEST_PATH.read_text(encoding="utf-8"))
