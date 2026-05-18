// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::tests::common;

/// An entry function that returns a value triggers an extended-check error.
#[test]
fn lint_failure() {
    let pkg = common::make_package("lint_bad", &[(
        "lint_bad",
        "module 0xCAFE::lint_bad {
    public entry fun bad_entry(): u64 { 42 }
}",
    )]);
    let dir = pkg.path().to_str().unwrap();
    let output = common::run_cli(&["lint", "--package-dir", dir, "--skip-fetch-latest-git-deps"]);
    common::check_baseline(file!(), &output);
}
