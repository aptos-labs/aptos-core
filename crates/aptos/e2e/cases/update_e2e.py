# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

"""
Comprehensive E2E tests for the `aptos update` CLI subcommand.

This module provides comprehensive testing for:
1. CLI flag availability and regression testing
2. Help text verification
3. Functional tests for update operations

Note: Actual update functionality is not tested to avoid modifying system state.
"""

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
# Expected flags for each update subcommand
# =============================================================================

UPDATE_APTOS_FLAGS = [
    ExpectedFlag(
        name="check",
        long_form="--check",
        description_contains="check",
    ),
]

UPDATE_REVELA_FLAGS = [
    ExpectedFlag(
        name="check",
        long_form="--check",
        description_contains="check",
    ),
]

UPDATE_MOVEFMT_FLAGS = [
    ExpectedFlag(
        name="check",
        long_form="--check",
        description_contains="check",
    ),
]

UPDATE_PROVER_DEPENDENCIES_FLAGS = [
    ExpectedFlag(
        name="check",
        long_form="--check",
        description_contains="check",
    ),
]


# =============================================================================
# Help text and flag availability tests (CLI regression testing)
# =============================================================================

@test_case
def test_update_help_subcommands(run_helper: RunHelper, test_name=None):
    """
    Test that the `aptos update` command lists all expected subcommands.
    """
    help_text = run_help_command(run_helper, ["update"], test_name)
    
    expected_subcommands = [
        "aptos",
        "revela",
        "movefmt",
        "move-mutation-test",
        "prover-dependencies",
    ]
    
    verify_subcommands_present(help_text, expected_subcommands)


@test_case
def test_update_aptos_flags(run_helper: RunHelper, test_name=None):
    """
    Test that `aptos update aptos` has all expected flags.
    """
    help_text = run_help_command(run_helper, ["update", "aptos"], test_name)
    
    verify_flags_present(help_text, UPDATE_APTOS_FLAGS)


@test_case
def test_update_revela_flags(run_helper: RunHelper, test_name=None):
    """
    Test that `aptos update revela` has all expected flags.
    """
    help_text = run_help_command(run_helper, ["update", "revela"], test_name)
    
    verify_flags_present(help_text, UPDATE_REVELA_FLAGS)


@test_case
def test_update_movefmt_flags(run_helper: RunHelper, test_name=None):
    """
    Test that `aptos update movefmt` has all expected flags.
    """
    help_text = run_help_command(run_helper, ["update", "movefmt"], test_name)
    
    verify_flags_present(help_text, UPDATE_MOVEFMT_FLAGS)


@test_case
def test_update_move_mutation_test_flags(run_helper: RunHelper, test_name=None):
    """
    Test that `aptos update move-mutation-test` has all expected flags.
    """
    help_text = run_help_command(run_helper, ["update", "move-mutation-test"], test_name)
    
    # move-mutation-test should also have --check flag
    expected_flags = [
        ExpectedFlag(
            name="check",
            long_form="--check",
            description_contains="check",
        ),
    ]
    
    verify_flags_present(help_text, expected_flags)


@test_case
def test_update_prover_dependencies_flags(run_helper: RunHelper, test_name=None):
    """
    Test that `aptos update prover-dependencies` has all expected flags.
    """
    help_text = run_help_command(run_helper, ["update", "prover-dependencies"], test_name)
    
    verify_flags_present(help_text, UPDATE_PROVER_DEPENDENCIES_FLAGS)


# =============================================================================
# Functional E2E tests for update operations (check only, no actual updates)
# =============================================================================

@test_case
def test_update_aptos_check(run_helper: RunHelper, test_name=None):
    """
    Test that `aptos update aptos --check` returns version info.
    
    Note: This only checks for updates, does not perform actual update.
    """
    result = run_helper.run_command(
        test_name,
        ["aptos", "update", "aptos", "--check"],
    )
    
    # Should return some version information
    output = result.stdout.lower()
    if "up to date" not in output and "available" not in output and "result" not in output:
        raise TestError(f"Expected version info in check output, got: {result.stdout}")


@test_case
def test_update_revela_check(run_helper: RunHelper, test_name=None):
    """
    Test that `aptos update revela --check` returns version info.
    
    Note: This only checks for updates, does not perform actual update.
    """
    result = run_helper.run_command(
        test_name,
        ["aptos", "update", "revela", "--check"],
    )
    
    # Should return some version information or status
    output = result.stdout.lower()
    if "up to date" not in output and "available" not in output and "result" not in output:
        raise TestError(f"Expected version info in check output, got: {result.stdout}")


@test_case
def test_update_movefmt_check(run_helper: RunHelper, test_name=None):
    """
    Test that `aptos update movefmt --check` returns version info.
    
    Note: This only checks for updates, does not perform actual update.
    """
    result = run_helper.run_command(
        test_name,
        ["aptos", "update", "movefmt", "--check"],
    )
    
    # Should return some version information or status
    output = result.stdout.lower()
    if "up to date" not in output and "available" not in output and "result" not in output:
        raise TestError(f"Expected version info in check output, got: {result.stdout}")
