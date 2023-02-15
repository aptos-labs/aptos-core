# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0


from common import ACCOUNT_ONE


def test_account_fund_with_faucet(run_helper):
    run_helper.run_command(
        "test_account_fund_with_faucet",
        [
            "aptos",
            "account",
            "fund-with-faucet",
            "--account",
            run_helper.get_account_info().account_address,
        ],
    )


def test_account_create(run_helper):
    run_helper.run_command(
        "test_account_create",
        [
            "aptos",
            "account",
            "create",
            "--account",
            ACCOUNT_ONE.account_address,
            "--assume-yes",
        ],
    )
