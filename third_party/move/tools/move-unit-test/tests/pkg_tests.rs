// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use move_cli::base::test::{run_move_unit_tests, UnitTestResult};
use move_core_types::{account_address::AccountAddress, effects::ChangeSet};
use move_model::metadata::CompilerVersion;
use move_package::CompilerConfig;
use move_stdlib::natives::{all_natives, GasParameters};
use move_unit_test::UnitTestingConfig;
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

fn run_tests_for_pkg(path_to_pkg: impl Into<String>, v2: bool) {
    let pkg_path = path_in_crate(path_to_pkg);

    let natives = all_natives(
        AccountAddress::from_hex_literal("0x1").unwrap(),
        GasParameters::zeros(),
    );

    let result = run_move_unit_tests(
        &pkg_path,
        move_package::BuildConfig {
            test_mode: true,
            install_dir: Some(tempdir().unwrap().path().to_path_buf()),
            compiler_config: CompilerConfig {
                compiler_version: if v2 {
                    Some(CompilerVersion::latest())
                } else {
                    None
                },
                ..Default::default()
            },
            ..Default::default()
        },
        UnitTestingConfig::default(),
        natives,
        ChangeSet::new(),
        /* gas_limit */ Some(100_000),
        /* cost_table */ None,
        /* compute_coverage */ false,
        &mut std::io::stdout(),
    );
    if result.is_err() || result.is_ok_and(|r| r == UnitTestResult::Failure) {
        panic!("aborting because of Move unit test failures")
    }
}

#[test]
fn one_bytecode_dep() {
    // TODO: automatically discovers all Move packages under a package directory and runs unit tests for them
    run_tests_for_pkg("tests/packages/one-bytecode-dep", true);
    run_tests_for_pkg("tests/packages/one-bytecode-dep", false);
}
