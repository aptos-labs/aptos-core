# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

"""
Helper utilities for testing CLI flag availability.

This module provides functions to verify that specific CLI flags are present
in help text, enabling regression testing for CLI interface stability.
"""

import re
import subprocess
from dataclasses import dataclass
from typing import List, Optional, Set


class CLIFlagTestError(Exception):
    """Exception raised when CLI flag validation fails."""
    pass


@dataclass
class ExpectedFlag:
    """Represents an expected CLI flag."""
    name: str
    long_form: str  # e.g., "--account"
    short_form: Optional[str] = None  # e.g., "-a"
    required: bool = False
    description_contains: Optional[str] = None


def run_help_command(cli_path: str, subcommands: List[str], working_dir: str) -> str:
    """
    Run a CLI help command and return its output.
    
    Args:
        cli_path: Path to the CLI binary
        subcommands: List of subcommands to get help for (e.g., ["account", "create"])
        working_dir: Working directory to run the command from
    
    Returns:
        The help text output
    """
    command = [cli_path] + subcommands + ["--help"]
    result = subprocess.run(
        command,
        cwd=working_dir,
        capture_output=True,
        text=True,
    )
    # Help command should succeed
    if result.returncode != 0:
        raise CLIFlagTestError(
            f"Help command failed: {' '.join(command)}\n"
            f"stdout: {result.stdout}\n"
            f"stderr: {result.stderr}"
        )
    return result.stdout


def verify_flag_present(help_text: str, flag: ExpectedFlag) -> None:
    """
    Verify that a flag is present in help text.
    
    Args:
        help_text: The help text to search in
        flag: The expected flag to verify
    
    Raises:
        CLIFlagTestError: If the flag is not found or doesn't match expectations
    """
    # Check for long form flag
    if flag.long_form not in help_text:
        raise CLIFlagTestError(
            f"Expected flag '{flag.long_form}' not found in help text.\n"
            f"Flag name: {flag.name}\n"
            f"Help text:\n{help_text}"
        )
    
    # Check for short form flag if specified
    if flag.short_form and flag.short_form not in help_text:
        raise CLIFlagTestError(
            f"Expected short flag '{flag.short_form}' for '{flag.name}' not found in help text.\n"
            f"Help text:\n{help_text}"
        )
    
    # Check that description contains expected text if specified
    if flag.description_contains:
        if flag.description_contains.lower() not in help_text.lower():
            raise CLIFlagTestError(
                f"Expected description text '{flag.description_contains}' for flag '{flag.name}' "
                f"not found in help text.\n"
                f"Help text:\n{help_text}"
            )


def verify_flags_present(help_text: str, flags: List[ExpectedFlag]) -> None:
    """
    Verify that all specified flags are present in help text.
    
    Args:
        help_text: The help text to search in
        flags: List of expected flags to verify
    
    Raises:
        CLIFlagTestError: If any flag is not found
    """
    for flag in flags:
        verify_flag_present(help_text, flag)


def verify_subcommand_present(help_text: str, subcommand: str) -> None:
    """
    Verify that a subcommand is listed in help text.
    
    Args:
        help_text: The help text to search in
        subcommand: The subcommand name to verify
    
    Raises:
        CLIFlagTestError: If the subcommand is not found
    """
    if subcommand not in help_text:
        raise CLIFlagTestError(
            f"Expected subcommand '{subcommand}' not found in help text.\n"
            f"Help text:\n{help_text}"
        )


def verify_subcommands_present(help_text: str, subcommands: List[str]) -> None:
    """
    Verify that all specified subcommands are listed in help text.
    
    Args:
        help_text: The help text to search in
        subcommands: List of subcommand names to verify
    
    Raises:
        CLIFlagTestError: If any subcommand is not found
    """
    for subcommand in subcommands:
        verify_subcommand_present(help_text, subcommand)


def extract_flags_from_help(help_text: str) -> Set[str]:
    """
    Extract all long-form flags from help text.
    
    Args:
        help_text: The help text to parse
    
    Returns:
        Set of flag names found (e.g., {"--account", "--amount"})
    """
    # Match patterns like --flag-name or --flag
    pattern = r'--[a-zA-Z][a-zA-Z0-9-]*'
    return set(re.findall(pattern, help_text))


def get_cli_version(cli_path: str, working_dir: str) -> str:
    """
    Get the CLI version string.
    
    Args:
        cli_path: Path to the CLI binary
        working_dir: Working directory
    
    Returns:
        Version string
    """
    result = subprocess.run(
        [cli_path, "--version"],
        cwd=working_dir,
        capture_output=True,
        text=True,
    )
    return result.stdout.strip()
