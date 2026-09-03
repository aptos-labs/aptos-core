"""Deterministic quota-constrained selection and provenance generation."""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import re
from collections import Counter, defaultdict
from pathlib import Path
from typing import Any

from .artifacts import canonical_json, load_object, sha256_file, write_json


FRAMEWORK = "aptos-move/framework/aptos-framework"
EXPERIMENTAL = "aptos-move/framework/aptos-experimental"


def build_provenance(
    inventory_path: Path,
    config_path: Path,
    screening_ledger_path: Path | None = None,
) -> dict[str, Any]:
    inventory = load_object(inventory_path)
    config = load_object(config_path)
    if inventory.get("schema_version") != 1 or config.get("schema_version") != 1:
        raise ValueError("unsupported inventory or corpus configuration schema")
    commit = inventory["source_commit"]
    seed = hashlib.sha256((commit + config["selection_seed_suffix"]).encode()).hexdigest()
    records = [dict(candidate) for candidate in inventory["candidates"]]
    thresholds = _add_size_and_depth_strata(records)
    _assign_stable_ids(records)
    screening_ledger = (
        load_object(screening_ledger_path) if screening_ledger_path else None
    )
    if screening_ledger:
        _apply_screening_ledger(records, commit, screening_ledger)
    for record in records:
        record["sampling_cell"] = _sampling_cell(record)

    selected: list[dict[str, Any]] = []
    selected_modules: set[tuple[str, str]] = set()
    module_function_counts: Counter[tuple[str, str]] = Counter()
    quotas: dict[str, dict[str, int]] = config["quotas"]

    # Select module tasks first so the no-overlap rule cannot be invalidated by
    # an earlier function choice from the same module.
    for root in (FRAMEWORK, EXPERIMENTAL):
        for granularity in ("module", "function"):
            quota = int(quotas[root][granularity])
            pool = [
                record
                for record in records
                if record["source_root"] == root
                and record["granularity"] == granularity
                and record["eligibility"] == "eligible"
            ]
            choices = _greedy_select(
                pool,
                quota,
                selected,
                selected_modules,
                module_function_counts,
                config["minimum_feature_counts"],
                int(config["maximum_functions_per_module"]),
                seed,
            )
            if len(choices) != quota:
                raise ValueError(
                    f"candidate frame cannot satisfy {root}/{granularity} quota: "
                    f"needed {quota}, selected {len(choices)}"
                )
            for record in choices:
                record["selection_status"] = "selected"
                record["selection_or_exclusion_reason"] = "deterministic_constrained_sample"
                selected.append(record)
                key = (record["source_root"], record["module"])
                if granularity == "module":
                    selected_modules.add(key)
                else:
                    module_function_counts[key] += 1

    selected_ids = {id(record) for record in selected}
    reserve_groups: defaultdict[tuple[str, str, str], list[dict[str, Any]]] = defaultdict(list)
    for record in records:
        if id(record) in selected_ids:
            continue
        if record["eligibility"] != "eligible":
            record["selection_status"] = "excluded"
            record["selection_or_exclusion_reason"] = record["decision_reason"]
            continue
        key = (
            record["source_root"],
            record["granularity"],
            record["sampling_cell"],
        )
        reserve_groups[key].append(record)
    for group in reserve_groups.values():
        group.sort(key=lambda record: _random_key(seed, record))
        for index, record in enumerate(group, 1):
            record["selection_status"] = "reserve"
            record["reserve_order"] = index
            record["selection_or_exclusion_reason"] = "deterministic_reserve"

    for record in records:
        record["source_commit"] = commit
        record["reference_origin"] = (
            "upstream" if record["source_root"] == FRAMEWORK else "study-authored"
        )
        record.setdefault("pristine_sha256", None)
        record.setdefault("prepared_sha256", None)
        record.setdefault("preparation_patch", None)
        record.setdefault("reference_sha256", None)
        record.setdefault("reference_review", {"status": "pending", "reviewers": []})
        record.setdefault("mutant_review", {"status": "pending", "approved_count": 0})

    selected_counts = Counter(
        (record["source_root"], record["granularity"])
        for record in records
        if record["selection_status"] == "selected"
    )
    feature_counts = Counter(
        feature
        for record in records
        if record["selection_status"] == "selected"
        for feature in record["feature_strata"]
    )
    unmet = {
        feature: minimum - feature_counts[feature]
        for feature, minimum in config["minimum_feature_counts"].items()
        if feature_counts[feature] < minimum
    }
    result = {
        "schema_version": 1,
        "source_commit": commit,
        "selection_seed": seed,
        "selection_config_sha256": hashlib.sha256(canonical_json(config)).hexdigest(),
        "inventory_sha256": sha256_file(inventory_path),
        "screening_ledger_sha256": (
            sha256_file(screening_ledger_path)
            if screening_ledger_path
            else None
        ),
        "thresholds": thresholds,
        "minimum_feature_counts": config["minimum_feature_counts"],
        "selected_counts": {f"{root}:{granularity}": count for (root, granularity), count in sorted(selected_counts.items())},
        "selected_feature_counts": dict(sorted(feature_counts.items())),
        "unmet_feature_minima": unmet,
        "corpus_status": "selected",
        "records": sorted(records, key=lambda record: record["task_id"]),
    }
    return result


def _greedy_select(
    pool: list[dict[str, Any]],
    quota: int,
    already_selected: list[dict[str, Any]],
    selected_modules: set[tuple[str, str]],
    module_function_counts: Counter[tuple[str, str]],
    minimums: dict[str, int],
    maximum_functions_per_module: int,
    seed: str,
) -> list[dict[str, Any]]:
    chosen: list[dict[str, Any]] = []
    feature_counts = Counter(
        feature for record in already_selected for feature in record["feature_strata"]
    )
    remaining = list(pool)
    while len(chosen) < quota:
        feasible = []
        for record in remaining:
            module_key = (record["source_root"], record["module"])
            if record["granularity"] == "function":
                if module_key in selected_modules:
                    continue
                count = module_function_counts[module_key] + sum(
                    1
                    for item in chosen
                    if item["granularity"] == "function"
                    and (item["source_root"], item["module"]) == module_key
                )
                if count >= maximum_functions_per_module:
                    continue
            elif any(
                item["granularity"] == "function"
                and (item["source_root"], item["module"]) == module_key
                for item in already_selected + chosen
            ):
                continue
            deficit_gain = sum(
                max(0, int(minimums.get(feature, 0)) - feature_counts[feature])
                for feature in set(record["feature_strata"])
            )
            feasible.append((-deficit_gain, _random_key(seed, record), record))
        if not feasible:
            break
        _, _, selected = min(feasible, key=lambda item: (item[0], item[1]))
        chosen.append(selected)
        remaining.remove(selected)
        feature_counts.update(selected["feature_strata"])
    return chosen


def _add_size_and_depth_strata(records: list[dict[str, Any]]) -> dict[str, int]:
    eligible_functions = [
        record
        for record in records
        if record["eligibility"] == "eligible" and record["granularity"] == "function"
    ]
    loc_values = sorted(int(record["source_loc"]) for record in eligible_functions)
    depth_values = sorted(int(record["dependency_depth"]) for record in eligible_functions)
    small_max = _quantile(loc_values, 1 / 3)
    medium_max = _quantile(loc_values, 2 / 3)
    deep_min = _quantile(depth_values, 2 / 3)
    for record in records:
        loc = int(record["source_loc"])
        if record["granularity"] == "module" or loc > medium_max:
            size = "large"
        elif loc <= small_max:
            size = "small"
        else:
            size = "medium"
        features = set(record["feature_strata"])
        features.add(size)
        features.add("deep-calls" if int(record["dependency_depth"]) >= deep_min else "shallow-calls")
        record["feature_strata"] = sorted(features)
    return {"small_loc_max": small_max, "medium_loc_max": medium_max, "deep_call_min": deep_min}


def _quantile(values: list[int], fraction: float) -> int:
    if not values:
        return 0
    return values[min(len(values) - 1, max(0, math.ceil(len(values) * fraction) - 1))]


def _assign_stable_ids(records: list[dict[str, Any]]) -> None:
    counters: Counter[str] = Counter()
    for record in sorted(
        records,
        key=lambda item: (
            item["source_root"],
            item["source_path"],
            item["granularity"],
            item["package_module_target"],
        ),
    ):
        prefix = "AF" if record["source_root"] == FRAMEWORK else "AX"
        slug = _slug(record["module"].split("::")[-1])
        counters[f"{prefix}-{slug}"] += 1
        record["task_id"] = f"{prefix}-{slug}-{counters[f'{prefix}-{slug}']:03d}"


def _apply_screening_ledger(
    records: list[dict[str, Any]], commit: str, ledger: dict[str, Any]
) -> None:
    if ledger.get("schema_version") != 1 or ledger.get("source_commit") != commit:
        raise ValueError("screening ledger schema or source commit mismatch")
    by_id = {record["task_id"]: record for record in records}
    for entry in ledger.get("entries", []):
        record = by_id.get(entry.get("task_id"))
        if record is None:
            raise ValueError(f"screening ledger has unknown task {entry.get('task_id')}")
        if record["package_module_target"] != entry.get("package_module_target"):
            raise ValueError(f"screening target mismatch for {record['task_id']}")
        if record["source_sha256"] != entry.get("source_sha256"):
            raise ValueError(f"screening source mismatch for {record['task_id']}")
        record["compatibility_screen"] = entry
        if entry.get("passed") is not True:
            reason = entry.get("reason")
            if reason != "compatibility_timeout":
                raise ValueError(
                    f"screening result for {record['task_id']} requires a fix or rerun; "
                    "only a measured compatibility timeout may exclude a candidate"
                )
            record["eligibility"] = "excluded"
            record["decision_reason"] = reason


def _slug(value: str) -> str:
    return re.sub(r"[^a-z0-9]+", "-", value.lower()).strip("-") or "target"


def _sampling_cell(record: dict[str, Any]) -> str:
    features = set(record["feature_strata"])
    size = next((name for name in ("small", "medium", "large") if name in features), "unknown")
    semantic = next(
        (
            name
            for name in (
                "loop",
                "higher-order",
                "global-state",
                "mutable-reference",
                "arithmetic-abort",
                "multiple-calls",
                "straight-line",
            )
            if name in features
        ),
        "other",
    )
    return f"{size}:{semantic}"


def _random_key(seed: str, record: dict[str, Any]) -> str:
    identity = "\0".join(
        str(record[key])
        for key in ("source_root", "source_path", "granularity", "package_module_target")
    )
    return hashlib.sha256(f"{seed}\0{identity}".encode()).hexdigest()



def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--inventory", type=Path, required=True)
    parser.add_argument("--config", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--screening-ledger", type=Path)
    args = parser.parse_args()
    provenance = build_provenance(args.inventory, args.config, args.screening_ledger)
    write_json(args.output, provenance)
    if provenance["unmet_feature_minima"]:
        raise SystemExit(
            "selected quotas could not meet feature minima: "
            + json.dumps(provenance["unmet_feature_minima"], sort_keys=True)
        )


if __name__ == "__main__":
    main()
