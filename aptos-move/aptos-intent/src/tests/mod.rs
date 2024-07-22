// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{BatchArgument, BatchedFunctionCallBuilder};
use aptos_types::{
    state_store::state_key::StateKey,
    transaction::{ExecutionStatus, TransactionStatus},
};
use e2e_move_tests::MoveHarness;
use move_binary_format::CompiledModule;
use move_core_types::{
    account_address::AccountAddress, language_storage::ModuleId, value::MoveValue,
};
use std::str::FromStr;

fn load_module(builder: &mut BatchedFunctionCallBuilder, harness: &MoveHarness, module_name: &str) {
    let module = ModuleId::from_str(module_name).unwrap();
    let bytes = harness
        .read_state_value_bytes(&StateKey::module_id(&module))
        .unwrap();
    builder.insert_module(CompiledModule::deserialize(&bytes).unwrap());
}

#[test]
fn simple_builder() {
    let mut h = MoveHarness::new();
    let alice = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());
    let bob = h.new_account_at(AccountAddress::from_hex_literal("0xface").unwrap());

    let mut builder = BatchedFunctionCallBuilder::single_signer();
    load_module(&mut builder, &h, "0x1::aptos_account");
    builder
        .add_batched_call(
            "0x1::aptos_account".to_string(),
            "transfer".to_string(),
            vec![],
            vec![
                BatchArgument::new_signer(0),
                BatchArgument::new_bytes(
                    MoveValue::Address(*bob.address())
                        .simple_serialize()
                        .unwrap(),
                ),
                BatchArgument::new_bytes(MoveValue::U64(10).simple_serialize().unwrap()),
            ],
        )
        .unwrap();

    let txn = alice
        .transaction()
        .script(bcs::from_bytes(&builder.generate_batched_calls().unwrap()).unwrap())
        .sequence_number(10)
        .sign();
    assert_eq!(
        h.run(txn),
        TransactionStatus::Keep(ExecutionStatus::Success)
    );

    assert_eq!(h.read_aptos_balance(bob.address()), 1_000_000_000_000_010);
}

#[test]
fn chained_deposit() {
    let mut h = MoveHarness::new();
    let alice = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());
    let bob = h.new_account_at(AccountAddress::from_hex_literal("0xface").unwrap());

    let mut builder = BatchedFunctionCallBuilder::single_signer();
    load_module(&mut builder, &h, "0x1::coin");
    load_module(&mut builder, &h, "0x1::primary_fungible_store");
    let mut returns_1 = builder
        .add_batched_call(
            "0x1::coin".to_string(),
            "withdraw".to_string(),
            vec!["0x1::aptos_coin::AptosCoin".to_string()],
            vec![
                BatchArgument::new_signer(0),
                BatchArgument::new_bytes(MoveValue::U64(10).simple_serialize().unwrap()),
            ],
        )
        .unwrap();
    let mut returns_2 = builder
        .add_batched_call(
            "0x1::coin".to_string(),
            "coin_to_fungible_asset".to_string(),
            vec!["0x1::aptos_coin::AptosCoin".to_string()],
            vec![returns_1.pop().unwrap()],
        )
        .unwrap();
    builder
        .add_batched_call(
            "0x1::primary_fungible_store".to_string(),
            "deposit".to_string(),
            vec![],
            vec![
                BatchArgument::new_bytes(
                    MoveValue::Address(*bob.address())
                        .simple_serialize()
                        .unwrap(),
                ),
                returns_2.pop().unwrap(),
            ],
        )
        .unwrap();

    let txn = alice
        .transaction()
        .script(bcs::from_bytes(&builder.generate_batched_calls().unwrap()).unwrap())
        .sequence_number(10)
        .sign();

    assert_eq!(
        h.run(txn),
        TransactionStatus::Keep(ExecutionStatus::Success)
    );

    assert_eq!(h.read_aptos_balance(bob.address()), 1_000_000_000_000_010);
}

#[test]
fn chained_deposit_mismatch() {
    let mut h = MoveHarness::new();
    let bob = h.new_account_at(AccountAddress::from_hex_literal("0xface").unwrap());

    let mut builder = BatchedFunctionCallBuilder::single_signer();
    load_module(&mut builder, &h, "0x1::coin");
    load_module(&mut builder, &h, "0x1::primary_fungible_store");
    let mut returns_1 = builder
        .add_batched_call(
            "0x1::coin".to_string(),
            "withdraw".to_string(),
            vec!["0x1::aptos_coin::AptosCoin".to_string()],
            vec![
                BatchArgument::new_signer(0),
                BatchArgument::new_bytes(MoveValue::U64(10).simple_serialize().unwrap()),
            ],
        )
        .unwrap();
    assert!(builder
        .add_batched_call(
            "0x1::primary_fungible_store".to_string(),
            "deposit".to_string(),
            vec![],
            vec![
                BatchArgument::new_bytes(
                    MoveValue::Address(*bob.address())
                        .simple_serialize()
                        .unwrap(),
                ),
                // Passing value of unexpected type
                returns_1.pop().unwrap(),
            ],
        )
        .is_err());
}

#[test]
fn chained_deposit_invalid_copy() {
    let mut h = MoveHarness::new();
    let bob = h.new_account_at(AccountAddress::from_hex_literal("0xface").unwrap());

    let mut builder = BatchedFunctionCallBuilder::single_signer();
    load_module(&mut builder, &h, "0x1::coin");
    load_module(&mut builder, &h, "0x1::primary_fungible_store");
    let mut returns_1 = builder
        .add_batched_call(
            "0x1::coin".to_string(),
            "withdraw".to_string(),
            vec!["0x1::aptos_coin::AptosCoin".to_string()],
            vec![
                BatchArgument::new_signer(0),
                BatchArgument::new_bytes(MoveValue::U64(10).simple_serialize().unwrap()),
            ],
        )
        .unwrap();
    let mut returns_2 = builder
        .add_batched_call(
            "0x1::coin".to_string(),
            "coin_to_fungible_asset".to_string(),
            vec!["0x1::aptos_coin::AptosCoin".to_string()],
            vec![returns_1.pop().unwrap()],
        )
        .unwrap();
    let return_val = returns_2.pop().unwrap();
    assert!(builder
        .add_batched_call(
            "0x1::primary_fungible_store".to_string(),
            "deposit".to_string(),
            vec![],
            vec![
                BatchArgument::new_bytes(
                    MoveValue::Address(*bob.address())
                        .simple_serialize()
                        .unwrap(),
                ),
                // Copying a non-copyable value should be rejected.
                return_val.copy().unwrap(),
            ],
        )
        .is_err());

    assert!(builder
        .add_batched_call(
            "0x1::primary_fungible_store".to_string(),
            "deposit".to_string(),
            vec![],
            vec![
                BatchArgument::new_bytes(
                    MoveValue::Address(*bob.address())
                        .simple_serialize()
                        .unwrap(),
                ),
                // Passing a reference should result in a type error
                return_val.borrow().unwrap(),
            ],
        )
        .is_err());
}
