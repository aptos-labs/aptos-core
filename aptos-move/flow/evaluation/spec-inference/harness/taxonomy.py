"""Turn machine-labelled transcripts into an adopted failure taxonomy.

The design's proposed diagnostic categories were written before any data. This
report holds each of them against what the rounds actually produced, so a
category enters the schema because it was observed rather than because it was
imagined.

A silent category is read carefully. It is evidence about the design only when
the corpus could have produced it, so each proposal is also judged on whether
any task required the contract category the failure needs.

The development corpus is small and its tasks are short. A category that is
rare here is not thereby rare in the benchmark corpus, whose framework code is
where frames and dependency contracts are difficult. This report governs what
to build next for development; it is not a licence to drop a category from the
benchmark's diagnostics.
"""

from __future__ import annotations

import argparse
import json
from collections import Counter
from pathlib import Path
from typing import Any

from .artifacts import write_json
from .mine import analyze_round


# Diagnostic categories proposed before any data was collected (see `DESIGN.md`,
# "Measuring cost"): the machine labels that would
# evidence each, and the contract category a task must require for the failure
# to be possible at all. `None` means reachability cannot be decided from the
# run manifests alone.
PROPOSED_CATEGORIES: tuple[tuple[str, tuple[str, ...], str | None], ...] = (
    ("invariant initialization", ("loop_invariant_base",), "loop-invariant"),
    ("invariant preservation", ("loop_invariant_induction",), "loop-invariant"),
    ("invariant insufficient at loop exit", ("loop_invariant",), "loop-invariant"),
    (
        "missing or overly broad abort behavior",
        ("abort_not_covered", "abort_never_happens"),
        "abort",
    ),
    ("incorrect normal-return behavior", ("postcondition",), "normal-result"),
    ("missing global frame", ("frame", "global_invariant"), "frame"),
    ("insufficient callee contract", (), None),
    ("forbidden weakening or edit-policy violation", (), ""),
    ("solver timeout", ("solver_timeout",), ""),
)

# Policy violation codes are recorded by the judge, not the verifier.
WEAKENING_CATEGORY = "forbidden weakening or edit-policy violation"


def build_report(runs_dir: Path, schedule_dir: Path | None = None) -> dict[str, Any]:
    mined = analyze_round(runs_dir, schedule_dir)
    observed = Counter(mined["failure_kinds"])
    violations = _policy_violations(runs_dir)
    terminal = Counter(item["terminal_status"] for item in mined["per_run"])

    required = _required_categories(runs_dir)
    categories = []
    for name, labels, requirement in PROPOSED_CATEGORIES:
        count = sum(observed.get(label, 0) for label in labels)
        evidence = sorted({label for label in labels if observed.get(label)})
        if name == WEAKENING_CATEGORY:
            count = sum(violations.values())
            evidence = sorted(violations)
        # A category that never fired says something about the design only if
        # the corpus could have produced it. Otherwise it says the corpus is
        # silent on the question.
        if requirement is None:
            reachable: bool | None = None
        elif requirement == "":
            reachable = True
        else:
            reachable = requirement in required
        if count:
            status = "observed"
        elif reachable is None:
            status = "reachability unknown"
        elif reachable:
            status = "reachable but never triggered"
        else:
            status = "unreachable in this corpus"
        categories.append(
            {
                "category": name,
                "observations": count,
                "evidence": evidence,
                "reachable": reachable,
                "requires_contract_category": requirement,
                "status": status,
                "note": "" if labels or count else "no machine label; needs hand labelling",
            }
        )

    unmapped = sorted(
        kind
        for kind in observed
        if not any(kind in labels for _, labels, _ in PROPOSED_CATEGORIES)
    )
    return {
        "schema_version": 1,
        "runs": mined["runs"],
        "terminal_statuses": dict(sorted(terminal.items())),
        "failure_kinds": dict(sorted(observed.items(), key=lambda item: -item[1])),
        "policy_violations": dict(sorted(violations.items(), key=lambda item: -item[1])),
        "required_contract_categories": sorted(required),
        "proposed_categories": categories,
        # Kinds the design never proposed. These are the categories the data
        # argues for adding, and they matter as much as the proposals it drops.
        "unproposed_kinds": unmapped,
        "adopted": sorted(
            {item["category"] for item in categories if item["status"] == "observed"}
            | set(unmapped)
        ),
    }


def _required_categories(runs_dir: Path) -> set[str]:
    """Contract categories the round's tasks actually require."""
    required: set[str] = set()
    for run_path in sorted(runs_dir.glob("*/run.json")) + sorted(
        runs_dir.glob("*/*/run.json")
    ):
        try:
            record = json.loads(run_path.read_text(encoding="utf-8"))
        except json.JSONDecodeError:
            continue
        required.update(record.get("required_contract_categories", []))
    return required


def _policy_violations(runs_dir: Path) -> Counter:
    counts: Counter = Counter()
    for judge_path in sorted(runs_dir.glob("*/judge.json")) + sorted(
        runs_dir.glob("*/*/judge.json")
    ):
        try:
            record = json.loads(judge_path.read_text(encoding="utf-8"))
        except json.JSONDecodeError:
            continue
        for key in ("final_judge", "eventual_judge"):
            verdict = ((record.get(key) or {}).get("verdict") or {})
            policy = verdict.get("policy") or {}
            for key_ in ("violations", "scope_violations"):
                for violation in policy.get(key_, []):
                    counts[violation.get("code", "unknown")] += 1
            coverage = policy.get("contract_coverage") or {}
            for violation in coverage.get("violations", []):
                counts[violation.get("code", "unknown")] += 1
    return counts


def render_markdown(report: dict[str, Any]) -> str:
    lines = [
        f"# Failure taxonomy over {report['runs']} runs",
        "",
        "## Terminal statuses",
        "",
        "| status | runs |",
        "|---|---:|",
    ]
    for status, count in report["terminal_statuses"].items():
        lines.append(f"| {status} | {count} |")
    lines += ["", "## Observed failure kinds", "", "| kind | count |", "|---|---:|"]
    for kind, count in report["failure_kinds"].items():
        lines.append(f"| {kind} | {count} |")
    if report["policy_violations"]:
        lines += ["", "## Policy violations", "", "| code | count |", "|---|---:|"]
        for code, count in report["policy_violations"].items():
            lines.append(f"| {code} | {count} |")
    lines += [
        "",
        "## Categories proposed by the feedback design",
        "",
        "A category that never fired is evidence about the design only when the",
        "corpus could have produced it.",
        "",
        "| proposed category | status | observations | evidence |",
        "|---|---|---:|---|",
    ]
    for item in report["proposed_categories"]:
        evidence = ", ".join(item["evidence"]) or item["note"] or "—"
        lines.append(
            f"| {item['category']} | {item['status']} | {item['observations']} | {evidence} |"
        )
    if report["unproposed_kinds"]:
        lines += [
            "",
            "## Kinds the design did not propose",
            "",
            "The data argues for these as much as it argues against the unobserved",
            "proposals.",
            "",
        ]
        lines += [f"- `{kind}`" for kind in report["unproposed_kinds"]]
    lines += ["", "## Adopted schema", ""]
    lines += [f"- {name}" for name in report["adopted"]]
    return "\n".join(lines) + "\n"


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--runs-dir", type=Path, required=True)
    parser.add_argument(
        "--schedule-dir",
        type=Path,
        help="the round's schedule, so cells that never produced an artifact are reported",
    )
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--markdown", type=Path)
    args = parser.parse_args()
    report = build_report(
        args.runs_dir.resolve(),
        args.schedule_dir.resolve() if args.schedule_dir else None,
    )
    write_json(args.output, report)
    if args.markdown:
        args.markdown.parent.mkdir(parents=True, exist_ok=True)
        args.markdown.write_text(render_markdown(report), encoding="utf-8")
    print(json.dumps({"runs": report["runs"], "adopted": len(report["adopted"])}))


if __name__ == "__main__":
    main()
