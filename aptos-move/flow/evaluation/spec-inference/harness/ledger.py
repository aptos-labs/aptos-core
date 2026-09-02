"""Merge treatment-blind screening results into the preselection ledger."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

from .artifacts import load_object, sha256_file, write_json


def merge_screening_ledger(
    screen_manifest_path: Path,
    existing_path: Path | None = None,
    invalidate_failure_stages: set[str] | None = None,
) -> dict[str, Any]:
    screen = load_object(screen_manifest_path)
    commit = screen["source_commit"]
    existing = load_object(existing_path) if existing_path else None
    if existing and (
        existing.get("schema_version") != 1
        or existing.get("source_commit") != commit
    ):
        raise ValueError("existing screening ledger schema or commit mismatch")
    entries = {
        entry["task_id"]: entry for entry in (existing or {}).get("entries", [])
    }
    invalidate_failure_stages = invalidate_failure_stages or set()
    entries = {
        task_id: entry
        for task_id, entry in entries.items()
        if not (
            entry.get("reason") != "compatibility_timeout"
            and entry.get("failed_stage") in invalidate_failure_stages
        )
    }
    added = 0
    for record in screen["records"]:
        compatibility = record.get("compatibility_screen")
        if record["selection_status"] != "selected" or not compatibility:
            continue
        reason = compatibility["reason"]
        failed_stage = _failed_stage(screen_manifest_path, compatibility)
        if (
            reason != "compatibility_timeout"
            and failed_stage in invalidate_failure_stages
        ):
            continue
        if compatibility["passed"] is not True and reason != "compatibility_timeout":
            raise ValueError(
                f"cannot record {record['task_id']} in the screening ledger: "
                f"{reason} must be fixed or rerun"
            )
        entry = {
            "task_id": record["task_id"],
            "package_module_target": record["package_module_target"],
            "source_sha256": record["source_sha256"],
            "passed": compatibility["passed"],
            "reason": reason,
            "threshold_seconds": compatibility["threshold_seconds"],
            "threshold_exceeded_stage": compatibility["threshold_exceeded_stage"],
            "total_duration_ms": compatibility["total_duration_ms"],
            "stage_duration_ms": compatibility["stage_duration_ms"],
            "result_sha256": compatibility["result_sha256"],
            "failed_stage": failed_stage,
            "tool_executables": compatibility.get("tool_executables", {}),
        }
        previous = entries.get(record["task_id"])
        if previous is not None and previous != entry:
            raise ValueError(f"conflicting screening result for {record['task_id']}")
        if previous is None:
            added += 1
        entries[record["task_id"]] = entry
    return {
        "schema_version": 1,
        "source_commit": commit,
        "screen_manifests": [
            *(existing or {}).get("screen_manifests", []),
            {
                "path": str(screen_manifest_path),
                "sha256": sha256_file(screen_manifest_path),
            },
        ],
        "entries": [entries[task_id] for task_id in sorted(entries)],
        "summary": {
            "screened": len(entries),
            "passed": sum(entry["passed"] is True for entry in entries.values()),
            "excluded": sum(entry["passed"] is not True for entry in entries.values()),
            "added_by_last_merge": added,
        },
    }



def _failed_stage(
    screen_manifest_path: Path, compatibility: dict[str, Any]
) -> str | None:
    if compatibility.get("passed") is True:
        return None
    result_path = compatibility.get("result_path")
    if not result_path:
        return compatibility.get("threshold_exceeded_stage")
    result = load_object((screen_manifest_path.parent / result_path).resolve())
    for name in ("compile", "wp_inference", "enriched_compile", "prover"):
        stage = result.get(name)
        if not stage:
            continue
        if (
            stage.get("timed_out")
            or stage.get("infrastructure_error")
            or stage.get("returncode") != 0
        ):
            return name
    return None


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--screen-manifest", type=Path, required=True)
    parser.add_argument("--existing", type=Path)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument(
        "--invalidate-failure-stage",
        action="append",
        default=[],
        choices=("compile", "wp_inference", "enriched_compile", "prover"),
    )
    args = parser.parse_args()
    ledger = merge_screening_ledger(
        args.screen_manifest.resolve(),
        args.existing.resolve() if args.existing else None,
        set(args.invalidate_failure_stage),
    )
    write_json(args.output.resolve(), ledger)
    print(json.dumps(ledger["summary"], sort_keys=True))


if __name__ == "__main__":
    main()
