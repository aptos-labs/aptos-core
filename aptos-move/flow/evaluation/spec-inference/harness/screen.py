"""Treatment-blind compatibility and runtime screening for prepared tasks."""

from __future__ import annotations

import argparse
import asyncio
import hashlib
import json
import os
import tempfile
from dataclasses import asdict
from pathlib import Path
from typing import Any

from .identifiers import require_plain_name
from .artifacts import canonical_json, load_object, sha256_file, tree_hash, write_json
from .compatibility import (
    COMPATIBILITY_SCHEMA_VERSION,
    apparatus_reached_a_verdict,
    binary_sha256,
    admission,
    check_compatibility,
    prove_reference,
    tool_executables,
)
from .config import ExperimentConfig
from .materialize import materialize_task


async def screen_corpus(
    manifest_path: Path,
    experiment_config_path: Path,
    corpus_config_path: Path,
    results_dir: Path,
    output_path: Path,
    resume: bool = False,
    screening_ledger_path: Path | None = None,
) -> dict[str, Any]:
    manifest = load_object(manifest_path)
    corpus_config = load_object(corpus_config_path)
    threshold = int(corpus_config["compatibility_threshold_seconds"])
    config = ExperimentConfig.load(experiment_config_path)
    ledger_entries = _load_ledger(
        screening_ledger_path,
        manifest["source_commit"],
        threshold,
        tool_executables(config),
    )
    selected = [
        record
        for record in manifest["records"]
        if record["selection_status"] == "selected"
    ]
    expected = sum(
        count
        for quotas in (manifest.get("quotas") or {}).values()
        for count in quotas.values()
    )
    # The corpus size is the sum of the selection quotas, not a constant: a
    # corpus that de-duplicates shapes is smaller than one that does not, and a
    # hard-coded thirty silently outlaws every size but the first one chosen.
    if expected and len(selected) != expected:
        raise ValueError(f"expected {expected} selected tasks, got {len(selected)}")
    results_dir.mkdir(parents=True, exist_ok=resume)
    summaries = []
    evidence = []
    shared_packages = set()
    for index, record in enumerate(selected, 1):
        require_plain_name(record["task_id"], "task_id")
        shared = (manifest_path.parent / record["shared_package_path"]).resolve()
        shared_packages.add(shared)
        patch = (manifest_path.parent / record["preparation_patch"]).resolve()
        with tempfile.TemporaryDirectory(
            prefix=f"move-inference-screen-{record['task_id']}-"
        ) as temporary:
            package = Path(temporary) / "package"
            materialize_task(shared, patch, package, record["prepared_sha256"])
            result_path = results_dir / f"{record['task_id']}.json"
            ledger_entry = ledger_entries.get(record["task_id"])
            if ledger_entry is not None:
                _validate_ledger_entry(record, ledger_entry)
                if ledger_entry["passed"] is not True:
                    raise ValueError(
                        f"selected task was previously excluded: {record['task_id']}"
                    )
                record["compatibility_screen"], apparatus_ok = (
                    await _screen_from_ledger(
                        config, shared, record, threshold, ledger_entry
                    )
                )
            else:
                result = (
                    _resume_result(
                        result_path, package, record, threshold, tool_executables(config)
                    )
                    if resume
                    else None
                )
                if result is None:
                    result = await check_compatibility(
                        config,
                        package,
                        record["package_module_target"],
                        threshold,
                    )
                    # The shared package carries every target's reference;
                    # the preparation patch only removes this target's. So
                    # the reference to prove is the target in the shared tree.
                    result["reference_proof"] = await prove_reference(
                        config, shared, reference_targets(record), threshold
                    )
                    write_json(result_path, result)
                verdict = admission(result)
                apparatus_ok = apparatus_reached_a_verdict(result)
                record["compatibility_screen"] = {
                    "passed": verdict["passed"],
                    "reason": verdict["reason"],
                    "well_formed": verdict["well_formed"],
                    "reference_proved": verdict["reference_proved"],
                    "wp_hard": verdict["wp_hard"],
                    "wp_failure_kind": verdict["wp_failure_kind"],
                    "threshold_seconds": threshold,
                    "threshold_exceeded_stage": result["threshold_exceeded_stage"],
                    "total_duration_ms": result["total_duration_ms"],
                    "stage_duration_ms": {
                        name: result[name]["duration_ms"] if result[name] else None
                        for name in (
                            "compile",
                            "wp_inference",
                            "enriched_compile",
                            "prover",
                        )
                    },
                    "result_path": os.path.relpath(result_path, output_path.parent),
                    "result_sha256": sha256_file(result_path),
                    # The identity `_load_ledger` compares against, so a
                    # ledger built from these verdicts is reusable by the
                    # same apparatus.
                    "tool_executables": tool_executables(config),
                    "origin": "executed",
                }
        summary = {
            "index": index,
            "task_id": record["task_id"],
            **record["compatibility_screen"],
        }
        summaries.append(summary)
        evidence.append(
            {
                "task_id": record["task_id"],
                "target": record["package_module_target"],
                "passed": summary["passed"],
                "apparatus_ok": apparatus_ok,
                # The prepared tree this verdict is about.
                "reference_sha256": record["prepared_sha256"],
            }
        )
        print(
            json.dumps(
                {
                    "index": index,
                    "task_id": record["task_id"],
                    "passed": summary["passed"],
                    "reason": summary["reason"],
                    "wp_hard": summary.get("wp_hard"),
                    "threshold_exceeded_stage": summary["threshold_exceeded_stage"],
                    "total_duration_ms": summary["total_duration_ms"],
                },
                sort_keys=True,
            ),
            flush=True,
        )

    failures = [summary for summary in summaries if not summary["passed"]]
    if len(shared_packages) != 1:
        raise ValueError("every selected task must share one package")
    (shared,) = shared_packages
    result_manifest = {
        **manifest,
        "corpus_status": "screened" if not failures else "screen_failed",
        "compatibility_screen": {
            "schema_version": 1,
            "threshold_seconds_per_stage": threshold,
            "experiment_config_sha256": sha256_file(experiment_config_path),
            "corpus_config_sha256": sha256_file(corpus_config_path),
            "input_manifest_sha256": sha256_file(manifest_path),
            "passed": len(summaries) - len(failures),
            "failed": len(failures),
            "excluded_for_timeout": sum(
                failure["reason"] == "compatibility_timeout" for failure in failures
            ),
            "requires_fix_or_rerun": sum(
                failure["reason"] != "compatibility_timeout" for failure in failures
            ),
            "failures": failures,
            "wp_hard": [
                summary["task_id"] for summary in summaries if summary.get("wp_hard")
            ],
            **screening_evidence(config, shared, evidence),
        },
        "records": manifest["records"],
    }
    write_json(output_path, result_manifest)
    return result_manifest



def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--experiment-config", type=Path, required=True)
    parser.add_argument("--corpus-config", type=Path, required=True)
    parser.add_argument("--results-dir", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--resume", action="store_true")
    parser.add_argument("--screening-ledger", type=Path)
    args = parser.parse_args()
    result = asyncio.run(
        screen_corpus(
            args.manifest.resolve(),
            args.experiment_config.resolve(),
            args.corpus_config.resolve(),
            args.results_dir.resolve(),
            args.output.resolve(),
            args.resume,
            args.screening_ledger.resolve() if args.screening_ledger else None,
        )
    )
    print(json.dumps(result["compatibility_screen"], sort_keys=True))
    if result["compatibility_screen"]["failed"]:
        raise SystemExit(
            "one or more tasks timed out or exposed an implementation/infrastructure failure"
        )


def reference_targets(record: dict[str, Any]) -> list[str]:
    """The task's functions as prover targets.

    A module task is screened under the same module target the scheduler runs.
    Deriving a narrower set from manifest-provided function names would let the
    screen prove a sibling while the round executes the whole module.
    """
    target = record["package_module_target"]
    if record.get("granularity") == "module":
        return [target]
    return [target]


async def _screen_from_ledger(
    config: ExperimentConfig,
    shared: Path,
    record: dict[str, Any],
    threshold: int,
    ledger_entry: dict[str, Any],
) -> tuple[dict[str, Any], bool]:
    """Reuse compatibility evidence while re-proving the current reference."""
    reference_proof = await prove_reference(
        config, shared, reference_targets(record), threshold
    )
    reference_proved = bool(reference_proof["proved"])
    return (
        {
            **ledger_entry,
            "passed": reference_proved,
            "reason": None if reference_proved else "reference_unproved",
            "reference_proved": reference_proved,
            "origin": "cumulative_screening_ledger",
        },
        bool(reference_proof["vacuity_checked"]),
    )


def screening_evidence(
    config: ExperimentConfig, package: Path, results: list[dict[str, Any]]
) -> dict[str, Any]:
    """Identify what a screen measured and with what, for a round to check.

    The scheduler admits a task only against evidence that names the same
    package tree and the same apparatus the round will run: a verdict depends
    on the binary and the configuration, and recording only a command name
    would let evidence produced by one build clear a round executed by another.
    `screen_v3` records the same block.
    """
    return {
        "package_tree_sha256": tree_hash(package),
        "tools": {
            "move_flow": config.check_candidate_command[:1],
            "move_flow_sha256": binary_sha256(config.check_candidate_command[0]),
            "stage_executables": tool_executables(config),
            "experiment_config_sha256": hashlib.sha256(
                canonical_json(asdict(config))
            ).hexdigest(),
            "model_independent": True,
        },
        "results": results,
    }


def _resume_result(
    path: Path,
    package: Path,
    record: dict[str, Any],
    threshold: int,
    tools: dict[str, Any],
) -> dict[str, Any] | None:
    """A result from an interrupted screen, if it is still this screen's.

    The toolchain is part of the identity for the reason the round's own
    apparatus check exists: a verdict from another build says nothing about
    this one, and resuming across a rebuild would publish the new binary's
    name over the old binary's measurements.
    """
    if tree_hash(package) != record["prepared_sha256"]:
        raise ValueError(f"prepared package hash mismatch during resume: {package}")
    if not path.is_file():
        return None
    result = load_object(path)
    expected = {
        "schema_version": COMPATIBILITY_SCHEMA_VERSION,
        "package_sha256": record["prepared_sha256"],
        "target": record["package_module_target"],
        "threshold_seconds": threshold,
    }
    if any(result.get(key) != value for key, value in expected.items()):
        raise ValueError(f"resume result identity mismatch: {path}")
    if result.get("tool_executables") != tools:
        raise ValueError(
            f"{path} was screened by a different toolchain than this screen runs; "
            "re-screen rather than resume"
        )
    return result


def _load_ledger(
    path: Path | None,
    source_commit: str,
    threshold: int,
    tool_executables: dict[str, dict[str, str]],
) -> dict[str, dict[str, Any]]:
    if path is None:
        return {}
    ledger = load_object(path)
    if ledger.get("schema_version") != 1 or ledger.get("source_commit") != source_commit:
        raise ValueError("screening ledger schema or source commit mismatch")
    entries = {
        entry["task_id"]: entry
        for entry in ledger.get("entries", [])
        if entry.get("tool_executables") == tool_executables
    }
    for entry in entries.values():
        if entry.get("threshold_seconds") != threshold:
            raise ValueError(
                f"screening threshold mismatch for {entry.get('task_id')}"
            )
    return entries


def _validate_ledger_entry(record: dict[str, Any], entry: dict[str, Any]) -> None:
    expected = {
        "package_module_target": record["package_module_target"],
        "source_sha256": record["source_sha256"],
    }
    if any(entry.get(key) != value for key, value in expected.items()):
        raise ValueError(f"screening identity mismatch for {record['task_id']}")


if __name__ == "__main__":
    main()
