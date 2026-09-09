#!/usr/bin/env python3
"""Record CI phase measurements and compare nextest test inventories."""

import argparse
import json
import os
import resource
import subprocess
import sys
import time
from collections import Counter
from datetime import datetime, timezone
from pathlib import Path


def inventory(listing):
    """Keep test identities and ignore status, not machine-specific paths."""
    return sorted(
        (binary, name, case["ignored"])
        for binary, suite in listing["rust-suites"].items()
        for name, case in suite["testcases"].items()
        if case["filter-match"]["status"] == "matches"
    )


def check_partitions(expected, partitions):
    expected = Counter(map(tuple, expected))
    actual = Counter(test for partition in partitions for test in map(tuple, partition))
    if expected != actual:
        raise ValueError(
            f"Partition mismatch: missing={list((expected - actual).elements())[:10]}, "
            f"extra={list((actual - expected).elements())[:10]}"
        )


def execution_mode(count, threshold):
    if type(count) is not int or count < 0 or threshold < 0:
        raise ValueError("Test count and threshold must be nonnegative integers")
    return "inline" if count <= threshold else "shard"


def cgroup_peak(root=Path("/sys/fs/cgroup")):
    for relative in ("memory.peak", "memory/memory.max_usage_in_bytes"):
        path = root / relative
        try:
            value = path.read_text().strip()
        except OSError:
            continue
        if value.isdecimal():
            return {"cgroup_peak_bytes": int(value), "cgroup_peak_source": str(path)}
    return {}


def host_memory_used(path=Path("/proc/meminfo")):
    try:
        fields = {line.split()[0]: int(line.split()[1]) for line in path.read_text().splitlines()}
        return (fields["MemTotal:"] - fields["MemAvailable:"]) * 1024
    except (OSError, KeyError, ValueError):
        return None


def measure(phase, command, output_dir):
    output_dir.mkdir(parents=True, exist_ok=True)
    before = resource.getrusage(resource.RUSAGE_CHILDREN)
    started_at = datetime.now(timezone.utc).isoformat()
    start = time.monotonic()
    sampled_memory = []
    try:
        with subprocess.Popen(command) as process:
            while True:
                used = host_memory_used()
                if used is not None:
                    sampled_memory.append(used)
                try:
                    result = process.wait(timeout=1)
                    break
                except subprocess.TimeoutExpired:
                    pass
    except OSError as error:
        print(error, file=sys.stderr)
        result = 127
    after = resource.getrusage(resource.RUSAGE_CHILDREN)
    row = {
        "phase": phase,
        "started_at": started_at,
        "elapsed_seconds": time.monotonic() - start,
        "user_cpu_seconds": after.ru_utime - before.ru_utime,
        "system_cpu_seconds": after.ru_stime - before.ru_stime,
        # This is the largest child process, not aggregate parallel-job memory.
        "largest_child_rss_bytes": after.ru_maxrss * (1 if sys.platform == "darwin" else 1024),
        "exit_code": result,
        "run_id": os.environ.get("GITHUB_RUN_ID"),
        "run_attempt": os.environ.get("GITHUB_RUN_ATTEMPT"),
        "job": os.environ.get("GITHUB_JOB"),
        "logical_cpus": os.cpu_count(),
    }
    # Keep the counter's scope explicit; this is not a per-phase memory peak.
    row.update(cgroup_peak())
    if sampled_memory:
        # Host-wide MemTotal - MemAvailable includes other runner processes.
        row["sampled_host_used_peak_bytes"] = max(sampled_memory)
        row["host_memory_samples"] = len(sampled_memory)
    with (output_dir / "phases.jsonl").open("a") as output:
        output.write(json.dumps(row) + "\n")
    print(json.dumps(row), file=sys.stderr)
    return result if result >= 0 else 128 - result


def gh_json(endpoint):
    return json.loads(subprocess.check_output(["gh", "api", endpoint], text=True))


def run_report(repo, run_id):
    run = gh_json(f"repos/{repo}/actions/runs/{run_id}")
    attempt = run["run_attempt"]
    jobs = []
    page = 1
    while True:
        batch = gh_json(
            f"repos/{repo}/actions/runs/{run_id}/attempts/{attempt}/jobs?per_page=100&page={page}"
        )["jobs"]
        jobs.extend(batch)
        if len(batch) < 100:
            break
        page += 1
    for job in jobs:
        if job["started_at"] and job["completed_at"]:
            job["runner_minutes"] = (
                datetime.fromisoformat(job["completed_at"].replace("Z", "+00:00"))
                - datetime.fromisoformat(job["started_at"].replace("Z", "+00:00"))
            ).total_seconds() / 60
        # Only count an allocation when its CPU size is explicit in runner labels.
        cpus = [
            int(part.removeprefix("cpu="))
            for label in job["labels"]
            for part in label.split(",")
            if part.startswith("cpu=") and part.removeprefix("cpu=").isdecimal()
        ]
        if cpus and "runner_minutes" in job:
            job["allocated_vcpu_minutes"] = cpus[0] * job["runner_minutes"]
    return {
        "run": run,
        "jobs": jobs,
        "artifacts": gh_json(f"repos/{repo}/actions/runs/{run_id}/artifacts?per_page=100"),
        "cache_usage_at_collection": gh_json(f"repos/{repo}/actions/cache/usage"),
        "cost_note": (
            "Runner time excludes provisioning. "
            "vCPU-minutes are an allocation proxy, not a dollar bill."
        ),
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    run = commands.add_parser("run")
    run.add_argument("phase")
    run.add_argument("argv", nargs=argparse.REMAINDER)
    plan = commands.add_parser("plan")
    plan.add_argument("listing", type=Path)
    plan.add_argument("--threshold", type=int, default=1000)
    compare = commands.add_parser("compare")
    compare.add_argument("expected", type=Path)
    compare.add_argument("partitions", nargs="+", type=Path)
    report = commands.add_parser("report")
    report.add_argument("run_id", type=int)
    report.add_argument("--repo", default="aptos-labs/aptos-core")
    args = parser.parse_args()
    if args.command == "run":
        argv = args.argv[1:] if args.argv[:1] == ["--"] else args.argv
        if not argv:
            parser.error("run requires a command")
        return measure(args.phase, argv, Path(os.environ.get("CI_METRICS_DIR", "ci-metrics")))
    if args.command == "plan":
        listing = json.loads(args.listing.read_text())
        mode = execution_mode(listing["test-count"], args.threshold)
        print(f"Archived {listing['test-count']} tests; mode={mode}")
        with open(os.environ["GITHUB_OUTPUT"], "a") as output:
            output.write(f"mode={mode}\n")
    elif args.command == "compare":
        expected = inventory(json.loads(args.expected.read_text()))
        partitions = [inventory(json.loads(path.read_text())) for path in args.partitions]
        check_partitions(expected, partitions)
        print(f"Exact inventory match: {len(expected)} tests across {len(partitions)} inputs")
    else:
        print(json.dumps(run_report(args.repo, args.run_id), indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main())
