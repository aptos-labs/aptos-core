#!/usr/bin/env python3
# Copyright (c) Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

"""Run mseir-compiler on every .mv file in a directory and aggregate statistics."""

import argparse
import os
import re
import subprocess
import sys

# Matches the verbose stderr lines produced by mseir-compiler.
# Example: "0x66::test::add_values  bytecode: 4 instrs, 0 locals  |  IR: 2 instrs, 3 regs ..."
STAT_RE = re.compile(
    r"bytecode:\s*(\d+)\s*instrs,\s*(\d+)\s*locals\s*\|\s*IR:\s*(\d+)\s*instrs,\s*(\d+)\s*regs"
)

# Like STAT_RE but also captures the function name prefix.
FUNC_STAT_RE = re.compile(
    r"(\S+)\s+bytecode:\s*(\d+)\s*instrs,\s*(\d+)\s*locals\s*\|\s*IR:\s*(\d+)\s*instrs,\s*(\d+)\s*regs"
)


def find_mv_files(directory: str) -> list[str]:
    paths = []
    for root, _dirs, files in os.walk(directory):
        for f in files:
            if f.endswith(".mv"):
                paths.append(os.path.join(root, f))
    paths.sort()
    return paths


def collect_function_stats(
    bin_path: str, pipeline: str, mv_files: list[str]
) -> tuple[dict[str, tuple[int, int, int, int]], int]:
    """Run mseir-compiler on all files and return per-function stats.

    Returns (stats_dict, error_count) where stats_dict maps
    function_name -> (bc_instrs, bc_locals, ir_instrs, ir_regs).
    """
    stats: dict[str, tuple[int, int, int, int]] = {}
    errors = 0
    for path in mv_files:
        result = subprocess.run(
            [bin_path, "--verbose", "--pipeline", pipeline, path],
            capture_output=True,
            text=True,
        )
        if result.returncode != 0:
            print(f"ERROR ({pipeline}): {path}: {result.stderr.strip()}", file=sys.stderr)
            errors += 1
            continue
        for line in result.stderr.splitlines():
            m = FUNC_STAT_RE.search(line)
            if m:
                stats[m.group(1)] = (
                    int(m.group(2)),
                    int(m.group(3)),
                    int(m.group(4)),
                    int(m.group(5)),
                )
    return stats, errors


def run_compare(bin_path: str, mv_files: list[str]) -> int:
    """Compare v1 and v2 pipelines and report regressions."""
    v1_stats, v1_errors = collect_function_stats(bin_path, "v1", mv_files)
    v2_stats, v2_errors = collect_function_stats(bin_path, "v2", mv_files)

    common = sorted(set(v1_stats) & set(v2_stats))
    regressions: list[tuple[str, tuple[int, int, int, int], tuple[int, int, int, int]]] = []

    for func in common:
        _, _, v1_instrs, v1_regs = v1_stats[func]
        _, _, v2_instrs, v2_regs = v2_stats[func]
        if v2_instrs > v1_instrs and v2_regs >= v1_regs:
            regressions.append((func, v1_stats[func], v2_stats[func]))

    print(f"functions (v1):       {len(v1_stats)}")
    print(f"functions (v2):       {len(v2_stats)}")
    print(f"functions (common):   {len(common)}")
    print(f"regressions:          {len(regressions)}")
    if common:
        print(f"regression rate:      {len(regressions) / len(common) * 100:.1f}%")

    if regressions:
        print()
        print("Functions where v2 has more IR instrs AND >= IR regs than v1:")
        for func, (_, _, v1_i, v1_r), (_, _, v2_i, v2_r) in regressions:
            print(f"  {func}  v1: {v1_i} instrs, {v1_r} regs  |  v2: {v2_i} instrs, {v2_r} regs")

    errors = v1_errors + v2_errors
    if errors:
        print(f"\nerrors:               {errors}")

    return 1 if errors else 0


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Aggregate mseir-compiler statistics over a directory of .mv files."
    )
    parser.add_argument("directory", help="Directory containing .mv files (searched recursively).")
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument(
        "--pipeline",
        choices=["v1", "v2"],
        help="Pipeline version to pass to mseir-compiler.",
    )
    mode.add_argument(
        "--compare",
        action="store_true",
        help="Compare v1 and v2 pipelines. Reports functions where v2 has more IR "
        "instructions AND >= total registers compared with v1.",
    )
    parser.add_argument(
        "--bin",
        default="mseir-compiler",
        help="Path to the mseir-compiler binary (default: mseir-compiler on PATH).",
    )
    args = parser.parse_args()

    mv_files = find_mv_files(args.directory)
    if not mv_files:
        print(f"No .mv files found in {args.directory}", file=sys.stderr)
        return 1

    if args.compare:
        return run_compare(args.bin, mv_files)

    total_bc_instrs = 0
    total_bc_locals = 0
    total_ir_instrs = 0
    total_ir_regs = 0
    total_functions = 0
    total_files = 0
    errors = 0

    for path in mv_files:
        result = subprocess.run(
            [args.bin, "--verbose", "--pipeline", args.pipeline, path],
            capture_output=True,
            text=True,
        )
        if result.returncode != 0:
            print(f"ERROR: {path}: {result.stderr.strip()}", file=sys.stderr)
            errors += 1
            continue

        total_files += 1
        for line in result.stderr.splitlines():
            m = STAT_RE.search(line)
            if m:
                total_functions += 1
                total_bc_instrs += int(m.group(1))
                total_bc_locals += int(m.group(2))
                total_ir_instrs += int(m.group(3))
                total_ir_regs += int(m.group(4))

    print(f"pipeline:             {args.pipeline}")
    print(f"files:                {total_files}")
    print(f"functions:            {total_functions}")
    print(f"bytecode instrs:      {total_bc_instrs}")
    print(f"bytecode locals:      {total_bc_locals}")
    print(f"IR instrs:            {total_ir_instrs}")
    print(f"IR regs:              {total_ir_regs}")

    if total_bc_instrs > 0:
        instr_pct = (total_bc_instrs - total_ir_instrs) / total_bc_instrs * 100
        print(f"instr decrease:       {instr_pct:.1f}%")
    if total_bc_locals > 0:
        reg_pct = (total_ir_regs - total_bc_locals) / total_bc_locals * 100
        print(f"reg increase:         {reg_pct:.1f}%")

    if errors:
        print(f"errors:               {errors}")

    return 1 if errors else 0


if __name__ == "__main__":
    sys.exit(main())
