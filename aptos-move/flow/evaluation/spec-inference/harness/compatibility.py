"""Pre-treatment compiler, WP, and prover compatibility screening."""

from __future__ import annotations

import argparse
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
                untrusted_inferred_conditions = _find_untrusted_inferred_conditions(
                    enriched_package
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
        "schema_version": 5,
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
    return {key: value for key, value in (entry or {}).items() if key != "path"}


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
