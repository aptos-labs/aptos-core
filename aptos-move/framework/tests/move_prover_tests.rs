// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_framework::prover::ProverOptions;
use std::{collections::BTreeMap, path::PathBuf};

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

pub fn read_env_var(v: &str) -> String {
    std::env::var(v).unwrap_or_else(|_| String::new())
}

pub fn run_prover_for_pkg(path_to_pkg: impl Into<String>) {
    let pkg_path = path_in_crate(path_to_pkg);
    let options = ProverOptions::default_for_test();
    let no_tools = read_env_var("BOOGIE_EXE").is_empty()
        || !options.cvc5 && read_env_var("Z3_EXE").is_empty()
        || options.cvc5 && read_env_var("CVC5_EXE").is_empty();
    if no_tools {
        panic!(
            "Prover tools are not configured, \
        See https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/FRAMEWORK-PROVER-GUIDE.md \
        for instructions, or \
        use \"-- --skip prover\" to filter out the prover tests"
        );
    } else {
        options
            .prove(false, pkg_path.as_path(), BTreeMap::default(), None)
            .unwrap()
    }
}

#[test]
fn move_framework_prover_tests() {
    run_prover_for_pkg("aptos-framework");
}

#[test]
fn move_token_prover_tests() {
    run_prover_for_pkg("aptos-token");
}

#[test]
fn move_aptos_stdlib_prover_tests() {
    run_prover_for_pkg("aptos-stdlib");
}

#[test]
fn move_stdlib_prover_tests() {
    run_prover_for_pkg("move-stdlib");
}
