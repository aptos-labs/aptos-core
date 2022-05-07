// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_vm::natives::aptos_natives;
use framework::path_in_crate;
use move_deps::{move_cli::package::cli, move_unit_test::UnitTestingConfig};
use tempfile::tempdir;

fn run_tests_for_pkg(path_to_pkg: impl Into<String>) {
    let pkg_path = path_in_crate(path_to_pkg);
    cli::run_move_unit_tests(
        &pkg_path,
        move_deps::move_package::BuildConfig {
            test_mode: true,
            install_dir: Some(tempdir().unwrap().path().to_path_buf()),
            ..Default::default()
        },
        UnitTestingConfig::default_with_bound(Some(100_000)),
        aptos_natives(),
        /* compute_coverage */ false,
    )
    .unwrap();
}
#[test]
fn move_unit_tests() {
    run_tests_for_pkg("aptos-framework");
}
