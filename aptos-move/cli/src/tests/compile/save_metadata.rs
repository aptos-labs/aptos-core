// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::tests::common;

#[test]
fn compile_save_metadata() {
    let pkg = common::make_package("meta", &[(
        "meta",
        "module 0xCAFE::meta {
    public fun value(): u64 { 1 }
}",
    )]);
    let dir = pkg.path().to_str().unwrap();
    let output = common::run_cli(&[
        "compile",
        "--package-dir",
        dir,
        "--skip-fetch-latest-git-deps",
        "--save-metadata",
    ]);
    assert!(
        output.result.is_ok(),
        "compile --save-metadata failed: {:?}",
        output.result
    );

    // Verify that metadata files were written into the build directory
    let build_dir = pkg.path().join("build").join("meta");
    assert!(
        build_dir.join("package-metadata.bcs").exists(),
        "package-metadata.bcs should exist after --save-metadata"
    );
}
