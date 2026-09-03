"""Treatment-blind compatibility and runtime screening for prepared tasks."""

from __future__ import annotations

import argparse
import asyncio
import json
import os
import tempfile
from pathlib import Path
from typing import Any

from .identifiers import require_plain_name
from .artifacts import load_object, sha256_file, tree_hash, write_json
from .compatibility import check_compatibility, tool_executables
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
    if len(selected) != 30:
        raise ValueError(f"expected 30 selected tasks, got {len(selected)}")
    results_dir.mkdir(parents=True, exist_ok=resume)
    summaries = []
    for index, record in enumerate(selected, 1):
        require_plain_name(record["task_id"], "task_id")
        shared = (manifest_path.parent / record["shared_package_path"]).resolve()
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
                record["compatibility_screen"] = {
                    **ledger_entry,
                    "origin": "cumulative_screening_ledger",
                }
            else:
                result = (
                    _resume_result(result_path, package, record, threshold)
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
                    write_json(result_path, result)
                reason = result.get("failure_kind") if not result["passed"] else None
                record["compatibility_screen"] = {
                    "passed": result["passed"],
                    "reason": reason,
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
                    "tool_executables": result.get("tool_executables", {}),
                    "origin": "executed",
                }
        summary = {
            "index": index,
            "task_id": record["task_id"],
            **record["compatibility_screen"],
        }
        summaries.append(summary)
        print(
            json.dumps(
                {
                    "index": index,
                    "task_id": record["task_id"],
                    "passed": summary["passed"],
                    "reason": summary["reason"],
                    "threshold_exceeded_stage": summary["threshold_exceeded_stage"],
                    "total_duration_ms": summary["total_duration_ms"],
                },
                sort_keys=True,
            ),
            flush=True,
        )

    failures = [summary for summary in summaries if not summary["passed"]]
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


def _resume_result(
    path: Path,
    package: Path,
    record: dict[str, Any],
    threshold: int,
) -> dict[str, Any] | None:
    if tree_hash(package) != record["prepared_sha256"]:
        raise ValueError(f"prepared package hash mismatch during resume: {package}")
    if not path.is_file():
        return None
    result = load_object(path)
    expected = {
        "schema_version": 4,
        "package_sha256": record["prepared_sha256"],
        "target": record["package_module_target"],
        "threshold_seconds": threshold,
    }
    if any(result.get(key) != value for key, value in expected.items()):
        raise ValueError(f"resume result identity mismatch: {path}")
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
