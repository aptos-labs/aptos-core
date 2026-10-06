// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::tests::common;

/// A specification that contradicts the implementation is reported at its
/// clause in the Move source.
#[test]
fn prove_lean_failure() {
    if !move_prover::leaner::verifier_available() {
        eprintln!("skipping prove --lean test: the Leaner Move verifier is not built");
        return;
    }

    let pkg = common::make_package("lean_bad", &[(
        "lean_bad",
        "module 0xCAFE::lean_bad {
    fun add(a: u64, b: u64): u64 { a + b }
    spec add {
        aborts_if a + b > MAX_U64;
        ensures result == a - b;
    }
}",
    )]);
    let dir = pkg.path().to_str().unwrap();
    let output = common::run_cli(&[
        "prove",
        "--lean",
        "--package-dir",
        dir,
        "--skip-fetch-latest-git-deps",
    ]);
    common::check_baseline(file!(), &output);
}
