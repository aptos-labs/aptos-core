#!/usr/bin/env python3

# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

"""
Tests for struct and enum transaction arguments in the CLI.

This test file focuses specifically on testing the CLI's ability to parse
and pass struct/enum arguments to Move entry functions.
"""

import json
import os
import subprocess
import tempfile

from common import TestError
from test_helpers import RunHelper
from test_results import test_case


@test_case
def test_publish_struct_enum_module(run_helper: RunHelper, test_name=None):
    """Publish the struct-enum-args test module."""
    package_dir = "move/cli-e2e-tests/struct-enum-args"

    run_helper.run_command(
        test_name or "publish_struct_enum_module",
        [
            "aptos",
            "move",
            "publish",
            "--assume-yes",
            "--language-version",
            "2.4",
            "--package-dir",
            package_dir,
            "--named-addresses",
            f"struct_enum_tests={str(run_helper.get_account_info().account_address)}",
        ],
    )


def run_move_function_with_json(run_helper: RunHelper, test_name: str, json_content: dict, error_msg: str):
    """Helper to run Move function with JSON args file."""
    with tempfile.NamedTemporaryFile(mode='w', suffix='.json', delete=False) as f:
        json.dump(json_content, f)
        json_file = f.name

    try:
        response = run_helper.run_command(
            test_name,
            [
                "aptos",
                "move",
                "run",
                "--json-file", json_file,
                "--assume-yes",
            ],
            input="\n",
        )

        # Verify transaction succeeded on-chain
        # The CLI can return exit code 0 even when the transaction fails,
        # so we must check stdout for the success indicator
        if '"success": true' not in response.stdout:
            raise TestError(f"{error_msg}: Transaction did not execute successfully on-chain")
    except TestError:
        # Re-raise TestError without wrapping to preserve specific error details
        raise
    except Exception as e:
        raise TestError(error_msg) from e
    finally:
        # Clean up temp file to avoid filesystem debris
        os.unlink(json_file)


# Struct argument tests

@test_case
def test_struct_argument_simple(run_helper: RunHelper, test_name=None):
    """Test passing a simple struct (Point) as transaction argument."""
    account_address = str(run_helper.get_account_info().account_address)

    json_content = {
        "function_id": "default::struct_enum_tests::test_struct_point",
        "type_args": [],
        "args": [
            {
                "type": f"{account_address}::struct_enum_tests::Point",
                "value": {"x": "10", "y": "20"}
            }
        ]
    }

    run_move_function_with_json(
        run_helper,
        test_name,
        json_content,
        "Failed to execute Move function with simple struct argument"
    )


@test_case
def test_struct_argument_nested(run_helper: RunHelper, test_name=None):
    """Test passing a struct with nested struct fields (Rectangle with Points)."""
    account_address = str(run_helper.get_account_info().account_address)

    json_content = {
        "function_id": "default::struct_enum_tests::test_struct_rectangle",
        "type_args": [],
        "args": [
            {
                "type": f"{account_address}::struct_enum_tests::Rectangle",
                "value": {
                    "top_left": {"x": "0", "y": "0"},
                    "bottom_right": {"x": "100", "y": "100"}
                }
            }
        ]
    }

    run_move_function_with_json(
        run_helper,
        test_name,
        json_content,
        "Failed to execute Move function with nested struct argument"
    )


# Option argument tests

@test_case
def test_option_variant_format(run_helper: RunHelper, test_name=None):
    """Test Option<T> with new variant format: {"None": {}} and {"Some": {"0": value}}."""
    # Test Option::Some
    json_content_some = {
        "function_id": "default::struct_enum_tests::test_option_some",
        "type_args": [],
        "args": [
            {
                "type": "0x1::option::Option<u64>",
                "value": {"Some": {"0": "100"}}
            }
        ]
    }

    run_move_function_with_json(
        run_helper,
        f"{test_name}_some",
        json_content_some,
        "Failed to execute Move function with Option::Some variant format"
    )

    # Test Option::None
    json_content_none = {
        "function_id": "default::struct_enum_tests::test_option_none",
        "type_args": [],
        "args": [
            {
                "type": "0x1::option::Option<u64>",
                "value": {"None": {}}
            }
        ]
    }

    run_move_function_with_json(
        run_helper,
        f"{test_name}_none",
        json_content_none,
        "Failed to execute Move function with Option::None variant format"
    )


@test_case
def test_option_legacy_format(run_helper: RunHelper, test_name=None):
    """Test Option<T> with legacy vector format: [] for None, [value] for Some."""
    # Test Option::Some with vector format
    json_content_some = {
        "function_id": "default::struct_enum_tests::test_option_some",
        "type_args": [],
        "args": [
            {
                "type": "0x1::option::Option<u64>",
                "value": ["100"]
            }
        ]
    }

    run_move_function_with_json(
        run_helper,
        f"{test_name}_some",
        json_content_some,
        "Failed to execute Move function with Option::Some legacy vector format"
    )

    # Test Option::None with vector format
    json_content_none = {
        "function_id": "default::struct_enum_tests::test_option_none",
        "type_args": [],
        "args": [
            {
                "type": "0x1::option::Option<u64>",
                "value": []
            }
        ]
    }

    run_move_function_with_json(
        run_helper,
        f"{test_name}_none",
        json_content_none,
        "Failed to execute Move function with Option::None legacy vector format"
    )


# Enum argument tests

@test_case
def test_enum_simple_variant(run_helper: RunHelper, test_name=None):
    """Test passing an enum with a simple variant (no fields)."""
    account_address = str(run_helper.get_account_info().account_address)

    json_content = {
        "function_id": "default::struct_enum_tests::test_enum_color_simple",
        "type_args": [],
        "args": [
            {
                "type": f"{account_address}::struct_enum_tests::Color",
                "value": {"Red": {}}
            }
        ]
    }

    run_move_function_with_json(
        run_helper,
        test_name,
        json_content,
        "Failed to execute Move function with simple enum variant"
    )


@test_case
def test_enum_variant_with_fields(run_helper: RunHelper, test_name=None):
    """Test passing an enum variant with fields."""
    account_address = str(run_helper.get_account_info().account_address)

    json_content = {
        "function_id": "default::struct_enum_tests::test_enum_color_rgb",
        "type_args": [],
        "args": [
            {
                "type": f"{account_address}::struct_enum_tests::Color",
                "value": {"RGB": {"r": "255", "g": "128", "b": "0"}}
            }
        ]
    }

    run_move_function_with_json(
        run_helper,
        test_name,
        json_content,
        "Failed to execute Move function with enum variant containing fields"
    )


@test_case
def test_enum_with_nested_struct(run_helper: RunHelper, test_name=None):
    """Test passing an enum variant that contains a nested struct."""
    account_address = str(run_helper.get_account_info().account_address)

    json_content = {
        "function_id": "default::struct_enum_tests::test_enum_shape_circle",
        "type_args": [],
        "args": [
            {
                "type": f"{account_address}::struct_enum_tests::Shape",
                "value": {
                    "Circle": {
                        "center": {"x": "50", "y": "50"},
                        "radius": "25"
                    }
                }
            }
        ]
    }

    run_move_function_with_json(
        run_helper,
        test_name,
        json_content,
        "Failed to execute Move function with enum containing nested struct"
    )


# Framework type tests

@test_case
def test_framework_string(run_helper: RunHelper, test_name=None):
    """Test String framework type with primitive string value."""
    json_content = {
        "function_id": "default::struct_enum_tests::test_framework_string",
        "type_args": [],
        "args": [
            {
                "type": "0x1::string::String",
                "value": "hello"
            }
        ]
    }

    run_move_function_with_json(
        run_helper,
        test_name,
        json_content,
        "Failed to execute Move function with String framework type"
    )


@test_case
def test_framework_fixed_point32(run_helper: RunHelper, test_name=None):
    """Test FixedPoint32 framework type with u64 primitive value."""
    json_content = {
        "function_id": "default::struct_enum_tests::test_framework_fixed_point32",
        "type_args": [],
        "args": [
            {
                "type": "0x1::fixed_point32::FixedPoint32",
                "value": "1000"
            }
        ]
    }

    run_move_function_with_json(
        run_helper,
        test_name,
        json_content,
        "Failed to execute Move function with FixedPoint32 framework type"
    )


@test_case
def test_framework_fixed_point64(run_helper: RunHelper, test_name=None):
    """Test FixedPoint64 framework type with u128 primitive value."""
    json_content = {
        "function_id": "default::struct_enum_tests::test_framework_fixed_point64",
        "type_args": [],
        "args": [
            {
                "type": "0x1::fixed_point64::FixedPoint64",
                "value": "2000"
            }
        ]
    }

    run_move_function_with_json(
        run_helper,
        test_name,
        json_content,
        "Failed to execute Move function with FixedPoint64 framework type"
    )


@test_case
def test_mixed_framework_struct(run_helper: RunHelper, test_name=None):
    """Test mixed framework types and custom structs in the same function."""
    account_address = str(run_helper.get_account_info().account_address)

    json_content = {
        "function_id": "default::struct_enum_tests::test_mixed_framework_struct",
        "type_args": [],
        "args": [
            {
                "type": "0x1::string::String",
                "value": "test"
            },
            {
                "type": f"{account_address}::struct_enum_tests::Point",
                "value": {"x": "100", "y": "200"}
            },
            {
                "type": "0x1::fixed_point32::FixedPoint32",
                "value": "500"
            }
        ]
    }

    run_move_function_with_json(
        run_helper,
        test_name,
        json_content,
        "Failed to execute Move function with mixed framework and struct types"
    )


# Validation tests

@test_case
def test_struct_unknown_field_rejected(run_helper: RunHelper, test_name=None):
    """Test that structs with unknown fields are rejected."""
    account_address = str(run_helper.get_account_info().account_address)

    json_content = {
        "function_id": "default::struct_enum_tests::test_struct_point",
        "type_args": [],
        "args": [
            {
                "type": f"{account_address}::struct_enum_tests::Point",
                "value": {"x": "10", "y": "20", "z": "30"}  # 'z' is unknown field
            }
        ]
    }

    # This should fail with an error about unknown field
    try:
        with tempfile.NamedTemporaryFile(mode='w', suffix='.json', delete=False) as f:
            json.dump(json_content, f)
            json_file = f.name

        result = run_helper.run_command(
            test_name,
            [
                "aptos",
                "move",
                "run",
                "--json-file", json_file,
                "--assume-yes",
            ],
            input="\n",
        )

        # Should fail - if we get here with success, that's wrong
        if '"success": true' in result.stdout:
            raise TestError("Expected rejection of unknown field 'z', but transaction succeeded")

        # Verify error message mentions unknown field
        combined_output = result.stdout + result.stderr
        if "Unknown field" not in combined_output and "z" not in combined_output:
            raise TestError(f"Expected error about unknown field 'z', got: {combined_output}")

    except subprocess.CalledProcessError as e:
        # Expected - command should fail
        # Verify the error message is about unknown field
        combined_output = e.stdout + e.stderr if hasattr(e, 'stdout') else str(e)
        if "Unknown field" not in combined_output:
            raise TestError(f"Expected error about unknown field, got: {combined_output}")
    finally:
        os.unlink(json_file)


@test_case
def test_enum_unknown_field_rejected(run_helper: RunHelper, test_name=None):
    """Test that enum variants with unknown fields are rejected."""
    account_address = str(run_helper.get_account_info().account_address)

    json_content = {
        "function_id": "default::struct_enum_tests::test_enum_color_rgb",
        "type_args": [],
        "args": [
            {
                "type": f"{account_address}::struct_enum_tests::Color",
                "value": {"RGB": {"r": "255", "g": "128", "b": "0", "a": "255"}}  # 'a' is unknown
            }
        ]
    }

    # This should fail with an error about unknown field
    try:
        with tempfile.NamedTemporaryFile(mode='w', suffix='.json', delete=False) as f:
            json.dump(json_content, f)
            json_file = f.name

        result = run_helper.run_command(
            test_name,
            [
                "aptos",
                "move",
                "run",
                "--json-file", json_file,
                "--assume-yes",
            ],
            input="\n",
        )

        # Should fail - if we get here with success, that's wrong
        if '"success": true' in result.stdout:
            raise TestError("Expected rejection of unknown field 'a', but transaction succeeded")

        # Verify error message mentions unknown field
        combined_output = result.stdout + result.stderr
        if "Unknown field" not in combined_output and "a" not in combined_output:
            raise TestError(f"Expected error about unknown field 'a', got: {combined_output}")

    except subprocess.CalledProcessError as e:
        # Expected - command should fail
        # Verify the error message is about unknown field
        combined_output = e.stdout + e.stderr if hasattr(e, 'stdout') else str(e)
        if "Unknown field" not in combined_output:
            raise TestError(f"Expected error about unknown field, got: {combined_output}")
    finally:
        os.unlink(json_file)


# Vector type tests

@test_case
def test_vector_of_strings(run_helper: RunHelper, test_name=None):
    """Test vector<0x1::string::String> type with primitive values."""
    json_content = {
        "function_id": "default::struct_enum_tests::test_vector_of_strings",
        "type_args": [],
        "args": [
            {
                "type": "vector<0x1::string::String>",
                "value": ["hello", "world", "test"]
            }
        ]
    }

    run_move_function_with_json(
        run_helper,
        test_name,
        json_content,
        "Failed to execute Move function with vector<String>"
    )


@test_case
def test_vector_of_structs(run_helper: RunHelper, test_name=None):
    """Test vector of custom structs."""
    account_address = str(run_helper.get_account_info().account_address)

    json_content = {
        "function_id": "default::struct_enum_tests::test_vector_of_structs",
        "type_args": [],
        "args": [
            {
                "type": f"vector<{account_address}::struct_enum_tests::Point>",
                "value": [
                    {"x": "10", "y": "20"},
                    {"x": "30", "y": "40"}
                ]
            }
        ]
    }

    run_move_function_with_json(
        run_helper,
        test_name,
        json_content,
        "Failed to execute Move function with vector<Point>"
    )


@test_case
def test_vector_of_options(run_helper: RunHelper, test_name=None):
    """Test vector<Option<T>> with mixed Some/None values."""
    json_content = {
        "function_id": "default::struct_enum_tests::test_vector_of_options",
        "type_args": [],
        "args": [
            {
                "type": "vector<0x1::option::Option<u64>>",
                "value": [
                    {"Some": {"0": "100"}},
                    {"None": {}},
                    {"Some": {"0": "200"}}
                ]
            }
        ]
    }

    run_move_function_with_json(
        run_helper,
        test_name,
        json_content,
        "Failed to execute Move function with vector<Option<u64>>"
    )


@test_case
def test_option_invalid_field_name_rejected(run_helper: RunHelper, test_name=None):
    """Test that Option::Some with wrong field name is rejected."""
    json_content = {
        "function_id": "default::struct_enum_tests::test_option_some",
        "type_args": [],
        "args": [
            {
                "type": "0x1::option::Option<u64>",
                "value": {"Some": {"wrong": "100"}}  # Field should be "0", not "wrong"
            }
        ]
    }

    # This should fail with an error about invalid field name
    try:
        with tempfile.NamedTemporaryFile(mode='w', suffix='.json', delete=False) as f:
            json.dump(json_content, f)
            json_file = f.name

        result = run_helper.run_command(
            test_name,
            [
                "aptos",
                "move",
                "run",
                "--json-file", json_file,
                "--assume-yes",
            ],
            input="\n",
        )

        # Should fail - if we get here with success, that's wrong
        if '"success": true' in result.stdout:
            raise TestError("Expected rejection of Option::Some with wrong field name, but transaction succeeded")

        # Verify error message mentions field name
        combined_output = result.stdout + result.stderr
        if "field must be named" not in combined_output.lower() and '"0"' not in combined_output:
            raise TestError(f"Expected error about field name, got: {combined_output}")

    except subprocess.CalledProcessError as e:
        # Expected - command should fail
        # Verify the error message is about field name
        combined_output = e.stdout + e.stderr if hasattr(e, 'stdout') else str(e)
        if "field" not in combined_output.lower():
            raise TestError(f"Expected error about field name, got: {combined_output}")
    finally:
        os.unlink(json_file)
