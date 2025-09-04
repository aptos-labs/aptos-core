# Copyright © Velor Foundation
# SPDX-License-Identifier: Apache-2.0

import json
import secrets
import time

from common import OTHER_ACCOUNT_ONE, TestError
from test_helpers import RunHelper
from test_results import test_case


@test_case
async def test_account_fund_with_faucet(run_helper: RunHelper, test_name=None):
    old_balance = int(
        await run_helper.api_client.account_balance(
            run_helper.get_account_info().account_address
        )
    )
    amount_in_octa = 100000000000

    # Fund the account.
    run_helper.run_command(
        test_name,
        [
            "velor",
            "account",
            "fund-with-faucet",
            "--account",
            str(run_helper.get_account_info().account_address),
            "--amount",
            str(amount_in_octa),
        ],
    )

    # Assert it has the requested balance.
    balance = int(
        await run_helper.api_client.account_balance(
            run_helper.get_account_info().account_address
        )
    )
    if balance != amount_in_octa + old_balance:
        raise TestError(
            f"Account {run_helper.get_account_info().account_address} has balance {balance}, expected {amount_in_octa + old_balance}"
        )


@test_case
async def test_account_create_and_transfer(run_helper: RunHelper, test_name=None):
    # Create the new account.
    run_helper.run_command(
        test_name,
        [
            "velor",
            "account",
            "create",
            "--account",
            str(OTHER_ACCOUNT_ONE.account_address),
            "--assume-yes",
        ],
    )

    # Assert it exists and has zero balance.
    balance = int(
        await run_helper.api_client.account_balance(OTHER_ACCOUNT_ONE.account_address)
    )
    if balance != 0:
        raise TestError(
            f"Account {OTHER_ACCOUNT_ONE.account_address} has balance {balance}, expected 0"
        )

    transfer_amount = 1000

    run_helper.run_command(
        test_name,
        [
            "velor",
            "account",
            "transfer",
            "--account",
            str(OTHER_ACCOUNT_ONE.account_address),
            "--amount",
            str(transfer_amount),
            "--assume-yes",
        ],
    )

    balance = int(
        await run_helper.api_client.account_balance(OTHER_ACCOUNT_ONE.account_address)
    )

    if balance != transfer_amount:
        raise TestError(
            f"Account {OTHER_ACCOUNT_ONE.account_address} has balance {balance}, expected {transfer_amount}"
        )


@test_case
def test_account_list(run_helper: RunHelper, test_name=None):
    # List the created account
    result = run_helper.run_command(
        test_name,
        [
            "velor",
            "account",
            "list",
            "--account",
            str(run_helper.get_account_info().account_address),
        ],
    )

    json_result = json.loads(result.stdout)
    found_account = False

    # Check if the resource account is in the list
    for module in json_result["Result"]:
        if module.get("0x1::account::Account") != None:
            found_account = True

    if not found_account:
        raise TestError(
            "Cannot find the account in the account list after account creation"
        )


@test_case
def test_account_lookup_address(run_helper: RunHelper, test_name=None):
    # Create the new account.
    result_addr = run_helper.run_command(
        test_name,
        [
            "velor",
            "account",
            "lookup-address",
            "--auth-key",
            str(run_helper.get_account_info().account_address),  # initially the account address is the auth key
        ],
    )

    if str(run_helper.get_account_info().account_address)[2:] not in result_addr.stdout:
        raise TestError(
            f"lookup-address result does not match {run_helper.get_account_info().account_address}"
        )


@test_case
def test_account_rotate_key(run_helper: RunHelper, test_name=None):
    # Generate new private key
    new_private_key = secrets.token_hex(32)

    # Current account info
    old_profile = run_helper.get_account_info()

    # Rotate the key.
    result = run_helper.run_command(
        test_name,
        [
            "velor",
            "account",
            "rotate-key",
            "--new-private-key",
            new_private_key,
            "--skip-saving-profile",
            "--assume-yes",
        ],
    )

    if '"success": true' not in result.stdout:
        raise TestError(
            f"[velor account rotate-key --new-private-key {new_private_key} --skip-saving-profile --assume-yes] failed"
        )

    new_profile = run_helper.get_account_info()
    # Make sure new and old account addresses match
    if old_profile.account_address != new_profile.account_address:
        raise TestError(
            f"Error: Account address changed after rotate-key: {old_profile.account_address} -> {new_profile.account_address}"
        )

    # lookup-address from old public key
    result = run_helper.run_command(
        test_name,
        [
            "velor",
            "account",
            "lookup-address",
            f"--public-key={old_profile.public_key}",
        ],
    )
    response = json.loads(result.stdout)
    if f"0x{response['Result']}" != str(old_profile.account_address):
        raise TestError(
            f"lookup-address of old public key does not match original address: {old_profile.account_address}"
        )

    # lookup-address with new public key
    result = run_helper.run_command(
        test_name,
        [
            "velor",
            "account",
            "lookup-address",
            f"--public-key={new_profile.public_key}",
        ],
    )
    response = json.loads(result.stdout)
    if f"0x{response['Result']}" != str(old_profile.account_address):
        raise TestError(
            f"lookup-address of new public key does not match original address: {old_profile.account_address}"
        )


@test_case
def test_account_resource_account(run_helper: RunHelper, test_name=None):
    # Seed for the resource account
    seed = "1"

    # Create the new resource account.
    result = run_helper.run_command(
        test_name,
        [
            "velor",
            "account",
            "create-resource-account",
            "--seed",
            seed,
            "--assume-yes",  # assume yes to gas prompt
        ],
    )

    result = json.loads(result.stdout)
    sender = result["Result"].get("sender")
    resource_account_address = result["Result"].get("resource_account")

    if resource_account_address == None or sender == None:
        raise TestError("Resource account creation failed")

    # Derive the resource account
    result = run_helper.run_command(
        test_name,
        [
            "velor",
            "account",
            "derive-resource-account-address",
            "--seed",
            seed,
            "--address",
            sender,
        ],
    )

    if resource_account_address not in result.stdout:
        raise TestError(
            f"derive-resource-account-address result does not match expected: {resource_account_address}"
        )

    # List the resource account
    time.sleep(5)
    result = run_helper.run_command(
        test_name,
        [
            "velor",
            "account",
            "list",
            "--query=resources",
        ],
    )

    json_result = json.loads(result.stdout)
    found_resource = False

    # Check if the resource account is in the list
    for module in json_result["Result"]:
        if module.get("0x1::resource_account::Container") != None:
            data = module["0x1::resource_account::Container"]["store"]["data"]
            for resource in data:
                if resource.get("key") == f"0x{resource_account_address}":
                    found_resource = True
                    break

    if not found_resource:
        raise TestError(
            "Cannot find the resource account in the account list after resource account creation"
        )
