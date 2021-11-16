// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use move_cli::package::cli;
use move_core_types::account_address::AccountAddress;
use move_stdlib::{natives::all_natives, path_in_crate};
use tempfile::tempdir;

fn run_tests_for_pkg(path_to_pkg: impl Into<String>) {
    let pkg_path = path_in_crate(path_to_pkg);
    cli::handle_package_commands(
        &Some(pkg_path),
        move_package::BuildConfig {
            test_mode: true,
            install_dir: Some(tempdir().unwrap().path().to_path_buf()),
            ..Default::default()
        },
        &cli::PackageCommand::UnitTest {
            instruction_execution_bound: 100_000,
            list: false,
            num_threads: 8,
            report_statistics: false,
            report_storage_on_error: false,
            check_stackless_vm: false,
            verbose_mode: false,
            compute_coverage: false,
            filter: None,
        },
        all_natives(AccountAddress::from_hex_literal("0x1").unwrap()),
    )
    .unwrap()
}

#[test]
fn move_unit_tests() {
    run_tests_for_pkg(".");
    run_tests_for_pkg("nursery");
}
