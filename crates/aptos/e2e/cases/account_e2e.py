# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

"""
Comprehensive E2E tests for the `aptos account` CLI subcommand.

This module provides comprehensive testing for:
1. CLI flag availability and regression testing
2. Help text verification
3. Functional tests for all account operations

These tests ensure CLI interface stability and catch any regressions
in flag availability or behavior.
"""

import json
import secrets
import subprocess

from cases.cli_flag_helpers import (
    ExpectedFlag,
    run_help_command,
    verify_flags_present,
    verify_subcommands_present,
)
from common import OTHER_ACCOUNT_ONE, TestError
from test_helpers import RunHelper
from test_results import test_case


# =============================================================================
# Expected flags for each account subcommand
# These definitions serve as regression tests - if any flag is removed or
# renamed, the corresponding test will fail.
# =============================================================================

# Common flags used across multiple commands
COMMON_PROFILE_FLAGS = [
    ExpectedFlag(
        name="profile",
        long_form="--profile",
        description_contains="profile to use",
    ),
]

COMMON_REST_OPTIONS_FLAGS = [
    ExpectedFlag(
        name="url",
        long_form="--url",
        description_contains="URL",
    ),
    ExpectedFlag(
        name="connection-timeout-secs",
        long_form="--connection-timeout-secs",
        description_contains="timeout",
    ),
]

COMMON_TRANSACTION_FLAGS = [
    ExpectedFlag(
        name="assume-yes",
        long_form="--assume-yes",
        description_contains="yes",
    ),
    ExpectedFlag(
        name="gas-unit-price",
        long_form="--gas-unit-price",
        description_contains="gas",
    ),
    ExpectedFlag(
        name="max-gas",
        long_form="--max-gas",
        description_contains="gas",
    ),
    ExpectedFlag(
        name="expiration-secs",
        long_form="--expiration-secs",
        description_contains="expiration",
    ),
    ExpectedFlag(
        name="private-key",
        long_form="--private-key",
        description_contains="private key",
    ),
    ExpectedFlag(
        name="private-key-file",
        long_form="--private-key-file",
        description_contains="private key",
    ),
    ExpectedFlag(
        name="sender-account",
        long_form="--sender-account",
        description_contains="sender",
    ),
]

# Flags for `aptos account create`
ACCOUNT_CREATE_FLAGS = [
    ExpectedFlag(
        name="account",
        long_form="--account",
        description_contains="Address",
    ),
] + COMMON_TRANSACTION_FLAGS + COMMON_PROFILE_FLAGS

# Flags for `aptos account create-resource-account`
ACCOUNT_CREATE_RESOURCE_FLAGS = [
    ExpectedFlag(
        name="seed",
        long_form="--seed",
        description_contains="seed",
    ),
    ExpectedFlag(
        name="seed-encoding",
        long_form="--seed-encoding",
        description_contains="encoding",
    ),
    ExpectedFlag(
        name="authentication-key",
        long_form="--authentication-key",
        description_contains="authentication",
    ),
] + COMMON_TRANSACTION_FLAGS + COMMON_PROFILE_FLAGS

# Flags for `aptos account derive-resource-account-address`
ACCOUNT_DERIVE_RESOURCE_FLAGS = [
    ExpectedFlag(
        name="address",
        long_form="--address",
        description_contains="address",
    ),
    ExpectedFlag(
        name="seed",
        long_form="--seed",
        description_contains="seed",
    ),
    ExpectedFlag(
        name="seed-encoding",
        long_form="--seed-encoding",
        description_contains="encoding",
    ),
]

# Flags for `aptos account fund-with-faucet`
ACCOUNT_FUND_FLAGS = [
    ExpectedFlag(
        name="account",
        long_form="--account",
        description_contains="Address",
    ),
    ExpectedFlag(
        name="amount",
        long_form="--amount",
        description_contains="Octas",
    ),
    ExpectedFlag(
        name="faucet-url",
        long_form="--faucet-url",
        description_contains="faucet",
    ),
] + COMMON_REST_OPTIONS_FLAGS + COMMON_PROFILE_FLAGS

# Flags for `aptos account balance`
ACCOUNT_BALANCE_FLAGS = [
    ExpectedFlag(
        name="account",
        long_form="--account",
        description_contains="account",
    ),
    ExpectedFlag(
        name="coin-type",
        long_form="--coin-type",
        description_contains="Coin type",
    ),
] + COMMON_REST_OPTIONS_FLAGS + COMMON_PROFILE_FLAGS

# Flags for `aptos account list`
ACCOUNT_LIST_FLAGS = [
    ExpectedFlag(
        name="account",
        long_form="--account",
        description_contains="account",
    ),
    ExpectedFlag(
        name="query",
        long_form="--query",
        description_contains="list",
    ),
] + COMMON_REST_OPTIONS_FLAGS + COMMON_PROFILE_FLAGS

# Flags for `aptos account lookup-address`
ACCOUNT_LOOKUP_FLAGS = [
    ExpectedFlag(
        name="public-key",
        long_form="--public-key",
        description_contains="public key",
    ),
    ExpectedFlag(
        name="public-key-file",
        long_form="--public-key-file",
        description_contains="public key",
    ),
    ExpectedFlag(
        name="auth-key",
        long_form="--auth-key",
        description_contains="auth",
    ),
] + COMMON_REST_OPTIONS_FLAGS + COMMON_PROFILE_FLAGS

# Flags for `aptos account rotate-key`
ACCOUNT_ROTATE_KEY_FLAGS = [
    ExpectedFlag(
        name="new-private-key",
        long_form="--new-private-key",
        description_contains="private key",
    ),
    ExpectedFlag(
        name="new-private-key-file",
        long_form="--new-private-key-file",
        description_contains="private key",
    ),
    ExpectedFlag(
        name="save-to-profile",
        long_form="--save-to-profile",
        description_contains="profile",
    ),
    ExpectedFlag(
        name="skip-saving-profile",
        long_form="--skip-saving-profile",
        description_contains="profile",
    ),
] + COMMON_TRANSACTION_FLAGS + COMMON_PROFILE_FLAGS

# Flags for `aptos account transfer`
ACCOUNT_TRANSFER_FLAGS = [
    ExpectedFlag(
        name="account",
        long_form="--account",
        description_contains="Address",
    ),
    ExpectedFlag(
        name="amount",
        long_form="--amount",
        description_contains="Octas",
    ),
] + COMMON_TRANSACTION_FLAGS + COMMON_PROFILE_FLAGS


# =============================================================================
# Help text and flag availability tests (CLI regression testing)
# =============================================================================

@test_case
def test_account_help_subcommands(run_helper: RunHelper, test_name=None):
    """
    Test that the `aptos account` command lists all expected subcommands.
    
    This ensures no subcommands are accidentally removed or renamed.
    """
    help_text = run_help_command(run_helper, ["account"], test_name)
    
    expected_subcommands = [
        "create",
        "create-resource-account",
        "derive-resource-account-address",
        "fund-with-faucet",
        "balance",
        "list",
        "lookup-address",
        "rotate-key",
        "transfer",
    ]
    
    verify_subcommands_present(help_text, expected_subcommands)


@test_case
def test_account_create_flags(run_helper: RunHelper, test_name=None):
    """
    Test that `aptos account create` has all expected flags.
    
    This catches any accidental flag removal or renaming.
    """
    help_text = run_help_command(run_helper, ["account", "create"], test_name)
    
    verify_flags_present(help_text, ACCOUNT_CREATE_FLAGS)


@test_case
def test_account_create_resource_account_flags(run_helper: RunHelper, test_name=None):
    """
    Test that `aptos account create-resource-account` has all expected flags.
    """
    help_text = run_help_command(run_helper, ["account", "create-resource-account"], test_name)
    
    verify_flags_present(help_text, ACCOUNT_CREATE_RESOURCE_FLAGS)


@test_case
def test_account_derive_resource_account_flags(run_helper: RunHelper, test_name=None):
    """
    Test that `aptos account derive-resource-account-address` has all expected flags.
    """
    help_text = run_help_command(run_helper, ["account", "derive-resource-account-address"], test_name)
    
    verify_flags_present(help_text, ACCOUNT_DERIVE_RESOURCE_FLAGS)


@test_case
def test_account_fund_with_faucet_flags(run_helper: RunHelper, test_name=None):
    """
    Test that `aptos account fund-with-faucet` has all expected flags.
    """
    help_text = run_help_command(run_helper, ["account", "fund-with-faucet"], test_name)
    
    verify_flags_present(help_text, ACCOUNT_FUND_FLAGS)


@test_case
def test_account_balance_flags(run_helper: RunHelper, test_name=None):
    """
    Test that `aptos account balance` has all expected flags.
    """
    help_text = run_help_command(run_helper, ["account", "balance"], test_name)
    
    verify_flags_present(help_text, ACCOUNT_BALANCE_FLAGS)


@test_case
def test_account_list_flags(run_helper: RunHelper, test_name=None):
    """
    Test that `aptos account list` has all expected flags.
    """
    help_text = run_help_command(run_helper, ["account", "list"], test_name)
    
    verify_flags_present(help_text, ACCOUNT_LIST_FLAGS)


@test_case
def test_account_lookup_address_flags(run_helper: RunHelper, test_name=None):
    """
    Test that `aptos account lookup-address` has all expected flags.
    """
    help_text = run_help_command(run_helper, ["account", "lookup-address"], test_name)
    
    verify_flags_present(help_text, ACCOUNT_LOOKUP_FLAGS)


@test_case
def test_account_rotate_key_flags(run_helper: RunHelper, test_name=None):
    """
    Test that `aptos account rotate-key` has all expected flags.
    """
    help_text = run_help_command(run_helper, ["account", "rotate-key"], test_name)
    
    verify_flags_present(help_text, ACCOUNT_ROTATE_KEY_FLAGS)


@test_case
def test_account_transfer_flags(run_helper: RunHelper, test_name=None):
    """
    Test that `aptos account transfer` has all expected flags.
    """
    help_text = run_help_command(run_helper, ["account", "transfer"], test_name)
    
    verify_flags_present(help_text, ACCOUNT_TRANSFER_FLAGS)


# =============================================================================
# Functional E2E tests for account operations
# =============================================================================

@test_case
async def test_account_balance_command(run_helper: RunHelper, test_name=None):
    """
    Test that `aptos account balance` correctly returns the account balance.
    """
    # Get balance using CLI
    result = run_helper.run_command(
        test_name,
        [
            "aptos",
            "account",
            "balance",
            "--account",
            str(run_helper.get_account_info().account_address),
        ],
    )
    
    # Parse the result
    json_result = json.loads(result.stdout)
    
    if "Result" not in json_result:
        raise TestError(f"Expected 'Result' in balance output, got: {result.stdout}")
    
    balance_info = json_result["Result"]
    
    if not isinstance(balance_info, list):
        raise TestError(f"Expected balance result to be a list, got: {type(balance_info)}")
    
    # Verify we got balance information
    if len(balance_info) == 0:
        raise TestError("Expected at least one balance entry")
    
    # Check that balance has expected fields
    first_balance = balance_info[0]
    if "balance" not in first_balance:
        raise TestError(f"Expected 'balance' field in result, got: {first_balance}")


@test_case
def test_account_balance_with_coin_type(run_helper: RunHelper, test_name=None):
    """
    Test that `aptos account balance` works with explicit coin type.
    """
    result = run_helper.run_command(
        test_name,
        [
            "aptos",
            "account",
            "balance",
            "--account",
            str(run_helper.get_account_info().account_address),
            "--coin-type",
            "0x1::aptos_coin::AptosCoin",
        ],
    )
    
    json_result = json.loads(result.stdout)
    
    if "Result" not in json_result:
        raise TestError(f"Expected 'Result' in balance output with coin type, got: {result.stdout}")


@test_case
def test_account_list_resources(run_helper: RunHelper, test_name=None):
    """
    Test that `aptos account list --query=resources` returns account resources.
    """
    result = run_helper.run_command(
        test_name,
        [
            "aptos",
            "account",
            "list",
            "--account",
            str(run_helper.get_account_info().account_address),
            "--query",
            "resources",
        ],
    )
    
    json_result = json.loads(result.stdout)
    
    if "Result" not in json_result:
        raise TestError(f"Expected 'Result' in list output, got: {result.stdout}")
    
    # Verify we have account resource
    found_account = False
    for module in json_result["Result"]:
        if "0x1::account::Account" in module:
            found_account = True
            break
    
    if not found_account:
        raise TestError("Expected to find 0x1::account::Account in resources list")


@test_case
def test_account_list_modules(run_helper: RunHelper, test_name=None):
    """
    Test that `aptos account list --query=modules` works (may return empty for new accounts).
    """
    result = run_helper.run_command(
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
    
    json_result = json.loads(result.stdout)
    
    if "Result" not in json_result:
        raise TestError(f"Expected 'Result' in modules list output, got: {result.stdout}")


@test_case
def test_account_list_balance_query(run_helper: RunHelper, test_name=None):
    """
    Test that `aptos account list --query=balance` returns balance information.
    """
    result = run_helper.run_command(
        test_name,
        [
            "aptos",
            "account",
            "list",
            "--account",
            str(run_helper.get_account_info().account_address),
            "--query",
            "balance",
        ],
    )
    
    json_result = json.loads(result.stdout)
    
    if "Result" not in json_result:
        raise TestError(f"Expected 'Result' in balance query output, got: {result.stdout}")


@test_case
async def test_account_transfer_coins(run_helper: RunHelper, test_name=None):
    """
    Test transferring coins using `aptos account transfer`.
    """
    transfer_amount = 500
    
    # Get initial balance
    initial_balance = int(
        await run_helper.api_client.account_balance(OTHER_ACCOUNT_ONE.account_address)
    )
    
    # Transfer coins
    run_helper.run_command(
        test_name,
        [
            "aptos",
            "account",
            "transfer",
            "--account",
            str(OTHER_ACCOUNT_ONE.account_address),
            "--amount",
            str(transfer_amount),
            "--assume-yes",
        ],
    )
    
    # Verify the transfer
    new_balance = int(
        await run_helper.api_client.account_balance(OTHER_ACCOUNT_ONE.account_address)
    )
    
    expected_balance = initial_balance + transfer_amount
    if new_balance != expected_balance:
        raise TestError(
            f"Expected balance {expected_balance} after transfer, got {new_balance}"
        )


@test_case
async def test_account_fund_with_faucet_explicit(run_helper: RunHelper, test_name=None):
    """
    Test funding an account with explicit amount using `aptos account fund-with-faucet`.
    """
    fund_amount = 50000000  # 0.5 APT
    
    old_balance = int(
        await run_helper.api_client.account_balance(
            run_helper.get_account_info().account_address
        )
    )
    
    # Fund the account
    run_helper.run_command(
        test_name,
        [
            "aptos",
            "account",
            "fund-with-faucet",
            "--account",
            str(run_helper.get_account_info().account_address),
            "--amount",
            str(fund_amount),
        ],
    )
    
    # Verify the balance increased
    new_balance = int(
        await run_helper.api_client.account_balance(
            run_helper.get_account_info().account_address
        )
    )
    
    expected_balance = old_balance + fund_amount
    if new_balance != expected_balance:
        raise TestError(
            f"Expected balance {expected_balance} after faucet fund, got {new_balance}"
        )


@test_case
def test_account_lookup_address_with_auth_key(run_helper: RunHelper, test_name=None):
    """
    Test looking up an address by auth key using `aptos account lookup-address`.
    """
    result = run_helper.run_command(
        test_name,
        [
            "aptos",
            "account",
            "lookup-address",
            "--auth-key",
            str(run_helper.get_account_info().account_address),
        ],
    )
    
    # Verify the result contains the expected address
    account_address = str(run_helper.get_account_info().account_address)
    # Remove the 0x prefix for comparison since result might not have it
    if account_address[2:] not in result.stdout and account_address not in result.stdout:
        raise TestError(
            f"lookup-address result does not match expected address. "
            f"Expected to find {account_address} in {result.stdout}"
        )


@test_case
def test_account_lookup_address_with_public_key(run_helper: RunHelper, test_name=None):
    """
    Test looking up an address by public key using `aptos account lookup-address`.
    """
    account_info = run_helper.get_account_info()
    
    result = run_helper.run_command(
        test_name,
        [
            "aptos",
            "account",
            "lookup-address",
            f"--public-key={account_info.public_key}",
        ],
    )
    
    json_result = json.loads(result.stdout)
    
    if "Result" not in json_result:
        raise TestError(f"Expected 'Result' in lookup-address output, got: {result.stdout}")


@test_case
def test_account_derive_resource_account_address(run_helper: RunHelper, test_name=None):
    """
    Test deriving a resource account address using `aptos account derive-resource-account-address`.
    """
    seed = "test_seed_123"
    sender_address = str(run_helper.get_account_info().account_address)
    
    # Derive the resource account address
    result = run_helper.run_command(
        test_name,
        [
            "aptos",
            "account",
            "derive-resource-account-address",
            "--address",
            sender_address,
            "--seed",
            seed,
        ],
    )
    
    json_result = json.loads(result.stdout)
    
    if "Result" not in json_result:
        raise TestError(
            f"Expected 'Result' in derive-resource-account-address output, got: {result.stdout}"
        )
    
    # Verify we got a valid address (64 hex characters without 0x prefix)
    derived_address = json_result["Result"]
    if not isinstance(derived_address, str) or len(derived_address) != 64:
        raise TestError(
            f"Expected 64-character hex address, got: {derived_address}"
        )


@test_case
def test_account_derive_resource_account_with_utf8_encoding(run_helper: RunHelper, test_name=None):
    """
    Test deriving a resource account address with UTF-8 seed encoding.
    """
    seed = "utf8_seed"
    sender_address = str(run_helper.get_account_info().account_address)
    
    result = run_helper.run_command(
        test_name,
        [
            "aptos",
            "account",
            "derive-resource-account-address",
            "--address",
            sender_address,
            "--seed",
            seed,
            "--seed-encoding",
            "utf8",
        ],
    )
    
    json_result = json.loads(result.stdout)
    
    if "Result" not in json_result:
        raise TestError(
            f"Expected 'Result' with utf8 encoding, got: {result.stdout}"
        )


@test_case
def test_account_derive_resource_account_with_hex_encoding(run_helper: RunHelper, test_name=None):
    """
    Test deriving a resource account address with hex seed encoding.
    """
    seed = "0x616263"  # "abc" in hex
    sender_address = str(run_helper.get_account_info().account_address)
    
    result = run_helper.run_command(
        test_name,
        [
            "aptos",
            "account",
            "derive-resource-account-address",
            "--address",
            sender_address,
            "--seed",
            seed,
            "--seed-encoding",
            "hex",
        ],
    )
    
    json_result = json.loads(result.stdout)
    
    if "Result" not in json_result:
        raise TestError(
            f"Expected 'Result' with hex encoding, got: {result.stdout}"
        )


@test_case
def test_account_create_resource_account(run_helper: RunHelper, test_name=None):
    """
    Test creating a resource account using `aptos account create-resource-account`.
    """
    # Use a unique seed to avoid conflicts
    seed = f"resource_test_{secrets.token_hex(8)}"
    
    result = run_helper.run_command(
        test_name,
        [
            "aptos",
            "account",
            "create-resource-account",
            "--seed",
            seed,
            "--assume-yes",
        ],
    )
    
    json_result = json.loads(result.stdout)
    
    if "Result" not in json_result:
        raise TestError(
            f"Expected 'Result' in create-resource-account output, got: {result.stdout}"
        )
    
    # Verify we got resource account address
    result_data = json_result["Result"]
    if "resource_account" not in result_data:
        raise TestError(
            f"Expected 'resource_account' in result, got: {result_data}"
        )
    
    if result_data.get("resource_account") is None:
        raise TestError("resource_account should not be None in successful creation")


@test_case
def test_account_rotate_key_with_skip_profile(run_helper: RunHelper, test_name=None):
    """
    Test key rotation with skip-saving-profile flag.
    
    Note: This test should be run last as it changes the key.
    """
    # Generate a new private key
    new_private_key = secrets.token_hex(32)
    
    # Store original profile info
    old_profile = run_helper.get_account_info()
    
    # Rotate the key
    result = run_helper.run_command(
        test_name,
        [
            "aptos",
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
            f"Key rotation failed. Expected success=true in output: {result.stdout}"
        )
    
    # Verify account address hasn't changed
    new_profile = run_helper.get_account_info()
    if old_profile.account_address != new_profile.account_address:
        raise TestError(
            f"Account address changed after rotate-key: "
            f"{old_profile.account_address} -> {new_profile.account_address}"
        )


# =============================================================================
# Error handling tests
# =============================================================================

@test_case
def test_account_balance_invalid_address(run_helper: RunHelper, test_name=None):
    """
    Test that balance command properly handles invalid addresses.
    """
    try:
        run_helper.run_command(
            test_name,
            [
                "aptos",
                "account",
                "balance",
                "--account",
                "invalid_address",
            ],
        )
        raise TestError("Expected command to fail with invalid address")
    except subprocess.CalledProcessError:
        # Expected to fail - this is the correct behavior
        pass


@test_case
def test_account_transfer_missing_amount(run_helper: RunHelper, test_name=None):
    """
    Test that transfer command requires amount flag.
    """
    try:
        run_helper.run_command(
            test_name,
            [
                "aptos",
                "account",
                "transfer",
                "--account",
                str(OTHER_ACCOUNT_ONE.account_address),
                "--assume-yes",
            ],
        )
        raise TestError("Expected command to fail without amount flag")
    except subprocess.CalledProcessError:
        # Expected to fail - amount is required
        pass


@test_case  
def test_account_create_resource_account_missing_seed(run_helper: RunHelper, test_name=None):
    """
    Test that create-resource-account requires seed flag.
    """
    try:
        run_helper.run_command(
            test_name,
            [
                "aptos",
                "account",
                "create-resource-account",
                "--assume-yes",
            ],
        )
        raise TestError("Expected command to fail without seed flag")
    except subprocess.CalledProcessError:
        # Expected to fail - seed is required
        pass


# =============================================================================
# JSON output format tests
# =============================================================================

@test_case
def test_account_balance_json_format(run_helper: RunHelper, test_name=None):
    """
    Test that balance output is valid JSON.
    """
    result = run_helper.run_command(
        test_name,
        [
            "aptos",
            "account",
            "balance",
            "--account",
            str(run_helper.get_account_info().account_address),
        ],
    )
    
    try:
        json_result = json.loads(result.stdout)
    except json.JSONDecodeError as e:
        raise TestError(f"Balance output is not valid JSON: {e}")
    
    # Verify expected structure
    if not isinstance(json_result, dict):
        raise TestError(f"Expected JSON object, got {type(json_result)}")
    
    if "Result" not in json_result:
        raise TestError(f"Expected 'Result' key in JSON output")


@test_case
def test_account_list_json_format(run_helper: RunHelper, test_name=None):
    """
    Test that list output is valid JSON.
    """
    result = run_helper.run_command(
        test_name,
        [
            "aptos",
            "account",
            "list",
            "--account",
            str(run_helper.get_account_info().account_address),
        ],
    )
    
    try:
        json_result = json.loads(result.stdout)
    except json.JSONDecodeError as e:
        raise TestError(f"List output is not valid JSON: {e}")
    
    if "Result" not in json_result:
        raise TestError(f"Expected 'Result' key in list JSON output")
