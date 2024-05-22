// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{assert_success, assert_vm_status, tests::common, MoveHarness};
use aptos_language_e2e_tests::account::TransactionBuilder;
use aptos_types::{
    account_address::{self, AccountAddress},
    transaction::{BatchArgument, BatchedFunctionCall},
};
use move_core_types::{
    identifier::Identifier,
    language_storage::{ModuleId, StructTag},
    vm_status::StatusCode,
};
use once_cell::sync::Lazy;
use serde::Deserialize;

#[derive(Debug, Deserialize, Eq, PartialEq)]
struct FungibleStore {
    metadata: AccountAddress,
    balance: u64,
    allow_ungated_balance_transfer: bool,
}

pub static FUNGIBLE_STORE_TAG: Lazy<StructTag> = Lazy::new(|| StructTag {
    address: AccountAddress::from_hex_literal("0x1").unwrap(),
    module: Identifier::new("fungible_asset").unwrap(),
    name: Identifier::new("FungibleStore").unwrap(),
    type_params: vec![],
});

pub static OBJ_GROUP_TAG: Lazy<StructTag> = Lazy::new(|| StructTag {
    address: AccountAddress::from_hex_literal("0x1").unwrap(),
    module: Identifier::new("object").unwrap(),
    name: Identifier::new("ObjectGroup").unwrap(),
    type_params: vec![],
});

#[test]
fn test_batched_transaction_with_fa() {
    let mut h = MoveHarness::new();

    let alice = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());
    let bob = h.new_account_at(AccountAddress::from_hex_literal("0xface").unwrap());
    let root = h.aptos_framework_account();

    let mut build_options = aptos_framework::BuildOptions::default();
    build_options
        .named_addresses
        .insert("example_addr".to_string(), *alice.address());

    let result = h.publish_package_with_options(
        &alice,
        &common::test_dir_path("../../../move-examples/fungible_asset/managed_fungible_asset"),
        build_options.clone(),
    );

    assert_success!(result);
    let result = h.publish_package_with_options(
        &alice,
        &common::test_dir_path("../../../move-examples/fungible_asset/managed_fungible_token"),
        build_options,
    );
    assert_success!(result);

    assert_success!(h.run_entry_function(
        &root,
        str::parse(&format!(
            "0x{}::coin::create_coin_conversion_map",
            (*root.address()).to_hex()
        ))
        .unwrap(),
        vec![],
        vec![],
    ));

    let metadata = h
        .execute_view_function(
            str::parse(&format!(
                "0x{}::managed_fungible_token::get_metadata",
                (*alice.address()).to_hex()
            ))
            .unwrap(),
            vec![],
            vec![],
        )
        .values
        .unwrap()
        .pop()
        .unwrap();
    let metadata = bcs::from_bytes::<AccountAddress>(metadata.as_slice()).unwrap();

    let result = h.run_entry_function(
        &alice,
        str::parse(&format!(
            "0x{}::managed_fungible_asset::mint_to_primary_stores",
            (*alice.address()).to_hex()
        ))
        .unwrap(),
        vec![],
        vec![
            bcs::to_bytes(&metadata).unwrap(),
            bcs::to_bytes(&vec![alice.address()]).unwrap(),
            bcs::to_bytes(&vec![100u64]).unwrap(), // amount
        ],
    );
    assert_success!(result);

    let payload = vec![
        BatchedFunctionCall {
            module: ModuleId::new(
                *alice.address(),
                Identifier::new("managed_fungible_asset").unwrap(),
            ),
            function: Identifier::new("withdraw_from_primary_stores").unwrap(),
            ty_args: vec![],
            args: vec![
                BatchArgument::Raw(bcs::to_bytes(&metadata).unwrap()),
                BatchArgument::Raw(bcs::to_bytes(&vec![alice.address()]).unwrap()),
                BatchArgument::Raw(bcs::to_bytes(&vec![30u64]).unwrap()), // amount
            ],
        },
        BatchedFunctionCall {
            module: ModuleId::new(
                *alice.address(),
                Identifier::new("managed_fungible_asset").unwrap(),
            ),
            function: Identifier::new("deposit_to_primary_stores_owned").unwrap(),
            ty_args: vec![],
            args: vec![
                // Return from first call
                BatchArgument::PreviousResult(0, 0),
                BatchArgument::Raw(bcs::to_bytes(&vec![bob.address()]).unwrap()),
                BatchArgument::Raw(bcs::to_bytes(&vec![30u64]).unwrap()), // amount
            ],
        },
    ];

    let txn = TransactionBuilder::new(alice.clone())
        .batched_transaction(payload)
        .sequence_number(h.sequence_number(alice.address()))
        .max_gas_amount(1_000_000)
        .gas_unit_price(1)
        .sign();

    let result = h.run(txn);
    assert_success!(result);

    let payload = vec![
        BatchedFunctionCall {
            module: ModuleId::new(
                *alice.address(),
                Identifier::new("managed_fungible_asset").unwrap(),
            ),
            function: Identifier::new("withdraw_from_primary_stores").unwrap(),
            ty_args: vec![],
            args: vec![
                BatchArgument::Raw(bcs::to_bytes(&metadata).unwrap()),
                BatchArgument::Raw(bcs::to_bytes(&vec![alice.address()]).unwrap()),
                BatchArgument::Raw(bcs::to_bytes(&vec![30u64]).unwrap()), // amount
            ],
        },
        BatchedFunctionCall {
            module: ModuleId::new(
                *alice.address(),
                Identifier::new("managed_fungible_asset").unwrap(),
            ),
            function: Identifier::new("deposit_to_primary_stores_owned").unwrap(),
            ty_args: vec![],
            args: vec![
                BatchArgument::Raw(bcs::to_bytes(&vec![30u64]).unwrap()), // amount
                BatchArgument::Raw(bcs::to_bytes(&vec![bob.address()]).unwrap()),
                BatchArgument::Raw(bcs::to_bytes(&vec![30u64]).unwrap()), // amount
            ],
        },
    ];

    let txn = TransactionBuilder::new(alice.clone())
        .batched_transaction(payload)
        .sequence_number(h.sequence_number(alice.address()))
        .max_gas_amount(1_000_000)
        .gas_unit_price(1)
        .sign();

    let result = h.run(txn);
    assert_vm_status!(result, StatusCode::INVALID_MAIN_FUNCTION_SIGNATURE);

    let payload = vec![BatchedFunctionCall {
        module: ModuleId::new(
            *alice.address(),
            Identifier::new("managed_fungible_asset").unwrap(),
        ),
        function: Identifier::new("withdraw_from_primary_stores").unwrap(),
        ty_args: vec![],
        args: vec![
            BatchArgument::Raw(bcs::to_bytes(&metadata).unwrap()),
            BatchArgument::Raw(bcs::to_bytes(&vec![alice.address()]).unwrap()),
            BatchArgument::Raw(bcs::to_bytes(&vec![30u64]).unwrap()), // amount
        ],
    }];

    let txn = TransactionBuilder::new(alice.clone())
        .batched_transaction(payload)
        .sequence_number(h.sequence_number(alice.address()))
        .max_gas_amount(1_000_000)
        .gas_unit_price(1)
        .sign();

    let result = h.run(txn);
    assert_vm_status!(result, StatusCode::INVALID_MAIN_FUNCTION_SIGNATURE);

    let result = h.run_entry_function(
        &alice,
        str::parse(&format!(
            "0x{}::managed_fungible_asset::burn_from_primary_stores",
            (*alice.address()).to_hex()
        ))
        .unwrap(),
        vec![],
        vec![
            bcs::to_bytes(&metadata).unwrap(),
            bcs::to_bytes(&vec![bob.address()]).unwrap(),
            bcs::to_bytes(&vec![20u64]).unwrap(), // amount
        ],
    );
    assert_success!(result);

    let token_addr = account_address::create_token_address(
        *alice.address(),
        "test collection name",
        "test token name",
    );
    let alice_primary_store_addr =
        account_address::create_derived_object_address(*alice.address(), token_addr);
    let bob_primary_store_addr =
        account_address::create_derived_object_address(*bob.address(), token_addr);

    // Ensure that the group data can be read
    let mut alice_store: FungibleStore = h
        .read_resource_from_resource_group(
            &alice_primary_store_addr,
            OBJ_GROUP_TAG.clone(),
            FUNGIBLE_STORE_TAG.clone(),
        )
        .unwrap();

    let bob_store: FungibleStore = h
        .read_resource_from_resource_group(
            &bob_primary_store_addr,
            OBJ_GROUP_TAG.clone(),
            FUNGIBLE_STORE_TAG.clone(),
        )
        .unwrap();

    assert_ne!(alice_store, bob_store);
    // Determine that the only difference is the balance
    assert_eq!(alice_store.balance, 70);
    alice_store.balance = 10;
    assert_eq!(alice_store, bob_store);
}

#[test]
fn test_batched_transaction_new_module() {
    let mut h = MoveHarness::new();

    let one = h.new_account_at(AccountAddress::from_hex_literal("0x1").unwrap());
    let module = ModuleId::new(
        *one.address(),
        Identifier::new("batched_execution").unwrap(),
    );

    assert_success!(
        h.publish_package_cache_building(&one, &common::test_dir_path("batched_transaction.data"))
    );

    let payload = vec![BatchedFunctionCall {
        module: module.clone(),
        function: Identifier::new("create_droppable_value_with_signer").unwrap(),
        ty_args: vec![],
        args: vec![BatchArgument::Raw(bcs::to_bytes(&20u8).unwrap())],
    }];

    let txn = TransactionBuilder::new(one.clone())
        .batched_transaction(payload)
        .sequence_number(h.sequence_number(one.address()))
        .max_gas_amount(1_000_000)
        .gas_unit_price(1)
        .sign();

    let result = h.run(txn);
    assert_success!(result);

    let payload = vec![BatchedFunctionCall {
        module: module.clone(),
        function: Identifier::new("create_non_droppable_value_with_signer").unwrap(),
        ty_args: vec![],
        args: vec![BatchArgument::Raw(bcs::to_bytes(&20u8).unwrap())],
    }];

    let txn = TransactionBuilder::new(one.clone())
        .batched_transaction(payload)
        .sequence_number(h.sequence_number(one.address()))
        .max_gas_amount(1_000_000)
        .gas_unit_price(1)
        .sign();

    let result = h.run(txn);
    assert_vm_status!(result, StatusCode::INVALID_MAIN_FUNCTION_SIGNATURE);

    let payload = vec![BatchedFunctionCall {
        module: module.clone(),
        function: Identifier::new("create_droppable_value").unwrap(),
        ty_args: vec![],
        args: vec![BatchArgument::Raw(bcs::to_bytes(&20u8).unwrap())],
    }];

    let txn = TransactionBuilder::new(one.clone())
        .batched_transaction(payload)
        .sequence_number(h.sequence_number(one.address()))
        .max_gas_amount(1_000_000)
        .gas_unit_price(1)
        .sign();

    let result = h.run(txn);
    assert_success!(result);

    let payload = vec![
        BatchedFunctionCall {
            module: module.clone(),
            function: Identifier::new("create_non_droppable_value_with_signer").unwrap(),
            ty_args: vec![],
            args: vec![BatchArgument::Raw(bcs::to_bytes(&20u8).unwrap())],
        },
        BatchedFunctionCall {
            module: module.clone(),
            function: Identifier::new("consume_non_droppable_value").unwrap(),
            ty_args: vec![],
            args: vec![
                BatchArgument::PreviousResult(0, 0),
                BatchArgument::Raw(bcs::to_bytes(&20u8).unwrap()),
            ],
        },
    ];

    let txn = TransactionBuilder::new(one.clone())
        .batched_transaction(payload)
        .sequence_number(h.sequence_number(one.address()))
        .max_gas_amount(1_000_000)
        .gas_unit_price(1)
        .sign();

    let result = h.run(txn);
    assert_success!(result);

    let payload = vec![
        BatchedFunctionCall {
            module: module.clone(),
            function: Identifier::new("create_non_droppable_value_with_signer").unwrap(),
            ty_args: vec![],
            args: vec![BatchArgument::Raw(bcs::to_bytes(&20u8).unwrap())],
        },
        BatchedFunctionCall {
            module: module.clone(),
            function: Identifier::new("consume_droppable_value").unwrap(),
            ty_args: vec![],
            args: vec![
                BatchArgument::PreviousResult(0, 0),
                BatchArgument::Raw(bcs::to_bytes(&20u8).unwrap()),
            ],
        },
    ];

    let txn = TransactionBuilder::new(one.clone())
        .batched_transaction(payload)
        .sequence_number(h.sequence_number(one.address()))
        .max_gas_amount(1_000_000)
        .gas_unit_price(1)
        .sign();

    let result = h.run(txn);
    assert_vm_status!(result, StatusCode::INVALID_MAIN_FUNCTION_SIGNATURE);

    let payload = vec![
        BatchedFunctionCall {
            module: module.clone(),
            function: Identifier::new("create_non_droppable_value_with_signer").unwrap(),
            ty_args: vec![],
            args: vec![BatchArgument::Raw(bcs::to_bytes(&20u8).unwrap())],
        },
        BatchedFunctionCall {
            module: module.clone(),
            function: Identifier::new("consume_non_droppable_value").unwrap(),
            ty_args: vec![],
            args: vec![
                BatchArgument::PreviousResult(0, 0),
                BatchArgument::Raw(bcs::to_bytes(&20u8).unwrap()),
            ],
        },
        BatchedFunctionCall {
            module: module.clone(),
            function: Identifier::new("consume_non_droppable_value").unwrap(),
            ty_args: vec![],
            args: vec![
                BatchArgument::PreviousResult(0, 0),
                BatchArgument::Raw(bcs::to_bytes(&20u8).unwrap()),
            ],
        },
    ];

    let txn = TransactionBuilder::new(one.clone())
        .batched_transaction(payload)
        .sequence_number(h.sequence_number(one.address()))
        .max_gas_amount(1_000_000)
        .gas_unit_price(1)
        .sign();

    let result = h.run(txn);
    assert_vm_status!(result, StatusCode::INVALID_MAIN_FUNCTION_SIGNATURE);

    let payload = vec![
        BatchedFunctionCall {
            module: module.clone(),
            function: Identifier::new("create_copyable_value").unwrap(),
            ty_args: vec![],
            args: vec![BatchArgument::Raw(bcs::to_bytes(&20u8).unwrap())],
        },
        BatchedFunctionCall {
            module: module.clone(),
            function: Identifier::new("consume_copyable_value").unwrap(),
            ty_args: vec![],
            args: vec![
                BatchArgument::PreviousResult(0, 0),
                BatchArgument::Raw(bcs::to_bytes(&20u8).unwrap()),
            ],
        },
        BatchedFunctionCall {
            module: module.clone(),
            function: Identifier::new("consume_copyable_value").unwrap(),
            ty_args: vec![],
            args: vec![
                BatchArgument::PreviousResult(0, 0),
                BatchArgument::Raw(bcs::to_bytes(&20u8).unwrap()),
            ],
        },
    ];

    let txn = TransactionBuilder::new(one.clone())
        .batched_transaction(payload)
        .sequence_number(h.sequence_number(one.address()))
        .max_gas_amount(1_000_000)
        .gas_unit_price(1)
        .sign();

    let result = h.run(txn);
    assert_success!(result);
}
