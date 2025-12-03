// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{assert_success, tests::common, MoveHarness};
use aptos_language_e2e_tests::executor::{ExecutorMode, FakeExecutor};
use aptos_types::transaction::{
    EntryFunction, ExecutionStatus, TransactionPayload, TransactionStatus,
};
use aptos_vm_environment::prod_configs::set_layout_caches;
use move_core_types::{
    account_address::AccountAddress, ident_str, language_storage::ModuleId, vm_status::StatusCode,
};
use std::{collections::BTreeSet, str::FromStr};

#[test]
fn test_layout_cache_successful_reads() {
    set_layout_caches(true);

    let executor =
        FakeExecutor::from_head_genesis().set_executor_mode(ExecutorMode::BothComparison);
    let mut h = MoveHarness::new_with_executor(executor);
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());

    assert_success!(
        h.publish_package_cache_building(&acc, &common::test_dir_path("layout_caches.data/p1"))
    );
    assert_success!(
        h.publish_package_cache_building(&acc, &common::test_dir_path("layout_caches.data/p2"))
    );
    assert_success!(
        h.publish_package_cache_building(&acc, &common::test_dir_path("layout_caches.data/p3"))
    );

    let mut txns = vec![];
    for i in 0..32 {
        let account =
            h.new_account_at(AccountAddress::from_hex_literal(&format!("0xcafe{}", i)).unwrap());
        let txn = h.create_transaction_payload(
            &account,
            TransactionPayload::EntryFunction(EntryFunction::new(
                ModuleId::from_str("0xcafe::m3").unwrap(),
                ident_str!("load_m3_with_extra_module").to_owned(),
                vec![],
                vec![],
            )),
        );
        txns.push(txn);
    }

    let mut gas_usage = BTreeSet::new();
    let outputs = h.run_block_get_output(txns);
    for output in outputs {
        assert_success!(output.status().clone());
        gas_usage.insert(output.gas_used());
    }
    assert_eq!(gas_usage.len(), 1);

    h.modify_gas_schedule(|gas_params| {
        gas_params.vm.txn.max_num_dependencies = 3.into();
    });

    let mut txns = vec![];
    for i in 0..32 {
        let account =
            h.new_account_at(AccountAddress::from_hex_literal(&format!("0xcafe2{}", i)).unwrap());
        let func_name = if i <= 15 {
            ident_str!("load_m3")
        } else {
            ident_str!("load_m3_with_extra_module")
        };
        let txn = h.create_transaction_payload(
            &account,
            TransactionPayload::EntryFunction(EntryFunction::new(
                ModuleId::from_str("0xcafe::m3").unwrap(),
                func_name.to_owned(),
                vec![],
                vec![],
            )),
        );
        txns.push(txn);
    }

    let mut success_gas_usage = BTreeSet::new();
    let mut failure_gas_usage = BTreeSet::new();
    let outputs = h.run_block_get_output(txns);
    for (i, output) in outputs.iter().enumerate() {
        if i <= 15 {
            assert_success!(output.status().clone());
            success_gas_usage.insert(output.gas_used());
        } else {
            assert!(matches!(
                output.status(),
                TransactionStatus::Keep(ExecutionStatus::MiscellaneousError(Some(
                    StatusCode::DEPENDENCY_LIMIT_REACHED
                )))
            ));
            failure_gas_usage.insert(output.gas_used());
        }
    }
    assert_eq!(success_gas_usage.len(), 1);
    assert_eq!(failure_gas_usage.len(), 1);

    h.modify_gas_schedule(|gas_params| {
        gas_params.vm.txn.max_num_dependencies = 8.into();
    });

    let mut txns = vec![];
    for i in 0..32 {
        let account =
            h.new_account_at(AccountAddress::from_hex_literal(&format!("0xcafe3{}", i)).unwrap());
        let txn = if i == 15 {
            h.create_publish_package_cache_building(
                &acc,
                &common::test_dir_path("layout_caches.data/p3_upgraded"),
                |_| {},
            )
        } else {
            h.create_transaction_payload(
                &account,
                TransactionPayload::EntryFunction(EntryFunction::new(
                    ModuleId::from_str("0xcafe::m3").unwrap(),
                    ident_str!("load_m3_with_extra_module").to_owned(),
                    vec![],
                    vec![],
                )),
            )
        };
        txns.push(txn);
    }

    let mut success_gas_usage = BTreeSet::new();
    let mut failure_gas_usage = BTreeSet::new();
    let outputs = h.run_block_get_output(txns);
    for (i, output) in outputs.iter().enumerate() {
        if i < 15 {
            assert_success!(output.status().clone());
            success_gas_usage.insert(output.gas_used());
        } else if i == 15 {
            // Publishing succeeds.
            assert_success!(output.status().clone());
        } else {
            // Transactions after publish fail on dependency limit reached (new layout is read).
            assert!(matches!(
                output.status(),
                TransactionStatus::Keep(ExecutionStatus::MiscellaneousError(Some(
                    StatusCode::DEPENDENCY_LIMIT_REACHED
                )))
            ));
            failure_gas_usage.insert(output.gas_used());
        }
    }
    assert_eq!(success_gas_usage.len(), 1);
    assert_eq!(failure_gas_usage.len(), 1);
}
