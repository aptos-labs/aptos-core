// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! `reflect::resolve` bounds the length of the names it resolves, and charges for a resolution
//! that finds no function.

use crate::{assert_success, tests::common, MoveHarness};
use aptos_framework::BuildOptions;
use aptos_gas_schedule::gas_feature_versions::RELEASE_V1_50;
use aptos_language_e2e_tests::account::Account;
use aptos_package_builder::PackageBuilder;
use aptos_types::{
    account_address::AccountAddress,
    move_stdlib::reflect::INVALID_IDENTIFIER,
    on_chain_config::FeatureFlag,
    transaction::{AuxiliaryInfo, TransactionStatus},
};
use move_core_types::{
    function::FunctionResolutionError,
    gas_algebra::{InternalGas, NumBytes},
};

const MAX_NAME_LEN: u64 = 255;
const FUNCTION_NOT_FOUND: u64 = FunctionResolutionError::FunctionNotFound as u64;

fn probe_source() -> String {
    let longest_function_name = "a".repeat(MAX_NAME_LEN as usize);
    format!(
        r#"
module 0xcafe::probe {{
    use std::reflect;
    use std::string::{{Self, String}};

    public fun {longest_function_name}(value: u64): u64 {{ value }}

    public entry fun resolve_function_of_len(len: u64) {{
        let resolved: |u64|u64 =
            reflect::resolve(@0xcafe, &string::utf8(b"probe"), &name_of_len(len)).unwrap();
        assert!(resolved(7) == 7, 1);
    }}

    public entry fun check_function_name(len: u64, expected_code: u64) {{
        let err = reflect::resolve<|u64|u64>(
            @0xcafe, &string::utf8(b"probe"), &name_of_len(len)).unwrap_err();
        assert!(err.error_code() == expected_code, 2);
    }}

    public entry fun check_module_name(len: u64, expected_code: u64) {{
        let err = reflect::resolve<|u64|u64>(
            @0x1, &name_of_len(len), &string::utf8(b"f")).unwrap_err();
        assert!(err.error_code() == expected_code, 3);
    }}

    fun name_of_len(len: u64): String {{
        let bytes = vector[];
        for (_index in 0..len) {{
            bytes.push_back(0x61);
        }};
        string::utf8(bytes)
    }}
}}
"#
    )
}

/// Publishes the probe module.
fn setup() -> (MoveHarness, Account) {
    let mut harness =
        MoveHarness::new_with_features(vec![FeatureFlag::ENABLE_FUNCTION_REFLECTION], vec![]);
    let mut builder = PackageBuilder::new("Probe");
    builder.add_source("probe.move", &probe_source());
    builder.add_local_dep(
        "MoveStdlib",
        &common::framework_dir_path("move-stdlib").to_string_lossy(),
    );
    let dir = builder.write_to_temp().expect("package writes to temp dir");
    let account =
        harness.new_account_at(AccountAddress::from_hex_literal("0xcafe").expect("valid address"));
    assert_success!(harness.publish_package_with_options(
        &account,
        dir.path(),
        BuildOptions::move_2().set_latest_language()
    ));
    (harness, account)
}

fn run(
    harness: &mut MoveHarness,
    account: &Account,
    function: &str,
    args: Vec<Vec<u8>>,
) -> TransactionStatus {
    harness.run_entry_function(
        account,
        str::parse(&format!("0xcafe::probe::{function}")).expect("valid function id"),
        vec![],
        args,
    )
}

fn len_and_code_args(len: u64, expected_code: u64) -> Vec<Vec<u8>> {
    vec![
        bcs::to_bytes(&len).expect("u64 serializes"),
        bcs::to_bytes(&expected_code).expect("u64 serializes"),
    ]
}

#[test]
fn resolve_rejects_names_over_identifier_size_limit() {
    let (mut harness, account) = setup();
    let invalid = INVALID_IDENTIFIER as u64;

    assert_success!(run(
        &mut harness,
        &account,
        "resolve_function_of_len",
        vec![bcs::to_bytes(&MAX_NAME_LEN).expect("u64 serializes")]
    ));
    assert_success!(run(
        &mut harness,
        &account,
        "check_function_name",
        len_and_code_args(MAX_NAME_LEN + 1, invalid)
    ));
    assert_success!(run(
        &mut harness,
        &account,
        "check_module_name",
        len_and_code_args(MAX_NAME_LEN, FUNCTION_NOT_FOUND)
    ));
    assert_success!(run(
        &mut harness,
        &account,
        "check_module_name",
        len_and_code_args(MAX_NAME_LEN + 1, invalid)
    ));
}

#[test]
fn resolve_limits_apply_from_gas_feature_version_v1_51() {
    let (mut harness, account) = setup();
    harness.modify_gas_schedule_raw(|schedule| schedule.feature_version = RELEASE_V1_50);

    assert_success!(run(
        &mut harness,
        &account,
        "check_module_name",
        len_and_code_args(MAX_NAME_LEN + 1, FUNCTION_NOT_FOUND)
    ));
}

/// Runs `0xcafe::probe::{function}` and returns the cost of its one `native_resolve` call.
fn profile_native_resolve(
    harness: &mut MoveHarness,
    account: &Account,
    function: &str,
    args: Vec<Vec<u8>>,
) -> InternalGas {
    let txn = harness.create_entry_function(
        account,
        str::parse(&format!("0xcafe::probe::{function}")).expect("valid function id"),
        vec![],
        args,
    );
    let (status, gas_log, _, _) =
        harness.evaluate_gas_with_profiler_and_status_signed(txn, &AuxiliaryInfo::default());
    assert_success!(status);

    let native_costs: Vec<_> = gas_log
        .exec_io
        .aggregate_gas_events()
        .ops
        .into_iter()
        .filter(|(op, _, _)| op.starts_with("0x1::reflect::native_resolve"))
        .map(|(_, hits, cost)| (hits, cost))
        .collect();
    let [(1, native_cost)] = native_costs[..] else {
        panic!("expected one native_resolve call, got {native_costs:?}");
    };
    native_cost
}

#[test]
fn resolve_charges_apply_from_gas_feature_version_v1_51() {
    let name_len = 100;
    let probe_module_len = "probe".len() as u64;
    let function_f_len = "f".len() as u64;

    let (mut harness, account) = setup();
    let params = harness.get_gas_params().1.natives.aptos_framework;
    assert!(!params.reflect_resolve_per_byte.is_zero());
    assert!(!params.reflect_resolve_not_found.is_zero());
    let found_cost = |name_bytes: u64| {
        params.reflect_resolve_base + params.reflect_resolve_per_byte * NumBytes::new(name_bytes)
    };
    let not_found_cost =
        |name_bytes: u64| found_cost(name_bytes) + params.reflect_resolve_not_found;

    // Missing module of `name_len` bytes at `0x1`, with function `f`.
    let missing_module_args = len_and_code_args(name_len, FUNCTION_NOT_FOUND);
    assert_eq!(
        profile_native_resolve(
            &mut harness,
            &account,
            "check_module_name",
            missing_module_args.clone()
        ),
        not_found_cost(name_len + function_f_len)
    );
    // Module `probe`, with a missing function of `name_len` bytes.
    assert_eq!(
        profile_native_resolve(
            &mut harness,
            &account,
            "check_function_name",
            len_and_code_args(name_len, FUNCTION_NOT_FOUND)
        ),
        not_found_cost(probe_module_len + name_len)
    );
    // Module `probe`, with its function of `MAX_NAME_LEN` bytes.
    assert_eq!(
        profile_native_resolve(&mut harness, &account, "resolve_function_of_len", vec![
            bcs::to_bytes(&MAX_NAME_LEN).expect("u64 serializes")
        ]),
        found_cost(probe_module_len + MAX_NAME_LEN)
    );

    harness.modify_gas_schedule_raw(|schedule| schedule.feature_version = RELEASE_V1_50);
    assert_eq!(
        profile_native_resolve(
            &mut harness,
            &account,
            "check_module_name",
            missing_module_args
        ),
        params.reflect_resolve_base
    );
}
