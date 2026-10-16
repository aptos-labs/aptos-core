import argparse
import json
import re
from pathlib import Path


def selected_tests(document):
    tests = set()
    for binary_id, suite in document["rust-suites"].items():
        for name, case in suite["testcases"].items():
            if not case["ignored"] and case["filter-match"]["status"] == "matches":
                tests.add((binary_id, name))
    if not tests:
        raise ValueError("No selected tests in inventory")
    return tests


def verify(expected_path, results_dir, shards):
    expected = selected_tests(json.loads(expected_path.read_text()))
    covered = set()
    counts = []
    for shard in range(1, shards + 1):
        selected = selected_tests(
            json.loads((results_dir / f"inventory-{shard}.json").read_text())
        )
        if covered & selected:
            raise ValueError(f"Duplicate tests in shard {shard}")
        covered.update(selected)
        log = (results_dir / f"tests-{shard}.log").read_text()
        log = re.sub(r"\x1b\[[0-9;]*m", "", log)
        summaries = re.findall(
            r"Summary\s+\[[^\]]+\]\s+(\d+) tests run: (\d+) passed", log
        )
        if len(summaries) != 1:
            raise ValueError(
                f"Missing or ambiguous successful summary for shard {shard}"
            )
        run, passed = map(int, summaries[0])
        if run != len(selected) or passed != run:
            raise ValueError(f"Executed count differs from inventory in shard {shard}")
        counts.append(run)
    if covered != expected:
        raise ValueError(
            f"Coverage mismatch: {len(expected - covered)} missing, "
            f"{len(covered - expected)} unexpected tests"
        )
    return counts


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("expected", type=Path)
    parser.add_argument("results", type=Path)
    parser.add_argument("--shards", type=int, default=4)
    args = parser.parse_args()
    counts = verify(args.expected, args.results, args.shards)
    print(f"Verified {sum(counts)} tests across disjoint shards: {counts}")
