// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    assert_success,
    tests::{
        common::test_dir_path,
        token_objects::{
            create_mint_hero_payload, create_set_hero_description_payload,
            publish_object_token_example,
        },
    },
    MoveHarness,
};
use aptos_cached_packages::{aptos_stdlib, aptos_token_sdk_builder};
use aptos_crypto::{bls12381, PrivateKey, Uniform};
use aptos_gas_algebra::GasQuantity;
use aptos_gas_profiling::TransactionGasLog;
use aptos_language_e2e_tests::account::Account;
use aptos_transaction_generator_lib::{
    entry_point_trait::{EntryPointTrait, MultiSigConfig},
    publishing::publish_util::PackageHandler,
};
use aptos_transaction_workloads_lib::{EntryPoints, LoopType};
use aptos_types::{
    account_address::{default_stake_pool_address, AccountAddress},
    account_config::CORE_CODE_ADDRESS,
    chain_id::ChainId,
    fee_statement::FeeStatement,
    transaction::{EntryFunction, TransactionPayload},
};
use aptos_vm_environment::prod_configs::set_paranoid_type_checks;
use move_core_types::{identifier::Identifier, language_storage::ModuleId, value::MoveValue};
use rand::{rngs::StdRng, SeedableRng};
use sha3::{Digest, Sha3_512};
use std::path::Path;

#[test]
fn test_shelby_audit_sim() {
    let mut h = MoveHarness::new();

    let account = Account::new();
    h.store_and_fund_account(&account, 1000000000000, 0);

    let mut build_options = aptos_framework::BuildOptions::default();
    build_options
        .named_addresses
        .insert("my_addr".to_string(), *account.address());

    let res = h.publish_package_with_options(
        &account,
        &test_dir_path("shelby_audit.data"),
        build_options,
    );

    println!("res: {:?}", res);

    assert_success!(res);

    let txn_payload = TransactionPayload::EntryFunction(EntryFunction::new(
        ModuleId::new(*account.address(), Identifier::new("test").unwrap()),
        Identifier::new("store_new_audit_report").unwrap(),
        vec![],
        vec![MoveValue::U8(0).simple_serialize().unwrap()],
    ));
    let (gas_log, _gas_used, _fee_statement) = h.evaluate_gas_with_profiler(&account, txn_payload);
    gas_log
        .generate_html_report(
            "gas/store-audit-report-0",
            "store-audit-report-0".to_string(),
        )
        .unwrap();

    let txn_payload = TransactionPayload::EntryFunction(EntryFunction::new(
        ModuleId::new(*account.address(), Identifier::new("test").unwrap()),
        Identifier::new("store_new_audit_report").unwrap(),
        vec![],
        vec![MoveValue::U8(1).simple_serialize().unwrap()],
    ));
    let (gas_log, _gas_used, _fee_statement) = h.evaluate_gas_with_profiler(&account, txn_payload);
    gas_log
        .generate_html_report(
            "gas/store-audit-report-1",
            "store-audit-report-1".to_string(),
        )
        .unwrap();
}
