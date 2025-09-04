# Copyright © Velor Foundation
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
            "velor",
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
            "velor",
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
            "velor",
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
            "velor",
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
            "velor",
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
            "velor",
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
            "velor",
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
