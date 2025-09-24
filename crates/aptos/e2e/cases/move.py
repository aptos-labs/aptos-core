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
            f"addr={str(run_helper.get_account_info().account_address)}",
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
            str(run_helper.get_account_info().account_address),
            "--query",
            "modules",
        ],
    )

    # Confirm that the module exists on chain.
    response = json.loads(response.stdout)
    for module in response["Result"]:
        if (
            module["abi"]["address"]
            == str(run_helper.get_account_info().account_address)
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
            f"addr={str(account_info.account_address)}",
        ],
    )

    if f"{str(account_info.account_address)[2:]}::cli_e2e_tests" not in response.stdout:
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
def test_move_compile_fetch_deps_only(run_helper: RunHelper, test_name=None):
    package_dir = f"move/cli-e2e-tests/{run_helper.base_network}"
    account_info = run_helper.get_account_info()

    # Compile the module. Compilation should not be invoked, and return should be [].
    response = run_helper.run_command(
        test_name,
        [
            "aptos",
            "move",
            "compile",
            "--package-dir",
            package_dir,
            "--fetch-deps-only"
        ],
    )

    if f"{account_info.account_address}::cli_e2e_tests" in response.stdout:
        raise TestError("Module compilation should not be invoked")


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
            f"{str(account_info.account_address)}::cli_e2e_tests::view_hero",
            "--args",
            f"address:{str(account_info.account_address)}",
            "string:Hero Quest",
            "string:Jin",
        ],
    )

    result = json.loads(response.stdout)["Result"]
    if result[0].get("gender") != "Male" and result[0].get("race") != "Undead":
        raise TestError(
            "Data on chain (view_hero) does not match expected data from (mint_hero)"
        )

    # Run test_move_run to entry function with default profile
    # Make sure other parameters are able to be called using "move run"
    # Notice the entry function is not running anything but just testing the parameters
    response = run_helper.run_command(
        test_name,
        [
            "aptos",
            "move",
            "run",
            "--assume-yes",
            "--function-id",
            "default::cli_e2e_tests::test_move_run",
            "--args",
            "string:1234",  # Notice this is testing u8 vector instead of actual string
            "u16:[1,2]",
            "u32:[1,2]",
            "u64:[1,2]",
            "u128:[1,2]",
            "u256:[1,2]",
            'address:["0x123","0x456"]',
            "bool:[true,false]",
            'string:["abc","efg"]',
        ],
    )

    if '"success": true' not in response.stdout:
        raise TestError("Move run did not execute successfully")


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

    # Test view function with with big number arguments
    expected_u64 = 18446744073709551615
    expected_128 = 340282366920938463463374607431768211455
    expected_256 = (
        115792089237316195423570985008687907853269984665640564039457584007913129639935
    )
    response = run_helper.run_command(
        test_name,
        [
            "aptos",
            "move",
            "view",
            "--assume-yes",
            "--function-id",
            "default::cli_e2e_tests::test_big_number",
            "--args",
            f"u64:{expected_u64}",
            f"u128:{expected_128}",
            f"u256:{expected_256}",  # Important to test this big number
        ],
    )

    response = json.loads(response.stdout)
    if (
        response["Result"] == None
        or response["Result"][0] != f"{expected_u64}"
        or response["Result"][1] != f"{expected_128}"
        or response["Result"][2] != f"{expected_256}"
    ):
        raise TestError(
            f"View function [test_big_number] did not return correct result"
        )

    # Test view function with vector arguments
    # Follow 2 lines are for testing vector of u16-u256
    response = run_helper.run_command(
        test_name,
        [
            "aptos",
            "move",
            "view",
            "--assume-yes",
            "--function-id",
            "default::cli_e2e_tests::test_vector",
            "--args",
            "string:1234",  # Notice this is testing u8 vector instead of actual string
            f"u16:[1,2]",
            f"u32:[1,2]",
            f"u64:[1,2]",
            f"u128:[1,2]",
            f"u256:[1,2]",
            f'address:["0x123","0x456"]',
            "bool:[true,false]",
            'string:["abc","efg"]',
        ],
    )

    response = json.loads(response.stdout)
    if response["Result"] == None or len(response["Result"]) != 9:
        raise TestError(f"View function [test_vector] did not return correct result")
