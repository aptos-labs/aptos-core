#!/usr/bin/env python3
"""Re-derive corpus-v4's committed evidence from the pinned sources.

Rebuilds the package and checks every generated file against the manifest,
assembles the reference packages, and re-validates both mutant sets of every
ready task with `harness.validate_mutants`, which proves the reference, checks
it for vacuity and its implementation against the package, and re-runs every
mutant. Validation rewrites `mutants*/TASK/mutants.json` in place, so a clean
`git diff` afterwards means the committed records were reproduced.

Run from the evaluation root with the pinned `move-flow` first on PATH:

    python3 corpus-v4/verify.py
    python3 corpus-v4/verify.py --tasks TR-match-029 JW-upsert-049 --timeout 40
"""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent
EVALUATION = ROOT.parent
sys.path.insert(0, str(EVALUATION))

from harness.identifiers import module_name, require_plain_name  # noqa: E402

MUTANT_SETS = ("mutants", "mutants-scoring")


def run(command: list[str]) -> subprocess.CompletedProcess:
    return subprocess.run(command, cwd=EVALUATION, capture_output=True, text=True)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--tasks", nargs="+", help="only these task ids (default: every ready task)")
    parser.add_argument("--timeout", type=int, default=20, help="per-proof solver budget in seconds")
    args = parser.parse_args()

    for step, command in (
        ("package", [sys.executable, str(ROOT / "build.py")]),
        ("package digests", [sys.executable, str(ROOT / "build.py"), "--verify"]),
        ("references", [sys.executable, str(ROOT / "build_references.py")]),
    ):
        result = run(command)
        if result.returncode != 0:
            raise SystemExit(f"{step} failed:\n{result.stderr or result.stdout}")
        print(f"ok   {step}", flush=True)

    manifest = json.loads((ROOT / "manifest.json").read_text(encoding="utf-8"))
    records = [r for r in manifest["records"] if r["screening_status"] == "ready"]
    if args.tasks:
        unknown = sorted(set(args.tasks) - {r["task_id"] for r in records})
        if unknown:
            raise SystemExit(f"not ready tasks of this corpus: {', '.join(unknown)}")
        records = [r for r in records if r["task_id"] in args.tasks]

    problems = 0
    for record in records:
        task_id = require_plain_name(record["task_id"], "task_id")
        reference = ROOT / "references" / "build" / module_name(record["module"])
        for mutant_set in MUTANT_SETS:
            manifest_path = ROOT / mutant_set / task_id / "mutants.json"
            start = time.time()
            result = run([
                sys.executable, "-m", "harness.validate_mutants",
                "--config", "config/default.json",
                "--reference", str(reference),
                "--baseline", str(ROOT / "package"),
                "--target", record["target"],
                "--mutants", str(manifest_path),
                "--timeout", str(args.timeout),
            ])
            elapsed = time.time() - start
            if result.returncode != 0:
                problems += 1
                print(f"FAIL {task_id} {mutant_set} ({elapsed:.0f}s): "
                      f"{(result.stderr or result.stdout).strip()[-400:]}", flush=True)
                continue
            cases = json.loads(manifest_path.read_text(encoding="utf-8"))["mutants"]
            survivors = [c["mutant_id"] for c in cases if c["validated"]["outcome"] != "killed"]
            problems += bool(survivors)
            state = "ok  " if not survivors else "BAD "
            print(f"{state} {task_id} {mutant_set} ({elapsed:.0f}s): "
                  f"{len(cases) - len(survivors)}/{len(cases)} killed", flush=True)

    if problems:
        raise SystemExit(f"{problems} problem(s)")
    print("every reference proves and kills every mutant; `git diff` should now be empty")


if __name__ == "__main__":
    main()
