// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::tests::common;

#[test]
fn clean_no_build() {
    let pkg = common::make_package("fresh", &[(
        "fresh",
        "module 0xCAFE::fresh {
    public fun hi(): u64 { 1 }
}",
    )]);
    let dir = pkg.path().to_str().unwrap();

    // Clean without prior build — should be a no-op success
    let output = common::run_cli(&["clean", "--package-dir", dir, "--assume-yes"]);
    common::check_baseline(file!(), &output);
}
