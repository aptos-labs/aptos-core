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
    account_address = str(run_helper.get_account_info().account_address)

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
    account_address = str(run_helper.get_account_info().account_address)

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
