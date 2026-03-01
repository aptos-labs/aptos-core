# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

"""
Comprehensive E2E tests for the `aptos init` CLI command.

This module provides comprehensive testing for:
1. CLI flag availability and regression testing
2. Help text verification
3. Functional tests for init operations
"""

import json
import os

from cases.cli_flag_helpers import (
    ExpectedFlag,
    run_help_command,
    verify_flags_present,
)
from common import TestError
from test_helpers import RunHelper
from test_results import test_case


# =============================================================================
# Expected flags for the init command
# =============================================================================

INIT_FLAGS = [
    ExpectedFlag(
        name="network",
        long_form="--network",
        description_contains="Network",
    ),
    ExpectedFlag(
        name="rest-url",
        long_form="--rest-url",
        description_contains="URL",
    ),
    ExpectedFlag(
        name="faucet-url",
        long_form="--faucet-url",
        description_contains="faucet",
    ),
    ExpectedFlag(
        name="skip-faucet",
        long_form="--skip-faucet",
        description_contains="faucet",
    ),
    ExpectedFlag(
        name="ledger",
        long_form="--ledger",
        description_contains="ledger",
    ),
    ExpectedFlag(
        name="profile",
        long_form="--profile",
        description_contains="profile",
    ),
    ExpectedFlag(
        name="assume-yes",
        long_form="--assume-yes",
        description_contains="yes",
    ),
    ExpectedFlag(
        name="assume-no",
        long_form="--assume-no",
        description_contains="no",
    ),
    ExpectedFlag(
        name="private-key",
        long_form="--private-key",
        description_contains="private",
    ),
    ExpectedFlag(
        name="private-key-file",
        long_form="--private-key-file",
        description_contains="private",
    ),
]


# =============================================================================
# Help text and flag availability tests (CLI regression testing)
# =============================================================================

@test_case
def test_init_help(run_helper: RunHelper, test_name=None):
    """
    Test that the `aptos init` command help is available and shows expected content.
    """
    help_text = run_help_command(run_helper, ["init"], test_name)
    
    # Verify help text contains expected content
    if "initialize" not in help_text.lower() and "init" not in help_text.lower():
        raise TestError(f"Expected 'initialize' or 'init' in init help text, got: {help_text}")


@test_case
def test_init_flags(run_helper: RunHelper, test_name=None):
    """
    Test that `aptos init` has all expected flags.
    
    This catches any accidental flag removal or renaming.
    """
    help_text = run_help_command(run_helper, ["init"], test_name)
    
    verify_flags_present(help_text, INIT_FLAGS)


@test_case
def test_init_network_options_in_help(run_helper: RunHelper, test_name=None):
    """
    Test that init help text shows available network options.
    """
    help_text = run_help_command(run_helper, ["init"], test_name)
    
    # Should mention network options
    expected_networks = ["devnet", "testnet", "mainnet", "local", "custom"]
    for network in expected_networks:
        if network not in help_text.lower():
            raise TestError(f"Expected network option '{network}' in init help text")


# =============================================================================
# Functional E2E tests for init command
# =============================================================================

@test_case
def test_init_creates_config(run_helper: RunHelper, test_name=None):
    """
    Test that init creates the .aptos config directory and config.yaml file.
    
    Note: This test relies on the fact that test_init has already been run
    which creates the initial profile.
    """
    config_path = os.path.join(
        run_helper.host_working_directory, ".aptos", "config.yaml"
    )
    
    if not os.path.exists(config_path):
        raise TestError(f"Config file {config_path} should exist after init")


@test_case
def test_init_config_has_profile(run_helper: RunHelper, test_name=None):
    """
    Test that the config file created by init contains a profile.
    """
    account_info = run_helper.get_account_info()
    
    if not account_info:
        raise TestError("Config should contain account info after init")
    
    if not account_info.account_address:
        raise TestError("Profile should have an account address")
    
    if not account_info.public_key:
        raise TestError("Profile should have a public key")
    
    if not account_info.private_key:
        raise TestError("Profile should have a private key")


@test_case
def test_init_config_has_network(run_helper: RunHelper, test_name=None):
    """
    Test that the config contains network information.
    """
    account_info = run_helper.get_account_info()
    
    if not account_info:
        raise TestError("Config should contain account info")
    
    # The network should be set (we initialized with --network local)
    if not account_info.network:
        raise TestError("Profile should have network set")


@test_case
def test_init_with_named_profile(run_helper: RunHelper, test_name=None):
    """
    Test that init can create a named profile.
    """
    profile_name = "test_profile_e2e"
    
    # Create a new profile
    run_helper.run_command(
        test_name,
        [
            "aptos", "init",
            "--assume-yes",
            "--network", "local",
            "--profile", profile_name,
        ],
        input="\n",
    )
    
    # Verify the profile exists by showing profiles
    result = run_helper.run_command(
        test_name,
        ["aptos", "config", "show-profiles"],
    )
    
    json_result = json.loads(result.stdout)
    
    if "Result" not in json_result:
        raise TestError(f"Expected 'Result' in show-profiles output")
    
    profiles = json_result["Result"]
    
    if profile_name not in profiles:
        raise TestError(f"Expected profile '{profile_name}' to exist after init")


@test_case
def test_init_with_skip_faucet(run_helper: RunHelper, test_name=None):
    """
    Test that init with --skip-faucet creates a profile without funding.
    """
    profile_name = "test_skip_faucet_profile"
    
    # Create a new profile with --skip-faucet
    run_helper.run_command(
        test_name,
        [
            "aptos", "init",
            "--assume-yes",
            "--network", "local",
            "--profile", profile_name,
            "--skip-faucet",
        ],
        input="\n",
    )
    
    # Verify the profile was created
    result = run_helper.run_command(
        test_name,
        ["aptos", "config", "show-profiles", "--profile", profile_name],
    )
    
    json_result = json.loads(result.stdout)
    
    if "Result" not in json_result:
        raise TestError(f"Expected 'Result' in show-profiles output")
    
    profiles = json_result["Result"]
    
    if profile_name not in profiles:
        raise TestError(f"Expected profile '{profile_name}' to exist after init with --skip-faucet")


@test_case
def test_init_with_private_key(run_helper: RunHelper, test_name=None):
    """
    Test that init can use a provided private key.
    """
    import secrets
    
    profile_name = "test_private_key_profile"
    # Generate a random private key (64 hex characters)
    private_key = "0x" + secrets.token_hex(32)
    
    # Create a new profile with the private key
    run_helper.run_command(
        test_name,
        [
            "aptos", "init",
            "--assume-yes",
            "--network", "local",
            "--profile", profile_name,
            "--private-key", private_key,
            "--skip-faucet",
        ],
    )
    
    # Verify the profile was created
    result = run_helper.run_command(
        test_name,
        ["aptos", "config", "show-profiles", "--profile", profile_name],
    )
    
    json_result = json.loads(result.stdout)
    
    if "Result" not in json_result:
        raise TestError(f"Expected 'Result' in show-profiles output")
    
    profiles = json_result["Result"]
    
    if profile_name not in profiles:
        raise TestError(f"Expected profile '{profile_name}' to exist after init with private key")
    
    # Verify the profile has a private key
    profile = profiles[profile_name]
    if not profile.get("has_private_key"):
        raise TestError(f"Expected profile to have private key")


# =============================================================================
# JSON output format tests
# =============================================================================

@test_case
def test_init_profiles_json_format(run_helper: RunHelper, test_name=None):
    """
    Test that profiles created by init can be shown as valid JSON.
    """
    result = run_helper.run_command(
        test_name,
        ["aptos", "config", "show-profiles"],
    )
    
    try:
        json_result = json.loads(result.stdout)
    except json.JSONDecodeError as e:
        raise TestError(f"show-profiles output is not valid JSON: {e}")
    
    if not isinstance(json_result, dict):
        raise TestError(f"Expected JSON object, got {type(json_result)}")
    
    if "Result" not in json_result:
        raise TestError("Expected 'Result' key in JSON output")
