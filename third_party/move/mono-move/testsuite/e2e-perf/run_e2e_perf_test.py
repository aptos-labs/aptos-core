#!/usr/bin/env python

# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

"""End-to-end performance comparison of MonoMove against the V1 MoveVM.

Both VMs execute byte-identical blocks. Each workload is generated once and
written to a file (`--dump-blocks`), then replayed (`--replay-blocks`) once per
VM per repeat. Without that, the two runs would draw different transactions from
the generators' entropy, and the difference in workload would show up as a
difference in speed.

The two replays differ only in a feature flag override applied after workload
initialization: MonoMove gets `--enable-feature-after-init ENABLE_MONO_MOVE`,
V1 gets `--disable-feature-after-init`. Both run the same governance script and
the same epoch change, so the only difference is the flag's value.

Initialization always runs on the V1 VM. MonoMove discards module-publish
payloads, so a workload that publishes modules could not be set up under it.

`EXECUTION_THREADS` sets the block executor's concurrency level for both VMs.
At 1 the numbers are per-transaction cost. Above it they also carry how well
each VM scales, so the two are calibrated separately.

Run locally:

    REPEATS=1 NUM_BLOCKS_PER_TEST=3 NUM_INIT_ACCOUNTS=20000 \\
      ONLY_WORKLOADS=no-op,apt-fa-transfer \\
      python3 third_party/move/mono-move/testsuite/e2e-perf/run_e2e_perf_test.py
"""

import json
import os
import re
import shutil
import signal
import statistics
import sys
import tempfile
import threading
import time
from dataclasses import dataclass, field
from subprocess import Popen, PIPE, STDOUT

from tabulate import tabulate

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from calibrate_e2e_perf_test import (
    CALIBRATED_METRICS,
    GREP_KEY,
    PARALLEL_EXECUTION_THREADS,
    calibration_path,
    load_calibration,
    speedup_band,
)

MONO_MOVE_FLAG = "ENABLE_MONO_MOVE"

# A workload whose repeats range wider than this under either VM is reported as
# noisy and can never be a regression.
MAX_DEVIATION = 0.10

# Under SELF_COMPARE both sides run V1 over identical bytes, so every ratio
# should come out at 1.00x and anything past this is the harness's own error.
# Much tighter than MAX_DEVIATION: that one bounds the range within one VM's
# repeats, while this one bounds a difference that should not exist at all. The
# measured floor is under 1%, so this still leaves room for a noisier runner.
SELF_COMPARE_MAX_DEVIATION = 0.03

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

    `description` is rendered into the report, so a reader does not have to go
    look up what a workload name means.

    `blocking` is False while a workload's band is still being established: a
    regression is reported but does not fail the job.
    """

    name: str
    block_size: int
    description: str
    blocking: bool = False


WORKLOADS = [
    Workload(
        "no-op",
        block_size=1000,
        description="Entry function with an empty body. Full transaction "
        "overhead, no Move computation and no application state.",
    ),
    Workload(
        "apt-fa-transfer",
        block_size=500,
        description="One APT fungible asset transfer per transaction.",
    ),
    Workload(
        "account-generation",
        block_size=500,
        description="Creates one new account per transaction.",
    ),
    Workload(
        "batch100-transfer",
        block_size=100,
        description="100 APT transfers in one transaction.",
    ),
    Workload(
        "token-v2-ambassador-mint",
        block_size=500,
        description="Mints a Token v2 ambassador NFT.",
    ),
    Workload(
        "liquidity-pool-swap",
        block_size=500,
        description="One swap against a liquidity pool per transaction.",
    ),
    Workload(
        "order-book-no-matches1-market",
        block_size=500,
        description="Places orders on a single market whose buy and sell prices "
        "never overlap, so every order rests in the book.",
    ),
    # The benches-e2e suite. Each of these publishes one application-shaped
    # package and runs a weighted mix of its entry points, so a number here
    # moves for the same reasons a real protocol's would.
    Workload(
        "clob-avl",
        block_size=500,
        description="Central limit order book over a bit-packed AVL queue in "
        "table items. Mixes resting placements, matching, cancels, and "
        "traversal, so the read set is a pointer chase of data-dependent depth.",
    ),
    Workload(
        "lending-market",
        block_size=500,
        description="Lending protocol adjusted from Aave v3. Supply, withdraw, "
        "borrow, repay, collateral toggles, and flash loans over eight "
        "reserves, with u256 index accrual once per reserve per block.",
    ),
    Workload(
        "clmm-swap",
        block_size=500,
        description="Concentrated liquidity swap in the Uniswap V3 shape. How "
        "densely the pool initializes ticks drives how many a swap crosses, so "
        "the read set widens from a few slots to dozens with no code change.",
    ),
    Workload(
        "stableswap",
        block_size=500,
        description="Curve-style stableswap. The get_D and get_y Newton loops "
        "run a data-dependent number of iterations, so per-transaction compute "
        "cannot be constant-folded.",
    ),
    Workload(
        "bridge-relay",
        block_size=500,
        description="LayerZero-shaped cross-chain message relay. Near-zero "
        "compute per message, so the number measures the storage IO floor: "
        "nonce compare, payload write, attestation.",
    ),
    Workload(
        "airdrop-fanout",
        block_size=500,
        description="Batch token distribution to 100 recipients per "
        "transaction. Mixes first-touch creation writes with repeat "
        "modification writes across sharded tables and primary stores.",
    ),
    Workload(
        "oracle-batch",
        block_size=500,
        description="Switchboard-shaped oracle update. Verifies real ed25519 "
        "and secp256k1 signatures over batched price reports, with matched "
        "write-only and verify-only controls that isolate the native's share.",
    ),
    Workload(
        "cdp-liquidation",
        block_size=500,
        description="Liquity-style CDP with a doubly-linked sorted vault list "
        "over resources. Each hop's address comes only from the previous "
        "resource, so the walk is a chain of sequentially dependent reads.",
    ),
    Workload(
        "dex-aggregator",
        block_size=500,
        description="Panora-shaped router over four pool backends. Routes carry "
        "up to 32 type parameters, driving monomorphization and generic "
        "dispatch far past anything else in the suite.",
    ),
    Workload(
        "nft-mint-market",
        block_size=500,
        description="Token v2 mint and marketplace. Buys, listings, offers, and "
        "fee splits over resource-group members at derived object addresses.",
    ),
]


@dataclass
class RunStats:
    """Metrics parsed out of one benchmark invocation."""

    tps: float
    # Per-stage throughput, from the "(component TPS: X)" the benchmark prints.
    stage_tps: dict
    output_bytes_per_txn: float
    mono_move_enabled: bool
    concurrency: int

    def metric(self, name):
        if name == "total":
            return self.tps
        if name == "output_bytes_per_txn":
            return self.output_bytes_per_txn
        return self.stage_tps[name]


@dataclass
class WorkloadResult:
    workload: Workload
    v1_runs: list = field(default_factory=list)
    mono_runs: list = field(default_factory=list)
    speedup: dict = field(default_factory=dict)
    # Per metric, the wider of the two VMs' (max - min) / median.
    spread: dict = field(default_factory=dict)
    verdict: str = "ok"
    # Per metric, the speedup this workload is calibrated at. A metric with no
    # row in the TSV is absent.
    calibrated: dict = field(default_factory=dict)
    warnings: list = field(default_factory=list)


REPEATS = int(os.environ.get("REPEATS", default=3))
NUM_BLOCKS = int(os.environ.get("NUM_BLOCKS_PER_TEST", default=30))
EXECUTION_THREADS = int(os.environ.get("EXECUTION_THREADS", default=1))
NUM_INIT_ACCOUNTS = int(os.environ.get("NUM_INIT_ACCOUNTS", default=2000000))
CREATE_DB_THREADS = int(os.environ.get("CREATE_DB_THREADS", default=32))
BUILD = os.environ.get("BUILD", default="release")
ONLY_WORKLOADS = os.environ.get("ONLY_WORKLOADS")
SELF_COMPARE = bool(os.environ.get("SELF_COMPARE"))
RUN_SOURCE = os.environ.get("RUN_SOURCE", default="local")
RUNNER_NAME = os.environ.get("RUNNER_NAME", default="none")
REPORT_PATH = os.environ.get("REPORT_PATH")
HIDE_OUTPUT = bool(os.environ.get("HIDE_OUTPUT"))
# How long a subprocess may print nothing before it is killed as hung. Silence,
# not total runtime, is what separates a hang from a slow workload: the
# benchmark logs every block and cargo logs every crate, so any of these
# commands going quiet for half an hour has stopped making progress. Without
# this a single stuck workload takes the job's whole six-hour budget and the
# job ends with no report at all.
SILENCE_TIMEOUT_SECS = int(os.environ.get("SILENCE_TIMEOUT_SECS", default=30 * 60))

if BUILD not in ("release", "performance"):
    print(f"BUILD must be 'release' or 'performance', got {BUILD!r}")
    sys.exit(1)

if EXECUTION_THREADS < 1:
    print(f"EXECUTION_THREADS must be at least 1, got {EXECUTION_THREADS}")
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
    def __init__(self, returncode, output, reason=None, hung=False):
        super().__init__(reason or f"exit code {returncode}: {panic_reason(output)}")
        self.returncode = returncode
        self.output = output
        self.hung = hung


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
    last_output_at = time.monotonic()
    hung = threading.Event()

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
        # Puts the shell and the benchmark it spawns in one process group.
        # Killing the shell alone orphans the benchmark, which then holds the
        # runner after the job that started it is gone.
        start_new_session=True,
    ) as p:

        def watchdog():
            while p.poll() is None:
                if time.monotonic() - last_output_at > SILENCE_TIMEOUT_SECS:
                    hung.set()
                    os.killpg(os.getpgid(p.pid), signal.SIGKILL)
                    return
                time.sleep(min(30, SILENCE_TIMEOUT_SECS / 4))

        threading.Thread(target=watchdog, daemon=True).start()
        for line in p.stdout:
            last_output_at = time.monotonic()
            if not HIDE_OUTPUT:
                print(line, end="")
            lines.append(line)

    output = "".join(lines)
    if hung.is_set():
        if HIDE_OUTPUT:
            print(output)
        raise CommandFailed(
            p.returncode,
            output,
            f"printed nothing for {SILENCE_TIMEOUT_SECS}s, killed as hung",
            hung=True,
        )
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
        mono_move_enabled=mono_move_was_enabled(output),
        concurrency=block_executor_concurrency(output),
    )


def block_executor_concurrency(output):
    """The concurrency level the block executor ran the last block at.

    Not the same as what `--execution-threads` asked for: MonoMove caps it at
    the number of arenas its global context holds, so an under-provisioned
    context turns a parallel run into a near-sequential one that still reports
    a speedup.
    """
    matches = re.findall(r"Overall block executor concurrency: (\d+)", output)
    if not matches:
        raise ValueError("run did not report a block executor concurrency level")
    return int(matches[-1])


def mono_move_was_enabled(output):
    """Whether the run's post-init override turned MonoMove on.

    An unapplied flag would give a V1-versus-V1 comparison reporting a flat
    1.00x, which looks exactly like "MonoMove is no faster".
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
    raise ValueError(f"run did not override {MONO_MOVE_FLAG} either way")


METRICS = [
    "total",
    "execution",
    "block_executor",
    "inner_block_executor",
    "ledger_update",
    "commit",
    "sigver",
    "output_bytes_per_txn",
]


def summarize(runs, metric):
    values = [r.metric(metric) for r in runs]
    median = statistics.median(values)
    spread = (max(values) - min(values)) / median if median else 0.0
    return median, spread


def verdict_for(workload, speedup, spread, calibration):
    """Classify the execution speedup against the workload's calibrated band."""
    if any(spread[m] > MAX_DEVIATION for m in VERDICT_METRICS):
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
        f"--execution-threads {EXECUTION_THREADS} --generate-then-execute "
        # Several generator threads assign sequence numbers in whatever order
        # they run, but the block keeps the order the slots were laid out in. A
        # workload that draws the same account twice in a block then lands its
        # transactions reversed, and the second one is discarded.
        f"--num-generator-workers 1 "
        f"--block-size {workload.block_size} "
        f"run-executor "
        f"--data-dir {db_dir} --checkpoint-dir {checkpoint_dir}"
    )


def record(workload, db_dir, checkpoint_dir, blocks_path):
    """Generate the blocks once and leave the initialized DB in checkpoint_dir.

    The recording is not executed and no feature flag override is applied, so
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
    override = "--enable-feature-after-init" if mono else "--disable-feature-after-init"
    output = execute_command(
        f"{common_flags(workload, recorded_db_dir, checkpoint_dir)} "
        f"--replay-blocks {blocks_path} {override} {MONO_MOVE_FLAG}"
    )
    return extract_run_stats(output)


def workload_dirs(workload, tmpdir):
    """The blocks file, the recorded DB, and the replay checkpoint.

    The last two are full copies of the warmup DB, so they are dropped once the
    workload is done rather than kept for the rest of the run.
    """
    return (
        os.path.join(tmpdir, f"{workload.name}.blocks"),
        os.path.join(tmpdir, f"{workload.name}-recorded-db"),
        os.path.join(tmpdir, f"{workload.name}-cp"),
    )


def run_workload(workload, db_dir, tmpdir, calibration):
    result = WorkloadResult(workload=workload)

    blocks_path, recorded_db, checkpoint = workload_dirs(workload, tmpdir)
    record(workload, db_dir, recorded_db, blocks_path)

    # Alternating rather than grouping the two VMs' runs is what makes the median
    # robust: thermal drift and noisy neighbours hit both equally.
    for _ in range(REPEATS):
        result.v1_runs.append(
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
        if any(r.mono_move_enabled for r in result.v1_runs):
            raise ValueError("V1 run enabled MonoMove")
        if not all(r.mono_move_enabled for r in result.mono_runs):
            raise ValueError("MonoMove run did not enable MonoMove")

    for run in result.v1_runs + result.mono_runs:
        if run.concurrency != EXECUTION_THREADS:
            raise ValueError(
                f"asked for {EXECUTION_THREADS} execution threads but the block "
                f"executor ran at {run.concurrency}"
            )

    for metric in METRICS:
        v1_median, v1_spread = summarize(result.v1_runs, metric)
        mono_median, mono_spread = summarize(result.mono_runs, metric)
        result.speedup[metric] = mono_median / v1_median if v1_median else 0.0
        result.spread[metric] = max(v1_spread, mono_spread)

    if SELF_COMPARE:
        # Both sides ran V1 on the same bytes, so every deviation from 1.00x
        # is the harness's own measurement error. Every calibrated band has to
        # sit above whatever this reports.
        for metric in VERDICT_METRICS + ["output_bytes_per_txn"]:
            value = result.speedup[metric]
            if abs(value - 1.0) > SELF_COMPARE_MAX_DEVIATION:
                result.warnings.append(
                    f"{metric} came out at {value:.2f}x under SELF_COMPARE, but both "
                    f"runs were V1; that is harness noise, not a speedup"
                )
    else:
        for stage in NEUTRAL_STAGES:
            value = result.speedup[stage]
            if not (NEUTRAL_STAGE_LOW <= value <= NEUTRAL_STAGE_HIGH):
                result.warnings.append(
                    f"{stage} speedup is {value:.2f}x, but that stage runs no Move "
                    f"code; the comparison may be skewed"
                )

    if SELF_COMPARE:
        result.verdict = "self-compare"
    else:
        result.calibrated = {
            metric: calibration[(workload.name, metric)]["median_speedup"]
            for metric in CALIBRATED_METRICS
            if (workload.name, metric) in calibration
        }
        result.verdict = verdict_for(
            workload, result.speedup, result.spread, calibration
        )
    return result


def emit_json_lines(result, test_index):
    """One line per calibrated metric, for Humio to aggregate."""
    for metric in CALIBRATED_METRICS:
        v1_median, _ = summarize(result.v1_runs, metric)
        mono_median, _ = summarize(result.mono_runs, metric)
        print(
            json.dumps(
                {
                    "grep": GREP_KEY,
                    "run_source": RUN_SOURCE,
                    "runner_name": RUNNER_NAME,
                    "workload": result.workload.name,
                    "metric": metric,
                    "speedup": result.speedup[metric],
                    "spread": result.spread[metric],
                    "v1": v1_median,
                    "mono": mono_median,
                    "block_size": result.workload.block_size,
                    "blocks": NUM_BLOCKS,
                    "repeats": REPEATS,
                    "execution_threads": EXECUTION_THREADS,
                    "warmup_num_accounts": NUM_ACCOUNTS,
                    "blocking": result.workload.blocking,
                    "verdict": result.verdict,
                    "test_index": test_index,
                }
            )
        )


def ratio(value):
    return f"{value:.2f}x"


def calibrated(result, metric):
    """The calibrated ratio for a metric, and how far this run moved from it. The
    ratio alone says where the band sits, not whether the run barely moved or
    doubled.
    """
    baseline = result.calibrated.get(metric)
    if not baseline:
        return "-"
    change = result.speedup[metric] / baseline - 1.0
    return f"{ratio(baseline)} ({change * 100:+.1f}%)"


def headline_table(results, failures):
    rows = []
    for r in results:
        rows.append(
            [
                r.workload.name,
                f"{statistics.median([x.tps for x in r.v1_runs]):.0f}",
                f"{statistics.median([x.tps for x in r.mono_runs]):.0f}",
                ratio(r.speedup["total"]),
                calibrated(r, "total"),
                ratio(r.speedup["execution"]),
                calibrated(r, "execution"),
                ratio(r.speedup["inner_block_executor"]),
                calibrated(r, "inner_block_executor"),
                f"{max(r.spread[m] for m in VERDICT_METRICS) * 100:.1f}%",
                r.verdict,
            ]
        )
    for name, _ in failures:
        rows.append([name] + ["-"] * 9 + ["failed"])
    return tabulate(
        rows,
        headers=[
            "workload",
            "V1 txn/s",
            "MonoMove txn/s",
            "total speedup",
            "calibrated",
            "execution speedup",
            "calibrated",
            "Block-STM speedup",
            "calibrated",
            "run-to-run range",
            "verdict",
        ],
        tablefmt="github",
    )


def output_size_table(results):
    rows = []
    for r in results:
        v1_bpt, _ = summarize(r.v1_runs, "output_bytes_per_txn")
        mono_bpt, _ = summarize(r.mono_runs, "output_bytes_per_txn")
        rows.append(
            [
                r.workload.name,
                f"{v1_bpt:.0f}",
                f"{mono_bpt:.0f}",
                ratio(r.speedup["output_bytes_per_txn"]),
                calibrated(r, "output_bytes_per_txn"),
            ]
        )
    return tabulate(
        rows,
        headers=[
            "workload",
            "V1 bytes/txn",
            "MonoMove bytes/txn",
            "bytes/txn ratio",
            "calibrated",
        ],
        tablefmt="github",
    )


def pipeline_table(results):
    rows = []
    for r in results:
        v1_exec, _ = summarize(r.v1_runs, "execution")
        mono_exec, _ = summarize(r.mono_runs, "execution")
        rows.append(
            [
                r.workload.name,
                ratio(r.speedup["ledger_update"]),
                ratio(r.speedup["commit"]),
                ratio(r.speedup["sigver"]),
                f"{v1_exec:.0f}",
                f"{mono_exec:.0f}",
            ]
        )
    return tabulate(
        rows,
        headers=[
            "workload",
            "ledger update",
            "commit",
            "sigver",
            "V1 exec txn/s",
            "MonoMove exec txn/s",
        ],
        tablefmt="github",
    )


def per_repeat_table(results):
    rows = []
    for r in results:
        rows.append(
            [
                r.workload.name,
                ", ".join(f"{x.stage_tps['execution']:.0f}" for x in r.v1_runs),
                ", ".join(f"{x.stage_tps['execution']:.0f}" for x in r.mono_runs),
            ]
        )
    return tabulate(
        rows,
        headers=[
            "workload",
            "V1 exec txn/s",
            "MonoMove exec txn/s",
        ],
        tablefmt="github",
    )


def glossary(selected, results):
    """Everything a reader needs to interpret the tables above it.

    Collapsed, so it costs no vertical space in the PR comment until someone
    wants it.
    """
    if EXECUTION_THREADS == 1:
        how_it_ran = (
            "run on the sequential path, since the executor only speculates above "
            "concurrency 1"
        )
    else:
        how_it_ran = f"run on {EXECUTION_THREADS} threads"

    parts = [
        "<details>",
        "<summary>Column reference, verdicts, workloads, pipeline stages</summary>",
        "",
        "#### Columns",
        "",
        "Every throughput is a median over the repeats, and every ratio is "
        "MonoMove over V1.",
        "",
        "- `V1 txn/s`, `MonoMove txn/s` — throughput of the whole pipeline.",
        "- `total speedup` — the whole pipeline: signature verification, "
        "execution, ledger update, commit.",
        "- `execution speedup` — the executor's execute stage alone.",
        "- `calibrated` — the ratio in the column to its left as recorded in "
        f"`{os.path.basename(calibration_path(EXECUTION_THREADS))}`: the median over "
        "the runs that row was last seeded from, and in parentheses how far this "
        "run moved from it. `-` when the workload has no row for that metric yet.",
        f"- `Block-STM speedup` — `BlockExecutor::execute_block`, {how_it_ran}. "
        "The innermost of the three timers and the closest proxy for VM-only "
        "time: it leaves out the block setup and output conversion that "
        "`execution` carries.",
        "- `run-to-run range` — `(max - min) / median` across one VM's repeats, "
        "reported for whichever VM and whichever of the two execution metrics "
        "came out worse. It says how repeatable each side was, not how uncertain "
        "the ratio is: the two VMs run alternating, so drift hits both and "
        "largely cancels in the ratio. Past "
        f"{MAX_DEVIATION * 100:.0f}% the verdict becomes `noisy`.",
        "- `bytes/txn` — what each VM wrote per transaction.",
        "",
        "The stages nest: `total` ⊃ `execution` ⊃ `Block-STM`.",
        "",
        "#### Verdicts",
        "",
        "Only `execution` decides a verdict, by where its speedup fell relative "
        "to the band around the `calibrated` value beside it. The percentages in "
        "that column measure drift from the last calibration, not from the base "
        "branch: while the bands are stale every run carries the same offset, "
        "and a local run carries whatever this machine differs from the "
        "benchmark runner by.",
        "",
        "- `ok` — the execution speedup landed inside the workload's calibrated "
        "band.",
        "- `improvement`, `regression` — above or below that band.",
        "- `noisy` — the range was too wide to judge either way.",
        "- `uncalibrated` — no band recorded for this workload yet.",
        "- `failed` — the workload did not finish; the reason is listed above.",
        "- `self-compare` — V1 ran on both sides, so there is nothing to judge.",
        "",
        "#### Workloads",
        "",
    ]
    parts += [
        f"- `{w.name}` — {w.description} Blocks of {w.block_size} transactions."
        for w in selected
    ]
    parts += [
        "",
        "#### Pipeline stages",
        "",
        "None of these run Move code. Signature verification does identical work "
        "in both replays, so it sits near 1.00x and whatever it deviates by is "
        "noise and core contention with the concurrent execution stage. Ledger "
        "update and commit move with output size, so they track the bytes/txn "
        "ratio above rather than staying flat.",
        "",
        pipeline_table(results),
        "",
        "#### Execution throughput per repeat",
        "",
        "The raw values behind the medians, in the order they ran.",
        "",
        per_repeat_table(results),
        "",
        "</details>",
    ]
    return "\n".join(parts)


def build_report(selected, results, failures):
    if EXECUTION_THREADS == 1:
        title = "MonoMove vs V1 MoveVM, sequential execution"
    else:
        title = (
            "MonoMove vs V1 MoveVM, parallel execution on "
            f"{EXECUTION_THREADS} threads"
        )
    if SELF_COMPARE:
        title += " (SELF_COMPARE: V1 vs V1, everything should be 1.00x)"

    calibration_file = os.path.basename(calibration_path(EXECUTION_THREADS))
    parts = [
        f"### {title}",
        "",
        f"{NUM_BLOCKS} blocks, {REPEATS} repeats per VM, {NUM_ACCOUNTS} account DB, "
        f"`{BUILD}` build, {EXECUTION_THREADS} execution thread"
        f"{'' if EXECUTION_THREADS == 1 else 's'}. Ratios are MonoMove over V1; "
        "above 1.00x means MonoMove is faster.",
        "",
        "Every replay behind these tables ran the block executor at concurrency "
        f"{EXECUTION_THREADS}. A replay capped below it fails its workload rather "
        "than reporting a speedup, so the two VMs are always compared at the same "
        "level.",
        "",
    ]

    if EXECUTION_THREADS not in (1, PARALLEL_EXECUTION_THREADS):
        parts += [
            f"**The bands in `{calibration_file}` were measured at "
            f"{PARALLEL_EXECUTION_THREADS} threads, not {EXECUTION_THREADS}, so "
            f"every verdict below is read against the wrong band.**",
            "",
        ]

    parts += [
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
            f"largest range is {spread * 100:.1f}%. Every calibrated band and "
            f"`MAX_DEVIATION` has to sit above these. Record them in the README.",
            "",
        ]

    parts += [
        "#### Output size",
        "",
        "Bytes per transaction records what each VM wrote. The two need not "
        "match: MonoMove is unmetered today, so it writes no fee slots, and it "
        "copies on every `borrow_global_mut`, so its write set is an "
        "overapproximation. Either can change. What the calibration catches is "
        "the ratio moving away from where it was measured.",
        "",
        output_size_table(results),
    ]

    warnings = [(r.workload.name, w) for r in results for w in r.warnings]
    if warnings:
        parts += ["", "#### Warnings", ""]
        parts += [f"- `{name}`: {message}" for name, message in warnings]

    if failures:
        parts += ["", "#### Failed workloads", ""]
        parts += [f"- `{name}`: {message}" for name, message in failures]

    parts += ["", glossary(selected, results)]

    return "\n".join(parts) + "\n"


def run_workload_with_retry(workload, db_dir, tmpdir, calibration):
    """Run a workload, retrying once if the first attempt hangs.

    Opening a workload's copy of the warmup DB sometimes stalls and never
    recovers. With one copy per workload the suite hits that often enough to
    lose a run to it. A hang says nothing about the workload, unlike a panic or
    an assert, so it is the one failure worth a second attempt.
    """
    try:
        return run_workload(workload, db_dir, tmpdir, calibration)
    except CommandFailed as e:
        if not e.hung:
            raise
        print(f"Workload {workload.name} hung, retrying once: {e}")

    # The killed attempt leaves a partial copy and possibly a truncated blocks
    # file, and a replay checks the recording's header against what it is
    # handed. Start the second attempt from nothing.
    blocks, *dirs = workload_dirs(workload, tmpdir)
    for path in dirs:
        shutil.rmtree(path, ignore_errors=True)
    if os.path.exists(blocks):
        os.remove(blocks)
    return run_workload(workload, db_dir, tmpdir, calibration)


def main():
    selected = WORKLOADS
    if ONLY_WORKLOADS:
        wanted = {name.strip() for name in ONLY_WORKLOADS.split(",") if name.strip()}
        selected = [w for w in WORKLOADS if w.name in wanted]
        unknown = wanted - {w.name for w in selected}
        if unknown:
            print(f"Unknown workloads in ONLY_WORKLOADS: {sorted(unknown)}")
            return 1

    calibration = load_calibration(calibration_path(EXECUTION_THREADS))
    build()

    results = []
    failures = []

    with tempfile.TemporaryDirectory() as tmpdir:
        db_dir = os.path.join(tmpdir, "db")
        create_db(db_dir)

        for test_index, workload in enumerate(selected):
            try:
                result = run_workload_with_retry(workload, db_dir, tmpdir, calibration)
            except (CommandFailed, ValueError) as e:
                # One unsupported workload must not take down the whole job, but
                # it is a real finding, so it still fails at the end.
                print(f"Workload {workload.name} failed: {e}")
                failures.append((workload.name, str(e)))
                continue
            finally:
                for path in workload_dirs(workload, tmpdir)[1:]:
                    shutil.rmtree(path, ignore_errors=True)
            results.append(result)
            # A self-compare produces no speedups, so it must not reach the
            # calibration history.
            if not SELF_COMPARE:
                emit_json_lines(result, test_index)

    report = build_report(selected, results, failures)
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
