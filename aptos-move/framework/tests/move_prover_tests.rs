// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use framework::prover::ProverOptions;
use std::collections::BTreeMap;
use std::path::PathBuf;

// Note: to run these tests, use:
//
//   cargo test -- --include-ignored prover

pub fn path_in_crate<S>(relative: S) -> PathBuf
where
    S: Into<String>,
{
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push(relative.into());
    path
}

pub fn run_prover_for_pkg(path_to_pkg: impl Into<String>) {
    let pkg_path = path_in_crate(path_to_pkg);
    let options = ProverOptions::default_for_test();
    options
        .prove(pkg_path.as_path(), BTreeMap::default())
        .unwrap()
}

#[ignore]
#[test]
fn move_framework_prover_tests() {
    run_prover_for_pkg("aptos-framework");
}

#[ignore]
#[test]
fn move_token_prover_tests() {
    run_prover_for_pkg("aptos-token");
}

#[ignore]
#[test]
fn move_aptos_stdlib_prover_tests() {
    run_prover_for_pkg("aptos-stdlib");
}

#[ignore]
#[test]
fn move_stdlib_prover_tests() {
    run_prover_for_pkg("move-stdlib");
}
