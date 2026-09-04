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


#: Inventory schemas this selection can read. Additive versions only.
SUPPORTED_INVENTORY_SCHEMAS = frozenset({1, 2, 3})


def build_provenance(
    inventory_path: Path,
    config_path: Path,
    screening_ledger_path: Path | None = None,
) -> dict[str, Any]:
    inventory = load_object(inventory_path)
    config = load_object(config_path)
    # The inventory schema has only ever grown: every version adds candidate
    # fields and removes none, so a selection written against version 1 reads a
    # later one unchanged. Pinning to 1 meant selection silently could not run
    # on a current inventory at all -- which is how the corpus came to be
    # stratified without the fields later versions added.
    if (
        inventory.get("schema_version") not in SUPPORTED_INVENTORY_SCHEMAS
        or config.get("schema_version") != 1
    ):
        raise ValueError(
            "unsupported inventory or corpus configuration schema: inventory "
            f"{inventory.get('schema_version')}, config {config.get('schema_version')}"
        )
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
            # A frame the policy describes but the corpus draws nothing from
            # contributes no tasks: the policy says which candidates are
            # eligible, the quotas say how many of them a corpus takes.
            quota = int(quotas.get(root, {}).get(granularity, 0))
            if quota == 0:
                continue
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
        "quotas": config["quotas"],
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
    shapes = {_shape(record) for record in already_selected}
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
            # Two candidates with the same granularity and the same semantic
            # features are the same task twice: a corpus of thirty small
            # straight-line getters measures one thing thirty times. Size and
            # call-depth are properties of the sample rather than of what the
            # contract has to say, so they do not distinguish a shape.
            if _shape(record) in shapes:
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
        shapes.add(_shape(selected))
    return chosen


#: Strata that say how a target is reached rather than what its contract says.
SHAPE_INSENSITIVE_STRATA = frozenset({"deep-calls", "shallow-calls"})


def _shape(record: dict[str, Any]) -> tuple[str, frozenset[str]]:
    """What a task teaches.

    Two candidates with the same granularity and the same semantic features are
    the same task twice, and a corpus of thirty small straight-line getters
    measures one thing thirty times. Size stays in the signature: a five-line
    accessor and a two-hundred-line settlement function share a stratum set and
    are not the same problem. Call depth does not: it describes how the target
    is reached, not what has to be said about it.
    """
    return (
        record["granularity"],
        frozenset(set(record["feature_strata"]) - SHAPE_INSENSITIVE_STRATA),
    )


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


#: Screening outcomes that exclude a candidate rather than demanding a repair.
#:
#: Each is a measurement of what the apparatus can do with the target, made
#: without reference to any arm: the stage timed out, the prover or inference
#: could not carry the target, or the inferred contract rests on a boundary
#: nobody has reviewed. A corpus cannot measure a target it cannot screen, and
#: dropping one for that reason chooses membership on the apparatus rather than
#: on an arm's behaviour. An infrastructure failure is not among them: it says
#: the tooling broke, not that the target is beyond it.
# A task leaves the corpus only for a measured reason: the apparatus timed out,
# the target is not well-formed, or its reference does not prove. WP falling
# short of a verifying contract is a property of the task, never an exclusion.
SCREENING_EXCLUSION_REASONS = frozenset(
    {
        "compatibility_timeout",
        "implementation_failure",
        "reference_unproved",
    }
)


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
            if reason not in SCREENING_EXCLUSION_REASONS:
                raise ValueError(
                    f"screening result for {record['task_id']} requires a fix or rerun; "
                    f"only a measured screening failure may exclude a candidate, not {reason}"
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
