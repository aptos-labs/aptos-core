// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::tests::common;

#[test]
fn test_filter() {
    let pkg = common::make_package("test_filter", &[(
        "test_filter",
        "module 0xCAFE::test_filter {
    #[test]
    fun test_one() {
        assert!(1 == 1, 0);
    }

    #[test]
    fun test_two() {
        abort 1
    }
}",
    )]);
    let dir = pkg.path().to_str().unwrap();
    let output = common::run_cli(&[
        "test",
        "--package-dir",
        dir,
        "--skip-fetch-latest-git-deps",
        "--filter",
        "test_one",
    ]);
    common::check_baseline(file!(), &output);
}
