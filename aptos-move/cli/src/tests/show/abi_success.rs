// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::tests::common;

#[test]
fn show_abi_success() {
    let pkg = common::make_package("entry_mod", &[(
        "entry_mod",
        "module 0xCAFE::entry_mod {
    public entry fun do_something(_x: u64) {}
}",
    )]);
    let dir = pkg.path().to_str().unwrap();
    let output = common::run_cli(&[
        "show",
        "abi",
        "--package-dir",
        dir,
        "--skip-fetch-latest-git-deps",
    ]);
    common::check_baseline(file!(), &output);
}
