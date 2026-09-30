#!/usr/bin/env python3
# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

"""The verification benchmark (`designs/verification-benchmarks.md`).

    run      measure the problems of `bench/problems.toml`
    history  fetch the results of recent CI runs
    report   render the history, with a local run appended
    compare  compare two runs of one machine

Wall time is the main measure; heartbeats compare across machines. Uses only
the standard library and the `gh` CLI (for the history).

A series of local runs on a branch: `run --only <names>` after each change,
keeping the names. It builds `leaner-bench`, measures, records the run, and
compares it with the previous one. `compare` only reads recorded runs:
`history --local` lists them, `compare @-N @-1` the change since run @-N.
"""

import argparse
import datetime
import hashlib
import html
import json
import math
import os
import platform
import shutil
import signal
import statistics
import subprocess
import sys
import time
import tomllib
from pathlib import Path

LEAN_DIR = Path(__file__).resolve().parent.parent
REPO = LEAN_DIR.parents[2]
MANIFEST = LEAN_DIR / "bench" / "problems.toml"
PACKAGE = LEAN_DIR / "leaner-e2e-tests"
# A local run's page, the history of local runs, and the problems' logs of
# the latest local run, all git-ignored.
LOCAL_PAGE = LEAN_DIR / "local_benchmark.html"
LOCAL_HISTORY = LEAN_DIR / "local_benchmark_history.jsonl"
LOCAL_LOGS = LEAN_DIR / "local_benchmark_logs"
LOCAL_KEEP = 100
SCHEMA = "leaner-bench"
VERSION = 1
WORKFLOW = "leaner-bench.yaml"
ARTIFACT = "leaner-bench-results"
KINDS = ("move", "lean", "rust")
DEFAULT_TIMEOUT = 20 * 60
DEFAULT_THREADS = 8
SPARKS = "▁▂▃▄▅▆▇█"
# The report's groups of phases, in chart order.
GROUPS = (("elaboration", "Elaboration"), ("verification", "Verification"), ("overall", "Overall"))


def fail(message):
    print(f"leaner-bench: {message}", file=sys.stderr)
    sys.exit(1)


def git(*args):
    return subprocess.run(
        ["git", *args], cwd=REPO, capture_output=True, text=True, check=True
    ).stdout.strip()


def in_ci():
    return os.environ.get("GITHUB_ACTIONS") == "true"


def current_branch():
    """The branch measured: the pull request's or the ref's in CI, the
    checked-out one locally."""
    return (os.environ.get("GITHUB_HEAD_REF") or os.environ.get("GITHUB_REF_NAME")
            or git("rev-parse", "--abbrev-ref", "HEAD"))


# --------------------------------------------------------------------------
# Manifest


def load_manifest(path):
    with open(path, "rb") as file:
        problems = tomllib.load(file).get("problem", [])
    names = set()
    for problem in problems:
        name = problem.get("name")
        if not name or name in names:
            fail(f"{path}: a problem needs a unique name ({name!r})")
        names.add(name)
        if problem.get("kind") not in KINDS:
            fail(f"{path}: `{name}` has kind {problem.get('kind')!r}, not one of {KINDS}")
        if problem["kind"] == "move" and not (problem.get("package") and problem.get("modules")):
            fail(f"{path}: the move problem `{name}` needs `package` and `modules`")
        if problem["kind"] != "move" and not problem.get("file"):
            fail(f"{path}: the {problem['kind']} problem `{name}` needs `file`")
    return problems


def input_paths(problem):
    """The tree paths a problem reads: its sources and any declared `inputs`."""
    paths = [problem.get("package") or problem["file"]]
    if problem.get("spec"):
        paths.append(problem["spec"])
    return paths + problem.get("inputs", [])


def input_id(problem):
    """The git objects of a problem's inputs at `HEAD`, marked when the
    working tree changes them."""
    paths = input_paths(problem)
    objects = [git("rev-parse", f"HEAD:{path}") for path in paths]
    digest = hashlib.sha1("\n".join(objects).encode()).hexdigest()[:12]
    dirty = git("status", "--porcelain", "--", *paths)
    return digest + ("-dirty" if dirty else "")


# --------------------------------------------------------------------------
# Running


def bench_environment(package, threads):
    """The environment `lake env` gives the benchmark executable, so it runs
    without `lake` around it (whose time would count as the problem's)."""
    probe = "import json, os; print(json.dumps(dict(os.environ)))"
    output = subprocess.run(
        ["lake", "--dir", str(package), "env", sys.executable, "-c", probe],
        cwd=LEAN_DIR, capture_output=True, text=True,
    )
    if output.returncode != 0:
        fail(f"`lake env` failed in {package}:\n{output.stderr}")
    env = json.loads(output.stdout.strip().splitlines()[-1])
    env["LEAN_NUM_THREADS"] = str(threads)
    move_cli = os.environ.get("APTOS_MOVE_CLI") or str(REPO / "target" / "ci" / "move")
    if not Path(move_cli).is_file():
        fail(f"no Move CLI at {move_cli}; build it with `cargo build --locked --profile ci "
             "-p aptos-move-cli --features binary --bin move` or set APTOS_MOVE_CLI")
    env["APTOS_MOVE_CLI"] = move_cli
    return env


def command_of(problem, executable, out):
    kind = problem["kind"]
    if kind == "move":
        return [str(executable), kind, str(REPO / problem["package"]),
                "--modules", ",".join(problem["modules"]), "--out", str(out)]
    command = [str(executable), kind, str(REPO / problem["file"]), "--out", str(out)]
    if problem.get("spec"):
        command[3:3] = ["--spec", str(REPO / problem["spec"])]
    return command


def repository_relative(text):
    """A message with the files it names relative to the repository root, as
    the problems name them, so that runs on different checkouts read alike."""
    return text.replace(f"{REPO}/", "")


def attempt(problem, executable, env, workdir):
    """One run of a problem in its own process group, under its timeout;
    the result with the CPU time of the process tree added."""
    out = workdir / f"{problem['name']}.json"
    log = workdir / f"{problem['name']}.log"
    out.unlink(missing_ok=True)
    timeout = problem.get("timeout", DEFAULT_TIMEOUT)
    started = time.monotonic()
    with open(log, "w") as log_file:
        # The Rust exporter is found from the working directory.
        process = subprocess.Popen(
            command_of(problem, executable, out), cwd=LEAN_DIR, env=env,
            stdout=log_file, stderr=subprocess.STDOUT, start_new_session=True,
        )
        timed_out = False
        try:
            while True:
                pid, status, usage = os.wait4(process.pid, os.WNOHANG)
                if pid:
                    break
                if time.monotonic() - started > timeout:
                    os.killpg(process.pid, signal.SIGKILL)
                    _, status, usage = os.wait4(process.pid, 0)
                    timed_out = True
                    break
                time.sleep(0.2)
        except BaseException:
            # The driver is stopped: the problem's process group goes with it.
            os.killpg(process.pid, signal.SIGKILL)
            raise
        process.returncode = os.waitstatus_to_exitcode(status)
    wall_ms = round((time.monotonic() - started) * 1000)
    cpu_ms = round((usage.ru_utime + usage.ru_stime) * 1000)
    if timed_out:
        return {"status": "timeout", "wall_ms": {"total": wall_ms}, "cpu_ms": cpu_ms}
    if process.returncode != 0 or not out.exists():
        tail = log.read_text(errors="replace").splitlines()[-20:]
        return {"status": "crashed", "exit": process.returncode, "wall_ms": {"total": wall_ms},
                "cpu_ms": cpu_ms, "error_messages": [repository_relative(line) for line in tail]}
    result = json.loads(out.read_text())
    result["cpu_ms"] = cpu_ms
    result["error_messages"] = [repository_relative(message)
                                for message in result.get("error_messages", [])]
    return result


def measure(problem, executable, env, workdir):
    """A problem's result: the median of its repeats by wall time, with the
    spread of the repeats."""
    results = []
    for _ in range(problem.get("repeat", 1)):
        result = attempt(problem, executable, env, workdir)
        results.append(result)
        if "heartbeats" not in result:
            break
    finished = [result for result in results if "heartbeats" in result]
    if not finished:
        chosen = results[-1]
    else:
        ordered = sorted(finished, key=lambda result: result["wall_ms"]["total"])
        chosen = dict(ordered[(len(ordered) - 1) // 2])
        if len(finished) > 1:
            chosen["repeats"] = {
                "wall_ms": [result["wall_ms"]["total"] for result in finished],
                "cpu_ms": [result["cpu_ms"] for result in finished],
                "heartbeats": [result["heartbeats"]["total"] for result in finished],
            }
    return {"name": problem["name"], "kind": problem["kind"], "input": input_id(problem),
            **chosen}


def cpu_model():
    try:
        for line in Path("/proc/cpuinfo").read_text().splitlines():
            if line.startswith("model name"):
                return line.split(":", 1)[1].strip()
    except OSError:
        pass
    return platform.processor() or platform.machine()


def run(args):
    # A cancelled CI job terminates the driver; stop the running problem too.
    signal.signal(signal.SIGTERM, lambda *_: sys.exit(143))
    problems = load_manifest(args.manifest)
    if args.only:
        wanted = args.only.split(",")
        unknown = set(wanted) - {problem["name"] for problem in problems}
        if unknown:
            fail(f"no such problem: {', '.join(sorted(unknown))}")
        problems = [problem for problem in problems if problem["name"] in wanted]
    package = Path(args.package).resolve()
    # A run measures the executable, so it is built from the current sources
    # first; up to date, the build is a no-op.
    build = subprocess.run(["lake", "--dir", str(package), "build", "leaner-bench"],
                           cwd=LEAN_DIR, capture_output=True, text=True)
    if build.returncode != 0:
        fail(f"building leaner-bench in {package} failed:\n{build.stdout[-3000:]}"
             f"{build.stderr[-3000:]}")
    executable = package / ".lake" / "build" / "bin" / "leaner-bench"
    if in_ci() and not args.out:
        fail("a CI run names its results file with --out")
    env = bench_environment(package, args.threads)
    if args.out:
        workdir = Path(args.out).resolve().parent / (Path(args.out).stem + ".work")
    else:
        workdir = LOCAL_LOGS
        shutil.rmtree(workdir, ignore_errors=True)
    workdir.mkdir(parents=True, exist_ok=True)
    # Load the environment once untimed, so the first problem does not pay
    # for a cold file cache.
    warmup = subprocess.run([str(executable), "warmup"], cwd=LEAN_DIR, env=env,
                            capture_output=True, text=True)
    if warmup.returncode != 0:
        fail(f"the warm-up failed:\n{warmup.stderr}")
    measured = []
    for index, problem in enumerate(problems, 1):
        result = measure(problem, executable, env, workdir)
        measured.append(result)
        wall = result["wall_ms"]["total"] / 1000
        beats = result.get("heartbeats", {}).get("total")
        print(f"[{index}/{len(problems)}] {problem['name']}: {result['status']}, {wall:.1f} s"
              + (f", {beats / 1e6:.0f}M heartbeats" if beats is not None else ""),
              file=sys.stderr, flush=True)
    results = {
        "schema": SCHEMA,
        "version": VERSION,
        "commit": git("rev-parse", "HEAD"),
        "dirty": bool(git("status", "--porcelain")),
        "date": datetime.datetime.now(datetime.timezone.utc).isoformat(timespec="seconds"),
        "toolchain": (LEAN_DIR / "lean-toolchain").read_text().strip(),
        "runner": os.environ.get("RUNNER_NAME") or platform.node(),
        "cpu": cpu_model(),
        "threads": args.threads,
        "branch": current_branch(),
        "subset": bool(args.only),
        "problems": measured,
    }
    if args.out:
        Path(args.out).write_text(json.dumps(results, indent=2) + "\n")
        print(f"leaner-bench: wrote {args.out}", file=sys.stderr)
    # A local run joins the local history and renders the branch's local runs
    # against the CI history; in CI the report job renders.
    if not in_ci():
        record_local(results)
        report(parser().parse_args(["report", "--local-runs"]))
        if len(local_runs(results["branch"])) > 1:
            print()
            compare(parser().parse_args(["compare"]))


# --------------------------------------------------------------------------
# Local history


def local_runs(branch=None):
    """The recorded local runs, oldest first; those of `branch` when given."""
    if not LOCAL_HISTORY.exists():
        return []
    runs = [json.loads(line) for line in LOCAL_HISTORY.read_text().splitlines() if line.strip()]
    return [run for run in runs
            if run.get("schema") == SCHEMA and run.get("version") == VERSION
            and (branch is None or run.get("branch") == branch)]


def record_local(results):
    """Append a local run to the history, which keeps the latest runs."""
    runs = local_runs() + [results]
    LOCAL_HISTORY.write_text("".join(json.dumps(run) + "\n" for run in runs[-LOCAL_KEEP:]))
    print(f"leaner-bench: recorded in {LOCAL_HISTORY} "
          f"(@-1 of {len(local_runs(results['branch']))} on {results['branch']})", file=sys.stderr)


def run_at(reference):
    """A run named by a results file, or by `@N`: the N-th local run of the
    current branch, negative indices counting back from the latest."""
    if not reference.startswith("@"):
        return json.loads(Path(reference).read_text())
    branch = current_branch()
    runs = local_runs(branch)
    try:
        return runs[int(reference[1:])]
    except (ValueError, IndexError):
        fail(f"no local run {reference} on {branch}: {len(runs)} recorded")


# --------------------------------------------------------------------------
# History


def repo_args(args):
    return ["--repo", args.repo]


def cache_dir():
    return Path(os.environ.get("LEANER_BENCH_CACHE")
                or Path.home() / ".cache" / "leaner-bench")


def fetch_history(args):
    """The results of the last `window` CI runs on the branch that produced
    results, oldest first, from a cache keyed by run id. A run counts by its
    results artifact, not its conclusion: a run whose report or publication
    failed after the benchmark finished still measured."""
    listing = subprocess.run(
        ["gh", "run", "list", "--workflow", WORKFLOW, "--branch", args.branch,
         "--status", "completed", "--limit", str(min(3 * args.window, 300)),
         "--json", "databaseId,headSha,createdAt,url", *repo_args(args)],
        capture_output=True, text=True,
    )
    if listing.returncode != 0:
        print(f"leaner-bench: no history: {listing.stderr.strip()}", file=sys.stderr)
        return []
    history = []
    for entry in json.loads(listing.stdout):
        if len(history) == args.window:
            break
        target = cache_dir() / str(entry["databaseId"])
        results = target / "results.json"
        if not results.exists():
            download = subprocess.run(
                ["gh", "run", "download", str(entry["databaseId"]), "--name", ARTIFACT,
                 "--dir", str(target), *repo_args(args)],
                capture_output=True, text=True,
            )
            if download.returncode != 0 or not results.exists():
                continue
        data = json.loads(results.read_text())
        if data.get("schema") != SCHEMA or data.get("version") != VERSION:
            continue
        data["run"] = {"id": str(entry["databaseId"]), "url": entry["url"]}
        history.append(data)
    history.sort(key=lambda data: data["date"])
    return history


def history(args):
    if args.local:
        runs = local_runs(current_branch())
        for index, data in enumerate(runs):
            verified = sum(problem["status"] == "verified" for problem in data["problems"])
            suite = suite_value(data, "wall_ms")
            print(f"@{index - len(runs)}  {label_of(data)}  "
                  f"{verified}/{len(data['problems'])} verified  "
                  + ("subset" if data.get("subset") else f"suite {seconds(suite)} s"))
        return
    for data in fetch_history(args):
        verified = sum(problem["status"] == "verified" for problem in data["problems"])
        print(f"{data['run']['id']}  {data['date']}  {data['commit'][:10]}  "
              f"{verified}/{len(data['problems'])} verified")


# --------------------------------------------------------------------------
# Values


def group_values(result, measure):
    """Elaboration, verification, and overall of a result in `measure`
    (`wall_ms` or `heartbeats`), or `None` where it has none."""
    phases = result.get(measure) if result else None
    if not phases or "verification" not in phases:
        return {name: None for name, _ in GROUPS} | (
            {"overall": phases.get("total")} if phases else {})
    return {"elaboration": phases["lowering"] + phases["certification"],
            "verification": phases["verification"], "overall": phases["total"]}


def problem_names(points):
    names = []
    for point in reversed(points):
        for problem in point["problems"]:
            if problem["name"] not in names:
                names.append(problem["name"])
    return names


def result_of(point, name):
    return next((problem for problem in point["problems"] if problem["name"] == name), None)


def suite_value(point, measure):
    """The overall total of the problems a run verified completely; none for
    a run of a subset, whose total compares with no other."""
    if point.get("subset"):
        return None
    return sum(problem[measure]["total"] for problem in point["problems"]
               if problem["status"] == "verified")


def median(values):
    present = [value for value in values if value is not None]
    return statistics.median(present) if present else None


def change(new, old):
    if new is None or old in (None, 0):
        return None
    return (new - old) / old * 100


def percent(value):
    return "–" if value is None else f"{value:+.1f} %"


def seconds(ms):
    return "–" if ms is None else f"{ms / 1000:.1f}"


def millions(beats):
    return "–" if beats is None else f"{beats / 1e6:,.0f}M"


def sparkline(values):
    present = [value for value in values if value is not None]
    if not present:
        return ""
    low, high = min(present), max(present)
    span = (high - low) or 1
    return "".join(" " if value is None else SPARKS[round((value - low) / span * 7)]
                   for value in values)


def label_of(point):
    if point.get("run"):
        return f"{point['date'][:10]} {point['commit'][:7]}"
    return (f"local {point['date'][5:16].replace('T', ' ')} {point['commit'][:7]}"
            + ("+" if point.get("dirty") else ""))


def annotations(points, name):
    """What changed at each point since the previous one with a result."""
    notes = {}
    previous = None
    for index, point in enumerate(points):
        result = result_of(point, name)
        if result is None:
            continue
        if previous is not None:
            before, before_point = previous
            changed = []
            if result.get("input") != before.get("input"):
                changed.append("input")
            if point.get("toolchain") != before_point.get("toolchain"):
                changed.append("toolchain")
            if result["status"] != before["status"]:
                changed.append(f"{before['status']} → {result['status']}")
            if changed:
                notes[index] = changed
        previous = (result, point)
    return notes


# --------------------------------------------------------------------------
# Markdown and Slack


def run_url():
    if os.environ.get("GITHUB_RUN_ID"):
        return (f"{os.environ['GITHUB_SERVER_URL']}/{os.environ['GITHUB_REPOSITORY']}"
                f"/actions/runs/{os.environ['GITHUB_RUN_ID']}")
    return None


def latest_changes(points, name):
    """Of a problem's latest result: its values and the change of its overall
    wall time against the previous result and the window's median, and of its
    overall heartbeats against the previous result."""
    results = [result_of(point, name) for point in points]
    latest = results[-1]
    earlier = [result for result in results[:-1] if result and result["status"] == "verified"]
    walls = [result["wall_ms"]["total"] for result in earlier]
    wall = latest["wall_ms"]["total"] if latest else None
    beats = latest.get("heartbeats", {}).get("total") if latest else None
    previous = earlier[-1] if earlier else None
    return {
        "latest": latest,
        "previous": change(wall, previous["wall_ms"]["total"] if previous else None),
        "median": change(wall, statistics.median(walls) if walls else None),
        "heartbeats": change(beats, previous["heartbeats"]["total"] if previous else None),
        "trend": sparkline([result["wall_ms"]["total"] if result and result["status"] == "verified"
                            else None for result in results]),
    }


def markdown(points, threshold, page_url=None):
    latest = points[-1]
    names = problem_names(points)
    suite = [suite_value(point, "wall_ms") for point in points]
    lines = [f"### Leaner verification benchmark: {label_of(latest)}", ""]
    if page_url:
        lines += [f"Charts: {page_url}", ""]
    counts = {status: sum(problem["status"] == status for problem in latest["problems"])
              for status in ("verified", "failed", "timeout", "crashed")}
    lines.append(
        f"Suite overall **{seconds(suite[-1])} s** ({percent(change(suite[-1], suite[-2] if len(suite) > 1 else None))} "
        f"vs previous, {percent(change(suite[-1], median(suite[:-1])))} "
        f"vs median of {len(suite) - 1} runs); "
        + ", ".join(f"{count} {status}" for status, count in counts.items() if count)
        + f". Runner `{latest.get('runner')}`, {latest.get('threads')} threads.")
    lines += ["", "| Problem | Status | Elaboration s | Verification s | Overall s | "
              "Δ previous | Δ median | Δ heartbeats | Overall, window |",
              "|---|---|---:|---:|---:|---:|---:|---:|---|"]
    for name in names:
        changes = latest_changes(points, name)
        result = changes["latest"]
        values = group_values(result, "wall_ms")
        status = result["status"] if result else "absent"
        flag = " ⚠" if changes["median"] is not None and abs(changes["median"]) >= threshold else ""
        lines.append(
            f"| `{name}` | {status} | {seconds(values['elaboration'])} | "
            f"{seconds(values['verification'])} | {seconds(values['overall'])} | "
            f"{percent(changes['previous'])} | {percent(changes['median'])}{flag} | "
            f"{percent(changes['heartbeats'])} | `{changes['trend']}` |")
    return "\n".join(lines) + "\n"


def slack(points, threshold, heartbeat_threshold, page_url=None):
    latest = points[-1]
    suite = [suite_value(point, "wall_ms") for point in points]
    counts = {status: sum(problem["status"] == status for problem in latest["problems"])
              for status in ("verified", "failed", "timeout", "crashed")}
    lines = [
        f"*Leaner verification benchmark* · `{latest['commit'][:10]}` · {latest['date'][:10]}",
        f"Suite overall *{seconds(suite[-1])} s* "
        f"({percent(change(suite[-1], suite[-2] if len(suite) > 1 else None))} vs previous, "
        f"{percent(change(suite[-1], median(suite[:-1])))} "
        f"vs {len(suite) - 1}-run median) · "
        + ", ".join(f"{count} {status}" for status, count in counts.items() if count),
    ]
    notable = []
    for name in problem_names(points):
        changes = latest_changes(points, name)
        notes = annotations(points, name).get(len(points) - 1, [])
        moved = changes["median"] is not None and abs(changes["median"]) >= threshold
        work = changes["heartbeats"] is not None and abs(changes["heartbeats"]) >= heartbeat_threshold
        if moved or work or notes:
            result = changes["latest"]
            detail = [f"{seconds(result['wall_ms']['total'])} s" if result else "absent"]
            if moved:
                detail.append(f"{percent(changes['median'])} vs median")
            if work:
                detail.append(f"heartbeats {percent(changes['heartbeats'])}")
            detail += notes
            notable.append(f"• `{name}` {', '.join(detail)} `{changes['trend']}`")
    lines += (["Changes:"] + notable) if notable else ["No problem moved beyond the thresholds."]
    links = [f"<{page_url}|Charts>"] if page_url else []
    if run_url():
        links.append(f"<{run_url()}|Run>")
    if links:
        lines.append(" · ".join(links))
    return {"text": "\n".join(lines)}


# --------------------------------------------------------------------------
# HTML

STYLE = """
.viz-root { color-scheme: light;
  --surface-1: #fcfcfb; --page: #f9f9f7; --text-primary: #0b0b0b; --text-secondary: #52514e;
  --muted: #898781; --grid: #e1e0d9; --axis: #c3c2b7; --border: rgba(11,11,11,0.10);
  --series-1: #2a78d6; --series-2: #eb6834; --series-3: #1baf7a; }
@media (prefers-color-scheme: dark) { :root:where(:not([data-theme="light"])) .viz-root {
  color-scheme: dark; --surface-1: #1a1a19; --page: #0d0d0d; --text-primary: #ffffff;
  --text-secondary: #c3c2b7; --grid: #2c2c2a; --axis: #383835; --border: rgba(255,255,255,0.10);
  --series-1: #3987e5; --series-2: #d95926; --series-3: #199e70; } }
:root[data-theme="dark"] .viz-root { color-scheme: dark; --surface-1: #1a1a19; --page: #0d0d0d;
  --text-primary: #ffffff; --text-secondary: #c3c2b7; --grid: #2c2c2a; --axis: #383835;
  --border: rgba(255,255,255,0.10); --series-1: #3987e5; --series-2: #d95926; --series-3: #199e70; }
body { margin: 0; background: var(--page); }
.viz-root { background: var(--page); color: var(--text-primary); padding: 16px;
  font: 14px system-ui, -apple-system, "Segoe UI", sans-serif; max-width: 1400px; margin: 0 auto; }
h1 { font-size: 20px; margin: 0 0 4px; } h2 { font-size: 16px; margin: 0 0 8px; }
.meta { color: var(--text-secondary); margin-bottom: 16px; }
section { background: var(--surface-1); border: 1px solid var(--border); border-radius: 8px;
  padding: 12px 16px; margin-bottom: 16px; }
.charts { display: flex; flex-wrap: wrap; gap: 16px; }
.chart { position: relative; flex: 1 1 420px; min-width: 0; }
.chart svg { width: 100%; height: auto; display: block; }
.legend { display: flex; gap: 16px; color: var(--text-secondary); font-size: 12px; margin: 4px 0; }
.legend span::before { content: ""; display: inline-block; width: 14px; height: 2px;
  margin-right: 6px; vertical-align: middle; background: var(--swatch); }
.status { font-size: 12px; color: var(--text-secondary); margin-left: 8px; }
table { border-collapse: collapse; font-size: 12px; font-variant-numeric: tabular-nums; }
th, td { padding: 2px 10px 2px 0; text-align: right; } th:first-child, td:first-child { text-align: left; }
th { color: var(--text-secondary); font-weight: 500; }
details { margin-top: 8px; color: var(--text-secondary); }
.tooltip { position: absolute; pointer-events: none; background: var(--surface-1);
  border: 1px solid var(--border); border-radius: 6px; padding: 6px 8px; font-size: 12px;
  box-shadow: 0 2px 8px rgba(0,0,0,0.15); display: none; white-space: nowrap; }
.tooltip strong { font-weight: 600; }
"""

SCRIPT = """
document.querySelectorAll('.chart').forEach(chart => {
  const data = JSON.parse(chart.dataset.points);
  const svg = chart.querySelector('svg');
  const cross = svg.querySelector('.cross');
  const tip = chart.querySelector('.tooltip');
  const box = () => svg.viewBox.baseVal;
  svg.addEventListener('pointermove', event => {
    const rect = svg.getBoundingClientRect();
    const x = (event.clientX - rect.left) / rect.width * box().width;
    let best = 0;
    data.xs.forEach((px, i) => { if (Math.abs(px - x) < Math.abs(data.xs[best] - x)) best = i; });
    cross.setAttribute('x1', data.xs[best]); cross.setAttribute('x2', data.xs[best]);
    cross.style.display = '';
    tip.replaceChildren();
    const head = document.createElement('div'); head.textContent = data.labels[best]; tip.append(head);
    data.series.forEach(series => {
      const row = document.createElement('div');
      const value = document.createElement('strong');
      const v = series.values[best];
      value.textContent = v === null ? '–'
        : (v >= 100 ? Math.round(v).toLocaleString() : v.toFixed(v >= 1 ? 1 : 2)) + ' ' + data.unit;
      row.append(value, document.createTextNode(' ' + series.name));
      tip.append(row);
    });
    if (data.notes[best]) { const note = document.createElement('div');
      note.textContent = 'changed: ' + data.notes[best]; tip.append(note); }
    tip.style.display = 'block';
    const left = data.xs[best] / box().width * rect.width;
    tip.style.left = Math.min(left + 12, rect.width - tip.offsetWidth) + 'px';
    tip.style.top = '8px';
  });
  svg.addEventListener('pointerleave', () => { cross.style.display = 'none'; tip.style.display = 'none'; });
});
"""


def short(value):
    """A chart value: whole above 100, one decimal above 1, two below."""
    return f"{value:,.0f}" if value >= 100 else f"{value:.1f}" if value >= 1 else f"{value:.2f}"


def axis(maximum):
    """At most four gridline steps of 1, 2, 2.5, or 5 times a power of ten
    covering `maximum`: the axis maximum, the step, and the decimals the
    labels need."""
    target = maximum / 4 if maximum > 0 else 0.25
    magnitude = 10 ** math.floor(math.log10(target))
    step = next(factor * magnitude for factor in (1, 2, 2.5, 5, 10)
                if factor * magnitude >= target * (1 - 1e-9))
    decimals = max(0, math.ceil(-math.log10(step) - 1e-9))
    if abs(step * 10 ** decimals - round(step * 10 ** decimals)) > 1e-9:
        decimals += 1
    steps = max(1, math.ceil(maximum / step - 1e-9))
    return step * steps, step, decimals


def chart(title, unit, labels, series, notes, band=None):
    """A line chart of up to three series over the window: gridlines, the
    series with markers and direct labels at their ends, change markers,
    and a crosshair the page script drives. A series is named by its group,
    whose slot gives its color on every chart."""
    width, height = 640, 220
    left, right, top, bottom = 52, 128, 12, 24
    plot_w, plot_h = width - left - right, height - top - bottom
    present = [value for _, values in series for value in values if value is not None]
    if band:
        present += [high for _, high in band if high is not None]
    y_max, y_step, tick_digits = axis(max(present) if present else 1)
    count = len(labels)
    xs = [left + (plot_w * index / (count - 1) if count > 1 else plot_w / 2)
          for index in range(count)]

    def y_of(value):
        return top + plot_h - value / y_max * plot_h

    parts = [f'<svg viewBox="0 0 {width} {height}" role="img" aria-label="{html.escape(title)}">']
    for tick in range(round(y_max / y_step) + 1):
        value = y_step * tick
        y = y_of(value)
        stroke = "var(--axis)" if tick == 0 else "var(--grid)"
        parts.append(f'<line x1="{left}" x2="{left + plot_w}" y1="{y:.1f}" y2="{y:.1f}" '
                     f'stroke="{stroke}" stroke-width="1"/>')
        parts.append(f'<text x="{left - 6}" y="{y + 4:.1f}" text-anchor="end" font-size="11" '
                     f'fill="var(--muted)">{value:,.{tick_digits}f}</text>')
    for index, changed in notes.items():
        parts.append(f'<line x1="{xs[index]:.1f}" x2="{xs[index]:.1f}" y1="{top}" '
                     f'y2="{top + plot_h}" stroke="var(--muted)" stroke-dasharray="3 3"/>')
        parts.append(f'<text x="{xs[index] + 3:.1f}" y="{top + 10}" font-size="10" '
                     f'fill="var(--muted)">{html.escape(", ".join(changed))}</text>')
    if band:
        upper = [(xs[i], y_of(high)) for i, (low, high) in enumerate(band) if high is not None]
        lower = [(xs[i], y_of(low)) for i, (low, high) in enumerate(band) if low is not None]
        if len(upper) > 1:
            points = " ".join(f"{x:.1f},{y:.1f}" for x, y in upper + lower[::-1])
            parts.append(f'<polygon points="{points}" fill="var(--series-{slot_of("Overall")})" '
                         'opacity="0.15"/>')
    labels_at = []
    for name, values in series:
        color = f"var(--series-{slot_of(name)})"
        segment = []
        for index, value in enumerate(values + [None]):
            if value is not None:
                segment.append((xs[index], y_of(value)))
                continue
            if len(segment) > 1:
                points = " ".join(f"{x:.1f},{y:.1f}" for x, y in segment)
                parts.append(f'<polyline points="{points}" fill="none" stroke="{color}" '
                             f'stroke-width="2" stroke-linejoin="round"/>')
            segment = []
        for index, value in enumerate(values):
            if value is not None:
                parts.append(f'<circle cx="{xs[index]:.1f}" cy="{y_of(value):.1f}" r="4" '
                             f'fill="{color}" stroke="var(--surface-1)" stroke-width="2"/>')
        last = max((index for index, value in enumerate(values) if value is not None), default=None)
        if last is not None:
            labels_at.append([y_of(values[last]), color, f"{name} {short(values[last])}"])
    # Direct labels at the line ends, spread apart and kept beside the plot.
    labels_at.sort(key=lambda label: label[0])
    for index in range(1, len(labels_at)):
        labels_at[index][0] = max(labels_at[index][0], labels_at[index - 1][0] + 14)
    overflow = labels_at[-1][0] - (top + plot_h) if labels_at else 0
    for label in labels_at:
        label[0] -= max(0, overflow)
    for y, color, text in labels_at:
        parts.append(f'<line x1="{left + plot_w + 8}" x2="{left + plot_w + 20}" y1="{y:.1f}" '
                     f'y2="{y:.1f}" stroke="{color}" stroke-width="2"/>')
        parts.append(f'<text x="{left + plot_w + 24}" y="{y + 4:.1f}" font-size="11" '
                     f'fill="var(--text-secondary)">{html.escape(text)}</text>')
    parts.append(f'<line class="cross" x1="0" x2="0" y1="{top}" y2="{top + plot_h}" '
                 f'stroke="var(--muted)" stroke-width="1" style="display:none"/>')
    if count == 1:
        parts.append(f'<text x="{xs[0]:.1f}" y="{height - 6}" font-size="11" '
                     f'text-anchor="middle" fill="var(--muted)">{html.escape(labels[0])}</text>')
    else:
        parts.append(f'<text x="{left}" y="{height - 6}" font-size="11" fill="var(--muted)">'
                     f'{html.escape(labels[0])}</text>')
        parts.append(f'<text x="{left + plot_w}" y="{height - 6}" font-size="11" '
                     f'text-anchor="end" fill="var(--muted)">{html.escape(labels[-1])}</text>')
    parts.append("</svg>")
    data = {"xs": xs, "labels": labels, "unit": unit,
            "notes": {index: ", ".join(changed) for index, changed in notes.items()},
            "series": [{"name": name, "values": values} for name, values in series]}
    # A single series is named by its chart's heading.
    legend = "".join(f'<span style="--swatch: var(--series-{slot_of(name)})">{html.escape(name)}</span>'
                     for name, _ in series) if len(series) > 1 else ""
    return (f'<div class="chart" data-points="{html.escape(json.dumps(data))}">'
            f'<div class="legend">{legend}</div>{"".join(parts)}'
            f'<div class="tooltip"></div></div>')


def slot_of(name):
    return 1 + [title for _, title in GROUPS].index(name)


def value_table(labels, rows):
    head = "".join(f"<th>{html.escape(label)}</th>" for label in ["Run", *[name for name, _ in rows]])
    body = "".join(
        "<tr>" + f"<td>{html.escape(label)}</td>"
        + "".join(f"<td>{cells[index]}</td>" for _, cells in rows) + "</tr>"
        for index, label in enumerate(labels))
    return f"<details><summary>Table</summary><table><tr>{head}</tr>{body}</table></details>"


def problem_section(points, name):
    labels = [label_of(point) for point in points]
    results = [result_of(point, name) for point in points]
    notes = annotations(points, name)
    walls = [group_values(result, "wall_ms") for result in results]
    beats = [group_values(result, "heartbeats") for result in results]
    wall_series = [(title, [None if value[key] is None else value[key] / 1000 for value in walls])
                   for key, title in GROUPS]
    beat_series = [(title, [None if value[key] is None else value[key] / 1e6 for value in beats])
                   for key, title in GROUPS]
    band = [(min(result["repeats"]["wall_ms"]) / 1000, max(result["repeats"]["wall_ms"]) / 1000)
            if result and result.get("repeats") else (None, None) for result in results]
    latest = results[-1]
    status = latest["status"] if latest else "absent"
    targets = sorted((latest or {}).get("targets", []), key=lambda t: -t["wall_ms"])[:8]
    target_rows = "".join(
        f"<tr><td>{html.escape(target['target'])}</td><td>{target['wall_ms'] / 1000:.1f}</td>"
        f"<td>{target['heartbeats'] / 1e6:,.0f}</td></tr>" for target in targets)
    errors = "".join(f"<li>{html.escape(message)}</li>"
                     for message in (latest or {}).get("error_messages", []))
    rows = [(f"{title} s", [seconds(value[key]) for value in walls]) for key, title in GROUPS]
    rows += [("Heartbeats", [millions(value["overall"]) for value in beats]),
             ("Status", [result["status"] if result else "–" for result in results])]
    return (
        f'<section><h2>{html.escape(name)}<span class="status">{html.escape(status)}</span></h2>'
        f'<div class="charts">'
        f'{chart(f"{name}: seconds", "s", labels, wall_series, notes, band if any(b[1] for b in band) else None)}'
        f'{chart(f"{name}: heartbeats", "M", labels, beat_series, notes)}</div>'
        + (f"<details open><summary>Most expensive targets, latest run</summary><table>"
           f"<tr><th>Target</th><th>Seconds</th><th>Heartbeats (M)</th></tr>{target_rows}"
           f"</table></details>" if targets else "")
        + (f"<details open><summary>Errors, latest run</summary><ul>{errors}</ul></details>"
           if errors else "")
        + value_table(labels, rows) + "</section>")


def suite_section(points):
    labels = [label_of(point) for point in points]
    def scaled(measure, unit):
        return [None if value is None else value / unit
                for value in (suite_value(point, measure) for point in points)]
    wall = scaled("wall_ms", 1000)
    beats = scaled("heartbeats", 1e6)
    if all(value is None for value in wall):
        return ('<section><h2>Suite</h2><p class="meta">No full run in this window: the '
                "suite total compares full runs only.</p></section>")
    return (
        '<section><h2>Suite<span class="status">problems verified completely, '
        'full runs</span></h2>'
        '<div class="charts">'
        f'{chart("Suite: seconds", "s", labels, [("Overall", wall)], {})}'
        f'{chart("Suite: heartbeats", "M", labels, [("Overall", beats)], {})}'
        "</div></section>")


def page(points, local_table):
    latest = points[-1]
    sections = [suite_section(points)] + [problem_section(points, name)
                                          for name in problem_names(points)]
    return (
        "<!doctype html><html><head><meta charset=\"utf-8\">"
        "<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">"
        f"<title>Leaner benchmark</title><style>{STYLE}</style></head><body>"
        '<div class="viz-root"><h1>Leaner verification benchmark</h1>'
        f'<div class="meta">{len(points)} runs, latest {html.escape(label_of(latest))}, '
        f'runner {html.escape(str(latest.get("runner")))}, {latest.get("threads")} threads, '
        f'{html.escape(latest.get("toolchain", ""))}</div>'
        + local_table + "".join(sections)
        + f"</div><script>{SCRIPT}</script></body></html>\n")


# --------------------------------------------------------------------------
# Local comparison


def comparison_base(history, base_ref):
    """The latest CI run at or before the merge base of `HEAD` with
    `base_ref`, so that what landed since does not count as the local
    change."""
    try:
        merge_base = git("merge-base", "HEAD", base_ref)
    except subprocess.CalledProcessError:
        return None
    for point in reversed(history):
        is_ancestor = subprocess.run(
            ["git", "merge-base", "--is-ancestor", point["commit"], merge_base], cwd=REPO)
        if is_ancestor.returncode == 0:
            return point
    return None


def local_comparison(base, local):
    """The local run against a CI run: heartbeats directly, wall time as each
    problem's share of the suite, since the machines differ."""
    names = [name for name in problem_names([local])
             if (result_of(base, name) or {}).get("status") == "verified"
             and (result_of(local, name) or {}).get("status") == "verified"]
    base_total = sum(result_of(base, name)["wall_ms"]["total"] for name in names) or 1
    local_total = sum(result_of(local, name)["wall_ms"]["total"] for name in names) or 1
    rows = []
    for name in names:
        before, after = result_of(base, name), result_of(local, name)
        share_before = before["wall_ms"]["total"] / base_total
        share_after = after["wall_ms"]["total"] / local_total
        rows.append((name, before["heartbeats"]["total"], after["heartbeats"]["total"],
                     share_before, share_after))
    return rows


def local_text(base, rows):
    lines = [f"Local run against CI run {base['run']['id']} ({base['commit'][:10]}, "
             f"{base['date'][:10]})", "",
             f"{'problem':<24} {'heartbeats CI':>14} {'local':>10} {'change':>9}"
             f" {'time share CI':>14} {'local':>7}"]
    for name, beats_before, beats_after, share_before, share_after in rows:
        lines.append(f"{name:<24} {millions(beats_before):>14} {millions(beats_after):>10} "
                     f"{percent(change(beats_after, beats_before)):>9} "
                     f"{share_before * 100:>13.1f}% {share_after * 100:>6.1f}%")
    return "\n".join(lines)


def local_html(base, rows):
    if base is None:
        return ""
    body = "".join(
        f"<tr><td>{html.escape(name)}</td><td>{millions(before)}</td><td>{millions(after)}</td>"
        f"<td>{percent(change(after, before))}</td><td>{share_before * 100:.1f} %</td>"
        f"<td>{share_after * 100:.1f} %</td></tr>"
        for name, before, after, share_before, share_after in rows)
    return (f"<section><h2>Local run against CI run {base['run']['id']}"
            f'<span class="status">{html.escape(base["commit"][:10])}, at or before the merge '
            "base</span></h2><table><tr><th>Problem</th><th>Heartbeats CI</th><th>Local</th>"
            "<th>Change</th><th>Time share CI</th><th>Local</th></tr>"
            f"{body}</table></section>")


def report(args):
    points = [] if args.no_history else fetch_history(args)
    if args.current:
        # This CI run's results, which the history lists only once the run
        # has completed.
        current = json.loads(Path(args.current).read_text())
        current["run"] = {"id": os.environ.get("GITHUB_RUN_ID", "current"), "url": run_url()}
        points = [point for point in points if point["run"]["id"] != current["run"]["id"]]
        points.append(current)
    locals_ = [json.loads(Path(args.local).read_text())] if args.local else []
    if args.local_runs:
        locals_ += local_runs(current_branch())
    base, rows = None, []
    if locals_:
        base = comparison_base(points, args.base_ref) if points else None
        if base:
            rows = local_comparison(base, locals_[-1])
        points = points + locals_
    if not points:
        fail("nothing to report: no history and no local run")
    html_path = args.html or (LOCAL_PAGE if locals_ else None)
    if html_path:
        Path(html_path).write_text(page(points, local_html(base, rows)))
        print(f"leaner-bench: wrote {html_path}", file=sys.stderr)
    if args.markdown:
        Path(args.markdown).write_text(markdown(points, args.threshold, args.page_url))
    if args.slack:
        Path(args.slack).write_text(json.dumps(
            slack(points, args.threshold, args.heartbeat_threshold, args.page_url)) + "\n")
    if not (html_path or args.markdown or args.slack):
        print(markdown(points, args.threshold))
    if base:
        print(local_text(base, rows))


def compare(args):
    before, after = run_at(args.before), run_at(args.after)
    print(f"before: {label_of(before)}\nafter:  {label_of(after)}\n")
    names = problem_names([before, after])
    print(f"{'problem':<24} {'status':>20} {'seconds':>16} {'change':>9} "
          f"{'heartbeats':>16} {'change':>9}")
    common = []
    for name in names:
        old, new = result_of(before, name), result_of(after, name)
        status = f"{(old or {}).get('status', '–')} → {(new or {}).get('status', '–')}"
        wall = [result["wall_ms"]["total"] if result else None for result in (old, new)]
        beats = [result.get("heartbeats", {}).get("total") if result else None
                 for result in (old, new)]
        if old and new and old["status"] == new["status"] == "verified":
            common.append((wall, beats))
        print(f"{name:<24} {status:>20} {seconds(wall[0]):>7} → {seconds(wall[1]):>6} "
              f"{percent(change(wall[1], wall[0])):>9} {millions(beats[0]):>7} → "
              f"{millions(beats[1]):>6} {percent(change(beats[1], beats[0])):>9}")
    # The problems both runs verified completely, so subsets compare too.
    wall = [sum(pair[0][side] for pair in common) for side in (0, 1)]
    beats = [sum(pair[1][side] for pair in common) for side in (0, 1)]
    print(f"{f'verified in both ({len(common)})':<24} {'':>20} {seconds(wall[0]):>7} → "
          f"{seconds(wall[1]):>6} {percent(change(wall[1], wall[0])):>9} "
          f"{millions(beats[0]):>7} → {millions(beats[1]):>6} "
          f"{percent(change(beats[1], beats[0])):>9}")


def parser():
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    commands = parser.add_subparsers(dest="command", required=True)

    run_parser = commands.add_parser("run", help="measure the problems")
    run_parser.add_argument("--out", help="a results file to write; a local run is also "
                            f"recorded in {LOCAL_HISTORY.name}")
    run_parser.add_argument("--only", help="comma-separated problem names")
    run_parser.add_argument("--manifest", default=MANIFEST)
    run_parser.add_argument("--package", default=PACKAGE,
                            help="the Lake package that built leaner-bench")
    run_parser.add_argument("--threads", type=int,
                            default=int(os.environ.get("LEAN_NUM_THREADS", DEFAULT_THREADS)))
    run_parser.set_defaults(action=run)

    def history_options(sub):
        sub.add_argument("--window", type=int, default=30, help="number of recent CI runs")
        sub.add_argument("--branch", default="main")
        sub.add_argument("--repo", default=os.environ.get("GITHUB_REPOSITORY",
                                                          "aptos-labs/aptos-core"))

    history_parser = commands.add_parser("history", help="fetch the results of recent CI runs")
    history_options(history_parser)
    history_parser.add_argument("--local", action="store_true",
                                help="list the local runs of the current branch instead")
    history_parser.set_defaults(action=history)

    report_parser = commands.add_parser("report", help="render the history")
    history_options(report_parser)
    report_parser.add_argument("--current", help="the results of the running CI run")
    report_parser.add_argument("--local", help="a local results file to append")
    report_parser.add_argument("--local-runs", action="store_true",
                               help="append the recorded local runs of the current branch")
    report_parser.add_argument("--no-history", action="store_true",
                               help="report the local run alone")
    report_parser.add_argument("--base-ref", default="upstream/main",
                               help="the ref whose merge base picks the CI run to compare with")
    report_parser.add_argument("--html",
                               help=f"the page to write; with --local, {LOCAL_PAGE.name} "
                                    "in the Lean tree by default")
    report_parser.add_argument("--markdown")
    report_parser.add_argument("--slack")
    report_parser.add_argument("--page-url", help="where the HTML report is published")
    report_parser.add_argument("--threshold", type=float, default=10.0,
                               help="wall-time change against the median worth flagging, in percent")
    report_parser.add_argument("--heartbeat-threshold", type=float, default=5.0,
                               help="heartbeat change worth flagging, in percent")
    report_parser.set_defaults(action=report)

    compare_parser = commands.add_parser(
        "compare", help="compare two runs of one machine: results files, or @N for the "
                        "N-th local run of the current branch (@-1 the latest)")
    compare_parser.add_argument("before", nargs="?", default="@-2")
    compare_parser.add_argument("after", nargs="?", default="@-1")
    compare_parser.set_defaults(action=compare)
    return parser


def main():
    args = parser().parse_args()
    args.action(args)


if __name__ == "__main__":
    main()
