// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Note[Orderless]: Done
use crate::{assert_success, tests::common::test_dir_path, MoveHarness};
use aptos_framework::BuildOptions;
use aptos_language_e2e_tests::account::Account;
use aptos_types::{
    on_chain_config::FeatureFlag,
    transaction::{ExecutionStatus, TransactionStatus},
    write_set::WriteOp,
};
use aptos_vm::testing::{testing_only::inject_error_once, InjectedError};
use move_core_types::account_address::AccountAddress;
use rstest::rstest;
use serde::Serialize;
use std::cmp::max;

#[rstest(
    mod_stateless_account,
    user_stateless_account,
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(true, true, false, false),
    case(true, true, true, false),
    case(true, true, true, true),
    case(true, false, false, false),
    case(true, false, true, false),
    case(true, false, true, true),
    case(false, true, false, false),
    case(false, true, true, false),
    case(false, true, true, true),
    case(false, false, false, false),
    case(false, false, true, false),
    case(false, false, true, true)
)]
fn test_refunds(
    mod_stateless_account: bool,
    user_stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut h = MoveHarness::new_with_flags(use_txn_payload_v2_format, use_orderless_transactions);

    h.enable_features(
        vec![
            FeatureFlag::STORAGE_SLOT_METADATA,
            FeatureFlag::MODULE_EVENT,
            FeatureFlag::EMIT_FEE_STATEMENT,
            FeatureFlag::STORAGE_DELETION_REFUND,
        ],
        vec![],
    );
    // Note: This test uses a lot of execution gas so we need to bump the limit in order for it
    //       to pass.
    h.modify_gas_schedule(|params| {
        params.vm.txn.max_execution_gas = 40_000_000_000.into();
        params.vm.txn.storage_fee_per_state_byte = 0.into(); // tested in DiskSpacePricing.
    });
    let mod_acc = h.new_account_with_key_pair(if mod_stateless_account { None } else { Some(0) });
    let user_acc = h.new_account_with_key_pair(
        if user_stateless_account {
            None
        } else {
            Some(0)
        },
    );
    let mod_addr = *mod_acc.address();
    let mut build_options = BuildOptions::default();
    build_options
        .named_addresses
        .insert("publisher".to_string(), *mod_acc.address());
    assert_success!(h.publish_package_with_options(
        &mod_acc,
        &test_dir_path("storage_refund.data/pack"),
        build_options
    ));
    if !use_orderless_transactions {
        assert_eq!(h.sequence_number_opt(mod_acc.address()), Some(1));
    }

    // store a resource under mod_addr
    let nonce_table_entry_created_writeops = if use_orderless_transactions { 1 } else { 0 };
    assert_succ(
        &mut h,
        mod_addr,
        &mod_acc,
        "store_resource_to",
        vec![],
        1,
        nonce_table_entry_created_writeops,
    );

    // user_addr removes it
    let args = vec![ser(mod_acc.address())];
    let user_acc_creation_writeops = if user_stateless_account && !use_orderless_transactions {
        1
    } else {
        0
    };
    assert_succ(
        &mut h,
        mod_addr,
        &user_acc,
        "remove_resource_from",
        args,
        -1,
        user_acc_creation_writeops + nonce_table_entry_created_writeops,
    );
    if !use_orderless_transactions {
        assert_eq!(h.sequence_number_opt(user_acc.address()), Some(1));
    }

    // initialize global stack and push a few items
    assert_succ(
        &mut h,
        mod_addr,
        &mod_acc,
        "init_stack",
        vec![],
        1,
        nonce_table_entry_created_writeops,
    );
    assert_succ(
        &mut h,
        mod_addr,
        &user_acc,
        "stack_push",
        vec![ser(&10u64)],
        10,
        nonce_table_entry_created_writeops,
    );

    // pop stack items and assert refund amount
    assert_succ(
        &mut h,
        mod_addr,
        &user_acc,
        "stack_pop",
        vec![ser(&2u64)],
        -2,
        nonce_table_entry_created_writeops,
    );
    assert_succ(
        &mut h,
        mod_addr,
        &mod_acc,
        "stack_pop",
        vec![ser(&5u64)],
        -5,
        nonce_table_entry_created_writeops,
    );

    // Inject error in epilogue, observe refund is not applied (slot allocation is still charged.)
    // (need to disable parallel execution)
    inject_error_once(InjectedError::EndOfRunEpilogue);
    assert_result(
        &mut h,
        mod_addr,
        &mod_acc,
        "store_1_pop_2",
        vec![],
        1,
        nonce_table_entry_created_writeops,
        false,
    );

    // Same thing is expected to succeed without injected error. (two slots freed, net refund for 1 slot)
    assert_succ(
        &mut h,
        mod_addr,
        &mod_acc,
        "store_1_pop_2",
        vec![],
        -1,
        nonce_table_entry_created_writeops,
    );

    // Create many slots (with SmartTable)
    assert_succ(
        &mut h,
        mod_addr,
        &user_acc,
        "init_collection_of_1000",
        vec![],
        1025,
        nonce_table_entry_created_writeops,
    );

    // Release many slots.
    assert_succ(
        &mut h,
        mod_addr,
        &user_acc,
        "destroy_collection",
        vec![],
        -1025,
        nonce_table_entry_created_writeops,
    );

    // Create many many slots
    assert_succ(
        &mut h,
        mod_addr,
        &user_acc,
        "init_collection_of_1000",
        vec![],
        1025,
        nonce_table_entry_created_writeops,
    );
    assert_succ(
        &mut h,
        mod_addr,
        &user_acc,
        "grow_collection",
        vec![ser(&1000u64), ser(&6000u64)],
        2977,
        nonce_table_entry_created_writeops,
    );
    assert_succ(
        &mut h,
        mod_addr,
        &user_acc,
        "grow_collection",
        vec![ser(&6000u64), ser(&11000u64)],
        3333,
        nonce_table_entry_created_writeops,
    );
    assert_succ(
        &mut h,
        mod_addr,
        &user_acc,
        "grow_collection",
        vec![ser(&11000u64), ser(&16000u64)],
        3333,
        nonce_table_entry_created_writeops,
    );

    // Try to release the entire collection, expect failure because too many items are being released in one single txn.
    assert_result(
        &mut h,
        mod_addr,
        &user_acc,
        "destroy_collection",
        vec![],
        0,
        nonce_table_entry_created_writeops,
        false,
    );
}

const LEEWAY: u64 = 2000;

fn read_slot_fee_from_gas_schedule(h: &MoveHarness) -> u64 {
    let slot_fee = h
        .get_gas_params()
        .1
        .vm
        .txn
        .storage_fee_per_state_slot
        .into();
    assert!(slot_fee > 0);
    assert!(slot_fee > LEEWAY * 10);
    slot_fee
}

fn ser<T: Serialize>(t: &T) -> Vec<u8> {
    bcs::to_bytes(t).unwrap()
}

fn assert_succ(
    h: &mut MoveHarness,
    mod_addr: AccountAddress,
    account: &Account,
    fun: &str,
    args: Vec<Vec<u8>>,
    expect_num_slots_charged: i64,       // negative for refund
    expect_permanent_slots_created: i64, // due to stateless accounts (creation of 0x1::Account resource), and orderless transactions (creation of nonce entries in nonce tbale)
) {
    assert_result(
        h,
        mod_addr,
        account,
        fun,
        args,
        expect_num_slots_charged,
        expect_permanent_slots_created,
        true,
    );
}

fn assert_result(
    h: &mut MoveHarness,
    mod_address: AccountAddress,
    account: &Account,
    fun: &str,
    args: Vec<Vec<u8>>,
    expect_num_slots_charged: i64, // negative for refund
    expect_permanent_slots_created: i64,
    expect_success: bool,
) {
    let start_balance = h.read_aptos_balance(account.address());

    // run the function
    let txn = h.create_entry_function(
        account,
        format!("{}::test::{}", mod_address, fun).parse().unwrap(),
        vec![],
        args,
    );
    let gas_unit_price = txn.gas_unit_price();
    assert!(gas_unit_price > 0);
    let txn_out = h.run_raw(txn);
    if expect_success {
        assert_success!(*txn_out.status());
    } else {
        assert_ne!(
            *txn_out.status(),
            TransactionStatus::Keep(ExecutionStatus::Success)
        );
    }

    let end_balance = h.read_aptos_balance(account.address());

    // check the creates / deletes in the txn output
    let mut creates = 0;
    let mut deletes = 0;
    for (_state_key, write_op) in txn_out.write_set() {
        match write_op {
            WriteOp::Creation { .. } => creates += 1,
            WriteOp::Deletion(metadata) => {
                if metadata.is_none() {
                    panic!("This test expects all deletions to have metadata")
                }
                deletes += 1
            },
            WriteOp::Modification { .. } => (),
        }
    }
    if expect_success {
        assert_eq!(
            creates - deletes,
            expect_num_slots_charged + expect_permanent_slots_created
        );

        // check the balance
        let slot_fee = read_slot_fee_from_gas_schedule(h);
        let expected_end = (start_balance as i64
            - slot_fee as i64 * (expect_num_slots_charged + expect_permanent_slots_created))
            as u64;
        let leeway = LEEWAY
            * (max(
                1,
                (expect_num_slots_charged + expect_permanent_slots_created).unsigned_abs(),
            ));
        assert!(expected_end + leeway > end_balance);
        assert!(expected_end < end_balance + leeway);
    } else {
        assert!(expect_num_slots_charged + expect_permanent_slots_created >= creates);
    }

    // check the fee statement
    let fee_statement = txn_out.try_extract_fee_statement().unwrap().unwrap();
    let diff_from_fee_statement = fee_statement.storage_fee_refund() as i64
        - (fee_statement.gas_used() * gas_unit_price) as i64;
    assert_eq!(
        diff_from_fee_statement,
        end_balance as i64 - start_balance as i64
    );
}
