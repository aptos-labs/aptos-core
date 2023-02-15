// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::DBTool;
use clap::Parser;

#[test]
fn test_various_cmd_parsing() {
    run_cmd(&[
        "aptos-db-tool",
        "restore",
        "oneoff",
        "epoch-ending",
        "--epoch-ending-manifest",
        ".",
        "--local-fs-dir",
        ".",
        "--target-db-dir",
        ".",
    ]);
    run_cmd(&[
        "aptos-db-tool",
        "backup",
        "oneoff",
        "transaction",
        "--start-version",
        "100",
        "--num_transactions",
        "100",
        "--local-fs-dir",
        ".",
    ]);
    run_cmd(&[
        "aptos-db-tool",
        "backup",
        "continuously",
        "--local-fs-dir",
        ".",
    ]);
    run_cmd(&[
        "aptos-db-tool",
        "debug",
        "state-tree",
        "get-snapshots",
        "--db-dir",
        ".",
    ]);

    run_cmd(&["aptos-db-tool", "backup", "verify", "--local-fs-dir", "."]);
    run_cmd(&[
        "aptos-db-tool",
        "replay-verify",
        "--target-db-dir",
        ".",
        "--local-fs-dir",
        ".",
    ]);
}

fn run_cmd(args: &[&str]) {
    DBTool::try_parse_from(args).expect("command parse unsuccessful");
}
