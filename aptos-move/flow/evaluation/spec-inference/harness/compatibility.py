"""Pre-treatment compiler, WP, and prover compatibility screening."""

from __future__ import annotations

import argparse
import collections
import asyncio
import json
import re
import os
import shutil
import tempfile
from dataclasses import asdict
from pathlib import Path
from typing import Any

from .artifacts import sha256_file, tree_hash, write_json
from .move_source import mask_comments_and_strings
from .clean_unused_aliases import clean_unused_aliases
from .config import ExperimentConfig
from .judge import render_command, run_command

# Version of the result `check_compatibility` writes; a resumed screen accepts
# only results of this shape.
COMPATIBILITY_SCHEMA_VERSION = 5


async def check_compatibility(
    config: ExperimentConfig,
    package: Path,
    target: str,
    threshold_seconds: int | None = None,
) -> dict[str, Any]:
    threshold = threshold_seconds or config.eventual_timeout_seconds
    if threshold < 1:
        raise ValueError("compatibility threshold must be positive")
    with tempfile.TemporaryDirectory(prefix="move-inference-compatibility-") as temporary:
        temporary_path = Path(temporary)
        values = {
            "package": package,
            "baseline": package,
            "target": target,
            "timeout": config.eventual_timeout_seconds,
            "output": temporary_path / "stage.json",
        }
        compile_result = await run_command(
            render_command(config.compile_command, **values),
            timeout_seconds=threshold,
        )
        compile_report = _read_stage_report(values["output"])
        inference_result = None
        inference_report = None
        enriched_compile_result = None
        enriched_compile_report = None
        alias_cleanup = None
        prover_result = None
        prover_report = None
        untrusted_inferred_conditions: list[dict[str, object]] = []
        if compile_result.succeeded:
            # Inference writes specs into a disposable, byte-for-byte copy. The
            # prover must consume that enriched copy; proving the original
            # spec-stripped package would not test WP compatibility.
            enriched_package = temporary_path / "enriched-package"
            shutil.copytree(
                package,
                enriched_package,
                ignore=shutil.ignore_patterns("build"),
            )
            values["package"] = enriched_package
            values["output"].unlink(missing_ok=True)
            inference_result = await run_command(
                render_command(config.inference_command, **values),
                timeout_seconds=threshold,
            )
            inference_report = _read_stage_report(values["output"])
            if inference_result.succeeded:
                # Only what this run inferred. The scan reads every
                # `.spec.move` in the package, and a prepared corpus ships
                # reviewed dependency contracts that already carry flagged
                # clauses -- they are the corpus's trusted boundary, not
                # something the target introduced. Subtracting the conditions
                # already present before enrichment leaves exactly the ones WP
                # emitted for this target, which is what the check is for.
                untrusted_inferred_conditions = _conditions_introduced_by(
                    package, enriched_package
                )
                values["output"].unlink(missing_ok=True)
                # Recompile and prove exactly the inferred source. In
                # particular, never delete a `sathard`/vacuous clause to make
                # the screen pass: it is a repair obligation, normally a
                # missing invariant or supporting lemma.
                enriched_compile_result = await run_command(
                    render_command(config.compile_command, **values),
                    timeout_seconds=threshold,
                )
                enriched_compile_report = _read_stage_report(values["output"])
                if enriched_compile_result.succeeded:
                    alias_cleanup = clean_unused_aliases(
                        enriched_package,
                        temporary_path,
                        values["output"],
                        temporary_path / "unused-alias-cleanup.json",
                    )
                    values["output"].unlink(missing_ok=True)
                    prover_result = await run_command(
                        render_command(config.prove_command, **values),
                        timeout_seconds=threshold,
                    )
                    prover_report = _read_stage_report(values["output"])
    stages = {
        "compile": compile_result,
        "wp_inference": inference_result,
        "enriched_compile": enriched_compile_result,
        "prover": prover_result,
    }
    exceeded_stage = next(
        (name for name, result in stages.items() if result is not None and result.timed_out),
        None,
    )
    passed = bool(
        compile_result.succeeded
        and inference_result
        and inference_result.succeeded
        and enriched_compile_result
        and enriched_compile_result.succeeded
        and prover_result
        and prover_result.succeeded
        and not untrusted_inferred_conditions
    )
    failure_kind = (
        "untrusted_inferred_contract"
        if untrusted_inferred_conditions
        else (_failure_kind(stages) if not passed else None)
    )
    report = {
        "schema_version": COMPATIBILITY_SCHEMA_VERSION,
        "package_sha256": tree_hash(package),
        "target": target,
        "threshold_seconds": threshold,
        "threshold_exceeded_stage": exceeded_stage,
        "total_duration_ms": sum(
            result.duration_ms for result in stages.values() if result is not None
        ),
        "passed": passed,
        "tool_executables": tool_executables(config),
        "failure_kind": failure_kind,
        "untrusted_inferred_conditions": untrusted_inferred_conditions,
        "unused_alias_cleanup": alias_cleanup,
        "compile": _stage_result(compile_result, compile_report),
        "wp_inference": (
            _stage_result(inference_result, inference_report)
            if inference_result
            else None
        ),
        "enriched_compile": (
            _stage_result(enriched_compile_result, enriched_compile_report)
            if enriched_compile_result
            else None
        ),
        "prover": (
            _stage_result(prover_result, prover_report) if prover_result else None
        ),
    }
    return report


def _read_stage_report(path: Path) -> dict[str, Any] | None:
    if not path.is_file():
        return None
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return None
    return value if isinstance(value, dict) else None


def _stage_result(result: Any, stage_report: dict[str, Any] | None) -> dict[str, Any]:
    return {**asdict(result), "stage_report": stage_report}


def stage_identity(entry: dict[str, Any] | None) -> dict[str, Any]:
    """What a stage runs, as opposed to where it is.

    A stage is a command, not a file: `render_command` executes the whole
    argument vector, and `tool_executables` hashes every argument that resolves
    to one for that reason. Comparing the top-level digest alone would accept a
    `["python3", "wrapper.py"]` stage whose wrapper was rewritten under an
    unchanged interpreter.

    Everything the record carries is compared except `path`, which says only
    where a build was found -- relocating one does not change what it decides.
    Subtracting the one irrelevant key rather than listing the relevant ones
    means a field added to the record later is compared by default, which is
    the safe direction.
    """
    identity = {
        key: value for key, value in (entry or {}).items() if key != "path"
    }
    arguments = identity.get("arguments")
    if isinstance(arguments, dict):
        # Argument paths are useful while diagnosing a local run, but the
        # ordered content digests are the portable identity.  Keeping the map
        # keys would make a relocated wrapper look different and would expose
        # the checkout layout when this identity is published as evidence.
        identity["arguments"] = list(arguments.values())
    return identity


def changed_stages(
    expected: dict[str, dict[str, Any]], actual: dict[str, dict[str, Any]]
) -> list[str]:
    """The stages that differ between two recorded apparatus identities.

    Named once because it is asked twice: a run refuses to execute against an
    apparatus it did not schedule, and post-round scoring refuses to measure
    with one. A stage present on one side and absent on the other counts as
    changed -- an absent backend is not an unchanged one, and whatever the
    environment resolves next would silently take its place.
    """
    return sorted(
        name
        for name in set(expected) | set(actual)
        if stage_identity(expected.get(name)) != stage_identity(actual.get(name))
    )


def tool_executables(config: ExperimentConfig) -> dict[str, dict[str, Any]]:
    """Digest everything a screening stage actually runs.

    `render_command` executes the whole configured argument vector, so hashing
    only its first word identifies the interpreter and not the program: a
    `["python3", "wrapper.py"]` stage would keep its recorded identity while
    the wrapper is rewritten underneath it. Every argument that resolves to a
    file is hashed for the same reason.

    The prover backends are named by environment rather than by the command, so
    they are recorded alongside: a screening verdict is a claim about what
    Boogie and Z3 decided, and swapping either changes what "proved" meant.
    """
    result: dict[str, dict[str, Any]] = {}
    for name, command in (
        ("compile", config.compile_command),
        ("wp_inference", config.inference_command),
        ("enriched_compile", config.compile_command),
        ("prover", config.prove_command),
        # The judge that decides `operational_success`, and therefore which
        # runs are scored at all. The configuration digest binds the command's
        # words; this binds what those words execute.
        ("check_candidate", config.check_candidate_command),
    ):
        # A stage may be unconfigured, in which case there is no executable to
        # identify -- rather than an executable that failed to resolve.
        resolved = shutil.which(command[0]) if command else None
        if resolved is None:
            continue
        path = Path(resolved).resolve()
        entry: dict[str, Any] = {"path": str(path), "sha256": sha256_file(path)}
        # Later arguments that name a file are part of what runs.
        arguments = {}
        for argument in command[1:]:
            candidate = Path(argument)
            if candidate.is_file():
                arguments[argument] = sha256_file(candidate.resolve())
        if arguments:
            entry["arguments"] = arguments
        result[name] = entry
    for name, variable, fallback in (
        ("boogie", "BOOGIE_EXE", "boogie"),
        ("z3", "Z3_EXE", "z3"),
    ):
        located = os.environ.get(variable) or shutil.which(fallback)
        if located and Path(located).is_file():
            path = Path(located).resolve()
            result[name] = {"path": str(path), "sha256": sha256_file(path)}
    return result


def _failure_kind(stages: dict[str, Any]) -> str:
    if any(result is not None and result.timed_out for result in stages.values()):
        return "compatibility_timeout"
    if any(
        result is not None
        and (
            result.infrastructure_error
            or _looks_like_infrastructure_failure(result.diagnostics)
        )
        for result in stages.values()
    ):
        return "infrastructure_failure"
    # A compiler, WP, or prover failure is actionable evidence about the
    # implementation. It must be fixed and re-screened, never converted into a
    # corpus exclusion.
    return "implementation_failure"


def _looks_like_infrastructure_failure(diagnostics: str) -> bool:
    lower = diagnostics.lower()
    markers = (
        "no such file or directory",
        "executable file not found",
        "command not found",
        "failed to spawn",
        "could not execute",
        "permission denied",
    )
    return any(marker in lower for marker in markers)


_FLAGGED_CONDITION = re.compile(
    r"(?m)^[ \t]*(?:requires|ensures|aborts_if|aborts_with|modifies|emits|"
    r"invariant|decreases)\s*\[\s*inferred\s*=\s*(?:vacuous|sathard)\s*\]"
)
def _conditions_introduced_by(
    baseline: Path, enriched: Path
) -> list[dict[str, object]]:
    """Flagged clauses present after inference and absent before it.

    A prepared corpus carries dependency contracts that were authored and
    reviewed against an earlier prover, and a later prover may flag some of
    them. Those are inputs to the screen, not findings of it: re-reporting them
    fails every target in the package for the same reason and says nothing
    about any of them. What the screen asks is whether WP produced an untrusted
    clause *here*.

    Compared by file and kind rather than by line, so a clause that merely
    shifted position when the inferred specification was written in is still
    recognized as pre-existing.
    """
    def digest(package: Path) -> collections.Counter:
        counted: collections.Counter = collections.Counter()
        for finding in _find_untrusted_inferred_conditions(package):
            counted[(finding["path"], finding["kind"])] += 1
        return counted

    before = digest(baseline)
    introduced: list[dict[str, object]] = []
    seen: collections.Counter = collections.Counter()
    for finding in _find_untrusted_inferred_conditions(enriched):
        key = (finding["path"], finding["kind"])
        seen[key] += 1
        if seen[key] > before[key]:
            introduced.append(finding)
    return introduced


def _find_untrusted_inferred_conditions(package: Path) -> list[dict[str, object]]:
    """Return, but never alter, WP clauses it marked unfit for a contract."""
    findings: list[dict[str, object]] = []
    for path in package.rglob("*.spec.move"):
        text = path.read_text(encoding="utf-8")
        masked = mask_comments_and_strings(text)
        for match in _FLAGGED_CONDITION.finditer(masked):
            findings.append(
                {
                    "path": path.relative_to(package).as_posix(),
                    "line": masked.count("\n", 0, match.start()) + 1,
                    "kind": match.group(0).split()[0],
                }
            )
    return findings


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--config", type=Path, required=True)
    parser.add_argument("--package", type=Path, required=True)
    parser.add_argument("--target", required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--threshold-seconds", type=int)
    args = parser.parse_args()
    result = asyncio.run(
        check_compatibility(
            ExperimentConfig.load(args.config.resolve()),
            args.package.resolve(),
            args.target,
            args.threshold_seconds,
        )
    )
    write_json(args.output, result)
    print(json.dumps({"passed": result["passed"], "target": result["target"]}, sort_keys=True))
    if not result["passed"]:
        raise SystemExit("target failed compiler/WP/prover compatibility screening")


if __name__ == "__main__":
    main()


def apparatus_reached_a_verdict(result: dict[str, Any]) -> bool:
    """Whether the screen actually measured the target.

    `check_compatibility` reports `infrastructure_failure` for tooling that was
    unavailable and `compatibility_timeout` for a stage that ran out of time.
    Neither is evidence about the target, so neither can be read as one.
    """
    if result.get("failure_kind") in ("infrastructure_failure", "compatibility_timeout"):
        return False
    # Every stage the screen ran, not only WP. A prover that dies on the
    # inferred source says as little about the target as a WP that dies on the
    # original, and `_failure_kind` lands both on `implementation_failure`;
    # reading only the inference stage admitted a target whose enriched proof
    # had crashed as a screened, WP-hard corpus member.
    return all(
        _stage_reached_a_verdict(result.get(stage))
        for stage in ("compile", "wp_inference", "enriched_compile", "prover")
    )


def _stage_reached_a_verdict(stage: dict[str, Any] | None) -> bool:
    """Whether one stage's exit says anything about the target.

    Success is a verdict, and so is a non-zero exit carrying a stage report: a
    tool that declines with a diagnosis has diagnosed something. A crash
    arrives with neither, and `_failure_kind` cannot tell the two apart. A
    negative return code is a signal, which is never a refusal.

    A stage that did not run has no return code and is not held against the
    target: when WP declines there is nothing to recompile or prove.
    """
    stage = stage or {}
    returncode = stage.get("returncode")
    if returncode in (0, None):
        return True
    return returncode > 0 and bool(stage.get("stage_report"))


def is_well_formed(result: dict[str, Any]) -> bool:
    """Whether the target itself is admissible, given a working apparatus.

    Inference failing is a property of the task, not a defect in it: an
    uninvariant loop makes WP drop what the havoc left unconstrained, and in an
    evaluation that is an error rather than an empty `aborts_if_is_partial`
    contract -- so the loop targets, which are the interesting ones, report a
    failed inference stage. When WP declines there is nothing to recompile, so
    requiring the enriched compile would eject exactly the tasks worth asking.

    But a failed inference stage covers two different things, and only one of
    them is about the target. WP that could not run at all says nothing, and
    admitting it records an unscreened task as a corpus member.
    """
    compiles = (result.get("compile") or {}).get("returncode") == 0
    inferred = (result.get("wp_inference") or {}).get("returncode") == 0
    enriched_ok = (
        (result.get("enriched_compile") or {}).get("returncode") == 0
        if inferred
        else True
    )
    return compiles and enriched_ok and apparatus_reached_a_verdict(result)


async def prove_reference(
    config: ExperimentConfig, package: Path, targets: list[str], threshold: int
) -> dict[str, Any]:
    """Prove a task's reference contracts, the evidence that it is solvable.

    `targets` are the task's functions as `address::module::function`: a
    function-level prover scope verifies each one even under
    `pragma verify = false`, whereas a module-level scope would refuse the
    module for functions the task does not ask about.
    """
    outcomes = [
        await _prove_reference_target(config, package, target, threshold)
        for target in targets
    ]
    return {
        "proved": all(outcome["proved"] for outcome in outcomes),
        "vacuity_checked": all(outcome["vacuity_checked"] for outcome in outcomes),
        "vacuous": any(outcome["vacuous"] for outcome in outcomes),
        "targets": {target: outcome for target, outcome in zip(targets, outcomes)},
        "package": str(package),
        "reference_sha256": tree_hash(package),
    }


async def _prove_reference_target(
    config: ExperimentConfig, package: Path, target: str, threshold: int
) -> dict[str, Any]:
    with tempfile.TemporaryDirectory(prefix="move-inference-reference-") as temporary:
        outcome = await run_command(
            render_command(
                config.prove_command,
                package=package,
                baseline=package,
                target=target,
                timeout=threshold,
                output=Path(temporary) / "reference.json",
            ),
            timeout_seconds=max(120, threshold * 4),
        )
        # A reference with contradictory assumptions proves every
        # postcondition, so a successful prove says nothing about solvability
        # on its own. `validate_mutants` refuses such a reference before
        # certifying essentiality; the screen has to refuse it before
        # certifying that the task is solvable at all.
        inconsistency = await run_command(
            render_command(
                [*config.prove_command, "--check-inconsistency"],
                package=package,
                baseline=package,
                target=target,
                timeout=threshold,
                output=Path(temporary) / "inconsistency.json",
            ),
            timeout_seconds=max(120, threshold * 4),
        )
        vacuous = "inconsistent assumption" in inconsistency.diagnostics
        # Silence only means something if the check reached a verdict: a
        # timeout or a missing solver produces the same absence of the
        # diagnostic as a sound reference does.
        vacuity_checked = inconsistency.succeeded and not vacuous
    # `returncode == 0` is not success: a process can exit zero while the
    # watchdog is tearing it down, and `succeeded` also covers an
    # infrastructure error. A reference that did not finish proving has not
    # proved anything.
    return {
        "proved": outcome.succeeded and vacuity_checked,
        "vacuity_checked": vacuity_checked,
        "vacuous": vacuous,
    }


def admission(result: dict[str, Any]) -> dict[str, Any]:
    """Whether a screened target is a corpus member, and why not otherwise.

    `check_compatibility` also asks that WP's unaided output verify. That was
    the bar for the first prepared corpus, where the dependency contracts had to
    be complete before a target was asked of anyone. Here the target *is* the
    task, and a target WP cannot do unaided is the interesting kind -- admitting
    only what WP already solves would keep the easy member of every family. So
    admission asks whether the task is well-formed and provable, and
    WP-hardness is recorded as a property of the task, with the stage WP fell
    short at as its kind.
    """
    well_formed = is_well_formed(result)
    reference = result.get("reference_proof") or {}
    wp_hard = not result["passed"]
    passed = bool(well_formed and reference.get("proved"))
    if passed:
        reason = None
    elif not well_formed:
        reason = result.get("failure_kind") or _failure_kind(
            {
                name: result.get(name)
                for name in ("compile", "wp_inference", "enriched_compile", "prover")
            }
        )
    else:
        reason = "reference_unproved"
    return {
        "passed": passed,
        "reason": reason,
        "well_formed": well_formed,
        "reference_proved": bool(reference.get("proved")),
        "wp_hard": wp_hard,
        "wp_failure_kind": result.get("failure_kind") if wp_hard else None,
    }


def binary_sha256(name: str) -> str | None:
    """The digest of the tool the screen actually invoked, if it is on PATH."""
    resolved = shutil.which(name)
    return sha256_file(Path(resolved)) if resolved else None
