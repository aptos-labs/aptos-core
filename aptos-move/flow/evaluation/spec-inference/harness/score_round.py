"""Strict mutation scoring for a finished round.

Scoring runs here rather than inside the controller because the agent shares
the sandbox wrapper's mount namespace: a mutant manifest the controller could
read during a session is a manifest the agent could read too. Mutants are
hidden inputs, so they are applied only after every session has ended, against
the workspace the run left behind.

A run is scored when it reached `operational_success` and its manifest names a
mutant set. `strict_success` then means the specification both verified and
killed every essential mutant.
"""

from __future__ import annotations

import argparse
import asyncio
import hashlib
import json
from dataclasses import asdict
from pathlib import Path
from typing import Any

from .artifacts import canonical_json, load_object, sha256_file, tree_hash, write_json
from .compatibility import changed_stages, tool_executables
from .config import ExperimentConfig
from .mutants import NO_MUTANTS, overlapping_mutations, score_mutants


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


async def score_round(
    config: ExperimentConfig,
    round_dir: Path,
    mutants_root: Path,
    timeout_seconds: int,
    allow_corrected_mutants: bool = False,
    concurrency: int = 1,
) -> dict[str, Any]:
    runs_dir = round_dir / "runs"
    if not runs_dir.is_dir():
        raise FileNotFoundError(runs_dir)
    scored: list[dict[str, Any]] = []
    pending: list[tuple[dict[str, Any], Path, str, Path, int]] = []
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
            pending.append(
                (
                    entry,
                    artifact / "final",
                    artifact / "baseline" / record["package_relpath"],
                    target,
                    manifest,
                    record.get("prove_timeout_seconds") or timeout_seconds,
                )
            )
        scored.append(entry)

    await _score_pending(config, pending, concurrency)
    summary = {
        "schema_version": 1,
        "round_id": round_dir.name,
        "mutant_set": "corrected" if allow_corrected_mutants else "as_scheduled",
        "scored": sum(1 for entry in scored if entry["outcome"] == "scored"),
        "strict_successes": sum(1 for entry in scored if entry.get("strict_success")),
        "runs": scored,
    }
    write_json(round_dir / "mutation-summary.json", summary)
    return summary


async def _score_pending(
    config: ExperimentConfig,
    pending: list[tuple[dict[str, Any], Path, Path, str, Path, int]],
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

    async def score_one(
        entry: dict[str, Any],
        candidate: Path,
        baseline: Path,
        target: str,
        manifest: Path,
        timeout: int,
    ) -> None:
        async with semaphore:
            # A run that cannot be scored is recorded as such rather than
            # aborting the round: one candidate whose own proof does not
            # reproduce at this timeout says nothing about the other cells, and
            # losing them to it would be an apparatus failure reported as an
            # absence of results.
            try:
                score = await score_mutants(
                    config,
                    candidate,
                    baseline,
                    target,
                    manifest,
                    timeout,
                )
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
                write_json(candidate.parent / "mutation-score.json", score)
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

    await asyncio.gather(*(score_one(*item) for item in pending))


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
    summary = asyncio.run(
        score_round(
            ExperimentConfig.load(args.config.resolve()),
            args.round_dir.resolve(),
            args.mutants_root.resolve(),
            args.timeout,
            args.allow_corrected_mutants,
            args.concurrency,
        )
    )
    print(json.dumps({k: v for k, v in summary.items() if k != "runs"}, sort_keys=True))


if __name__ == "__main__":
    main()
