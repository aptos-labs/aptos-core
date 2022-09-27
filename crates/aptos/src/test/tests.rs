// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{CliResult, Tool};
use clap::Parser;

/// In order to ensure that there aren't duplicate input arguments for untested CLI commands,
/// we call help on every command to ensure it at least runs
#[tokio::test]
async fn ensure_every_command_args_work() {
    assert_cmd_not_panic(&["aptos"]).await;

    assert_cmd_not_panic(&["aptos", "account"]).await;
    assert_cmd_not_panic(&["aptos", "account", "create", "--help"]).await;
    assert_cmd_not_panic(&["aptos", "account", "create-resource-account", "--help"]).await;
    assert_cmd_not_panic(&["aptos", "account", "fund-with-faucet", "--help"]).await;
    assert_cmd_not_panic(&["aptos", "account", "list", "--help"]).await;
    assert_cmd_not_panic(&["aptos", "account", "lookup-address", "--help"]).await;
    assert_cmd_not_panic(&["aptos", "account", "rotate-key", "--help"]).await;
    assert_cmd_not_panic(&["aptos", "account", "transfer", "--help"]).await;

    assert_cmd_not_panic(&["aptos", "config"]).await;
    assert_cmd_not_panic(&["aptos", "config", "generate-shell-completions", "--help"]).await;
    assert_cmd_not_panic(&["aptos", "config", "init", "--help"]).await;
    assert_cmd_not_panic(&["aptos", "config", "set-global-config", "--help"]).await;
    assert_cmd_not_panic(&["aptos", "config", "show-global-config"]).await;
    assert_cmd_not_panic(&["aptos", "config", "show-profiles"]).await;

    assert_cmd_not_panic(&["aptos", "genesis"]).await;
    assert_cmd_not_panic(&["aptos", "genesis", "generate-genesis", "--help"]).await;
    assert_cmd_not_panic(&["aptos", "genesis", "generate-keys", "--help"]).await;
    assert_cmd_not_panic(&["aptos", "genesis", "generate-layout-template", "--help"]).await;
    assert_cmd_not_panic(&["aptos", "genesis", "set-validator-configuration", "--help"]).await;
    assert_cmd_not_panic(&["aptos", "genesis", "setup-git", "--help"]).await;
    assert_cmd_not_panic(&["aptos", "genesis", "generate-admin-write-set", "--help"]).await;

    assert_cmd_not_panic(&["aptos", "governance"]).await;
    assert_cmd_not_panic(&["aptos", "governance", "execute-proposal", "--help"]).await;
    assert_cmd_not_panic(&["aptos", "governance", "generate-upgrade-proposal", "--help"]).await;
    assert_cmd_not_panic(&["aptos", "governance", "propose", "--help"]).await;
    assert_cmd_not_panic(&["aptos", "governance", "vote", "--help"]).await;

    assert_cmd_not_panic(&["aptos", "info"]).await;

    assert_cmd_not_panic(&["aptos", "init", "--help"]).await;

    assert_cmd_not_panic(&["aptos", "key"]).await;
    assert_cmd_not_panic(&["aptos", "key", "generate", "--help"]).await;
    assert_cmd_not_panic(&["aptos", "key", "extract-peer", "--help"]).await;

    assert_cmd_not_panic(&["aptos", "move"]).await;
    assert_cmd_not_panic(&["aptos", "move", "clean", "--help"]).await;
    assert_cmd_not_panic(&["aptos", "move", "compile", "--help"]).await;
    assert_cmd_not_panic(&["aptos", "move", "download", "--help"]).await;
    assert_cmd_not_panic(&["aptos", "move", "init", "--help"]).await;
    assert_cmd_not_panic(&["aptos", "move", "list", "--help"]).await;
    assert_cmd_not_panic(&["aptos", "move", "prove", "--help"]).await;
    assert_cmd_not_panic(&["aptos", "move", "publish", "--help"]).await;
    assert_cmd_not_panic(&["aptos", "move", "run", "--help"]).await;
    assert_cmd_not_panic(&["aptos", "move", "run-script", "--help"]).await;
    assert_cmd_not_panic(&["aptos", "move", "test", "--help"]).await;
    assert_cmd_not_panic(&["aptos", "move", "transactional-test", "--help"]).await;

    assert_cmd_not_panic(&["aptos", "node"]).await;
    assert_cmd_not_panic(&["aptos", "node", "get-stake-pool", "--help"]).await;
    assert_cmd_not_panic(&["aptos", "node", "analyze-validator-performance", "--help"]).await;
    assert_cmd_not_panic(&["aptos", "node", "bootstrap-db-from-backup", "--help"]).await;
    assert_cmd_not_panic(&["aptos", "node", "initialize-validator", "--help"]).await;
    assert_cmd_not_panic(&["aptos", "node", "join-validator-set", "--help"]).await;
    assert_cmd_not_panic(&["aptos", "node", "leave-validator-set", "--help"]).await;
    assert_cmd_not_panic(&["aptos", "node", "run-local-testnet", "--help"]).await;
    assert_cmd_not_panic(&["aptos", "node", "show-validator-config", "--help"]).await;
    assert_cmd_not_panic(&["aptos", "node", "show-validator-set", "--help"]).await;
    assert_cmd_not_panic(&["aptos", "node", "show-validator-stake", "--help"]).await;
    assert_cmd_not_panic(&["aptos", "node", "update-consensus-key", "--help"]).await;
    assert_cmd_not_panic(&[
        "aptos",
        "node",
        "update-validator-network-addresses",
        "--help",
    ])
    .await;

    assert_cmd_not_panic(&["aptos", "stake"]).await;
    assert_cmd_not_panic(&["aptos", "stake", "add-stake", "--help"]).await;
    assert_cmd_not_panic(&["aptos", "stake", "increase-lockup", "--help"]).await;
    assert_cmd_not_panic(&["aptos", "stake", "initialize-stake-owner", "--help"]).await;
    assert_cmd_not_panic(&["aptos", "stake", "set-delegated-voter", "--help"]).await;
    assert_cmd_not_panic(&["aptos", "stake", "set-operator", "--help"]).await;
    assert_cmd_not_panic(&["aptos", "stake", "unlock-stake", "--help"]).await;
    assert_cmd_not_panic(&["aptos", "stake", "withdraw-stake", "--help"]).await;
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
