#!/usr/bin/env python3

# Copyright © Velor Foundation
# SPDX-License-Identifier: Apache-2.0

import os
import shutil
import subprocess

from verify_core.common import warm_cache_and_get_latest_backup_version, clear_artifacts


def main():
    # collect all required ENV variables
    REQUIRED_ENVS = [
        "BUCKET",
        "SUB_DIR",
        "BACKUP_CONFIG_TEMPLATE_PATH",
    ]
    if not all(env in os.environ for env in REQUIRED_ENVS):
        raise Exception("Missing required ENV variables")

    BACKUP_CONFIG_TEMPLATE_PATH = os.environ["BACKUP_CONFIG_TEMPLATE_PATH"]

    if not os.path.exists(BACKUP_CONFIG_TEMPLATE_PATH):
        raise Exception("BACKUP_CONFIG_TEMPLATE_PATH does not exist")
    with open(BACKUP_CONFIG_TEMPLATE_PATH, "r") as f:
        config = f.read()
        if "aws" in config and shutil.which("aws") is None:
            raise Exception("Missing required AWS CLI for pulling backup data from S3")

    if os.environ.get("REUSE_BACKUP_ARTIFACTS", "true") == "true":
        print("[main process] clearing existing backup artifacts")
        clear_artifacts()
    else:
        print("[main process] skipping clearing backup artifacts")

    # run verify-modules
    os.mkdir("local")
    subprocess.run(
        [
            "target/release/velor-debugger",
            "velor-db",
            "backup",
            "verify",
            "--validate-modules",
            "--concurrent-downloads=16",
            "--metadata-cache-dir=./local/metadata-cache",
            "--start-version=max",  # to disable transaction verification
            "--skip-epoch-endings",
            "--command-adapter-config",
            f"{BACKUP_CONFIG_TEMPLATE_PATH}",
        ]
    )


if __name__ == "__main__":
    main()
