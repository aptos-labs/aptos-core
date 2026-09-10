"""Treatment-blind compatibility screening for the corpus-v3.2 package.

`screen.py` drives the corpus-v1.1 shape: one overlay patch and snapshot per
sample, thirty selected records, a `source_commit` at the manifest root.
corpus-v3.2 is a single package with targets named inside it, so it needs its own
driver rather than a manifest bent to fit the other one.

The screen is blind to any arm: for each target it compiles the unmodified
package, runs inference, recompiles with what inference produced, and proves --
which is what `check_compatibility` does. Nothing here consults a session, a
model or a result, so a target's membership can never be chosen by how well an
arm did on it.

Writes one result per target plus a summary, so `screening_status` in the
manifest is evidenced rather than asserted:

    python3 -m harness.screen_v3 \\
      --manifest corpus-v3.2/manifest.json \\
      --experiment-config config/default.json \\
      --corpus-config config/corpus.json \\
      --results-dir corpus-v3.2/screening \\
      --output corpus-v3.2/screening/summary.json
"""

from __future__ import annotations

import argparse
import asyncio
import json
import shutil
import hashlib
import tempfile
import time
from pathlib import Path
from typing import Any

from .artifacts import canonical_json, load_object, sha256_file, tree_hash, write_json
from .compatibility import (
    apparatus_reached_a_verdict,
    binary_sha256,
    check_compatibility,
    is_well_formed,
    prove_reference,
    stage_identity,
    tool_executables,
)
from dataclasses import asdict

from .config import ExperimentConfig
from .judge import render_command, run_command
from .identifiers import require_plain_name


async def screen_corpus_v3(
    manifest_path: Path,
    experiment_config_path: Path,
    corpus_config_path: Path,
    results_dir: Path,
    output_path: Path,
    selected_only: bool = True,
) -> dict[str, Any]:
    manifest = load_object(manifest_path)
    corpus_config = load_object(corpus_config_path)
    threshold = int(corpus_config["compatibility_threshold_seconds"])
    config = ExperimentConfig.load(experiment_config_path)
    package = (manifest_path.parent / "package").resolve()
    if not (package / "Move.toml").is_file():
        raise SystemExit(f"corpus package is missing Move.toml: {package}")

    records = [
        record
        for record in manifest["records"]
        # `screening_status` records why a target is out -- `excluded_prover_defect`,
        # `blocked_solver_cost` -- and only `ready` ones have a reference to prove.
        if record["screening_status"] == "ready"
        and (not selected_only or record.get("round_selection") == "selected")
    ]
    if not records:
        raise SystemExit("no records to screen; check `round_selection`")

    results_dir.mkdir(parents=True, exist_ok=True)
    summaries: list[dict[str, Any]] = []
    for index, record in enumerate(records, 1):
        task_id = require_plain_name(record["task_id"], "task_id")
        started = time.monotonic()
        result = await check_compatibility(
            config, package, record["target"], threshold
        )
        reference = await _prove_reference(config, manifest_path, record, threshold)
        # `check_compatibility` also requires that WP's unaided output verify.
        # That is the right bar for a prepared corpus-v1.1 sample, where the
        # dependency contracts have to be complete before a target is asked of
        # anyone. Here the target *is* the task, and a target WP cannot do
        # unaided is the interesting kind -- admitting only what WP already
        # solves would keep the easy member of every family. So admission asks
        # whether the task is well-formed and provable, and WP-hardness is
        # recorded as a property of the task.
        well_formed = is_well_formed(result)
        apparatus_ok = apparatus_reached_a_verdict(result)
        wp_hard = not result["passed"]
        summary = {
            "schema_version": 2,
            "task_id": task_id,
            "target": record["target"],
            "passed": bool(well_formed and reference["proved"]),
            "well_formed": well_formed,
            "reference_proved": reference["proved"],
            # The package is always assembled beneath the corpus root.  Its
            # digest identifies the content; a corpus-relative label identifies
            # the package without publishing the machine's checkout path.
            "reference_package": str(
                Path(reference["package"]).relative_to(manifest_path.parent)
            ),
            "reference_sha256": reference["reference_sha256"],
            # WP alone does not reach a verifying contract: a task property, not
            # a defect. See corpus-v3.2/README.md and issue #20490 for one cause.
            "wp_hard": wp_hard,
            "wp_failure_kind": result.get("failure_kind") if wp_hard else None,
            "apparatus_ok": apparatus_ok,
            "exceeded_stage": result.get("exceeded_stage"),
            "wall_seconds": round(time.monotonic() - started, 1),
            "threshold_seconds": threshold,
        }
        # Compiler and prover diagnostics include source frames. This corpus is
        # generated partly from private Etna source, so committing the raw
        # command result would publish those excerpts. The durable screening
        # record keeps the verdict, timing, failure class, reference digest,
        # and tool identity without the diagnostic payload.
        write_json(results_dir / f"{task_id}.json", summary)
        summaries.append(summary)
        if summary["passed"]:
            state = "pass" + (" (wp-hard)" if wp_hard else "")
        else:
            state = "FAIL (not well-formed)" if not well_formed else "FAIL (reference does not prove)"
        print(f"[{index}/{len(records)}] {task_id}: {state} in {summary['wall_seconds']}s", flush=True)

    failed = [s for s in summaries if not s["passed"]]
    report = {
        "schema_version": 1,
        "corpus": manifest.get("corpus"),
        # What was screened, so a later corpus change invalidates this record
        # rather than silently inheriting it.
        "package_tree_sha256": tree_hash(package),
        "source_commit": manifest["provenance"]["aptos_core"]["commit"],
        "threshold_seconds": threshold,
        # Screening runs the prover and WP, so its verdicts depend on the
        # binary and on the experiment configuration. Recording only the
        # command name lets evidence produced by one apparatus clear a round
        # executed by another, which the corpus digest cannot notice because
        # the corpus did not change.
        "tools": {
            "move_flow": config.check_candidate_command[:1],
            "move_flow_sha256": binary_sha256(config.check_candidate_command[0]),
            # Screening also drives the compile, inference and prove commands,
            # which the config may point at different executables. Recording
            # only the checker leaves those unpinned.
            "stage_executables": {
                name: stage_identity(entry)
                for name, entry in tool_executables(config).items()
            },
            "experiment_config_sha256": hashlib.sha256(
                canonical_json(asdict(config))
            ).hexdigest(),
            "model_independent": True,
        },
        "screened": len(summaries),
        "passed": len(summaries) - len(failed),
        "failed": [s["task_id"] for s in failed],
        "wp_hard": [s["task_id"] for s in summaries if s["wp_hard"]],
        "results": summaries,
    }
    write_json(output_path, report)

    return report



async def _prove_reference(
    config: ExperimentConfig,
    manifest_path: Path,
    record: dict[str, Any],
    threshold: int,
) -> dict[str, Any]:
    """Prove the task's reference contract, the evidence that it is solvable.

    Assembled by `build_references.py` into a gitignored tree, so a missing one
    is a preparation error rather than a property of the task.
    """
    # The manifest supplies this, so it is input: an absolute segment would
    # discard the reference root and `..` would climb out of it, and the proof
    # that came back would still be recorded as this task's solvability
    # evidence.
    module = require_plain_name(record["module"].split("::")[-1], "module")
    package = manifest_path.parent / "references" / "build" / module
    if not (package / "Move.toml").is_file():
        raise SystemExit(
            f"no assembled reference for {record['task_id']} at {package}; "
            "run `python3 corpus-v3.2/build_references.py` first"
        )
    return await prove_reference(config, package, [record["target"]], threshold)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--experiment-config", type=Path, required=True)
    parser.add_argument("--corpus-config", type=Path, required=True)
    parser.add_argument("--results-dir", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument(
        "--all-ready",
        action="store_true",
        help="screen every ready target, not only the round's selection",
    )
    args = parser.parse_args()
    report = asyncio.run(
        screen_corpus_v3(
            args.manifest.resolve(),
            args.experiment_config.resolve(),
            args.corpus_config.resolve(),
            args.results_dir.resolve(),
            args.output.resolve(),
            selected_only=not args.all_ready,
        )
    )
    print(json.dumps({k: report[k] for k in ("screened", "passed", "failed")}, indent=1))
    if report["failed"]:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
