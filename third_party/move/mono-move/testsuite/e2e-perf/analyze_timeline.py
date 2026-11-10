#!/usr/bin/env python

# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

"""Summarizes block timelines the benchmark writes with `BLOCK_TIMELINE_PATH`.

For each file: per stage, the time inside a block and the gap between
consecutive blocks on that stage, per transaction; how long a block waited
between stages; and the executor's split into state view, Block-STM and
output parsing. The first block of each stage is skipped (warm-up).

    analyze_timeline.py timeline/*.jsonl
"""

import argparse
import json
import statistics
from collections import defaultdict

from tabulate import tabulate

EXECUTE_PARTS = ["get_state_view", "vm_execute_block", "parse_raw_output"]


def load(path):
    by_stage = defaultdict(dict)
    with open(path) as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            rec = json.loads(line)
            by_stage[rec["stage"]][rec["block"]] = rec
    return by_stage


def summarize(path):
    by_stage = load(path)
    rows = []
    execute = by_stage.get("execute", {})
    for stage in ["execute", "ledger_update", "commit"]:
        blocks = by_stage.get(stage)
        if not blocks:
            continue
        ordered = [blocks[b] for b in sorted(blocks)][1:]
        if len(ordered) < 2:
            continue
        inside = [(r["end"] - r["start"]) * 1e6 / r["num_txns"] for r in ordered]
        gaps = [
            (b["start"] - a["end"]) * 1e6 / b["num_txns"]
            for a, b in zip(ordered, ordered[1:])
        ]
        waits = []
        if stage != "execute":
            prev = "execute" if stage == "ledger_update" else "ledger_update"
            for r in ordered:
                p = by_stage.get(prev, {}).get(r["block"])
                if p:
                    waits.append((r["start"] - p["end"]) * 1e6 / r["num_txns"])
        row = [
            stage,
            f"{statistics.median(inside):.2f}",
            f"{statistics.median(gaps):.2f}",
            f"{statistics.median(waits):.2f}" if waits else "-",
            f"{1e6 / statistics.median(inside):.0f}",
        ]
        if stage == "execute":
            parts = {
                name: statistics.median(r[name] * 1e6 / r["num_txns"] for r in ordered)
                for name in EXECUTE_PARTS
                if name in ordered[0]
            }
            row.append(" ".join(f"{k}={v:.2f}" for k, v in parts.items()))
        else:
            row.append("")
        rows.append(row)
    span = max(r["end"] for s in by_stage.values() for r in s.values()) - min(
        r["start"] for s in by_stage.values() for r in s.values()
    )
    txns = sum(r["num_txns"] for r in execute.values())
    print(f"### {path}: {txns} txns over {span:.2f} s = {txns / span:.0f} TPS end to end")
    print(
        tabulate(
            rows,
            headers=[
                "stage",
                "inside block us/txn",
                "gap to next block us/txn",
                "wait from previous stage us/txn",
                "stage TPS",
                "execute split us/txn",
            ],
            tablefmt="github",
        )
    )
    print()


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("files", nargs="+")
    args = ap.parse_args()
    for path in args.files:
        summarize(path)


if __name__ == "__main__":
    main()
