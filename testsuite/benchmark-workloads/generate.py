#!/usr/bin/env python3

# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

import subprocess
import pathlib


ROOT = pathlib.Path(__file__).absolute().parents[2]

# Specifies directories for regular packages.
# Paths are relative to the root of the aptos-core repository.
PACKAGES = [
    "testsuite/benchmark-workloads/packages",
    "aptos-move/move-examples/token_objects/ambassador",
    "aptos-move/move-examples/aggregator_examples",
    "aptos-move/move-examples/bcs-stream"
]
# Specifies directories for experimental packages (will be compiled with latest, possibly unstable) language version.
# Paths are relative to the root of the aptos-core repository.
EXPERIMENTAL_PACKAGES = [
    "testsuite/benchmark-workloads/packages-experimental/experimental_usecases",
]

# Directory where the prebuilt binary is saved.
PREBUILT_PACKAGES_DIR = "crates/transaction-workloads-lib"

if __name__ == "__main__":
    package_paths = " ".join(f"--packages-path {ROOT / package}" for package in PACKAGES)
    experimental_package_paths = " ".join(f"--experimental-packages-path {ROOT / package}" for package in EXPERIMENTAL_PACKAGES)

    # By default, we built in with local framework.
    command = (f"cargo run --package package-generator -- "
               f"--use-local-std "
               f"{package_paths} "
               f"{experimental_package_paths} "
               f"--prebuilt-packages-file-dir {ROOT / PREBUILT_PACKAGES_DIR} "
               f"--prebuilt-packages-rust-dir {ROOT / PREBUILT_PACKAGES_DIR}/src")
    print(command)
    subprocess.run(command, shell=True)
