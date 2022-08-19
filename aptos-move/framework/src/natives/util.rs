// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use move_deps::{
    move_binary_format::errors::{PartialVMError, PartialVMResult},
    move_core_types::{
        gas_algebra::{InternalGas, InternalGasPerByte, NumBytes},
        vm_status::StatusCode,
    },
    move_vm_runtime::native_functions::{NativeContext, NativeFunction},
    move_vm_types::{
        loaded_data::runtime_types::Type, natives::function::NativeResult, pop_arg, values::Value,
    },
};
use smallvec::smallvec;
use std::{collections::VecDeque, sync::Arc};

/// Abort code when from_bytes fails (0x01 == INVALID_ARGUMENT)
const EFROM_BYTES: u64 = 0x01_0001;

/// Used to pass gas parameters into native functions.
pub fn make_native_from_func<T: std::marker::Send + std::marker::Sync + 'static>(
    gas_params: T,
    func: fn(&T, &mut NativeContext, Vec<Type>, VecDeque<Value>) -> PartialVMResult<NativeResult>,
) -> NativeFunction {
    Arc::new(move |context, ty_args, args| func(&gas_params, context, ty_args, args))
}

/// Used to pop a Vec<Vec<u8>> argument off the stack.
#[macro_export]
macro_rules! pop_vec_arg {
    ($arguments:ident, $t:ty) => {{
        // Replicating the code from pop_arg! here
        use move_deps::move_vm_types::natives::function::{PartialVMError, StatusCode};
        let value_vec = match $arguments.pop_back().map(|v| v.value_as::<Vec<Value>>()) {
            None => {
                return Err(PartialVMError::new(
                    StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
                ))
            }
            Some(Err(e)) => return Err(e),
            Some(Ok(v)) => v,
        };

        // Pop each Value from the popped Vec<Value>, cast it as a Vec<u8>, and push it to a Vec<Vec<u8>>
        let mut vec_vec = vec![];
        for value in value_vec {
            let vec = match value.value_as::<$t>() {
                Err(e) => return Err(e),
                Ok(v) => v,
            };
            vec_vec.push(vec);
        }

        vec_vec
    }};
}

/***************************************************************************************************
 * native fun from_bytes
 *
 *   gas cost: base_cost + unit_cost * bytes_len
 *
 **************************************************************************************************/
#[derive(Debug, Clone)]
pub struct FromBytesGasParameters {
    pub base: InternalGas,
    pub per_byte: InternalGasPerByte,
}

fn native_from_bytes(
    gas_params: &FromBytesGasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    debug_assert_eq!(ty_args.len(), 1);
    debug_assert_eq!(args.len(), 1);

    // TODO(Gas): charge for getting the layout
    let layout = context.type_to_type_layout(&ty_args[0])?.ok_or_else(|| {
        PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR).with_message(format!(
            "Failed to get layout of type {:?} -- this should not happen",
            ty_args[0]
        ))
    })?;

    let bytes = pop_arg!(args, Vec<u8>);
    let cost = gas_params.base + gas_params.per_byte * NumBytes::new(bytes.len() as u64);
    let val = match Value::simple_deserialize(&bytes, &layout) {
        Some(val) => val,
        None => return Ok(NativeResult::err(cost, EFROM_BYTES)),
    };

    Ok(NativeResult::ok(cost, smallvec![val]))
}

pub fn make_native_from_bytes(gas_params: FromBytesGasParameters) -> NativeFunction {
    Arc::new(move |context, ty_args, args| native_from_bytes(&gas_params, context, ty_args, args))
}

/***************************************************************************************************
 * module
 *
 **************************************************************************************************/
#[derive(Debug, Clone)]
pub struct GasParameters {
    pub from_bytes: FromBytesGasParameters,
}

pub fn make_all(gas_params: GasParameters) -> impl Iterator<Item = (String, NativeFunction)> {
    let natives = [("from_bytes", make_native_from_bytes(gas_params.from_bytes))];

    crate::natives::helpers::make_module_natives(natives)
}
