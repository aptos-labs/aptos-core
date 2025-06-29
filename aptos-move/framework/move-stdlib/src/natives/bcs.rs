// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use aptos_gas_schedule::gas_params::natives::move_stdlib::*;
use aptos_native_interface::{
    safely_pop_arg, RawSafeNative, SafeNativeBuilder, SafeNativeContext, SafeNativeError,
    SafeNativeResult,
};
use move_core_types::{
    account_address::AccountAddress,
    gas_algebra::{NumBytes, NumTypeNodes},
    u256,
    value::{MoveStructLayout, MoveTypeLayout},
    vm_status::{sub_status::NFE_BCS_SERIALIZATION_FAILURE, StatusCode},
};
use move_vm_runtime::native_functions::NativeFunction;
use move_vm_types::{
    loaded_data::runtime_types::Type,
    natives::function::{PartialVMError, PartialVMResult},
    value_serde::ValueSerDeContext,
    values::{values_impl::Reference, Struct, Value},
};
use smallvec::{smallvec, SmallVec};
use std::collections::VecDeque;

pub fn create_option_u64(value: Option<u64>) -> Value {
    Value::struct_(Struct::pack(vec![Value::vector_u64(value)]))
}

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
/// Rust implementation of Move's `native public fun to_bytes<T>(&T): vector<u8>`
#[inline]
fn native_to_bytes(
    context: &mut SafeNativeContext,
    mut ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert!(ty_args.len() == 1);
    debug_assert!(args.len() == 1);

    let ref_to_val = safely_pop_arg!(args, Reference);
    let arg_type = ty_args.pop().unwrap();

    let layout = if context.get_feature_flags().is_lazy_loading_enabled() {
        // With lazy loading, propagate the error directly. This is because errors here are likely
        // from metering, so we should not remap them in any way. Note that makes it possible to
        // fail on constructing a very deep / large layout and not be charged, but this is already
        // the case for regular execution, so we keep it simple. Also, charging more gas after
        // out-of-gas failure in layout construction does not make any sense.
        context.type_to_type_layout_with_delayed_fields(&arg_type)?
    } else {
        match context.type_to_type_layout_with_delayed_fields(&arg_type) {
            Ok(layout) => layout,
            Err(_) => {
                context.charge(BCS_TO_BYTES_FAILURE)?;
                return Err(SafeNativeError::Abort {
                    abort_code: NFE_BCS_SERIALIZATION_FAILURE,
                });
            },
        }
    };

    // TODO(#14175): Reading the reference performs a deep copy, and we can
    //               implement it in a more efficient way.
    let val = ref_to_val.read_ref()?;

    let function_value_extension = context.function_value_extension();
    let serialized_value = match ValueSerDeContext::new()
        .with_legacy_signer()
        .with_func_args_deserialization(&function_value_extension)
        .serialize(&val, &layout.unpack().0)?
    {
        Some(serialized_value) => serialized_value,
        None => {
            context.charge(BCS_TO_BYTES_FAILURE)?;
            return Err(SafeNativeError::Abort {
                abort_code: NFE_BCS_SERIALIZATION_FAILURE,
            });
        },
    };
    context
        .charge(BCS_TO_BYTES_PER_BYTE_SERIALIZED * NumBytes::new(serialized_value.len() as u64))?;

    Ok(smallvec![Value::vector_u8(serialized_value)])
}

/***************************************************************************************************
 * native fun serialized_size
 *
 *   gas cost: size_of(output)
 *
 *   If the getting the type layout or serialization results in error, a special failure
 *   cost is charged.
 *
 **************************************************************************************************/
fn native_serialized_size(
    context: &mut SafeNativeContext,
    mut ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert!(ty_args.len() == 1);
    debug_assert!(args.len() == 1);

    context.charge(BCS_SERIALIZED_SIZE_BASE)?;

    let reference = safely_pop_arg!(args, Reference);
    let ty = ty_args.pop().unwrap();

    let serialized_size = match serialized_size_impl(context, reference, &ty) {
        Ok(serialized_size) => serialized_size as u64,
        Err(_) => {
            context.charge(BCS_SERIALIZED_SIZE_FAILURE)?;

            // Re-use the same abort code as bcs::to_bytes.
            return Err(SafeNativeError::Abort {
                abort_code: NFE_BCS_SERIALIZATION_FAILURE,
            });
        },
    };
    context.charge(BCS_SERIALIZED_SIZE_PER_BYTE_SERIALIZED * NumBytes::new(serialized_size))?;

    Ok(smallvec![Value::u64(serialized_size)])
}

fn serialized_size_impl(
    context: &mut SafeNativeContext,
    reference: Reference,
    ty: &Type,
) -> PartialVMResult<usize> {
    // TODO(#14175): Reading the reference performs a deep copy, and we can
    //               implement it in a more efficient way.
    let value = reference.read_ref()?;
    let ty_layout = context
        .type_to_type_layout_with_delayed_fields(ty)?
        .unpack()
        .0;

    let function_value_extension = context.function_value_extension();
    ValueSerDeContext::new()
        .with_legacy_signer()
        .with_func_args_deserialization(&function_value_extension)
        .with_delayed_fields_serde()
        .serialized_size(&value, &ty_layout)
}

fn native_constant_serialized_size(
    context: &mut SafeNativeContext,
    mut ty_args: Vec<Type>,
    _args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert!(ty_args.len() == 1);

    context.charge(BCS_CONSTANT_SERIALIZED_SIZE_BASE)?;

    let ty = ty_args.pop().unwrap();
    let ty_layout = context
        .type_to_type_layout_with_delayed_fields(&ty)?
        .unpack()
        .0;

    let (visited_count, serialized_size_result) = constant_serialized_size(&ty_layout);
    context
        .charge(BCS_CONSTANT_SERIALIZED_SIZE_PER_TYPE_NODE * NumTypeNodes::new(visited_count))?;

    let result = match serialized_size_result {
        Ok(value) => create_option_u64(value.map(|v| v as u64)),
        Err(_) => {
            context.charge(BCS_SERIALIZED_SIZE_FAILURE)?;

            // Re-use the same abort code as bcs::to_bytes.
            return Err(SafeNativeError::Abort {
                abort_code: NFE_BCS_SERIALIZATION_FAILURE,
            });
        },
    };

    Ok(smallvec![result])
}

/// If given type has a constant serialized size (irrespective of the instance), it returns the serialized
/// size in bytes any value would have.
/// Otherwise it returns None.
/// First element of the returned tuple represents number of visited nodes, used to charge gas.
fn constant_serialized_size(ty_layout: &MoveTypeLayout) -> (u64, PartialVMResult<Option<usize>>) {
    let mut visited_count = 1;
    let bcs_size_result = match ty_layout {
        MoveTypeLayout::Bool => bcs::serialized_size(&false).map(Some),
        MoveTypeLayout::U8 => bcs::serialized_size(&0u8).map(Some),
        MoveTypeLayout::U16 => bcs::serialized_size(&0u16).map(Some),
        MoveTypeLayout::U32 => bcs::serialized_size(&0u32).map(Some),
        MoveTypeLayout::U64 => bcs::serialized_size(&0u64).map(Some),
        MoveTypeLayout::U128 => bcs::serialized_size(&0u128).map(Some),
        MoveTypeLayout::U256 => bcs::serialized_size(&u256::U256::zero()).map(Some),
        MoveTypeLayout::Address => bcs::serialized_size(&AccountAddress::ZERO).map(Some),
        // signer's size is VM implementation detail, and can change at will.
        MoveTypeLayout::Signer => Ok(None),
        // vectors have no constant size
        MoveTypeLayout::Vector(_) => Ok(None),
        // enums and functions have no constant size
        MoveTypeLayout::Struct(
            MoveStructLayout::RuntimeVariants(_) | MoveStructLayout::WithVariants(_),
        )
        | MoveTypeLayout::Function => Ok(None),
        MoveTypeLayout::Struct(MoveStructLayout::Runtime(fields)) => {
            let mut total = Some(0);
            for field in fields {
                let (cur_visited_count, cur) = constant_serialized_size(field);
                visited_count += cur_visited_count;
                match cur {
                    Err(e) => return (visited_count, Err(e)),
                    Ok(Some(cur_value)) => total = total.map(|v| v + cur_value),
                    Ok(None) => {
                        total = None;
                        break;
                    },
                }
            }
            Ok(total)
        },
        MoveTypeLayout::Struct(MoveStructLayout::WithFields(_))
        | MoveTypeLayout::Struct(MoveStructLayout::WithTypes { .. }) => {
            return (
                visited_count,
                Err(
                    PartialVMError::new(StatusCode::VALUE_SERIALIZATION_ERROR).with_message(
                        "Only runtime types expected, but found WithFields/WithTypes".to_string(),
                    ),
                ),
            )
        },
        MoveTypeLayout::Native(_, inner) => {
            let (cur_visited_count, cur) = constant_serialized_size(inner);
            visited_count += cur_visited_count;
            match cur {
                Err(e) => return (visited_count, Err(e)),
                Ok(v) => Ok(v),
            }
        },
    };
    (
        visited_count,
        bcs_size_result.map_err(|e| {
            PartialVMError::new(StatusCode::VALUE_SERIALIZATION_ERROR).with_message(format!(
                "failed to compute serialized size of a value: {:?}",
                e
            ))
        }),
    )
}

/***************************************************************************************************
 * module
 **************************************************************************************************/
pub fn make_all(
    builder: &SafeNativeBuilder,
) -> impl Iterator<Item = (String, NativeFunction)> + '_ {
    let funcs = [
        ("to_bytes", native_to_bytes as RawSafeNative),
        ("serialized_size", native_serialized_size),
        ("constant_serialized_size", native_constant_serialized_size),
    ];

    builder.make_named_natives(funcs)
}
