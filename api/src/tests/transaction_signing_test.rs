// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

/// The purpose of these tests are to record BCS serialized and Ed25519 signed tests in golden files.
/// These files will serve as references for the transaction signing implementations in client SDK.
///
/// Most of the transactions are with faked transaction payloads. The goal of these tests are verifying
/// transaction serialization and signing. The type args and arguments in payloads do not always make sense.
use crate::{current_function_name, tests::new_test_context};
use aptos_types::{
    account_address::AccountAddress,
    chain_id::ChainId,
    transaction::{RawTransaction, Script, ScriptFunction, SignedTransaction, TransactionArgument},
};

use aptos_crypto::{
    ed25519::{Ed25519PrivateKey, Ed25519PublicKey},
    SigningKey, Uniform,
};
use aptos_proptest_helpers::ValueGenerator;
use move_deps::move_core_types::{
    identifier::Identifier,
    language_storage::{ModuleId, StructTag, TypeTag},
};
use proptest::{arbitrary::any, collection, prelude::*, string};
use serde::Serialize;
use serde_json::{self, json, ser::Formatter};
use std::io::{self, Write};

#[cfg(test)]
struct U64ToStringFormatter;

/// u64 might get truncated when being serialized into javascript Number.
/// This formatter converts u64 numbers into strings in javascript.
#[cfg(test)]
impl Formatter for U64ToStringFormatter {
    fn write_u64<W>(&mut self, writer: &mut W, value: u64) -> io::Result<()>
    where
        W: ?Sized + Write,
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

                TypeTag::Struct(StructTag {
                    address: addr,
                    module: Identifier::new(module).unwrap(),
                    name: Identifier::new(name).unwrap(),
                    type_params: t_vec,
                })}),
        ]
    })
}

#[cfg(test)]
#[derive(Debug)]
struct Arg {
    bcs_bytes: Vec<u8>,
    literal: String,
}

#[cfg(test)]
impl Arg {
    fn new(arg: impl Serialize) -> Arg {
        Arg {
            bcs_bytes: bcs::to_bytes(&arg).unwrap(),
            literal: serde_json::to_string(&arg).unwrap(),
        }
    }
}

#[cfg(test)]
fn arg_strategy() -> impl Strategy<Value = Arg> {
    prop_oneof![
        1 => any::<u8>().prop_map(Arg::new),
        1 => any::<u64>().prop_map(Arg::new),
        // u128 is not well supported by serde::json
        // 1 => any::<u128>().prop_map(Arg::new),
        1 => any::<bool>().prop_map(Arg::new),
        1 => any::<AccountAddress>().prop_map(Arg::new),
    ]
}

#[cfg(test)]
fn script_function_strategy() -> impl Strategy<Value = ScriptFunction> {
    (
        any::<AccountAddress>(),
        coin_name_strategy(),
        identifier_strategy(),
        collection::vec(type_tag_strategy(), 0..=10),
        collection::vec(arg_strategy(), 0..=10),
    )
        .prop_map(|(addr, coin, func, type_args, args)| {
            ScriptFunction::new(
                ModuleId::new(addr, Identifier::new(coin).unwrap()),
                Identifier::new(func).unwrap(),
                type_args,
                args.iter().map(|arg| arg.bcs_bytes.clone()).collect(),
            )
        })
}

#[cfg(test)]
#[derive(Debug)]
struct HexCode {
    bytes: Vec<u8>,
    literal: String,
}

#[cfg(test)]
impl HexCode {
    fn new(hex: String) -> HexCode {
        HexCode {
            bytes: hex::decode(&hex).unwrap(),
            literal: hex,
        }
    }
}

#[cfg(test)]
fn hex_code_strategy() -> impl Strategy<Value = HexCode> {
    string::string_regex("[a-f0-9]+")
        .unwrap()
        .prop_filter("only even letters count", |s| s.len() % 2 == 0)
        .prop_map(HexCode::new)
}

#[cfg(test)]
fn random_bytes_strategy() -> impl Strategy<Value = Vec<u8>> {
    hex_code_strategy().prop_map(|h| h.bytes)
}

#[cfg(test)]
fn transaction_argument_strategy() -> impl Strategy<Value = TransactionArgument> {
    prop_oneof![
        1 => any::<u8>().prop_map(TransactionArgument::U8),
        1 => any::<u64>().prop_map(TransactionArgument::U64),
        // u128 is not well supported by serde::json
        // 1 => any::<u128>().prop_map(TransactionArgument::U128),
        1 => any::<bool>().prop_map(TransactionArgument::Bool),
        1 => any::<AccountAddress>().prop_map(TransactionArgument::Address),
        1 => random_bytes_strategy().prop_map(TransactionArgument::U8Vector),
    ]
}

#[cfg(test)]
fn script_strategy() -> impl Strategy<Value = Script> {
    (
        hex_code_strategy(),
        collection::vec(type_tag_strategy(), 0..=10),
        collection::vec(transaction_argument_strategy(), 0..=10),
    )
        .prop_map(|(hex_code, type_args, args)| Script::new(hex_code.bytes, type_args, args))
}

#[cfg(test)]
fn gen_u64(gen: &mut ValueGenerator) -> u64 {
    gen.generate(any::<u64>())
}

#[cfg(test)]
fn gen_chain_id(gen: &mut ValueGenerator) -> u8 {
    gen.generate(chain_id_strategy())
}

#[cfg(test)]
fn gen_address(gen: &mut ValueGenerator) -> AccountAddress {
    gen.generate(any::<AccountAddress>())
}

#[cfg(test)]
fn gen_script_function(gen: &mut ValueGenerator) -> ScriptFunction {
    gen.generate(script_function_strategy())
}

#[cfg(test)]
fn gen_script(gen: &mut ValueGenerator) -> Script {
    gen.generate(script_strategy())
}

#[cfg(test)]
fn gen_module_code(gen: &mut ValueGenerator) -> Vec<u8> {
    gen.generate(random_bytes_strategy())
}

#[cfg(test)]
fn sign_transaction(raw_txn: RawTransaction) -> serde_json::Value {
    let private_key = Ed25519PrivateKey::generate_for_testing();
    let public_key = Ed25519PublicKey::from(&private_key);

    let signature = private_key.sign(&raw_txn);
    let txn = SignedTransaction::new(raw_txn.clone(), public_key.clone(), signature);

    let mut raw_txn_json_out = Vec::new();
    let formatter = U64ToStringFormatter;
    let mut ser = serde_json::Serializer::with_formatter(&mut raw_txn_json_out, formatter);
    raw_txn.serialize(&mut ser).unwrap();

    json!({
        "raw_txn": serde_json::from_slice::<serde_json::Value>(&raw_txn_json_out[..]).unwrap(),
        "signed_txn_bcs": serde_json::Value::String(hex::encode(bcs::to_bytes(&txn).unwrap())),
        "private_key": serde_json::Value::String(hex::encode(private_key.to_bytes())),
    })
}

#[tokio::test]
async fn test_script_function_payload() {
    let mut context = new_test_context(current_function_name!());

    let mut value_gen = ValueGenerator::deterministic();
    let mut txns = vec![];
    for _ in 0..100 {
        let transaction_factory = context.transaction_factory();
        let raw_txn = transaction_factory
            .script_function(gen_script_function(&mut value_gen))
            .sender(gen_address(&mut value_gen))
            .sequence_number(gen_u64(&mut value_gen))
            .expiration_timestamp_secs(gen_u64(&mut value_gen))
            .max_gas_amount(gen_u64(&mut value_gen))
            .gas_unit_price(gen_u64(&mut value_gen))
            .chain_id(ChainId::new(gen_chain_id(&mut value_gen)))
            .build();
        txns.push(sign_transaction(raw_txn));
    }

    context.check_golden_output(json!(txns));
}

#[tokio::test]
async fn test_script_payload() {
    let mut context = new_test_context(current_function_name!());

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
            .build();
        txns.push(sign_transaction(raw_txn));
    }

    context.check_golden_output(json!(txns));
}

#[tokio::test]
async fn test_module_payload() {
    let mut context = new_test_context(current_function_name!());

    let mut value_gen = ValueGenerator::deterministic();
    let mut txns = vec![];
    for _ in 0..100 {
        let transaction_factory = context.transaction_factory();
        let raw_txn = transaction_factory
            .module(gen_module_code(&mut value_gen))
            .sender(gen_address(&mut value_gen))
            .sequence_number(gen_u64(&mut value_gen))
            .expiration_timestamp_secs(gen_u64(&mut value_gen))
            .max_gas_amount(gen_u64(&mut value_gen))
            .gas_unit_price(gen_u64(&mut value_gen))
            .chain_id(ChainId::new(gen_chain_id(&mut value_gen)))
            .build();
        txns.push(sign_transaction(raw_txn));
    }

    context.check_golden_output(json!(txns));
}
