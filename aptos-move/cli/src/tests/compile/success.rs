// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::tests::common;

#[test]
fn compile_success() {
    let pkg = common::make_package("hello", &[(
        "hello",
        "module 0xCAFE::hello {
    public fun greet(): u64 { 42 }
}",
    )]);
    let dir = pkg.path().to_str().unwrap();
    let output = common::run_cli(&[
        "compile",
        "--package-dir",
        dir,
        "--skip-fetch-latest-git-deps",
    ]);
    common::check_baseline(file!(), &output);
}
