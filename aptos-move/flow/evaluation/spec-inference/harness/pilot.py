"""Build the unscored development-round schedule."""

from __future__ import annotations

import argparse
import hashlib
import itertools
import json
import os
import random
import shutil
from dataclasses import asdict, dataclass, field
from pathlib import Path
from collections.abc import Mapping, Sequence
from typing import Any

from .identifiers import require_plain_name
from .artifacts import canonical_json, sha256_file, tree_hash, write_json
from .config import ExperimentConfig, FEEDBACK_LEVELS, RunSpec
from .mutants import NO_MUTANTS
from .schedule import ARMS


def build_pilot(
    corpus_manifest: Path,
    plugins: dict[str, dict[str, Path]],
    output_dir: Path,
    source_commit: str,
    experiment_config_path: Path,
    replicates: int | Mapping[str, int] = 3,
    round_id: str = "pilot-001",
    task_names: Sequence[str] | None = None,
    mutants_root: Path | None = None,
) -> dict[str, Any]:
    """Schedule one round over tasks, arms, and feedback levels.

    Feedback level is a factor inside the round rather than a reason to run a
    separate one. A block then contains every arm/level cell for one task, so a
    comparison between apparatus configurations is made within the same block
    and is not confounded by drift between rounds.

    `task_names` restricts the round to a subset of the corpus samples, which
    spends a fixed budget on more replicates of fewer tasks. The subset is
    recorded in the manifest, so a round never silently covers less than it
    appears to.

    `replicates` may name a count per feedback level. A level held as a control
    then costs a fraction of the level under study while still appearing in the
    first replicate of every block, so the two remain comparable within a block.

    `mutants_root` turns on strict scoring. It must hold `TASK_ID/mutants.json`
    for every scheduled task; a round that names it and cannot supply one for
    each task fails here rather than degrading to core scoring in silence.
    """
    if task_names is not None and not task_names:
        raise ValueError("a pilot round needs at least one task")
    if not plugins:
        raise ValueError("at least one feedback level must be scheduled")
    for level, arm_plugins in plugins.items():
        if level not in FEEDBACK_LEVELS:
            raise ValueError(f"unknown feedback level `{level}`")
        unknown_arms = sorted(set(arm_plugins) - set(ARMS))
        if unknown_arms:
            raise ValueError(f"unknown arm(s) for `{level}`: {', '.join(unknown_arms)}")
        if not arm_plugins:
            raise ValueError(f"level `{level}` schedules no arm")
    # A round may study a subset of the arms — comparing two of them needs no
    # third — but every level must schedule the same set, or a block would not
    # hold the same comparison for each task.
    scheduled_arms = [arm for arm in ARMS if arm in next(iter(plugins.values()))]
    for level, arm_plugins in plugins.items():
        if [arm for arm in ARMS if arm in arm_plugins] != scheduled_arms:
            raise ValueError("every feedback level must schedule the same arms")
    feedback_levels = sorted(plugins)
    if isinstance(replicates, Mapping):
        unknown = sorted(set(replicates) - set(feedback_levels))
        if unknown:
            raise ValueError(f"replicates name unscheduled level(s): {', '.join(unknown)}")
        missing = sorted(set(feedback_levels) - set(replicates))
        if missing:
            raise ValueError(f"replicates omit scheduled level(s): {', '.join(missing)}")
        level_replicates = dict(replicates)
    else:
        level_replicates = {level: replicates for level in feedback_levels}
    if any(count < 1 for count in level_replicates.values()):
        raise ValueError("every scheduled feedback level needs at least one replicate")
    config = ExperimentConfig.load(experiment_config_path)
    if config.source_commit != source_commit:
        raise ValueError("experiment config and pilot source commit disagree")
    config_sha256 = hashlib.sha256(canonical_json(asdict(config))).hexdigest()
    evaluation_root = Path(__file__).resolve().parent.parent
    controller_harness_sha256 = tree_hash(evaluation_root / "harness")
    controller_prompts_sha256 = tree_hash(evaluation_root / "prompts")
    move_flow_sha256 = _move_flow_sha256()
    snapshots_dir = output_dir / "snapshots"
    patches_dir = output_dir / "patches"
    runs_dir = output_dir / "runs"
    # A schedule is written once. Rescheduling over an existing one replaces
    # the shared snapshot and the run manifests that cells still resolve
    # against, while dispatch treats an existing `judge.json` as
    # `already_complete` and never revisits the apparatus hashes -- so the
    # round would silently mix two schedule versions and its artifacts would
    # no longer describe what ran. Every round gets a new round ID instead.
    existing = sorted(path.name for path in runs_dir.glob("*.json")) if runs_dir.is_dir() else []
    if existing:
        raise FileExistsError(
            f"{runs_dir} already holds {len(existing)} run manifest(s); a schedule is "
            "written once. Use a new round ID, or remove the round directory deliberately."
        )
    output_dir.mkdir(parents=True, exist_ok=True)
    snapshots_dir.mkdir(exist_ok=True)
    patches_dir.mkdir(exist_ok=True)
    runs_dir.mkdir(exist_ok=True)

    plugin_records = {
        level: {
            arm: _plugin_record(path.resolve(), arm, source_commit, level)
            for arm, path in arm_plugins.items()
        }
        for level, arm_plugins in plugins.items()
    }
    tasks = _corpus_tasks(corpus_manifest, snapshots_dir, patches_dir, task_names)

    mutant_digests = _resolve_mutant_manifests(tasks, mutants_root)

    seed = hashlib.sha256(
        (source_commit + "\0unscored-pilot\0" + require_plain_name(round_id, "round_id")).encode()
    ).hexdigest()
    rng = random.Random(int(seed, 16))
    max_replicates = max(level_replicates.values())
    blocks = [(task, replicate) for task in tasks for replicate in range(1, max_replicates + 1)]
    rng.shuffle(blocks)

    runs: list[dict[str, Any]] = []
    arm_orders = _balanced_arm_orders(rng, scheduled_arms, len(blocks))
    for block, (task, replicate) in enumerate(blocks, 1):
        # A block holds every arm/level cell the block's replicate reaches. The
        # arm order comes from a balanced cycle, so no position correlates with
        # an arm; the levels within an arm are shuffled. A level with fewer
        # replicates simply stops appearing in later blocks for that task.
        levels = [level for level in feedback_levels if replicate <= level_replicates[level]]
        rng.shuffle(levels)
        order = [(arm, level) for arm in arm_orders[block - 1] for level in levels]
        for position, (arm, level) in enumerate(order, 1):
            run_id = (
                f"{round_id}-{task['task_id']}-r{replicate:02d}-"
                f"{arm.replace('_', '-')}-{level}"
            )
            run_path = runs_dir / f"{run_id}.json"
            record = {
                "schema_version": 2,
                "run_id": run_id,
                "round_id": round_id,
                "task_id": task["task_id"],
                "target": task["target"],
                "arm": arm,
                "replicate": replicate,
                "order": position,
                "block": block,
                "feedback_level": level,
                "shared_package": os.path.relpath(task["snapshot"], run_path.parent),
                "task_patch": os.path.relpath(task["patch"], run_path.parent),
                "plugin_dir": os.path.relpath(
                    plugin_records[level][arm]["path"], run_path.parent
                ),
                "plugin_manifest_sha256": plugin_records[level][arm]["manifest_sha256"],
                "plugin_tree_sha256": plugin_records[level][arm]["tree_sha256"],
                "initial_tree_sha256": task["initial_tree_sha256"],
                "mutant_manifest_sha256": mutant_digests[task["task_id"]],
                "experiment_config_sha256": config_sha256,
                "controller_harness_sha256": controller_harness_sha256,
                "move_flow_sha256": move_flow_sha256,
                "required_contract_categories": task["categories"],
                "prove_timeout_seconds": task.get("prove_timeout_seconds"),
                "package_relpath": ".",
                "allowed_edit_paths": ["sources/**/*.move"],
            }
            write_json(run_path, record)
            # Fail generation immediately if controller deserialization disagrees.
            RunSpec.load(run_path).resolve_paths(run_path)
            runs.append(record)

    manifest = {
        "schema_version": 1,
        "pilot_status": "scheduled_development_round",
        "round_id": round_id,
        "source_commit": source_commit,
        "experiment_config_sha256": config_sha256,
        "controller_harness_sha256": controller_harness_sha256,
        "controller_prompts_sha256": controller_prompts_sha256,
        "move_flow_sha256": move_flow_sha256,
        "randomization_seed": seed,
        "replicates": max_replicates,
        "replicates_by_level": level_replicates,
        "blocks": len(blocks),
        "runs": len(runs),
        "task_count": len(tasks),
        "feedback_levels": feedback_levels,
        "tasks": [
            {
                key: value
                for key, value in task.items()
                if key not in {"snapshot", "patch"}
            }
            | {"snapshot": str(task["snapshot"]), "patch": str(task["patch"])}
            for task in tasks
        ],
        "plugins": {
            level: {
                arm: {
                    key: str(value) if isinstance(value, Path) else value
                    for key, value in record.items()
                }
                for arm, record in arm_records.items()
            }
            for level, arm_records in plugin_records.items()
        },
        "scoring_mode": "core" if mutants_root is None else "reference_mutants",
        "arms": list(scheduled_arms),
    }
    manifest["schedule_sha256"] = hashlib.sha256(canonical_json(runs)).hexdigest()
    write_json(output_dir / "pilot-manifest.json", manifest)
    return manifest


def _balanced_arm_orders(
    rng: random.Random, arms: Sequence[str], count: int
) -> list[tuple[str, ...]]:
    """An arm order per block, every permutation used equally often.

    Position-dependent effects -- cache warmth, provider drift -- then cannot
    correlate with an arm. Each cycle through all permutations is shuffled
    afresh, as `schedule.py` does.
    """
    permutations = list(itertools.permutations(arms))
    orders: list[tuple[str, ...]] = []
    while len(orders) < count:
        cycle = list(permutations)
        rng.shuffle(cycle)
        orders.extend(cycle)
    return orders[:count]


def _require_committed_corpus(package: Path, recorded: dict[str, str] | None) -> None:
    """Refuse to schedule a package that is not the committed corpus.

    The manifest records a digest per generated source file, but only
    `corpus-v3/build.py --verify` read it. A round snapshots whatever is on
    disk and hashes the copy, so a locally altered package would have been
    scheduled with a self-consistent snapshot hash and measured as if it were
    the corpus. The comparison is the one `build.py --verify` makes.

    A manifest that records no digests makes no reproducibility claim to
    enforce (corpus-v1 pins its sources by commit instead), and the audit's
    `initial_tree_sha256` still fixes what a round actually ran.
    """
    # A symlink is never part of the committed corpus, and one pointing at a
    # directory is not descended into by the digest walk below, so it would
    # carry an arbitrary tree into the snapshot past the digest check.
    symlinks = sorted(
        path.relative_to(package).as_posix() for path in package.rglob("*") if path.is_symlink()
    )
    if symlinks:
        raise ValueError("package contains symlinks, which the corpus does not: " + ", ".join(symlinks))
    if not recorded:
        return
    sources = package / "sources"
    actual = {
        path.relative_to(sources).as_posix(): hashlib.sha256(path.read_bytes()).hexdigest()
        for path in sorted(sources.rglob("*.move"))
    }
    drifted = sorted(
        name for name in set(recorded) | set(actual) if recorded.get(name) != actual.get(name)
    )
    if drifted:
        raise ValueError(
            "package differs from the committed corpus: " + ", ".join(drifted)
        )


def _corpus_tasks(
    manifest_path: Path,
    snapshots_dir: Path,
    patches_dir: Path,
    sample_ids: Sequence[str] | None,
) -> list[dict[str, Any]]:
    """Schedule tasks from a corpus manifest.

    Every sample shares one package and differs only in its target, so the
    snapshot is taken once and reused. That mirrors how the corpus is actually
    verified, and keeps a round's packages identical across its tasks.
    """
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    records = manifest["records"]
    # A sample the screen did not clear is not a task. Scheduling it would spend
    # a cell on a known defect and report the failure as a specification result,
    # so the manifest's own status is the gate rather than the operator's memory.
    blocked = {
        record["task_id"]: record["screening_status"]
        for record in records
        if record.get("screening_status", "ready") != "ready"
    }
    if sample_ids is not None:
        named = sorted(set(sample_ids) & set(blocked))
        if named:
            raise ValueError(
                "sample(s) named but not cleared by screening: "
                + ", ".join(f"{task} ({blocked[task]})" for task in named)
            )
    records = [record for record in records if record["task_id"] not in blocked]
    if sample_ids is not None:
        wanted = set(sample_ids)
        unknown = sorted(wanted - {record["task_id"] for record in records})
        if unknown:
            raise ValueError(f"unknown corpus sample(s): {', '.join(unknown)}")
        records = [record for record in records if record["task_id"] in wanted]
    if not records:
        raise ValueError("a corpus round needs at least one sample")

    package = (manifest_path.parent / "package").resolve()
    if not (package / "Move.toml").is_file():
        raise FileNotFoundError(package / "Move.toml")
    _require_committed_corpus(package, manifest.get("generated_file_sha256"))
    snapshot = snapshots_dir / "corpus"
    if snapshot.exists():
        shutil.rmtree(snapshot)
    shutil.copytree(package, snapshot, ignore=shutil.ignore_patterns("build"))
    digest = tree_hash(snapshot)

    tasks: list[dict[str, Any]] = []
    for record in records:
        require_plain_name(record["task_id"], "task_id")
        patch = patches_dir / f"{record['task_id']}.patch"
        patch.write_text("", encoding="utf-8")
        tasks.append(
            {
                "name": record["task_id"],
                "task_id": record["task_id"],
                "target": record["target"],
                "categories": tuple(record["required_contract_categories"]),
                "snapshot": snapshot,
                "patch": patch,
                "initial_tree_sha256": digest,
                "source_sha256": digest,
                "prove_timeout_seconds": record.get("prove_timeout_seconds"),
                "corpus": manifest.get("corpus"),
                "provenance": record.get("provenance"),
                "feature_strata": record.get("feature_strata"),
            }
        )
    return tasks


def _move_flow_sha256() -> str | None:
    """Hash the `move-flow` build a round will run against, when resolvable."""
    binary = shutil.which("move-flow")
    return sha256_file(Path(binary).resolve()) if binary else None


@dataclass(frozen=True)
class RoundShape:
    """Expected size of a scheduled round, as recorded by its manifest."""

    runs: int
    blocks: int
    replicates: int
    task_count: int
    feedback_levels: tuple[str, ...]
    replicates_by_level: Mapping[str, int] = field(default_factory=dict)
    # A round may study a subset of the arms; one scheduled before that was
    # possible declares none and covered all of them.
    arms: tuple[str, ...] = ARMS

    def cells_in_block(self, replicate: int) -> int:
        """Arm/level cells a block carries, given a level may be a control."""
        levels = self.feedback_levels or ("",)
        reached = sum(
            1
            for level in levels
            if replicate <= self.replicates_by_level.get(level, self.replicates)
        )
        return len(self.arms) * reached


def load_round_shape(schedule_dir: Path) -> RoundShape:
    """Read and validate the expected size of a scheduled pilot round.

    Round size varies by purpose: a baseline slice uses one replicate while a
    variance-bearing round uses several. Dispatch, preflight, and audit all read
    the size from the manifest so they agree on what a complete round is.
    """
    manifest = json.loads(
        (schedule_dir / "pilot-manifest.json").read_text(encoding="utf-8")
    )
    if not isinstance(manifest, dict):
        raise ValueError("pilot manifest must be a JSON object")
    values: dict[str, int] = {}
    for name in ("runs", "blocks", "replicates", "task_count"):
        value = manifest.get(name)
        if not isinstance(value, int) or value < 1:
            raise ValueError(f"pilot manifest lacks a positive `{name}`")
        values[name] = value
    # A round scheduled before the feedback factor declares no levels. Claiming
    # one on its behalf would label its runs with an apparatus it never chose.
    levels = manifest.get("feedback_levels", [])
    if not isinstance(levels, list) or any(
        level not in FEEDBACK_LEVELS for level in levels
    ):
        raise ValueError("pilot manifest lacks a valid `feedback_levels` list")
    if values["blocks"] != values["task_count"] * values["replicates"]:
        raise ValueError(
            f"pilot manifest declares {values['blocks']} blocks for "
            f"{values['task_count']} tasks and {values['replicates']} replicates"
        )
    # A level may be scheduled as a control with fewer replicates than the level
    # under study, so a block holds only the cells its replicate reaches and the
    # run count follows the per-level counts rather than a uniform block size.
    by_level = manifest.get("replicates_by_level")
    if by_level is None:
        by_level = {level: values["replicates"] for level in levels} or {
            "": values["replicates"]
        }
    if not isinstance(by_level, dict) or any(
        not isinstance(count, int) or count < 1 for count in by_level.values()
    ):
        raise ValueError("pilot manifest has an invalid `replicates_by_level`")
    if levels and sorted(by_level) != sorted(levels):
        raise ValueError("`replicates_by_level` disagrees with `feedback_levels`")
    if values["replicates"] != max(by_level.values()):
        raise ValueError(
            "pilot manifest `replicates` must be the widest per-level count"
        )
    # A round may study a subset of the arms; one scheduled before that was
    # possible declares none and covered all of them.
    arms = manifest.get("arms") or list(ARMS)
    if not isinstance(arms, list) or not arms or not set(arms) <= set(ARMS):
        raise ValueError("pilot manifest has an invalid `arms` list")
    expected = len(arms) * values["task_count"] * sum(by_level.values())
    if values["runs"] != expected:
        raise ValueError(
            f"pilot manifest declares {values['runs']} runs; "
            f"{values['task_count']} tasks over {len(arms)} arms and "
            f"per-level replicates {by_level} give {expected}"
        )
    return RoundShape(
        **values,
        feedback_levels=tuple(levels),
        replicates_by_level=dict(by_level),
        arms=tuple(arms),
    )


def _plugin_record(
    path: Path, arm: str, source_commit: str, feedback_level: str
) -> dict[str, Any]:
    manifest_path = path / "move-flow-manifest.json"
    value = json.loads(manifest_path.read_text(encoding="utf-8"))
    if value.get("inference_tactic") != arm:
        raise ValueError(f"plugin tactic mismatch for {arm}")
    if value.get("feedback_level") != feedback_level:
        raise ValueError(
            f"plugin for `{arm}` declares feedback level "
            f"{value.get('feedback_level')!r}, expected {feedback_level!r}"
        )
    if value.get("evaluation_mode") is not True or value.get("flow_source_commit") != source_commit:
        raise ValueError(f"plugin round identity mismatch for {arm}")
    return {
        "path": path,
        "manifest_sha256": sha256_file(manifest_path),
        "tree_sha256": tree_hash(path),
    }


def plugins_from_file(path: Path) -> dict[str, dict[str, Path]]:
    """Load per-level plugin paths, resolving relative entries beside the file.

    The map is `{feedback_level: {arm: path}}`. A flat `{arm: path}` map is read
    as a single `acceptance` level so that a round which does not vary the
    apparatus needs no extra nesting.
    """
    path = path.resolve()
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict) or not value:
        raise ValueError(f"plugin map must be a non-empty JSON object: {path}")
    if all(isinstance(entry, str) for entry in value.values()):
        value = {"acceptance": value}
    result: dict[str, dict[str, Path]] = {}
    for level, arm_plugins in value.items():
        if not isinstance(arm_plugins, dict):
            raise ValueError(f"plugin map for {level!r} must be an object")
        resolved: dict[str, Path] = {}
        for arm, entry in arm_plugins.items():
            if not isinstance(entry, str):
                raise ValueError(f"plugin path for {level!r}/{arm!r} must be a string")
            plugin = Path(entry)
            resolved[arm] = plugin if plugin.is_absolute() else path.parent / plugin
        result[level] = resolved
    return result


def _resolve_mutant_manifests(
    tasks: Sequence[Mapping[str, Any]], mutants_root: Path | None
) -> dict[str, str]:
    """Map each task to the digest of its hidden mutant manifest.

    Without `mutants_root` every task takes the all-zero sentinel, which the
    controller reads as core scoring. With it, every task must have a readable
    manifest carrying at least one mutant: a round that asks for strict scoring
    and silently falls back to core scoring would report `strict_success: false`
    for an apparatus reason and look like a specification result.
    """
    if mutants_root is None:
        return {task["task_id"]: NO_MUTANTS for task in tasks}
    digests: dict[str, str] = {}
    missing: list[str] = []
    for task in tasks:
        manifest = mutants_root / task["task_id"] / "mutants.json"
        if not manifest.is_file():
            missing.append(task["task_id"])
            continue
        if not json.loads(manifest.read_text(encoding="utf-8")).get("mutants"):
            missing.append(task["task_id"])
            continue
        digests[task["task_id"]] = sha256_file(manifest)
    if missing:
        raise FileNotFoundError(
            "strict scoring needs TASK_ID/mutants.json under "
            f"{mutants_root} for every task; missing or empty: {', '.join(sorted(missing))}"
        )
    return digests


def _parse_replicates(value: str) -> int | Mapping[str, int]:
    """Read `--replicates` as a single count or one count per feedback level."""
    if "=" not in value:
        return int(value)
    counts: dict[str, int] = {}
    for entry in value.split(","):
        level, _, count = entry.partition("=")
        counts[level.strip()] = int(count)
    return counts


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--corpus-manifest",
        type=Path,
        required=True,
        help="corpus manifest naming the round's samples and their package",
    )
    parser.add_argument("--plugins", type=Path, required=True, help="JSON arm-to-plugin-path map")
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument("--source-commit", required=True)
    parser.add_argument("--experiment-config", type=Path, required=True)
    parser.add_argument("--round-id", default="pilot-001")
    parser.add_argument(
        "--replicates",
        default="3",
        help="replicate count, either a number or LEVEL=N,LEVEL=N per feedback level",
    )
    parser.add_argument(
        "--tasks",
        nargs="+",
        help="restrict the round to these corpus sample ids (default: all)",
    )
    parser.add_argument(
        "--mutants-root",
        type=Path,
        help="directory of TASK_ID/mutants.json enabling strict scoring; every "
        "scheduled task must have one",
    )
    args = parser.parse_args()
    result = build_pilot(
        args.corpus_manifest.resolve(),
        plugins_from_file(args.plugins),
        args.output_dir.resolve(),
        args.source_commit,
        args.experiment_config.resolve(),
        replicates=_parse_replicates(args.replicates),
        round_id=args.round_id,
        task_names=args.tasks,
        mutants_root=args.mutants_root.resolve() if args.mutants_root else None,
    )
    print(
        json.dumps(
            {
                "blocks": result["blocks"],
                "runs": result["runs"],
                "scoring_mode": result["scoring_mode"],
            },
            sort_keys=True,
        )
    )


if __name__ == "__main__":
    main()
