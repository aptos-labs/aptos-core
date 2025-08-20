#!/usr/bin/env python3

# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

import subprocess

# Specifies directories and the latest (unstable) language version is needed when building the packages there.
DIRECTORIES = [
    ("packages", " --latest-language"),
    ("../../aptos-move/move-examples/token_objects/ambassador", " --latest-language"),
    ("../../aptos-move/move-examples/aggregator_examples", " --latest-language"),
    ("../../aptos-move/move-examples/bcs-stream", " --latest-language"),
    ("packages-experimental", " --latest-language"),
]
# Directory where all binaries (package metadata, modules, scripts) are saved.
PREBUILT_PACKAGES_DIR = "../../crates/transaction-workloads-lib/prebuilt-packages"

if __name__ == "__main__":
    subprocess.run(f"rm -rf {PREBUILT_PACKAGES_DIR}/*", shell=True)

    for (dir, latest_language_arg) in DIRECTORIES:
        # By default, we built in dev mode with local framework.
        command = (f"cargo run --package package-generator -- --dev --use-local-std --packages-path {dir} "
                   f"--prebuilt-packages-path {PREBUILT_PACKAGES_DIR}{latest_language_arg}")
        print(command)
        subprocess.run(command, shell=True)

    # Also, update the packages using the legacy flow (generating Rust files with inline binaries).
    command = "cargo run -p module-publish"
    print(f"[WARNING] Generating packages using the deprecated flow: {command}")
    subprocess.run(command, shell=True)
