# Copyright © Velor Foundation
# SPDX-License-Identifier: Apache-2.0

import os
import re
import shutil
import subprocess
from typing import IO


def find_latest_version_from_db_backup_output(output: IO[bytes]):
    latest_version = -1
    for line in iter(output.readline, b""):
        log_line = line.decode("utf-8")
        latest_version = find_latest_version_from_db_back_log_line(log_line)
        print(log_line.strip(), flush=True)
        if latest_version > 0:
            break

    return latest_version


def find_latest_version_from_db_back_log_line(log_line: str):
    match = re.search(r"latest_transaction_version: (\d+)", log_line)
    if match:
        print(match.group(1))
        return int(match.group(1))
    else:
        return -1


def clear_artifacts():
    """Clears artifacts from previous runs"""
    shutil.rmtree("metadata-cache", ignore_errors=True)
    shutil.rmtree("local", ignore_errors=True)
    files = [f for f in os.listdir(".") if re.match(r"run_[0-9]+_[0-9]+", f)]

    for f in files:
        shutil.rmtree(f)


def warm_cache_and_get_latest_backup_version(
    backup_config_template_path: str,
) -> int:
    """query latest version in backup, at the same time, pre-heat metadata cache"""
    db_backup_result = subprocess.Popen(
        [
            "target/release/velor-debugger",
            "velor-db",
            "backup",
            "query",
            "backup-storage-state",
            "--metadata-cache-dir",
            "./metadata-cache",
            "--command-adapter-config",
            backup_config_template_path,
        ],
        stdout=subprocess.PIPE,
    )
    if db_backup_result.stdout is None:
        raise Exception("Failed to run velor db tool backup. Cannot get stdout.")
    latest_version = find_latest_version_from_db_backup_output(db_backup_result.stdout)
    if latest_version < 0:
        raise Exception("Failed to find latest version")
    db_backup_result.wait()

    return latest_version
