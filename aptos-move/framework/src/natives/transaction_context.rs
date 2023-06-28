// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::natives::helpers::{make_safe_native, SafeNativeContext, SafeNativeResult};
use aptos_types::{
    on_chain_config::{Features, TimedFeatures},
    transaction::authenticator::{AuthenticationKey, AuthenticationKeyPreimage},
};
use better_any::{Tid, TidAble};
use move_core_types::{account_address::AccountAddress, gas_algebra::InternalGas};
use move_vm_runtime::native_functions::NativeFunction;
use move_vm_types::{loaded_data::runtime_types::Type, values::Value};
use smallvec::{smallvec, SmallVec};
use std::{collections::VecDeque, fmt::Debug, sync::Arc};

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
#[derive(Clone, Debug)]
pub struct GetTxnHashGasParameters {
    pub base: InternalGas,
}

fn native_get_txn_hash(
    gas_params: &GetTxnHashGasParameters,
    context: &mut SafeNativeContext,
    mut _ty_args: Vec<Type>,
    _args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    context.charge(gas_params.base)?;

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
#[derive(Clone, Debug)]
pub struct GenerateUniqueAddressGasParameters {
    pub base: InternalGas,
}

fn native_generate_unique_address(
    gas_params: &GenerateUniqueAddressGasParameters,
    context: &mut SafeNativeContext,
    mut _ty_args: Vec<Type>,
    _args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    context.charge(gas_params.base)?;

    let mut transaction_context = context
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
#[derive(Clone, Debug)]
pub struct GetScriptHashGasParameters {
    pub base: InternalGas,
}

fn native_get_script_hash(
    gas_params: &GetScriptHashGasParameters,
    context: &mut SafeNativeContext,
    mut _ty_args: Vec<Type>,
    _args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    context.charge(gas_params.base)?;

    let transaction_context = context.extensions().get::<NativeTransactionContext>();

    Ok(smallvec![Value::vector_u8(
        transaction_context.script_hash.clone()
    )])
}

/***************************************************************************************************
 * module
 *
 **************************************************************************************************/
#[derive(Debug, Clone)]
pub struct GasParameters {
    pub get_txn_hash: GetTxnHashGasParameters,
    pub get_script_hash: GetScriptHashGasParameters,
    pub generate_unique_address: GenerateUniqueAddressGasParameters,
}

pub fn make_all(
    gas_params: GasParameters,
    timed_features: TimedFeatures,
    features: Arc<Features>,
) -> impl Iterator<Item = (String, NativeFunction)> {
    let natives = [
        (
            "get_script_hash",
            make_safe_native(
                gas_params.get_script_hash,
                timed_features.clone(),
                features.clone(),
                native_get_script_hash,
            ),
        ),
        (
            "generate_unique_address",
            make_safe_native(
                gas_params.generate_unique_address,
                timed_features.clone(),
                features.clone(),
                native_generate_unique_address,
            ),
        ),
        (
            "get_txn_hash",
            make_safe_native(
                gas_params.get_txn_hash,
                timed_features,
                features,
                native_get_txn_hash,
            ),
        ),
    ];

    crate::natives::helpers::make_module_natives(natives)
}
