"""Normalize prover/WP warnings to actionable shared-package source sites."""

from __future__ import annotations

import argparse
import json
import posixpath
import re
from collections import defaultdict
from pathlib import Path
from typing import Any

from .artifacts import write_json


_LOCATION = re.compile(r"\u250c\u2500\s+([^\n]+\.move):(\d+):(\d+)")
_WEAKENING_MARKERS = (
    "cannot derive `folds_of` exactly",
    "weakening the enclosing loop invariant",
    "behavioral predicates",
    "per-iteration",
)


def audit_warnings(results: list[Path], output: Path) -> dict[str, Any]:
    sites: dict[tuple[str, str, int, int], set[str]] = defaultdict(set)
    for path in results:
        value = json.loads(path.read_text(encoding="utf-8"))
        task_id = path.stem
        for diagnostic in _diagnostics(value):
            if not diagnostic.startswith("warning:"):
                continue
            locations = _LOCATION.findall(diagnostic)
            if not locations:
                continue
            source, line, column = locations[-1]
            source = _normalize_source_path(source)
            kind = diagnostic.splitlines()[0].removeprefix("warning: ")
            sites[(kind, source, int(line), int(column))].add(task_id)
    records = [
        {
            "warning": kind,
            "source": source,
            "line": line,
            "column": column,
            "tasks": sorted(tasks),
            "requires_loop_normalization": any(
                marker in kind for marker in _WEAKENING_MARKERS
            ),
        }
        for (kind, source, line, column), tasks in sorted(sites.items())
    ]
    result = {
        "schema_version": 1,
        "results": [str(path) for path in results],
        "unique_warning_sites": len(records),
        "loop_normalization_sites": sum(
            record["requires_loop_normalization"] for record in records
        ),
        "warnings": records,
    }
    write_json(output, result)
    return result


def _diagnostics(value: Any) -> list[str]:
    result = []
    pending = [value]
    while pending:
        item = pending.pop()
        if isinstance(item, dict):
            for key, child in item.items():
                if key == "diagnostics":
                    if isinstance(child, list):
                        result.extend(value for value in child if isinstance(value, str))
                    elif isinstance(child, str):
                        result.append(child)
                else:
                    pending.append(child)
        elif isinstance(item, list):
            pending.extend(item)
    return result


def _normalize_source_path(value: str) -> str:
    value = value.replace("\\", "/")
    marker = "/sources/"
    if marker in value:
        value = "sources/" + value.rsplit(marker, 1)[1]
    return posixpath.normpath(value)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--results", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    paths = sorted(args.results.resolve().glob("*.json"))
    result = audit_warnings(paths, args.output.resolve())
    print(
        json.dumps(
            {
                "unique_warning_sites": result["unique_warning_sites"],
                "loop_normalization_sites": result["loop_normalization_sites"],
            },
            sort_keys=True,
        )
    )


if __name__ == "__main__":
    main()
