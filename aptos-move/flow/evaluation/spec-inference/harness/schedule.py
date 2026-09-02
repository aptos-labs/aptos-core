"""Balanced randomized-block run schedule generation."""

from __future__ import annotations

import argparse
import hashlib
import itertools
import json
import os
import random
import tempfile
from dataclasses import asdict
from pathlib import Path
from typing import Any

from .artifacts import canonical_json, load_object, sha256_file, tree_hash, write_json
from .config import ExperimentConfig
from .materialize import materialize_task


ARMS = ("agent_only", "hybrid_guided", "hybrid_flexible")


def build_schedule(
    corpus_path: Path,
    plugins_path: Path,
    output_dir: Path,
    replicates: int,
    round_id: str = "round-001",
    experiment_config_path: Path | None = None,
    parent_round_id: str | None = None,
    changes: tuple[str, ...] = (),
) -> dict[str, Any]:
    corpus = load_object(corpus_path)
    plugins = load_object(plugins_path)
    _validate_identifier(round_id, "round ID")
    if parent_round_id is not None:
        _validate_identifier(parent_round_id, "parent round ID")
    if replicates < 1:
        raise ValueError("replicates must be positive")
    selected = [record for record in corpus["records"] if record["selection_status"] == "selected"]
    if len(selected) != 30:
        raise ValueError(f"corpus must contain exactly 30 selected tasks, got {len(selected)}")
    screen = corpus.get("compatibility_screen", {})
    if (
        screen.get("passed") != len(selected)
        or screen.get("failed") != 0
        or screen.get("requires_fix_or_rerun", 0) != 0
    ):
        raise ValueError(
            "corpus must pass the treatment-blind machine compatibility screen"
        )
    if set(plugins) != set(ARMS):
        raise ValueError(f"plugin map must contain exactly {list(ARMS)}")
    if output_dir.exists():
        raise FileExistsError(
            f"refusing to overwrite experiment round directory: {output_dir}"
        )
    config_sha256 = None
    if experiment_config_path is not None:
        config = ExperimentConfig.load(experiment_config_path)
        if config.source_commit != corpus["source_commit"]:
            raise ValueError("experiment config and corpus source commit disagree")
        config_sha256 = hashlib.sha256(canonical_json(asdict(config))).hexdigest()
    plugin_data = {arm: _plugin_record(Path(plugins[arm]).resolve(), arm) for arm in ARMS}
    for arm, data in plugin_data.items():
        if data["flow_source_commit"] != corpus["source_commit"]:
            raise ValueError(f"plugin `{arm}` and corpus source commit disagree")
    if plugin_data["hybrid_guided"]["mcp_tools"] != plugin_data["hybrid_flexible"]["mcp_tools"]:
        raise ValueError("the two hybrid MCP inventories differ")
    agent_tools = set(plugin_data["agent_only"]["mcp_tools"])
    hybrid_tools = set(plugin_data["hybrid_guided"]["mcp_tools"])
    if hybrid_tools - agent_tools != {"move_package_wp"} or agent_tools - hybrid_tools:
        raise ValueError("agent-only and hybrid MCP inventories differ by more than WP")
    recipes: dict[str, tuple[Path, Path, str]] = {}
    for task in selected:
        shared = (corpus_path.parent / task["shared_package_path"]).resolve()
        patch = (corpus_path.parent / task["preparation_patch"]).resolve()
        expected_hash = task.get("prepared_sha256")
        if (
            not expected_hash
            or tree_hash(shared) != task.get("shared_package_sha256")
            or sha256_file(patch) != task.get("preparation_patch_sha256")
        ):
            raise ValueError(f"shared package recipe mismatch for {task['task_id']}")
        with tempfile.TemporaryDirectory(
            prefix="move-inference-schedule-"
        ) as temporary:
            materialize_task(
                shared, patch, Path(temporary) / "package", expected_hash
            )
        recipes[task["task_id"]] = (shared, patch, expected_hash)

    # Do not leave a misleading round directory behind when any input fails
    # validation. Once created, the directory is immutable and never reused.
    output_dir.mkdir(parents=True)
    seed = hashlib.sha256(
        (corpus["selection_seed"] + "\0run-schedule\0" + round_id).encode()
    ).hexdigest()
    rng = random.Random(int(seed, 16))
    blocks = [(task, replicate) for task in sorted(selected, key=lambda item: item["task_id"]) for replicate in range(1, replicates + 1)]
    rng.shuffle(blocks)
    permutations = list(itertools.permutations(ARMS))
    assigned_orders: list[tuple[str, str, str]] = []
    while len(assigned_orders) < len(blocks):
        cycle = list(permutations)
        rng.shuffle(cycle)
        assigned_orders.extend(cycle)

    runs: list[dict[str, Any]] = []
    run_dir = output_dir / "runs"
    for block_index, ((task, replicate), order) in enumerate(zip(blocks, assigned_orders), 1):
        shared, patch, expected_hash = recipes[task["task_id"]]
        for order_index, arm in enumerate(order, 1):
            run_id = (
                f"{round_id}-{task['task_id']}-r{replicate:02d}-"
                f"{arm.replace('_', '-')}"
            )
            run_path = run_dir / f"{run_id}.json"
            run = {
                "schema_version": 2,
                "run_id": run_id,
                "round_id": round_id,
                "task_id": task["task_id"],
                "target": task["package_module_target"],
                "arm": arm,
                "replicate": replicate,
                "order": order_index,
                "block": block_index,
                "shared_package": os.path.relpath(shared, run_path.parent),
                "task_patch": os.path.relpath(patch, run_path.parent),
                "package_relpath": task.get("package_relpath", "."),
                "initial_tree_sha256": expected_hash,
                "plugin_dir": os.path.relpath(plugin_data[arm]["path"], run_path.parent),
                "plugin_manifest_sha256": plugin_data[arm]["manifest_sha256"],
                # Mutants are an optional scoring layer. Core-only rounds use
                # the all-zero sentinel and omit --hidden-mutants-root.
                "mutant_manifest_sha256": task.get(
                    "mutant_manifest_sha256", "0" * 64
                ),
                "experiment_config_sha256": config_sha256,
                "required_contract_categories": task["required_contract_categories"],
                "allowed_edit_paths": task.get("allowed_edit_paths", ["**/*.move"]),
            }
            write_json(run_path, run)
            runs.append(run)
    schedule = {
        "schema_version": 1,
        "round_id": round_id,
        "parent_round_id": parent_round_id,
        "changes_from_parent": list(changes),
        "round_kind": "iterative_benchmark",
        "source_commit": corpus["source_commit"],
        "corpus_sha256": sha256_file(corpus_path),
        "experiment_config_sha256": config_sha256,
        "randomization_seed": seed,
        "replicates": replicates,
        "blocks": len(blocks),
        "runs": runs,
        "plugins": {arm: {**data, "path": str(data["path"])} for arm, data in plugin_data.items()},
    }
    write_json(output_dir / "schedule.json", schedule)
    return schedule


def _validate_identifier(value: str, label: str) -> None:
    if not value or any(
        char
        not in "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789._-"
        for char in value
    ):
        raise ValueError(
            f"{label} must contain only letters, digits, '.', '_', or '-'"
        )


def _plugin_record(path: Path, arm: str) -> dict[str, Any]:
    manifest_path = path / "move-flow-manifest.json"
    manifest = load_object(manifest_path)
    if manifest.get("inference_tactic") != arm or manifest.get("evaluation_mode") is not True:
        raise ValueError(f"plugin `{path}` is not the evaluation plugin for `{arm}`")
    return {
        "path": path,
        "manifest_sha256": sha256_file(manifest_path),
        "rendered_inference_skill_sha256": manifest["rendered_inference_skill_sha256"],
        "mcp_tool_list_sha256": manifest["mcp_tool_list_sha256"],
        "flow_source_commit": manifest["flow_source_commit"],
        "mcp_tools": manifest["mcp_tools"],
    }



def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--corpus", type=Path, required=True)
    parser.add_argument("--plugins", type=Path, required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument("--replicates", type=int, default=5)
    parser.add_argument("--round-id", required=True)
    parser.add_argument("--experiment-config", type=Path, required=True)
    parser.add_argument("--parent-round-id")
    parser.add_argument(
        "--change",
        action="append",
        default=[],
        help="concise change from the parent round; may be repeated",
    )
    args = parser.parse_args()
    schedule = build_schedule(
        args.corpus.resolve(),
        args.plugins.resolve(),
        args.output_dir.resolve(),
        args.replicates,
        args.round_id,
        args.experiment_config.resolve(),
        args.parent_round_id,
        tuple(args.change),
    )
    print(json.dumps({"blocks": schedule["blocks"], "runs": len(schedule["runs"])}, sort_keys=True))


if __name__ == "__main__":
    main()
