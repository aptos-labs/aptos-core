// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use velor_framework::{extended_checks, path_in_crate, BuildOptions};
use velor_gas_schedule::{MiscGasParameters, NativeGasParameters, LATEST_GAS_FEATURE_VERSION};
use velor_types::on_chain_config::{
    velor_test_feature_flags_genesis, Features, TimedFeaturesBuilder,
};
use velor_vm::natives;
use move_cli::base::test::{run_move_unit_tests, UnitTestResult};
use move_package::CompilerConfig;
use move_unit_test::UnitTestingConfig;
use move_vm_runtime::native_functions::NativeFunctionTable;
use tempfile::tempdir;

fn run_tests_for_pkg(path_to_pkg: impl Into<String>, use_latest_language: bool) {
    let pkg_path = path_in_crate(path_to_pkg);
    let compiler_config = CompilerConfig {
        known_attributes: extended_checks::get_all_attribute_names().clone(),
        ..Default::default()
    };
    let mut build_config = move_package::BuildConfig {
        test_mode: true,
        install_dir: Some(tempdir().unwrap().path().to_path_buf()),
        compiler_config: compiler_config.clone(),
        full_model_generation: true, // Run extended checks also on test code
        ..Default::default()
    };
    if use_latest_language {
        let latest_build_options = BuildOptions::default().set_latest_language();
        build_config.compiler_config.bytecode_version = latest_build_options.bytecode_version;
        build_config.compiler_config.language_version = latest_build_options.language_version;
    }

    let utc = UnitTestingConfig {
        filter: std::env::var("TEST_FILTER").ok(),
        report_statistics: matches!(std::env::var("REPORT_STATS"), Ok(s) if s.as_str() == "1"),
        ..Default::default()
    };
    let ok = run_move_unit_tests(
        &pkg_path,
        build_config.clone(),
        // TODO(Gas): double check if this is correct
        utc,
        velor_test_natives(),
        velor_test_feature_flags_genesis(),
        /* gas limit */ Some(100_000),
        /* cost_table */ None,
        /* compute_coverage */ false,
        &mut std::io::stdout(),
    );
    if ok.is_err() || ok.is_ok_and(|r| r == UnitTestResult::Failure) {
        panic!("move unit tests failed")
    }
}

/// TODO: per @vgao1996:
/// - There should be only one ground truth of `velor_test_natives`.
///   But rn it's defined here, in `move-examples` and in `framework-experimental`.
/// - This function updates a global config (in `configure_extended_checks_for_unit_test`)
///   then returns a list natives. This pattern is confusing.
/// More discussion: https://github.com/velor-chain/velor-core/pull/15997#discussion_r1994469668
pub fn velor_test_natives() -> NativeFunctionTable {
    // By side effect, configure for unit tests
    natives::configure_for_unit_test();
    extended_checks::configure_extended_checks_for_unit_test();
    // move_stdlib has the testing feature enabled to include debug native functions
    natives::velor_natives(
        LATEST_GAS_FEATURE_VERSION,
        NativeGasParameters::zeros(),
        MiscGasParameters::zeros(),
        TimedFeaturesBuilder::enable_all().build(),
        Features::default(),
    )
}

#[test]
fn move_framework_unit_tests() {
    run_tests_for_pkg("velor-framework", false);
}

#[test]
fn move_velor_stdlib_unit_tests() {
    run_tests_for_pkg("velor-stdlib", false);
}

#[test]
fn move_stdlib_unit_tests() {
    run_tests_for_pkg("move-stdlib", false);
}

#[test]
fn move_token_unit_tests() {
    run_tests_for_pkg("velor-token", false);
}

#[test]
fn move_token_objects_unit_tests() {
    run_tests_for_pkg("velor-token-objects", false);
}

#[test]
fn move_experimental_unit_tests() {
    run_tests_for_pkg("velor-experimental", true);
}
