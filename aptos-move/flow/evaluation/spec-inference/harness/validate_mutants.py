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
from .mutants import reached_a_verdict, run_mutant_cases


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
    # The absence of that diagnostic only means something if the check reached a
    # verdict. The prover reports an inconsistency as an error, so a vacuous
    # reference makes this command fail -- but so does a timeout, a missing
    # solver, or a bad invocation, and those produce the same silence. Treating
    # silence as a clean bill of health would let essentiality, and so the
    # strict-success score built on it, rest on a reference whose non-vacuity
    # was never established.
    if not inconsistency.succeeded:
        if inconsistency.timed_out:
            reason = "it timed out"
        elif inconsistency.infrastructure_error:
            reason = f"it could not run: {inconsistency.infrastructure_error}"
        else:
            reason = f"it exited {inconsistency.returncode}"
        raise ValueError(
            "reference specification vacuity is unproven: the inconsistency check "
            f"reached no verdict because {reason}: {inconsistency.diagnostics[:400]}"
        )
    # The reference certifies which mutants are essential, and that claim is
    # only about the *specification*: a reference whose executable code differs
    # from the baseline would be certifying mutants against different
    # behaviour, and its `reference_sha256` would carry that difference into
    # the strict scores. The authoritative comparator is the one the judge
    # uses, rather than a second implementation of the same question.
    await _require_reference_implementation(
        config,
        reference,
        baseline,
        target,
        Path(scratch),
        timeout_seconds,
        # The checker requires at least one category, and the mutants name the
        # ones this reference has to cover: each probes an obligation the
        # contract must state to kill it. Only `implementation.equal` is read
        # from the verdict, but the categories have to be honest anyway.
        sorted({case["obligation_category"] for case in cases if case.get("obligation_category")}),
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
        # `killed` and `survived` are verdicts about the mutant; a timeout,
        # a compile failure or a prover crash is not. Recording an unmeasured
        # mutant as non-essential drops it from the scoring set silently, and a
        # weak contract then reaches strict success by killing what is left.
        case["essential"] = result["killed"]
        # Written every time, not only when true: `_approved` refuses a case
        # carrying this flag, so a run that only ever set it would leave the
        # mutant permanently unscorable and make the recovery this tool names
        # -- re-running validation -- impossible.
        validated["inconclusive"] = not reached_a_verdict(result)
    # The whole reference tree, not its `Move.toml`: every reference package is
    # a copy of the same corpus package with one module swapped, so hashing the
    # manifest file gives every reference the same digest and identifies
    # nothing. Essentiality is only reproducible if this pins the specification
    # the mutants were validated against.
    manifest["reference_sha256"] = tree_hash(reference)
    write_json(mutant_manifest, manifest)
    killed = sum(1 for case in cases if case["essential"])
    inconclusive = [
        case["mutant_id"]
        for case in cases
        if case.get("validated", {}).get("inconclusive")
    ]
    return {
        "inconclusive": inconclusive,
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
    if summary["inconclusive"]:
        # Exit non-zero rather than leaving a quietly smaller scoring set: the
        # manifest is written either way, so the evidence is kept and the run
        # can be repeated against the same reference.
        raise SystemExit(
            "no verdict for "
            + ", ".join(summary["inconclusive"])
            + "; their essentiality is unestablished and they would be dropped "
            "from scoring. Re-run once the prover can reach a verdict."
        )


async def _require_reference_implementation(
    config: ExperimentConfig,
    reference: Path,
    baseline: Path,
    target: str,
    scratch: Path,
    timeout_seconds: int,
    required_contract_categories: list[str],
) -> None:
    """Refuse a reference whose executable code differs from the baseline."""
    check_config = scratch / "reference-implementation.json"
    report = scratch / "reference-implementation-verdict.json"
    write_json(
        check_config,
        {
            "schema_version": 1,
            "baseline": str(baseline),
            "package": str(reference),
            "target": target,
            # The reference adds specification to the target's own source file.
            # `implementation.equal` is what enforces that nothing executable
            # changed; this only has to admit the file it writes to.
            "allowed_edit_paths": ["sources/**/*.move"],
            "required_contract_categories": required_contract_categories,
            "timeout_seconds": timeout_seconds,
            "enforce_edit_policy": True,
        },
    )
    outcome = await run_command(
        render_command(config.check_candidate_command, config=check_config, output=report),
        timeout_seconds=max(120, timeout_seconds * 4),
    )
    if not report.is_file():
        raise ValueError(
            "reference implementation could not be compared with the baseline: "
            f"{outcome.diagnostics[:400]}"
        )
    verdict = json.loads(report.read_text(encoding="utf-8"))
    implementation = verdict.get("implementation") or {}
    if not implementation.get("ran"):
        raise ValueError(
            "reference implementation comparison did not run, so essentiality "
            "cannot be established"
        )
    if not implementation.get("equal"):
        changed = ", ".join(
            implementation.get("changed_modules", [])
            + implementation.get("added_modules", [])
            + implementation.get("removed_modules", [])
        )
        raise ValueError(
            "reference implementation differs from the baseline, so it cannot "
            f"certify which mutants are essential: {changed or 'see verdict'}"
        )


if __name__ == "__main__":
    main()
