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
            "1000000",
            "--assume-yes",
        ],
    )

    result = json.loads(response.stdout)["Result"]
    if result.get("success") != True:
        raise TestError("Did not initialize stake owner successfully")

    # make sure the the stake pool initialized on chain
    response = run_helper.run_command(
        test_name,
        [
            "aptos",
            "node",
            "get-stake-pool",
            "--profile",
            "default",
            "--owner-address",
            "default",
        ],
    )

    result = json.loads(response.stdout)["Result"]
    if result[0] == None or result[0].get("total_stake") != 1000000:
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
        raise TestError("Did not execute [add-stake] successfully")

    # verify that the stake was added
    response = run_helper.run_command(
        test_name,
        [
            "aptos",
            "node",
            "get-stake-pool",
            "--owner-address",
            "default",
        ],
    )

    result = json.loads(response.stdout)["Result"]
    if result[0].get("total_stake") != 2000000:  # initial 1M + added 1M
        raise TestError(
            f"Did not add stake successfully. Expected 2000000, got {result[0].get('total_stake')}"
        )


@test_case
def test_stake_withdraw_stake_before_unlock(run_helper: RunHelper, test_name=None):
    # get the current stake amount
    response = run_helper.run_command(
        test_name,
        [
            "aptos",
            "node",
            "get-stake-pool",
            "--owner-address",
            "default",
        ],
    )
    result = json.loads(response.stdout)["Result"]
    current_stake = result[0].get("total_stake")

    # run the withdraw-stake command
    response = run_helper.run_command(
        test_name,
        [
            "aptos",
            "stake",
            "withdraw-stake",
            "--amount",
            "1000000",
            "--assume-yes",
        ],
    )

    result = json.loads(response.stdout)["Result"]
    if result[0].get("success") != True:
        raise TestError("Did not execute [withdraw-stake] successfully")

    # Get the current stake amount again
    response = run_helper.run_command(
        test_name,
        [
            "aptos",
            "node",
            "get-stake-pool",
            "--owner-address",
            "default",
        ],
    )

    # Total stake should not have change because the stake is still locked
    result = json.loads(response.stdout)["Result"]
    if result[0].get("total_stake") != current_stake:
        raise TestError(
            f"Total stake should not change before unlock. Expected {current_stake}, got {result[0].get('total_stake')}"
        )


@test_case
def test_stake_set_operator(run_helper: RunHelper, test_name=None):
    # create a new operator account
    run_helper.run_command(
        test_name,
        [
            "aptos",
            "init",
            "--profile",
            "operator",
            "--assume-yes",
            "--network",
            "local",
        ],
        input="\n",
    )

    # run the set-operator command
    response = run_helper.run_command(
        test_name,
        [
            "aptos",
            "stake",
            "set-operator",
            "--operator-address",
            "operator",
            "--assume-yes",
        ],
    )

    result = json.loads(response.stdout)["Result"]
    if result[0].get("success") != True:
        raise TestError("Did not set operator successfully")


@test_case
def test_stake_set_voter(run_helper: RunHelper, test_name=None):
    # create a new voter account
    run_helper.run_command(
        test_name,
        ["aptos", "init", "--profile", "voter", "--assume-yes", "--network", "local"],
        input="\n",
    )

    # run the set-operator command
    response = run_helper.run_command(
        test_name,
        [
            "aptos",
            "stake",
            "set-delegated-voter",
            "--voter-address",
            "voter",
            "--assume-yes",
        ],
    )

    result = json.loads(response.stdout)["Result"]
    if result[0].get("success") != True:
        raise TestError("Did not set delegated-voter successfully")


@test_case
def test_stake_create_staking_contract(run_helper: RunHelper, test_name=None):
    # run the set-operator command
    # Note: This command has to run after set-operator and set-voter
    # because it needs to know the operator and voter addresses
    response = run_helper.run_command(
        test_name,
        [
            "aptos",
            "stake",
            "create-staking-contract",
            "--operator",
            "operator",
            "--voter",
            "voter",
            "--amount",
            "1000000",
            "--commission-percentage",
            "1",
            "--assume-yes",
        ],
    )

    result = json.loads(response.stdout)["Result"]
    if result.get("success") != True:
        raise TestError("Did not set create staking contract successfully")


@test_case
def test_stake_create_staking_contract(run_helper: RunHelper, test_name=None):
    # run the set-operator command
    response = run_helper.run_command(
        test_name,
        [
            "aptos",
            "stake",
            "create-staking-contract",
            "--operator",
            "operator",
            "--voter",
            "voter",
            "--amount",
            "1000000",
            "--commission-percentage",
            "1",
            "--assume-yes",
        ],
    )

    result = json.loads(response.stdout)["Result"]
    if result.get("success") != True:
        raise TestError("Did not set create staking contract successfully")


@test_case
def test_stake_increase_lockup(run_helper: RunHelper, test_name=None):
    # run the set-operator command
    response = run_helper.run_command(
        test_name,
        [
            "aptos",
            "stake",
            "increase-lockup",
            "--assume-yes",
        ],
    )

    result = json.loads(response.stdout)["Result"]
    if result[0].get("success") != True:
        raise TestError("Did not increase lockup successfully")


@test_case
def test_stake_unlock_stake(run_helper: RunHelper, test_name=None):
    # run the unlock-stake command
    response = run_helper.run_command(
        test_name,
        ["aptos", "stake", "unlock-stake", "--amount", "1000000", "--assume-yes"],
    )

    result = json.loads(response.stdout)["Result"]
    if result[0].get("success") != True:
        raise TestError("Did not unlock stake successfully")


@test_case
def test_stake_withdraw_stake_after_unlock(run_helper: RunHelper, test_name=None):
    # get the current stake amount
    response = run_helper.run_command(
        test_name,
        [
            "aptos",
            "node",
            "get-stake-pool",
            "--owner-address",
            "default",
        ],
    )
    result = json.loads(response.stdout)["Result"]
    current_stake = result[0].get("total_stake")

    # run the unlock-stake command
    amount_to_withdraw = 1000000
    response = run_helper.run_command(
        test_name,
        [
            "aptos",
            "stake",
            "unlock-stake",
            "--amount",
            f"{amount_to_withdraw}",
            "--profile",
            "default",
            "--assume-yes",
        ],
    )

    result = json.loads(response.stdout)["Result"]
    if result[0].get("success") != True:
        raise TestError("Did not unlock stake successfully")

    # run the withdraw-stake command
    response = run_helper.run_command(
        test_name,
        [
            "aptos",
            "stake",
            "withdraw-stake",
            "--amount",
            f"{amount_to_withdraw}",
            "--profile",
            "default",
            "--assume-yes",
        ],
    )

    result = json.loads(response.stdout)["Result"]
    if result[0].get("success") != True:
        raise TestError("Did not execute [withdraw-stake] successfully")

    # verify that the stake was withdrawed
    response = run_helper.run_command(
        test_name,
        [
            "aptos",
            "node",
            "get-stake-pool",
            "--owner-address",
            "default",
        ],
    )

    result = json.loads(response.stdout)["Result"]
    if result[0].get("total_stake") != current_stake - amount_to_withdraw:
        raise TestError(
            f"The stake should be decreased by {amount_to_withdraw}. Expected {current_stake - amount_to_withdraw}, got {result[0].get('total_stake')}"
        )


@test_case
def test_stake_request_commission(run_helper: RunHelper, test_name=None):
    # create a new account
    run_helper.run_command(
        test_name,
        [
            "aptos",
            "init",
            "--profile",
            "request_commission",
            "--assume-yes",
            "--network",
            "local",
        ],
        input="\n",
    )

    # create staking contract
    run_helper.run_command(
        test_name,
        [
            "aptos",
            "stake",
            "create-staking-contract",
            "--profile",
            "request_commission",
            "--operator",
            "request_commission",
            "--voter",
            "request_commission",
            "--amount",
            "3",
            "--commission-percentage",
            "1",
            "--assume-yes",
        ],
    )

    # run the request-commission command
    response = run_helper.run_command(
        test_name,
        [
            "aptos",
            "stake",
            "request-commission",
            "--profile",
            "request_commission",
            "--owner-address",
            "request_commission",
            "--operator-address",
            "request_commission",
            "--assume-yes",
        ],
    )

    result = json.loads(response.stdout)["Result"]
    if result.get("success") != True:
        raise TestError("Did not execute [request-commission] successfully")
