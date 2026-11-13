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
        [--modules a::m,...] [--jobs N] [--heartbeats N] [--timeout SECONDS]
        [--ignore-proofs] [--dev] [--no-build] [--logs <dir>]
        [--namespaces <dir>] [--no-namespaces]

A first pass compiles the namespaces, as Lean compiles modules: the package
and its dependencies are exported once, each namespace rendered to a file of
its own and elaborated to an `.olean` file, in dependency order, several at a
time. They live in `build/verex/namespaces` of the package (`--namespaces`),
each with a trace, the hash of what it was made from: the executable, the
libraries it is elaborated over, the options, its rendering, and the traces of
what it imports. A namespace whose
trace matches is not compiled again. With a narrowed run (`--filter`, `--modules`),
only the namespaces the selected modules import are.

Each verification process exports only what verifying its module reads
(`--modules`), imports the namespaces before its module, and writes its
rendering into the log directory, so the package's
`build/verex-verify.lean` is left as the editor reads it. With
`--no-namespaces` a rendering elaborates all of them. A module's opaque
callees in other modules are verified again in every importer
(`roadmap.md`, R3), so a report can repeat across modules; it is printed
once. `VEREX_OPTIONS` reaches every process; each proves its module's targets
several at a time (`verex.jobs`), the processes sharing half the hardware
threads, unless `VEREX_OPTIONS` sets `verex.jobs` itself. With `--logs`, each
module's stage log (`verex.stageLog`), one line per target with its outcome, is
kept there as `<module>.stages`. `--timeout` stops a module's process after that
many seconds and reports it as `timeout`; its stage log keeps what finished.
"""

import argparse
import concurrent.futures
import hashlib
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


def with_option(env, option):
    """`env` with `option` added to `VEREX_OPTIONS`."""
    options = [existing for existing in env.get("VEREX_OPTIONS", "").split(",") if existing]
    return dict(env, VEREX_OPTIONS=",".join(options + [option]))


def share_threads(env, processes):
    """Each process proves the targets of its module several at a time
    (`verex.jobs`): the processes share half the hardware threads, unless
    `VEREX_OPTIONS` sets the option itself."""
    options = [option for option in env.get("VEREX_OPTIONS", "").split(",") if option]
    if any(option.startswith("verex.jobs=") for option in options):
        return env
    jobs = max(1, max(1, (os.cpu_count() or 2) // 2) // max(1, processes))
    return with_option(env, f"verex.jobs={jobs}")


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

    def __init__(self, package, env, logs, extra, timeout):
        self.package, self.env, self.logs, self.extra = package, env, logs, extra
        self.timeout = timeout
        self.groups = set()
        self.lock = threading.Lock()
        self.finished = 0

    def run(self, selector, total):
        stem = selector.replace("::", "__")
        rendering = self.logs / f"{stem}.lean"
        command = [str(EXECUTABLE), "verify", str(self.package), "--modules", selector,
                   "--output", str(rendering), *self.extra]
        stages = self.logs / f"{stem}.stages"
        stages.unlink(missing_ok=True)
        env = with_option(self.env, f"verex.stageLog={stages}")
        started = time.monotonic()
        with open(self.logs / f"{stem}.out", "w") as out, \
                open(self.logs / f"{stem}.err", "w") as err:
            process = subprocess.Popen(command, cwd=self.package, env=env, stdout=out,
                                       stderr=err, start_new_session=True)
            with self.lock:
                self.groups.add(process.pid)
            timer = None
            expired = threading.Event()
            if self.timeout:
                def expire():
                    expired.set()
                    os.killpg(process.pid, signal.SIGKILL)
                timer = threading.Timer(self.timeout, expire)
                timer.start()
            _, status, usage = os.wait4(process.pid, 0)
            if timer:
                timer.cancel()
            with self.lock:
                self.groups.discard(process.pid)
        wall = time.monotonic() - started
        stdout = (self.logs / f"{stem}.out").read_text(errors="replace")
        stderr = (self.logs / f"{stem}.err").read_text(errors="replace")
        code = os.waitstatus_to_exitcode(status)
        errors = sum(1 for match in REPORT.finditer(stdout) if ": error: " in match.group(0))
        if expired.is_set():
            outcome = "timeout"
        elif "verex-move: verification complete" not in stderr:
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


def file_hash(path):
    digest = hashlib.sha256()
    with open(path, "rb") as stream:
        while block := stream.read(1 << 20):
            digest.update(block)
    return digest.hexdigest()


def run_tool(arguments, cwd, env, log, timeout=None):
    """Run the executable with its output in `log`: whether it succeeded."""
    with open(log, "w") as out:
        try:
            return subprocess.run([str(EXECUTABLE), *arguments], cwd=cwd, env=env, stdout=out,
                                  stderr=subprocess.STDOUT, timeout=timeout).returncode == 0
        except subprocess.TimeoutExpired:
            out.write(f"\nverify-modules: stopped after {timeout} seconds\n")
            return False


def imports_of(rendering):
    """The compiled namespaces a rendering's header imports."""
    found = []
    with open(rendering, errors="replace") as stream:
        for line in stream:
            match = re.match(r"import (VerexNamespaces\.\S+)", line)
            if match:
                found.append(match.group(1))
            elif line.startswith("verex ") or line.startswith("/-!"):
                break
    return found


def libraries_hash(env, root):
    """The hash of the libraries a namespace is elaborated over: each library's
    Lake hash, or the size and time of a library without one, as the toolchain's."""
    output = subprocess.run([str(EXECUTABLE), "snapshot-modules"], cwd=root, env=env,
                            capture_output=True, text=True)
    if output.returncode != 0:
        fail(f"listing the libraries failed:\n{output.stderr}")
    digest = hashlib.sha256()
    for line in sorted(output.stdout.splitlines()):
        olean = Path(line)
        recorded = olean.with_name(olean.name + ".hash")
        if recorded.is_file():
            digest.update(recorded.read_bytes())
        else:
            status = olean.stat()
            digest.update(f"{line} {status.st_size} {status.st_mtime_ns}".encode())
    return digest.hexdigest()


def render_namespaces(package, root, env, args):
    """Render the namespaces of the package and its dependencies: by module, each
    with its file and imports."""
    root.mkdir(parents=True, exist_ok=True)
    plan = root / "plan.jsonl"
    command = ["render-namespaces", str(package), str(root)] + (["--dev"] if args.dev else [])
    started = time.monotonic()
    with open(plan, "w") as out, open(root / "render.err", "w") as err:
        code = subprocess.run([str(EXECUTABLE), *command], cwd=package, env=env, stdout=out,
                              stderr=err).returncode
    if code != 0:
        fail(f"rendering the namespaces of {package} failed:\n"
             + (root / "render.err").read_text(errors="replace"))
    print(f"verify-modules: rendered namespaces in {time.monotonic() - started:.1f}s",
          file=sys.stderr, flush=True)
    return {entry["module"]: entry
            for entry in map(json.loads, plan.read_text().splitlines())}


def compile_namespaces(plan, wanted, root, env, args, jobs, options):
    """Compile the namespaces `wanted` and what they import, each once what it
    imports is compiled, `jobs` at a time; `options` is the `VEREX_OPTIONS` the
    caller set. The modules that failed, with their output."""
    closure = set()
    pending = list(wanted)
    while pending:
        module = pending.pop()
        if module not in closure:
            closure.add(module)
            pending.extend(plan[module]["imports"])
    # A namespace's lemmas are verified under the budget and proofs it is compiled with.
    salt = hashlib.sha256(f"{file_hash(EXECUTABLE)} {libraries_hash(env, root)} "
                          f"{args.heartbeats} {args.ignore_proofs} {options}".encode()).hexdigest()
    traces = {}
    failed = {}
    stale = 0
    started = time.monotonic()
    summed = [0.0]

    def trace_of(module):
        entry = plan[module]
        digest = hashlib.sha256(salt.encode())
        digest.update(Path(entry["file"]).read_bytes())
        for imported in sorted(entry["imports"]):
            digest.update(traces[imported].encode())
        return digest.hexdigest()

    def compile_one(module):
        olean = Path(plan[module]["file"]).with_suffix(".olean")
        log = Path(plan[module]["file"]).with_suffix(".log")
        command = ["compile-namespace", str(root), module]
        if args.heartbeats:
            command += ["--heartbeats", args.heartbeats]
        if args.ignore_proofs:
            command.append("--ignore-proofs")
        begun = time.monotonic()
        code = run_tool(command, root, env, log, args.timeout)
        summed[0] += time.monotonic() - begun
        return module, code and olean.is_file(), log

    remaining = set(closure)
    running = {}
    with concurrent.futures.ThreadPoolExecutor(max_workers=jobs) as pool:
        while remaining or running:
            for module in sorted(remaining):
                imported = plan[module]["imports"]
                if not all(name in traces for name in imported):
                    if any(name in failed for name in imported):
                        failed[module] = f"{module}: imports a namespace that did not compile"
                        remaining.discard(module)
                    continue
                remaining.discard(module)
                traces[module] = trace_of(module)
                entry_path = Path(plan[module]["file"])
                trace_file = entry_path.with_suffix(".trace")
                if trace_file.is_file() and trace_file.read_text() == traces[module] \
                        and entry_path.with_suffix(".olean").is_file():
                    continue
                trace_file.unlink(missing_ok=True)
                entry_path.with_suffix(".olean").unlink(missing_ok=True)
                stale += 1
                # Its importers wait until it is written.
                pending_trace = traces.pop(module)
                running[pool.submit(compile_one, module)] = pending_trace
            if not running:
                if remaining and not any(all(name in traces for name in plan[module]["imports"])
                                         or any(name in failed for name in plan[module]["imports"])
                                         for module in remaining):
                    fail("the namespaces import each other in a cycle")
                continue
            done, _ = concurrent.futures.wait(
                running, return_when=concurrent.futures.FIRST_COMPLETED)
            for future in done:
                trace = running.pop(future)
                module, succeeded, log = future.result()
                if succeeded:
                    traces[module] = trace
                    Path(plan[module]["file"]).with_suffix(".trace").write_text(trace)
                else:
                    failed[module] = log.read_text(errors="replace")
                    print(f"verify-modules: namespace {module} did not compile", file=sys.stderr,
                          flush=True)
    wall = time.monotonic() - started
    print(f"verify-modules: {len(closure)} namespaces, {stale} compiled; wall {wall:.1f}s, "
          f"sum {summed[0]:.1f}s, {jobs} jobs", file=sys.stderr, flush=True)
    return failed


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


def targets_of(stages):
    """Completed target measurements, plus started but interrupted targets.

    Stage timestamps are milliseconds. Use theorem start, not the earlier
    preparation, which can interleave with preparation of other targets.
    """
    starts, targets = {}, {}
    for line in stages.splitlines():
        match = re.fullmatch(r"(\d+) (.+): theorem start", line)
        if match:
            starts[match[2]] = int(match[1])
            continue
        match = re.fullmatch(
            r"(\d+) (.+): theorem done, (\d+) heartbeats, (\w+), (.+)", line)
        if match:
            end, name, beats, status, mode = match.groups()
            targets[name] = {"target": name, "heartbeats": int(beats),
                             "status": status, "mode": mode}
            if name in starts:
                targets[name]["wall_ms"] = int(end) - starts[name]
            starts.pop(name, None)
    for name in starts:
        targets[name] = {"target": name, "status": "interrupted"}
    return list(targets.values())


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("package", type=Path)
    parser.add_argument("--filter", help="only modules whose source file name contains this")
    parser.add_argument("--modules", help="these modules only (`address::name`, comma-separated)")
    parser.add_argument("--jobs", "-j", type=int, help="processes at once (default: "
                        f"half the hardware threads, one per {PROCESS_GB} GB available)")
    parser.add_argument("--heartbeats", help="passed to verex-move verify")
    parser.add_argument("--timeout", type=float, help="stop a module's process after these seconds")
    parser.add_argument("--ignore-proofs", action="store_true", help="passed to verex-move verify")
    parser.add_argument("--dev", action="store_true", help="passed to verex-move verify")
    parser.add_argument("--no-build", action="store_true", help="use the built verex-move as it is")
    parser.add_argument("--logs", type=Path, help="keep each module's output and rendering here")
    parser.add_argument("--json", type=Path, help="write module and target measurements as JSON")
    parser.add_argument("--namespaces", type=Path, help="where the compiled namespaces are "
                        "(default: build/verex/namespaces of the package)")
    parser.add_argument("--no-namespaces", action="store_true",
                        help="elaborate every namespace in each module's process")
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

    started = time.monotonic()
    if not args.no_namespaces:
        root = (args.namespaces or package / "build" / "verex" / "namespaces").resolve()
        plan = render_namespaces(package, root, env, args)
        if args.modules or args.filter:
            # What a module's rendering imports is known once it is rendered.
            wanted = set()

            def render_only(name):
                rendering = logs / f"{name.replace('::', '__')}.lean"
                log = logs / f"{name.replace('::', '__')}.render"
                if not run_tool(["verify", str(package), "--modules", name, "--namespaces",
                                 str(root), "--render-only", "--output", str(rendering), *extra],
                                package, env, log):
                    fail(f"rendering {name} failed:\n" + log.read_text(errors="replace"))
                return imports_of(rendering)

            with concurrent.futures.ThreadPoolExecutor(max_workers=jobs) as pool:
                for imports in pool.map(render_only, order):
                    wanted.update(imports)
        else:
            wanted = set(plan)
        failed_namespaces = compile_namespaces(plan, wanted, root, env, args, jobs,
                                               os.environ.get("VEREX_OPTIONS", ""))
        for module, output in sorted(failed_namespaces.items()):
            print(f"{module}:\n{output.strip()}")
        extra = extra + ["--namespaces", str(root)]
    prepared = time.monotonic() - started
    print(f"verify-modules: namespaces ready in {prepared:.1f}s", file=sys.stderr, flush=True)

    runner = Runner(package, env, logs, extra, args.timeout)
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
    if args.json:
        modules = []
        for name in sorted(results):
            result = {key: value for key, value in results[name].items()
                      if key not in ("stdout", "stderr")}
            stages = logs / f"{name.replace('::', '__')}.stages"
            result["targets"] = targets_of(stages.read_text() if stages.exists() else "")
            modules.append(result)
        args.json.parent.mkdir(parents=True, exist_ok=True)
        args.json.write_text(json.dumps({
            "schema": "verex-module-sweep", "version": 1,
            "package": str(package), "jobs": jobs, "heartbeats": args.heartbeats,
            "wall": wall, "namespace_wall": prepared, "modules": modules,
            "namespace_failures": sorted(failed_namespaces) if not args.no_namespaces else [],
        }, indent=2) + "\n")
    print(f"verify-modules: {len(results)} modules, {len(failed)} not verified; wall {wall:.1f}s "
          f"(namespaces {prepared:.1f}s), "
          f"modules' sum {total:.1f}s ({total / wall:.1f}× parallel), {jobs} jobs; logs in {logs}")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
