// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use move_cli::base::test::run_move_unit_tests;
use move_core_types::{account_address::AccountAddress, effects::ChangeSet};
use move_model::metadata::CompilerVersion;
use move_package::CompilerConfig;
use move_stdlib::natives::{all_natives, GasParameters};
use move_unit_test::UnitTestingConfig;
use regex::Regex;
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

fn run_tests_for_pkg(path_to_pkg: impl Into<String>, fail_fast: bool) {
    let pkg_path = path_in_crate(path_to_pkg);

    let natives = all_natives(
        AccountAddress::from_hex_literal("0x1").unwrap(),
        GasParameters::zeros(),
    );

    let unit_test_config = UnitTestingConfig {
        report_statistics: true,
        fail_fast,
        ..Default::default()
    };
    let mut out: Vec<u8> = Vec::new();
    let _result = run_move_unit_tests(
        &pkg_path,
        move_package::BuildConfig {
            test_mode: true,
            install_dir: Some(tempdir().unwrap().path().to_path_buf()),
            compiler_config: CompilerConfig {
                compiler_version: Some(CompilerVersion::latest()),
                ..Default::default()
            },
            ..Default::default()
        },
        unit_test_config,
        natives,
        ChangeSet::new(),
        /* gas_limit */ Some(100_000),
        /* cost_table */ None,
        /* compute_coverage */ false,
        &mut out,
    );
    let output = String::from_utf8_lossy(&out);
    let failed_re = Regex::new(r"failed:\s*(\d+)").unwrap();
    let pass_re = Regex::new(r"passed:\s*(\d+)").unwrap();

    let passed = if let Some(caps) = pass_re.captures(&output) {
        caps[1].parse().unwrap()
    } else {
        0
    };
    let failed = if let Some(caps) = failed_re.captures(&output) {
        caps[1].parse().unwrap()
    } else {
        0
    };
    if fail_fast {
        assert_eq!(
            failed, 1,
            "Expected exactly one failure in fail-fast mode, got output:\n{}",
            output
        );
        // we can't guarantee how many tests passed before the failure, test order is non-deterministic
    } else {
        // Without fail-fast we expect all tests to run and report 2 failures and 2 passes
        assert_eq!(
            failed, 2,
            "Expected exactly two failures without fail-fast mode, got output:\n{}",
            output
        );
        assert_eq!(
            passed, 2,
            "Expected exactly two passing tests, got output:\n{}",
            output
        );
    }
}

#[test]
fn fail_fast() {
    run_tests_for_pkg("tests/fail_fast/", true);
}

#[test]
fn no_fail_fast() {
    run_tests_for_pkg("tests/fail_fast/", false);
}
