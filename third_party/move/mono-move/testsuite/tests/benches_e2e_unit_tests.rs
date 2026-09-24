// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Runs the `benches-e2e` packages' Move unit tests on both VMs.
//!
//! These packages are only ever exercised by the perf harness, which takes
//! hours. Running their unit tests in the normal test job catches a broken
//! package before anyone pays for that.

use aptos_framework::extended_checks;
use aptos_gas_schedule::{MiscGasParameters, NativeGasParameters, LATEST_GAS_FEATURE_VERSION};
use aptos_types::{
    account_address::AccountAddress,
    on_chain_config::{aptos_test_feature_flags_genesis, Features, TimedFeaturesBuilder},
};
use aptos_vm::natives;
use mono_move_testsuite::unit_test;
use move_model::model::GlobalEnv;
use move_package::{source_package::std_lib::StdVersion, BuildConfig, CompilerConfig};
use move_unit_test::{
    package_test::{run_move_unit_tests, UnitTestResult},
    test_validation, UnitTestingConfig,
};
use move_vm_runtime::native_functions::NativeFunctionTable;
use std::{collections::BTreeMap, path::PathBuf};
use tempfile::{tempdir, TempDir};

/// The address `Move.toml` declares for every package in the suite. The value
/// is arbitrary here; on chain the publisher address is substituted in.
const BENCH_ADDRESS: &str = "0xb0";

fn package_path(package: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("benches-e2e")
        .join(package)
}

fn framework_path() -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(4)
        .expect("repository root")
        .join("aptos-move/framework")
        .to_string_lossy()
        .to_string()
}

fn configure_extended_checks_for_unit_test() {
    fn validate(env: &GlobalEnv) {
        extended_checks::run_extended_checks(env);
    }
    test_validation::set_validation_hook(Box::new(validate));
}

fn aptos_test_natives() -> NativeFunctionTable {
    natives::configure_for_unit_test();
    configure_extended_checks_for_unit_test();
    natives::aptos_natives(
        LATEST_GAS_FEATURE_VERSION,
        NativeGasParameters::zeros(),
        MiscGasParameters::zeros(),
        TimedFeaturesBuilder::enable_all().build(),
        Features::default(),
    )
}

fn build_config(install_dir: &TempDir) -> BuildConfig {
    let named_addresses = BTreeMap::from([(
        "bench".to_string(),
        AccountAddress::from_hex_literal(BENCH_ADDRESS).unwrap(),
    )]);
    BuildConfig {
        test_mode: true,
        install_dir: Some(install_dir.path().to_path_buf()),
        override_std: Some(StdVersion::Local(framework_path())),
        additional_named_addresses: named_addresses,
        compiler_config: CompilerConfig {
            known_attributes: extended_checks::get_all_attribute_names().clone(),
            ..Default::default()
        },
        ..Default::default()
    }
}

/// Runs one package on the V1 VM and then on MonoMove, from the same build so
/// both see identical bytecode. A test MonoMove cannot run yet is reported as
/// unsupported; only a test it runs and gets wrong fails.
fn run_package(package: &str) {
    let path = package_path(package);
    // Held for the whole run: dropping it early leaves the build output behind
    // in a directory nothing ever cleans up.
    let install_dir = tempdir().unwrap();
    let config = build_config(&install_dir);

    let result = run_move_unit_tests(
        &path,
        config.clone(),
        UnitTestingConfig::default(),
        aptos_test_natives(),
        aptos_test_feature_flags_genesis(),
        /* gas limit */ Some(1_000_000),
        /* cost_table */ None,
        /* compute_coverage */ false,
        &mut std::io::stdout(),
        true,
    );
    if result.is_err() || result.is_ok_and(|r| r == UnitTestResult::Failure) {
        panic!("V1 unit tests failed for {package}");
    }

    let summary = unit_test::run_package_unit_tests(&path, config)
        .unwrap_or_else(|err| panic!("failed to run MonoMove unit tests for {package}: {err}"));
    println!("{}", summary.render());
    assert!(
        summary.failed.is_empty(),
        "{} test(s) MonoMove ran but got wrong in {package}:\n{}",
        summary.failed.len(),
        summary.failed.join("\n"),
    );
}

#[test]
fn test_clob_avl() {
    run_package("clob_avl");
}

#[test]
fn test_lending_market() {
    run_package("lending_market");
}

#[test]
fn test_clmm_swap() {
    run_package("clmm_swap");
}

#[test]
fn test_stableswap() {
    run_package("stableswap");
}

#[test]
fn test_bridge_relay() {
    run_package("bridge_relay");
}

#[test]
fn test_airdrop_fanout() {
    run_package("airdrop_fanout");
}

#[test]
fn test_oracle_batch() {
    run_package("oracle_batch");
}

#[test]
fn test_cdp_liquidation() {
    run_package("cdp_liquidation");
}

#[test]
fn test_dex_aggregator() {
    run_package("dex_aggregator");
}

#[test]
fn test_nft_mint_market() {
    run_package("nft_mint_market");
}
