#!/usr/bin/env python

# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

"""End-to-end performance comparison of MonoMove against the legacy MoveVM.

Both VMs execute byte-identical blocks. Each workload is generated once and
written to a file (`--dump-blocks`), then replayed (`--replay-blocks`) once per
VM per repeat. Without that, the two runs would draw different transactions from
the generators' entropy, and the difference in workload would show up as a
difference in speed.

The two replays differ only in a feature flip applied after workload
initialization: MonoMove gets `--enable-feature-after-init ENABLE_MONO_MOVE`,
legacy gets `--disable-feature-after-init`. Both run the same governance script
and the same epoch change, so the only difference is the flag's value.

Initialization always runs on the legacy VM. MonoMove discards module-publish
payloads, so a workload that publishes modules could not be set up under it.

Run locally:

    REPEATS=1 NUM_BLOCKS_PER_TEST=3 NUM_INIT_ACCOUNTS=20000 \\
      ONLY_WORKLOADS=no-op,apt-fa-transfer \\
      python3 testsuite/mono_move/e2e-perf/run_e2e_perf_test.py
"""

import json
import os
import re
import statistics
import sys
import tempfile
from dataclasses import dataclass, field
from subprocess import Popen, PIPE, STDOUT

from tabulate import tabulate

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from calibrate_e2e_perf_test import (
    CALIBRATED_METRICS,
    GREP_KEY,
    load_calibration,
    speedup_band,
)

# Bump after a change that moves the numbers, so runs on top of this commit are
# easy to separate from older ones in Humio.
CODE_PERF_VERSION = "v1"

MONO_MOVE_FLAG = "ENABLE_MONO_MOVE"

# A workload whose measurements spread wider than this under either VM is
# reported as noisy and can never be a regression.
NOISY_SPREAD = 0.10

# The throughput metrics the verdict rests on. Only the two that measure Move
# execution itself. Everything else in the pipeline is disk bound and swings by
# tens of percent between two identical runs on a shared runner, so judging
# noise on it would mark most workloads noisy forever. Those metrics are still
# reported and calibrated, they just do not veto a verdict.
VERDICT_METRICS = ["execution", "inner_block_executor"]

# Signature verification is the only stage that neither runs Move code nor scales
# with output size, so it is the only one that has to sit near 1.00x. Ledger
# update and commit move with how much the VM wrote, which legitimately differs.
NEUTRAL_STAGES = ["sigver"]
NEUTRAL_STAGE_LOW = 0.8
NEUTRAL_STAGE_HIGH = 1.25


@dataclass(frozen=True)
class Workload:
    """One transaction type, measured on both VMs.

    `blocking` is False while a workload's band is still being established: a
    regression is reported but does not fail the job.
    """

    name: str
    block_size: int
    blocking: bool = False


WORKLOADS = [
    Workload("no-op", block_size=1000),
    Workload("apt-fa-transfer", block_size=500),
    Workload("account-generation", block_size=500),
    Workload("modify-global-resource", block_size=500),
    Workload("batch100-transfer", block_size=100),
    Workload("token-v2-ambassador-mint", block_size=500),
    Workload("liquidity-pool-swap", block_size=500),
    Workload("order-book-no-matches1-market", block_size=500),
]


@dataclass
class RunStats:
    """Metrics parsed out of one benchmark invocation."""

    tps: float
    # Per-stage throughput, from the "(component TPS: X)" the benchmark prints.
    stage_tps: dict
    output_bytes_per_txn: float
    output_bytes_per_sec: float
    mono_move_enabled: bool

    def metric(self, name):
        if name == "total":
            return self.tps
        if name == "output_bytes_per_txn":
            return self.output_bytes_per_txn
        if name == "output_bytes_per_sec":
            return self.output_bytes_per_sec
        return self.stage_tps[name]


@dataclass
class WorkloadResult:
    workload: Workload
    legacy_runs: list = field(default_factory=list)
    mono_runs: list = field(default_factory=list)
    speedup: dict = field(default_factory=dict)
    # Per metric, the wider of the two VMs' (max - min) / median.
    spread: dict = field(default_factory=dict)
    verdict: str = "ok"
    warnings: list = field(default_factory=list)


REPEATS = int(os.environ.get("REPEATS", default=3))
NUM_BLOCKS = int(os.environ.get("NUM_BLOCKS_PER_TEST", default=30))
NUM_INIT_ACCOUNTS = int(os.environ.get("NUM_INIT_ACCOUNTS", default=2000000))
CREATE_DB_THREADS = int(os.environ.get("CREATE_DB_THREADS", default=32))
BUILD = os.environ.get("BUILD", default="release")
ONLY_WORKLOADS = os.environ.get("ONLY_WORKLOADS")
SELF_COMPARE = bool(os.environ.get("SELF_COMPARE"))
RUN_SOURCE = os.environ.get("RUN_SOURCE", default="local")
RUNNER_NAME = os.environ.get("RUNNER_NAME", default="none")
REPORT_PATH = os.environ.get("REPORT_PATH")
HIDE_OUTPUT = bool(os.environ.get("HIDE_OUTPUT"))

if BUILD not in ("release", "performance"):
    print(f"BUILD must be 'release' or 'performance', got {BUILD!r}")
    sys.exit(1)

if RUN_SOURCE not in ("ci", "manual", "local"):
    print(f"RUN_SOURCE must be 'ci', 'manual' or 'local', got {RUN_SOURCE!r}")
    sys.exit(1)

BUILD_FLAG = "--release" if BUILD == "release" else "--profile performance"
BUILD_FOLDER = f"target/{BUILD}"

MAX_BLOCK_SIZE = max(w.block_size for w in WORKLOADS)
MAIN_SIGNER_ACCOUNTS = 2 * MAX_BLOCK_SIZE
ADDITIONAL_DST_POOL_ACCOUNTS = 2 * MAX_BLOCK_SIZE * NUM_BLOCKS
# The account pool has to fit inside the warmup DB.
NUM_ACCOUNTS = max(NUM_INIT_ACCOUNTS, (2 + 2 * NUM_BLOCKS) * MAX_BLOCK_SIZE)


class CommandFailed(Exception):
    def __init__(self, returncode, output):
        super().__init__(f"exit code {returncode}: {panic_reason(output)}")
        self.returncode = returncode
        self.output = output


def panic_reason(output):
    """The message of the last Rust panic in `output`, for the report.

    The benchmark's own asserts are the interesting failures here: an
    unsupported workload trips the abort or discard assert rather than
    reporting a slow number, and the assert message says which.
    """
    matches = re.findall(r"panicked at [^\n]*:\n([^\n]*)", output)
    if not matches:
        return "no panic in the output; see the job log"
    # The first panic is the cause. The ones after it are worker threads
    # unwinding and the main thread re-raising, which say nothing.
    reason = matches[0].strip()
    return reason if len(reason) <= 200 else reason[:200] + " ..."


def execute_command(command):
    print(f"Executing command:\n\t{command}\nand waiting for it to finish...")
    lines = []
    # The benchmark logs to stderr, so it is merged into stdout: draining one
    # pipe at a time would deadlock once the other filled up.
    with Popen(
        command,
        shell=True,
        text=True,
        stdout=PIPE,
        stderr=STDOUT,
        bufsize=1,
        universal_newlines=True,
    ) as p:
        for line in p.stdout:
            if not HIDE_OUTPUT:
                print(line, end="")
            lines.append(line)

    output = "".join(lines)
    if p.returncode != 0:
        if HIDE_OUTPUT:
            print(output)
        raise CommandFailed(p.returncode, output)
    return output


def get_only(values, what):
    if len(values) != 1:
        raise ValueError(f"expected exactly one {what}, parsed {values}")
    return values[0]


NUMBER = r"(\d+\.?\d*)"


def extract_run_stats(output):
    """Parse the "Overall" measurement block the benchmark prints at the end."""
    tps = float(get_only(re.findall(r"Overall TPS: " + NUMBER + r" txn/s", output), "TPS"))
    output_bps = float(
        get_only(re.findall(r"Overall output: " + NUMBER + r" bytes/s", output), "bytes/s")
    )
    output_bpt = float(
        get_only(
            re.findall(r"Overall output: " + NUMBER + r" bytes/txn", output), "bytes/txn"
        )
    )

    def component_tps(pattern):
        matches = re.findall(pattern + r".*?\(component TPS: " + NUMBER + r"\)", output)
        if not matches:
            raise ValueError(f"no component TPS line matching {pattern!r}")
        return float(matches[-1])

    stage_tps = {
        "sigver": component_tps(
            r"Overall fraction of total: \d+\.?\d* in signature verification"
        ),
        "execution": component_tps(
            r"Overall fraction of total: \d+\.?\d* in execution"
        ),
        "block_executor": component_tps(
            r"Overall fraction of execution \d+\.?\d* in get execution output by executing"
        ),
        "inner_block_executor": component_tps(
            r"Overall fraction of execution \d+\.?\d* in inner block executor"
        ),
        "ledger_update": component_tps(
            r"Overall fraction of total: \d+\.?\d* in ledger update"
        ),
        "commit": component_tps(r"Overall fraction of total: \d+\.?\d* in commit"),
    }

    return RunStats(
        tps=tps,
        stage_tps=stage_tps,
        output_bytes_per_txn=output_bpt,
        output_bytes_per_sec=output_bps,
        mono_move_enabled=mono_move_was_enabled(output),
    )


def mono_move_was_enabled(output):
    """Whether the run's post-init flip turned MonoMove on.

    An unapplied flag would give a legacy-versus-legacy comparison reporting a
    flat 1.00x, which looks exactly like "MonoMove is no faster".
    """
    matches = re.findall(
        r"Feature flag overrides after init: enable=\[([^\]]*)\] disable=\[([^\]]*)\]",
        output,
    )
    if not matches:
        raise ValueError("run did not apply any feature flag overrides")
    enabled, disabled = matches[-1]
    if MONO_MOVE_FLAG in enabled:
        return True
    if MONO_MOVE_FLAG in disabled:
        return False
    raise ValueError(f"run did not flip {MONO_MOVE_FLAG} either way")


METRICS = [
    "total",
    "execution",
    "block_executor",
    "inner_block_executor",
    "ledger_update",
    "commit",
    "sigver",
    "output_bytes_per_txn",
    "output_bytes_per_sec",
]


def summarize(runs, metric):
    values = [r.metric(metric) for r in runs]
    median = statistics.median(values)
    spread = (max(values) - min(values)) / median if median else 0.0
    return median, spread


def verdict_for(workload, speedup, spread, calibration):
    """Classify the execution speedup against the workload's calibrated band."""
    if any(spread[m] > NOISY_SPREAD for m in VERDICT_METRICS):
        return "noisy"

    row = calibration.get((workload.name, "execution"))
    measured = speedup["execution"]
    if row is None:
        return "uncalibrated"

    low, high = speedup_band(
        row["median_speedup"],
        row["num_samples"],
        row["lowest_over_median"],
        row["highest_over_median"],
    )
    if measured < low:
        return "regression"
    if measured > high:
        return "improvement"
    return "ok"


def build():
    execute_command(f"cargo build {BUILD_FLAG} --package aptos-executor-benchmark")


def create_db(db_dir):
    print(f"Warmup - creating DB with {NUM_ACCOUNTS} accounts")
    execute_command(
        f"PUSH_METRICS_NAMESPACE=benchmark-create-db RUST_BACKTRACE=1 "
        f"{BUILD_FOLDER}/aptos-executor-benchmark "
        f"--block-executor-type aptos-vm-with-block-stm "
        f"--block-size {MAX_BLOCK_SIZE} --execution-threads {CREATE_DB_THREADS} "
        f"create-db --data-dir {db_dir} --num-accounts {NUM_ACCOUNTS}"
    )


def common_flags(workload, db_dir, checkpoint_dir):
    return (
        f"RUST_BACKTRACE=1 {BUILD_FOLDER}/aptos-executor-benchmark "
        f"--block-executor-type aptos-vm-with-block-stm "
        f"--execution-threads 1 --generate-then-execute "
        f"--block-size {workload.block_size} "
        f"run-executor "
        f"--data-dir {db_dir} --checkpoint-dir {checkpoint_dir}"
    )


def record(workload, db_dir, checkpoint_dir, blocks_path):
    """Generate the blocks once and leave the initialized DB in checkpoint_dir.

    The recording is not executed and no feature flip is applied, so
    checkpoint_dir is exactly the state every replay starts from.
    """
    execute_command(
        f"{common_flags(workload, db_dir, checkpoint_dir)} "
        f"--transaction-type {workload.name} --module-working-set-size 1 "
        f"--main-signer-accounts {MAIN_SIGNER_ACCOUNTS} "
        f"--additional-dst-pool-accounts {ADDITIONAL_DST_POOL_ACCOUNTS} "
        f"--blocks {NUM_BLOCKS} --dump-blocks {blocks_path}"
    )


def replay(workload, recorded_db_dir, checkpoint_dir, blocks_path, mono):
    """Replay the recorded blocks with MonoMove on or off.

    Both settings run the same governance script and epoch change; disabling an
    already-disabled flag writes no state.
    """
    flip = "--enable-feature-after-init" if mono else "--disable-feature-after-init"
    output = execute_command(
        f"{common_flags(workload, recorded_db_dir, checkpoint_dir)} "
        f"--replay-blocks {blocks_path} {flip} {MONO_MOVE_FLAG}"
    )
    return extract_run_stats(output)


def run_workload(workload, db_dir, tmpdir, calibration):
    result = WorkloadResult(workload=workload)

    blocks_path = os.path.join(tmpdir, f"{workload.name}.blocks")
    recorded_db = os.path.join(tmpdir, f"{workload.name}-recorded-db")
    record(workload, db_dir, recorded_db, blocks_path)

    # Alternating rather than grouping the two VMs' runs is what makes the median
    # robust: thermal drift and noisy neighbours hit both equally.
    for _ in range(REPEATS):
        checkpoint = os.path.join(tmpdir, f"{workload.name}-cp")
        result.legacy_runs.append(
            replay(workload, recorded_db, checkpoint, blocks_path, mono=False)
        )
        result.mono_runs.append(
            replay(
                workload,
                recorded_db,
                checkpoint,
                blocks_path,
                mono=not SELF_COMPARE,
            )
        )

    if not SELF_COMPARE:
        if any(r.mono_move_enabled for r in result.legacy_runs):
            raise ValueError("legacy run enabled MonoMove")
        if not all(r.mono_move_enabled for r in result.mono_runs):
            raise ValueError("MonoMove run did not enable MonoMove")

    for metric in METRICS:
        legacy_median, legacy_spread = summarize(result.legacy_runs, metric)
        mono_median, mono_spread = summarize(result.mono_runs, metric)
        result.speedup[metric] = mono_median / legacy_median if legacy_median else 0.0
        result.spread[metric] = max(legacy_spread, mono_spread)

    if SELF_COMPARE:
        # Both sides ran legacy on the same bytes, so every deviation from 1.00x
        # is the harness's own measurement error. Every calibrated band has to
        # sit above whatever this reports.
        for metric in VERDICT_METRICS + ["output_bytes_per_txn"]:
            value = result.speedup[metric]
            if abs(value - 1.0) > NOISY_SPREAD:
                result.warnings.append(
                    f"{metric} came out at {value:.2f}x under SELF_COMPARE, but both "
                    f"runs were legacy; that is harness noise, not a speedup"
                )
    else:
        for stage in NEUTRAL_STAGES:
            value = result.speedup[stage]
            if not (NEUTRAL_STAGE_LOW <= value <= NEUTRAL_STAGE_HIGH):
                result.warnings.append(
                    f"{stage} speedup is {value:.2f}x, but that stage runs no Move "
                    f"code; the comparison may be skewed"
                )

    result.verdict = (
        "self-compare"
        if SELF_COMPARE
        else verdict_for(workload, result.speedup, result.spread, calibration)
    )
    return result


def emit_json_lines(result, test_index):
    """One line per calibrated metric, for Humio to aggregate."""
    for metric in CALIBRATED_METRICS:
        legacy_median, _ = summarize(result.legacy_runs, metric)
        mono_median, _ = summarize(result.mono_runs, metric)
        print(
            json.dumps(
                {
                    "grep": GREP_KEY,
                    "run_source": RUN_SOURCE,
                    "runner_name": RUNNER_NAME,
                    "code_perf_version": CODE_PERF_VERSION,
                    "workload": result.workload.name,
                    "metric": metric,
                    "speedup": result.speedup[metric],
                    "spread": result.spread[metric],
                    "legacy": legacy_median,
                    "mono": mono_median,
                    "block_size": result.workload.block_size,
                    "blocks": NUM_BLOCKS,
                    "repeats": REPEATS,
                    "warmup_num_accounts": NUM_ACCOUNTS,
                    "blocking": result.workload.blocking,
                    "verdict": result.verdict,
                    "test_index": test_index,
                }
            )
        )


def ratio(value):
    return f"{value:.2f}x"


def headline_table(results, failures):
    rows = []
    for r in results:
        rows.append(
            [
                r.workload.name,
                f"{statistics.median([x.tps for x in r.legacy_runs]):.0f}",
                f"{statistics.median([x.tps for x in r.mono_runs]):.0f}",
                ratio(r.speedup["total"]),
                ratio(r.speedup["execution"]),
                ratio(r.speedup["inner_block_executor"]),
                f"{max(r.spread[m] for m in VERDICT_METRICS) * 100:.1f}%",
                r.verdict,
            ]
        )
    for name, _ in failures:
        rows.append([name, "-", "-", "-", "-", "-", "-", "failed"])
    return tabulate(
        rows,
        headers=[
            "workload",
            "legacy t/s",
            "mono t/s",
            "total",
            "execution",
            "inner blk exe",
            "spread",
            "verdict",
        ],
        tablefmt="github",
    )


def output_size_table(results):
    rows = []
    for r in results:
        legacy_bpt, _ = summarize(r.legacy_runs, "output_bytes_per_txn")
        mono_bpt, _ = summarize(r.mono_runs, "output_bytes_per_txn")
        legacy_bps, _ = summarize(r.legacy_runs, "output_bytes_per_sec")
        mono_bps, _ = summarize(r.mono_runs, "output_bytes_per_sec")
        rows.append(
            [
                r.workload.name,
                f"{legacy_bpt:.0f}",
                f"{mono_bpt:.0f}",
                ratio(r.speedup["output_bytes_per_txn"]),
                f"{legacy_bps / 1e6:.2f}",
                f"{mono_bps / 1e6:.2f}",
                ratio(r.speedup["output_bytes_per_sec"]),
            ]
        )
    return tabulate(
        rows,
        headers=[
            "workload",
            "legacy B/txn",
            "mono B/txn",
            "B/txn",
            "legacy MB/s",
            "mono MB/s",
            "MB/s",
        ],
        tablefmt="github",
    )


def pipeline_table(results):
    rows = []
    for r in results:
        rows.append(
            [
                r.workload.name,
                ratio(r.speedup["ledger_update"]),
                ratio(r.speedup["commit"]),
                ratio(r.speedup["sigver"]),
                ", ".join(f"{x.stage_tps['execution']:.0f}" for x in r.legacy_runs),
                ", ".join(f"{x.stage_tps['execution']:.0f}" for x in r.mono_runs),
            ]
        )
    return tabulate(
        rows,
        headers=[
            "workload",
            "ledger update",
            "commit",
            "sigver",
            "legacy exec t/s per repeat",
            "mono exec t/s per repeat",
        ],
        tablefmt="github",
    )


def build_report(results, failures):
    title = "MonoMove vs legacy MoveVM, sequential execution"
    if SELF_COMPARE:
        title += " (SELF_COMPARE: legacy vs legacy, everything should be 1.00x)"

    parts = [
        f"### {title}",
        "",
        f"{NUM_BLOCKS} blocks, {REPEATS} repeats per VM, {NUM_ACCOUNTS} account DB, "
        f"`{BUILD}` build. Ratios are MonoMove over legacy; above 1.00x means "
        f"MonoMove is faster.",
        "",
        "Gas is not compared. MonoMove runs unmetered, so its gas metrics are zero.",
        "",
        headline_table(results, failures),
        "",
    ]

    if SELF_COMPARE and results:
        deviation = max(
            abs(r.speedup[m] - 1.0) for r in results for m in VERDICT_METRICS
        )
        spread = max(r.spread[m] for r in results for m in VERDICT_METRICS)
        parts += [
            f"Noise floor: largest deviation from 1.00x is {deviation * 100:.1f}%, "
            f"largest spread is {spread * 100:.1f}%. Every calibrated band and "
            f"`NOISY_SPREAD` has to sit above these. Record them in the README.",
            "",
        ]

    parts += [
        "#### Output size",
        "",
        "Bytes per transaction says whether the two VMs wrote the same thing. "
        "MonoMove runs unmetered, so it writes no fee slots and emits no fee "
        "statement; it sits below 1.00x on every workload. What matters is that the "
        "ratio stays where it was calibrated. A drop means MonoMove skipped real "
        "work, and the speedup next to it is not a speedup.",
        "",
        output_size_table(results),
        "",
        "#### Pipeline stages",
        "",
        "None of these run Move code. Signature verification should sit at 1.00x. "
        "Ledger update and commit move with output size, so they track the B/txn "
        "ratio above rather than staying flat.",
        "",
        pipeline_table(results),
    ]

    warnings = [(r.workload.name, w) for r in results for w in r.warnings]
    if warnings:
        parts += ["", "#### Warnings", ""]
        parts += [f"- `{name}`: {message}" for name, message in warnings]

    if failures:
        parts += ["", "#### Failed workloads", ""]
        parts += [f"- `{name}`: {message}" for name, message in failures]

    return "\n".join(parts) + "\n"


def main():
    selected = WORKLOADS
    if ONLY_WORKLOADS:
        wanted = {name.strip() for name in ONLY_WORKLOADS.split(",") if name.strip()}
        selected = [w for w in WORKLOADS if w.name in wanted]
        unknown = wanted - {w.name for w in selected}
        if unknown:
            print(f"Unknown workloads in ONLY_WORKLOADS: {sorted(unknown)}")
            return 1

    calibration = load_calibration()
    build()

    results = []
    failures = []

    with tempfile.TemporaryDirectory() as tmpdir:
        db_dir = os.path.join(tmpdir, "db")
        create_db(db_dir)

        for test_index, workload in enumerate(selected):
            try:
                result = run_workload(workload, db_dir, tmpdir, calibration)
            except (CommandFailed, ValueError) as e:
                # One unsupported workload must not take down the whole job, but
                # it is a real finding, so it still fails at the end.
                print(f"Workload {workload.name} failed: {e}")
                failures.append((workload.name, str(e)))
                continue
            results.append(result)
            # A self-compare produces no speedups, so it must not reach the
            # calibration history.
            if not SELF_COMPARE:
                emit_json_lines(result, test_index)

    report = build_report(results, failures)
    print()
    print(report)

    if REPORT_PATH:
        with open(REPORT_PATH, "w") as f:
            f.write(report)
        print(f"Report written to {REPORT_PATH}")

    if failures:
        return 1
    if any(r.verdict == "regression" and r.workload.blocking for r in results):
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
