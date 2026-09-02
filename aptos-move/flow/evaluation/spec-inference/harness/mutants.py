"""Hidden essential-mutant validation and scoring."""

from __future__ import annotations

import argparse
import asyncio
import hashlib
import json
import shutil
import tempfile
from dataclasses import asdict
from difflib import SequenceMatcher
from pathlib import Path
from typing import Any

from .artifacts import copy_snapshot, load_object, sha256_file, tree_hash, write_json
from .config import ExperimentConfig
from .judge import render_command, run_command


#: Digest recorded by a run that has no mutant set and is scored on core
#: criteria only.
NO_MUTANTS = "0" * 64


async def score_mutants(
    config: ExperimentConfig,
    package: Path,
    baseline: Path,
    target: str,
    mutant_manifest: Path,
    timeout_seconds: int,
) -> dict[str, Any]:
    manifest = load_object(mutant_manifest)
    cases = manifest.get("mutants", [])
    approved = [case for case in cases if _approved(case)]
    if len(approved) < 3:
        raise ValueError("at least three independently reviewed essential mutants are required")
    # Three entries have to be three mutations. Copies of one approved mutant
    # would each count a kill while exercising a single obligation.
    if len({_mutation_identity(case) for case in approved}) < 3:
        raise ValueError(
            "at least three distinct essential mutants are required; repeated "
            "mutations of the same code do not count separately"
        )
    # Scratch output belongs beside the run, not in the corpus the manifest
    # lives in; the corpus is an input and must stay unchanged by scoring.
    scratch = tempfile.mkdtemp(prefix="move-inference-clean-")
    clean_compile = await run_command(
        render_command(
            config.compile_command,
            package=package,
            baseline=package,
            target=target,
            timeout=timeout_seconds,
            output=Path(scratch) / "unused-clean.json",
        ),
        timeout_seconds=max(120, timeout_seconds * 4),
    )
    clean_prover = None
    if clean_compile.succeeded:
        clean_prover = await run_command(
            render_command(
                config.prove_command,
                package=package,
                baseline=package,
                target=target,
                timeout=timeout_seconds,
                output=Path(scratch) / "unused-clean.json",
            ),
            timeout_seconds=max(timeout_seconds + 30, timeout_seconds * 3),
        )
    if not clean_compile.succeeded or clean_prover is None or not clean_prover.succeeded:
        raise ValueError("clean package/reference must compile and verify before mutant scoring")
    shutil.rmtree(scratch, ignore_errors=True)
    results = await run_mutant_cases(
        config, package, baseline, target, approved, timeout_seconds
    )
    killed_count = sum(result["killed"] for result in results)
    return {
        "schema_version": 1,
        "target": target,
        "package_sha256": tree_hash(package),
        "mutant_manifest_sha256": sha256_file(mutant_manifest),
        "essential_mutants": len(results),
        "killed": killed_count,
        "mutation_adequacy": killed_count / len(results),
        "clean": {"compile": asdict(clean_compile), "prover": asdict(clean_prover)},
        "results": results,
    }


# A candidate may add a great deal of specification to a small source, so the
# bound on what is aligned is relative to the source with an absolute floor.
MAX_ALIGNMENT_GROWTH = 8
MAX_ALIGNMENT_SLACK = 64 * 1024


def _implementation_offset(baseline_text: str, candidate_text: str, at: int) -> int | None:
    """Map an offset in the pristine source to the same code in a candidate.

    A candidate adds specification text to the file it was given but may not
    change the code. Aligning the two therefore locates the implementation
    exactly, which no textual rule can: a candidate's specification restates
    the mutated expression, sometimes at the same indentation, and the word
    `spec` also appears in prose comments.
    """
    baseline_lines = baseline_text.splitlines(keepends=True)
    candidate_lines = candidate_text.splitlines(keepends=True)
    # Alignment is quadratic in what it aligns. A candidate that has grown far
    # beyond the source it was given did not merely add a specification to it,
    # and is not aligned at all; the caller then reports the mutant as not
    # applicable rather than spending the round's time on it.
    if len(candidate_text) > MAX_ALIGNMENT_GROWTH * len(baseline_text) + MAX_ALIGNMENT_SLACK:
        return None
    # Lines first: a specification is added as whole lines, so this is both
    # cheap and exact for the common case.
    consumed = 0
    line_index = column = None
    for index, line in enumerate(baseline_lines):
        if consumed <= at < consumed + len(line):
            line_index, column = index, at - consumed
            break
        consumed += len(line)
    if line_index is not None:
        starts = [0]
        for line in candidate_lines:
            starts.append(starts[-1] + len(line))
        for tag, i1, i2, j1, j2 in SequenceMatcher(
            None, baseline_lines, candidate_lines, autojunk=False
        ).get_opcodes():
            if tag == "equal" and i1 <= line_index < i2:
                return starts[j1 + (line_index - i1)] + column
    # A loop invariant rewrites the loop's closing line, so the line containing
    # a fragment may itself have changed; characters still align it, within
    # the size bound above.
    for tag, i1, i2, j1, j2 in SequenceMatcher(
        None, baseline_text, candidate_text, autojunk=False
    ).get_opcodes():
        if tag == "equal" and i1 <= at < i2:
            return j1 + (at - i1)
    return None


def _anchored_fragment(pristine: str, case: dict[str, Any]) -> tuple[int, str]:
    """Recover the fragment a mutant rewrites, from the pristine source.

    A mutant stores an offset and a digest, never the source text. The corpus
    package is generated from a private repository into a gitignored tree, so a
    manifest that quoted the code it mutates would put that code in a public
    repository by the back door. Recovering it here keeps the manifest free of
    it while the digest still proves the mutant describes the code it was
    authored against.
    """
    anchor = case["anchor"]
    offset, length = anchor["offset"], anchor["length"]
    fragment = pristine[offset:offset + length]
    actual = hashlib.sha256(fragment.encode("utf-8")).hexdigest()
    if len(fragment) != length or actual != anchor["sha256"]:
        raise ValueError(
            f"mutant {case['mutant_id']} does not describe {case['file']} as "
            "scheduled: the anchored fragment has changed since it was authored"
        )
    if pristine.count(fragment) != 1:
        raise ValueError(
            f"mutant {case['mutant_id']} is ambiguous: its fragment occurs "
            f"{pristine.count(fragment)} times in the baseline"
        )
    return offset, fragment


def _mutate(fragment: str, edit: dict[str, Any], mutant_id: str) -> str:
    """Apply one edit to the recovered fragment."""
    kind = edit["kind"]
    if kind == "substitute":
        at, length = edit["at"], edit["length"]
        return fragment[:at] + edit["to"] + fragment[at + length:]
    if kind == "swap":
        at, a, sep, b = (
            edit["at"], edit["a_length"], edit["separator_length"], edit["b_length"]
        )
        head, rest = fragment[:at], fragment[at:]
        first, separator, second = rest[:a], rest[a:a + sep], rest[a + sep:a + sep + b]
        return head + second + separator + first + rest[a + sep + b:]
    raise ValueError(f"mutant {mutant_id} has unknown edit kind `{kind}`")


def apply_mutant(package: Path, baseline: Path, case: dict[str, Any]) -> None:
    """Rewrite one exact fragment of the implementation.

    Mutants are substitutions rather than diffs because they are applied to a
    finished candidate, whose specification sits in the same file as the code.
    A context diff cut against the pristine package stops applying as soon as an
    agent writes a `spec` block near the mutated lines, which would silently
    turn a scoring failure into an apparatus failure. The fragment itself is
    implementation text, which a candidate may not change, so requiring exactly
    one occurrence is both stable and a check that it did not.
    """
    relative = case["file"]
    parts = Path(relative).parts
    if not parts or ".." in parts or Path(relative).is_absolute():
        raise ValueError(f"unsafe mutant path `{relative}` in {case['mutant_id']}")
    source = package / relative
    text = source.read_text(encoding="utf-8")
    pristine = (baseline / relative).read_text(encoding="utf-8")
    offset, fragment = _anchored_fragment(pristine, case)
    at = _implementation_offset(pristine, text, offset)
    if at is None or not text.startswith(fragment, at):
        raise ValueError(
            f"cannot apply mutant {case['mutant_id']}: the implementation it "
            f"rewrites is not present unchanged in {relative}"
        )
    mutated = _mutate(fragment, case["edit"], case["mutant_id"])
    source.write_text(
        text[:at] + mutated + text[at + len(fragment):], encoding="utf-8"
    )


async def run_mutant_cases(
    config: ExperimentConfig,
    package: Path,
    baseline: Path,
    target: str,
    cases: list[dict[str, Any]],
    timeout_seconds: int,
) -> list[dict[str, Any]]:
    """Apply each mutant to a copy of `package` and record whether it dies.

    A mutant is killed when the prover reports a logical failure against the
    specification already in the package. Validation and scoring ask the same
    question of different packages: validation asks it of a reference
    specification to establish that the mutant is essential, scoring asks it of
    an arm's specification to establish whether that specification is precise.
    """
    results = []
    for case in cases:
        with tempfile.TemporaryDirectory(prefix="move-inference-mutant-") as temporary:
            mutant_package = Path(temporary) / "package"
            copy_snapshot(package, mutant_package)
            apply_mutant(mutant_package, baseline, case)
            compile_result = await run_command(
                render_command(
                    config.compile_command,
                    package=mutant_package,
                    baseline=package,
                    target=target,
                    timeout=timeout_seconds,
                    output=Path(temporary) / "unused.json",
                ),
                timeout_seconds=max(120, timeout_seconds * 4),
            )
            prover_result = None
            report = None
            outcome = "compile_failure"
            if compile_result.succeeded:
                report_path = Path(temporary) / "prover-report.json"
                prover_result = await run_command(
                    render_command(
                        config.prove_command,
                        package=mutant_package,
                        baseline=package,
                        target=target,
                        timeout=timeout_seconds,
                        output=report_path,
                    ),
                    timeout_seconds=max(timeout_seconds + 30, timeout_seconds * 3),
                )
                report = _read_report(report_path)
                outcome = classify_prover_outcome(prover_result, report)
            results.append(
                {
                    "mutant_id": case["mutant_id"],
                    "obligation_category": case["obligation_category"],
                    "killed": outcome == "killed",
                    "outcome": outcome,
                    "compile": asdict(compile_result),
                    "prover": asdict(prover_result) if prover_result else None,
                    "prover_errors": _error_headlines(report),
                }
            )
    return results


def _read_report(path: Path) -> dict[str, Any] | None:
    if not path.is_file():
        return None
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return None
    return value if isinstance(value, dict) else None


def _error_headlines(report: dict[str, Any] | None) -> list[str]:
    """The first line of every error the prover reported, for the record."""
    if report is None:
        return []
    return [
        text.splitlines()[0]
        for text in report.get("diagnostics", [])
        if isinstance(text, str) and text.startswith("error")
    ]


_INFRASTRUCTURE_MARKERS = (
    "panicked at",
    "internal compiler error",
    "no such file",
    "not found",
    "failed to spawn",
    "could not execute",
    "permission denied",
)
_SOLVER_EXHAUSTION_MARKER = "out of resources/timeout"


def classify_prover_outcome(result: Any, report: dict[str, Any] | None) -> str:
    """What a prover run against a mutant established.

    The prover writes its diagnostics to the report it was asked for and says
    nothing on its standard streams beyond a one-line exit reason, so the
    report is the authority: a mutant is killed when the prover ran to
    completion and reported a verification error. A verification error is any
    error the prover reports that is neither a solver budget exhaustion --
    which proves nothing either way -- nor an infrastructure failure.
    """
    if result.timed_out:
        return "prover_timeout"
    if result.infrastructure_error:
        return "infrastructure_failure"
    if result.succeeded:
        return "survived"
    if report is None or report.get("passed") is not False:
        return "unclassified_prover_failure"
    errors = [
        text
        for text in report.get("diagnostics", [])
        if isinstance(text, str) and text.startswith("error")
    ]
    joined = "\n".join([*errors, result.diagnostics]).lower()
    if any(marker in joined for marker in _INFRASTRUCTURE_MARKERS):
        return "infrastructure_failure"
    if any(_SOLVER_EXHAUSTION_MARKER in text for text in errors):
        return "prover_timeout"
    if errors:
        return "killed"
    return "unclassified_prover_failure"


def _mutation_identity(case: dict[str, Any]) -> tuple[str, str, str]:
    """What a mutant changes, for telling repeats apart."""
    return (
        str(case.get("file")),
        json.dumps(case.get("anchor"), sort_keys=True),
        json.dumps(case.get("edit"), sort_keys=True),
    )


def _approved(case: dict[str, Any]) -> bool:
    """Whether a mutant may be scored against.

    `essential` used to be a judgement, and two independent reviewers were the
    check on it. It is now a measurement: `harness.validate_mutants` records
    whether a complete reference specification kills the mutant, against a
    reference the same tool refuses if it does not itself prove or if its
    assumptions are inconsistent. That evidence is reproducible, so it replaces
    the second opinion. What review still supplies is the judgement no run can
    make — that the mutant set was authored before the round and without
    reference to any arm's output — so one recorded approval remains required.
    """
    reviews = case.get("reviews", [])
    reviewers = {review.get("reviewer") for review in reviews if review.get("approved") is True}
    validated = case.get("validated") or {}
    return (
        case.get("essential") is True
        and validated.get("killed_by_reference") is True
        and len(reviewers) >= 1
    )



def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--config", type=Path, required=True)
    parser.add_argument("--package", type=Path, required=True)
    parser.add_argument(
        "--baseline",
        type=Path,
        required=True,
        help="pristine package the candidate was given",
    )
    parser.add_argument("--target", required=True)
    parser.add_argument("--mutants", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--timeout", type=int, default=40)
    args = parser.parse_args()
    result = asyncio.run(
        score_mutants(
            ExperimentConfig.load(args.config),
            args.package.resolve(),
            args.baseline.resolve(),
            args.target,
            args.mutants.resolve(),
            args.timeout,
        )
    )
    write_json(args.output, result)
    if result["killed"] != result["essential_mutants"]:
        raise SystemExit("reference/inferred specification did not kill every essential mutant")


if __name__ == "__main__":
    main()
