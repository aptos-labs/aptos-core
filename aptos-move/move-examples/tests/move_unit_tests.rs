// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_framework::extended_checks;
use aptos_gas_schedule::{MiscGasParameters, NativeGasParameters, LATEST_GAS_FEATURE_VERSION};
use aptos_types::{
    account_address::{create_resource_address, AccountAddress},
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
use tempfile::tempdir;

pub fn path_in_crate<S>(relative: S) -> PathBuf
where
    S: Into<String>,
{
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push(relative.into());
    path
}

/// Configures the unit test validation hook to run the extended checker.
fn configure_extended_checks_for_unit_test() {
    fn validate(env: &GlobalEnv) {
        extended_checks::run_extended_checks(env);
    }
    test_validation::set_validation_hook(Box::new(validate));
}

fn build_config(named_addr: BTreeMap<String, AccountAddress>) -> BuildConfig {
    BuildConfig {
        test_mode: true,
        install_dir: Some(tempdir().unwrap().path().to_path_buf()),
        override_std: Some(StdVersion::Local(get_local_framework_path())),
        additional_named_addresses: named_addr,
        compiler_config: CompilerConfig {
            known_attributes: extended_checks::get_all_attribute_names().clone(),
            ..Default::default()
        },
        ..Default::default()
    }
}

/// Runs a package's unit tests on the V1 VM and then on MonoMove, from the
/// same build configuration so that both see identical bytecode.
///
/// A test MonoMove cannot run yet is reported as unsupported and does not fail
/// the run; only a test it runs and gets wrong is a failure.
pub fn run_tests_for_pkg(
    path_to_pkg: impl Into<String>,
    named_addr: BTreeMap<String, AccountAddress>,
) {
    let pkg_path = path_in_crate(path_to_pkg);
    let config = build_config(named_addr);

    let ok = run_move_unit_tests(
        &pkg_path,
        config.clone(),
        UnitTestingConfig::default(),
        // TODO(Gas): we may want to switch to non-zero costs in the future
        aptos_test_natives(),
        aptos_test_feature_flags_genesis(),
        /* gas limit */ Some(100_000),
        /* cost_table */ None,
        /* compute_coverage */ false,
        &mut std::io::stdout(),
        true,
    );
    if ok.is_err() || ok.is_ok_and(|r| r == UnitTestResult::Failure) {
        panic!("move unit tests failed")
    }

    let summary = unit_test::run_package_unit_tests(&pkg_path, config)
        .unwrap_or_else(|err| panic!("failed to run MonoMove unit tests: {err}"));
    println!("{}", summary.render());
    assert!(
        summary.failed.is_empty(),
        "{} test(s) MonoMove ran but got wrong:\n{}",
        summary.failed.len(),
        summary.failed.join("\n"),
    );
}

pub fn aptos_test_natives() -> NativeFunctionTable {
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

/// Get the local framework path based on this source file's location.
/// Note: If this source file is moved to a different location, this function
/// may need to be updated.
fn get_local_framework_path() -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(|p| p.join("framework"))
        .expect("framework path")
        .to_string_lossy()
        .to_string()
}

fn test_common(pkg: &str) {
    let named_address = BTreeMap::from([(
        String::from(pkg),
        AccountAddress::from_hex_literal("0xf00d").unwrap(),
    )]);
    run_tests_for_pkg(pkg, named_address);
}

fn test_resource_account_common(pkg: &str) {
    let named_address = BTreeMap::from([(
        String::from(pkg),
        create_resource_address(AccountAddress::from_hex_literal("0xcafe").unwrap(), &[]),
    )]);
    run_tests_for_pkg(pkg, named_address);
}

#[test]
fn test_vector_pushback() {
    let named_address = BTreeMap::new();
    run_tests_for_pkg("vector_pushback", named_address);
}

#[test]
fn test_fixed_point64() {
    let named_address = BTreeMap::new();
    run_tests_for_pkg("fixed_point64", named_address);
}

#[test]
#[should_panic(expected = "move unit tests failed")]
fn test_duplicate_scripts() {
    run_tests_for_pkg("duplicate_scripts", BTreeMap::new());
}

#[test]
fn test_common_account() {
    test_common("common_account");
}

#[test]
fn test_data_structures() {
    test_common("data_structures");
}

#[test]
fn test_defi() {
    test_common("defi");
}

#[test]
fn test_groth16() {
    test_common("groth16_example");
}

#[test]
fn test_hello_blockchain() {
    test_common("hello_blockchain");
}

#[test]
fn test_drand_lottery() {
    test_common("drand");
}

#[test]
fn test_raffle() {
    test_common("raffle");
}

#[test]
fn test_marketplace() {
    test_common("marketplace")
}

#[test]
fn test_message_board() {
    test_common("message_board");
}

#[test]
fn test_dispatching() {
    test_common("dispatching");
}

#[test]
fn test_fungible_asset() {
    let named_address = BTreeMap::from([
        (
            String::from("example_addr"),
            AccountAddress::from_hex_literal("0xcafe").unwrap(),
        ),
        (
            String::from("FACoin"),
            AccountAddress::from_hex_literal("0xcafe").unwrap(),
        ),
    ]);
    run_tests_for_pkg(
        "fungible_asset/managed_fungible_asset",
        named_address.clone(),
    );
    run_tests_for_pkg(
        "fungible_asset/managed_fungible_token",
        named_address.clone(),
    );
    run_tests_for_pkg(
        "fungible_asset/preminted_managed_coin",
        named_address.clone(),
    );
    run_tests_for_pkg("fungible_asset/fa_coin", named_address);
}

#[test]
fn test_mint_nft() {
    let addr = AccountAddress::from_hex_literal("0xcafe").unwrap();
    let named_address = BTreeMap::from([
        (String::from("mint_nft"), create_resource_address(addr, &[])),
        (String::from("source_addr"), addr),
    ]);
    run_tests_for_pkg("mint_nft/4-Getting-Production-Ready", named_address);
}

#[test]
fn test_minter() {
    run_tests_for_pkg("scripts/minter", BTreeMap::new());
}

#[test]
fn test_resource_account() {
    test_resource_account_common("resource_account");
}

#[test]
fn test_resource_groups() {
    let named_address = BTreeMap::from([
        (
            String::from("resource_groups_primary"),
            AccountAddress::from_hex_literal("0xf00d").unwrap(),
        ),
        (
            String::from("resource_groups_secondary"),
            AccountAddress::from_hex_literal("0xcafe").unwrap(),
        ),
    ]);
    run_tests_for_pkg("resource_groups/primary", named_address.clone());
    run_tests_for_pkg("resource_groups/secondary", named_address);
}

#[test]
fn test_shared_account() {
    test_common("shared_account");
}

#[test]
fn test_token_objects() {
    let named_addresses = BTreeMap::from([
        (
            String::from("ambassador"),
            AccountAddress::from_hex_literal("0xcafe").unwrap(),
        ),
        (
            String::from("hero"),
            AccountAddress::from_hex_literal("0xcafe").unwrap(),
        ),
        (
            String::from("knight"),
            AccountAddress::from_hex_literal("0xcafe").unwrap(),
        ),
        (
            String::from("token_lockup"),
            AccountAddress::from_hex_literal("0xcafe").unwrap(),
        ),
    ]);
    run_tests_for_pkg("token_objects/ambassador", named_addresses.clone());
    run_tests_for_pkg("token_objects/hero", named_addresses.clone());
    run_tests_for_pkg("token_objects/knight", named_addresses.clone());
    run_tests_for_pkg("token_objects/token_lockup", named_addresses);
}

#[test]
fn test_two_by_two_transfer() {
    run_tests_for_pkg("scripts/two_by_two_transfer", BTreeMap::new());
}

#[test]
fn test_post_mint_reveal_nft() {
    let addr = AccountAddress::from_hex_literal("0xcafe").unwrap();
    let named_address = BTreeMap::from([(String::from("post_mint_reveal_nft"), addr)]);
    run_tests_for_pkg("post_mint_reveal_nft", named_address);
}

#[test]
fn test_nft_dao_test() {
    let named_address = BTreeMap::from([(
        String::from("dao_platform"),
        AccountAddress::from_hex_literal("0xcafe").unwrap(),
    )]);
    run_tests_for_pkg("dao/nft_dao", named_address);
}

#[test]
fn test_swap() {
    let named_address = BTreeMap::from([
        (
            String::from("deployer"),
            AccountAddress::from_hex_literal("0xcafe").unwrap(),
        ),
        (
            String::from("swap"),
            AccountAddress::from_hex_literal("0xcafe").unwrap(),
        ),
    ]);
    run_tests_for_pkg("swap", named_address);
}

#[test]
fn test_package_manager() {
    let named_address = BTreeMap::from([
        (
            String::from("deployer"),
            AccountAddress::from_hex_literal("0xcafe").unwrap(),
        ),
        (
            String::from("package"),
            AccountAddress::from_hex_literal("0xcafe").unwrap(),
        ),
    ]);
    run_tests_for_pkg("package_manager", named_address);
}

#[test]
fn test_rewards_pool() {
    test_common("rewards_pool");
}

#[test]
fn test_ring_deque() {
    test_common("ring_deque");
}

#[test]
fn test_event() {
    test_common("event");
}

#[test]
fn test_tic_tac_toe() {
    let named_address = BTreeMap::from([(
        String::from("tic_tac_toe"),
        AccountAddress::from_hex_literal("0xf00d").unwrap(),
    )]);
    run_tests_for_pkg("tic-tac-toe", named_address);
}

#[test]
fn test_my_first_dapp() {
    let named_address = BTreeMap::from([(
        String::from("todolist_addr"),
        AccountAddress::from_hex_literal("0xf00d").unwrap(),
    )]);
    run_tests_for_pkg("my_first_dapp/contract", named_address);
}

#[test]
fn test_large_package_example() {
    let named_address = BTreeMap::from([(
        String::from("large_package_example"),
        AccountAddress::from_hex_literal("0xf00d").unwrap(),
    )]);
    run_tests_for_pkg("large_packages/large_package_example", named_address);
}

#[test]
fn test_struct_enum_args() {
    let named_address = BTreeMap::from([(
        String::from("struct_enum_tests"),
        AccountAddress::from_hex_literal("0xf00d").unwrap(),
    )]);
    run_tests_for_pkg("cli-e2e-tests/struct-enum-args", named_address);
}

// `staking` and `bcs-stream` bind every address in their own manifest, so they
// need nothing extra here.

#[test]
fn test_staking() {
    run_tests_for_pkg("staking", BTreeMap::new());
}

#[test]
fn test_bcs_stream() {
    run_tests_for_pkg("bcs-stream", BTreeMap::new());
}

/// The tutorial steps that carry `#[test]` functions. The remaining steps are
/// compile-only exercises.
#[test]
fn test_move_tutorial() {
    for step in [
        "step_2", "step_2_sol", "step_5", "step_5_sol", "step_6",
    ] {
        run_tests_for_pkg(format!("move-tutorial/{step}/basic_coin"), BTreeMap::new());
    }
}
