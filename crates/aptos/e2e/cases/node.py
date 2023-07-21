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

    validator_0 = result.get("active_validators")[0]
    validator_config = validator_0.get("config")
    if (
        validator_0.get("account_address") == None
        or validator_config.get("consensus_public_key") == None
        or validator_config.get("validator_network_addresses") == None
        or validator_config.get("fullnode_network_addresses") == None
    ):
        raise TestError(
            "Node show validator set command failed: Did not return account_address, consensus_public_key, or config"
        )
