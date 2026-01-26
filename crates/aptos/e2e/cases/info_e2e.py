# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

"""
Comprehensive E2E tests for the `aptos info` CLI command.

This module provides comprehensive testing for:
1. CLI flag availability and regression testing
2. Help text verification
3. Functional tests for info command
"""

import json

from cases.cli_flag_helpers import run_help_command
from common import TestError
from test_helpers import RunHelper
from test_results import test_case


# =============================================================================
# Help text and flag availability tests (CLI regression testing)
# =============================================================================

@test_case
def test_info_help(run_helper: RunHelper, test_name=None):
    """
    Test that the `aptos info` command help is available.
    """
    help_text = run_help_command(run_helper, ["info"], test_name)
    
    # Verify help text contains expected content
    if "build" not in help_text.lower():
        raise TestError(f"Expected 'build' in info help text, got: {help_text}")


# =============================================================================
# Functional E2E tests for info command
# =============================================================================

@test_case
def test_info_command(run_helper: RunHelper, test_name=None):
    """
    Test that `aptos info` returns build information.
    """
    result = run_helper.run_command(
        test_name,
        ["aptos", "info"],
    )
    
    json_result = json.loads(result.stdout)
    
    if "Result" not in json_result:
        raise TestError(f"Expected 'Result' in info output, got: {result.stdout}")
    
    info = json_result["Result"]
    
    # Verify expected build info fields
    expected_fields = ["build_branch", "build_commit_hash", "build_tag"]
    for field in expected_fields:
        if field not in info:
            raise TestError(f"Expected '{field}' in info result, got: {list(info.keys())}")


@test_case
def test_info_json_format(run_helper: RunHelper, test_name=None):
    """
    Test that info output is valid JSON with expected structure.
    """
    result = run_helper.run_command(
        test_name,
        ["aptos", "info"],
    )
    
    try:
        json_result = json.loads(result.stdout)
    except json.JSONDecodeError as e:
        raise TestError(f"Info output is not valid JSON: {e}")
    
    if not isinstance(json_result, dict):
        raise TestError(f"Expected JSON object, got {type(json_result)}")
    
    if "Result" not in json_result:
        raise TestError("Expected 'Result' key in JSON output")
    
    # Result should be a dict of build info
    if not isinstance(json_result["Result"], dict):
        raise TestError(f"Expected Result to be dict, got {type(json_result['Result'])}")
