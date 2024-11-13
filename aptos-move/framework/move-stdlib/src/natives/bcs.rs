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
    value_serde::serialized_size_allowing_delayed_values,
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

    let layout = match context.type_to_type_layout(&arg_type) {
        Ok(layout) => layout,
        Err(_) => {
            context.charge(BCS_TO_BYTES_FAILURE)?;
            return Err(SafeNativeError::Abort {
                abort_code: NFE_BCS_SERIALIZATION_FAILURE,
            });
        },
    };

    // TODO(#14175): Reading the reference performs a deep copy, and we can
    //               implement it in a more efficient way.
    let val = ref_to_val.read_ref()?;

    let serialized_value = match val.simple_serialize(&layout) {
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
    let ty_layout = context.type_to_type_layout(ty)?;
    serialized_size_allowing_delayed_values(&value, &ty_layout)
}

fn native_constant_serialized_size(
    context: &mut SafeNativeContext,
    mut ty_args: Vec<Type>,
    _args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert!(ty_args.len() == 1);

    context.charge(BCS_CONSTANT_SERIALIZED_SIZE_BASE)?;

    let ty = ty_args.pop().unwrap();
    let ty_layout = context.type_to_type_layout(&ty)?;

    context.charge(
        BCS_CONSTANT_SERIALIZED_SIZE_PER_TYPE_NODE
            * NumTypeNodes::new(type_visit_count_for_constant_serialized_size(&ty_layout)),
    )?;

    let result = match constant_serialized_size(&ty_layout) {
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

/// Count upper limit on the number of types constant_serialized_size would visit,
/// which is then used for gas charging, before performing the operation.
/// This is different done type.num_nodes(), as some types are not traversed (i.e. vector),
/// and for structs types and number of fields matter as well.
///
/// Unclear if type_visit_count would be the same for other usages
/// (for example, whether vector types need to be traversed),
/// so name it very specifically, and on future usages see how it generalizes.
fn type_visit_count_for_constant_serialized_size(ty_layout: &MoveTypeLayout) -> u64 {
    match ty_layout {
        MoveTypeLayout::Bool
        | MoveTypeLayout::U8
        | MoveTypeLayout::U16
        | MoveTypeLayout::U32
        | MoveTypeLayout::U128
        | MoveTypeLayout::U256
        | MoveTypeLayout::U64
        | MoveTypeLayout::Address
        | MoveTypeLayout::Signer => 1,
        // non-recursed:
        MoveTypeLayout::Struct(
            MoveStructLayout::RuntimeVariants(_) | MoveStructLayout::WithVariants(_),
        )
        | MoveTypeLayout::Vector(_) => 1,
        // recursed:
        MoveTypeLayout::Struct(MoveStructLayout::Runtime(fields)) => {
            let mut total = 1; // Count the current visit, and aggregate all children
            for field in fields {
                total += type_visit_count_for_constant_serialized_size(field);
            }
            total
        },
        MoveTypeLayout::Struct(MoveStructLayout::WithFields(fields))
        | MoveTypeLayout::Struct(MoveStructLayout::WithTypes { fields, .. }) => {
            let mut total = 1; // Count the current visit, and aggregate all children
            for field in fields {
                total += type_visit_count_for_constant_serialized_size(&field.layout);
            }
            total
        },
        // Count the current visit, and inner visits
        MoveTypeLayout::Native(_, inner) => {
            1 + type_visit_count_for_constant_serialized_size(inner)
        },
    }
}

/// If given type has a constant serialized size (irrespective of the instance), it returns the serialized
/// size in bytes any value would have.
/// Otherwise it returns None.
fn constant_serialized_size(ty_layout: &MoveTypeLayout) -> PartialVMResult<Option<usize>> {
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
        // enums have no constant size
        MoveTypeLayout::Struct(
            MoveStructLayout::RuntimeVariants(_) | MoveStructLayout::WithVariants(_),
        ) => Ok(None),
        MoveTypeLayout::Struct(MoveStructLayout::Runtime(fields)) => {
            let mut total = Some(0);
            for field in fields {
                let cur = constant_serialized_size(field)?;
                match cur {
                    Some(cur_value) => total = total.map(|v| v + cur_value),
                    None => {
                        total = None;
                        break;
                    },
                }
            }
            Ok(total)
        },
        MoveTypeLayout::Struct(MoveStructLayout::WithFields(_))
        | MoveTypeLayout::Struct(MoveStructLayout::WithTypes { .. }) => {
            return Err(
                PartialVMError::new(StatusCode::VALUE_SERIALIZATION_ERROR).with_message(
                    "Only runtime types expected, but found WithFields/WithTypes".to_string(),
                ),
            )
        },
        MoveTypeLayout::Native(_, inner) => Ok(constant_serialized_size(inner)?),
    };
    bcs_size_result.map_err(|e| {
        PartialVMError::new(StatusCode::VALUE_SERIALIZATION_ERROR).with_message(format!(
            "failed to compute serialized size of a value: {:?}",
            e
        ))
    })
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
