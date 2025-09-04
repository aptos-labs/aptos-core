# Copyright © Velor Foundation
# SPDX-License-Identifier: Apache-2.0

import logging
import subprocess

LOG = logging.getLogger(__name__)


def run_faucet_integration_tests():
    LOG.info("Running the faucet integration tests")
    subprocess.run(
        [
            "cargo",
            "test",
            "--package",
            "velor-faucet-core",
            "--features",
            "integration-tests",
        ],
        check=True,
    )
    LOG.info("The faucet integration tests passed!")
