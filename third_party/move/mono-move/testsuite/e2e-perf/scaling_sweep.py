#!/usr/bin/env python

# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

"""Thread-scaling sweep of MonoMove against the V1 MoveVM.

Replays the same recorded blocks on both VMs at several execution thread
counts and reports, per transaction, where the time goes: block executor wall
time, worker wall and CPU time by phase, process CPU, page faults and context
switches. Everything is in microseconds per transaction so that the growth from
the lowest to the highest thread count can be compared between the VMs
component by component.

The replays use `--split-stages`, so the execution stage runs alone and the
process-level counters over that stage belong to it.

Optionally records a `perf` profile of one extra replay per selected
configuration and writes folded stacks restricted to the execution stage.

Run locally:

    REPEATS=1 NUM_BLOCKS=20 NUM_INIT_ACCOUNTS=50000 THREADS=2,4 \\
      WORKLOADS=no-op OUT_DIR=/tmp/sweep \\
      python3 third_party/move/mono-move/testsuite/e2e-perf/scaling_sweep.py
"""

import gzip
import json
import os
import re
import shutil
import statistics
import subprocess
import sys
import time
from collections import defaultdict

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from run_e2e_perf_test import (  # noqa: E402
    BUILD_FLAG,
    BUILD_FOLDER,
    MONO_MOVE_FLAG,
    WORKLOADS,
    CommandFailed,
    execute_command,
)
from tabulate import tabulate  # noqa: E402

DEFAULT_WORKLOADS = "no-op,apt-fa-transfer,account-generation,order-book-no-matches1-market"

WORKLOAD_NAMES = os.environ.get("WORKLOADS", DEFAULT_WORKLOADS)
THREADS = [int(t) for t in os.environ.get("THREADS", "2,4,8,16").split(",")]
# Variants of each VM to replay, as `name:ENV=1 OTHER=2` entries separated by
# `;`, so that an A/B of a runtime knob runs on the same recorded blocks. The
# default is one unnamed variant with no extra environment.
VARIANTS = [
    (entry.split(":", 1)[0], entry.split(":", 1)[1] if ":" in entry else "")
    for entry in (os.environ.get("VARIANTS") or "default:").split(";")
    if entry
]
# A VM label is `v1` or `mono`, suffixed with `/variant` when there is more
# than one variant. Every table keys on the label.
VMS = [
    f"{vm}/{name}" if len(VARIANTS) > 1 else vm
    for vm in os.environ.get("VMS", "v1,mono").split(",")
    for name, _ in VARIANTS
]


def vm_and_variant(label):
    """Splits a VM label into the VM and its variant's environment prefix."""
    vm, _, name = label.partition("/")
    env = next((env for n, env in VARIANTS if n == name), "") if name else VARIANTS[0][1]
    return vm, (env + " " if env else "")
REPEATS = int(os.environ.get("REPEATS", default=3))
NUM_BLOCKS = int(os.environ.get("NUM_BLOCKS", default=100))
# Overrides every workload's block size when set.
BLOCK_SIZE = os.environ.get("BLOCK_SIZE")
# Consecutive transactions a sender has in a block. The benchmark's default is
# 5, which chains every sender's transactions through its sequence number and
# makes most transactions depend on the one before them. At 1 the only
# dependencies are the workload's own.
TXNS_PER_SENDER = int(os.environ.get("TXNS_PER_SENDER", default=1))
# Charges gas on MonoMove. Its gas schedule is a placeholder, so the amounts
# are wrong, but the fee is deducted, burnt and the supply updated, which is
# what shapes the write set. Unmetered, a transaction writes nothing for its
# fee and its store is only read.
MONO_METERED = bool(os.environ.get("MONO_METERED"))
# Run the stages one after another (the default), so that the execution
# stage's counters are its own, or pipelined as a node runs them.
SPLIT_STAGES = os.environ.get("SPLIT_STAGES", "1") != "0"
# Overlap each block's output indexing with the next block's execution.
PIPELINED = bool(os.environ.get("PIPELINED"))
NUM_INIT_ACCOUNTS = int(os.environ.get("NUM_INIT_ACCOUNTS", default=2000000))
CREATE_DB_THREADS = int(os.environ.get("CREATE_DB_THREADS", default=32))
OUT_DIR = os.environ.get("OUT_DIR", default="scaling-sweep-out")
# Where the DB and recordings go. They are large, so this should be a disk with
# room, not the artifact directory.
WORK_DIR = os.environ.get("WORK_DIR", default=os.path.join(OUT_DIR, "work"))
# `perf` records one extra replay per entry of PROFILE_CONFIGS; empty disables.
PROFILE = os.environ.get("PROFILE", default="")
PROFILE_CONFIGS = [
    tuple(c.split(":")) for c in os.environ.get("PROFILE_CONFIGS", "v1:2,v1:16,mono:2,mono:16").split(",") if c
]
PERF_FREQ = int(os.environ.get("PERF_FREQ", default=499))
PERF_BIN = os.environ.get("PERF_BIN", default="perf")
# Pin the benchmark to one logical CPU per physical core, so SMT siblings never
# share a core. Only matters on machines with SMT.
PIN_ONE_PER_CORE = bool(os.environ.get("PIN_ONE_PER_CORE"))
SKIP_BUILD = bool(os.environ.get("SKIP_BUILD"))

PHASES = [
    "commit_hooks",
    "scheduler",
    "execute_vm",
    "execute_vm_wasted",
    "execute_apply",
    "materialize",
    "module_validation",
]
NUMBER = r"(-?\d+\.?\d*(?:e[-+]?\d+)?)"


def selected_workloads():
    wanted = [name.strip() for name in WORKLOAD_NAMES.split(",") if name.strip()]
    by_name = {w.name: w for w in WORKLOADS}
    unknown = [name for name in wanted if name not in by_name]
    if unknown:
        raise SystemExit(f"Unknown workloads: {unknown}")
    return [by_name[name] for name in wanted]


def block_size_of(workload):
    return int(BLOCK_SIZE) if BLOCK_SIZE else workload.block_size


def taskset_prefix():
    if not PIN_ONE_PER_CORE:
        return ""
    out = subprocess.run(
        "lscpu -p=CPU,CORE,SOCKET", shell=True, text=True, capture_output=True
    ).stdout
    seen, cpus = set(), []
    for line in out.splitlines():
        if line.startswith("#") or not line.strip():
            continue
        cpu, core, socket = line.split(",")[:3]
        if (core, socket) not in seen:
            seen.add((core, socket))
            cpus.append(cpu)
    return f"taskset -c {','.join(cpus)} "


def common_flags(block_size, threads, db_dir, checkpoint_dir):
    """The benchmark invocation without any environment prefix, so that it can
    also be handed to `perf record`.
    """
    split = "--split-stages " if SPLIT_STAGES else ""
    return (
        f"{taskset_prefix()}{BUILD_FOLDER}/aptos-executor-benchmark "
        f"--block-executor-type aptos-vm-with-block-stm "
        f"--execution-threads {threads} --generate-then-execute {split}"
        f"--num-generator-workers 1 "
        f"--block-size {block_size} --transactions-per-sender {TXNS_PER_SENDER} "
        f"run-executor "
        f"--data-dir {db_dir} --checkpoint-dir {checkpoint_dir}"
    )


def build():
    execute_command(f"cargo build {BUILD_FLAG} --package aptos-executor-benchmark")


def create_db(db_dir, num_accounts):
    print(f"Warmup - creating DB with {num_accounts} accounts")
    execute_command(
        f"RUST_BACKTRACE=1 {BUILD_FOLDER}/aptos-executor-benchmark "
        f"--block-executor-type aptos-vm-with-block-stm "
        f"--block-size 1000 --execution-threads {CREATE_DB_THREADS} "
        f"create-db --data-dir {db_dir} --num-accounts {num_accounts}"
    )


def record(workload, db_dir, checkpoint_dir, blocks_path):
    block_size = block_size_of(workload)
    execute_command(
        f"RUST_BACKTRACE=1 {common_flags(block_size, 8, db_dir, checkpoint_dir)} "
        f"--transaction-type {workload.name} --module-working-set-size 1 "
        f"--main-signer-accounts {2 * block_size} "
        f"--additional-dst-pool-accounts {2 * block_size * NUM_BLOCKS} "
        f"--blocks {NUM_BLOCKS} --dump-blocks {blocks_path}"
    )


def replay_command(workload, threads, vm, recorded_db, checkpoint_dir, blocks_path, tag=None):
    """The environment prefix and the program of one replay, kept apart so a
    profiler can wrap the program alone. `tag` names the replay's block
    timeline file.
    """
    vm, variant_env = vm_and_variant(vm)
    override = "--enable-feature-after-init" if vm == "mono" else "--disable-feature-after-init"
    env = "RUST_BACKTRACE=1 " + variant_env
    if MONO_METERED and vm == "mono":
        env += "MONO_MOVE_METERED=1 "
    if PIPELINED:
        env += "APTOS_PIPELINED_EXECUTION=1 "
    if tag:
        timeline_dir = os.path.join(OUT_DIR, "timeline")
        os.makedirs(timeline_dir, exist_ok=True)
        env += f"BLOCK_TIMELINE_PATH={os.path.join(timeline_dir, tag + '.jsonl')} "
    program = (
        f"{common_flags(block_size_of(workload), threads, recorded_db, checkpoint_dir)} "
        f"--replay-blocks {blocks_path} {override} {MONO_MOVE_FLAG}"
    )
    return env, program


def get_last(pattern, output, what):
    matches = re.findall(pattern, output)
    if not matches:
        raise ValueError(f"no {what} in the output (pattern {pattern!r})")
    return matches[-1]


def parse_kv(line):
    return {k: float(v) for k, v in re.findall(r"(\w+)=" + NUMBER, line)}


def parse_run(output, workload, threads, vm, repeat):
    """Everything the sweep keeps from one replay, in microseconds per txn."""
    stage = "Overall execution resources: "
    inner_tps = float(
        get_last(
            r"Overall fraction of execution \d+\.?\d* in inner block executor \(component TPS: "
            + NUMBER
            + r"\)",
            output,
            "inner block executor TPS",
        )
    )
    execution_tps = float(
        get_last(
            r"Overall fraction of total: \d+\.?\d* in execution \(component TPS: " + NUMBER + r"\)",
            output,
            "execution TPS",
        )
    )
    row = {
        "workload": workload.name,
        "vm": vm,
        "threads": threads,
        "repeat": repeat,
        "block_size": block_size_of(workload),
        "blocks": NUM_BLOCKS,
        "txns_per_sender": TXNS_PER_SENDER,
        "mono_metered": MONO_METERED,
        "total_tps": float(get_last(r"Overall TPS: " + NUMBER + r" txn/s", output, "TPS")),
        "execution_tps": execution_tps,
        "inner_tps": inner_tps,
        "execution_us": 1e6 / execution_tps,
        "inner_us": 1e6 / inner_tps,
        "concurrency": int(
            get_last(r"Overall block executor concurrency: (\d+)", output, "concurrency")
        ),
        "aborts_per_txn": float(
            get_last(r"Overall speculative aborts: " + NUMBER + r" aborts/txn", output, "aborts")
        ),
        "output_bytes_per_txn": float(
            get_last(r"Overall output: " + NUMBER + r" bytes/txn", output, "bytes/txn")
        ),
    }

    enabled, disabled = get_last(
        r"Feature flag overrides after init: enable=\[([^\]]*)\] disable=\[([^\]]*)\]",
        output,
        "feature flag overrides",
    )
    mono_enabled = MONO_MOVE_FLAG in enabled
    if mono_enabled != (vm_and_variant(vm)[0] == "mono"):
        raise ValueError(f"{vm} run had MonoMove {'enabled' if mono_enabled else 'disabled'}")
    if row["concurrency"] != threads:
        raise ValueError(
            f"asked for {threads} execution threads but the block executor ran at "
            f"{row['concurrency']}"
        )

    window = parse_kv(get_last(stage + r"monotonic window secs: (.*)", output, "window"))
    row["window_start"] = window["start"]
    row["window_end"] = window["end"]
    for key, value in parse_kv(
        get_last(stage + r"block executor wall us/txn: (.*)", output, "block executor wall")
    ).items():
        row[f"{key}_us"] = value
    for clock in ("wall", "cpu"):
        for key, value in parse_kv(
            get_last(stage + rf"worker {clock} us/txn: (.*)", output, f"worker {clock}")
        ).items():
            row[f"worker_{clock}_{key}_us"] = value
        row[f"worker_{clock}_total_us"] = sum(row[f"worker_{clock}_{p}_us"] for p in PHASES)
    for key, value in parse_kv(
        get_last(stage + r"process cpu us/txn: (.*)", output, "process cpu")
    ).items():
        row[f"process_{key}_cpu_us"] = value
    row["process_cpu_us"] = row["process_user_cpu_us"] + row["process_system_cpu_us"]
    for key, value in parse_kv(
        get_last(stage + r"process counters per txn: (.*)", output, "process counters")
    ).items():
        row[f"{key}_per_txn"] = value
    cold = parse_kv(get_last(stage + r"cold state reads per txn: (.*)", output, "cold reads"))
    row["cold_reads_per_txn"] = cold["count"]
    row["cold_read_us"] = cold["us"]
    row["hot_promotions_per_txn"] = cold.get("promotions", 0.0)
    for key, value in parse_kv(
        get_last(stage + r"state view lookups per txn: (.*)", output, "state view lookups")
    ).items():
        row[f"{key}_per_txn"] = value
    row["interrupts_per_txn"] = float(
        get_last(stage + r"interrupted incarnations per txn: " + NUMBER, output, "interrupts")
    )
    row["predicted_stalls_per_txn"] = float(
        get_last(stage + r"predicted stalls per txn: " + NUMBER, output, "predicted stalls")
    )
    row["max_rss_kib"] = int(get_last(stage + r"max rss: (\d+) KiB", output, "max rss"))
    # The ledger's root hash after the replay: identical bytes in, identical
    # state out, so two replays of the same recording must agree.
    row["root_hash"] = get_last(
        r"final state: version \d+, root hash ([0-9a-f]+)", output, "final root hash"
    )
    row["workflow_timer_us"] = {
        name: float(value)
        for name, value in re.findall(stage + r"workflow timer us/txn: ([^=]+)=" + NUMBER, output)
    }
    row["thread_group_cpu_us"] = {
        group: float(value)
        for group, value in re.findall(stage + r"thread group cpu us/txn: ([^=]+)=" + NUMBER, output)
    }
    return row


def perf_available():
    return shutil.which(PERF_BIN) is not None


def fold_perf_script(script_path, folded_path, window):
    """Collapses `perf script` output into `comm;frame;...;leaf count` lines,
    keeping only samples inside the execution stage's monotonic window.

    Frames are outermost first. Rust hash suffixes and offsets are stripped so
    the same function folds together across samples.
    """
    start, end = window
    counts = defaultdict(int)
    comm, sample_time, frames = None, None, []
    hash_suffix = re.compile(r"::h[0-9a-f]{16}")

    def flush():
        if comm is not None and start <= sample_time <= end:
            counts[comm + ";" + ";".join(reversed(frames))] += 1

    with open(script_path, errors="replace") as f:
        for line in f:
            if not line.strip():
                flush()
                comm, sample_time, frames = None, None, []
                continue
            if not line.startswith(("\t", " ")):
                # Sample header: "comm tid time: ..." with comm possibly containing spaces.
                m = re.match(r"(.*?)\s+\d+(?:/\d+)?\s+" + NUMBER + r":", line)
                if m:
                    comm = m.group(1).strip()
                    sample_time = float(m.group(2))
                    frames = []
                continue
            # "\t     7f0c1234abcd symbol+0x1a (dso)"
            parts = line.strip().split(" ", 1)
            if len(parts) < 2:
                continue
            sym = parts[1]
            sym = re.sub(r"\s+\([^)]*\)$", "", sym)
            sym = re.sub(r"\+0x[0-9a-f]+$", "", sym)
            sym = hash_suffix.sub("", sym)
            frames.append(sym)
    flush()
    with gzip.open(folded_path, "wt") as f:
        for stack, count in sorted(counts.items(), key=lambda kv: -kv[1]):
            f.write(f"{stack} {count}\n")
    return sum(counts.values())


def profile_replay(workload, threads, vm, recorded_db, checkpoint_dir, blocks_path, prof_dir):
    """Records one replay under `perf` and summarizes it. Returns the parsed
    run, which is reported separately from the unprofiled repeats.
    """
    name = f"{workload.name}-{vm}-{threads}t"
    data = os.path.join(prof_dir, f"{name}.perf.data")
    env, program = replay_command(workload, threads, vm, recorded_db, checkpoint_dir, blocks_path)
    output = execute_command(
        f"{env}{PERF_BIN} record -F {PERF_FREQ} -g -k CLOCK_MONOTONIC -o {data} -- {program}"
    )
    row = parse_run(output, workload, threads, vm, repeat=-1)
    window = (row["window_start"], row["window_end"])
    time_filter = f"--time {window[0]:.6f},{window[1]:.6f}"

    for mode, flags in (
        ("self", "--no-children --sort comm,dso,sym --percent-limit 0.2"),
        ("children", "--children --sort sym --percent-limit 0.5"),
    ):
        report = os.path.join(prof_dir, f"{name}.{mode}.txt")
        execute_command(
            f"{PERF_BIN} report -i {data} {time_filter} {flags} --stdio > {report} 2>&1 || true"
        )

    script = os.path.join(prof_dir, f"{name}.script")
    execute_command(
        f"{PERF_BIN} script -i {data} {time_filter} -F comm,tid,time,ip,sym,dso > {script}"
    )
    folded = os.path.join(prof_dir, f"{name}.folded.gz")
    samples = fold_perf_script(script, folded, window)
    print(f"PROFILE {name}: {samples} samples in the execution window, folded to {folded}")
    os.remove(script)
    os.remove(data)
    row["profile"] = name
    return row


def median_rows(rows):
    """Per (workload, vm, threads), the median of every numeric metric across
    repeats, plus the spread of the inner block executor time.
    """
    groups = defaultdict(list)
    for row in rows:
        groups[(row["workload"], row["vm"], row["threads"])].append(row)
    out = {}
    for key, group in groups.items():
        med = {}
        for metric, value in group[0].items():
            if isinstance(value, (int, float)) and not isinstance(value, bool):
                med[metric] = statistics.median(r[metric] for r in group)
        groups_cpu = defaultdict(list)
        for r in group:
            for tg, v in r["thread_group_cpu_us"].items():
                groups_cpu[tg].append(v)
        med["thread_group_cpu_us"] = {tg: statistics.median(v) for tg, v in groups_cpu.items()}
        timers = defaultdict(list)
        for r in group:
            for name, v in r.get("workflow_timer_us", {}).items():
                timers[name].append(v)
        med["workflow_timer_us"] = {name: statistics.median(v) for name, v in timers.items()}
        inner = [r["inner_us"] for r in group]
        med["inner_spread"] = (max(inner) - min(inner)) / statistics.median(inner)
        med["repeats"] = len(group)
        out[key] = med
    return out


METRIC_ROWS = [
    # (label, key, description)
    ("execution stage wall", "execution_us", "the executor's execute stage"),
    ("inner block executor wall", "inner_us", "`BlockExecutor::execute_block`"),
    ("parallel execution wall", "parallel_execution_us", "inside `execute_transactions_parallel_v2`"),
    ("worker scope wall", "worker_scope_us", "inside the worker scope, where the workers run"),
    ("worker cpu total", "worker_cpu_total_us", "thread CPU summed over all workers"),
    ("worker cpu execute_vm", "worker_cpu_execute_vm_us", "inside the VM's execute"),
    ("worker cpu execute_vm_wasted", "worker_cpu_execute_vm_wasted_us", "VM time of incarnations that were aborted"),
    ("worker cpu execute_apply", "worker_cpu_execute_apply_us", "applying writes, recording reads"),
    ("worker cpu materialize", "worker_cpu_materialize_us", "output materialization"),
    ("worker cpu commit_hooks", "worker_cpu_commit_hooks_us", "sequential commit path"),
    ("worker cpu scheduler", "worker_cpu_scheduler_us", "next_task, spinning included"),
    ("worker wall total", "worker_wall_total_us", "wall summed over all workers"),
    ("worker wall scheduler", "worker_wall_scheduler_us", "waiting for a task"),
    ("worker wall execute_vm", "worker_wall_execute_vm_us", "wall inside the VM's execute"),
    ("dependency wait", "dependency_wait_us", "parked on a dependency"),
    ("vm init (all workers)", "vm_init_us", "per-worker VM construction per block"),
    ("finalize", "finalize_us", "after the workers return: epilogue, flush"),
    ("worker loop wall (all workers)", "worker_loop_us", "inside the worker loops"),
    ("process cpu user", "process_user_cpu_us", "whole process, execution stage"),
    ("process cpu system", "process_system_cpu_us", "whole process, execution stage"),
    ("minor faults /txn", "minor_faults_per_txn", ""),
    ("voluntary ctx switches /txn", "voluntary_ctx_switches_per_txn", ""),
    ("involuntary ctx switches /txn", "involuntary_ctx_switches_per_txn", ""),
    ("speculative aborts /txn", "aborts_per_txn", ""),
    ("interrupted incarnations /txn", "interrupts_per_txn", "stopped early by the kill switch"),
    ("predicted stalls /txn", "predicted_stalls_per_txn", "aborted readers held back for the next writer"),
    ("cold state reads /txn", "cold_reads_per_txn", "state view lookups that reached the DB"),
    ("cold state reads", "cold_read_us", "time inside those DB reads"),
    ("hot state promotions /txn", "hot_promotions_per_txn", "keys the block epilogues made hot"),
    ("sv lookups /txn", "sv_total_get_per_txn", "all state view lookups"),
    ("sv hot hits /txn", "sv_hit_hot_per_txn", "served by the hot state"),
    ("sv speculative hits /txn", "sv_hit_speculative_per_txn", "served by the in-memory delta"),
    ("max rss MiB", "max_rss_kib", ""),
]


def fmt(value, key):
    if key == "max_rss_kib":
        return f"{value / 1024:.0f}"
    if key.endswith("_per_txn"):
        return f"{value:.3f}"
    return f"{value:.2f}"


def workload_table(medians, workload):
    """One row per (metric, vm), one column per thread count, then the growth
    from the lowest to the highest thread count.
    """
    lo, hi = min(THREADS), max(THREADS)
    rows = []
    for label, key, _ in METRIC_ROWS:
        for vm in VMS:
            cells = [label, vm]
            values = {}
            for t in THREADS:
                med = medians.get((workload, vm, t))
                if med is None or key not in med:
                    cells.append("-")
                else:
                    values[t] = med[key]
                    cells.append(fmt(med[key], key))
            if lo in values and hi in values and key != "max_rss_kib":
                cells.append(f"{values[hi] - values[lo]:+.2f}")
                cells.append(f"{values[hi] / values[lo]:.2f}x" if values[lo] else "-")
            else:
                cells += ["-", "-"]
            rows.append(cells)
    headers = ["metric (us/txn)", "vm"] + [f"{t}T" for t in THREADS] + [
        f"growth {lo}T→{hi}T",
        "ratio",
    ]
    return tabulate(rows, headers=headers, tablefmt="github")


def growth_comparison(medians, workload):
    """Side by side: how much each component grew per VM between the lowest
    and highest thread count. Equal growth means shared code; more growth on
    one VM means the cause is inside that VM's own path.
    """
    lo, hi = min(THREADS), max(THREADS)
    rows = []
    for label, key, _ in METRIC_ROWS:
        if key == "max_rss_kib":
            continue
        cells = [label]
        growths = {}
        for vm in VMS:
            a = medians.get((workload, vm, lo))
            b = medians.get((workload, vm, hi))
            if a is None or b is None or key not in a:
                cells.append("-")
                continue
            growths[vm] = b[key] - a[key]
            cells.append(f"{growths[vm]:+.2f}")
        if len(growths) == 2:
            v1, mono = growths.get("v1"), growths.get("mono")
            if v1 is not None and mono is not None:
                cells.append(f"{mono - v1:+.2f}")
        rows.append(cells)
    headers = [f"growth {lo}T→{hi}T (us/txn)"] + VMS + (
        ["mono − v1"] if set(VMS) == {"v1", "mono"} else []
    )
    return tabulate(rows, headers=headers, tablefmt="github")


def workflow_timer_table(medians, workload):
    names = sorted(
        {n for (w, _, _), med in medians.items() if w == workload for n in med["workflow_timer_us"]}
    )
    rows = []
    for name in names:
        for vm in VMS:
            cells = [name, vm]
            for t in THREADS:
                med = medians.get((workload, vm, t))
                v = med["workflow_timer_us"].get(name) if med else None
                cells.append("-" if v is None else f"{v:.2f}")
            rows.append(cells)
    headers = ["workflow timer wall (us/txn)", "vm"] + [f"{t}T" for t in THREADS]
    return tabulate(rows, headers=headers, tablefmt="github")


def thread_group_table(medians, workload):
    groups = sorted(
        {tg for (w, _, _), med in medians.items() if w == workload for tg in med["thread_group_cpu_us"]}
    )
    rows = []
    for tg in groups:
        for vm in VMS:
            cells = [tg, vm]
            for t in THREADS:
                med = medians.get((workload, vm, t))
                v = med["thread_group_cpu_us"].get(tg) if med else None
                cells.append("-" if v is None else f"{v:.2f}")
            rows.append(cells)
    headers = ["thread group cpu (us/txn)", "vm"] + [f"{t}T" for t in THREADS]
    return tabulate(rows, headers=headers, tablefmt="github")


def speedup_table(medians, workloads):
    rows = []
    for w in workloads:
        cells = [w.name]
        for t in THREADS:
            v1 = medians.get((w.name, VMS[0], t))
            mono = medians.get((w.name, VMS[-1], t))
            if v1 and mono and VMS[0] != VMS[-1]:
                cells.append(f"{v1['inner_tps']:.0f} / {mono['inner_tps']:.0f} = {mono['inner_tps'] / v1['inner_tps']:.2f}x")
            else:
                cells.append("-")
        rows.append(cells)
    return tabulate(
        rows,
        headers=[f"inner block executor TPS, {VMS[0]} / {VMS[-1]}"] + [f"{t}T" for t in THREADS],
        tablefmt="github",
    )


def scaling_table(medians, workloads):
    lo = min(THREADS)
    rows = []
    for w in workloads:
        for vm in VMS:
            cells = [w.name, vm]
            base = medians.get((w.name, vm, lo))
            for t in THREADS:
                med = medians.get((w.name, vm, t))
                if med and base:
                    cells.append(
                        f"{med['inner_tps']:.0f} ({med['inner_tps'] / base['inner_tps']:.2f}x, ±{med['inner_spread'] * 100:.1f}%)"
                    )
                else:
                    cells.append("-")
            rows.append(cells)
    return tabulate(
        rows,
        headers=["inner block executor TPS (x vs lowest, spread)", "vm"] + [f"{t}T" for t in THREADS],
        tablefmt="github",
    )


def build_report(workloads, medians, profile_rows, failures, machine):
    parts = [
        "### MonoMove vs V1 MoveVM thread-scaling sweep",
        "",
        f"{NUM_BLOCKS} blocks per workload, {REPEATS} repeats, {NUM_INIT_ACCOUNTS} account DB, "
        f"{TXNS_PER_SENDER} transaction(s) per sender per block, threads {THREADS}, "
        f"stages {'split' if SPLIT_STAGES else 'pipelined'}, "
        f"{'pipelined' if PIPELINED else 'serial'} block indexing, MonoMove "
        f"{'metered' if MONO_METERED else 'unmetered'}. Every metric is a "
        "median over repeats, in "
        "microseconds per transaction unless marked. Worker metrics are summed over all "
        "workers, so `worker wall total` ≈ threads × `worker scope wall`.",
        "",
        "```",
        machine.strip(),
        "```",
        "",
        "#### Speedup by thread count",
        "",
        speedup_table(medians, workloads),
        "",
        "#### Scaling by VM",
        "",
        scaling_table(medians, workloads),
        "",
    ]
    for w in workloads:
        parts += [
            f"#### {w.name} (block size {block_size_of(w)})",
            "",
            "Growth per component, side by side. Equal growth on both VMs points at shared "
            "code; growth only on one VM points at that VM's own path.",
            "",
            growth_comparison(medians, w.name),
            "",
            "<details>",
            f"<summary>{w.name}: every metric by thread count</summary>",
            "",
            workload_table(medians, w.name),
            "",
            workflow_timer_table(medians, w.name),
            "",
            thread_group_table(medians, w.name),
            "",
            "</details>",
            "",
        ]
    if profile_rows:
        parts += ["#### Profiled replays", ""]
        parts += [
            f"- `{r['profile']}`: inner {r['inner_us']:.2f} us/txn, worker cpu "
            f"{r['worker_cpu_total_us']:.2f} us/txn (the profiled run itself)"
            for r in profile_rows
        ]
        parts.append("")
    if failures:
        parts += ["#### Failures", ""]
        parts += [f"- `{name}`: {message}" for name, message in failures]
        parts.append("")
    return "\n".join(parts)


def machine_description():
    out = subprocess.run(
        "uname -r; nproc; lscpu 2>/dev/null | grep -E 'Model name|^CPU\\(s\\)|Thread\\(s\\) per core|Core\\(s\\) per socket|NUMA node\\(s\\)'; "
        "free -g 2>/dev/null | head -2",
        shell=True,
        text=True,
        capture_output=True,
    )
    return out.stdout or "(no machine description)"


def main():
    workloads = selected_workloads()
    os.makedirs(OUT_DIR, exist_ok=True)
    os.makedirs(WORK_DIR, exist_ok=True)
    prof_dir = os.path.join(OUT_DIR, "profiles")
    if PROFILE:
        if PROFILE != "perf":
            raise SystemExit(f"PROFILE must be 'perf' or empty, got {PROFILE!r}")
        if not perf_available():
            raise SystemExit(f"PROFILE=perf but {PERF_BIN!r} is not on PATH")
        os.makedirs(prof_dir, exist_ok=True)

    machine = machine_description()
    print(machine)
    if not SKIP_BUILD:
        build()

    max_block_size = max(block_size_of(w) for w in workloads)
    num_accounts = max(NUM_INIT_ACCOUNTS, (2 + 2 * NUM_BLOCKS) * max_block_size)
    db_dir = os.path.join(WORK_DIR, "db")
    create_db(db_dir, num_accounts)

    results_path = os.path.join(OUT_DIR, "results.jsonl")
    results = open(results_path, "a")
    rows, profile_rows, failures = [], [], []

    def emit(row):
        line = json.dumps(row)
        print("SWEEP_RESULT " + line, flush=True)
        results.write(line + "\n")
        results.flush()

    for workload in workloads:
        blocks_path = os.path.join(WORK_DIR, f"{workload.name}.blocks")
        recorded_db = os.path.join(WORK_DIR, f"{workload.name}-recorded-db")
        checkpoint = os.path.join(WORK_DIR, f"{workload.name}-cp")
        try:
            record(workload, db_dir, recorded_db, blocks_path)
        except (CommandFailed, ValueError) as e:
            print(f"Recording {workload.name} failed: {e}")
            failures.append((workload.name, f"record: {e}"))
            continue

        configs = [(t, vm) for t in THREADS for vm in VMS]
        for repeat in range(REPEATS):
            # Alternate the order so drift over the run hits every
            # configuration from both sides.
            order = configs if repeat % 2 == 0 else list(reversed(configs))
            for threads, vm in order:
                shutil.rmtree(checkpoint, ignore_errors=True)
                started = time.monotonic()
                try:
                    env, program = replay_command(
                        workload,
                        threads,
                        vm,
                        recorded_db,
                        checkpoint,
                        blocks_path,
                        tag=f"{workload.name}-{vm}-{threads}t-r{repeat}",
                    )
                    output = execute_command(env + program)
                    row = parse_run(output, workload, threads, vm, repeat)
                except (CommandFailed, ValueError) as e:
                    print(f"Replay {workload.name} {vm} {threads}T failed: {e}")
                    failures.append((f"{workload.name} {vm} {threads}T", str(e)))
                    continue
                row["wall_secs"] = round(time.monotonic() - started, 1)
                rows.append(row)
                emit(row)

        if PROFILE:
            for vm, threads in PROFILE_CONFIGS:
                threads = int(threads)
                if vm not in VMS or threads not in THREADS:
                    continue
                shutil.rmtree(checkpoint, ignore_errors=True)
                try:
                    row = profile_replay(
                        workload, threads, vm, recorded_db, checkpoint, blocks_path, prof_dir
                    )
                except (CommandFailed, ValueError) as e:
                    print(f"Profile {workload.name} {vm} {threads}T failed: {e}")
                    failures.append((f"profile {workload.name} {vm} {threads}T", str(e)))
                    continue
                profile_rows.append(row)
                emit(row)

        shutil.rmtree(recorded_db, ignore_errors=True)
        shutil.rmtree(checkpoint, ignore_errors=True)

    medians = median_rows(rows)
    report = build_report(workloads, medians, profile_rows, failures, machine)
    print()
    print(report)
    with open(os.path.join(OUT_DIR, "report.md"), "w") as f:
        f.write(report)
    with open(os.path.join(OUT_DIR, "medians.json"), "w") as f:
        json.dump({"|".join(map(str, k)): v for k, v in medians.items()}, f, indent=1)
    print(f"Results in {OUT_DIR}")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
