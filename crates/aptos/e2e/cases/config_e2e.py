# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

"""
Comprehensive E2E tests for the `aptos config` CLI subcommand.

This module provides comprehensive testing for:
1. CLI flag availability and regression testing
2. Help text verification
3. Functional tests for config operations
"""

import json
import subprocess

from cases.cli_flag_helpers import (
    ExpectedFlag,
    run_help_command,
    verify_flags_present,
    verify_subcommands_present,
)
from common import TestError
from test_helpers import RunHelper
from test_results import test_case


# =============================================================================
# Expected flags for each config subcommand
# =============================================================================

CONFIG_GENERATE_SHELL_COMPLETIONS_FLAGS = [
    ExpectedFlag(
        name="shell",
        long_form="--shell",
        description_contains="Shell",
    ),
    ExpectedFlag(
        name="output-file",
        long_form="--output-file",
        description_contains="output",
    ),
]

CONFIG_SET_GLOBAL_CONFIG_FLAGS = [
    ExpectedFlag(
        name="config-type",
        long_form="--config-type",
        description_contains="config",
    ),
    ExpectedFlag(
        name="default-prompt-response",
        long_form="--default-prompt-response",
        description_contains="prompt",
    ),
]

CONFIG_SHOW_PROFILES_FLAGS = [
    ExpectedFlag(
        name="profile",
        long_form="--profile",
        description_contains="profile",
    ),
]

CONFIG_SHOW_PRIVATE_KEY_FLAGS = [
    ExpectedFlag(
        name="profile",
        long_form="--profile",
        description_contains="profile",
    ),
]

CONFIG_DELETE_PROFILE_FLAGS = [
    ExpectedFlag(
        name="profile",
        long_form="--profile",
        description_contains="profile",
    ),
]

CONFIG_RENAME_PROFILE_FLAGS = [
    ExpectedFlag(
        name="profile",
        long_form="--profile",
        description_contains="profile",
    ),
    ExpectedFlag(
        name="new-profile-name",
        long_form="--new-profile-name",
        description_contains="new",
    ),
]


# =============================================================================
# Help text and flag availability tests (CLI regression testing)
# =============================================================================

@test_case
def test_config_help_subcommands(run_helper: RunHelper, test_name=None):
    """
    Test that the `aptos config` command lists all expected subcommands.
    """
    help_text = run_help_command(run_helper, ["config"], test_name)
    
    expected_subcommands = [
        "generate-shell-completions",
        "show-global-config",
        "set-global-config",
        "show-profiles",
        "show-private-key",
        "rename-profile",
        "delete-profile",
    ]
    
    verify_subcommands_present(help_text, expected_subcommands)


@test_case
def test_config_generate_shell_completions_flags(run_helper: RunHelper, test_name=None):
    """
    Test that `aptos config generate-shell-completions` has all expected flags.
    """
    help_text = run_help_command(run_helper, ["config", "generate-shell-completions"], test_name)
    
    verify_flags_present(help_text, CONFIG_GENERATE_SHELL_COMPLETIONS_FLAGS)


@test_case
def test_config_set_global_config_flags(run_helper: RunHelper, test_name=None):
    """
    Test that `aptos config set-global-config` has all expected flags.
    """
    help_text = run_help_command(run_helper, ["config", "set-global-config"], test_name)
    
    verify_flags_present(help_text, CONFIG_SET_GLOBAL_CONFIG_FLAGS)


@test_case
def test_config_show_profiles_flags(run_helper: RunHelper, test_name=None):
    """
    Test that `aptos config show-profiles` has all expected flags.
    """
    help_text = run_help_command(run_helper, ["config", "show-profiles"], test_name)
    
    verify_flags_present(help_text, CONFIG_SHOW_PROFILES_FLAGS)


@test_case
def test_config_show_private_key_flags(run_helper: RunHelper, test_name=None):
    """
    Test that `aptos config show-private-key` has all expected flags.
    """
    help_text = run_help_command(run_helper, ["config", "show-private-key"], test_name)
    
    verify_flags_present(help_text, CONFIG_SHOW_PRIVATE_KEY_FLAGS)


@test_case
def test_config_delete_profile_flags(run_helper: RunHelper, test_name=None):
    """
    Test that `aptos config delete-profile` has all expected flags.
    """
    help_text = run_help_command(run_helper, ["config", "delete-profile"], test_name)
    
    verify_flags_present(help_text, CONFIG_DELETE_PROFILE_FLAGS)


@test_case
def test_config_rename_profile_flags(run_helper: RunHelper, test_name=None):
    """
    Test that `aptos config rename-profile` has all expected flags.
    """
    help_text = run_help_command(run_helper, ["config", "rename-profile"], test_name)
    
    verify_flags_present(help_text, CONFIG_RENAME_PROFILE_FLAGS)


# =============================================================================
# Functional E2E tests for config operations
# =============================================================================

@test_case
def test_config_show_global_config(run_helper: RunHelper, test_name=None):
    """
    Test that `aptos config show-global-config` returns valid configuration.
    """
    result = run_helper.run_command(
        test_name,
        ["aptos", "config", "show-global-config"],
    )
    
    json_result = json.loads(result.stdout)
    
    if "Result" not in json_result:
        raise TestError(f"Expected 'Result' in show-global-config output, got: {result.stdout}")
    
    config = json_result["Result"]
    
    # Verify expected fields
    if "config_type" not in config:
        raise TestError(f"Expected 'config_type' in config, got: {config}")


@test_case
def test_config_show_profiles_default(run_helper: RunHelper, test_name=None):
    """
    Test that `aptos config show-profiles` returns the default profile.
    """
    result = run_helper.run_command(
        test_name,
        ["aptos", "config", "show-profiles"],
    )
    
    json_result = json.loads(result.stdout)
    
    if "Result" not in json_result:
        raise TestError(f"Expected 'Result' in show-profiles output, got: {result.stdout}")
    
    profiles = json_result["Result"]
    
    # Check that default profile exists
    if "default" not in profiles:
        raise TestError(f"Expected 'default' profile, got profiles: {list(profiles.keys())}")
    
    # Verify default profile has expected structure
    default_profile = profiles["default"]
    if "has_private_key" not in default_profile:
        raise TestError(f"Expected 'has_private_key' in profile, got: {default_profile}")


@test_case
def test_config_show_profiles_specific(run_helper: RunHelper, test_name=None):
    """
    Test that `aptos config show-profiles --profile default` returns specific profile.
    """
    result = run_helper.run_command(
        test_name,
        ["aptos", "config", "show-profiles", "--profile", "default"],
    )
    
    json_result = json.loads(result.stdout)
    
    if "Result" not in json_result:
        raise TestError(f"Expected 'Result' in show-profiles output, got: {result.stdout}")


@test_case
def test_config_show_private_key_default(run_helper: RunHelper, test_name=None):
    """
    Test that `aptos config show-private-key --profile default` returns the private key.
    """
    result = run_helper.run_command(
        test_name,
        ["aptos", "config", "show-private-key", "--profile", "default"],
    )
    
    json_result = json.loads(result.stdout)
    
    if "Result" not in json_result:
        raise TestError(f"Expected 'Result' in show-private-key output, got: {result.stdout}")
    
    # Verify the result is a hex string (private key)
    private_key = json_result["Result"]
    if not isinstance(private_key, str):
        raise TestError(f"Expected private key string, got: {type(private_key)}")


# =============================================================================
# Error handling tests
# =============================================================================

@test_case
def test_config_show_private_key_nonexistent_profile(run_helper: RunHelper, test_name=None):
    """
    Test that show-private-key fails for nonexistent profile.
    """
    try:
        run_helper.run_command(
            test_name,
            ["aptos", "config", "show-private-key", "--profile", "nonexistent_profile_xyz"],
        )
        raise TestError("Expected command to fail for nonexistent profile")
    except subprocess.CalledProcessError:
        # Expected to fail
        pass


@test_case
def test_config_delete_profile_nonexistent(run_helper: RunHelper, test_name=None):
    """
    Test that delete-profile fails for nonexistent profile.
    """
    try:
        run_helper.run_command(
            test_name,
            ["aptos", "config", "delete-profile", "--profile", "nonexistent_profile_xyz"],
        )
        raise TestError("Expected command to fail for nonexistent profile")
    except subprocess.CalledProcessError:
        # Expected to fail
        pass


@test_case
def test_config_rename_profile_nonexistent(run_helper: RunHelper, test_name=None):
    """
    Test that rename-profile fails for nonexistent profile.
    """
    try:
        run_helper.run_command(
            test_name,
            [
                "aptos", "config", "rename-profile",
                "--profile", "nonexistent_profile_xyz",
                "--new-profile-name", "new_name",
            ],
        )
        raise TestError("Expected command to fail for nonexistent profile")
    except subprocess.CalledProcessError:
        # Expected to fail
        pass
