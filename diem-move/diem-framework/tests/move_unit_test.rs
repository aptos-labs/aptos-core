// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use diem_framework::path_in_crate;
use diem_vm::natives::diem_natives;
use move_cli::package::cli;

fn run_tests_for_pkg(path_to_pkg: impl Into<String>) {
    let pkg_path = path_in_crate(path_to_pkg);
    cli::handle_package_commands(
        &Some(pkg_path),
        move_package::BuildConfig {
            test_mode: true,
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
        diem_natives(),
    )
    .unwrap()
}

#[test]
fn move_unit_tests() {
    run_tests_for_pkg("core");
    run_tests_for_pkg("experimental");
    run_tests_for_pkg("DPN");
}
