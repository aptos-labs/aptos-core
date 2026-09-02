#!/usr/bin/env python3
"""Choose a treatment-blind subset of the corpus for one benchmark round.

A full round costs one model session per task, arm and replicate, so a round
may run a subset. Which subset is a corpus decision, and it has to be made
without reference to any arm's behaviour -- so this reads only the corpus's own
description of each task and its target source. Nothing is deleted: every
sample stays in the manifest, and the selection is recorded as a field.

The rule, in order:

1. Keep every task that uniquely carries a feature stratum. Dropping one of
   these removes a capability from the benchmark outright, which no saving of
   sessions justifies.
2. Within a redundancy cluster -- tasks with identical strata, or whose target
   sources are near-duplicates -- keep one representative: the largest target,
   as a proxy for the richest contract to state.
3. Fill the remaining slots by rarity-weighted distinctiveness, so a task
   carrying rare strata outranks one carrying common ones.
4. Cap the guessable tasks, which exist for difficulty spread rather than for
   discrimination between arms.

Run from the evaluation root:

    python3 corpus-v3/select_round.py --size 16 --write
"""

from __future__ import annotations

import argparse
import collections
import difflib
import itertools
import json
import re
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parent
SOURCES = ROOT / "package" / "sources"
#: Two spellings of one idea in the hand-written manifest.
STRATUM_ALIASES = {"loop": "loops"}
#: Target sources at least this similar pose the same specification problem.
SOURCE_SIMILARITY = 0.50


def strata(record: dict[str, Any]) -> set[str]:
    return {STRATUM_ALIASES.get(x, x) for x in record["feature_strata"]}


def target_source(record: dict[str, Any]) -> str:
    """The target function's body, comments and layout normalised away."""
    name = record["module"].split("::")[-1]
    matches = [
        path
        for path in SOURCES.rglob("*.move")
        if path.stem == name and not path.name.endswith(".spec.move")
    ]
    if not matches:
        raise SystemExit(f"no source for {record['module']}; run build.py first")
    text = matches[0].read_text(encoding="utf-8")
    start = re.search(rf"\bfun\s+{re.escape(record['function'])}\b", text)
    if start is None:
        raise SystemExit(f"no `{record['function']}` in {matches[0]}")
    opening = text.index("{", start.end())
    depth, index = 0, opening
    while index < len(text):
        if text[index] == "{":
            depth += 1
        elif text[index] == "}":
            depth -= 1
            if depth == 0:
                break
        index += 1
    body = text[start.start() : index + 1]
    body = re.sub(r"//[^\n]*", "", body)
    body = re.sub(r"/\*.*?\*/", "", body, flags=re.S)
    return re.sub(r"\s+", " ", body).strip()


def select(records: list[dict[str, Any]], size: int, max_guessable: int) -> dict[str, Any]:
    ready = [r for r in records if r["screening_status"] == "ready"]
    if size > len(ready):
        raise SystemExit(f"asked for {size} tasks, only {len(ready)} are ready")
    by_id = {r["task_id"]: r for r in ready}
    sources = {r["task_id"]: target_source(r) for r in ready}
    rarity = collections.Counter(x for r in ready for x in strata(r))

    similar = [
        (round(ratio, 3), a, b)
        for a, b in itertools.combinations(sorted(by_id), 2)
        if (ratio := difflib.SequenceMatcher(None, sources[a], sources[b]).ratio())
        >= SOURCE_SIMILARITY
    ]

    clusters: list[list[str]] = []
    identical: dict[frozenset[str], list[str]] = collections.defaultdict(list)
    for record in ready:
        identical[frozenset(strata(record))].append(record["task_id"])
    clusters += [sorted(v) for v in identical.values() if len(v) > 1]
    clusters += [sorted((a, b)) for _, a, b in similar]

    def richness(task_id: str) -> tuple:
        record = by_id[task_id]
        weighted = sum(1 / rarity[x] for x in strata(record))
        return (weighted, len(sources[task_id]), task_id)

    unique_carriers = {
        r["task_id"] for r in ready if any(rarity[x] == 1 for x in strata(r))
    }
    keep = set(unique_carriers)
    held: dict[str, str] = {}
    # A task can belong to more than one cluster -- identical strata and
    # near-duplicate source are different tests -- so every winner is decided
    # before any loser is recorded. Interleaving the two let a later cluster
    # hold back a task an earlier one had already selected, leaving it in both
    # sets while `round_selection` followed only one of them.
    reasons: dict[str, str] = {}
    winners: set[str] = set()
    for cluster in clusters:
        members = [t for t in cluster if t not in unique_carriers]
        if len(members) < 2:
            continue
        # The largest target: identical strata mean the same kind of problem,
        # so the richest contract is the one worth keeping.
        winner = max(members, key=lambda t: (len(sources[t]), t))
        winners.add(winner)
        for task_id in members:
            if task_id != winner:
                shared = sorted(strata(by_id[task_id]) & strata(by_id[winner]))
                reasons.setdefault(task_id, f"redundant with {winner} on {shared}")
    keep |= winners
    held.update({t: reason for t, reason in reasons.items() if t not in keep})

    def guessable(selection: set[str]) -> int:
        return sum(1 for t in selection if by_id[t]["guessability"] == "guessable")

    for task_id in sorted(set(by_id) - keep - set(held), key=richness, reverse=True):
        if len(keep) >= size:
            break
        if by_id[task_id]["guessability"] == "guessable" and guessable(keep) >= max_guessable:
            held[task_id] = f"guessable, and the round already holds {max_guessable}"
            continue
        keep.add(task_id)

    if len(keep) > size:
        raise SystemExit(
            f"{len(keep)} tasks are required by the rule but the round takes {size}; "
            "raise --size or the benchmark loses a capability"
        )
    for task_id in sorted(set(by_id) - keep):
        held.setdefault(task_id, "lower rarity-weighted distinctiveness")

    selected = sorted(keep)
    overlap = sorted(set(selected) & set(held))
    if overlap:
        raise SystemExit(f"selection is inconsistent: {overlap} are both kept and held")
    covered = collections.Counter(x for t in selected for x in strata(by_id[t]))
    return {
        "schema_version": 1,
        "size": len(selected),
        "max_guessable": max_guessable,
        "source_similarity_threshold": SOURCE_SIMILARITY,
        "selected": selected,
        "held_back": {t: held[t] for t in sorted(held)},
        "unique_stratum_carriers": sorted(unique_carriers),
        "near_duplicate_targets": [
            {"similarity": s, "tasks": [a, b]} for s, a, b in sorted(similar, reverse=True)
        ],
        "strata_covered": len(covered),
        "strata_total": len(rarity),
        "strata_lost": sorted(set(rarity) - set(covered)),
        "loops": sum(1 for t in selected if by_id[t]["has_loop"]),
        "guessable": guessable(set(selected)),
        "modules": len({by_id[t]["module"] for t in selected}),
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--size", type=int, default=16)
    parser.add_argument("--max-guessable", type=int, default=2)
    parser.add_argument(
        "--write",
        action="store_true",
        help="record the selection in the manifest and in metadata/selection.json",
    )
    args = parser.parse_args()

    manifest_path = ROOT / "manifest.json"
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    result = select(manifest["records"], args.size, args.max_guessable)

    if result["strata_lost"]:
        raise SystemExit(f"selection loses strata: {result['strata_lost']}")

    print(json.dumps(result, indent=1))
    if not args.write:
        return

    selected = set(result["selected"])
    for record in manifest["records"]:
        if record["screening_status"] != "ready":
            record["round_selection"] = "not_ready"
        else:
            record["round_selection"] = (
                "selected" if record["task_id"] in selected else "held_back"
            )
    manifest_path.write_text(
        json.dumps(manifest, indent=1, sort_keys=True) + "\n", encoding="utf-8"
    )
    metadata = ROOT / "metadata"
    metadata.mkdir(exist_ok=True)
    (metadata / "selection.json").write_text(
        json.dumps(result, indent=1, sort_keys=True) + "\n", encoding="utf-8"
    )
    print(f"\nrecorded in {manifest_path} and {metadata / 'selection.json'}")


if __name__ == "__main__":
    main()
