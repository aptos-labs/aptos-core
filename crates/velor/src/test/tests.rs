// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    move_tool::{ArgWithType, FunctionArgType},
    CliResult, Tool,
};
use clap::Parser;
use std::str::FromStr;

/// In order to ensure that there aren't duplicate input arguments for untested CLI commands,
/// we call help on every command to ensure it at least runs
#[tokio::test]
async fn ensure_every_command_args_work() {
    assert_cmd_not_panic(&["velor"]).await;

    assert_cmd_not_panic(&["velor", "account"]).await;
    assert_cmd_not_panic(&["velor", "account", "create", "--help"]).await;
    assert_cmd_not_panic(&["velor", "account", "create-resource-account", "--help"]).await;
    assert_cmd_not_panic(&["velor", "account", "fund-with-faucet", "--help"]).await;
    assert_cmd_not_panic(&["velor", "account", "list", "--help"]).await;
    assert_cmd_not_panic(&["velor", "account", "lookup-address", "--help"]).await;
    assert_cmd_not_panic(&["velor", "account", "rotate-key", "--help"]).await;
    assert_cmd_not_panic(&["velor", "account", "transfer", "--help"]).await;

    assert_cmd_not_panic(&["velor", "config"]).await;
    assert_cmd_not_panic(&["velor", "config", "generate-shell-completions", "--help"]).await;
    assert_cmd_not_panic(&["velor", "config", "init", "--help"]).await;
    assert_cmd_not_panic(&["velor", "config", "set-global-config", "--help"]).await;
    assert_cmd_not_panic(&["velor", "config", "show-global-config"]).await;
    assert_cmd_not_panic(&["velor", "config", "show-profiles"]).await;

    assert_cmd_not_panic(&["velor", "genesis"]).await;
    assert_cmd_not_panic(&["velor", "genesis", "generate-genesis", "--help"]).await;
    assert_cmd_not_panic(&["velor", "genesis", "generate-keys", "--help"]).await;
    assert_cmd_not_panic(&["velor", "genesis", "generate-layout-template", "--help"]).await;
    assert_cmd_not_panic(&["velor", "genesis", "set-validator-configuration", "--help"]).await;
    assert_cmd_not_panic(&["velor", "genesis", "setup-git", "--help"]).await;
    assert_cmd_not_panic(&["velor", "genesis", "generate-admin-write-set", "--help"]).await;

    assert_cmd_not_panic(&["velor", "governance"]).await;
    assert_cmd_not_panic(&["velor", "governance", "execute-proposal", "--help"]).await;
    assert_cmd_not_panic(&["velor", "governance", "generate-upgrade-proposal", "--help"]).await;
    assert_cmd_not_panic(&["velor", "governance", "propose", "--help"]).await;
    assert_cmd_not_panic(&["velor", "governance", "vote", "--help"]).await;
    assert_cmd_not_panic(&["velor", "governance", "delegation_pool", "--help"]).await;
    assert_cmd_not_panic(&["velor", "governance", "delegation_pool", "vote", "--help"]).await;
    assert_cmd_not_panic(&[
        "velor",
        "governance",
        "delegation_pool",
        "propose",
        "--help",
    ])
    .await;

    assert_cmd_not_panic(&["velor", "info"]).await;

    assert_cmd_not_panic(&["velor", "init", "--help"]).await;

    assert_cmd_not_panic(&["velor", "key"]).await;
    assert_cmd_not_panic(&["velor", "key", "generate", "--help"]).await;
    assert_cmd_not_panic(&["velor", "key", "extract-peer", "--help"]).await;

    assert_cmd_not_panic(&["velor", "move"]).await;
    assert_cmd_not_panic(&["velor", "move", "clean", "--help"]).await;
    assert_cmd_not_panic(&["velor", "move", "compile", "--help"]).await;
    assert_cmd_not_panic(&["velor", "move", "compile-script", "--help"]).await;
    assert_cmd_not_panic(&["velor", "move", "decompile", "--help"]).await;
    assert_cmd_not_panic(&["velor", "move", "disassemble", "--help"]).await;
    assert_cmd_not_panic(&["velor", "move", "download", "--help"]).await;
    assert_cmd_not_panic(&["velor", "move", "init", "--help"]).await;
    assert_cmd_not_panic(&["velor", "move", "list", "--help"]).await;
    assert_cmd_not_panic(&["velor", "move", "prove", "--help"]).await;
    assert_cmd_not_panic(&["velor", "move", "publish", "--help"]).await;
    assert_cmd_not_panic(&["velor", "move", "run", "--help"]).await;
    assert_cmd_not_panic(&["velor", "move", "run-script", "--help"]).await;
    assert_cmd_not_panic(&["velor", "move", "test", "--help"]).await;
    assert_cmd_not_panic(&["velor", "move", "transactional-test", "--help"]).await;
    assert_cmd_not_panic(&["velor", "move", "view", "--help"]).await;

    assert_cmd_not_panic(&["velor", "node"]).await;
    assert_cmd_not_panic(&["velor", "node", "check-network-connectivity", "--help"]).await;
    assert_cmd_not_panic(&["velor", "node", "get-stake-pool", "--help"]).await;
    assert_cmd_not_panic(&["velor", "node", "analyze-validator-performance", "--help"]).await;
    assert_cmd_not_panic(&["velor", "node", "bootstrap-db-from-backup", "--help"]).await;
    assert_cmd_not_panic(&["velor", "node", "initialize-validator", "--help"]).await;
    assert_cmd_not_panic(&["velor", "node", "join-validator-set", "--help"]).await;
    assert_cmd_not_panic(&["velor", "node", "leave-validator-set", "--help"]).await;
    assert_cmd_not_panic(&["velor", "node", "run-local-testnet", "--help"]).await;
    assert_cmd_not_panic(&["velor", "node", "show-validator-config", "--help"]).await;
    assert_cmd_not_panic(&["velor", "node", "show-validator-set", "--help"]).await;
    assert_cmd_not_panic(&["velor", "node", "show-validator-stake", "--help"]).await;
    assert_cmd_not_panic(&["velor", "node", "update-consensus-key", "--help"]).await;
    assert_cmd_not_panic(&[
        "velor",
        "node",
        "update-validator-network-addresses",
        "--help",
    ])
    .await;

    assert_cmd_not_panic(&["velor", "stake"]).await;
    assert_cmd_not_panic(&["velor", "stake", "add-stake", "--help"]).await;
    assert_cmd_not_panic(&["velor", "stake", "increase-lockup", "--help"]).await;
    assert_cmd_not_panic(&["velor", "stake", "initialize-stake-owner", "--help"]).await;
    assert_cmd_not_panic(&["velor", "stake", "set-delegated-voter", "--help"]).await;
    assert_cmd_not_panic(&["velor", "stake", "set-operator", "--help"]).await;
    assert_cmd_not_panic(&["velor", "stake", "unlock-stake", "--help"]).await;
    assert_cmd_not_panic(&["velor", "stake", "withdraw-stake", "--help"]).await;
}

/// Ensure we can parse URLs for args
#[tokio::test]
async fn ensure_can_parse_args_with_urls() {
    let result = ArgWithType::from_str("string:https://velorlabs.com").unwrap();
    matches!(result._ty, FunctionArgType::String);
    assert_eq!(
        result.arg,
        bcs::to_bytes(&"https://velorlabs.com".to_string()).unwrap()
    );
}

async fn assert_cmd_not_panic(args: &[&str]) {
    // When a command fails, it will have a panic in it due to an improperly setup command
    // thread 'main' panicked at 'Command propose: Argument names must be unique, but 'assume-yes' is
    // in use by more than one argument or group', ...

    match run_cmd(args).await {
        Ok(inner) => assert!(
            !inner.contains("panic"),
            "Failed to not panic cmd {}: {}",
            args.join(" "),
            inner
        ),
        Err(inner) => assert!(
            !inner.contains("panic"),
            "Failed to not panic cmd {}: {}",
            args.join(" "),
            inner
        ),
    }
}

async fn run_cmd(args: &[&str]) -> CliResult {
    let tool: Tool = Tool::try_parse_from(args).map_err(|msg| msg.to_string())?;
    tool.execute().await
}
