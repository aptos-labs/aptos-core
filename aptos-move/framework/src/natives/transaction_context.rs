// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::natives::helpers::{make_safe_native, SafeNativeContext, SafeNativeResult};
use aptos_types::on_chain_config::{Features, TimedFeatures};
use better_any::{Tid, TidAble};
use move_core_types::gas_algebra::InternalGas;
use move_vm_runtime::native_functions::NativeFunction;
use move_vm_types::{loaded_data::runtime_types::Type, values::Value};
use smallvec::{smallvec, SmallVec};
use std::{collections::VecDeque, fmt::Debug, sync::Arc};

/// The native transaction context extension. This needs to be attached to the
/// NativeContextExtensions value which is passed into session functions, so its accessible from
/// natives of this extension.
#[derive(Tid)]
pub struct NativeTransactionContext {
    script_hash: Vec<u8>,
    chain_id: u8,
}

impl NativeTransactionContext {
    /// Create a new instance of a native transaction context. This must be passed in via an
    /// extension into VM session functions.
    pub fn new(script_hash: Vec<u8>, chain_id: u8) -> Self {
        Self {
            script_hash,
            chain_id,
        }
    }

    pub fn chain_id(&self) -> u8 {
        self.chain_id
    }
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
    pub get_script_hash: GetScriptHashGasParameters,
}

pub fn make_all(
    gas_params: GasParameters,
    timed_features: TimedFeatures,
    features: Arc<Features>,
) -> impl Iterator<Item = (String, NativeFunction)> {
    let natives = [(
        "get_script_hash",
        make_safe_native(
            gas_params.get_script_hash,
            timed_features,
            features,
            native_get_script_hash,
        ),
    )];

    crate::natives::helpers::make_module_natives(natives)
}
