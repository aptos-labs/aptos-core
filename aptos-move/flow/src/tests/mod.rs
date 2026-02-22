// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! End-to-end tests for the MCP server tools.
//!
//! Each submodule corresponds to an MCP tool (or meta-operation like
//! `list_tools`) and contains test cases with `.exp` baseline files.

mod common;

mod edit_hook;
mod list_tools;
mod move_package_manifest;
mod move_package_status;
mod move_package_verify;

use super::*;

#[test]
fn test_platform_display_name() {
    assert_eq!(Platform::Claude.display_name(), "Claude Code");
}
