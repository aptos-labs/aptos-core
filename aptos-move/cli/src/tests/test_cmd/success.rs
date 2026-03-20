// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::tests::common;

#[test]
fn test_success() {
    let pkg = common::make_package("test_ok", &[(
        "test_ok",
        "module 0xCAFE::test_ok {
    #[test]
    fun test_greet() {
        assert!(1 + 1 == 2, 0);
    }
}",
    )]);
    let dir = pkg.path().to_str().unwrap();
    let output = common::run_cli(&["test", "--package-dir", dir, "--skip-fetch-latest-git-deps"]);
    common::check_baseline(file!(), &output);
}
