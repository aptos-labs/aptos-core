// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    natives::helpers::{
        make_module_natives, make_safe_native, SafeNativeContext, SafeNativeError, SafeNativeResult,
    },
    safely_pop_arg,
};
use aptos_gas_algebra_ext::{AbstractValueSize, InternalGasPerAbstractValueUnit};
use aptos_types::on_chain_config::{Features, TimedFeatures};
use move_core_types::gas_algebra::InternalGas;
use move_vm_runtime::native_functions::NativeFunction;
use move_vm_types::{loaded_data::runtime_types::Type, values::Value};
use smallvec::{smallvec, SmallVec};
use std::{collections::VecDeque, sync::Arc};

/***************************************************************************************************
 * native fun write_to_event_store
 *
 *   gas cost: base_cost
 *
 **************************************************************************************************/
#[derive(Debug, Clone)]
pub struct WriteToEventStoreGasParameters {
    pub base: InternalGas,
    pub per_abstract_value_unit: InternalGasPerAbstractValueUnit,
}

#[inline]
fn native_write_to_event_store(
    gas_params: &WriteToEventStoreGasParameters,
    calc_abstract_val_size: impl FnOnce(&Value) -> AbstractValueSize,
    context: &mut SafeNativeContext,
    mut ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert!(ty_args.len() == 1);
    debug_assert!(arguments.len() == 3);

    let ty = ty_args.pop().unwrap();
    let msg = arguments.pop_back().unwrap();
    let seq_num = safely_pop_arg!(arguments, u64);
    let guid = safely_pop_arg!(arguments, Vec<u8>);

    // TODO(Gas): Get rid of abstract memory size
    context.charge(
        gas_params.base + gas_params.per_abstract_value_unit * calc_abstract_val_size(&msg),
    )?;

    if !context.save_event(guid, seq_num, ty, msg)? {
        return Err(SafeNativeError::Abort { abort_code: 0 });
    }

    Ok(smallvec![])
}

pub fn make_native_write_to_event_store(
    calc_abstract_val_size: impl Fn(&Value) -> AbstractValueSize + Send + Sync + 'static,
) -> impl Fn(
    &WriteToEventStoreGasParameters,
    &mut SafeNativeContext,
    Vec<Type>,
    VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    move |gas_params, context, ty_args, args| -> SafeNativeResult<SmallVec<[Value; 1]>> {
        native_write_to_event_store(gas_params, &calc_abstract_val_size, context, ty_args, args)
    }
}

/***************************************************************************************************
 * module
 *
 **************************************************************************************************/
#[derive(Debug, Clone)]
pub struct GasParameters {
    pub write_to_event_store: WriteToEventStoreGasParameters,
}

pub fn make_all(
    gas_params: GasParameters,
    calc_abstract_val_size: impl Fn(&Value) -> AbstractValueSize + Send + Sync + 'static,
    timed_features: TimedFeatures,
    features: Arc<Features>,
) -> impl Iterator<Item = (String, NativeFunction)> {
    let natives = [(
        "write_to_event_store",
        make_safe_native(
            gas_params.write_to_event_store,
            timed_features,
            features,
            make_native_write_to_event_store(calc_abstract_val_size),
        ),
    )];

    make_module_natives(natives)
}
