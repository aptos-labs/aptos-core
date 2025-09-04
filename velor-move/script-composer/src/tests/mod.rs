// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{CallArgument, TransactionComposer};
use velor_types::{
    state_store::state_key::StateKey,
    transaction::{ExecutionStatus, TransactionStatus},
};
use e2e_move_tests::MoveHarness;
use move_binary_format::CompiledModule;
use move_core_types::{
    account_address::AccountAddress, language_storage::ModuleId, value::MoveValue,
};
use std::{path::PathBuf, str::FromStr};

fn load_module(builder: &mut TransactionComposer, harness: &MoveHarness, module_name: &str) {
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

    let mut builder = TransactionComposer::single_signer();
    load_module(&mut builder, &h, "0x1::velor_account");
    builder
        .add_batched_call(
            "0x1::velor_account".to_string(),
            "transfer".to_string(),
            vec![],
            vec![
                CallArgument::new_signer(0),
                CallArgument::new_bytes(
                    MoveValue::Address(*bob.address())
                        .simple_serialize()
                        .unwrap(),
                ),
                CallArgument::new_bytes(MoveValue::U64(10).simple_serialize().unwrap()),
            ],
        )
        .unwrap();
    let script = builder.clone().generate_batched_calls(true).unwrap();

    let txn = alice
        .transaction()
        .script(bcs::from_bytes(&script).unwrap())
        .sequence_number(10)
        .sign();
    assert_eq!(
        h.run(txn),
        TransactionStatus::Keep(ExecutionStatus::Success)
    );

    assert_eq!(h.read_velor_balance(bob.address()), 1_000_000_000_000_010);

    builder.assert_decompilation_eq(
        &crate::decompiler::generate_batched_call_payload_serialized(&script).unwrap(),
    );
}

#[test]
fn chained_deposit() {
    let mut h = MoveHarness::new();
    let alice = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());
    let bob = h.new_account_at(AccountAddress::from_hex_literal("0xface").unwrap());

    let mut builder = TransactionComposer::single_signer();
    load_module(&mut builder, &h, "0x1::coin");
    load_module(&mut builder, &h, "0x1::velor_coin");
    load_module(&mut builder, &h, "0x1::primary_fungible_store");
    let mut returns_1 = builder
        .add_batched_call(
            "0x1::coin".to_string(),
            "withdraw".to_string(),
            vec!["0x1::velor_coin::VelorCoin".to_string()],
            vec![
                CallArgument::new_signer(0),
                CallArgument::new_bytes(MoveValue::U64(10).simple_serialize().unwrap()),
            ],
        )
        .unwrap();
    let mut returns_2 = builder
        .add_batched_call(
            "0x1::coin".to_string(),
            "coin_to_fungible_asset".to_string(),
            vec!["0x1::velor_coin::VelorCoin".to_string()],
            vec![returns_1.pop().unwrap()],
        )
        .unwrap();
    builder
        .add_batched_call(
            "0x1::primary_fungible_store".to_string(),
            "deposit".to_string(),
            vec![],
            vec![
                CallArgument::new_bytes(
                    MoveValue::Address(*bob.address())
                        .simple_serialize()
                        .unwrap(),
                ),
                returns_2.pop().unwrap(),
            ],
        )
        .unwrap();

    let script = builder.clone().generate_batched_calls(true).unwrap();

    let txn = alice
        .transaction()
        .script(bcs::from_bytes(&script).unwrap())
        .sequence_number(10)
        .sign();

    assert_eq!(
        h.run(txn),
        TransactionStatus::Keep(ExecutionStatus::Success)
    );

    assert_eq!(h.read_velor_balance(bob.address()), 1_000_000_000_000_010);
    builder.assert_decompilation_eq(
        &crate::decompiler::generate_batched_call_payload_serialized(&script).unwrap(),
    );
}

#[test]
fn chained_deposit_mismatch() {
    let mut h = MoveHarness::new();
    let bob = h.new_account_at(AccountAddress::from_hex_literal("0xface").unwrap());

    let mut builder = TransactionComposer::single_signer();
    load_module(&mut builder, &h, "0x1::coin");
    load_module(&mut builder, &h, "0x1::velor_coin");
    load_module(&mut builder, &h, "0x1::primary_fungible_store");
    let mut returns_1 = builder
        .add_batched_call(
            "0x1::coin".to_string(),
            "withdraw".to_string(),
            vec!["0x1::velor_coin::VelorCoin".to_string()],
            vec![
                CallArgument::new_signer(0),
                CallArgument::new_bytes(MoveValue::U64(10).simple_serialize().unwrap()),
            ],
        )
        .unwrap();
    assert!(builder
        .add_batched_call(
            "0x1::primary_fungible_store".to_string(),
            "deposit".to_string(),
            vec![],
            vec![
                CallArgument::new_bytes(
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

    let mut builder = TransactionComposer::single_signer();
    load_module(&mut builder, &h, "0x1::coin");
    load_module(&mut builder, &h, "0x1::velor_coin");
    load_module(&mut builder, &h, "0x1::primary_fungible_store");
    let mut returns_1 = builder
        .add_batched_call(
            "0x1::coin".to_string(),
            "withdraw".to_string(),
            vec!["0x1::velor_coin::VelorCoin".to_string()],
            vec![
                CallArgument::new_signer(0),
                CallArgument::new_bytes(MoveValue::U64(10).simple_serialize().unwrap()),
            ],
        )
        .unwrap();
    let mut returns_2 = builder
        .add_batched_call(
            "0x1::coin".to_string(),
            "coin_to_fungible_asset".to_string(),
            vec!["0x1::velor_coin::VelorCoin".to_string()],
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
                CallArgument::new_bytes(
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
                CallArgument::new_bytes(
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

#[test]
fn test_module() {
    let mut h = MoveHarness::new();
    let account = h.new_account_at(AccountAddress::ONE);
    let alice = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());
    let mut seq_num = 10;

    let module_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("tests")
        .join("test_modules");
    h.publish_package_cache_building(&account, &module_path);

    let mut run_txn = |batch_builder: TransactionComposer, h: &mut MoveHarness| {
        let script = batch_builder.generate_batched_calls(true).unwrap();
        let txn = alice
            .transaction()
            .script(bcs::from_bytes(&script).unwrap())
            .sequence_number(seq_num)
            .sign();

        seq_num += 1;

        assert_eq!(
            h.run(txn),
            TransactionStatus::Keep(ExecutionStatus::Success)
        );
    };

    // Create a copyable value and copy it twice
    let mut builder = TransactionComposer::single_signer();
    load_module(&mut builder, &h, "0x1::batched_execution");
    let returns_1 = builder
        .add_batched_call(
            "0x1::batched_execution".to_string(),
            "create_copyable_value".to_string(),
            vec![],
            vec![CallArgument::new_bytes(
                MoveValue::U8(10).simple_serialize().unwrap(),
            )],
        )
        .unwrap()
        .pop()
        .unwrap();

    builder
        .add_batched_call(
            "0x1::batched_execution".to_string(),
            "consume_copyable_value".to_string(),
            vec![],
            vec![
                returns_1.copy().unwrap(),
                CallArgument::new_bytes(MoveValue::U8(10).simple_serialize().unwrap()),
            ],
        )
        .unwrap();

    builder
        .add_batched_call(
            "0x1::batched_execution".to_string(),
            "consume_copyable_value".to_string(),
            vec![],
            vec![
                returns_1,
                CallArgument::new_bytes(MoveValue::U8(10).simple_serialize().unwrap()),
            ],
        )
        .unwrap();

    run_txn(builder, &mut h);

    // Create a droppable and copyable value and move it twice. This is ok because the builder
    // will use copy instruction instead of move instruction for values that are both copyable
    // and droppable.
    let mut builder = TransactionComposer::single_signer();
    load_module(&mut builder, &h, "0x1::batched_execution");
    let returns_1 = builder
        .add_batched_call(
            "0x1::batched_execution".to_string(),
            "create_copyable_value".to_string(),
            vec![],
            vec![CallArgument::new_bytes(
                MoveValue::U8(10).simple_serialize().unwrap(),
            )],
        )
        .unwrap()
        .pop()
        .unwrap();

    builder
        .add_batched_call(
            "0x1::batched_execution".to_string(),
            "consume_copyable_value".to_string(),
            vec![],
            vec![
                returns_1.clone(),
                CallArgument::new_bytes(MoveValue::U8(10).simple_serialize().unwrap()),
            ],
        )
        .unwrap();

    builder
        .add_batched_call(
            "0x1::batched_execution".to_string(),
            "consume_copyable_value".to_string(),
            vec![],
            vec![
                returns_1,
                CallArgument::new_bytes(MoveValue::U8(10).simple_serialize().unwrap()),
            ],
        )
        .unwrap();

    run_txn(builder, &mut h);

    // Create a droppable value and move it twice
    let mut builder = TransactionComposer::single_signer();
    load_module(&mut builder, &h, "0x1::batched_execution");
    let returns_1 = builder
        .add_batched_call(
            "0x1::batched_execution".to_string(),
            "create_droppable_value".to_string(),
            vec![],
            vec![CallArgument::new_bytes(
                MoveValue::U8(10).simple_serialize().unwrap(),
            )],
        )
        .unwrap()
        .pop()
        .unwrap();

    builder
        .add_batched_call(
            "0x1::batched_execution".to_string(),
            "consume_droppable_value".to_string(),
            vec![],
            vec![
                returns_1.clone(),
                CallArgument::new_bytes(MoveValue::U8(10).simple_serialize().unwrap()),
            ],
        )
        .unwrap();
    assert!(builder
        .add_batched_call(
            "0x1::batched_execution".to_string(),
            "consume_droppable_value".to_string(),
            vec![],
            vec![
                returns_1,
                CallArgument::new_bytes(MoveValue::U8(10).simple_serialize().unwrap()),
            ],
        )
        .is_err());

    // Copying a non-copyable value should return error on call.
    let mut builder = TransactionComposer::single_signer();
    load_module(&mut builder, &h, "0x1::batched_execution");
    let returns_1 = builder
        .add_batched_call(
            "0x1::batched_execution".to_string(),
            "create_non_droppable_value".to_string(),
            vec![],
            vec![CallArgument::new_bytes(
                MoveValue::U8(10).simple_serialize().unwrap(),
            )],
        )
        .unwrap()
        .pop()
        .unwrap();

    assert!(builder
        .add_batched_call(
            "0x1::batched_execution".to_string(),
            "consume_non_droppable_value".to_string(),
            vec![],
            vec![
                returns_1.copy().unwrap(),
                CallArgument::new_bytes(MoveValue::U8(10).simple_serialize().unwrap()),
            ],
        )
        .is_err());

    // Create a value and pass it to the wrong type
    let mut builder = TransactionComposer::single_signer();
    load_module(&mut builder, &h, "0x1::batched_execution");
    let returns_1 = builder
        .add_batched_call(
            "0x1::batched_execution".to_string(),
            "create_non_droppable_value".to_string(),
            vec![],
            vec![CallArgument::new_bytes(
                MoveValue::U8(10).simple_serialize().unwrap(),
            )],
        )
        .unwrap()
        .pop()
        .unwrap();

    assert!(builder
        .add_batched_call(
            "0x1::batched_execution".to_string(),
            "consume_droppable_value".to_string(),
            vec![],
            vec![
                returns_1,
                CallArgument::new_bytes(MoveValue::U8(10).simple_serialize().unwrap()),
            ],
        )
        .is_err());

    // Create a non droppable value and never use it.
    let mut builder = TransactionComposer::single_signer();
    load_module(&mut builder, &h, "0x1::batched_execution");
    let _returns_1 = builder
        .add_batched_call(
            "0x1::batched_execution".to_string(),
            "create_non_droppable_value".to_string(),
            vec![],
            vec![CallArgument::new_bytes(
                MoveValue::U8(10).simple_serialize().unwrap(),
            )],
        )
        .unwrap()
        .pop()
        .unwrap();

    assert!(builder.generate_batched_calls(true).is_err());

    // Create a value and pass by reference
    let mut builder = TransactionComposer::single_signer();
    load_module(&mut builder, &h, "0x1::batched_execution");
    let returns_1 = builder
        .add_batched_call(
            "0x1::batched_execution".to_string(),
            "create_copyable_value".to_string(),
            vec![],
            vec![CallArgument::new_bytes(
                MoveValue::U8(10).simple_serialize().unwrap(),
            )],
        )
        .unwrap()
        .pop()
        .unwrap();

    builder
        .add_batched_call(
            "0x1::batched_execution".to_string(),
            "check_copyable_value".to_string(),
            vec![],
            vec![
                returns_1.borrow().unwrap(),
                CallArgument::new_bytes(MoveValue::U8(10).simple_serialize().unwrap()),
            ],
        )
        .unwrap();

    builder
        .add_batched_call(
            "0x1::batched_execution".to_string(),
            "consume_copyable_value".to_string(),
            vec![],
            vec![
                returns_1,
                CallArgument::new_bytes(MoveValue::U8(10).simple_serialize().unwrap()),
            ],
        )
        .unwrap();

    run_txn(builder, &mut h);

    // Create a value and pass by mutable reference and then mutate.
    let mut builder = TransactionComposer::single_signer();
    load_module(&mut builder, &h, "0x1::batched_execution");
    let returns_1 = builder
        .add_batched_call(
            "0x1::batched_execution".to_string(),
            "create_non_droppable_value".to_string(),
            vec![],
            vec![CallArgument::new_bytes(
                MoveValue::U8(10).simple_serialize().unwrap(),
            )],
        )
        .unwrap()
        .pop()
        .unwrap();

    builder
        .add_batched_call(
            "0x1::batched_execution".to_string(),
            "mutate_non_droppable_value".to_string(),
            vec![],
            vec![
                returns_1.borrow_mut().unwrap(),
                CallArgument::new_bytes(MoveValue::U8(42).simple_serialize().unwrap()),
            ],
        )
        .unwrap();

    builder
        .add_batched_call(
            "0x1::batched_execution".to_string(),
            "consume_non_droppable_value".to_string(),
            vec![],
            vec![
                returns_1,
                CallArgument::new_bytes(MoveValue::U8(42).simple_serialize().unwrap()),
            ],
        )
        .unwrap();

    run_txn(builder, &mut h);

    // Create a value and pass it to a generic function
    let mut builder = TransactionComposer::single_signer();
    load_module(&mut builder, &h, "0x1::batched_execution");
    let returns_1 = builder
        .add_batched_call(
            "0x1::batched_execution".to_string(),
            "create_non_droppable_value".to_string(),
            vec![],
            vec![CallArgument::new_bytes(
                MoveValue::U8(10).simple_serialize().unwrap(),
            )],
        )
        .unwrap()
        .pop()
        .unwrap();

    let returns_2 = builder
        .add_batched_call(
            "0x1::batched_execution".to_string(),
            "id".to_string(),
            vec!["0x1::batched_execution::NonDroppableValue".to_string()],
            vec![returns_1],
        )
        .unwrap()
        .pop()
        .unwrap();

    builder
        .add_batched_call(
            "0x1::batched_execution".to_string(),
            "consume_non_droppable_value".to_string(),
            vec![],
            vec![
                returns_2,
                CallArgument::new_bytes(MoveValue::U8(10).simple_serialize().unwrap()),
            ],
        )
        .unwrap();

    // Create a value and pass it to a generic function with invalid type.
    let mut builder = TransactionComposer::single_signer();
    load_module(&mut builder, &h, "0x1::batched_execution");
    let returns_1 = builder
        .add_batched_call(
            "0x1::batched_execution".to_string(),
            "create_non_droppable_value".to_string(),
            vec![],
            vec![CallArgument::new_bytes(
                MoveValue::U8(10).simple_serialize().unwrap(),
            )],
        )
        .unwrap()
        .pop()
        .unwrap();

    assert!(builder
        .add_batched_call(
            "0x1::batched_execution".to_string(),
            "id".to_string(),
            vec!["0x1::batched_execution::DroppableValue".to_string()],
            vec![returns_1],
        )
        .is_err());

    // Create a droppable value with generics and don't use it
    let mut builder = TransactionComposer::single_signer();
    load_module(&mut builder, &h, "0x1::batched_execution");
    builder
        .add_batched_call(
            "0x1::batched_execution".to_string(),
            "create_generic_droppable_value".to_string(),
            vec!["0x1::batched_execution::Foo".to_string()],
            vec![CallArgument::new_bytes(
                MoveValue::U8(10).simple_serialize().unwrap(),
            )],
        )
        .unwrap();
    run_txn(builder, &mut h);

    // Adding two calls to the same functions
    let mut builder = TransactionComposer::single_signer();
    load_module(&mut builder, &h, "0x1::batched_execution");
    builder
        .add_batched_call(
            "0x1::batched_execution".to_string(),
            "create_generic_droppable_value".to_string(),
            vec!["0x1::batched_execution::Foo".to_string()],
            vec![CallArgument::new_bytes(
                MoveValue::U8(10).simple_serialize().unwrap(),
            )],
        )
        .unwrap();

    builder
        .add_batched_call(
            "0x1::batched_execution".to_string(),
            "create_generic_droppable_value".to_string(),
            vec!["0x1::batched_execution::Foo".to_string()],
            vec![CallArgument::new_bytes(
                MoveValue::U8(10).simple_serialize().unwrap(),
            )],
        )
        .unwrap();
    run_txn(builder, &mut h);

    // Create a generic value and consume it
    let mut builder = TransactionComposer::single_signer();
    load_module(&mut builder, &h, "0x1::batched_execution");
    let returns_1 = builder
        .add_batched_call(
            "0x1::batched_execution".to_string(),
            "create_generic_non_droppable_value".to_string(),
            vec!["0x1::batched_execution::Foo".to_string()],
            vec![CallArgument::new_bytes(
                MoveValue::U8(10).simple_serialize().unwrap(),
            )],
        )
        .unwrap()
        .pop()
        .unwrap();

    builder
        .add_batched_call(
            "0x1::batched_execution".to_string(),
            "consume_generic_non_droppable_value".to_string(),
            vec!["0x1::batched_execution::Foo".to_string()],
            vec![
                returns_1,
                CallArgument::new_bytes(MoveValue::U8(10).simple_serialize().unwrap()),
            ],
        )
        .unwrap();

    run_txn(builder, &mut h);

    // Create a generic value and destruct it with wrong type parameter.
    let mut builder = TransactionComposer::single_signer();
    load_module(&mut builder, &h, "0x1::batched_execution");
    let returns_1 = builder
        .add_batched_call(
            "0x1::batched_execution".to_string(),
            "create_generic_non_droppable_value".to_string(),
            vec!["0x1::batched_execution::Foo".to_string()],
            vec![CallArgument::new_bytes(
                MoveValue::U8(10).simple_serialize().unwrap(),
            )],
        )
        .unwrap()
        .pop()
        .unwrap();

    assert!(builder
        .add_batched_call(
            "0x1::batched_execution".to_string(),
            "consume_generic_non_droppable_value".to_string(),
            vec!["0x1::batched_execution::Bar".to_string()],
            vec![
                returns_1,
                CallArgument::new_bytes(MoveValue::U8(10).simple_serialize().unwrap()),
            ],
        )
        .is_err());

    // Test functions with multiple return values.
    // Create a copyable value and copy it twice
    let mut builder = TransactionComposer::single_signer();
    load_module(&mut builder, &h, "0x1::batched_execution");
    let returns = builder
        .add_batched_call(
            "0x1::batched_execution".to_string(),
            "multiple_returns".to_string(),
            vec![],
            vec![],
        )
        .unwrap();

    builder
        .add_batched_call(
            "0x1::batched_execution".to_string(),
            "consume_non_droppable_value".to_string(),
            vec![],
            vec![
                returns[1].clone(),
                CallArgument::new_bytes(MoveValue::U8(1).simple_serialize().unwrap()),
            ],
        )
        .unwrap();

    run_txn(builder, &mut h);
}
