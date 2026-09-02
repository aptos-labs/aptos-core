// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! End-to-end tests for the MCP server tools.
//!
//! Each submodule corresponds to an MCP tool (or meta-operation like
//! `list_tools`) and contains test cases with `.exp` baseline files.

pub(crate) mod common;

mod edit_hook;
mod list_tools;
mod move_package_manifest;
mod move_package_query;
mod move_package_spec_infer;
mod move_package_status;
mod move_package_test;
mod move_package_verify;
mod move_replay_transaction;
mod spec_check;
mod update;

use super::*;

#[test]
fn test_platform_display_name() {
    assert_eq!(Platform::Claude.display_name(), "Claude Code");
}

#[test]
fn inference_tactic_global_flag_is_accepted_after_subcommand() {
    let cli = FlowCli::try_parse_from([
        "move-flow",
        "plugin",
        "generated",
        "--inference-tactic",
        "agent-only",
        "--evaluation-mode",
    ])
    .expect("parse plugin evaluation flags");
    assert_eq!(
        cli.global.inference_tactic,
        Some(evaluation::InferenceTactic::AgentOnly)
    );
    assert!(cli.global.evaluation_mode);
}
