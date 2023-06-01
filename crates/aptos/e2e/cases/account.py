# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0


from common import OTHER_ACCOUNT_ONE, TestError
from test_helpers import RunHelper
from test_results import test_case


@test_case
def test_account_fund_with_faucet(run_helper: RunHelper, test_name=None):
    amount_in_octa = 100000000000

    # Fund the account.
    run_helper.run_command(
        test_name,
        [
            "aptos",
            "account",
            "fund-with-faucet",
            "--account",
            run_helper.get_account_info().account_address,
            "--amount",
            str(amount_in_octa),
        ],
    )

    # Assert it has the requested balance.
    balance = int(
        run_helper.api_client.account_balance(
            run_helper.get_account_info().account_address
        )
    )
    if balance == amount_in_octa:
        raise TestError(
            f"Account {run_helper.get_account_info().account_address} has balance {balance}, expected {amount_in_octa}"
        )


@test_case
def test_account_create(run_helper: RunHelper, test_name=None):
    # Create the new account.
    run_helper.run_command(
        test_name,
        [
            "aptos",
            "account",
            "create",
            "--account",
            OTHER_ACCOUNT_ONE.account_address,
            "--assume-yes",
        ],
    )

    # Assert it exists and has zero balance.
    balance = int(
        run_helper.api_client.account_balance(OTHER_ACCOUNT_ONE.account_address)
    )
    if balance != 0:
        raise TestError(
            f"Account {OTHER_ACCOUNT_ONE.account_address} has balance {balance}, expected 0"
        )
