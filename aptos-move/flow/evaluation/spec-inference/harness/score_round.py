"""Strict mutation scoring for a finished round.

Scoring runs here rather than inside the controller because the agent shares
the sandbox wrapper's mount namespace: a mutant manifest the controller could
read during a session is a manifest the agent could read too. Mutants are
hidden inputs, so they are applied only after every session has ended, against
the workspace the run left behind.

A run is scored when it reached `operational_success` and its manifest names a
mutant set. `strict_success` then means the specification both verified and
killed every essential mutant.

A second, disjoint set may act as a gate rather than a measure. Refutation
shows its set to the session and lets it repair the contract; a corpus that
does not want that mechanism can withhold the set entirely and apply it here
instead, where a mutation that survives refutes the contract outright and the
run is disqualified rather than measured. Both readings of the same set are
about what the contract rules out; they differ in whether the session was given
a second attempt at it, so a run that was shown a mutation may not then be
disqualified by it.
"""

from __future__ import annotations

import argparse
import asyncio
import hashlib
import json
from dataclasses import asdict, dataclass
from collections.abc import Sequence
from pathlib import Path
from typing import Any

from .artifacts import canonical_json, load_object, sha256_file, tree_hash, write_json
from .compatibility import changed_stages, tool_executables
from .config import ExperimentConfig
from .mutants import (
    NO_MUTANTS,
    mutation_fingerprint,
    overlapping_mutations,
    require_unique_mutant_ids,
    score_mutants,
)


def _require_scoring_apparatus_agrees(
    config: ExperimentConfig, record: dict[str, Any], run_id: str
) -> None:
    """Refuse to measure a run with an apparatus it did not run under.

    A run refuses to execute against an apparatus it did not schedule. Scoring
    is the other half of that claim and was not making it: `strict_success` is
    decided here, by a compile and a prover invoked from the live
    configuration, and a solver or a stage command replaced since the round ran
    would produce a number attributed to the scheduled apparatus and measured
    by a different one.

    A record that pins nothing is left alone, as the controller leaves one:
    rounds scheduled before the apparatus was pinned stay scorable. What must
    not pass is a record that pins something and disagrees.
    """
    expected_config = record.get("config_sha256")
    if expected_config is not None:
        actual_config = hashlib.sha256(canonical_json(asdict(config))).hexdigest()
        if actual_config != expected_config:
            raise ValueError(
                f"run {run_id} ran under experiment configuration "
                f"{expected_config} but scoring was given {actual_config}: a "
                "strict-success result must be measured by the apparatus that "
                "produced the run"
            )
    # The scoring code is part of the apparatus, not a neutral observer of it:
    # how a mutation is applied, when a result counts as killed, and what
    # `strict_success` requires all live in this tree. A run pins the harness it
    # ran under for the same reason, and scoring reads the same pin.
    expected_harness = record.get("controller_harness_sha256")
    if expected_harness is not None:
        actual_harness = tree_hash(Path(__file__).resolve().parent)
        if actual_harness != expected_harness:
            raise ValueError(
                f"harness changed since run {run_id} was recorded: expected "
                f"{expected_harness}, scoring with {actual_harness}; how a "
                "mutation is applied and classified is part of what a "
                "strict-success result claims"
            )
    expected_stages = record.get("stage_executables")
    if expected_stages:
        changed = changed_stages(expected_stages, tool_executables(config))
        if changed:
            raise ValueError(
                f"stage executable(s) changed since run {run_id} was recorded "
                f"({', '.join(changed)}): a verdict from one toolchain cannot "
                "be scored as a result from another"
            )


@dataclass(frozen=True)
class PendingScore:
    """One run waiting to be measured, and the sets it is measured with."""

    entry: dict[str, Any]
    candidate: Path
    baseline: Path
    target: str
    manifest: Path
    timeout_seconds: int
    #: Applied before the scored set, when the round withholds a set instead of
    #: refuting with it: a mutation that survives this one refutes the contract,
    #: so the run is disqualified rather than given a mutation score.
    disqualification_manifest: Path | None = None


def _disqualification_manifest(
    root: Path,
    task_id: str,
    run_id: str,
    scored_manifest: Path,
    baseline: Path,
    shown: Sequence[str] | None,
    expected_sha256: str | None,
) -> Path:
    """The gate set for one run, once it is disjoint from what may measure it.

    The two sets have to stay separable for the same reason refutation and
    scoring do. A mutation in both would disqualify a run and then be counted
    among the mutants its score is out of, and a mutation the session was shown
    is one it was invited to repair against -- disqualifying a contract by a
    counterexample it was handed measures the feedback, not the contract.
    """
    manifest = root / task_id / "mutants.json"
    if not manifest.is_file():
        raise FileNotFoundError(
            f"run {run_id} is gated on a disqualification set but {manifest} is missing"
        )
    if expected_sha256 is None:
        raise ValueError(
            f"run {run_id} was not scheduled with a disqualification set; "
            "reschedule it before applying a withheld gate"
        )
    actual_sha256 = sha256_file(manifest)
    if actual_sha256 != expected_sha256:
        raise ValueError(
            f"the disqualification set for {task_id} disagrees with the digest "
            f"recorded when run {run_id} was scheduled"
        )
    cases = load_object(manifest)["mutants"]
    scored_cases = load_object(scored_manifest)["mutants"]
    require_unique_mutant_ids(cases, f"disqualification set for {task_id}")
    require_unique_mutant_ids(scored_cases, f"scored set for {task_id}")
    repeated = overlapping_mutations(
        cases,
        {mutation_fingerprint(case, baseline) for case in scored_cases},
        baseline,
    )
    if repeated:
        raise ValueError(
            f"the disqualification set for {task_id} repeats mutation(s) the "
            f"round is scored on ({', '.join(repeated)}): a mutation cannot both "
            "void a run and count towards its score"
        )
    was_shown = overlapping_mutations(cases, set(shown or ()), baseline)
    if was_shown:
        raise ValueError(
            f"run {run_id} was shown mutation(s) the disqualification set "
            f"repeats ({', '.join(was_shown)}): a contract repaired against a "
            "counterexample it was given cannot then be refuted by it"
        )
    return manifest


async def score_round(
    config: ExperimentConfig,
    round_dir: Path,
    mutants_root: Path,
    timeout_seconds: int,
    allow_corrected_mutants: bool = False,
    concurrency: int = 1,
    disqualification_root: Path | None = None,
) -> dict[str, Any]:
    runs_dir = round_dir / "runs"
    if not runs_dir.is_dir():
        raise FileNotFoundError(runs_dir)
    scored: list[dict[str, Any]] = []
    pending: list[PendingScore] = []
    for artifact in sorted(path for path in runs_dir.iterdir() if path.is_dir()):
        record_path = artifact / "run.json"
        if not record_path.is_file():
            continue
        # The controller enriches this file with its own result, so it is no
        # longer a bare run manifest; read the scheduling fields directly
        # rather than through the manifest schema.
        record = json.loads(record_path.read_text(encoding="utf-8"))
        run_id = record["run_id"]
        task_id = record["task_id"]
        target = record["target"]
        mutant_digest = record["mutant_manifest_sha256"]
        disqualification_digest = record.get(
            "disqualification_mutant_manifest_sha256"
        )
        if disqualification_digest is not None and disqualification_root is None:
            raise ValueError(
                f"run {run_id} is bound to a disqualification mutant set but "
                "--disqualification-mutants-root was not provided"
            )
        # A run that did not reach operational success records no eventual
        # judge at all, so the key is present and null. Mutation scoring is
        # gated on that judge -- it is the authority that this very tree
        # proves -- but the reported status comes from the controller, which
        # records one for every run. Reporting the judge state as the terminal
        # status made a compile failure, a timeout and an exhausted budget
        # indistinguishable in the round's own scoring record.
        _require_scoring_apparatus_agrees(config, record, run_id)
        result = record.get("result") or {}
        judge = result.get("eventual_judge") or {}
        status = judge.get("state")
        entry: dict[str, Any] = {
            "run_id": run_id,
            "task_id": task_id,
            "arm": record.get("arm"),
            "terminal_status": result.get("terminal_status"),
            "eventual_judge_state": status,
        }
        if mutant_digest == NO_MUTANTS:
            entry["outcome"] = "no_mutant_set"
        elif status != "operational_success":
            # A run that never verified cannot be asked whether its
            # specification is precise; the question presupposes a proof.
            entry["outcome"] = "not_operationally_successful"
        else:
            manifest = mutants_root / task_id / "mutants.json"
            if not manifest.is_file():
                raise FileNotFoundError(
                    f"run {run_id} names a mutant set but {manifest} is missing"
                )
            scored_digest = sha256_file(manifest)
            if scored_digest != mutant_digest:
                # The digest binds scoring to the set the round was scheduled
                # against, so a different set cannot be used by accident. A set
                # corrected after a defect was found in it is a deliberate
                # exception, and both digests are recorded so the summary shows
                # which mutants produced its numbers.
                if not allow_corrected_mutants:
                    raise ValueError(
                        f"mutant manifest for {task_id} disagrees with the digest "
                        f"recorded when the round was scheduled; pass "
                        f"--allow-corrected-mutants to score against a corrected set"
                    )
                entry["scheduled_mutant_manifest_sha256"] = mutant_digest
                entry["scored_mutant_manifest_sha256"] = scored_digest
                # The scheduler proved the refutation set disjoint from the
                # *scheduled* scoring set. Replacing that set replaces one side
                # of the comparison, so the guarantee does not carry over: a
                # corrected manifest that happens to contain a mutation this
                # run was shown would credit "the contract is complete" for
                # what was really "the agent can act on feedback". Same
                # relation, re-checked against the set actually being scored.
                shown = record.get("refutation_mutant_identities")
                if shown is None:
                    raise ValueError(
                        f"run {run_id} records no refutation identities, so a "
                        f"corrected mutant set for {task_id} cannot be shown "
                        "disjoint from what the run was shown; rerun the round "
                        "with a current controller build"
                    )
                repeated = overlapping_mutations(
                    load_object(manifest)["mutants"],
                    set(shown),
                    artifact / "baseline" / record["package_relpath"],
                )
                if repeated:
                    raise ValueError(
                        f"corrected mutant set for {task_id} repeats mutation(s) "
                        f"run {run_id} was shown during refutation "
                        f"({', '.join(repeated)}): a contract repaired against a "
                        "mutant it was shown cannot then be measured by it"
                    )
            # A run that cannot be scored is recorded as such rather than
            # aborting the round: one candidate whose own proof does not
            # reproduce at this timeout says nothing about the other cells, and
            # losing them to it would be an apparatus failure reported as an
            # absence of results.
            # `final/` rather than `workspace/`: it is the package the
            # controller judged, already at `package_relpath`, and its
            # escaping symlinks have been defused -- scoring the live
            # workspace would compile the wrong tree for a nested package and
            # could fail on a link the finalized record was built to survive.
            baseline = artifact / "baseline" / record["package_relpath"]
            pending.append(
                PendingScore(
                    entry,
                    artifact / "final",
                    baseline,
                    target,
                    manifest,
                    record.get("prove_timeout_seconds") or timeout_seconds,
                    _disqualification_manifest(
                        disqualification_root,
                        task_id,
                        run_id,
                        manifest,
                        baseline,
                        record.get("refutation_mutant_identities"),
                        disqualification_digest,
                    )
                    if disqualification_root is not None
                    else None,
                )
            )
        scored.append(entry)

    await _score_pending(config, pending, concurrency)
    summary = {
        "schema_version": 1,
        "round_id": round_dir.name,
        "mutant_set": "corrected" if allow_corrected_mutants else "as_scheduled",
        "scored": sum(1 for entry in scored if entry["outcome"] == "scored"),
        "disqualified": sum(1 for entry in scored if entry["outcome"] == "disqualified"),
        "strict_successes": sum(1 for entry in scored if entry.get("strict_success")),
        "runs": scored,
    }
    write_json(round_dir / "mutation-summary.json", summary)
    return summary


async def _score_pending(
    config: ExperimentConfig,
    pending: list[PendingScore],
    concurrency: int,
) -> None:
    """Score every run, at most `concurrency` at a time.

    The runs are independent, but each is a batch of prover invocations and the
    solver budget is wall-clock. Concurrent scoring makes those budgets compete
    for CPU, which can turn a mutant that would have been killed into a timeout,
    so the default stays sequential and raising it is the operator's call.
    """
    if concurrency < 1:
        raise ValueError("concurrency must be positive")
    semaphore = asyncio.Semaphore(concurrency)

    async def measure(item: PendingScore, manifest: Path) -> dict[str, Any]:
        return await score_mutants(
            config,
            item.candidate,
            item.baseline,
            item.target,
            manifest,
            item.timeout_seconds,
        )

    async def score_one(item: PendingScore) -> None:
        entry = item.entry
        async with semaphore:
            # A run that cannot be scored is recorded as such rather than
            # aborting the round: one candidate whose own proof does not
            # reproduce at this timeout says nothing about the other cells, and
            # losing them to it would be an apparatus failure reported as an
            # absence of results.
            try:
                if item.disqualification_manifest is not None:
                    gate = await measure(item, item.disqualification_manifest)
                    write_json(
                        item.candidate.parent / "disqualification-score.json", gate
                    )
                    entry["disqualification_manifest_sha256"] = gate[
                        "mutant_manifest_sha256"
                    ]
                    # A mutant that reached no verdict is not one the contract
                    # failed to kill, so it does not refute anything; it is
                    # recorded so a gate that measured nothing cannot read as a
                    # gate the contract passed.
                    survived = [
                        result["mutant_id"]
                        for result in gate["results"]
                        if not result["killed"]
                        and result["mutant_id"] not in gate["inconclusive"]
                    ]
                    if gate["inconclusive"]:
                        entry["disqualification_inconclusive"] = gate["inconclusive"]
                    if survived:
                        # Not measured afterwards: a refuted contract has no
                        # mutation score to report, and reporting one invites
                        # reading a disqualified run as a partial result.
                        entry["outcome"] = "disqualified"
                        entry["disqualified_by"] = survived
                        entry["strict_success"] = False
                        return
                    if gate["inconclusive"]:
                        entry["outcome"] = "not_scorable"
                        entry["detail"] = (
                            "disqualification gate reached no verdict for: "
                            + ", ".join(gate["inconclusive"])
                        )
                        entry["strict_success"] = False
                        return
                score = await measure(item, item.manifest)
            except Exception as error:
                # Deliberately broad. The comment above is the contract: a run
                # that cannot be scored is recorded, not raised. A missing or
                # unreadable workspace, a symlink refusal, a solver that will
                # not start -- each is a property of one cell, and letting it
                # escape cancels the gather and discards every other cell's
                # score. `BaseException` is still allowed through, so a
                # cancellation or interrupt stops the round as it should.
                entry["outcome"] = "not_scorable"
                entry["detail"] = f"{type(error).__name__}: {error}"
            else:
                write_json(item.candidate.parent / "mutation-score.json", score)
                entry["mutation_adequacy"] = score["mutation_adequacy"]
                # A mutant that reached no verdict is not a mutant the contract
                # failed to kill. Scoring it as one reports an infrastructure
                # failure as evidence against the specification, so the run is
                # marked unmeasured rather than unsuccessful.
                if score.get("inconclusive"):
                    entry["outcome"] = "inconclusive"
                    entry["inconclusive"] = score["inconclusive"]
                    entry["strict_success"] = False
                else:
                    entry["outcome"] = "scored"
                    entry["strict_success"] = (
                        score["killed"] == score["essential_mutants"]
                    )

    await asyncio.gather(*(score_one(item) for item in pending))


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--config", type=Path, required=True)
    parser.add_argument("--round-dir", type=Path, required=True)
    parser.add_argument(
        "--mutants-root",
        type=Path,
        required=True,
        help="directory of TASK_ID/mutants.json; must never be mounted in an agent sandbox",
    )
    parser.add_argument(
        "--disqualification-mutants-root",
        type=Path,
        help="directory of TASK_ID/mutants.json applied as a gate rather than a "
        "measure: a run whose contract lets one of these mutations survive is "
        "disqualified instead of scored. Use it for a corpus that withholds the "
        "set during the round rather than refuting with it. Must never be "
        "mounted in an agent sandbox, and must be disjoint from --mutants-root",
    )
    parser.add_argument("--timeout", type=int, default=40)
    parser.add_argument(
        "--concurrency",
        type=int,
        default=1,
        help="score this many runs at once; above 1 their solver budgets compete "
        "for CPU, which can turn a killed mutant into a timeout",
    )
    parser.add_argument(
        "--allow-corrected-mutants",
        action="store_true",
        help="score against a mutant set corrected after the round was scheduled; "
        "the summary records both the scheduled and the scored digest",
    )
    args = parser.parse_args()
    if (
        args.disqualification_mutants_root
        and args.disqualification_mutants_root.resolve() == args.mutants_root.resolve()
    ):
        raise SystemExit(
            "--disqualification-mutants-root and --mutants-root name the same set; "
            "a mutation cannot both void a run and count towards its score"
        )
    summary = asyncio.run(
        score_round(
            ExperimentConfig.load(args.config.resolve()),
            args.round_dir.resolve(),
            args.mutants_root.resolve(),
            args.timeout,
            args.allow_corrected_mutants,
            args.concurrency,
            args.disqualification_mutants_root.resolve()
            if args.disqualification_mutants_root
            else None,
        )
    )
    print(json.dumps({k: v for k, v in summary.items() if k != "runs"}, sort_keys=True))


if __name__ == "__main__":
    main()
