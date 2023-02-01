#!/usr/bin/env python3

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

import os
import unittest
import subprocess

from verify_core.common import find_latest_version_from_db_backup_output


class ReplayVerifyHarnessTests(unittest.TestCase):
    def testFindLatestVersionFromDbBackupOutput(self) -> None:
        proc = subprocess.Popen(
            f"cat {os.path.dirname(__file__)}/fixtures/backup_oneshot.fixture", shell=True, stdout=subprocess.PIPE
        )
        latest_version = find_latest_version_from_db_backup_output(proc.stdout)
        self.assertEqual(latest_version, 417000000)
        proc.communicate()
