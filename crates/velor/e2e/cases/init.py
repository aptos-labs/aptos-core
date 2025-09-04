# Copyright © Velor Foundation
# SPDX-License-Identifier: Apache-2.0

import os

import requests
from common import TestError
from test_helpers import RunHelper
from test_results import test_case


@test_case
def test_init(run_helper: RunHelper, test_name=None):
    # Inititalize a profile for the CLI to use. Note that we do not set the
    # --skip-faucet flag. This means that in addition to creating a profile locally,
    # it will use the faucet to create the account on chain. This will fund the
    # account with the default amount of 100000000 OCTA.
    run_helper.run_command(
        test_name,
        ["velor", "init", "--assume-yes", "--network", "local"],
        input="\n",
    )

    # Assert that the CLI config is there.
    config_path = os.path.join(
        run_helper.host_working_directory, ".velor", "config.yaml"
    )
    if not os.path.exists(config_path):
        raise TestError(
            f"{config_path} not found (in host working dir) after running velor init"
        )

    # Assert that it contains info for the account that was created.
    account_info = run_helper.get_account_info()
    if not account_info:
        raise TestError("Failed to read account info from newly created config file")

    # Confirm with the localnet that it was created.
    try:
        run_helper.api_client.account(account_info.account_address)
    except Exception as e:
        raise TestError(
            f"Failed to query localnet for account {account_info.account_address}"
        ) from e


@test_case
def test_metrics_accessible(run_helper: RunHelper, test_name=None):
    # Assert that the metrics endpoint is accessible and returns valid json if
    # requested. If the endpoint is not accessible or does not return valid
    # JSON this will throw an exception which will be caught as a test failure.
    metrics_url = run_helper.get_metrics_url(json=True)
    requests.get(metrics_url).json()


@test_case
def test_velor_header_included(run_helper: RunHelper, test_name=None):
    # Make sure the velor-cli header is included on the original request
    response = requests.get(run_helper.get_metrics_url())

    if 'request_source_client="velor-cli' not in response.text:
        raise TestError("Request should contain the correct velor header: velor-cli")
