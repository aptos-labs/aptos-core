// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{assert_success, tests::common::test_dir_path, MoveHarness};
use velor_language_e2e_tests::account::Account;
use velor_types::{
    on_chain_config::FeatureFlag,
    transaction::{ExecutionStatus, TransactionStatus},
    write_set::BaseStateOp,
};
use velor_vm::testing::{testing_only::inject_error_once, InjectedError};
use move_core_types::account_address::AccountAddress;
use serde::Serialize;

#[test]
fn test_refunds() {
    let mut h = MoveHarness::new_with_features(
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
    let mod_addr = AccountAddress::from_hex_literal("0xcafe").unwrap();
    let user_addr = AccountAddress::from_hex_literal("0x100").unwrap();
    let mod_acc = h.new_account_at(mod_addr);
    let user_acc = h.new_account_at(user_addr);

    assert_success!(h.publish_package(&mod_acc, &test_dir_path("storage_refund.data/pack")));

    // store a resource under 0xcafe
    assert_succ(&mut h, &mod_acc, "store_resource_to", vec![], 1);

    // 0x100 removes it
    let args = vec![ser(&mod_addr)];
    assert_succ(&mut h, &user_acc, "remove_resource_from", args, -1);

    // initialize global stack and push a few items
    assert_succ(&mut h, &mod_acc, "init_stack", vec![], 1);
    assert_succ(&mut h, &user_acc, "stack_push", vec![ser(&10u64)], 10);

    // pop stack items and assert refund amount
    assert_succ(&mut h, &user_acc, "stack_pop", vec![ser(&2u64)], -2);
    assert_succ(&mut h, &mod_acc, "stack_pop", vec![ser(&5u64)], -5);

    // Inject error in epilogue, observe refund is not applied (slot allocation is still charged.)
    // (need to disable parallel execution)
    inject_error_once(InjectedError::EndOfRunEpilogue);
    assert_result(&mut h, &mod_acc, "store_1_pop_2", vec![], 1, false);

    // Same thing is expected to succeed without injected error. (two slots freed, net refund for 1 slot)
    assert_succ(&mut h, &mod_acc, "store_1_pop_2", vec![], -1);

    // Create many slots (with SmartTable)
    assert_succ(&mut h, &user_acc, "init_collection_of_1000", vec![], 1025);

    // Release many slots.
    assert_succ(&mut h, &user_acc, "destroy_collection", vec![], -1025);

    // Create many many slots
    assert_succ(&mut h, &user_acc, "init_collection_of_1000", vec![], 1025);
    assert_succ(
        &mut h,
        &user_acc,
        "grow_collection",
        vec![ser(&1000u64), ser(&6000u64)],
        2977,
    );
    assert_succ(
        &mut h,
        &user_acc,
        "grow_collection",
        vec![ser(&6000u64), ser(&11000u64)],
        3333,
    );
    assert_succ(
        &mut h,
        &user_acc,
        "grow_collection",
        vec![ser(&11000u64), ser(&16000u64)],
        3333,
    );

    // Try to release the entire collection, expect failure because too many items are being released in one single txn.
    assert_result(&mut h, &user_acc, "destroy_collection", vec![], 0, false);
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
    account: &Account,
    fun: &str,
    args: Vec<Vec<u8>>,
    expect_num_slots_charged: i64, // negative for refund
) {
    assert_result(h, account, fun, args, expect_num_slots_charged, true);
}

fn assert_result(
    h: &mut MoveHarness,
    account: &Account,
    fun: &str,
    args: Vec<Vec<u8>>,
    expect_num_slots_charged: i64, // negative for refund
    expect_success: bool,
) {
    let start_balance = h.read_velor_balance(account.address());

    // run the function
    let txn = h.create_entry_function(
        account,
        format!("0xcafe::test::{}", fun).parse().unwrap(),
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

    let end_balance = h.read_velor_balance(account.address());

    // check the creates / deletes in the txn output
    let mut creates = 0;
    let mut deletes = 0;
    for (_state_key, write_op) in txn_out.write_set().write_op_iter() {
        match write_op.as_base_op() {
            BaseStateOp::Creation { .. } => creates += 1,
            BaseStateOp::Deletion(metadata) => {
                if metadata.is_none() {
                    panic!("This test expects all deletions to have metadata")
                }
                deletes += 1
            },
            BaseStateOp::Modification { .. } => (),
            BaseStateOp::MakeHot { .. } | BaseStateOp::Eviction { .. } => unreachable!(),
        }
    }
    if expect_success {
        assert_eq!(creates - deletes, expect_num_slots_charged);

        // check the balance
        let slot_fee = read_slot_fee_from_gas_schedule(h);
        let expected_end =
            (start_balance as i64 - slot_fee as i64 * expect_num_slots_charged) as u64;
        let leeway = LEEWAY * expect_num_slots_charged.unsigned_abs();
        assert!(expected_end + leeway > end_balance);
        assert!(expected_end < end_balance + leeway);
    } else {
        assert!(expect_num_slots_charged >= creates);
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
