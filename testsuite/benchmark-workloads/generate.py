#!/usr/bin/env python3

# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

import subprocess

# Specifies directories and the latest (unstable) language version is needed when building the packages there.
DIRECTORIES = [
    ("packages", ""),
    ("../../aptos-move/move-examples/token_objects/ambassador", ""),
    ("../../aptos-move/move-examples/aggregator_examples", ""),
    ("../../aptos-move/move-examples/bcs-stream", ""),
    ("packages-experimental", " --latest-language"),
]
# Directory where all binaries (package metadata, modules, scripts) are saved.
PREBUILT_PACKAGES_DIR = "../../crates/transaction-workloads-lib/prebuilt-packages"


# Runs a command to generate prebuilt packages for specified directory.
def generate_prebuilt_packages(dir, extra_args=""):
    # By default, we built in dev mode with local framework.
    command = (f"cargo run --package package-generator -- --dev --use-local-std --packages-path {dir} "
               f"--prebuilt-packages-path {PREBUILT_PACKAGES_DIR}{extra_args}")
    subprocess.run(command, shell=True)


# Generates packages which are used to stress-test the loader. These packages contain many modules and are very large.
# As a result, we auto-generate them, save the binaries, and then delete the generated sources.
def generate_prebuilt_large_packages():
    deps_dir = "packages/dependencies"
    subprocess.run(f"mkdir {deps_dir}", shell=True)

    def generate(shape, size, package_name, extra_args=""):
        script = "scripts/generate_dependencies.py"
        command = (f"{script} {shape} --num-nodes {size} --package-name {package_name} "
                   f"--out-dir {deps_dir}/{package_name}{extra_args}")
        subprocess.run(command, shell=True)

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

    generate_prebuilt_packages(deps_dir)

    subprocess.run(f"rm -rf {deps_dir}", shell=True)


if __name__ == "__main__":
    subprocess.run(f"rm -rf {PREBUILT_PACKAGES_DIR}/*", shell=True)

    for (dir, extra_args) in DIRECTORIES:
        generate_prebuilt_packages(dir, extra_args)
    generate_prebuilt_large_packages()
