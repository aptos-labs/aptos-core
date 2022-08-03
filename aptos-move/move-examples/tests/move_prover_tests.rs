// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_types::account_address::AccountAddress;
use move_deps::move_cli::base::prove::run_move_prover;
use move_deps::move_prover;
use std::{collections::BTreeMap, path::PathBuf};
use tempfile::tempdir;

pub fn path_in_crate<S>(relative: S) -> PathBuf
where
    S: Into<String>,
{
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push(relative.into());
    path
}

pub fn run_prover_for_pkg(
    path_to_pkg: impl Into<String>,
    named_addr: BTreeMap<String, AccountAddress>,
) {
    let pkg_path = path_in_crate(path_to_pkg);
    let config = move_deps::move_package::BuildConfig {
        additional_named_addresses: named_addr,
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
fn test_hello_prover() {
    let named_address = BTreeMap::new();
    run_prover_for_pkg("hello_prover", named_address);
}
