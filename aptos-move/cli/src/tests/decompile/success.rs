// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::tests::common;

#[test]
fn decompile_success() {
    let pkg = common::make_package("decomp_pkg", &[(
        "decomp_pkg",
        "module 0xCAFE::decomp_pkg {
    public fun add(a: u64, b: u64): u64 { a + b }
}",
    )]);
    let dir = pkg.path().to_str().unwrap();

    // Compile first to produce bytecode
    let compile_output = common::run_cli(&[
        "compile",
        "--package-dir",
        dir,
        "--skip-fetch-latest-git-deps",
    ]);
    assert!(compile_output.result.is_ok(), "compile should succeed");

    // Find the .mv file in the build directory
    let mv_path = pkg
        .path()
        .join("build/decomp_pkg/bytecode_modules/decomp_pkg.mv");
    assert!(mv_path.exists(), ".mv bytecode file should exist");

    let mv_str = mv_path.to_str().unwrap();
    let output = common::run_cli(&["decompile", "--bytecode-path", mv_str]);
    common::check_baseline(file!(), &output);
}
