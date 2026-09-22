#!/usr/bin/env python3
# Copyright (c) Aptos Foundation
# Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

"""Translate the Cargo test plan into fixed workflow outputs; never run tests."""

import argparse
import json
import os
from pathlib import Path
import subprocess

REGISTRY = json.loads(Path(__file__).with_name("registry.json").read_text())


def selections(plan, mode):
    if plan.get("schema_version") != 1 or plan.get("mode") != mode:
        raise ValueError("Unexpected test-plan schema or mode")
    if plan.get("explicit_packages") is not False:
        raise ValueError("E2E selection requires automatic determination")
    selected = plan.get("e2e_tests")
    if not isinstance(selected, dict) or selected.keys() - REGISTRY.keys():
        raise ValueError("Unknown or missing E2E test plan")
    if mode in ("legacy", "compare") and selected.keys() != REGISTRY.keys():
        raise ValueError("Legacy/compare must retain all existing E2E runners")
    return sorted(selected)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--mode", choices=("legacy", "compare", "subsystem"), required=True
    )
    parser.add_argument("--planner-bin", help="Prebuilt trusted planner executable")
    parser.add_argument("--base", required=True)
    parser.add_argument("--plan-file", type=Path, required=True)
    args = parser.parse_args()
    if args.mode == "legacy":
        # Preserve today's gates without requiring Cargo or subsystem config.
        plan = {
            "schema_version": 1,
            "mode": "legacy",
            "explicit_packages": False,
            "e2e_tests": {name: ["Legacy workflow selection"] for name in REGISTRY},
        }
    else:
        result = subprocess.run(
            [
                *([args.planner_bin] if args.planner_bin else ["cargo", "x"]),
                "--determinator",
                args.mode,
                "--base",
                args.base,
                "test-plan",
                "--format",
                "json",
            ],
            check=True,
            stdout=subprocess.PIPE,
            text=True,
        )
        plan = json.loads(result.stdout)
    selected = selections(plan, args.mode)
    rendered = json.dumps(plan, indent=2)
    args.plan_file.write_text(rendered + "\n", encoding="utf-8")
    print(rendered)
    # Write outputs only after validating a complete successful plan.
    with open(os.environ["GITHUB_OUTPUT"], "a", encoding="utf-8") as output:
        output.write("selected_e2e_tests=" + json.dumps(selected) + "\n")
    with open(os.environ["GITHUB_STEP_SUMMARY"], "a", encoding="utf-8") as summary:
        summary.write("### E2E selection\n\n```json\n" + rendered + "\n```\n")


if __name__ == "__main__":
    main()
