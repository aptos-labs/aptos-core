// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use move_deps::move_cli::base::prove::run_move_prover;
use move_deps::move_prover;
use std::path::PathBuf;
use tempfile::tempdir;

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
    let config = move_deps::move_package::BuildConfig {
        test_mode: true,
        install_dir: Some(tempdir().unwrap().path().to_path_buf()),
        ..Default::default()
    };
    run_move_prover(
        config,
        &pkg_path,
        &None,
        true,
        move_prover::cli::Options::default(),
    )
    .unwrap();
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
