"""Reproduce Table 1's costs and percentile bootstrap intervals (stdlib only).

Usage: python3 cost_intervals.py /path/to/round-001-full.tar.gz

The estimand is mean cost over the fixed, equally weighted retained corpus.
VS-shares-002 is excluded in all arms and replicates after a tooling incident
was identified in hybrid-flexible replicate 4. The raw archive remains intact.
Within each task, resample four task/replicate blocks with replacement, keeping
the three arms together. Do not treat the 68 runs as interchangeable tasks.
Per-task intervals enumerate all 4**4 resamples; aggregate intervals use 100,000
draws. Percentile endpoints use linear interpolation at 2.5% and 97.5%.
With four replicates, per-task intervals are exploratory and can under-cover.
These intervals do not establish generalization to a population of Move tasks.
"""

import argparse
import itertools
import json
import random
import re
import statistics
import tarfile


ARMS = ("agent-only", "hybrid-guided", "hybrid-flexible")
EXCLUDED_TASKS = {"VS-shares-002"}


def interval(values):
    values = sorted(values)

    def quantile(p):
        position = (len(values) - 1) * p
        lower = int(position)
        upper = min(lower + 1, len(values) - 1)
        return values[lower] + (position - lower) * (values[upper] - values[lower])

    return [quantile(0.025), quantile(0.975)]


def load_costs(path):
    costs = {}
    with tarfile.open(path) as archive:
        for member in archive:
            match = re.fullmatch(
                r"round-001/runs/round-001-(.+)-r(\d+)-"
                r"(agent-only|hybrid-guided|hybrid-flexible)-acceptance/"
                r"controller-events.jsonl", member.name
            )
            if not match:
                continue
            task, replicate, arm = match.groups()
            sessions = {}
            for line in archive.extractfile(member):
                event = json.loads(line)
                if event.get("event") == "agent_result":
                    result = event["result"]
                    # Counters accumulate within a session. Retries with new
                    # sessions incur additional cost, including failed runs.
                    sessions[result["session_id"]] = result
            if not sessions:
                raise ValueError(f"Missing usage: {member.name}")
            cost = 0.0
            for result in sessions.values():
                usage = result["model_usage"]
                if not usage:
                    raise ValueError(f"Missing model usage: {member.name}")
                for counters in usage.values():
                    cost += (
                        1.40 * counters["inputTokens"]
                        + 0.26 * counters["cacheReadInputTokens"]
                        + 4.40 * counters["outputTokens"]
                    ) / 1_000_000
            key = (task, int(replicate), arm)
            if key in costs:
                raise ValueError(f"Duplicate run: {key}")
            costs[key] = cost
    tasks = sorted({key[0] for key in costs})
    expected = {(t, r, a) for t in tasks for r in range(1, 5) for a in ARMS}
    if len(tasks) != 17 or set(costs) != expected:
        raise ValueError("Expected a complete 17-task, 4-replicate, 3-arm design")
    return tasks, costs


def calculate(path, draws=100_000, seed=20260907):
    tasks, costs = load_costs(path)
    tasks = [task for task in tasks if task not in EXCLUDED_TASKS]
    selections = list(itertools.product(range(1, 5), repeat=4))
    task_bootstraps = []
    rows = {}
    for task in tasks:
        means = [statistics.mean(costs[task, r, a] for r in range(1, 5)) for a in ARMS]
        samples = [
            tuple(statistics.mean(costs[task, r, a] for r in selected) for a in ARMS)
            for selected in selections
        ]
        task_bootstraps.append(samples)
        rows[task] = {
            arm: {"mean_usd": means[i], "ci95_usd": interval(s[i] for s in samples)}
            for i, arm in enumerate(ARMS)
        }

    rng = random.Random(seed)
    aggregate = [[] for _ in ARMS]
    ratios = [[], []]
    for _ in range(draws):
        totals = [0.0, 0.0, 0.0]
        for samples in task_bootstraps:
            selected = samples[rng.randrange(len(samples))]
            for i in range(3):
                totals[i] += selected[i]
        for i in range(3):
            aggregate[i].append(totals[i] / len(tasks))
        for i in range(2):
            ratios[i].append(totals[i + 1] / totals[0])

    means = [statistics.mean(costs[t, r, a] for t in tasks for r in range(1, 5)) for a in ARMS]
    rows[f"all {len(tasks)}"] = {
        arm: {"mean_usd": means[i], "ci95_usd": interval(aggregate[i])}
        for i, arm in enumerate(ARMS)
    }
    return {
        "method": "percentile bootstrap, fixed tasks, paired replicate blocks within task",
        "draws": draws,
        "seed": seed,
        "excluded_tasks": sorted(EXCLUDED_TASKS),
        "retained_cells": len(tasks) * 4 * len(ARMS),
        "rows": rows,
        "hybrid_agent_cost_ratios": {
            arm: {"estimate": means[i + 1] / means[0], "ci95": interval(ratios[i])}
            for i, arm in enumerate(ARMS[1:])
        },
    }


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("archive")
    args = parser.parse_args()
    print(json.dumps(calculate(args.archive), indent=2))
