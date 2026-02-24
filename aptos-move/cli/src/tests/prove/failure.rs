// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::tests::common;

fn prover_tools_available() -> bool {
    std::env::var("BOOGIE_EXE").is_ok() && std::env::var("Z3_EXE").is_ok()
}

/// A specification that contradicts the implementation should cause the prover
/// to report a verification error.
#[test]
fn prove_failure() {
    if !prover_tools_available() {
        eprintln!("skipping prove test: BOOGIE_EXE and Z3_EXE not set");
        return;
    }

    let pkg = common::make_package("prove_bad", &[(
        "prove_bad",
        "module 0xCAFE::prove_bad {
    fun add(a: u64, b: u64): u64 { a + b }
    spec add { ensures result == a - b; }
}",
    )]);
    let dir = pkg.path().to_str().unwrap();
    let output = common::run_cli(&[
        "prove",
        "--package-dir",
        dir,
        "--skip-fetch-latest-git-deps",
        "--stable-test-output",
        "--for-test",
    ]);
    common::check_baseline(file!(), &output);
}
