// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::tests::common;

#[test]
fn lint_success() {
    let pkg = common::make_package("lint_ok", &[(
        "lint_ok",
        "module 0xCAFE::lint_ok {
    public fun value(): u64 { 42 }
}",
    )]);
    let dir = pkg.path().to_str().unwrap();
    let output = common::run_cli(&["lint", "--package-dir", dir, "--skip-fetch-latest-git-deps"]);
    common::check_baseline(file!(), &output);
}
