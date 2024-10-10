// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{BatchArgumentWASM, BatchedFunctionCallBuilder};
use aptos_types::{
    state_store::state_key::StateKey,
    transaction::{ExecutionStatus, TransactionStatus},
};
use e2e_move_tests::MoveHarness;
use move_binary_format::CompiledModule;
use move_core_types::{
    account_address::AccountAddress, language_storage::ModuleId, value::MoveValue,
};
use std::{path::PathBuf, str::FromStr};

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
        .add_batched_call_wasm(
            "0x1::aptos_account".to_string(),
            "transfer".to_string(),
            vec![],
            vec![
                BatchArgumentWASM::new_signer(0),
                BatchArgumentWASM::new_bytes(
                    MoveValue::Address(*bob.address())
                        .simple_serialize()
                        .unwrap(),
                ),
                BatchArgumentWASM::new_bytes(MoveValue::U64(10).simple_serialize().unwrap()),
            ],
        )
        .unwrap();

    let expected_calls = builder.calls().to_vec();
    let script = builder.generate_batched_calls().unwrap();

    let txn = alice
        .transaction()
        .script(bcs::from_bytes(&script).unwrap())
        .sequence_number(10)
        .sign();
    assert_eq!(
        h.run(txn),
        TransactionStatus::Keep(ExecutionStatus::Success)
    );

    assert_eq!(h.read_aptos_balance(bob.address()), 1_000_000_000_000_010);

    assert_eq!(
        crate::decompiler::generate_batched_call_payload_serialized(&script).unwrap(),
        expected_calls
    );
}

#[test]
fn chained_deposit() {
    let mut h = MoveHarness::new();
    let alice = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());
    let bob = h.new_account_at(AccountAddress::from_hex_literal("0xface").unwrap());

    let mut builder = BatchedFunctionCallBuilder::single_signer();
    load_module(&mut builder, &h, "0x1::coin");
    load_module(&mut builder, &h, "0x1::aptos_coin");
    load_module(&mut builder, &h, "0x1::primary_fungible_store");
    let mut returns_1 = builder
        .add_batched_call_wasm(
            "0x1::coin".to_string(),
            "withdraw".to_string(),
            vec!["0x1::aptos_coin::AptosCoin".to_string()],
            vec![
                BatchArgumentWASM::new_signer(0),
                BatchArgumentWASM::new_bytes(MoveValue::U64(10).simple_serialize().unwrap()),
            ],
        )
        .unwrap();
    let mut returns_2 = builder
        .add_batched_call_wasm(
            "0x1::coin".to_string(),
            "coin_to_fungible_asset".to_string(),
            vec!["0x1::aptos_coin::AptosCoin".to_string()],
            vec![returns_1.pop().unwrap()],
        )
        .unwrap();
    builder
        .add_batched_call_wasm(
            "0x1::primary_fungible_store".to_string(),
            "deposit".to_string(),
            vec![],
            vec![
                BatchArgumentWASM::new_bytes(
                    MoveValue::Address(*bob.address())
                        .simple_serialize()
                        .unwrap(),
                ),
                returns_2.pop().unwrap(),
            ],
        )
        .unwrap();

    let expected_calls = builder.calls().to_vec();
    let script = builder.generate_batched_calls().unwrap();

    let txn = alice
        .transaction()
        .script(bcs::from_bytes(&script).unwrap())
        .sequence_number(10)
        .sign();

    assert_eq!(
        h.run(txn),
        TransactionStatus::Keep(ExecutionStatus::Success)
    );

    assert_eq!(h.read_aptos_balance(bob.address()), 1_000_000_000_000_010);
    assert_eq!(
        crate::decompiler::generate_batched_call_payload_serialized(&script).unwrap(),
        expected_calls
    );
}

#[test]
fn chained_deposit_mismatch() {
    let mut h = MoveHarness::new();
    let bob = h.new_account_at(AccountAddress::from_hex_literal("0xface").unwrap());

    let mut builder = BatchedFunctionCallBuilder::single_signer();
    load_module(&mut builder, &h, "0x1::coin");
    load_module(&mut builder, &h, "0x1::aptos_coin");
    load_module(&mut builder, &h, "0x1::primary_fungible_store");
    let mut returns_1 = builder
        .add_batched_call_wasm(
            "0x1::coin".to_string(),
            "withdraw".to_string(),
            vec!["0x1::aptos_coin::AptosCoin".to_string()],
            vec![
                BatchArgumentWASM::new_signer(0),
                BatchArgumentWASM::new_bytes(MoveValue::U64(10).simple_serialize().unwrap()),
            ],
        )
        .unwrap();
    assert!(builder
        .add_batched_call_wasm(
            "0x1::primary_fungible_store".to_string(),
            "deposit".to_string(),
            vec![],
            vec![
                BatchArgumentWASM::new_bytes(
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
    load_module(&mut builder, &h, "0x1::aptos_coin");
    load_module(&mut builder, &h, "0x1::primary_fungible_store");
    let mut returns_1 = builder
        .add_batched_call_wasm(
            "0x1::coin".to_string(),
            "withdraw".to_string(),
            vec!["0x1::aptos_coin::AptosCoin".to_string()],
            vec![
                BatchArgumentWASM::new_signer(0),
                BatchArgumentWASM::new_bytes(MoveValue::U64(10).simple_serialize().unwrap()),
            ],
        )
        .unwrap();
    let mut returns_2 = builder
        .add_batched_call_wasm(
            "0x1::coin".to_string(),
            "coin_to_fungible_asset".to_string(),
            vec!["0x1::aptos_coin::AptosCoin".to_string()],
            vec![returns_1.pop().unwrap()],
        )
        .unwrap();
    let return_val = returns_2.pop().unwrap();
    assert!(builder
        .add_batched_call_wasm(
            "0x1::primary_fungible_store".to_string(),
            "deposit".to_string(),
            vec![],
            vec![
                BatchArgumentWASM::new_bytes(
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
        .add_batched_call_wasm(
            "0x1::primary_fungible_store".to_string(),
            "deposit".to_string(),
            vec![],
            vec![
                BatchArgumentWASM::new_bytes(
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

    let mut run_txn = |batch_builder: BatchedFunctionCallBuilder, h: &mut MoveHarness| {
        let script = batch_builder.generate_batched_calls().unwrap();
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
    let mut builder = BatchedFunctionCallBuilder::single_signer();
    load_module(&mut builder, &h, "0x1::batched_execution");
    let returns_1 = builder
        .add_batched_call_wasm(
            "0x1::batched_execution".to_string(),
            "create_copyable_value".to_string(),
            vec![],
            vec![BatchArgumentWASM::new_bytes(
                MoveValue::U8(10).simple_serialize().unwrap(),
            )],
        )
        .unwrap()
        .pop()
        .unwrap();

    builder
        .add_batched_call_wasm(
            "0x1::batched_execution".to_string(),
            "consume_copyable_value".to_string(),
            vec![],
            vec![
                returns_1.copy().unwrap(),
                BatchArgumentWASM::new_bytes(MoveValue::U8(10).simple_serialize().unwrap()),
            ],
        )
        .unwrap();

    builder
        .add_batched_call_wasm(
            "0x1::batched_execution".to_string(),
            "consume_copyable_value".to_string(),
            vec![],
            vec![
                returns_1,
                BatchArgumentWASM::new_bytes(MoveValue::U8(10).simple_serialize().unwrap()),
            ],
        )
        .unwrap();

    run_txn(builder, &mut h);

    // Copying a non-copyable value should return error on call.
    let mut builder = BatchedFunctionCallBuilder::single_signer();
    load_module(&mut builder, &h, "0x1::batched_execution");
    let returns_1 = builder
        .add_batched_call_wasm(
            "0x1::batched_execution".to_string(),
            "create_non_droppable_value".to_string(),
            vec![],
            vec![BatchArgumentWASM::new_bytes(
                MoveValue::U8(10).simple_serialize().unwrap(),
            )],
        )
        .unwrap()
        .pop()
        .unwrap();

    assert!(builder
        .add_batched_call_wasm(
            "0x1::batched_execution".to_string(),
            "consume_non_droppable_value".to_string(),
            vec![],
            vec![
                returns_1.copy().unwrap(),
                BatchArgumentWASM::new_bytes(MoveValue::U8(10).simple_serialize().unwrap()),
            ],
        )
        .is_err());

    // Create a value and pass it to the wrong type
    let mut builder = BatchedFunctionCallBuilder::single_signer();
    load_module(&mut builder, &h, "0x1::batched_execution");
    let returns_1 = builder
        .add_batched_call_wasm(
            "0x1::batched_execution".to_string(),
            "create_non_droppable_value".to_string(),
            vec![],
            vec![BatchArgumentWASM::new_bytes(
                MoveValue::U8(10).simple_serialize().unwrap(),
            )],
        )
        .unwrap()
        .pop()
        .unwrap();

    assert!(builder
        .add_batched_call_wasm(
            "0x1::batched_execution".to_string(),
            "consume_droppable_value".to_string(),
            vec![],
            vec![
                returns_1,
                BatchArgumentWASM::new_bytes(MoveValue::U8(10).simple_serialize().unwrap()),
            ],
        )
        .is_err());

    // Create a non droppable value and never use it.
    let mut builder = BatchedFunctionCallBuilder::single_signer();
    load_module(&mut builder, &h, "0x1::batched_execution");
    let _returns_1 = builder
        .add_batched_call_wasm(
            "0x1::batched_execution".to_string(),
            "create_non_droppable_value".to_string(),
            vec![],
            vec![BatchArgumentWASM::new_bytes(
                MoveValue::U8(10).simple_serialize().unwrap(),
            )],
        )
        .unwrap()
        .pop()
        .unwrap();

    assert!(builder.generate_batched_calls().is_err());

    // Create a value and pass by reference
    let mut builder = BatchedFunctionCallBuilder::single_signer();
    load_module(&mut builder, &h, "0x1::batched_execution");
    let returns_1 = builder
        .add_batched_call_wasm(
            "0x1::batched_execution".to_string(),
            "create_copyable_value".to_string(),
            vec![],
            vec![BatchArgumentWASM::new_bytes(
                MoveValue::U8(10).simple_serialize().unwrap(),
            )],
        )
        .unwrap()
        .pop()
        .unwrap();

    builder
        .add_batched_call_wasm(
            "0x1::batched_execution".to_string(),
            "check_copyable_value".to_string(),
            vec![],
            vec![
                returns_1.borrow().unwrap(),
                BatchArgumentWASM::new_bytes(MoveValue::U8(10).simple_serialize().unwrap()),
            ],
        )
        .unwrap();

    builder
        .add_batched_call_wasm(
            "0x1::batched_execution".to_string(),
            "consume_copyable_value".to_string(),
            vec![],
            vec![
                returns_1,
                BatchArgumentWASM::new_bytes(MoveValue::U8(10).simple_serialize().unwrap()),
            ],
        )
        .unwrap();

    run_txn(builder, &mut h);

    // Create a value and pass by mutable reference and then mutate.
    let mut builder = BatchedFunctionCallBuilder::single_signer();
    load_module(&mut builder, &h, "0x1::batched_execution");
    let returns_1 = builder
        .add_batched_call_wasm(
            "0x1::batched_execution".to_string(),
            "create_non_droppable_value".to_string(),
            vec![],
            vec![BatchArgumentWASM::new_bytes(
                MoveValue::U8(10).simple_serialize().unwrap(),
            )],
        )
        .unwrap()
        .pop()
        .unwrap();

    builder
        .add_batched_call_wasm(
            "0x1::batched_execution".to_string(),
            "mutate_non_droppable_value".to_string(),
            vec![],
            vec![
                returns_1.borrow_mut().unwrap(),
                BatchArgumentWASM::new_bytes(MoveValue::U8(42).simple_serialize().unwrap()),
            ],
        )
        .unwrap();

    builder
        .add_batched_call_wasm(
            "0x1::batched_execution".to_string(),
            "consume_non_droppable_value".to_string(),
            vec![],
            vec![
                returns_1,
                BatchArgumentWASM::new_bytes(MoveValue::U8(42).simple_serialize().unwrap()),
            ],
        )
        .unwrap();

    run_txn(builder, &mut h);

    // Create a value and pass it to a generic function
    let mut builder = BatchedFunctionCallBuilder::single_signer();
    load_module(&mut builder, &h, "0x1::batched_execution");
    let returns_1 = builder
        .add_batched_call_wasm(
            "0x1::batched_execution".to_string(),
            "create_non_droppable_value".to_string(),
            vec![],
            vec![BatchArgumentWASM::new_bytes(
                MoveValue::U8(10).simple_serialize().unwrap(),
            )],
        )
        .unwrap()
        .pop()
        .unwrap();

    let returns_2 = builder
        .add_batched_call_wasm(
            "0x1::batched_execution".to_string(),
            "id".to_string(),
            vec!["0x1::batched_execution::NonDroppableValue".to_string()],
            vec![returns_1],
        )
        .unwrap()
        .pop()
        .unwrap();

    builder
        .add_batched_call_wasm(
            "0x1::batched_execution".to_string(),
            "consume_non_droppable_value".to_string(),
            vec![],
            vec![
                returns_2,
                BatchArgumentWASM::new_bytes(MoveValue::U8(10).simple_serialize().unwrap()),
            ],
        )
        .unwrap();

    run_txn(builder, &mut h);

    // Create a droppable value with generics and don't use it
    let mut builder = BatchedFunctionCallBuilder::single_signer();
    load_module(&mut builder, &h, "0x1::batched_execution");
    builder
        .add_batched_call_wasm(
            "0x1::batched_execution".to_string(),
            "create_generic_droppable_value".to_string(),
            vec!["0x1::batched_execution::Foo".to_string()],
            vec![BatchArgumentWASM::new_bytes(
                MoveValue::U8(10).simple_serialize().unwrap(),
            )],
        )
        .unwrap();
    run_txn(builder, &mut h);

    // Create a generic value and consume it
    let mut builder = BatchedFunctionCallBuilder::single_signer();
    load_module(&mut builder, &h, "0x1::batched_execution");
    let returns_1 = builder
        .add_batched_call_wasm(
            "0x1::batched_execution".to_string(),
            "create_generic_non_droppable_value".to_string(),
            vec!["0x1::batched_execution::Foo".to_string()],
            vec![BatchArgumentWASM::new_bytes(
                MoveValue::U8(10).simple_serialize().unwrap(),
            )],
        )
        .unwrap()
        .pop()
        .unwrap();

    builder
        .add_batched_call_wasm(
            "0x1::batched_execution".to_string(),
            "consume_generic_non_droppable_value".to_string(),
            vec!["0x1::batched_execution::Foo".to_string()],
            vec![
                returns_1,
                BatchArgumentWASM::new_bytes(MoveValue::U8(10).simple_serialize().unwrap()),
            ],
        )
        .unwrap();

    run_txn(builder, &mut h);

    // Create a generic value and destruct it with wrong type parameter.
    let mut builder = BatchedFunctionCallBuilder::single_signer();
    load_module(&mut builder, &h, "0x1::batched_execution");
    let returns_1 = builder
        .add_batched_call_wasm(
            "0x1::batched_execution".to_string(),
            "create_generic_non_droppable_value".to_string(),
            vec!["0x1::batched_execution::Foo".to_string()],
            vec![BatchArgumentWASM::new_bytes(
                MoveValue::U8(10).simple_serialize().unwrap(),
            )],
        )
        .unwrap()
        .pop()
        .unwrap();

    assert!(builder
        .add_batched_call_wasm(
            "0x1::batched_execution".to_string(),
            "consume_generic_non_droppable_value".to_string(),
            vec!["0x1::batched_execution::Bar".to_string()],
            vec![
                returns_1,
                BatchArgumentWASM::new_bytes(MoveValue::U8(10).simple_serialize().unwrap()),
            ],
        )
        .is_err());
}
