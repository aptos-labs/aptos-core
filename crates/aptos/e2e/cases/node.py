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


@test_case
def test_node_update_consensus_key(run_helper: RunHelper, test_name=None):
    # run init a new profile
    run_helper.run_command(
        test_name,
        [
            "aptos",
            "init",
            "--assume-yes",
            "--network",
            "local",
            "--profile",
            "consensus",
        ],
        input="\n",
    )

    # run initialize stake to make sure stake pool created
    run_helper.run_command(
        test_name,
        [
            "aptos",
            "stake",
            "initialize-stake-owner",
            "--profile",
            "consensus",
            "--initial-stake-amount",
            "1",
            "--assume-yes",
        ],
    )

    # hardcode the bls12381 key
    bls12381_public_key = "91067b73f284b4fa47141a023b4bbdc819e92a80058cbedfa2a39b43782c624eea1b281b0a8862aee23e77fb59a2ba75"
    bls12381_pop = "857ddee36936599f98376ebc1a4d70a9ea9c066d27c03ff34300cc52ddea72760c3f8c8b8e423626cff585130d38370712945f48438ff24dcac0179dd4eb47389fd317395a36766490ce34c852b4b23fa8f0b55561275014ae56f79a86eff3ef"

    # run the update consensus key command
    response = run_helper.run_command(
        test_name,
        [
            "aptos",
            "node",
            "update-consensus-key",
            "--profile",
            "consensus",
            "--consensus-public-key",
            bls12381_public_key,
            "--proof-of-possession",
            bls12381_pop,
            "--assume-yes",
        ],
    )

    result = json.loads(response.stdout)["Result"]
    if result.get("success") == None or result.get("success") != True:
        raise TestError("update-consensus-key command failed: Did not return success")

    # Make sure consensus key is actually updated on chain
    response = run_helper.run_command(
        test_name,
        [
            "aptos",
            "node",
            "get-stake-pool",
            "--profile",
            "consensus",
            "--owner-address",
            "consensus",
        ],
    )

    result = json.loads(response.stdout)["Result"]
    if result[0] == None or result[0].get("consensus_public_key") == None:
        raise TestError(f"Error: No consensus key on result")

    if result[0].get("consensus_public_key") != f"0x{bls12381_public_key}":
        raise TestError(
            f"Error: Expected consensus key 0x{bls12381_public_key} but got {result[0].get('consensus_public_key')}"
        )


@test_case
def test_node_update_validator_network_address(run_helper: RunHelper, test_name=None):
    network_public_key = (
        "657dac6c023b39b53b8ae5fdc13a1e1222dacde98165324e16086a6766821628"
    )
    network_host = "127.0.0.1"
    network_port = "6180"

    # run the update command
    response = run_helper.run_command(
        test_name,
        [
            "aptos",
            "node",
            "update-validator-network-addresses",
            "--profile",
            "consensus",  # Assume consensus profile is already created from test_node_update_consensus_key
            "--validator-network-public-key",
            network_public_key,
            "--validator-host",
            f"{network_host}:{network_port}",
            "--assume-yes",
        ],
    )

    result = json.loads(response.stdout)["Result"]
    if result.get("success") == None or result.get("success") != True:
        raise TestError(
            "update-validator-network-addresses command failed: Did not return success"
        )

    # Make sure network address is actually updated on chain
    response = run_helper.run_command(
        test_name,
        [
            "aptos",
            "node",
            "get-stake-pool",
            "--profile",
            "consensus",
            "--owner-address",
            "consensus",
        ],
    )

    result = json.loads(response.stdout)["Result"]
    if result[0] == None or result[0].get("validator_network_addresses") == None:
        raise TestError(f"Error: No [validator_network_addresses] key found on result")

    expected_network_address = f"/ip4/{network_host}/tcp/{network_port}/noise-ik/0x{network_public_key}/handshake/0"
    if result[0].get("validator_network_addresses")[0] != expected_network_address:
        raise TestError(
            f"Error: Expected validator network address [{expected_network_address}] but got [{result[0].get('validator_network_addresses')[0]}]"
        )


@test_case
def test_node_join_validator_set(run_helper: RunHelper, test_name=None):
    # create new account to use as join validator
    run_helper.run_command(
        test_name,
        [
            "aptos",
            "init",
            "--assume-yes",
            "--network",
            "local",
            "--profile",
            "join_validator",
        ],
        input="\n",
    )

    # initialize stake owner
    run_helper.run_command(
        test_name,
        [
            "aptos",
            "stake",
            "initialize-stake-owner",
            "--initial-stake-amount",
            "1",
            "--assume-yes",
            "--profile",
            "join_validator",
        ],
    )

    # hardcode the bls12381 key
    bls12381_public_key = "A1E52C80074B8C3EAE4877A817EC4859870BFF5E94B5ACE76FDEECA872394F3B4E4E7D62757FD0EAAA41E5CDB61E6B04"
    bls12381_pop = "B3590F265A90DF2A63D0461F183CF92A58B913FBF3A3B5109C8D37D46D39149D6A5392D25D04B444F22E54682EFEA9F1124C55265C6B184D7DB784A14442BA273648CD780CE334E72AEBB5EEF75229DE038E69FB9E994C26FF10BCCA42D57067"

    # run the update consensus key command
    response = run_helper.run_command(
        test_name,
        [
            "aptos",
            "node",
            "update-consensus-key",
            "--profile",
            "join_validator",
            "--consensus-public-key",
            bls12381_public_key,
            "--proof-of-possession",
            bls12381_pop,
            "--assume-yes",
        ],
    )

    # run show-validator-set to get current voting power
    # and number of validators
    response = run_helper.run_command(
        test_name,
        [
            "aptos",
            "node",
            "show-validator-set",
            "--profile",
            "join_validator",
        ],
    )

    result = json.loads(response.stdout)["Result"]
    total_power = result.get("total_voting_power") + result.get("total_joining_power")

    # run the join validator set command
    response = run_helper.run_command(
        test_name,
        [
            "aptos",
            "node",
            "join-validator-set",
            "--profile",
            "join_validator",
            "--assume-yes",
        ],
    )

    result = json.loads(response.stdout)["Result"]
    if result.get("success") == None or result.get("success") != True:
        raise TestError("Error: did not execute join-validator-set successfully")

    # run show-validator-set to get updated number of validators
    response = run_helper.run_command(
        test_name,
        [
            "aptos",
            "node",
            "show-validator-set",
            "--profile",
            "join_validator",
        ],
    )

    result = json.loads(response.stdout)["Result"]
    total_power_after_join = result.get("total_voting_power") + result.get(
        "total_joining_power"
    )
    if total_power_after_join != total_power + 1:
        raise TestError(
            f"Error: total voting power did not increase by 1 after join-validator-set"
        )


@test_case
# Note: this test is dependent on test_node_join_validator_set
def test_node_leave_validator_set(run_helper: RunHelper, test_name=None):
    # run show-validator-set to get number of pending_inactive validators
    response = run_helper.run_command(
        test_name,
        [
            "aptos",
            "node",
            "show-validator-set",
            "--profile",
            "join_validator",
        ],
    )
    result = json.loads(response.stdout)["Result"]
    pending_inactive_validators = len(result.get("pending_inactive"))

    # run the leave validator set command
    run_helper.run_command(
        test_name,
        [
            "aptos",
            "node",
            "leave-validator-set",
            "--profile",
            "join_validator",  # This is the profile that was used to join the validator set
            "--assume-yes",
        ],
    )

    # run show-validator-set to get updated number of pending_inactive validators
    response = run_helper.run_command(
        test_name,
        [
            "aptos",
            "node",
            "show-validator-set",
            "--profile",
            "join_validator",
        ],
    )

    result = json.loads(response.stdout)["Result"]
    current_pending_inactive_validators = len(result.get("pending_inactive"))
    if current_pending_inactive_validators != pending_inactive_validators + 1:
        raise TestError(
            "Error: expected pending_inactive validators to increase by 1 after leave-validator-set"
        )
