// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

/// The purpose of these tests are to record BCS serialized and Ed25519 signed tests in golden files.
/// These files will serve as references for the transaction signing implementations in client SDK.
///
/// Most of the transactions are with faked transaction payloads. The goal of these tests are verifying
/// transaction serialization and signing. The type args and arguments in payloads do not always make sense.
use crate::{
    current_function_name,
    tests::{new_test_context, TestContext},
};
use aptos_types::{
    access_path::{AccessPath, Path},
    account_address::AccountAddress,
    state_store::state_key::StateKey,
    transaction::{
        ChangeSet, RawTransaction, Script, ScriptFunction, SignedTransaction, TransactionArgument,
    },
    utility_coin::TEST_COIN_TYPE,
    write_set::{WriteOp, WriteSetMut},
};

use aptos_crypto::{
    ed25519::{Ed25519PrivateKey, Ed25519PublicKey},
    SigningKey, Uniform,
};
use move_deps::move_core_types::{
    identifier::Identifier,
    language_storage::{ModuleId, StructTag},
};
use serde_json::{self, json};

#[cfg(test)]
async fn test_transaction_with_diffent_payload<F>(test_name: &'static str, f: F)
where
    F: Fn(&TestContext) -> RawTransaction,
{
    let mut context = new_test_context(test_name);

    let raw_txn = f(&context);

    let signing_message = hex::encode(raw_txn.signing_message());

    let private_key = Ed25519PrivateKey::generate_for_testing();
    let public_key = Ed25519PublicKey::from(&private_key);

    let signature = private_key.sign(&raw_txn);
    let txn = SignedTransaction::new(raw_txn.clone(), public_key.clone(), signature);

    context.check_golden_output(json!({
      "raw_txn": serde_json::from_slice::<serde_json::Value>(serde_json::to_string(&raw_txn).unwrap().as_bytes()).unwrap(),
      "signing_message": serde_json::Value::String(signing_message),
      "signed_txn_bcs": serde_json::Value::String(hex::encode(bcs::to_bytes(&txn).unwrap())),
      "private_key": serde_json::Value::String(hex::encode(private_key.to_bytes())),
      "public_key": serde_json::Value::String(hex::encode(public_key.to_bytes())),
    }));
}

// Script function has no type args
#[tokio::test]
async fn test_script_function_payload_variant_1() {
    test_transaction_with_diffent_payload(current_function_name!(), |context| {
        let sender = context.root_account();

        let transaction_factory = context.transaction_factory();
        transaction_factory
            .script_function(ScriptFunction::new(
                ModuleId::new(
                    AccountAddress::from_hex_literal("0x1222").unwrap(),
                    Identifier::new("TestCoin").unwrap(),
                ),
                Identifier::new("transfer").unwrap(),
                vec![],
                vec![
                    bcs::to_bytes(&AccountAddress::from_hex_literal("0xdd").unwrap()).unwrap(),
                    bcs::to_bytes(&1u64).unwrap(),
                ],
            ))
            .sender(sender.address())
            .sequence_number(sender.sequence_number())
            .expiration_timestamp_secs(u64::MAX)
            .build()
    })
    .await;
}

// Script function has type args
#[tokio::test]
async fn test_script_function_payload_variant_2() {
    test_transaction_with_diffent_payload(current_function_name!(), |context| {
        let sender = context.root_account();

        let transaction_factory = context.transaction_factory();
        transaction_factory
            .script_function(ScriptFunction::new(
                ModuleId::new(
                    AccountAddress::from_hex_literal("0x1222").unwrap(),
                    Identifier::new("Coin").unwrap(),
                ),
                Identifier::new("transfer").unwrap(),
                vec![TEST_COIN_TYPE.clone()],
                vec![
                    bcs::to_bytes(&AccountAddress::from_hex_literal("0xdd").unwrap()).unwrap(),
                    bcs::to_bytes(&1u64).unwrap(),
                ],
            ))
            .sender(sender.address())
            .sequence_number(sender.sequence_number())
            .expiration_timestamp_secs(u64::MAX)
            .build()
    })
    .await;
}

// Script function with no function arguments
#[tokio::test]
async fn test_script_function_payload_variant_3() {
    test_transaction_with_diffent_payload(current_function_name!(), |context| {
        let sender = context.root_account();

        let transaction_factory = context.transaction_factory();
        transaction_factory
            .script_function(ScriptFunction::new(
                ModuleId::new(
                    AccountAddress::from_hex_literal("0x1222").unwrap(),
                    Identifier::new("Coin").unwrap(),
                ),
                Identifier::new("fake_func").unwrap(),
                vec![TEST_COIN_TYPE.clone()],
                vec![],
            ))
            .sender(sender.address())
            .sequence_number(sender.sequence_number())
            .expiration_timestamp_secs(u64::MAX)
            .build()
    })
    .await;
}

// Script payload with no type args and no args to the decoded script function
#[tokio::test]
async fn test_script_payload_variant_1() {
    test_transaction_with_diffent_payload(current_function_name!(), |context| {
        let sender = context.root_account();

        let script = hex::decode(
            "a11ceb0b030000000105000100000000050601000000000000000600000000000000001a0102",
        )
        .unwrap();
        let transaction_factory = context.transaction_factory();
        transaction_factory
            .script(Script::new(script, vec![], vec![]))
            .sender(sender.address())
            .sequence_number(sender.sequence_number())
            .expiration_timestamp_secs(u64::MAX)
            .build()
    })
    .await;
}

// Script payload with type args but no args to the decoded script function
#[tokio::test]
async fn test_script_payload_variant_2() {
    test_transaction_with_diffent_payload(current_function_name!(), |context| {
        let sender = context.root_account();

        let script = hex::decode(
            "a11ceb0b030000000105000100000000050601000000000000000600000000000000001a0102",
        )
        .unwrap();
        let transaction_factory = context.transaction_factory();
        transaction_factory
            .script(Script::new(script, vec![TEST_COIN_TYPE.clone()], vec![]))
            .sender(sender.address())
            .sequence_number(sender.sequence_number())
            .expiration_timestamp_secs(u64::MAX)
            .build()
    })
    .await;
}

// Script payload with an argument of type 'U8'
#[tokio::test]
async fn test_script_payload_variant_3() {
    test_transaction_with_diffent_payload(current_function_name!(), |context| {
        let sender = context.root_account();

        let script = hex::decode(
            "a11ceb0b030000000105000100000000050601000000000000000600000000000000001a0102",
        )
        .unwrap();
        let transaction_factory = context.transaction_factory();
        transaction_factory
            .script(Script::new(
                script,
                vec![TEST_COIN_TYPE.clone()],
                vec![TransactionArgument::U8(2)],
            ))
            .sender(sender.address())
            .sequence_number(sender.sequence_number())
            .expiration_timestamp_secs(u64::MAX)
            .build()
    })
    .await;
}

// Script payload with an argument of type 'U8Vector' and argument of type 'Address'
#[tokio::test]
async fn test_script_payload_variant_4() {
    test_transaction_with_diffent_payload(current_function_name!(), |context| {
        let sender = context.root_account();

        let script = hex::decode(
            "a11ceb0b030000000105000100000000050601000000000000000600000000000000001a0102",
        )
        .unwrap();
        let transaction_factory = context.transaction_factory();
        transaction_factory
            .script(Script::new(
                script,
                vec![TEST_COIN_TYPE.clone()],
                vec![
                    TransactionArgument::U8Vector(bcs::to_bytes(&1u64).unwrap()),
                    TransactionArgument::Address(AccountAddress::from_hex_literal("0x1").unwrap()),
                ],
            ))
            .sender(sender.address())
            .sequence_number(sender.sequence_number())
            .expiration_timestamp_secs(u64::MAX)
            .build()
    })
    .await;
}

#[tokio::test]
async fn test_transaction_with_module_payload() {
    test_transaction_with_diffent_payload(current_function_name!(), |context| {
      let sender = context.root_account();

      let transaction_factory = context.transaction_factory();
      let code = "a11ceb0b0300000006010002030205050703070a0c0816100c260900000001000100000102084d794d6f64756c650269640000000000000000000000000b1e55ed00010000000231010200";
      transaction_factory
          .module(hex::decode(code).unwrap())
          .sender(sender.address())
          .sequence_number(sender.sequence_number())
          .expiration_timestamp_secs(u64::MAX)
          .build()
    }).await;
}

#[ignore]
#[tokio::test]
async fn test_transaction_with_write_set_payload() {
    test_transaction_with_diffent_payload(current_function_name!(), |context| {
        let sender = context.root_account();

        let code_address = AccountAddress::from_hex_literal("0x1").unwrap();
        let transaction_factory = context.transaction_factory();
        transaction_factory
            .change_set(ChangeSet::new(
                WriteSetMut::new(vec![
                    (
                        StateKey::AccessPath(AccessPath::new(
                            code_address,
                            bcs::to_bytes(&Path::Code(ModuleId::new(
                                code_address,
                                Identifier::new("Account").unwrap(),
                            )))
                            .unwrap(),
                        )),
                        WriteOp::Deletion,
                    ),
                    (
                        StateKey::AccessPath(AccessPath::new(
                            context.root_account().address(),
                            bcs::to_bytes(&Path::Resource(StructTag {
                                address: code_address,
                                module: Identifier::new("TestCoin").unwrap(),
                                name: Identifier::new("Balance").unwrap(),
                                type_params: vec![],
                            }))
                            .unwrap(),
                        )),
                        WriteOp::Deletion,
                    ),
                ])
                .freeze()
                .unwrap(),
                vec![],
            ))
            .sender(sender.address())
            .sequence_number(sender.sequence_number())
            .expiration_timestamp_secs(u64::MAX)
            .build()
    })
    .await;
}
