#!/usr/bin/env python3
# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

"""Verify a Move package with one `verex-move verify` process per module.

`verex-move verify <package>` verifies every target of a package in one
process, one target after another. This driver starts one process per module
that has specifications, `--jobs` at a time, longest first (by each module's
time in the previous run), and prints each module's status, then the modules'
reports in module order:

    scripts/verify-modules.py <package> [--filter <file name part>]
        [--modules a::m,...] [--jobs N] [--heartbeats N] [--ignore-proofs] [--dev]
        [--no-build] [--logs <dir>]

Each process exports only what verifying its module reads (`--modules`) and
writes its rendering into the log directory, so the package's
`build/verex-verify.lean` is left as the editor reads it. A module's opaque
callees in other modules are verified again in every importer
(`roadmap.md`, R3), so a report can repeat across modules; it is printed
once. `VEREX_OPTIONS` reaches every process; each proves its module's targets
several at a time (`verex.jobs`), the processes sharing half the hardware
threads, unless `VEREX_OPTIONS` sets `verex.jobs` itself.
"""

import argparse
import concurrent.futures
import json
import os
import re
import signal
import subprocess
import sys
import tempfile
import threading
import time
from pathlib import Path

LEAN_DIR = Path(__file__).resolve().parent.parent
REPO = LEAN_DIR.parents[2]
VEREX_MOVE = LEAN_DIR / "verex-move"
EXECUTABLE = VEREX_MOVE / ".lake" / "build" / "bin" / "verex-move"
# Each module's wall time in the previous run, by package, for the schedule.
DURATIONS = VEREX_MOVE / ".lake" / "verify-modules-durations.json"
# Memory one verification process is planned with when choosing the default jobs.
PROCESS_GB = 4

DECLARATION = re.compile(r"^[ \t]*module[ \t]+(\w+)::(\w+)[ \t]*[{;]", re.M)
SPEC_OF = re.compile(r"^[ \t]*spec[ \t]+(\w+)::(\w+)[ \t]*\{", re.M)
SPEC = re.compile(r"^[ \t]*spec\b", re.M)
TEST_ONLY = re.compile(r"#\[\s*test_only\s*\]\s*$")
REPORT = re.compile(r"^\S[^\n]*?:\d+:\d+: (?:error|warning|info|information): ", re.M)


def fail(message):
    print(f"verify-modules: {message}", file=sys.stderr)
    sys.exit(2)


def modules_of(package, filter_part):
    """The package's modules with specifications, as `address::name`, with the
    source file declaring each; with `filter_part`, those whose file name
    contains it, as `verex-move verify --filter` selects."""
    declared = {}
    specified = set()
    for path in sorted((package / "sources").rglob("*.move")):
        text = path.read_text(errors="replace")
        has_spec = SPEC.search(text) is not None
        for match in DECLARATION.finditer(text):
            previous = text[:match.start()].rstrip().splitlines()[-1:]
            if previous and TEST_ONLY.search(previous[0]):
                continue
            module = (match.group(1), match.group(2))
            declared[module] = path
            if has_spec:
                specified.add(module)
        for match in SPEC_OF.finditer(text):
            specified.add((match.group(1), match.group(2)))
    selected = {}
    for module, path in declared.items():
        if module in specified and (filter_part is None or filter_part in path.name):
            selected[f"{module[0]}::{module[1]}"] = path
    return dict(sorted(selected.items()))


def default_jobs():
    """Half the hardware threads, at most one process per `PROCESS_GB` of
    available memory."""
    jobs = max(1, (os.cpu_count() or 2) // 2)
    try:
        with open("/proc/meminfo") as meminfo:
            for line in meminfo:
                if line.startswith("MemAvailable:"):
                    jobs = min(jobs, int(line.split()[1]) // (PROCESS_GB * 1024 * 1024))
    except OSError:
        pass
    return max(1, jobs)


def lake_environment():
    """The environment `lake env` gives `verex-move`, so each process runs
    without `lake` around it."""
    probe = "import json, os; print(json.dumps(dict(os.environ)))"
    output = subprocess.run(["lake", "env", sys.executable, "-c", probe], cwd=VEREX_MOVE,
                            capture_output=True, text=True)
    if output.returncode != 0:
        fail(f"`lake env` failed in {VEREX_MOVE}:\n{output.stderr}")
    env = json.loads(output.stdout.strip().splitlines()[-1])
    move_cli = os.environ.get("APTOS_MOVE_CLI") or str(REPO / "target" / "ci" / "move")
    if not os.environ.get("APTOS_CLI") and not Path(move_cli).is_file():
        fail(f"no Move CLI at {move_cli}; build it with `cargo build --locked --profile ci "
             "-p aptos-move-cli --features binary --bin move` or set APTOS_MOVE_CLI")
    env.setdefault("APTOS_MOVE_CLI", move_cli)
    return env


def share_threads(env, processes):
    """Each process proves the targets of its module several at a time
    (`verex.jobs`): the processes share half the hardware threads, unless
    `VEREX_OPTIONS` sets the option itself."""
    options = [option for option in env.get("VEREX_OPTIONS", "").split(",") if option]
    if any(option.startswith("verex.jobs=") for option in options):
        return env
    jobs = max(1, max(1, (os.cpu_count() or 2) // 2) // max(1, processes))
    env = dict(env)
    env["VEREX_OPTIONS"] = ",".join(options + [f"verex.jobs={jobs}"])
    return env


def build():
    started = time.monotonic()
    output = subprocess.run(["lake", "build", "verex-move"], cwd=VEREX_MOVE,
                            capture_output=True, text=True)
    if output.returncode != 0:
        print("\n".join((output.stdout + output.stderr).splitlines()[-30:]), file=sys.stderr)
        fail("building verex-move failed")
    print(f"verify-modules: built verex-move in {time.monotonic() - started:.1f}s",
          file=sys.stderr)


class Runner:
    """Runs the modules' processes, each in its own process group, so that an
    interrupted run stops all of them."""

    def __init__(self, package, env, logs, extra):
        self.package, self.env, self.logs, self.extra = package, env, logs, extra
        self.groups = set()
        self.lock = threading.Lock()
        self.finished = 0

    def run(self, selector, total):
        stem = selector.replace("::", "__")
        rendering = self.logs / f"{stem}.lean"
        command = [str(EXECUTABLE), "verify", str(self.package), "--modules", selector,
                   "--output", str(rendering), *self.extra]
        started = time.monotonic()
        with open(self.logs / f"{stem}.out", "w") as out, \
                open(self.logs / f"{stem}.err", "w") as err:
            process = subprocess.Popen(command, cwd=self.package, env=self.env, stdout=out,
                                       stderr=err, start_new_session=True)
            with self.lock:
                self.groups.add(process.pid)
            _, status, usage = os.wait4(process.pid, 0)
            with self.lock:
                self.groups.discard(process.pid)
        wall = time.monotonic() - started
        stdout = (self.logs / f"{stem}.out").read_text(errors="replace")
        stderr = (self.logs / f"{stem}.err").read_text(errors="replace")
        code = os.waitstatus_to_exitcode(status)
        errors = sum(1 for match in REPORT.finditer(stdout) if ": error: " in match.group(0))
        if "verex-move: verification complete" not in stderr:
            outcome = "crashed"
        elif errors or code != 0:
            outcome = "failed"
        else:
            outcome = "verified"
        result = {"module": selector, "outcome": outcome, "exit": code, "wall": wall,
                  "cpu": usage.ru_utime + usage.ru_stime, "rss_gb": usage.ru_maxrss / 1048576,
                  "errors": errors, "stdout": stdout, "stderr": stderr}
        with self.lock:
            self.finished += 1
            print(f"[{self.finished}/{total}] {selector} {outcome} {wall:.1f}s", file=sys.stderr,
                  flush=True)
        return result

    def stop(self):
        with self.lock:
            for group in self.groups:
                try:
                    os.killpg(group, signal.SIGKILL)
                except ProcessLookupError:
                    pass


def reports_of(stdout):
    """The reports of a process's output, each a header line and its message."""
    starts = [match.start() for match in REPORT.finditer(stdout)]
    if not starts:
        return [stdout.strip()] if stdout.strip() else []
    lead = stdout[:starts[0]].strip()
    pieces = [lead] if lead else []
    for begin, end in zip(starts, starts[1:] + [len(stdout)]):
        pieces.append(stdout[begin:end].rstrip())
    return pieces


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("package", type=Path)
    parser.add_argument("--filter", help="only modules whose source file name contains this")
    parser.add_argument("--modules", help="these modules only (`address::name`, comma-separated)")
    parser.add_argument("--jobs", "-j", type=int, help="processes at once (default: "
                        f"half the hardware threads, one per {PROCESS_GB} GB available)")
    parser.add_argument("--heartbeats", help="passed to verex-move verify")
    parser.add_argument("--ignore-proofs", action="store_true", help="passed to verex-move verify")
    parser.add_argument("--dev", action="store_true", help="passed to verex-move verify")
    parser.add_argument("--no-build", action="store_true", help="use the built verex-move as it is")
    parser.add_argument("--logs", type=Path, help="keep each module's output and rendering here")
    args = parser.parse_args()

    package = args.package.resolve()
    if not (package / "Move.toml").is_file():
        fail(f"{package} is not a Move package")
    if args.modules and args.filter:
        fail("--modules and --filter exclude each other")
    if args.modules:
        selected = {name: None for name in args.modules.split(",")}
    else:
        selected = modules_of(package, args.filter)
    if not selected:
        fail("no module with specifications selected")
    extra = []
    if args.heartbeats:
        extra += ["--heartbeats", args.heartbeats]
    if args.ignore_proofs:
        extra.append("--ignore-proofs")
    if args.dev:
        extra.append("--dev")
    jobs = args.jobs or default_jobs()

    if not args.no_build:
        build()
    env = share_threads(lake_environment(), jobs)
    logs = args.logs or Path(tempfile.mkdtemp(prefix="verify-modules."))
    logs.mkdir(parents=True, exist_ok=True)

    try:
        durations = json.loads(DURATIONS.read_text())
    except (OSError, ValueError):
        durations = {}
    known = durations.get(str(package), {})

    def estimate(name):
        if name in known:
            return known[name]
        path = selected[name]
        return float("inf") if path is None else path.stat().st_size

    # Unmeasured modules first, by size; then the longest of the last run.
    order = sorted(selected, key=lambda name: (name not in known, estimate(name)), reverse=True)
    print(f"verify-modules: {len(order)} modules of {package.name}, {jobs} at a time, "
          f"logs in {logs}", file=sys.stderr, flush=True)

    runner = Runner(package, env, logs, extra)
    started = time.monotonic()
    results = {}
    with concurrent.futures.ThreadPoolExecutor(max_workers=jobs) as pool:
        futures = {pool.submit(runner.run, name, len(order)): name for name in order}
        try:
            for future in concurrent.futures.as_completed(futures):
                result = future.result()
                results[result["module"]] = result
        except BaseException:
            for future in futures:
                future.cancel()
            runner.stop()
            raise
    wall = time.monotonic() - started

    known.update({name: round(result["wall"], 1) for name, result in results.items()})
    durations[str(package)] = known
    try:
        DURATIONS.write_text(json.dumps(durations, indent=1, sort_keys=True) + "\n")
    except OSError:
        pass

    width = max(len(name) for name in results)
    print(f"{'module':<{width}}  {'outcome':<8} {'wall':>7} {'cpu':>7} {'rss':>7} {'errors':>6}")
    for name in sorted(results):
        result = results[name]
        print(f"{name:<{width}}  {result['outcome']:<8} {result['wall']:6.1f}s "
              f"{result['cpu']:6.1f}s {result['rss_gb']:5.1f}GB {result['errors']:6d}")
    seen = set()
    for name in sorted(results):
        result = results[name]
        if result["outcome"] == "crashed":
            print(f"\n{name}: crashed (exit {result['exit']}); last lines of its error output:")
            print("\n".join(result["stderr"].splitlines()[-15:]))
        for report in reports_of(result["stdout"]):
            report = report.replace(f"{package}/", "").replace(f"{REPO}/", "")
            if report not in seen:
                seen.add(report)
                print(report)
    total = sum(result["wall"] for result in results.values())
    failed = [name for name, result in results.items() if result["outcome"] != "verified"]
    print(f"verify-modules: {len(results)} modules, {len(failed)} not verified; wall {wall:.1f}s, "
          f"modules' sum {total:.1f}s ({total / wall:.1f}× parallel), {jobs} jobs; logs in {logs}")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
