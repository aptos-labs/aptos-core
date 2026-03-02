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


def find_mv_files(directory: str) -> list[str]:
    paths = []
    for root, _dirs, files in os.walk(directory):
        for f in files:
            if f.endswith(".mv"):
                paths.append(os.path.join(root, f))
    paths.sort()
    return paths


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Aggregate mseir-compiler statistics over a directory of .mv files."
    )
    parser.add_argument("directory", help="Directory containing .mv files (searched recursively).")
    parser.add_argument(
        "pipeline",
        choices=["v1", "v2"],
        help="Pipeline version to pass to mseir-compiler.",
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
    if errors:
        print(f"errors:               {errors}")

    return 1 if errors else 0


if __name__ == "__main__":
    sys.exit(main())
