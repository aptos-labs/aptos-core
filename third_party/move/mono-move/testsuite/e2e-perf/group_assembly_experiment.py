#!/usr/bin/env python

# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

"""Where should MonoMove assemble resource-group blobs?

Replays the same recorded blocks on MonoMove under each `MONO_GROUP_ASSEMBLY`
variant and thread count, alternating variants within every repeat:

- `per_txn`: each transaction assembles its groups at commit (the default).
- `block_end`: transactions emit member changes; the executor assembles all
  groups at the end of the block, before indexing.
- `block_end_flat`: like `block_end`, and Block-STM versions group members
  under their own keys instead of inside a group.
- `chained`: Block-STM versions group members under their own keys, and each
  transaction assembles its groups at commit from the previous writer's view
  of the group, found through a chain of links at the group's slot.

A block-end variant may name the pass shape after a slash
(`GROUP_ASSEMBLY_PASS`): `walk` applies all changes in one serial walk and
encodes the snapshots in parallel, `by_group` buckets the changes by group and
replays each group in its own parallel task.

All variants must leave the same ledger root hash: the group blobs they write
are byte-identical. The replays use `--split-stages`, so the execution stage
runs alone.

Run locally:

    REPEATS=1 NUM_BLOCKS_PER_TEST=3 NUM_INIT_ACCOUNTS=20000 THREADS=1,4 \\
      WORKLOADS=apt-fa-transfer OUT_DIR=/tmp/groups \\
      python3 third_party/move/mono-move/testsuite/e2e-perf/group_assembly_experiment.py

Summarize the JSON lines of several runs (e.g. one per CI job):

    REPORT_FROM=/path/to/dir python3 .../group_assembly_experiment.py
"""

import glob
import json
import os
import re
import shutil
import statistics
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from run_e2e_perf_test import (  # noqa: E402
    BUILD_FLAG,
    BUILD_FOLDER,
    MONO_MOVE_FLAG,
    WORKLOADS,
    CommandFailed,
    create_db,
    execute_command,
    record,
)
from tabulate import tabulate  # noqa: E402

WORKLOAD_NAMES = os.environ.get("WORKLOADS", "apt-fa-transfer")
THREADS = [int(t) for t in os.environ.get("THREADS", "1,16").split(",") if t]
VARIANTS = [
    v
    for v in os.environ.get(
        "VARIANTS", "per_txn,block_end/walk,block_end/by_group,block_end_flat/by_group"
    ).split(",")
    if v
]
MODES = {
    "per_txn": "PerTxn",
    "block_end": "BlockEnd",
    "block_end_flat": "BlockEndFlat",
    "chained": "Chained",
}
PASSES = {"walk": "Walk", "by_group": "ByGroup"}
BASELINE = VARIANTS[0]
REPEATS = int(os.environ.get("REPEATS", default=5))
OUT_DIR = os.environ.get("OUT_DIR", default="group-assembly-out")
WORK_DIR = os.environ.get("WORK_DIR", default=os.path.join(OUT_DIR, "work"))
SKIP_BUILD = bool(os.environ.get("SKIP_BUILD"))
REPORT_FROM = os.environ.get("REPORT_FROM")

NUMBER = r"(-?\d+\.?\d*(?:e[-+]?\d+)?)"


def get_last(pattern, output, what):
    matches = re.findall(pattern, output)
    if not matches:
        raise ValueError(f"no {what} in the output (pattern {pattern!r})")
    return matches[-1]


def component_tps(pattern, output, what):
    return float(get_last(pattern + r".*?\(component TPS: " + NUMBER + r"\)", output, what))


def replay(workload, threads, variant, recorded_db, checkpoint_dir, blocks_path):
    base, *flags = variant.split("+")
    mode, _, assembly_pass = base.partition("/")
    assembly_pass = assembly_pass or "walk"
    members_first = False
    poll_env = ""
    poll = None
    for flag in flags:
        if flag == "members_first":
            members_first = True
        elif flag.startswith("poll="):
            poll = int(flag[len("poll="):])
            poll_env = f"MONO_POLL_INTERVAL={poll} "
        else:
            raise ValueError(f"unknown variant flag {flag!r}")
    output = execute_command(
        f"RUST_BACKTRACE=1 MONO_GROUP_ASSEMBLY={mode} GROUP_ASSEMBLY_PASS={assembly_pass} "
        f"MONO_WRITE_MEMBERS_FIRST={int(members_first)} {poll_env}"
        f"{BUILD_FOLDER}/aptos-executor-benchmark "
        f"--block-executor-type aptos-vm-with-block-stm "
        f"--execution-threads {threads} --generate-then-execute --split-stages "
        f"--num-generator-workers 1 --block-size {workload.block_size} "
        f"run-executor --data-dir {recorded_db} --checkpoint-dir {checkpoint_dir} "
        f"--replay-blocks {blocks_path} --enable-feature-after-init {MONO_MOVE_FLAG}"
    )

    enabled, _ = get_last(
        r"Feature flag overrides after init: enable=\[([^\]]*)\] disable=\[([^\]]*)\]",
        output,
        "feature flag overrides",
    )
    if MONO_MOVE_FLAG not in enabled:
        raise ValueError("replay did not enable MonoMove")
    ran_mode, ran_pass, assembly = get_last(
        r"Overall group assembly \((\w+), (\w+)\): (.*)", output, "group assembly line"
    )
    if (ran_mode, ran_pass) != (MODES[mode], PASSES[assembly_pass]):
        raise ValueError(f"asked for {variant} but the benchmark ran {ran_mode}/{ran_pass}")
    execution_tps = component_tps(
        r"Overall fraction of total: \d+\.?\d* in execution", output, "execution TPS"
    )
    block_executor_tps = component_tps(
        r"Overall fraction of execution \d+\.?\d* in get execution output by executing",
        output,
        "block executor TPS",
    )
    inner_tps = component_tps(
        r"Overall fraction of execution \d+\.?\d* in inner block executor",
        output,
        "inner block executor TPS",
    )
    row = {
        "workload": workload.name,
        "threads": threads,
        "variant": variant,
        "total_tps": float(get_last(r"Overall TPS: " + NUMBER + r" txn/s", output, "TPS")),
        "execution_us": 1e6 / execution_tps,
        "block_executor_us": 1e6 / block_executor_tps,
        "inner_us": 1e6 / inner_tps,
        "aborts_per_txn": float(
            get_last(r"Overall speculative aborts: " + NUMBER + r" aborts/txn", output, "aborts")
        ),
        "output_bytes_per_txn": float(
            get_last(r"Overall output: " + NUMBER + r" bytes/txn", output, "bytes/txn")
        ),
        "root_hash": get_last(
            r"final state: version \d+, root hash ([0-9a-f]+)", output, "final root hash"
        ),
    }
    for key, value in re.findall(r"(\w+)=" + NUMBER, assembly):
        row[key] = float(value)
    phases = re.findall(r"Overall worker phases per txn:(.*)", output)
    if phases:
        for key, value in re.findall(r"(\w+)=" + NUMBER, phases[-1]):
            row["ph_" + key] = float(value)
    if bool(row.get("members_first")) != members_first:
        raise ValueError(f"asked for {variant} but members_first={row.get('members_first')}")
    if poll is not None and row.get("poll_interval") != poll:
        raise ValueError(f"asked for {variant} but poll_interval={row.get('poll_interval')}")
    return row


def run_workload(workload, db_dir, emit):
    work = os.path.join(WORK_DIR, workload.name)
    blocks_path = os.path.join(work, "blocks")
    recorded_db = os.path.join(work, "recorded-db")
    checkpoint = os.path.join(work, "cp")
    os.makedirs(work, exist_ok=True)
    record(workload, db_dir, recorded_db, blocks_path)

    failures = []
    for repeat in range(REPEATS):
        for threads in THREADS:
            # Rotating the variant order each repeat keeps any drift over the
            # run from always landing on the same variant.
            shift = repeat % len(VARIANTS)
            for variant in VARIANTS[shift:] + VARIANTS[:shift]:
                try:
                    row = replay(workload, threads, variant, recorded_db, checkpoint, blocks_path)
                except (CommandFailed, ValueError) as e:
                    failures.append(f"{workload.name} {threads}T {variant} repeat {repeat}: {e}")
                    print(f"FAILED: {failures[-1]}")
                    continue
                row["repeat"] = repeat
                emit(row)
    shutil.rmtree(work, ignore_errors=True)
    return failures


def median(rows, key):
    values = [r[key] for r in rows if key in r]
    return statistics.median(values) if values else float("nan")


def spread(rows, key):
    values = [r[key] for r in rows if key in r]
    if not values:
        return float("nan")
    m = statistics.median(values)
    return (max(values) - min(values)) / m if m else 0.0


def build_report(rows, failures):
    lines = ["### MonoMove group assembly: per transaction vs block end", ""]
    lines.append(
        "Microseconds per transaction, median over repeats. `exec` is the whole "
        "execution stage, `blk` is Block-STM plus output parsing (where block-end "
        "assembly runs), `inner` is Block-STM alone. Deltas are against "
        "the first variant listed; negative is faster. `±` is the (max - min) / median spread."
    )
    lines.append("")

    variants = variants_of(rows)
    baseline = variants[0] if variants else BASELINE
    by_workload = {}
    for row in rows:
        by_workload.setdefault(row["workload"], []).append(row)

    for workload in sorted(by_workload):
        wrows = by_workload[workload]
        hashes = {r["root_hash"] for r in wrows}
        lines.append(f"#### {workload}")
        lines.append("")
        if len(hashes) != 1:
            lines.append(f"**ROOT HASH MISMATCH**: {len(hashes)} distinct hashes across runs.")
            for variant in variants:
                vh = sorted({r["root_hash"][:16] for r in wrows if r["variant"] == variant})
                lines.append(f"- {variant}: {', '.join(vh)}")
        else:
            lines.append(f"Root hash identical across all {len(wrows)} runs.")
        lines.append("")

        table = []
        for threads in sorted({r["threads"] for r in wrows}):
            base_rows = [r for r in wrows if r["threads"] == threads and r["variant"] == baseline]
            for variant in variants:
                vrows = [r for r in wrows if r["threads"] == threads and r["variant"] == variant]
                if not vrows:
                    continue

                def cell(key):
                    value = median(vrows, key)
                    text = f"{value:.2f} ±{spread(vrows, key):.0%}"
                    if variant != baseline and base_rows:
                        base = median(base_rows, key)
                        if base:
                            text += f" ({(value - base) / base:+.1%})"
                    return text

                table.append(
                    [
                        threads,
                        variant,
                        len(vrows),
                        cell("execution_us"),
                        cell("block_executor_us"),
                        cell("inner_us"),
                        f"{median(vrows, 'per_txn_cpu_us'):.3f}",
                        f"{median(vrows, 'block_end_wall_us'):.3f}",
                        f"{median(vrows, 'block_end_serial_us'):.3f}",
                        f"{median(vrows, 'block_end_parallel_us'):.3f}",
                        f"{max(median(vrows, 'per_txn_groups'), median(vrows, 'block_end_groups')):.2f}",
                        f"{median(vrows, 'aborts_per_txn'):.3f}",
                    ]
                )
        lines.append(
            tabulate(
                table,
                headers=[
                    "T",
                    "variant",
                    "n",
                    "exec us",
                    "blk us",
                    "inner us",
                    "commit asm cpu us",
                    "block-end asm us",
                    "serial us",
                    "parallel us",
                    "groups/txn",
                    "aborts/txn",
                ],
                tablefmt="github",
            )
        )
        lines.append("")

    lines.extend(phase_report(rows))

    if failures:
        lines.append("#### Failures")
        lines.append("")
        lines.extend(f"- {f}" for f in failures)
        lines.append("")
    return "\n".join(lines)


PHASES = [
    "commit_hooks",
    "next_task",
    "vm_first",
    "vm_retry",
    "apply",
    "post_commit",
    "module_validation",
    "dependency_wait",
]


def variants_of(rows):
    """The variants in `rows`, in VARIANTS order when listed there."""
    seen = []
    for row in rows:
        if row["variant"] not in seen:
            seen.append(row["variant"])
    listed = [v for v in VARIANTS if v in seen]
    return listed + sorted(v for v in seen if v not in listed)


def phase_report(rows):
    """Worker time per BlockSTMv2 phase, summed over workers, in us per txn."""
    rows = [r for r in rows if "ph_vm_first_us" in r and r["threads"] > 1]
    if not rows:
        return []
    variants = variants_of(rows)
    lines = [
        "### Worker phases (BlockSTMv2, us per txn summed over workers)",
        "",
        "`workers x inner` is the worker time available; `phases` is what the phases account for.",
        "",
    ]
    for workload in sorted({r["workload"] for r in rows}):
        table = []
        for threads in sorted({r["threads"] for r in rows if r["workload"] == workload}):
            for variant in variants:
                vrows = [
                    r
                    for r in rows
                    if r["workload"] == workload
                    and r["threads"] == threads
                    and r["variant"] == variant
                ]
                if not vrows:
                    continue
                phase_us = [median(vrows, f"ph_{p}_us") for p in PHASES]
                table.append(
                    [threads, variant, f"{threads * median(vrows, 'inner_us'):.1f}", f"{sum(phase_us):.1f}"]
                    + [f"{v:.2f}" for v in phase_us]
                    + [
                        f"{median(vrows, 'ph_vm_retry_n'):.3f}",
                        f"{median(vrows, 'ph_dependency_wait_n'):.3f}",
                        f"{median(vrows, 'ph_speculative_failure_n'):.3f}",
                        f"{median(vrows, 'ph_poll_n'):.2f}",
                        f"{median(vrows, 'ph_read_us'):.2f}",
                        f"{median(vrows, 'ph_read_n'):.2f}",
                    ]
                )
        lines.append(f"#### {workload}")
        lines.append("")
        lines.append(
            tabulate(
                table,
                headers=["T", "variant", "workers x inner", "phases"]
                + PHASES
                + ["retries/txn", "waits/txn", "spec fails/txn", "polls/txn", "read us", "reads/txn"],
                tablefmt="github",
            )
        )
        lines.append("")
    return lines


def report_from(directory):
    rows, failures = [], []
    for path in sorted(glob.glob(os.path.join(directory, "**", "runs.jsonl"), recursive=True)):
        with open(path) as f:
            rows.extend(json.loads(line) for line in f if line.strip())
    for path in sorted(glob.glob(os.path.join(directory, "**", "failures.txt"), recursive=True)):
        with open(path) as f:
            failures.extend(line.strip() for line in f if line.strip())
    report = build_report(rows, failures)
    with open(os.path.join(directory, "report.md"), "w") as f:
        f.write(report)
    print(report)
    return 1 if failures or "ROOT HASH MISMATCH" in report else 0


def main():
    if REPORT_FROM:
        return report_from(REPORT_FROM)

    by_name = {w.name: w for w in WORKLOADS}
    names = [n.strip() for n in WORKLOAD_NAMES.split(",") if n.strip()]
    unknown = [n for n in names if n not in by_name]
    if unknown:
        print(f"unknown workloads: {unknown}")
        return 1

    os.makedirs(OUT_DIR, exist_ok=True)
    os.makedirs(WORK_DIR, exist_ok=True)
    if not SKIP_BUILD:
        execute_command(f"cargo build {BUILD_FLAG} --package aptos-executor-benchmark")

    db_dir = os.path.join(WORK_DIR, "db")
    create_db(db_dir)

    runs_path = os.path.join(OUT_DIR, "runs.jsonl")
    rows = []

    def emit(row):
        rows.append(row)
        with open(runs_path, "a") as f:
            f.write(json.dumps(row) + "\n")

    failures = []
    for name in names:
        failures += run_workload(by_name[name], db_dir, emit)

    with open(os.path.join(OUT_DIR, "failures.txt"), "w") as f:
        f.write("\n".join(failures))
    report = build_report(rows, failures)
    with open(os.path.join(OUT_DIR, "report.md"), "w") as f:
        f.write(report)
    print(report)
    return 1 if failures or "ROOT HASH MISMATCH" in report else 0


if __name__ == "__main__":
    sys.exit(main())
