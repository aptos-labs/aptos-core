# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

import json

from common import TestError
from test_helpers import RunHelper
from test_results import test_case


@test_case
def test_node_show_validator_set(run_helper: RunHelper, test_name=None):
    # run the show validator set command
    response = run_helper.run_command(
        test_name,
        [
            "aptos",
            "node",
            "show-validator-set",
            "--profile",
            "default",
        ],
    )

    result = json.loads(response.stdout)["Result"]
    if result.get("scheme") == None or result.get("active_validators") == None:
        raise TestError(
            "Node show validator set command failed: Did not return scheme and active_validators"
        )
