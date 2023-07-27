// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_gas_schedule::gas_params::natives::aptos_framework::*;
use aptos_native_interface::{
    RawSafeNative, SafeNativeBuilder, SafeNativeContext, SafeNativeResult,
};
use aptos_types::transaction::authenticator::{AuthenticationKey, AuthenticationKeyPreimage};
use better_any::{Tid, TidAble};
use move_core_types::account_address::AccountAddress;
use move_vm_runtime::native_functions::NativeFunction;
use move_vm_types::{loaded_data::runtime_types::Type, values::Value};
use smallvec::{smallvec, SmallVec};
use std::collections::VecDeque;

/// The native transaction context extension. This needs to be attached to the
/// NativeContextExtensions value which is passed into session functions, so its accessible from
/// natives of this extension.
#[derive(Tid)]
pub struct NativeTransactionContext {
    txn_hash: Vec<u8>,
    /// This is the number of AUIDs (Aptos unique identifiers) issued during the execution of this transaction
    auid_counter: u64,
    script_hash: Vec<u8>,
    chain_id: u8,
}

impl NativeTransactionContext {
    /// Create a new instance of a native transaction context. This must be passed in via an
    /// extension into VM session functions.
    pub fn new(txn_hash: Vec<u8>, script_hash: Vec<u8>, chain_id: u8) -> Self {
        Self {
            txn_hash,
            auid_counter: 0,
            script_hash,
            chain_id,
        }
    }

    pub fn chain_id(&self) -> u8 {
        self.chain_id
    }
}

/***************************************************************************************************
 * native fun get_txn_hash
 *
 *   gas cost: base_cost
 *
 **************************************************************************************************/
fn native_get_txn_hash(
    context: &mut SafeNativeContext,
    mut _ty_args: Vec<Type>,
    _args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    context.charge(TRANSACTION_CONTEXT_GET_TXN_HASH_BASE)?;
    let transaction_context = context.extensions().get::<NativeTransactionContext>();

    Ok(smallvec![Value::vector_u8(
        transaction_context.txn_hash.clone()
    )])
}

/***************************************************************************************************
 * native fun generate_unique_address
 *
 *   gas cost: base_cost
 *
 **************************************************************************************************/
fn native_generate_unique_address(
    context: &mut SafeNativeContext,
    mut _ty_args: Vec<Type>,
    _args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    context.charge(TRANSACTION_CONTEXT_GENERATE_UNIQUE_ADDRESS_BASE)?;

    let transaction_context = context
        .extensions_mut()
        .get_mut::<NativeTransactionContext>();
    transaction_context.auid_counter += 1;

    let hash_vec = AuthenticationKey::from_preimage(&AuthenticationKeyPreimage::auid(
        transaction_context.txn_hash.clone(),
        transaction_context.auid_counter,
    ));
    Ok(smallvec![Value::address(AccountAddress::new(
        hash_vec
            .to_vec()
            .try_into()
            .expect("Unable to convert hash vector into [u8]")
    ))])
}

/***************************************************************************************************
 * native fun get_script_hash
 *
 *   gas cost: base_cost
 *
 **************************************************************************************************/
fn native_get_script_hash(
    context: &mut SafeNativeContext,
    mut _ty_args: Vec<Type>,
    _args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    context.charge(TRANSACTION_CONTEXT_GET_SCRIPT_HASH_BASE)?;

    let transaction_context = context.extensions().get::<NativeTransactionContext>();

    Ok(smallvec![Value::vector_u8(
        transaction_context.script_hash.clone()
    )])
}

/***************************************************************************************************
 * module
 *
 **************************************************************************************************/
pub fn make_all(
    builder: &SafeNativeBuilder,
) -> impl Iterator<Item = (String, NativeFunction)> + '_ {
    let natives = [
        ("get_script_hash", native_get_script_hash as RawSafeNative),
        ("generate_unique_address", native_generate_unique_address),
        ("get_txn_hash", native_get_txn_hash),
    ];

    builder.make_named_natives(natives)
}
