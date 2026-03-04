// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::tests::common;

#[test]
fn compile_fetch_deps_only() {
    let pkg = common::make_package("deps_only", &[(
        "deps_only",
        "module 0xCAFE::deps_only {
    public fun noop() {}
}",
    )]);
    let dir = pkg.path().to_str().unwrap();
    let output = common::run_cli(&[
        "compile",
        "--package-dir",
        dir,
        "--skip-fetch-latest-git-deps",
        "--fetch-deps-only",
    ]);
    common::check_baseline(file!(), &output);
}
