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
import json
from pathlib import Path
from typing import Any

from .artifacts import sha256_file, write_json
from .config import ExperimentConfig
from .mutants import NO_MUTANTS, score_mutants


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
        # judge at all, so the key is present and null.
        judge = (record.get("result") or {}).get("eventual_judge") or {}
        status = judge.get("state")
        entry: dict[str, Any] = {
            "run_id": run_id,
            "task_id": task_id,
            "arm": record.get("arm"),
            "terminal_status": status,
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
            # A run that cannot be scored is recorded as such rather than
            # aborting the round: one candidate whose own proof does not
            # reproduce at this timeout says nothing about the other cells, and
            # losing them to it would be an apparatus failure reported as an
            # absence of results.
            pending.append(
                (
                    entry,
                    artifact,
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
    pending: list[tuple[dict[str, Any], Path, str, Path, int]],
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
        artifact: Path,
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
                    artifact / "workspace",
                    artifact / "baseline",
                    target,
                    manifest,
                    timeout,
                )
            except ValueError as error:
                entry["outcome"] = "not_scorable"
                entry["detail"] = str(error)
            else:
                write_json(artifact / "mutation-score.json", score)
                entry["outcome"] = "scored"
                entry["mutation_adequacy"] = score["mutation_adequacy"]
                entry["strict_success"] = score["killed"] == score["essential_mutants"]

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
