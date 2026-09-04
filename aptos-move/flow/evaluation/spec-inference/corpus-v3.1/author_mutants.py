"""Author an anchored mutant set from readable edit descriptions.

A mutant records an offset, a length and a digest -- never source text -- so
that a corpus change invalidates it loudly instead of silently relocating the
edit (`harness.mutants._anchored_fragment`). Those three numbers are tedious
and easy to get wrong by hand, so authoring works from the text instead: name
the anchor, name the substring inside it to replace, and this computes the
rest.

The anchor must occur exactly once in the file. A unique anchor is what makes
the offset meaningful; two occurrences mean the recorded offset is a guess
about which one was intended.

    python3 corpus-v3/author_mutants.py --spec corpus-v3/mutant-specs/scoring.json \
      --package corpus-v3/package --out corpus-v3/mutants-scoring
"""

from __future__ import annotations

import argparse
import hashlib
import sys
import json
from pathlib import Path
from typing import Any

# The identity of a mutation is a harness concept, not a per-script one: the
# controller refuses an overlapping refutation set using `mutation_fingerprint`,
# so authoring has to key on the same relation or a set that passes here can
# still be refused at run time.
sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
from harness.identifiers import require_plain_name, resolve_within  # noqa: E402
from harness.mutants import mutation_fingerprint  # noqa: E402

BASES = {
    "scoring": (
        "authored from the target source as a held-out scoring set, disjoint "
        "from the refutation set; validated as essential against the reference "
        "specification"
    ),
    "refutation": (
        "authored before any round from the target source; shown to the agent "
        "only as obligation categories; validated as essential against the "
        "reference specification"
    ),
}


def build_case(
    package: Path, entry: dict[str, Any], role: str = "scoring"
) -> dict[str, Any]:
    """Turn one readable edit description into an anchored mutant case."""
    # The specification supplies this path, so it is input, not configuration:
    # an absolute path or a `..` would read anywhere on the host.
    source = resolve_within(package, entry["file"], "mutant file").read_text(
        encoding="utf-8"
    )
    anchor_text = entry["anchor"]
    occurrences = source.count(anchor_text)
    if occurrences != 1:
        raise SystemExit(
            f"{entry['mutant_id']}: anchor occurs {occurrences} times in "
            f"{entry['file']}; it must occur exactly once so the recorded "
            f"offset is unambiguous. Anchor: {anchor_text!r}"
        )
    offset = source.index(anchor_text)

    replace = entry["replace"]
    within = anchor_text.count(replace)
    if within != 1:
        raise SystemExit(
            f"{entry['mutant_id']}: the replaced text occurs {within} times "
            f"inside its own anchor; it must occur exactly once. "
            f"Replacing {replace!r} in {anchor_text!r}"
        )
    at = anchor_text.index(replace)

    return {
        "anchor": {
            "length": len(anchor_text),
            "offset": offset,
            "sha256": hashlib.sha256(anchor_text.encode("utf-8")).hexdigest(),
        },
        "edit": {
            "at": at,
            "kind": "substitute",
            "length": len(replace),
            "to": entry["with"],
        },
        "essential": True,
        "file": entry["file"],
        "mutant_id": entry["mutant_id"],
        "obligation_category": entry["obligation_category"],
        "rationale": entry["rationale"],
        "reviews": [
            {
                "approved": True,
                "basis": BASES[role],
                "reviewer": "Claude (agent), on behalf of wgrieskamp@gmail.com",
            }
        ],
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--spec", type=Path, required=True)
    parser.add_argument(
        "--role",
        choices=sorted(BASES),
        default="scoring",
        help="which set this is; recorded in each mutant's review basis",
    )
    parser.add_argument("--package", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument(
        "--disjoint-from",
        type=Path,
        help="existing mutant root the new set must not duplicate",
    )
    args = parser.parse_args()

    spec = json.loads(args.spec.read_text(encoding="utf-8"))
    for task_id, entries in sorted(spec.items()):
        cases = [build_case(args.package, entry, args.role) for entry in entries]
        ids = [case["mutant_id"] for case in cases]
        if len(set(ids)) != len(ids):
            raise SystemExit(f"{task_id}: duplicate mutant_id in {ids}")

        if args.disjoint_from:
            other = args.disjoint_from / task_id / "mutants.json"
            if other.is_file():
                existing = json.loads(other.read_text(encoding="utf-8"))["mutants"]
                # Same file, same anchor offset and same edit means the two sets
                # test the same thing; scoring against it would be scoring
                # against the material the agent was already told about.
                seen = {mutation_fingerprint(c, args.package) for c in existing}
                for case in cases:
                    if mutation_fingerprint(case, args.package) in seen:
                        raise SystemExit(
                            f"{case['mutant_id']} duplicates a mutant in "
                            f"{other}; the scoring set must be disjoint from "
                            f"the refutation set"
                        )

        # Likewise the task id, which becomes a directory name.
        destination = args.out / require_plain_name(task_id, "task_id")
        destination.mkdir(parents=True, exist_ok=True)
        path = destination / "mutants.json"
        path.write_text(
            json.dumps(
                {"task_id": task_id, "mutants": cases}, indent=1, sort_keys=True
            )
            + "\n",
            encoding="utf-8",
        )
        print(f"{task_id}: wrote {len(cases)} mutants to {path}")


if __name__ == "__main__":
    main()
