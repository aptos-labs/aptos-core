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
    assert_eq!(Platform::Codex.display_name(), "Codex");
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

#[test]
fn mcp_package_cache_can_be_disabled() {
    let cli = FlowCli::try_parse_from(["move-flow", "mcp", "--no-package-cache"])
        .expect("parse MCP cache flag");
    let FlowCommand::Mcp(args) = cli.command else {
        panic!("expected MCP command");
    };
    assert!(args.no_package_cache);
}

#[tokio::test]
async fn evaluation_mcp_rejects_packages_and_dependencies_outside_its_root() {
    let root = tempfile::TempDir::new().expect("temporary root");
    let package = root.path().join("workspace");
    let outside = root.path().join("baseline");
    std::fs::create_dir_all(package.join("sources")).expect("workspace sources");
    std::fs::create_dir_all(outside.join("sources")).expect("baseline sources");
    std::fs::write(
        package.join("Move.toml"),
        "[package]\nname = \"workspace\"\nversion = \"1.0.0\"\n",
    )
    .expect("workspace manifest");
    std::fs::write(
        outside.join("Move.toml"),
        "[package]\nname = \"baseline\"\nversion = \"1.0.0\"\n",
    )
    .expect("baseline manifest");

    let client = common::make_evaluation_client_with_package_root(&package).await;
    let error = common::call_tool_raw(
        &client,
        "move_package_status",
        serde_json::json!({"package_path": outside}),
    )
    .await
    .expect_err("a caller path outside the package root must be refused");
    assert!(error.to_string().contains("outside the configured root"));

    std::fs::write(
        package.join("Move.toml"),
        "[package]\nname = \"workspace\"\nversion = \"1.0.0\"\n\n[dependencies]\nbaseline = { local = \"../baseline\" }\n",
    )
    .expect("manifest with escaping dependency");
    let error = common::call_tool_raw(
        &client,
        "move_package_status",
        serde_json::json!({"package_path": package}),
    )
    .await
    .expect_err("a local dependency outside the package root must be refused");
    assert!(error.to_string().contains("outside"));
}
