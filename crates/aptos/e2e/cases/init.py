# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

import os

from cases.shared import TestError


def test_init(run_helper):
    run_helper.run_command(
        "test_init",
        ["aptos", "init", "--assume-yes", "--network", "local"],
        input="\n",
    )
    config_path = os.path.join(
        run_helper.host_working_directory, ".aptos", "config.yaml"
    )
    if not os.path.exists(config_path):
        raise TestError(
            f"{config_path} not found (in host working dir) after running aptos init"
        )
