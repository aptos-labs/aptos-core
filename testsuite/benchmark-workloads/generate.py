#!/usr/bin/env python3

# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

import subprocess
import pathlib

ROOT = pathlib.Path(__file__).absolute().parents[2]

# Directory where Move packages with large dependency graphs are auto-generated.
GENERATED_PACKAGES_WITH_MANY_DEPENDENCIES = ROOT / "testsuite/benchmark-workloads/packages/dependencies"
# Script to generate Move packages with large dependency graphs.
PACKAGES_WITH_MANY_DEPENDENCIES_SCRIPT = ROOT / "testsuite/benchmark-workloads/scripts/generate_dependencies.py"

# Specifies directories for regular packages.
PACKAGES = [
    ROOT / "testsuite/benchmark-workloads/packages",
    ROOT / "aptos-move/move-examples/token_objects/ambassador",
    ROOT / "aptos-move/move-examples/aggregator_examples",
    ROOT / "aptos-move/move-examples/bcs-stream",
    GENERATED_PACKAGES_WITH_MANY_DEPENDENCIES
]
# Specifies directories for experimental packages (will be compiled with latest, possibly unstable) language version.
EXPERIMENTAL_PACKAGES = [
    ROOT / "testsuite/benchmark-workloads/packages-experimental/experimental_usecases",
]

# Directory where the prebuilt binary is saved.
PREBUILT_PACKAGES_DIR = ROOT / "crates/transaction-workloads-lib"

def run_command(command):
    print(command)
    with subprocess.Popen(
            command,
            shell=True,
            text=True,
            stdout=subprocess.PIPE,
            bufsize=1,
            universal_newlines=True,
    ) as p:
        if p.stdout is not None:
            for line in p.stdout:
                print(line, end="")

    if p.returncode != 0:
        print(f"FAILED: {command}")
        exit(p.returncode)


if __name__ == "__main__":
    run_command(f"mkdir {GENERATED_PACKAGES_WITH_MANY_DEPENDENCIES}")

    def generate(shape, size, package_name, extra_args=""):
        command = (f"{PACKAGES_WITH_MANY_DEPENDENCIES_SCRIPT} {shape} --num-nodes {size} --package-name {package_name} "
                   f"--out-dir {GENERATED_PACKAGES_WITH_MANY_DEPENDENCIES}/{package_name}{extra_args}")
        run_command(command)

    # Note: if updating parameters of the generated graphs, you may also need to update Rust entrypoints accordingly.
    for size in [8, 64, 256, 512]:
        generate("chain", size, f"chain_{size}")
    for (k, size) in [(3, 81), (8, 585)]:
        generate("tree", size, f"tree_{size}_{k}", f" --tree-fanout {k}")
    for (p, size) in [(0.3, 64), (0.1, 256)]:
        generate("dag", size, f"dag_{size}_sparse", f" --dag-prob-edge {p}")
    for size in [64, 256]:
        generate("dag", size, f"dag_{size}_dense", " --dag-prob-edge 1.0")
    for size in [32, 512]:
        generate("star", size, f"star_{size}")

    package_paths = " ".join(f"--packages-path {package}" for package in PACKAGES)
    experimental_package_paths = " ".join(
        f"--experimental-packages-path {package}" for package in EXPERIMENTAL_PACKAGES)

    # By default, we built in with local framework.
    command = (f"cargo run --package package-generator -- "
               f"--use-local-std "
               f"{package_paths} "
               f"{experimental_package_paths} "
               f"--prebuilt-packages-file-dir {PREBUILT_PACKAGES_DIR} "
               f"--prebuilt-packages-rust-dir {PREBUILT_PACKAGES_DIR}/src")
    run_command(command)

    # Clean-up large auto-generated Move packages.
    run_command(f"rm -rf {GENERATED_PACKAGES_WITH_MANY_DEPENDENCIES}")
