#!/usr/bin/env python3

# Copyright © Velor Foundation
# SPDX-License-Identifier: Apache-2.0

import subprocess
import pathlib

ROOT = pathlib.Path(__file__).absolute().parents[2]

# Specifies directories for regular packages.
# Paths are relative to the root of the velor-core repository.
PACKAGES = [
    "testsuite/benchmark-workloads/packages",
    "velor-move/move-examples/token_objects/ambassador",
    "velor-move/move-examples/aggregator_examples",
    "velor-move/move-examples/bcs-stream"
]
# Specifies directories for experimental packages (will be compiled with latest, possibly unstable) language version.
# Paths are relative to the root of the velor-core repository.
EXPERIMENTAL_PACKAGES = [
    "testsuite/benchmark-workloads/packages-experimental/experimental_usecases",
]

# Directory where the prebuilt binary is saved.
PREBUILT_PACKAGES_DIR = "crates/transaction-workloads-lib"


def run_command(command):
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
        print("Failed to generate the workloads!")
        exit(p.returncode)


if __name__ == "__main__":
    package_paths = " ".join(f"--packages-path {ROOT / package}" for package in PACKAGES)
    experimental_package_paths = " ".join(
        f"--experimental-packages-path {ROOT / package}" for package in EXPERIMENTAL_PACKAGES)

    # By default, we built in with local framework.
    command = (f"cargo run --package package-generator -- "
               f"--use-local-std "
               f"{package_paths} "
               f"{experimental_package_paths} "
               f"--prebuilt-packages-file-dir {ROOT / PREBUILT_PACKAGES_DIR} "
               f"--prebuilt-packages-rust-dir {ROOT / PREBUILT_PACKAGES_DIR}/src")
    print(command)
    run_command(command)
