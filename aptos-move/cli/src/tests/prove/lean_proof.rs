// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::tests::common;

/// A specification the automatic verification cannot establish is proved
/// by the `verify` item of the proof file beside its source; a second
/// function of the file verifies automatically.
#[test]
fn prove_lean_proof() {
    if !move_prover::leaner::verifier_available() {
        eprintln!("skipping prove --lean test: the Leaner Move verifier is not built");
        return;
    }

    let pkg = common::make_package("lean_proof", &[(
        "lean_proof",
        "module 0xCAFE::lean_proof {
    fun square_of_sum(a: u64, b: u64): u64 { (a + b) * (a + b) }
    spec square_of_sum {
        aborts_if a + b > MAX_U64;
        aborts_if (a + b) * (a + b) > MAX_U64;
        ensures result == a * a + 2 * a * b + b * b;
    }
    fun twice(a: u64): u64 { a + a }
    spec twice {
        aborts_if a + a > MAX_U64;
        ensures result == 2 * a;
    }
}",
    )]);
    std::fs::write(
        pkg.path().join("sources/lean_proof.proof.lean"),
        "verify square_of_sum by
  case leaf_1 =>
    have := Int.mul_nonneg ‹0 ≤ a.val + b.val› ‹0 ≤ a.val + b.val›
    omega
  case leaf_2 =>
    rw [Int.add_mul, Int.mul_add, Int.mul_add, Int.mul_comm b.val a.val, Int.mul_assoc]
    omega
",
    )
    .expect("failed to write the proof file");
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
