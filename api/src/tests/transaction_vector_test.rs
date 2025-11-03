// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![allow(clippy::arc_with_non_send_sync)]

/***************************************************************************************
 *
 * Purpose of transaction vector test
 *
 *  a. Validates that the BCS and transaction signing code always genereate consistent bytes
 *  b. The golden files contain the expected outputs for various transaction payloads. These files could be used by
 *     other languages to verify their implementations of the transaction signing code.
 *  c. The transaction payload generation heavily relies on proptest. We need to make sure the proptest uses a
 *     deterministic RNG to not violate the golden file rules.
 *
 **************************************************************************************/

use super::new_test_context;
use aptos_api_test_context::{current_function_name, TestContext};
use aptos_crypto::{
    ed25519::{Ed25519PrivateKey, Ed25519PublicKey},
    SigningKey, Uniform,
};
use aptos_proptest_helpers::ValueGenerator;
use aptos_types::{
    account_address::AccountAddress,
    chain_id::ChainId,
    transaction::{EntryFunction, RawTransaction, Script, SignedTransaction, TransactionArgument},
};
use move_core_types::{
    identifier::Identifier,
    language_storage::{ModuleId, StructTag, TypeTag},
};
use proptest::{arbitrary::any, collection, prelude::*, string};
use serde::Serialize;
use serde_json::{self, json, ser::Formatter};
use std::io::{self, Write};

#[cfg(test)]
struct NumberToStringFormatter;

/// "u64" and "u128" might get truncated when being serialized into javascript Number.
/// This formatter converts u64 and u128 numbers into strings in javascript.
#[cfg(test)]
impl Formatter for NumberToStringFormatter {
    // Formats u64 as a string
    fn write_u64<W>(&mut self, writer: &mut W, value: u64) -> io::Result<()>
    where
        W: ?Sized + Write,
    {
        write!(writer, "\"{}\"", value)
    }

    // Formats u128 as a string
    fn write_u128<W>(&mut self, writer: &mut W, value: u128) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        write!(writer, "\"{}\"", value)
    }
}

#[cfg(test)]
fn chain_id_strategy() -> impl Strategy<Value = u8> {
    any::<u8>().prop_filter("no 0 chain id", |x| x > &0)
}

#[cfg(test)]
fn coin_name_strategy() -> impl Strategy<Value = String> {
    string::string_regex("[a-zA-Z]+[0-9]?_Coin").unwrap()
}

#[cfg(test)]
fn identifier_strategy() -> impl Strategy<Value = String> {
    string::string_regex("[a-zA-Z]+[0-9]?").unwrap()
}

#[cfg(test)]
fn type_tag_strategy() -> impl Strategy<Value = TypeTag> {
    let leaf = prop_oneof![
        1 => Just(TypeTag::Bool),
        1 => Just(TypeTag::U8),
        1 => Just(TypeTag::U64),
        1 => Just(TypeTag::U128),
        1 => Just(TypeTag::Address),
        1 => Just(TypeTag::Signer),
    ];

    leaf.prop_recursive(8, 32, 2, |inner| {
        prop_oneof![
            1 => inner.clone().prop_map(|type_tag| TypeTag::Vector(Box::new(type_tag))),
            1 => (
                collection::vec(inner, 0..1),
                any::<AccountAddress>(),
                identifier_strategy(),
                identifier_strategy()).prop_map(|(t_vec, addr, module, name)| {

                TypeTag::Struct(Box::new(StructTag {
                    address: addr,
                    module: Identifier::new(module).unwrap(),
                    name: Identifier::new(name).unwrap(),
                    type_args: t_vec,
                }))}),
        ]
    })
}

#[cfg(test)]
#[derive(Debug)]
struct Arg(Vec<u8>);

#[cfg(test)]
impl Arg {
    fn new(arg: impl Serialize) -> Arg {
        Arg(bcs::to_bytes(&arg).unwrap())
    }
}

#[cfg(test)]
fn arg_strategy() -> impl Strategy<Value = Arg> {
    prop_oneof![
        1 => any::<u8>().prop_map(Arg::new),
        1 => any::<u64>().prop_map(Arg::new),
        1 => any::<u128>().prop_map(Arg::new),
        1 => any::<bool>().prop_map(Arg::new),
        1 => any::<AccountAddress>().prop_map(Arg::new),
    ]
}

#[cfg(test)]
fn entry_function_strategy() -> impl Strategy<Value = EntryFunction> {
    (
        any::<AccountAddress>(),
        coin_name_strategy(),
        identifier_strategy(),
        collection::vec(type_tag_strategy(), 0..=10),
        collection::vec(arg_strategy(), 0..=10),
    )
        .prop_map(|(addr, coin, func, type_args, args)| {
            EntryFunction::new(
                ModuleId::new(addr, Identifier::new(coin).unwrap()),
                Identifier::new(func).unwrap(),
                type_args,
                args.iter().map(|arg| arg.0.clone()).collect(),
            )
        })
}

#[cfg(test)]
fn bytes_strategy() -> impl Strategy<Value = Vec<u8>> {
    string::string_regex("[a-f0-9]+")
        .unwrap()
        .prop_filter("only even letters count", |s| s.len() % 2 == 0)
        .prop_map(|s| hex::decode(s).unwrap())
}

#[cfg(test)]
fn transaction_argument_strategy() -> impl Strategy<Value = TransactionArgument> {
    prop_oneof![
        1 => any::<u8>().prop_map(TransactionArgument::U8),
        1 => any::<u64>().prop_map(TransactionArgument::U64),
        1 => any::<u128>().prop_map(TransactionArgument::U128),
        1 => any::<bool>().prop_map(TransactionArgument::Bool),
        1 => any::<AccountAddress>().prop_map(TransactionArgument::Address),
        1 => bytes_strategy().prop_map(TransactionArgument::U8Vector),
    ]
}

#[cfg(test)]
fn script_strategy() -> impl Strategy<Value = Script> {
    (
        bytes_strategy(),
        collection::vec(type_tag_strategy(), 0..=10),
        collection::vec(transaction_argument_strategy(), 0..=10),
    )
        .prop_map(|(bytes, type_args, args)| Script::new(bytes, type_args, args))
}

#[cfg(test)]
fn gen_u64(r#gen: &mut ValueGenerator) -> u64 {
    r#gen.generate(any::<u64>())
}

#[cfg(test)]
fn gen_chain_id(r#gen: &mut ValueGenerator) -> u8 {
    r#gen.generate(chain_id_strategy())
}

#[cfg(test)]
fn gen_address(r#gen: &mut ValueGenerator) -> AccountAddress {
    r#gen.generate(any::<AccountAddress>())
}

#[cfg(test)]
fn gen_entry_function(r#gen: &mut ValueGenerator) -> EntryFunction {
    r#gen.generate(entry_function_strategy())
}

#[cfg(test)]
fn gen_script(r#gen: &mut ValueGenerator) -> Script {
    r#gen.generate(script_strategy())
}

#[cfg(test)]
fn sign_transaction(raw_txn: RawTransaction) -> serde_json::Value {
    let private_key = Ed25519PrivateKey::generate_for_testing();
    let public_key = Ed25519PublicKey::from(&private_key);

    let signature = private_key.sign(&raw_txn).unwrap();
    let txn = SignedTransaction::new(raw_txn.clone(), public_key, signature);

    let mut raw_txn_json_out = Vec::new();
    let formatter = NumberToStringFormatter;
    let mut ser = serde_json::Serializer::with_formatter(&mut raw_txn_json_out, formatter);
    raw_txn.serialize(&mut ser).unwrap();

    json!({
        "raw_txn": serde_json::from_slice::<serde_json::Value>(&raw_txn_json_out[..]).unwrap(),
        "signed_txn_bcs": serde_json::Value::String(hex::encode(bcs::to_bytes(&txn).unwrap())),
        "private_key": serde_json::Value::String(hex::encode(private_key.to_bytes())),
    })
}

#[cfg(test)]
fn visit_json_field<'a>(v: &'a mut serde_json::Value, paths: &[&str]) -> &'a mut serde_json::Value {
    let mut field = v;
    for p in paths {
        let obj = field.as_object_mut().unwrap();
        field = obj.get_mut(*p).unwrap();
    }
    field
}

#[cfg(test)]
fn byte_array_to_hex(v: &mut serde_json::Value) -> serde_json::Value {
    let mut byte_array: Vec<u8> = vec![];
    for b in v.as_array_mut().unwrap() {
        byte_array.push(b.as_u64().unwrap() as u8);
    }
    serde_json::Value::String(hex::encode(byte_array))
}

async fn entry_function_payload(context: &mut TestContext) -> Vec<serde_json::Value> {
    // The purpose of patches is to convert bytes arrays to hex-coded strings.
    // Patches the serde_json result is easier comparing to implement a customized serializer.
    fn patch(raw_txn_json: &mut serde_json::Value, use_txn_payload_v2_format: bool) {
        let args = if use_txn_payload_v2_format {
            visit_json_field(raw_txn_json, &[
                "raw_txn",
                "payload",
                "Payload",
                "V1",
                "executable",
                "EntryFunction",
                "args",
            ])
        } else {
            visit_json_field(raw_txn_json, &[
                "raw_txn",
                "payload",
                "EntryFunction",
                "args",
            ])
        };

        let mut hex_args: Vec<serde_json::Value> = vec![];
        for arg in args.as_array_mut().unwrap() {
            hex_args.push(byte_array_to_hex(arg));
        }

        *args = json!(hex_args);
    }

    let mut value_gen = ValueGenerator::deterministic();
    let mut txns = vec![];
    for _ in 0..100 {
        let transaction_factory = context.transaction_factory();
        let raw_txn = transaction_factory
            .entry_function(gen_entry_function(&mut value_gen))
            .sender(gen_address(&mut value_gen))
            .sequence_number(gen_u64(&mut value_gen))
            .expiration_timestamp_secs(gen_u64(&mut value_gen))
            .max_gas_amount(gen_u64(&mut value_gen))
            .gas_unit_price(gen_u64(&mut value_gen))
            .chain_id(ChainId::new(gen_chain_id(&mut value_gen)))
            .upgrade_payload_with_rng(
                &mut context.rng,
                context.use_txn_payload_v2_format,
                context.use_orderless_transactions,
            )
            .build();
        let mut signed_txn = sign_transaction(raw_txn);
        patch(&mut signed_txn, context.use_txn_payload_v2_format);
        txns.push(signed_txn);
    }
    txns
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_entry_function_payload() {
    let mut context = new_test_context(current_function_name!());
    let txns = entry_function_payload(&mut context).await;
    context.check_golden_output(json!(txns));
}

async fn script_payload(context: &mut TestContext) -> Vec<serde_json::Value> {
    fn patch(raw_txn_json: &mut serde_json::Value, use_txn_payload_v2_format: bool) {
        let code = if use_txn_payload_v2_format {
            visit_json_field(raw_txn_json, &[
                "raw_txn",
                "payload",
                "Payload",
                "V1",
                "executable",
                "Script",
                "code",
            ])
        } else {
            visit_json_field(raw_txn_json, &["raw_txn", "payload", "Script", "code"])
        };
        *code = byte_array_to_hex(code);

        let args = if use_txn_payload_v2_format {
            visit_json_field(raw_txn_json, &[
                "raw_txn",
                "payload",
                "Payload",
                "V1",
                "executable",
                "Script",
                "args",
            ])
        } else {
            visit_json_field(raw_txn_json, &["raw_txn", "payload", "Script", "args"])
        };
        for arg in args.as_array_mut().unwrap() {
            let arg_obj = arg.as_object_mut().unwrap();

            if let Some(val) = arg_obj.get_mut("U8Vector") {
                *val = byte_array_to_hex(val)
            }
        }
    }

    let mut value_gen = ValueGenerator::deterministic();
    let mut txns = vec![];
    for _ in 0..100 {
        let transaction_factory = context.transaction_factory();
        let raw_txn = transaction_factory
            .script(gen_script(&mut value_gen))
            .sender(gen_address(&mut value_gen))
            .sequence_number(gen_u64(&mut value_gen))
            .expiration_timestamp_secs(gen_u64(&mut value_gen))
            .max_gas_amount(gen_u64(&mut value_gen))
            .gas_unit_price(gen_u64(&mut value_gen))
            .chain_id(ChainId::new(gen_chain_id(&mut value_gen)))
            .upgrade_payload_with_rng(
                &mut context.rng,
                context.use_txn_payload_v2_format,
                context.use_orderless_transactions,
            )
            .build();
        let mut signed_txn = sign_transaction(raw_txn);
        patch(&mut signed_txn, context.use_txn_payload_v2_format);
        txns.push(signed_txn);
    }
    txns
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_script_payload() {
    let mut context = new_test_context(current_function_name!());
    let txns = script_payload(&mut context).await;
    context.check_golden_output(json!(txns));
}
