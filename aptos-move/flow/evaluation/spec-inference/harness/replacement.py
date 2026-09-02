"""Auditable reserve replacement producing a new corpus version."""

from __future__ import annotations

import argparse
import json
from collections import Counter
from pathlib import Path
from typing import Any

from .artifacts import load_object, sha256_file, write_json


ALLOWED_REASONS = {
    "reference_preparation_failure",
    "compatibility_timeout",
    "snapshot_isolation_failure",
    "mutant_validation_failure",
}


def replace_task(
    manifest_path: Path,
    task_id: str,
    reason: str,
) -> dict[str, Any]:
    if reason not in ALLOWED_REASONS:
        raise ValueError(f"replacement reason must be one of {sorted(ALLOWED_REASONS)}")
    manifest = load_object(manifest_path)
    records = manifest["records"]
    target = next(
        (
            record
            for record in records
            if record["task_id"] == task_id and record["selection_status"] == "selected"
        ),
        None,
    )
    if target is None:
        raise ValueError(f"selected task `{task_id}` not found")
    selected = [
        record
        for record in records
        if record["selection_status"] == "selected" and record is not target
    ]
    selected_modules = {
        (record["source_root"], record["module"])
        for record in selected
        if record["granularity"] == "module"
    }
    function_counts = Counter(
        (record["source_root"], record["module"])
        for record in selected
        if record["granularity"] == "function"
    )
    target_size, target_semantic = target["sampling_cell"].split(":", 1)
    reserves = sorted(
        (
            record
            for record in records
            if record["selection_status"] == "reserve"
            and record["source_root"] == target["source_root"]
            and record["granularity"] == target["granularity"]
        ),
        key=lambda record: (
            _fallback_rank(record["sampling_cell"], target_size, target_semantic),
            int(record["reserve_order"]),
            record["task_id"],
        ),
    )
    replacement = next(
        (
            record
            for record in reserves
            if _feasible(record, selected_modules, function_counts)
            and _meets_feature_minima(
                selected + [record], manifest.get("minimum_feature_counts", {})
            )
        ),
        None,
    )
    if replacement is None:
        raise ValueError(
            "no feasible reserve remains for the same source root and granularity "
            f"after exhausting sampling cell {target['sampling_cell']}"
        )
    fallback_tier = _fallback_tier(
        replacement["sampling_cell"], target_size, target_semantic
    )
    target["selection_status"] = "excluded"
    target["selection_or_exclusion_reason"] = reason
    target["replaced_by"] = replacement["task_id"]
    replacement["selection_status"] = "selected"
    replacement["selection_or_exclusion_reason"] = "same_cell_reserve_replacement"
    replacement["replaces"] = target["task_id"]
    history = manifest.setdefault("replacement_history", [])
    history.append(
        {
            "sequence": len(history) + 1,
            "removed_task_id": target["task_id"],
            "replacement_task_id": replacement["task_id"],
            "sampling_cell": target["sampling_cell"],
            "replacement_sampling_cell": replacement["sampling_cell"],
            "fallback_tier": fallback_tier,
            "reason": reason,
            "input_manifest_sha256": sha256_file(manifest_path),
        }
    )
    selected_after = [
        record for record in records if record["selection_status"] == "selected"
    ]
    feature_counts = Counter(
        feature for record in selected_after for feature in record["feature_strata"]
    )
    manifest["selected_feature_counts"] = dict(sorted(feature_counts.items()))
    manifest["unmet_feature_minima"] = {
        feature: minimum - feature_counts[feature]
        for feature, minimum in manifest.get("minimum_feature_counts", {}).items()
        if feature_counts[feature] < minimum
    }
    return manifest


def _fallback_tier(cell: str, target_size: str, target_semantic: str) -> str:
    return (
        "same_cell",
        "same_semantic_stratum",
        "same_size_stratum",
        "same_source_and_granularity",
    )[_fallback_rank(cell, target_size, target_semantic)]


def _fallback_rank(cell: str, target_size: str, target_semantic: str) -> int:
    size, semantic = cell.split(":", 1)
    if size == target_size and semantic == target_semantic:
        return 0
    if semantic == target_semantic:
        return 1
    if size == target_size:
        return 2
    return 3


def _feasible(
    record: dict[str, Any],
    selected_modules: set[tuple[str, str]],
    function_counts: Counter[tuple[str, str]],
) -> bool:
    key = (record["source_root"], record["module"])
    if record["granularity"] == "function":
        return key not in selected_modules and function_counts[key] < 2
    return function_counts[key] == 0


def _meets_feature_minima(
    records: list[dict[str, Any]], minimums: dict[str, int]
) -> bool:
    counts = Counter(feature for record in records for feature in record["feature_strata"])
    return all(counts[feature] >= minimum for feature, minimum in minimums.items())



def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--task-id", required=True)
    parser.add_argument("--reason", choices=sorted(ALLOWED_REASONS), required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    manifest = replace_task(args.manifest.resolve(), args.task_id, args.reason)
    write_json(args.output.resolve(), manifest)


if __name__ == "__main__":
    main()
