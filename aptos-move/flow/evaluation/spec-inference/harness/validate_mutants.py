"""Establish which mutants are essential, against a reference specification.

A mutant is *essential* when a complete specification of the target rejects it.
That is a property of the mutant and the implementation, not of any arm: it says
the mutant changes behavior a correct contract is obliged to describe. Any
complete specification answers the question, so the reference may be authored
however is convenient.

A mutant that survives a complete reference is not essential. It either does not
change observable behavior, or it changes behavior outside the contract the task
requires; scoring an arm against it would punish a correct specification.
"""

from __future__ import annotations

import argparse
import asyncio
import json
import shutil
import tempfile
from pathlib import Path
from typing import Any

from .artifacts import tree_hash, write_json
from .config import ExperimentConfig
from .judge import render_command, run_command
from .mutants import run_mutant_cases


async def validate_mutants(
    config: ExperimentConfig,
    reference: Path,
    baseline: Path,
    target: str,
    mutant_manifest: Path,
    timeout_seconds: int,
) -> dict[str, Any]:
    manifest = json.loads(mutant_manifest.read_text(encoding="utf-8"))
    cases = manifest.get("mutants", [])
    if not cases:
        raise ValueError(f"{mutant_manifest} lists no mutants")
    scratch = tempfile.mkdtemp(prefix="move-inference-reference-")
    # An unproved reference cannot certify anything: every mutant would appear
    # killed by the same failure the reference already has.
    for stage, command in (("compile", config.compile_command), ("prove", config.prove_command)):
        outcome = await run_command(
            render_command(
                command,
                package=reference,
                baseline=reference,
                target=target,
                timeout=timeout_seconds,
                output=Path(scratch) / "unused-reference.json",
            ),
            timeout_seconds=max(120, timeout_seconds * 4),
        )
        if not outcome.succeeded:
            raise ValueError(
                f"reference specification does not {stage}: {outcome.diagnostics[:400]}"
            )
    # A vacuous reference certifies nothing. If its assumptions are
    # contradictory the prover succeeds on any postcondition, so every mutant
    # would appear to survive and every mutant would be marked non-essential.
    inconsistency = await run_command(
        render_command(
            [*config.prove_command, "--check-inconsistency"],
            package=reference,
            baseline=reference,
            target=target,
            timeout=timeout_seconds,
            output=Path(scratch) / "unused-inconsistency.json",
        ),
        timeout_seconds=max(120, timeout_seconds * 4),
    )
    if "inconsistent assumption" in inconsistency.diagnostics:
        raise ValueError(
            "reference specification is vacuous: the prover reports an inconsistent "
            "assumption, so it proves every postcondition and certifies no mutant"
        )
    results = await run_mutant_cases(
        config, reference, baseline, target, cases, timeout_seconds
    )
    shutil.rmtree(scratch, ignore_errors=True)
    by_id = {result["mutant_id"]: result for result in results}
    for case in cases:
        result = by_id[case["mutant_id"]]
        validated = case.setdefault("validated", {})
        validated["applies"] = True
        validated["compiles"] = result["outcome"] != "compile_failure"
        validated["killed_by_reference"] = result["killed"]
        validated["outcome"] = result["outcome"]
        case["essential"] = result["killed"]
    # The whole reference tree, not its `Move.toml`: every reference package is
    # a copy of the same corpus package with one module swapped, so hashing the
    # manifest file gives every reference the same digest and identifies
    # nothing. Essentiality is only reproducible if this pins the specification
    # the mutants were validated against.
    manifest["reference_sha256"] = tree_hash(reference)
    write_json(mutant_manifest, manifest)
    killed = sum(1 for case in cases if case["essential"])
    return {
        "schema_version": 1,
        "target": target,
        "manifest": str(mutant_manifest),
        "mutants": len(cases),
        "essential": killed,
        "not_essential": [
            case["mutant_id"] for case in cases if not case["essential"]
        ],
        "results": results,
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--config", type=Path, required=True)
    parser.add_argument(
        "--reference",
        type=Path,
        required=True,
        help="package carrying a complete specification of the target",
    )
    parser.add_argument(
        "--baseline",
        type=Path,
        required=True,
        help="pristine package the reference was written against",
    )
    parser.add_argument("--target", required=True)
    parser.add_argument("--mutants", type=Path, required=True)
    parser.add_argument("--timeout", type=int, default=40)
    args = parser.parse_args()
    summary = asyncio.run(
        validate_mutants(
            ExperimentConfig.load(args.config.resolve()),
            args.reference.resolve(),
            args.baseline.resolve(),
            args.target,
            args.mutants.resolve(),
            args.timeout,
        )
    )
    print(json.dumps({k: v for k, v in summary.items() if k != "results"}, sort_keys=True))


if __name__ == "__main__":
    main()
