// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::tests::common;

#[test]
fn publish_no_env() {
    let pkg = common::make_package("pub_pkg", &[(
        "pub_pkg",
        "module 0xCAFE::pub_pkg {
    public fun hello(): u64 { 42 }
}",
    )]);
    let dir = pkg.path().to_str().unwrap();
    let output = common::run_cli(&[
        "publish",
        "--package-dir",
        dir,
        "--skip-fetch-latest-git-deps",
        "--assume-yes",
    ]);
    // With default MoveEnv (no AptosContext), publish should fail
    common::check_baseline(file!(), &output);
}
