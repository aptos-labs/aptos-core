# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

import json

from common import TestError
from test_helpers import RunHelper
from test_results import test_case


@test_case
def test_stake_initialize_stake_owner(run_helper: RunHelper, test_name=None):
    # run the initialize-stake-owner command
    response = run_helper.run_command(
        test_name,
        [
            "aptos",
            "stake",
            "initialize-stake-owner",
            "--initial-stake-amount",
            "100000000000",
            "--assume-yes",
        ],
    )

    result = json.loads(response.stdout)["Result"]
    if result.get("success") != True:
        raise TestError("Did not initialize stake owner successfully")


@test_case
def test_stake_add_stake(run_helper: RunHelper, test_name=None):
    # run the add-stake command
    response = run_helper.run_command(
        test_name,
        [
            "aptos",
            "stake",
            "add-stake",
            "--amount",
            "1000000",
            "--assume-yes",
        ],
    )

    result = json.loads(response.stdout)["Result"]
    if result[0].get("success") != True:
        raise TestError("Did not add stake successfully")
