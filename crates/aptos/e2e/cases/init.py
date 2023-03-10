# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

import os

from common import TestError
from test_helpers import RunHelper
from test_results import test_case


@test_case
def test_init(run_helper: RunHelper, test_name=None):
    # Init the CLI, which creates an account.
    run_helper.run_command(
        test_name,
        ["aptos", "init", "--assume-yes", "--network", "local", "--skip-faucet"],
        input="\n",
    )

    # Assert that the CLI config is there.
    config_path = os.path.join(
        run_helper.host_working_directory, ".aptos", "config.yaml"
    )
    if not os.path.exists(config_path):
        raise TestError(
            f"{config_path} not found (in host working dir) after running aptos init"
        )

    # Assert that it contains info for the account that was created.
    account_info = run_helper.get_account_info()
    if not account_info:
        raise TestError("Failed to read account info from newly created config file")

    # Confirm with the local testnet that it was created.
    try:
        run_helper.api_client.account(account_info.account_address)
    except Exception as e:
        raise TestError(
            f"Failed to query local testnet for account {account_info.account_address}"
        ) from e
