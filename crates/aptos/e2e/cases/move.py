# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

import json

from common import TestError
from test_helpers import RunHelper
from test_results import test_case


@test_case
def test_move_publish(run_helper: RunHelper, test_name=None):
    # Prior to this function running the move/ directory was moved into the working
    # directory in the host, which is then mounted into the container. The CLI is
    # then run in this directory, meaning the move/ directory is in the same directory
    # as the CLI is run from. This is why we can just refer to the package dir starting
    # with move/ here.
    package_dir = f"move/cli-e2e-tests/{run_helper.base_network}"

    # Publish the module.
    run_helper.run_command(
        test_name,
        [
            "aptos",
            "move",
            "publish",
            "--assume-yes",
            "--package-dir",
            package_dir,
            "--named-addresses",
            f"addr={run_helper.get_account_info().account_address}",
        ],
    )

    # Get what modules exist on chain.
    response = run_helper.run_command(
        test_name,
        [
            "aptos",
            "account",
            "list",
            "--account",
            run_helper.get_account_info().account_address,
            "--query",
            "modules",
        ],
    )

    # Confirm that the module exists on chain.
    response = json.loads(response.stdout)
    for module in response["Result"]:
        if (
            module["abi"]["address"]
            == f"0x{run_helper.get_account_info().account_address}"
            and module["abi"]["name"] == "cli_e2e_tests"
        ):
            return

    raise TestError(
        "Module apparently published successfully but it could not be found on chain"
    )


@test_case
def test_move_compile(run_helper: RunHelper, test_name=None):
    package_dir = f"move/cli-e2e-tests/{run_helper.base_network}"
    account_info = run_helper.get_account_info()

    # Compile the module.
    response = run_helper.run_command(
        test_name,
        [
            "aptos",
            "move",
            "compile",
            "--package-dir",
            package_dir,
            "--named-addresses",
            f"addr={account_info.account_address}",
        ],
    )

    if f"{account_info.account_address}::cli_e2e_tests" not in response.stdout:
        raise TestError("Module did not compile successfully")


@test_case
def test_move_compile_dev_mode(run_helper: RunHelper, test_name=None):
    package_dir = f"move/cli-e2e-tests/{run_helper.base_network}"
    account_info = run_helper.get_account_info()

    # Compile the module.  Should not need an address passed in
    response = run_helper.run_command(
        test_name,
        [
            "aptos",
            "move",
            "compile",
            "--dev",
            "--package-dir",
            package_dir,
        ],
    )

    if f"{account_info.account_address}::cli_e2e_tests" not in response.stdout:
        raise TestError("Module did not compile successfully")


@test_case
def test_move_compile_script(run_helper: RunHelper, test_name=None):
    package_dir = f"move/cli-e2e-tests/{run_helper.base_network}"
    account_info = run_helper.get_account_info()

    # Compile the script.
    response = run_helper.run_command(
        test_name,
        [
            "aptos",
            "move",
            "compile-script",
            "--package-dir",
            package_dir,
            "--named-addresses",
            f"addr={account_info.account_address}",
        ],
    )

    if "script_hash" not in response.stdout:
        raise TestError("Script did not compile successfully")


@test_case
def test_move_run(run_helper: RunHelper, test_name=None):
    account_info = run_helper.get_account_info()

    # Run the min_hero entry function with default profile
    response = run_helper.run_command(
        test_name,
        [
            "aptos",
            "move",
            "run",
            "--assume-yes",
            "--function-id",
            "default::cli_e2e_tests::mint_hero",
            "--args",
            "string:Boss",
            "string:Male",
            "string:Jin",
            "string:Undead",
            "string:",
        ],
    )

    if '"success": true' not in response.stdout:
        raise TestError("Move run did not execute successfully")

    # Get what modules exist on chain.
    response = run_helper.run_command(
        test_name,
        [
            "aptos",
            "move",
            "view",
            "--assume-yes",
            "--function-id",
            f"0x{account_info.account_address}::cli_e2e_tests::view_hero",
            "--args",
            f"address:0x{account_info.account_address}",
            "string:Hero Quest",
            "string:Jin",
        ],
    )

    result = json.loads(response.stdout)["Result"]
    if result[0].get("gender") != "Male" and result[0].get("race") != "Undead":
        raise TestError(
            "Data on chain (view_hero) does not match expected data from (mint_hero)"
        )


@test_case
def test_move_view(run_helper: RunHelper, test_name=None):
    account_info = run_helper.get_account_info()

    # Run the view function
    response = run_helper.run_command(
        test_name,
        [
            "aptos",
            "move",
            "view",
            "--function-id",
            "0x1::account::exists_at",
            "--args",
            f"address:{account_info.account_address}",
        ],
    )

    response = json.loads(response.stdout)
    if response["Result"] == None or response["Result"][0] != True:
        raise TestError("View function did not return correct result")
