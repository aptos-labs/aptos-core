// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::natives::helpers::make_module_natives;
use move_deps::{
    move_binary_format::errors::PartialVMResult,
    move_core_types::gas_algebra::{InternalGas, InternalGasPerAbstractMemoryUnit},
    move_vm_runtime::native_functions::{NativeContext, NativeFunction},
    move_vm_types::{
        loaded_data::runtime_types::Type, natives::function::NativeResult, pop_arg, values::Value,
    },
};

use smallvec::smallvec;
use std::{collections::VecDeque, sync::Arc};

/***************************************************************************************************
 * native fun write_to_event_store
 *
 *   gas cost: base_cost
 *
 **************************************************************************************************/
#[derive(Debug, Clone)]
pub struct WriteToEventStoreGasParameters {
    pub base_cost: InternalGas,
    pub unit_cost: InternalGasPerAbstractMemoryUnit,
}

#[inline]
fn native_write_to_event_store(
    gas_params: &WriteToEventStoreGasParameters,
    context: &mut NativeContext,
    mut ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    debug_assert!(ty_args.len() == 1);
    debug_assert!(arguments.len() == 3);

    let ty = ty_args.pop().unwrap();
    let msg = arguments.pop_back().unwrap();
    let seq_num = pop_arg!(arguments, u64);
    let guid = pop_arg!(arguments, Vec<u8>);

    // TODO(Gas): fix this
    let cost = gas_params.base_cost + gas_params.unit_cost * msg.size();

    if !context.save_event(guid, seq_num, ty, msg)? {
        return Ok(NativeResult::err(cost, 0));
    }

    Ok(NativeResult::ok(cost, smallvec![]))
}

pub fn make_native_write_to_event_store(
    gas_params: WriteToEventStoreGasParameters,
) -> NativeFunction {
    Arc::new(
        move |context, ty_args, args| -> PartialVMResult<NativeResult> {
            native_write_to_event_store(&gas_params, context, ty_args, args)
        },
    )
}

/***************************************************************************************************
 * module
 *
 **************************************************************************************************/
#[derive(Debug, Clone)]
pub struct GasParameters {
    pub write_to_event_store: WriteToEventStoreGasParameters,
}

pub fn make_all(gas_params: GasParameters) -> impl Iterator<Item = (String, NativeFunction)> {
    let natives = [(
        "write_to_event_store",
        make_native_write_to_event_store(gas_params.write_to_event_store),
    )];

    make_module_natives(natives)
}
