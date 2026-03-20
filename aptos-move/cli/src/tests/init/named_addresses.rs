// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::tests::common;

#[test]
fn init_named_addresses() {
    let dir = tempfile::tempdir().unwrap();
    let dir_str = dir.path().to_str().unwrap();
    let output = common::run_cli(&[
        "init",
        "--name",
        "addr_pkg",
        "--package-dir",
        dir_str,
        "--assume-yes",
        "--skip-fetch-latest-git-deps",
        "--named-addresses",
        "alice=0x1",
    ]);
    common::check_baseline(file!(), &output);

    // Verify Move.toml contains the named address
    let toml_content = std::fs::read_to_string(dir.path().join("Move.toml")).unwrap();
    assert!(
        toml_content.contains("alice"),
        "Move.toml should contain named address 'alice'"
    );
}
