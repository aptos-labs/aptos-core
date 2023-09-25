// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::natives::helpers::make_module_natives;
use move_binary_format::errors::PartialVMResult;
use move_core_types::gas_algebra::InternalGasPerAbstractMemoryUnit;
use move_vm_runtime::native_functions::{NativeContext, NativeFunction};
use move_vm_types::{
    loaded_data::runtime_types::Type, natives::function::NativeResult, values::Value,
    views::ValueView,
};
use smallvec::smallvec;
use std::{collections::VecDeque, sync::Arc};

/***************************************************************************************************
 * [NURSERY-ONLY] native fun write_to_event_store
 *
 *   gas cost: base_cost
 *
 **************************************************************************************************/
#[derive(Debug, Clone)]
pub struct WriteToEventStoreGasParameters {
    pub unit_cost: InternalGasPerAbstractMemoryUnit,
}

#[inline]
fn native_write_to_event_store(
    gas_params: &WriteToEventStoreGasParameters,
    _context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    debug_assert!(ty_args.len() == 1);
    debug_assert!(arguments.len() == 3);

    let msg = arguments.pop_back().unwrap();
    let cost = gas_params.unit_cost * std::cmp::max(msg.legacy_abstract_memory_size(), 1.into());

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
