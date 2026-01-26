# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

"""
Comprehensive E2E tests for the `aptos key` CLI subcommand.

This module provides comprehensive testing for:
1. CLI flag availability and regression testing
2. Help text verification
3. Functional tests for key operations
"""

import json
import os
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
# Expected flags for each key subcommand
# =============================================================================

KEY_GENERATE_FLAGS = [
    ExpectedFlag(
        name="key-type",
        long_form="--key-type",
        description_contains="type",
    ),
    ExpectedFlag(
        name="output-file",
        long_form="--output-file",
        description_contains="output",
    ),
    ExpectedFlag(
        name="assume-yes",
        long_form="--assume-yes",
        description_contains="yes",
    ),
    ExpectedFlag(
        name="vanity-prefix",
        long_form="--vanity-prefix",
        description_contains="vanity",
    ),
]

KEY_EXTRACT_PUBLIC_KEY_FLAGS = [
    ExpectedFlag(
        name="key-type",
        long_form="--key-type",
        description_contains="type",
    ),
    ExpectedFlag(
        name="output-file",
        long_form="--output-file",
        description_contains="output",
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

KEY_EXTRACT_PEER_FLAGS = [
    ExpectedFlag(
        name="host",
        long_form="--host",
        description_contains="host",
    ),
    ExpectedFlag(
        name="output-file",
        long_form="--output-file",
        description_contains="output",
    ),
]


# =============================================================================
# Help text and flag availability tests (CLI regression testing)
# =============================================================================

@test_case
def test_key_help_subcommands(run_helper: RunHelper, test_name=None):
    """
    Test that the `aptos key` command lists all expected subcommands.
    """
    help_text = run_help_command(run_helper, ["key"], test_name)
    
    expected_subcommands = [
        "generate",
        "extract-public-key",
        "extract-peer",
    ]
    
    verify_subcommands_present(help_text, expected_subcommands)


@test_case
def test_key_generate_flags(run_helper: RunHelper, test_name=None):
    """
    Test that `aptos key generate` has all expected flags.
    """
    help_text = run_help_command(run_helper, ["key", "generate"], test_name)
    
    verify_flags_present(help_text, KEY_GENERATE_FLAGS)


@test_case
def test_key_extract_public_key_flags(run_helper: RunHelper, test_name=None):
    """
    Test that `aptos key extract-public-key` has all expected flags.
    """
    help_text = run_help_command(run_helper, ["key", "extract-public-key"], test_name)
    
    verify_flags_present(help_text, KEY_EXTRACT_PUBLIC_KEY_FLAGS)


@test_case
def test_key_extract_peer_flags(run_helper: RunHelper, test_name=None):
    """
    Test that `aptos key extract-peer` has all expected flags.
    """
    help_text = run_help_command(run_helper, ["key", "extract-peer"], test_name)
    
    verify_flags_present(help_text, KEY_EXTRACT_PEER_FLAGS)


# =============================================================================
# Functional E2E tests for key operations
# =============================================================================

@test_case
def test_key_generate_ed25519(run_helper: RunHelper, test_name=None):
    """
    Test that `aptos key generate` creates an ed25519 key pair.
    """
    key_file = os.path.join(run_helper.host_working_directory, "test_ed25519_key")
    
    result = run_helper.run_command(
        test_name,
        [
            "aptos", "key", "generate",
            "--key-type", "ed25519",
            "--output-file", key_file,
            "--assume-yes",
        ],
    )
    
    json_result = json.loads(result.stdout)
    
    if "Result" not in json_result:
        raise TestError(f"Expected 'Result' in generate output, got: {result.stdout}")
    
    # Verify files were created
    result_paths = json_result["Result"]
    if "PrivateKey Path" not in result_paths:
        raise TestError(f"Expected 'PrivateKey Path' in result, got: {result_paths}")
    if "PublicKey Path" not in result_paths:
        raise TestError(f"Expected 'PublicKey Path' in result, got: {result_paths}")


@test_case
def test_key_generate_x25519(run_helper: RunHelper, test_name=None):
    """
    Test that `aptos key generate` creates an x25519 key pair.
    """
    key_file = os.path.join(run_helper.host_working_directory, "test_x25519_key")
    
    result = run_helper.run_command(
        test_name,
        [
            "aptos", "key", "generate",
            "--key-type", "x25519",
            "--output-file", key_file,
            "--assume-yes",
        ],
    )
    
    json_result = json.loads(result.stdout)
    
    if "Result" not in json_result:
        raise TestError(f"Expected 'Result' in generate output, got: {result.stdout}")


@test_case
def test_key_generate_bls12381(run_helper: RunHelper, test_name=None):
    """
    Test that `aptos key generate` creates a bls12381 key.
    """
    key_file = os.path.join(run_helper.host_working_directory, "test_bls12381_key")
    
    result = run_helper.run_command(
        test_name,
        [
            "aptos", "key", "generate",
            "--key-type", "bls12381",
            "--output-file", key_file,
            "--assume-yes",
        ],
    )
    
    json_result = json.loads(result.stdout)
    
    if "Result" not in json_result:
        raise TestError(f"Expected 'Result' in generate output, got: {result.stdout}")
    
    # BLS keys should have proof of possession
    result_paths = json_result["Result"]
    if "Proof of possession Path" not in result_paths:
        raise TestError(f"Expected 'Proof of possession Path' in BLS result, got: {result_paths}")


@test_case
def test_key_extract_public_key(run_helper: RunHelper, test_name=None):
    """
    Test that `aptos key extract-public-key` extracts public key from private key.
    """
    # First generate a key
    key_file = os.path.join(run_helper.host_working_directory, "test_extract_key")
    
    run_helper.run_command(
        test_name,
        [
            "aptos", "key", "generate",
            "--key-type", "ed25519",
            "--output-file", key_file,
            "--assume-yes",
        ],
    )
    
    # Now extract public key
    output_file = os.path.join(run_helper.host_working_directory, "test_extracted_pub")
    
    result = run_helper.run_command(
        test_name,
        [
            "aptos", "key", "extract-public-key",
            "--key-type", "ed25519",
            "--private-key-file", key_file,
            "--output-file", output_file,
            "--assume-yes",
        ],
    )
    
    json_result = json.loads(result.stdout)
    
    if "Result" not in json_result:
        raise TestError(f"Expected 'Result' in extract output, got: {result.stdout}")


# =============================================================================
# Error handling tests
# =============================================================================

@test_case
def test_key_generate_missing_output_file(run_helper: RunHelper, test_name=None):
    """
    Test that key generate fails without output file.
    """
    try:
        run_helper.run_command(
            test_name,
            [
                "aptos", "key", "generate",
                "--key-type", "ed25519",
            ],
        )
        raise TestError("Expected command to fail without output file")
    except subprocess.CalledProcessError:
        # Expected to fail
        pass


@test_case
def test_key_extract_public_key_missing_input(run_helper: RunHelper, test_name=None):
    """
    Test that extract-public-key fails without private key input.
    """
    output_file = os.path.join(run_helper.host_working_directory, "test_no_input")
    
    try:
        run_helper.run_command(
            test_name,
            [
                "aptos", "key", "extract-public-key",
                "--key-type", "ed25519",
                "--output-file", output_file,
            ],
        )
        raise TestError("Expected command to fail without private key input")
    except subprocess.CalledProcessError:
        # Expected to fail
        pass
