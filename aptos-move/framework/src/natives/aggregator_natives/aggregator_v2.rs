// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::natives::aggregator_natives::{helpers_v2::*, NativeAggregatorContext};
use aptos_aggregator::{
    bounded_math::{BoundedMath, SignedU128},
    delayed_field_extension::DelayedFieldData,
    resolver::DelayedFieldResolver,
};
use aptos_gas_algebra::NumBytes;
use aptos_gas_schedule::gas_params::natives::aptos_framework::*;
use aptos_native_interface::{
    safely_pop_arg, RawSafeNative, SafeNativeBuilder, SafeNativeContext, SafeNativeError,
    SafeNativeResult,
};
use aptos_types::{
    delayed_fields::{
        calculate_width_for_constant_string, calculate_width_for_integer_embedded_string,
        SnapshotToStringFormula,
    },
    error::code_invariant_error,
};
use move_binary_format::errors::PartialVMError;
use move_vm_runtime::native_functions::NativeFunction;
use move_vm_types::{
    delayed_values::{
        delayed_field_id::DelayedFieldID,
        derived_string_snapshot::{
            bytes_and_width_to_derived_string_struct, string_to_bytes, u128_to_u64,
        },
    },
    loaded_data::runtime_types::Type,
    values::{Reference, Struct, StructRef, Value},
};
use smallvec::{smallvec, SmallVec};
use std::{cell::RefMut, collections::VecDeque};

/// The generic type supplied to aggregator snapshots is not supported.
pub const EUNSUPPORTED_AGGREGATOR_SNAPSHOT_TYPE: u64 = 0x03_0005;

/// The generic type supplied to the aggregators is not supported.
pub const EUNSUPPORTED_AGGREGATOR_TYPE: u64 = 0x03_0007;

/// Arguments passed to concat or create_snapshot exceed max limit of
/// STRING_SNAPSHOT_INPUT_MAX_LENGTH bytes (for prefix and suffix together).
pub const EINPUT_STRING_LENGTH_TOO_LARGE: u64 = 0x03_0008;

/// The native aggregator function, that is in the move file, is not yet supported.
/// and any calls will raise this error.
pub const EAGGREGATOR_FUNCTION_NOT_YET_SUPPORTED: u64 = 0x03_0009;

/// The maximum length of the input string for derived string snapshot.
/// If we want to increase this, we need to modify BITS_FOR_SIZE in types/src/delayed_fields.rs.
pub const DERIVED_STRING_INPUT_MAX_LENGTH: usize = 1024;

fn get_width_by_type(ty_arg: &Type, error_code_if_incorrect: u64) -> SafeNativeResult<u32> {
    match ty_arg {
        Type::U128 => Ok(16),
        Type::U64 => Ok(8),
        _ => Err(SafeNativeError::Abort {
            abort_code: error_code_if_incorrect,
        }),
    }
}

/// Given the list of native function arguments and a type, pop the next argument if it is of given type.
fn pop_value_by_type(
    ty_arg: &Type,
    args: &mut VecDeque<Value>,
    error_code_if_incorrect: u64,
) -> SafeNativeResult<u128> {
    match ty_arg {
        Type::U128 => Ok(safely_pop_arg!(args, u128)),
        Type::U64 => Ok(safely_pop_arg!(args, u64) as u128),
        _ => Err(SafeNativeError::Abort {
            abort_code: error_code_if_incorrect,
        }),
    }
}

fn create_value_by_type(
    value_ty: &Type,
    value: u128,
    error_code_if_incorrect: u64,
) -> SafeNativeResult<Value> {
    match value_ty {
        Type::U128 => Ok(Value::u128(value)),
        Type::U64 => Ok(Value::u64(u128_to_u64(value)?)),
        _ => Err(SafeNativeError::Abort {
            abort_code: error_code_if_incorrect,
        }),
    }
}

fn create_string_value(value: Vec<u8>) -> Value {
    Value::struct_(Struct::pack(vec![Value::vector_u8(value)]))
}

fn get_context_data<'t, 'b>(
    context: &'t mut SafeNativeContext<'_, 'b, '_, '_>,
) -> Option<(&'b dyn DelayedFieldResolver, RefMut<'t, DelayedFieldData>)> {
    let aggregator_context = context.extensions().get::<NativeAggregatorContext>();
    if aggregator_context.delayed_field_optimization_enabled {
        Some((
            aggregator_context.delayed_field_resolver,
            aggregator_context.delayed_field_data.borrow_mut(),
        ))
    } else {
        None
    }
}

fn create_aggregator_with_max_value(
    context: &mut SafeNativeContext,
    aggregator_value_ty: &Type,
    max_value: u128,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    let value = if let Some((resolver, mut delayed_field_data)) = get_context_data(context) {
        let width = get_width_by_type(aggregator_value_ty, EUNSUPPORTED_AGGREGATOR_TYPE)?;
        let id = resolver.generate_delayed_field_id(width);
        delayed_field_data.create_new_aggregator(id);
        Value::delayed_value(id)
    } else {
        create_value_by_type(aggregator_value_ty, 0, EUNSUPPORTED_AGGREGATOR_TYPE)?
    };

    let max_value =
        create_value_by_type(aggregator_value_ty, max_value, EUNSUPPORTED_AGGREGATOR_TYPE)?;
    Ok(smallvec![Value::struct_(Struct::pack(vec![
        value, max_value,
    ]))])
}

/***************************************************************************************************
 * native fun create_aggregator<IntElement>(max_value: IntElement): Aggregator<IntElement>;
 **************************************************************************************************/

fn native_create_aggregator(
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert_eq!(args.len(), 1);
    debug_assert_eq!(ty_args.len(), 1);
    context.charge(AGGREGATOR_V2_CREATE_AGGREGATOR_BASE)?;

    let max_value = pop_value_by_type(&ty_args[0], &mut args, EUNSUPPORTED_AGGREGATOR_TYPE)?;
    create_aggregator_with_max_value(context, &ty_args[0], max_value)
}

/***************************************************************************************************
 * native fun create_unbounded_aggregator<IntElement: copy + drop>(): Aggregator<IntElement>;
 **************************************************************************************************/

fn native_create_unbounded_aggregator(
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert_eq!(args.len(), 0);
    debug_assert_eq!(ty_args.len(), 1);
    context.charge(AGGREGATOR_V2_CREATE_AGGREGATOR_BASE)?;

    let max_value = unbounded_aggregator_max_value(&ty_args[0])?;
    create_aggregator_with_max_value(context, &ty_args[0], max_value)
}

/***************************************************************************************************
 * native fun try_add<IntElement>(aggregator: &mut Aggregator<IntElement>, value: IntElement): bool;
 **************************************************************************************************/
fn native_try_add(
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert_eq!(args.len(), 2);
    debug_assert_eq!(ty_args.len(), 1);
    context.charge(AGGREGATOR_V2_TRY_ADD_BASE)?;

    let aggregator_value_ty = &ty_args[0];
    let rhs = pop_value_by_type(aggregator_value_ty, &mut args, EUNSUPPORTED_AGGREGATOR_TYPE)?;
    let aggregator = safely_pop_arg!(args, StructRef);

    let max_value = get_aggregator_max_value(&aggregator, aggregator_value_ty)?;

    let success = if let Some((resolver, mut delayed_field_data)) = get_context_data(context) {
        let id = get_aggregator_value_as_id(&aggregator, aggregator_value_ty, resolver)?;
        delayed_field_data.try_add_or_check_delta(
            id,
            max_value,
            SignedU128::Positive(rhs),
            resolver,
            true,
        )?
    } else {
        let lhs = get_aggregator_value(&aggregator, aggregator_value_ty)?;
        match BoundedMath::new(max_value).unsigned_add(lhs, rhs) {
            Ok(result) => {
                let new_value = create_value_by_type(
                    aggregator_value_ty,
                    result,
                    EUNSUPPORTED_AGGREGATOR_TYPE,
                )?;
                set_aggregator_value(&aggregator, new_value)?;
                true
            },
            Err(_) => false,
        }
    };

    Ok(smallvec![Value::bool(success)])
}

/***************************************************************************************************
 * native fun try_sub<IntElement>(aggregator: &mut Aggregator<IntElement>, value: IntElement): bool;
 **************************************************************************************************/
fn native_try_sub(
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert_eq!(args.len(), 2);
    debug_assert_eq!(ty_args.len(), 1);
    context.charge(AGGREGATOR_V2_TRY_SUB_BASE)?;

    let aggregator_value_ty = &ty_args[0];
    let rhs = pop_value_by_type(aggregator_value_ty, &mut args, EUNSUPPORTED_AGGREGATOR_TYPE)?;
    let aggregator = safely_pop_arg!(args, StructRef);

    let max_value = get_aggregator_max_value(&aggregator, aggregator_value_ty)?;

    let success = if let Some((resolver, mut delayed_field_data)) = get_context_data(context) {
        let id = get_aggregator_value_as_id(&aggregator, aggregator_value_ty, resolver)?;
        delayed_field_data.try_add_or_check_delta(
            id,
            max_value,
            SignedU128::Negative(rhs),
            resolver,
            true,
        )?
    } else {
        let lhs = get_aggregator_value(&aggregator, aggregator_value_ty)?;
        match BoundedMath::new(max_value).unsigned_subtract(lhs, rhs) {
            Ok(result) => {
                let new_value = create_value_by_type(
                    aggregator_value_ty,
                    result,
                    EUNSUPPORTED_AGGREGATOR_TYPE,
                )?;
                set_aggregator_value(&aggregator, new_value)?;
                true
            },
            Err(_) => false,
        }
    };

    Ok(smallvec![Value::bool(success)])
}

fn native_is_at_least_impl(
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert_eq!(args.len(), 2);
    debug_assert_eq!(ty_args.len(), 1);
    context.charge(AGGREGATOR_V2_IS_AT_LEAST_BASE)?;

    let aggregator_value_ty = &ty_args[0];
    let rhs = pop_value_by_type(aggregator_value_ty, &mut args, EUNSUPPORTED_AGGREGATOR_TYPE)?;
    let aggregator = safely_pop_arg!(args, StructRef);

    let max_value = get_aggregator_max_value(&aggregator, aggregator_value_ty)?;

    let success = if let Some((resolver, mut delayed_field_data)) = get_context_data(context) {
        let id = get_aggregator_value_as_id(&aggregator, aggregator_value_ty, resolver)?;
        delayed_field_data.try_add_or_check_delta(
            id,
            max_value,
            SignedU128::Negative(rhs),
            resolver,
            false,
        )?
    } else {
        let lhs = get_aggregator_value(&aggregator, aggregator_value_ty)?;
        BoundedMath::new(max_value)
            .unsigned_subtract(lhs, rhs)
            .is_ok()
    };

    Ok(smallvec![Value::bool(success)])
}

/***************************************************************************************************
 * native fun read<IntElement>(aggregator: &Aggregator<IntElement>): IntElement;
 **************************************************************************************************/

fn native_read(
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert_eq!(args.len(), 1);
    debug_assert_eq!(ty_args.len(), 1);
    context.charge(AGGREGATOR_V2_READ_BASE)?;

    let aggregator_value_ty = &ty_args[0];
    let aggregator = safely_pop_arg!(args, StructRef);

    let value = if let Some((resolver, delayed_field_data)) = get_context_data(context) {
        let id = get_aggregator_value_as_id(&aggregator, aggregator_value_ty, resolver)?;
        delayed_field_data.read_aggregator(id, resolver)?
    } else {
        get_aggregator_value(&aggregator, aggregator_value_ty)?
    };

    // Paranoid check to make sure read result makes sense.
    let max_value = get_aggregator_max_value(&aggregator, aggregator_value_ty)?;
    if value > max_value {
        let error = code_invariant_error(format!("Aggregator read returned the value greater than maximum possible value: {value} > {max_value}"));
        return Err(SafeNativeError::InvariantViolation(PartialVMError::from(
            error,
        )));
    };

    let value = create_value_by_type(aggregator_value_ty, value, EUNSUPPORTED_AGGREGATOR_TYPE)?;
    Ok(smallvec![value])
}

/***************************************************************************************************
 * native fun snapshot<IntElement>(aggregator: &Aggregator<IntElement>): AggregatorSnapshot<IntElement>;
 **************************************************************************************************/

fn native_snapshot(
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert_eq!(args.len(), 1);
    debug_assert_eq!(ty_args.len(), 1);
    context.charge(AGGREGATOR_V2_SNAPSHOT_BASE)?;

    let aggregator_value_ty = &ty_args[0];
    let aggregator = safely_pop_arg!(args, StructRef);

    let result_value = if let Some((resolver, mut delayed_field_data)) = get_context_data(context) {
        let width = get_width_by_type(aggregator_value_ty, EUNSUPPORTED_AGGREGATOR_TYPE)?;
        let id = get_aggregator_value_as_id(&aggregator, aggregator_value_ty, resolver)?;
        let max_value = get_aggregator_max_value(&aggregator, aggregator_value_ty)?;

        let snapshot_id = delayed_field_data.snapshot(id, max_value, width, resolver)?;
        Value::delayed_value(snapshot_id)
    } else {
        let value = get_aggregator_value(&aggregator, aggregator_value_ty)?;
        create_value_by_type(
            aggregator_value_ty,
            value,
            EUNSUPPORTED_AGGREGATOR_SNAPSHOT_TYPE,
        )?
    };

    Ok(smallvec![Value::struct_(Struct::pack(vec![result_value]))])
}

/***************************************************************************************************
 * native fun create_snapshot<IntElement>(value: IntElement): AggregatorSnapshot<IntElement>
 **************************************************************************************************/

fn native_create_snapshot(
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert_eq!(ty_args.len(), 1);
    debug_assert_eq!(args.len(), 1);
    context.charge(AGGREGATOR_V2_CREATE_SNAPSHOT_BASE)?;

    let snapshot_value_ty = &ty_args[0];
    let value = pop_value_by_type(
        snapshot_value_ty,
        &mut args,
        EUNSUPPORTED_AGGREGATOR_SNAPSHOT_TYPE,
    )?;

    let snapshot_value = if let Some((resolver, mut delayed_field_data)) = get_context_data(context)
    {
        let width = get_width_by_type(snapshot_value_ty, EUNSUPPORTED_AGGREGATOR_TYPE)?;
        let snapshot_id = delayed_field_data.create_new_snapshot(value, width, resolver);
        Value::delayed_value(snapshot_id)
    } else {
        create_value_by_type(
            snapshot_value_ty,
            value,
            EUNSUPPORTED_AGGREGATOR_SNAPSHOT_TYPE,
        )?
    };

    Ok(smallvec![Value::struct_(Struct::pack(vec![
        snapshot_value
    ]))])
}

/***************************************************************************************************
 * native fun copy_snapshot<IntElement>(snapshot: &AggregatorSnapshot<IntElement>): AggregatorSnapshot<IntElement>
 **************************************************************************************************/

fn native_copy_snapshot(
    _context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    _args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    Err(SafeNativeError::Abort {
        abort_code: EAGGREGATOR_FUNCTION_NOT_YET_SUPPORTED,
    })
}

/***************************************************************************************************
 * native fun read_snapshot<IntElement>(snapshot: &AggregatorSnapshot<IntElement>): IntElement;
 **************************************************************************************************/

fn native_read_snapshot(
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert_eq!(ty_args.len(), 1);
    debug_assert_eq!(args.len(), 1);
    context.charge(AGGREGATOR_V2_READ_SNAPSHOT_BASE)?;

    let snapshot_value_ty = &ty_args[0];
    let snapshot = safely_pop_arg!(args, StructRef);

    let result_value = if let Some((resolver, mut delayed_field_data)) = get_context_data(context) {
        let id = get_snapshot_value_as_id(&snapshot, snapshot_value_ty, resolver)?;
        delayed_field_data.read_snapshot(id, resolver)?
    } else {
        get_snapshot_value(&snapshot, snapshot_value_ty)?
    };

    Ok(smallvec![create_value_by_type(
        snapshot_value_ty,
        result_value,
        EUNSUPPORTED_AGGREGATOR_SNAPSHOT_TYPE
    )?])
}

/***************************************************************************************************
 * native fun string_concat<IntElement>(before: String, snapshot: &AggregatorSnapshot<IntElement>, after: String): AggregatorSnapshot<String>;
 **************************************************************************************************/
fn native_string_concat(
    _context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    _args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    // Deprecated function in favor of `derive_string_concat`.
    Err(SafeNativeError::Abort {
        abort_code: EAGGREGATOR_FUNCTION_NOT_YET_SUPPORTED,
    })
}

/***************************************************************************************************
 * native fun read_derived_string(snapshot: &DerivedStringSnapshot): String
 **************************************************************************************************/

fn native_read_derived_string(
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert_eq!(ty_args.len(), 0);
    debug_assert_eq!(args.len(), 1);
    context.charge(AGGREGATOR_V2_READ_SNAPSHOT_BASE)?;

    let result_value = if let Some((resolver, mut delayed_field_data)) = get_context_data(context) {
        let derived_string_snapshot = safely_pop_arg!(args, Reference);
        let id = get_derived_string_snapshot_value_as_id(derived_string_snapshot, resolver)?;
        delayed_field_data.read_derived(id, resolver)?
    } else {
        let derived_string_snapshot = safely_pop_arg!(args, StructRef);
        get_derived_string_snapshot_value(&derived_string_snapshot)?
    };

    Ok(smallvec![create_string_value(result_value)])
}

/***************************************************************************************************
 * native fun create_derived_string(value: String): DerivedStringSnapshot
 **************************************************************************************************/

fn native_create_derived_string(
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert_eq!(ty_args.len(), 0);
    debug_assert_eq!(args.len(), 1);
    context.charge(AGGREGATOR_V2_CREATE_SNAPSHOT_BASE)?;

    let value_bytes = string_to_bytes(safely_pop_arg!(args, Struct))
        .map_err(SafeNativeError::InvariantViolation)?;
    context
        .charge(AGGREGATOR_V2_CREATE_SNAPSHOT_PER_BYTE * NumBytes::new(value_bytes.len() as u64))?;

    if value_bytes.len() > DERIVED_STRING_INPUT_MAX_LENGTH {
        return Err(SafeNativeError::Abort {
            abort_code: EINPUT_STRING_LENGTH_TOO_LARGE,
        });
    }

    let derived_string_snapshot =
        if let Some((resolver, mut delayed_field_data)) = get_context_data(context) {
            let id = delayed_field_data.create_new_derived(value_bytes, resolver)?;
            Value::delayed_value(id)
        } else {
            let width = calculate_width_for_constant_string(value_bytes.len());
            bytes_and_width_to_derived_string_struct(value_bytes, width)
                .map_err(SafeNativeError::InvariantViolation)?
        };

    Ok(smallvec![derived_string_snapshot])
}

/***************************************************************************************************
 * native fun derive_string_concat<IntElement>(
 *     before: String,
 *     snapshot: &AggregatorSnapshot<IntElement>,
 *     after: String,
 * ): DerivedStringSnapshot;
 **************************************************************************************************/

fn native_derive_string_concat(
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert_eq!(ty_args.len(), 1);
    debug_assert_eq!(args.len(), 3);
    context.charge(AGGREGATOR_V2_STRING_CONCAT_BASE)?;

    // Popping arguments from the end.
    let suffix = string_to_bytes(safely_pop_arg!(args, Struct))
        .map_err(SafeNativeError::InvariantViolation)?;
    context.charge(AGGREGATOR_V2_STRING_CONCAT_PER_BYTE * NumBytes::new(suffix.len() as u64))?;

    let snapshot_value_ty = &ty_args[0];
    let snapshot = safely_pop_arg!(args, StructRef);

    let prefix = string_to_bytes(safely_pop_arg!(args, Struct))
        .map_err(SafeNativeError::InvariantViolation)?;
    context.charge(AGGREGATOR_V2_STRING_CONCAT_PER_BYTE * NumBytes::new(prefix.len() as u64))?;

    if prefix
        .len()
        .checked_add(suffix.len())
        .is_some_and(|v| v > DERIVED_STRING_INPUT_MAX_LENGTH)
    {
        return Err(SafeNativeError::Abort {
            abort_code: EINPUT_STRING_LENGTH_TOO_LARGE,
        });
    }

    let derived_string_snapshot = if let Some((resolver, mut delayed_field_data)) =
        get_context_data(context)
    {
        let id = get_snapshot_value_as_id(&snapshot, snapshot_value_ty, resolver)?;
        let derived_string_snapshot_id =
            delayed_field_data.derive_string_concat(id, prefix, suffix, resolver)?;
        Value::delayed_value(derived_string_snapshot_id)
    } else {
        let snapshot_width =
            get_width_by_type(snapshot_value_ty, EUNSUPPORTED_AGGREGATOR_SNAPSHOT_TYPE)?;
        let width = calculate_width_for_integer_embedded_string(
            prefix.len() + suffix.len(),
            DelayedFieldID::new_with_width(0, snapshot_width),
        )
        .map_err(SafeNativeError::InvariantViolation)?;

        let snapshot_value = get_snapshot_value(&snapshot, snapshot_value_ty)?;
        let output = SnapshotToStringFormula::Concat { prefix, suffix }.apply_to(snapshot_value);
        bytes_and_width_to_derived_string_struct(output, width)?
    };

    Ok(smallvec![derived_string_snapshot])
}

#[test]
fn test_max_size_fits() {
    DelayedFieldID::new_with_width(
        0,
        u32::try_from(
            (calculate_width_for_integer_embedded_string(
                DERIVED_STRING_INPUT_MAX_LENGTH,
                DelayedFieldID::new_with_width(0, 16),
            ))
            .unwrap(),
        )
        .unwrap(),
    );
}

/***************************************************************************************************
 * module
 **************************************************************************************************/

pub fn make_all(
    builder: &SafeNativeBuilder,
) -> impl Iterator<Item = (String, NativeFunction)> + '_ {
    let natives = [
        (
            "create_aggregator",
            native_create_aggregator as RawSafeNative,
        ),
        (
            "create_unbounded_aggregator",
            native_create_unbounded_aggregator,
        ),
        ("try_add", native_try_add),
        ("try_sub", native_try_sub),
        ("is_at_least_impl", native_is_at_least_impl),
        ("read", native_read),
        ("snapshot", native_snapshot),
        ("create_snapshot", native_create_snapshot),
        ("copy_snapshot", native_copy_snapshot),
        ("read_snapshot", native_read_snapshot),
        ("string_concat", native_string_concat),
        ("read_derived_string", native_read_derived_string),
        ("create_derived_string", native_create_derived_string),
        ("derive_string_concat", native_derive_string_concat),
    ];
    builder.make_named_natives(natives)
}
