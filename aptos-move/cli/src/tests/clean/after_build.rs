// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::tests::common;

#[test]
fn clean_after_build() {
    let pkg = common::make_package("cleanme", &[(
        "cleanme",
        "module 0xCAFE::cleanme {
    public fun hi(): u64 { 1 }
}",
    )]);
    let dir = pkg.path().to_str().unwrap();

    // First compile to create build artifacts
    let compile_output = common::run_cli(&[
        "compile",
        "--package-dir",
        dir,
        "--skip-fetch-latest-git-deps",
    ]);
    assert!(compile_output.result.is_ok(), "compile should succeed");
    assert!(
        pkg.path().join("build").exists(),
        "build/ should exist after compile"
    );

    // Now clean
    let output = common::run_cli(&["clean", "--package-dir", dir, "--assume-yes"]);
    common::check_baseline(file!(), &output);

    assert!(
        !pkg.path().join("build").exists(),
        "build/ should be removed after clean"
    );
}
