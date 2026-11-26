// Parts of the file are Copyright (c) The Diem Core Contributors
// Parts of the file are Copyright (c) The Move Contributors
// Parts of the file are Copyright (c) Aptos Foundation
// All Aptos Foundation code and content is licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::natives::helpers::make_module_natives;
use move_binary_format::errors::PartialVMResult;
use move_core_types::{
    gas_algebra::{InternalGas, InternalGasPerByte, NumBytes},
    vm_status::sub_status::NFE_BCS_SERIALIZATION_FAILURE,
};
use move_vm_runtime::native_functions::{NativeContext, NativeFunction};
use move_vm_types::{
    loaded_data::runtime_types::Type,
    natives::function::NativeResult,
    pop_arg,
    value_serde::{FunctionValueExtension, ValueSerDeContext},
    values::{values_impl::Reference, Value},
};
use smallvec::smallvec;
use std::{collections::VecDeque, sync::Arc};
/***************************************************************************************************
 * native fun to_bytes
 *
 *   gas cost: size_of(val_type) * input_unit_cost +        | get type layout
 *             size_of(val) * input_unit_cost +             | serialize value
 *             max(size_of(output), 1) * output_unit_cost
 *
 *             If any of the first two steps fails, a partial cost + an additional failure_cost
 *             will be charged.
 *
 **************************************************************************************************/
#[derive(Debug, Clone)]
pub struct ToBytesGasParameters {
    pub per_byte_serialized: InternalGasPerByte,
    pub legacy_min_output_size: NumBytes,
    pub failure: InternalGas,
}

/// Rust implementation of Move's `native public fun to_bytes<T>(&T): vector<u8>`
#[inline]
fn native_to_bytes(
    gas_params: &ToBytesGasParameters,
    context: &mut NativeContext,
    ty_args: &[Type],
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    debug_assert!(ty_args.len() == 1);
    debug_assert!(args.len() == 1);

    let mut cost = 0.into();

    // pop type and value
    let ref_to_val = pop_arg!(args, Reference);
    let arg_type = &ty_args[0];

    // get type layout
    let layout = match context.type_to_type_layout(arg_type) {
        Ok(layout) => layout,
        Err(_) => {
            cost += gas_params.failure;
            return Ok(NativeResult::err(cost, NFE_BCS_SERIALIZATION_FAILURE));
        },
    };
    // serialize value
    let val = ref_to_val.read_ref()?;

    let function_value_extension = context.function_value_extension();
    let max_value_nest_depth = function_value_extension.max_value_nest_depth();
    let serialized_value = match ValueSerDeContext::new(max_value_nest_depth)
        .with_legacy_signer()
        .with_func_args_deserialization(&function_value_extension)
        .serialize(&val, &layout)?
    {
        Some(serialized_value) => serialized_value,
        None => {
            cost += gas_params.failure;
            return Ok(NativeResult::err(cost, NFE_BCS_SERIALIZATION_FAILURE));
        },
    };
    cost += gas_params.per_byte_serialized
        * std::cmp::max(
            NumBytes::new(serialized_value.len() as u64),
            gas_params.legacy_min_output_size,
        );

    Ok(NativeResult::ok(cost, smallvec![Value::vector_u8(
        serialized_value
    )]))
}

pub fn make_native_to_bytes(gas_params: ToBytesGasParameters) -> NativeFunction {
    Arc::new(
        move |context, ty_args, args| -> PartialVMResult<NativeResult> {
            native_to_bytes(&gas_params, context, ty_args, args)
        },
    )
}

/***************************************************************************************************
 * native fun serialized_size
 *
 *   gas cost: base + serialized value size in bytes * per_byte_serialized
 *
 *             Type-layout or serialized-size failures charge base + failure.
 *
 **************************************************************************************************/
#[derive(Debug, Clone)]
pub struct SerializedSizeGasParameters {
    pub base: InternalGas,
    pub per_byte_serialized: InternalGasPerByte,
    pub failure: InternalGas,
}

/// Rust implementation of Move's `native public fun serialized_size<T>(&T): u64`
#[inline]
fn native_serialized_size(
    gas_params: &SerializedSizeGasParameters,
    context: &mut NativeContext,
    ty_args: &[Type],
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    debug_assert!(ty_args.len() == 1);
    debug_assert!(args.len() == 1);

    let mut cost = gas_params.base;
    let ref_to_val = pop_arg!(args, Reference);
    let layout = match context.type_to_type_layout(&ty_args[0]) {
        Ok(layout) => layout,
        Err(_) => {
            cost += gas_params.failure;
            return Ok(NativeResult::err(cost, NFE_BCS_SERIALIZATION_FAILURE));
        },
    };
    let val = ref_to_val.read_ref()?;

    let function_value_extension = context.function_value_extension();
    let max_value_nest_depth = function_value_extension.max_value_nest_depth();
    let serialized_size = match ValueSerDeContext::new(max_value_nest_depth)
        .with_legacy_signer()
        .with_func_args_deserialization(&function_value_extension)
        .serialized_size(&val, &layout)
    {
        Ok(serialized_size) => serialized_size as u64,
        Err(_) => {
            cost += gas_params.failure;
            return Ok(NativeResult::err(cost, NFE_BCS_SERIALIZATION_FAILURE));
        },
    };
    cost += gas_params.per_byte_serialized * NumBytes::new(serialized_size);

    Ok(NativeResult::ok(cost, smallvec![Value::u64(
        serialized_size
    )]))
}

pub fn make_native_serialized_size(gas_params: SerializedSizeGasParameters) -> NativeFunction {
    Arc::new(
        move |context, ty_args, args| -> PartialVMResult<NativeResult> {
            native_serialized_size(&gas_params, context, ty_args, args)
        },
    )
}

/***************************************************************************************************
 * module
 **************************************************************************************************/
#[derive(Debug, Clone)]
pub struct GasParameters {
    pub to_bytes: ToBytesGasParameters,
    pub serialized_size: SerializedSizeGasParameters,
}

pub fn make_all(gas_params: GasParameters) -> impl Iterator<Item = (String, NativeFunction)> {
    let natives = [
        ("to_bytes", make_native_to_bytes(gas_params.to_bytes)),
        (
            "serialized_size",
            make_native_serialized_size(gas_params.serialized_size),
        ),
    ];

    make_module_natives(natives)
}
