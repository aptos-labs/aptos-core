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

from block_stage_chart import write_charts
from calibrate_e2e_perf_test import (
    CALIBRATED_METRICS,
    GREP_KEY,
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

# The throughput metric the verdict rests on. Only the one that measures Move
# execution itself. Everything else in the pipeline is disk bound and swings by
# tens of percent between two identical runs on a shared runner, so judging
# noise on it would mark most workloads noisy forever. Those metrics are still
# reported, they just do not veto a verdict.
VERDICT_METRICS = ["execution"]

# Signature verification is the only stage that neither runs Move code nor scales
# with output size, so it is the only one that has to sit near 1.00x. Ledger
# update and commit move with how much the VM wrote, which legitimately differs.
NEUTRAL_STAGES = ["sigver"]
NEUTRAL_STAGE_LOW = 0.8
NEUTRAL_STAGE_HIGH = 1.25


# Transactions per block, the same for every workload. A workload's own natural
# batch size would make the numbers incomparable across the suite: block
# overhead is per block, so a smaller block carries more of it per transaction
# and the speedup moves for a reason that has nothing to do with the workload.
BLOCK_SIZE = 1000


@dataclass(frozen=True)
class Workload:
    """One transaction type, measured on both VMs.

    `description` is rendered into the report, so a reader does not have to go
    look up what a workload name means.

    `blocking` is False while a workload's band is still being established: a
    regression is reported but does not fail the job.
    """

    name: str
    description: str
    blocking: bool = False


WORKLOADS = [
    Workload(
        "no-op",
        description="Entry function with an empty body. Full transaction "
        "overhead, no Move computation and no application state.",
    ),
    Workload(
        "apt-fa-transfer",
        description="One APT fungible asset transfer per transaction.",
    ),
    Workload(
        "account-generation",
        description="Creates one new account per transaction.",
    ),
    Workload(
        "batch100-transfer",
        description="100 APT transfers in one transaction.",
    ),
    Workload(
        "token-v2-ambassador-mint",
        description="Mints a Token v2 ambassador NFT.",
    ),
    Workload(
        "liquidity-pool-swap",
        description="One swap against a liquidity pool per transaction.",
    ),
    Workload(
        "order-book-no-matches1-market",
        description="Places orders on a single market whose buy and sell prices "
        "never overlap, so every order rests in the book.",
    ),
    # The benches-e2e suite. Each of these publishes one application-shaped
    # package and runs a weighted mix of its entry points, so a number here
    # moves for the same reasons a real protocol's would.
    Workload(
        "clob-avl",
        description="Central limit order book over a bit-packed AVL queue in "
        "table items. Mixes resting placements, matching, cancels, and "
        "traversal, so the read set is a pointer chase of data-dependent depth.",
    ),
    Workload(
        "lending-market",
        description="Lending protocol adjusted from Aave v3. Supply, withdraw, "
        "borrow, repay, collateral toggles, and flash loans over eight "
        "reserves, with u256 index accrual once per reserve per block.",
    ),
    Workload(
        "clmm-swap",
        description="Concentrated liquidity swap in the Uniswap V3 shape. How "
        "densely the pool initializes ticks drives how many a swap crosses, so "
        "the read set widens from a few slots to dozens with no code change.",
    ),
    Workload(
        "stableswap",
        description="Curve-style stableswap. The get_D and get_y Newton loops "
        "run a data-dependent number of iterations, so per-transaction compute "
        "cannot be constant-folded.",
    ),
    Workload(
        "bridge-relay",
        description="LayerZero-shaped cross-chain message relay. Near-zero "
        "compute per message, so the number measures the storage IO floor: "
        "nonce compare, payload write, attestation.",
    ),
    Workload(
        "airdrop-fanout",
        description="Batch token distribution to 100 recipients per "
        "transaction. Mixes first-touch creation writes with repeat "
        "modification writes across sharded tables and primary stores.",
    ),
    Workload(
        "oracle-batch",
        description="Switchboard-shaped oracle update. Verifies real ed25519 "
        "and secp256k1 signatures over batched price reports, with matched "
        "write-only and verify-only controls that isolate the native's share.",
    ),
    Workload(
        "cdp-liquidation",
        description="Liquity-style CDP with a doubly-linked sorted vault list "
        "over resources. Each hop's address comes only from the previous "
        "resource, so the walk is a chain of sequentially dependent reads.",
    ),
    Workload(
        "dex-aggregator",
        description="Panora-shaped router over four pool backends. Routes carry "
        "up to 32 type parameters, driving monomorphization and generic "
        "dispatch far past anything else in the suite.",
    ),
    Workload(
        "nft-mint-market",
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
    # One entry per block, from BLOCK_MEASUREMENTS_JSON.
    blocks: list
    # From STAGE_COUNTERS_JSON: {"executor": {...}, "storage": {...}}, each
    # mapping a timer label to its total seconds and call count over the run.
    timers: dict

    def metric(self, name):
        if name == "total":
            return self.tps
        if name == "total_steady":
            return self.steady_tps()
        if name == "output_bytes_per_txn":
            return self.output_bytes_per_txn
        return self.stage_tps[name]

    def steady_tps(self):
        """End-to-end throughput with the first WARMUP_BLOCKS blocks dropped.

        Measured at the commit stage, which is where a block is actually done.
        The window starts when block WARMUP_BLOCKS - 1 committed, so everything
        the warmup blocks paid for — lazily spawned thread pools, the first
        memtable, cold reads against the freshly copied DB — falls outside it.
        """
        if len(self.blocks) <= WARMUP_BLOCKS:
            return self.tps
        window = self.blocks[WARMUP_BLOCKS:]
        elapsed = (
            window[-1]["committed_at_ms"]
            - self.blocks[WARMUP_BLOCKS - 1]["committed_at_ms"]
        ) / 1000.0
        if elapsed <= 0:
            return self.tps
        return sum(b["num_txns"] for b in window) / elapsed


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
NUM_INIT_ACCOUNTS = int(os.environ.get("NUM_INIT_ACCOUNTS", default=2000000))
CREATE_DB_THREADS = int(os.environ.get("CREATE_DB_THREADS", default=32))
BUILD = os.environ.get("BUILD", default="release")
ONLY_WORKLOADS = os.environ.get("ONLY_WORKLOADS")
SELF_COMPARE = bool(os.environ.get("SELF_COMPARE"))
RUN_SOURCE = os.environ.get("RUN_SOURCE", default="local")
RUNNER_NAME = os.environ.get("RUNNER_NAME", default="none")
REPORT_PATH = os.environ.get("REPORT_PATH")
CHART_DIR = os.environ.get("CHART_DIR")
# Where the charts end up once CI has uploaded them. GitHub strips data-URI
# images from comment markdown, so the report links instead of embedding. Keep
# the artifact name in sync with the upload step in the workflow.
RUN_URL = os.environ.get("RUN_URL")
CHART_ARTIFACT = "mono-move-e2e-perf-charts"
HIDE_OUTPUT = bool(os.environ.get("HIDE_OUTPUT"))
# Blocks dropped from the head of the steady-state window. Fixed rather than
# derived per run: a cutoff that moves with the data makes two runs
# incomparable, which is the problem the window exists to fix.
WARMUP_BLOCKS = int(os.environ.get("WARMUP_BLOCKS", default=2))
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

if RUN_SOURCE not in ("ci", "manual", "local"):
    print(f"RUN_SOURCE must be 'ci', 'manual' or 'local', got {RUN_SOURCE!r}")
    sys.exit(1)

BUILD_FLAG = "--release" if BUILD == "release" else "--profile performance"
BUILD_FOLDER = f"target/{BUILD}"

MAIN_SIGNER_ACCOUNTS = 2 * BLOCK_SIZE
ADDITIONAL_DST_POOL_ACCOUNTS = 2 * BLOCK_SIZE * NUM_BLOCKS
# The account pool has to fit inside the warmup DB.
NUM_ACCOUNTS = max(NUM_INIT_ACCOUNTS, (2 + 2 * NUM_BLOCKS) * BLOCK_SIZE)


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

# Printed once per run by BlockMeasurements::print_json_line in
# execution/executor-benchmark/src/measurements.rs.
BLOCK_MEASUREMENTS_MARKER = "BLOCK_MEASUREMENTS_JSON: "

# Printed once per run by OverallMeasurement::print_counters_json_line, in the
# same file.
STAGE_COUNTERS_MARKER = "STAGE_COUNTERS_JSON: "


def extract_marked_json(output, marker, what):
    lines = [line for line in output.splitlines() if line.startswith(marker)]
    payload = get_only(lines, what)
    return json.loads(payload[len(marker) :])


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

    counters = extract_marked_json(output, STAGE_COUNTERS_MARKER, "stage counters line")
    return RunStats(
        tps=tps,
        stage_tps=stage_tps,
        output_bytes_per_txn=output_bpt,
        mono_move_enabled=mono_move_was_enabled(output),
        blocks=extract_marked_json(
            output, BLOCK_MEASUREMENTS_MARKER, "block measurements line"
        )["blocks"],
        timers={family: counters[family] for family in ("executor", "storage")},
    )


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
    "total_steady",
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


def median_run(runs):
    """The repeat that landed in the middle by execution throughput. An actual
    run rather than an interpolated one, so its per-block records are real.
    """
    ordered = sorted(runs, key=lambda r: r.stage_tps["execution"])
    return ordered[len(ordered) // 2]


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
        f"--block-size {BLOCK_SIZE} --execution-threads {CREATE_DB_THREADS} "
        f"create-db --data-dir {db_dir} --num-accounts {NUM_ACCOUNTS}"
    )


def common_flags(workload, db_dir, checkpoint_dir):
    return (
        f"RUST_BACKTRACE=1 {BUILD_FOLDER}/aptos-executor-benchmark "
        f"--block-executor-type aptos-vm-with-block-stm "
        f"--execution-threads 1 --generate-then-execute "
        # Several generator threads assign sequence numbers in whatever order
        # they run, but the block keeps the order the slots were laid out in. A
        # workload that draws the same account twice in a block then lands its
        # transactions reversed, and the second one is discarded.
        f"--num-generator-workers 1 "
        f"--block-size {BLOCK_SIZE} "
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
    """One line per calibrated metric, for Humio to aggregate.

    Emitting exactly the calibrated metrics is a choice, not a requirement. Any
    metric in METRICS can be sent; a band is not a prerequisite for a chart.
    """
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
                    "block_size": BLOCK_SIZE,
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


def execution_table(results, failures):
    """The verdict table. Execution is the only calibrated metric."""
    headers = [
        "workload",
        "V1 exec txn/s",
        "MonoMove exec txn/s",
        "execution speedup",
        "calibrated",
        "Block-STM",
        "execution range",
        "verdict",
    ]
    rows = []
    for r in results:
        v1_exec, _ = summarize(r.v1_runs, "execution")
        mono_exec, _ = summarize(r.mono_runs, "execution")
        rows.append(
            [
                r.workload.name,
                f"{v1_exec:.0f}",
                f"{mono_exec:.0f}",
                ratio(r.speedup["execution"]),
                calibrated(r, "execution"),
                ratio(r.speedup["inner_block_executor"]),
                f"{r.spread['execution'] * 100:.1f}%",
                r.verdict,
            ]
        )
    for name, _ in failures:
        rows.append([name] + ["-"] * (len(headers) - 2) + ["failed"])
    return tabulate(rows, headers=headers, tablefmt="github")


def end_to_end_table(results, failures):
    """Wall clock over the whole pipeline. Reported, never calibrated."""
    headers = [
        "workload",
        "V1 txn/s",
        "MonoMove txn/s",
        "total speedup",
        "steady speedup",
        "sigver",
        "ledger update",
        "commit",
        "total range",
    ]
    rows = []
    for r in results:
        rows.append(
            [
                r.workload.name,
                f"{statistics.median([x.tps for x in r.v1_runs]):.0f}",
                f"{statistics.median([x.tps for x in r.mono_runs]):.0f}",
                ratio(r.speedup["total"]),
                ratio(r.speedup["total_steady"]),
                ratio(r.speedup["sigver"]),
                ratio(r.speedup["ledger_update"]),
                ratio(r.speedup["commit"]),
                f"{r.spread['total'] * 100:.1f}%",
            ]
        )
    for name, _ in failures:
        rows.append([name] + ["-"] * (len(headers) - 1))
    return tabulate(rows, headers=headers, tablefmt="github")


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
            ]
        )
    return tabulate(
        rows,
        headers=[
            "workload",
            "V1 bytes/txn",
            "MonoMove bytes/txn",
            "bytes/txn ratio",
        ],
        tablefmt="github",
    )


# The rows of the two measurement tables: how deep to indent, what to call the
# stage, and which timer label it reads. A stage the run never entered is
# dropped rather than printed as a zero. Labels the benchmark reports but no row
# names are left out of the report entirely; the stdout tables carry all of them.
#
# Indent means containment: an indented stage runs inside the one above it, so
# its time is already counted there.
EXECUTION_ROWS = [
    (0, "build the state view", "get_state_view"),
    (0, "run the block through the VM", "vm_execute_block"),
    (0, "parse the VM output", "parse_raw_output"),
    (1, "collect statuses", "parse_raw_output__all_statuses"),
    (1, "split off retries and discards", "parse_raw_output__retries_and_discards"),
    (1, "index what will be committed", "parse_raw_output__to_commit"),
    (1, "read the next epoch state", "parse_raw_output__next_epoch_state"),
    (0, "state checkpoint", "do_state_checkpoint"),
    (1, "hash the state tree", "get_state_checkpoint_hashes"),
    (1, "hash the hot state tree", "get_hot_state_checkpoint_hashes"),
    (1, "hash the position tree", "get_position_checkpoint_hashes"),
    (0, "ledger update", "do_ledger_update"),
    (1, "assemble transaction infos", "assemble_transaction_infos"),
    (0, "commit the ledger", "commit_ledger"),
    (0, "count executed txns, off thread", "async_update_counters__by_execution"),
    (0, "count committed txns, off thread", "async_update_counters__by_output"),
]

STORAGE_ROWS = [
    (0, "pre-commit the block", "pre_commit_ledger"),
    (1, "write the ledger and the state", "save_transactions__work"),
    (2, "state values and ledger metadata", "commit_state_kv_and_ledger_metadata"),
    (3, "RocksDB write", "commit_state_kv_and_ledger_metadata___commit"),
    (2, "write sets", "commit_write_sets"),
    (3, "RocksDB write", "commit_write_sets___commit"),
    (2, "transactions", "commit_transactions"),
    (3, "RocksDB write", "commit_transactions___commit"),
    (2, "events", "commit_events"),
    (3, "RocksDB write", "commit_events___commit"),
    (2, "transaction accumulator", "commit_transaction_accumulator"),
    (3, "RocksDB write", "commit_transaction_accumulator___commit"),
    (1, "hand the state to the merkle pipeline", "save_transactions__others"),
    (0, "commit the ledger metadata", "commit_ledger"),
    (0, "merkle tree commit, off thread", "batch_committer_work"),
]

# What pre-commit splits into, for the per-workload table. Only MonoMove's
# side is broken out; the V1 total beside it is what there is room for.
STORAGE_SUMMARY_ROWS = [
    ("state kv", "commit_state_kv_and_ledger_metadata"),
    ("write sets", "commit_write_sets"),
    ("txns", "commit_transactions"),
    ("events", "commit_events"),
    ("accumulator", "commit_transaction_accumulator"),
]


def aggregate_timers(results, runs_of, family):
    """One VM's timer totals summed over the median run of every workload.

    Returns the totals keyed by label and the number of blocks they cover, so
    a caller can put everything on a per-block footing.
    """
    totals = {}
    num_blocks = 0
    for r in results:
        run = median_run(runs_of(r))
        num_blocks += len(run.blocks)
        for label, counter in run.timers[family].items():
            entry = totals.setdefault(label, {"total_s": 0.0, "calls": 0})
            entry["total_s"] += counter["total_s"]
            entry["calls"] += counter["calls"]
    return totals, num_blocks


def ms_per_block(totals, label, num_blocks):
    counter = totals.get(label)
    if counter is None or not num_blocks:
        return None
    return counter["total_s"] * 1000.0 / num_blocks


def indent(depth, name):
    """Markdown eats leading spaces in a table cell, so indent with entities."""
    if depth == 0:
        return name
    return "&nbsp;" * 2 * depth + "└ " + name


def timer_table(results, rows, family):
    """One row per timer, in the order the stages nest."""
    v1_totals, v1_blocks = aggregate_timers(results, lambda r: r.v1_runs, family)
    mono_totals, mono_blocks = aggregate_timers(results, lambda r: r.mono_runs, family)

    table = []
    for depth, name, label in rows:
        v1_ms = ms_per_block(v1_totals, label, v1_blocks)
        mono_ms = ms_per_block(mono_totals, label, mono_blocks)
        if v1_ms is None and mono_ms is None:
            continue
        calls = mono_totals.get(label, {}).get("calls", 0)
        table.append(
            [
                indent(depth, name),
                f"`{label}`",
                "-" if v1_ms is None else f"{v1_ms:.1f}",
                "-" if mono_ms is None else f"{mono_ms:.1f}",
                ratio(v1_ms / mono_ms) if v1_ms and mono_ms else "-",
                f"{calls / mono_blocks:.1f}" if mono_blocks else "-",
            ]
        )
    return tabulate(
        table,
        headers=[
            "stage",
            "timer",
            "V1 ms/block",
            "MonoMove ms/block",
            "speedup",
            "calls/block",
        ],
        tablefmt="github",
        disable_numparse=True,
    )


def storage_per_workload_table(results):
    """Pre-commit and what it splits into, one row per workload."""

    def cell(value):
        return "-" if value is None else f"{value:.1f}"

    rows = []
    for r in results:
        v1 = median_run(r.v1_runs)
        mono = median_run(r.mono_runs)
        v1_total = ms_per_block(v1.timers["storage"], "pre_commit_ledger", len(v1.blocks))
        mono_total = ms_per_block(
            mono.timers["storage"], "pre_commit_ledger", len(mono.blocks)
        )
        rows.append(
            [
                r.workload.name,
                cell(v1_total),
                cell(mono_total),
                ratio(v1_total / mono_total) if v1_total and mono_total else "-",
            ]
            + [
                cell(ms_per_block(mono.timers["storage"], label, len(mono.blocks)))
                for _, label in STORAGE_SUMMARY_ROWS
            ]
        )
    return tabulate(
        rows,
        headers=["workload", "V1 total", "MonoMove total", "speedup"]
        + [name for name, _ in STORAGE_SUMMARY_ROWS],
        tablefmt="github",
        disable_numparse=True,
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
    parts = [
        "<details>",
        "<summary>Column reference, verdicts, workloads</summary>",
        "",
        "#### Columns",
        "",
        "Every throughput is a median over the repeats, and every ratio is "
        "MonoMove over V1.",
        "",
        "- `V1 exec txn/s`, `MonoMove exec txn/s` — throughput of the execute "
        "stage alone.",
        "- `execution speedup` — the executor's execute stage alone. The only "
        "calibrated metric, and the only one that decides a verdict.",
        "- `calibrated` — `execution` as recorded in `e2e_perf_speedup.tsv`: the "
        "median over the runs that row was last seeded from, and in parentheses "
        "how far this run moved from it. `-` when the workload has no row yet.",
        "- `Block-STM` — `BlockExecutor::execute_block`, run sequentially here. "
        "The innermost of the three timers and the closest proxy for VM-only "
        "time: it leaves out the block setup and output conversion that "
        "`execution` carries.",
        "- `execution range`, `total range` — `(max - min) / median` across one "
        "VM's repeats of that metric, reported for whichever VM came out worse. "
        "It says how repeatable each side was, not how uncertain the ratio is. "
        "The two VMs run alternating, so drift hits both and largely cancels in "
        "the ratio. Only `execution range` bears on the verdict, which becomes "
        f"`noisy` past {MAX_DEVIATION * 100:.0f}%.",
        "- `total speedup` — wall clock over the whole pipeline.",
        f"- `steady speedup` — the same, with the first {WARMUP_BLOCKS} blocks "
        "dropped, measured at the commit stage from the per-block timings. It "
        f"and `total speedup` both divide by {BLOCK_SIZE} user transactions "
        "plus the block metadata. The block epilogue is left out of that count "
        "but not out of the time, so the two throughputs compare directly.",
        "- `sigver` — signature verification. Does identical work in both "
        "replays, so it sits near 1.00x and whatever it deviates by is noise and "
        "core contention with the concurrent execution stage.",
        "- `ledger update`, `commit` — the two stages after execution. Neither "
        "runs Move code; both move with how much the VM wrote, so they track the "
        "bytes/txn ratio rather than staying flat.",
        "- `bytes/txn` — what each VM wrote per transaction.",
        "",
        "The stages nest: `total` ⊃ `execution` ⊃ `Block-STM`.",
        "",
        "#### Measurement columns",
        "",
        "- `stage`, `timer` — the stage in words and the Prometheus label it "
        "reads. Indentation means containment: an indented stage runs inside "
        "the one above it, so its time is already counted there.",
        "- `V1 ms/block`, `MonoMove ms/block` — the timer's total over the "
        "median run of every workload, divided by the blocks those runs "
        "covered. It is time summed across threads, not a share of block "
        "latency: several of these stages run concurrently in one rayon scope, "
        "so the children can add up past their parent's wall clock.",
        "- `speedup` — V1 over MonoMove, inverted from the raw times so that "
        "above 1.00x still means MonoMove is faster.",
        "- `calls/block` — how often the timer fired per block, counted on the "
        "MonoMove runs. Both VMs commit the same blocks, so V1 fires the same "
        "number of times unless a stage is driven by how much was written.",
        "",
        "A stage no run entered is left out rather than printed as a zero. A "
        "timer with no row is left out of the report; the benchmark's stdout "
        "carries every label.",
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
        f"- `{w.name}` — {w.description}"
        for w in selected
    ]
    parts += [
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


def chart_section(charts):
    """A pointer to the rendered per-block charts, when there are any."""
    if not charts:
        return []
    where = (
        f"[the `{CHART_ARTIFACT}` artifact]({RUN_URL}#artifacts) on this run"
        if RUN_URL
        else f"`{CHART_DIR}`"
    )
    return [
        "",
        f"One SVG per workload in {where}: every block's execution, ledger "
        "update, and commit time, one panel per VM. The bars are grouped "
        "rather than stacked because the stages run concurrently, so they do "
        "not sum to block latency. Each panel is scaled to its own peak, named "
        "in its top right, so compare the two by axis and not by bar height. "
        "This is where to look when the end-to-end number moves but execution "
        "does not.",
    ]


def build_report(selected, results, failures, charts):
    title = "MonoMove vs V1 MoveVM, sequential execution"
    if SELF_COMPARE:
        title += " (SELF_COMPARE: V1 vs V1, everything should be 1.00x)"

    parts = [
        f"### {title}",
        "",
        f"{NUM_BLOCKS} blocks of {BLOCK_SIZE} transactions, {REPEATS} repeats "
        f"per VM, {NUM_ACCOUNTS} account DB, "
        f"`{BUILD}` build. Ratios are MonoMove over V1; above 1.00x means "
        f"MonoMove is faster.",
        "",
        "Gas is not compared. MonoMove runs unmetered, so its gas metrics are zero.",
        "",
        "#### Overall",
        "",
        execution_table(results, failures),
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
        "Wall clock over the whole pipeline. Not calibrated: the four stages run "
        "concurrently, so this tracks whichever is slowest, and MonoMove is fast "
        "enough that the slowest one is commit. That makes it several times "
        "noisier than execution without catching anything execution would miss.",
        "",
        end_to_end_table(results, failures),
        "",
    ]

    if results:
        parts += [
            "#### Execution measurements",
            "",
            "Where the executor spent its time, from "
            "`aptos_executor_other_timers_seconds`.",
            "",
            timer_table(results, EXECUTION_ROWS, "executor"),
            "",
            "#### Storage measurements",
            "",
            "Where AptosDB spent its time, from "
            "`aptos_storage_other_timers_seconds`. This is what the commit stage "
            "is made of.",
            "",
            timer_table(results, STORAGE_ROWS, "storage"),
            "",
            "<details>",
            "<summary>Storage per workload</summary>",
            "",
            "Pre-commit per workload rather than summed, and what MonoMove's "
            "share of it went into. Same units as above.",
            "",
            storage_per_workload_table(results),
            "",
            "</details>",
            "",
        ]

    parts += [
        "#### Output size",
        "",
        "Bytes per transaction records what each VM wrote. The two need not "
        "match: MonoMove is unmetered today, so it writes no fee slots, and it "
        "copies on every `borrow_global_mut`, so its write set is an "
        "overapproximation. Either can change.",
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

    charts_section = chart_section(charts)
    if charts_section:
        parts += ["", "#### Artifacts"] + charts_section

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

    calibration = load_calibration()
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

    charts = []
    if CHART_DIR and results:
        charts = write_charts(
            CHART_DIR,
            [
                (
                    r.workload.name,
                    median_run(r.v1_runs).blocks,
                    median_run(r.mono_runs).blocks,
                )
                for r in results
            ],
        )
        print(f"Charts written to {CHART_DIR}: {', '.join(charts)}")

    report = build_report(selected, results, failures, charts)
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
