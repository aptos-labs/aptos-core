#!/usr/bin/env python

# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

"""Puts the medians of several sweeps side by side.

Each argument is `label=dir`, where `dir` holds the `medians.json` files a
sweep wrote (one per workload artifact, any depth). Prints, per workload and
metric, one column per sweep and VM at the chosen thread counts.

    compare_sweeps.py --threads 2,16 base=run-a fixes=run-b
"""

import argparse
import glob
import json
import os

from tabulate import tabulate

METRICS = [
    ("inner wall", "inner_us"),
    ("execution wall", "execution_us"),
    ("worker cpu", "worker_cpu_total_us"),
    ("  execute_vm", "worker_cpu_execute_vm_us"),
    ("  execute_apply", "worker_cpu_execute_apply_us"),
    ("  materialize", "worker_cpu_materialize_us"),
    ("  commit_hooks", "worker_cpu_commit_hooks_us"),
    ("  scheduler", "worker_cpu_scheduler_us"),
    ("process system cpu", "process_system_cpu_us"),
    ("cold reads /txn", "cold_reads_per_txn"),
    ("cold reads us", "cold_read_us"),
    ("minor faults /txn", "minor_faults_per_txn"),
    ("aborts /txn", "aborts_per_txn"),
    ("inner TPS", "inner_tps"),
]


def load(directory):
    medians = {}
    for path in glob.glob(os.path.join(directory, "**", "medians.json"), recursive=True):
        with open(path) as f:
            for key, value in json.load(f).items():
                workload, vm, threads = key.split("|")
                medians[(workload, vm, int(threads))] = value
    return medians


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("sweeps", nargs="+", help="label=dir")
    ap.add_argument("--threads", default="2,16")
    ap.add_argument("--vms", default="v1,mono")
    args = ap.parse_args()
    threads = [int(t) for t in args.threads.split(",")]
    vms = args.vms.split(",")
    sweeps = [(s.split("=", 1)[0], load(s.split("=", 1)[1])) for s in args.sweeps]
    workloads = sorted({w for _, m in sweeps for (w, _, _) in m})

    for workload in workloads:
        print(f"### {workload}")
        print()
        headers = ["metric", "vm", "T"] + [label for label, _ in sweeps]
        rows = []
        for label, key in METRICS:
            for vm in vms:
                for t in threads:
                    cells = [label, vm, t]
                    base = None
                    for _, medians in sweeps:
                        med = medians.get((workload, vm, t))
                        if med is None or key not in med:
                            cells.append("-")
                            continue
                        value = med[key]
                        if base is None:
                            base = value
                            cells.append(f"{value:.2f}")
                        else:
                            delta = (value / base - 1) * 100 if base else 0.0
                            cells.append(f"{value:.2f} ({delta:+.0f}%)")
                    rows.append(cells)
        print(tabulate(rows, headers=headers, tablefmt="github"))
        print()


if __name__ == "__main__":
    main()
