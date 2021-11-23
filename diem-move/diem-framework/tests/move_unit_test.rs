// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use diem_framework::path_in_crate;
use diem_vm::natives::diem_natives;
use move_cli::package::cli;
use move_unit_test::UnitTestingConfig;
use tempfile::tempdir;

fn run_tests_for_pkg(path_to_pkg: impl Into<String>) {
    let pkg_path = path_in_crate(path_to_pkg);
    cli::run_move_unit_tests(
        &pkg_path,
        move_package::BuildConfig {
            test_mode: true,
            install_dir: Some(tempdir().unwrap().path().to_path_buf()),
            ..Default::default()
        },
        UnitTestingConfig::default_with_bound(Some(100_000)),
        diem_natives(),
        /* compute_coverage */ false,
    )
    .unwrap();
}

#[test]
fn move_unit_tests() {
    run_tests_for_pkg("core");
    run_tests_for_pkg("experimental");
    run_tests_for_pkg("DPN");
}
