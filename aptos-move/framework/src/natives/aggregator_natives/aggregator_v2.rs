// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::natives::{
    aggregator_natives::{
        helpers_v2::{
            aggregator_snapshot_field_value, aggregator_snapshot_value_field_as_id,
            aggregator_value_field_as_id, get_aggregator_fields_u128, get_aggregator_fields_u64,
            set_aggregator_value_field,
        },
        NativeAggregatorContext,
    },
    AccountAddress,
};
use aptos_aggregator::{
    bounded_math::{BoundedMath, SignedU128},
    delayed_field_extension::DelayedFieldData,
    resolver::DelayedFieldResolver,
    types::{SnapshotToStringFormula, SnapshotValue},
    utils::{string_to_bytes, to_utf8_bytes, u128_to_u64},
};
use aptos_gas_algebra::NumBytes;
use aptos_gas_schedule::gas_params::natives::aptos_framework::*;
use aptos_native_interface::{
    safely_pop_arg, RawSafeNative, SafeNativeBuilder, SafeNativeContext, SafeNativeError,
    SafeNativeResult,
};
use move_binary_format::errors::PartialVMError;
use move_core_types::{
    value::{MoveStructLayout, MoveTypeLayout},
    vm_status::StatusCode,
};
use move_vm_runtime::native_functions::NativeFunction;
use move_vm_types::{
    loaded_data::runtime_types::Type,
    values::{Struct, StructRef, Value},
};
use smallvec::{smallvec, SmallVec};
use std::{cell::RefMut, collections::VecDeque, ops::Deref};

/// The generic type supplied to aggregator snapshots is not supported.
pub const EUNSUPPORTED_AGGREGATOR_SNAPSHOT_TYPE: u64 = 0x03_0005;

/// The aggregator api feature is not enabled.
pub const EAGGREGATOR_API_NOT_ENABLED: u64 = 0x03_0006;

/// The generic type supplied to the aggregators is not supported.
pub const EUNSUPPORTED_AGGREGATOR_TYPE: u64 = 0x03_0007;

/// Arguments passed to concat or create_snapshot exceed max limit of
/// STRING_SNAPSHOT_INPUT_MAX_LENGTH bytes (for prefix and suffix together).
pub const EINPUT_STRING_LENGTH_TOO_LARGE: u64 = 0x03_0008;

/// The native aggregator function, that is in the move file, is not yet supported.
/// and any calls will raise this error.
pub const EAGGREGATOR_FUNCTION_NOT_YET_SUPPORTED: u64 = 0x03_0009;

pub const STRING_SNAPSHOT_INPUT_MAX_LENGTH: usize = 256;

/// Checks if the type argument `type_arg` is a string type.
fn is_string_type(context: &SafeNativeContext, type_arg: &Type) -> SafeNativeResult<bool> {
    let ty = context.deref().type_to_fully_annotated_layout(type_arg)?;
    if let MoveTypeLayout::Struct(MoveStructLayout::WithTypes { type_, .. }) = ty {
        return Ok(type_.name.as_str() == "String"
            && type_.module.as_str() == "string"
            && type_.address == AccountAddress::ONE);
    }
    Ok(false)
}

/// Given the native function argument and a type, returns a tuple of its
/// fields: (`aggregator id`, `max_value`).
pub fn get_aggregator_fields_by_type(
    ty_arg: &Type,
    agg: &StructRef,
) -> SafeNativeResult<(u128, u128)> {
    match ty_arg {
        Type::U128 => {
            let (id, max_value) = get_aggregator_fields_u128(agg)?;
            Ok((id, max_value))
        },
        Type::U64 => {
            let (id, max_value) = get_aggregator_fields_u64(agg)?;
            Ok((id as u128, max_value as u128))
        },
        _ => Err(SafeNativeError::Abort {
            abort_code: EUNSUPPORTED_AGGREGATOR_TYPE,
        }),
    }
}

/// Given the list of native function arguments and a type, pop the next argument if it is of given type.
pub fn pop_value_by_type(ty_arg: &Type, args: &mut VecDeque<Value>) -> SafeNativeResult<u128> {
    match ty_arg {
        Type::U128 => Ok(safely_pop_arg!(args, u128)),
        Type::U64 => Ok(safely_pop_arg!(args, u64) as u128),
        _ => Err(SafeNativeError::Abort {
            abort_code: EUNSUPPORTED_AGGREGATOR_TYPE,
        }),
    }
}

pub fn create_value_by_type(ty_arg: &Type, value: u128) -> SafeNativeResult<Value> {
    match ty_arg {
        Type::U128 => Ok(Value::u128(value)),
        Type::U64 => Ok(Value::u64(u128_to_u64(value)?)),
        _ => Err(SafeNativeError::Abort {
            abort_code: EUNSUPPORTED_AGGREGATOR_TYPE,
        }),
    }
}

// To avoid checking is_string_type multiple times, check type_arg only once, and convert into this enum
enum SnapshotType {
    U128,
    U64,
    String,
}

impl SnapshotType {
    fn from_ty_arg(context: &SafeNativeContext, ty_arg: &Type) -> SafeNativeResult<Self> {
        match ty_arg {
            Type::U128 => Ok(Self::U128),
            Type::U64 => Ok(Self::U64),
            _ => {
                // Check if the type is a string
                if is_string_type(context, ty_arg)? {
                    Ok(Self::String)
                } else {
                    // If not a string, return an error
                    Err(SafeNativeError::Abort {
                        abort_code: EUNSUPPORTED_AGGREGATOR_SNAPSHOT_TYPE,
                    })
                }
            },
        }
    }

    pub fn pop_snapshot_field_by_type(
        &self,
        args: &mut VecDeque<Value>,
    ) -> SafeNativeResult<SnapshotValue> {
        self.parse_snapshot_value_by_type(aggregator_snapshot_field_value(&safely_pop_arg!(
            args, StructRef
        ))?)
    }

    pub fn pop_snapshot_value_by_type(
        &self,
        args: &mut VecDeque<Value>,
    ) -> SafeNativeResult<SnapshotValue> {
        match self {
            SnapshotType::U128 => Ok(SnapshotValue::Integer(safely_pop_arg!(args, u128))),
            SnapshotType::U64 => Ok(SnapshotValue::Integer(safely_pop_arg!(args, u64) as u128)),
            SnapshotType::String => {
                let input = string_to_bytes(safely_pop_arg!(args, Struct))?;
                Ok(SnapshotValue::String(input))
            },
        }
    }

    pub fn parse_snapshot_value_by_type(&self, value: Value) -> SafeNativeResult<SnapshotValue> {
        // Simpler to wrap to be able to reuse safely_pop_arg functions
        self.pop_snapshot_value_by_type(&mut VecDeque::from([value]))
    }

    pub fn create_snapshot_value_by_type(&self, value: SnapshotValue) -> SafeNativeResult<Value> {
        match (self, value) {
            (SnapshotType::U128, SnapshotValue::Integer(v)) => Ok(Value::u128(v)),
            (SnapshotType::U64, SnapshotValue::Integer(v)) => Ok(Value::u64(u128_to_u64(v)?)),
            (SnapshotType::String, value) => {
                Ok(Value::struct_(Struct::pack(vec![Value::vector_u8(
                    match value {
                        SnapshotValue::String(v) => v,
                        SnapshotValue::Integer(v) => to_utf8_bytes(v),
                    },
                )])))
            },
            // Type cannot be Integer, if value is String
            _ => Err(SafeNativeError::Abort {
                abort_code: EUNSUPPORTED_AGGREGATOR_SNAPSHOT_TYPE,
            }),
        }
    }
}

fn get_context_data<'t, 'b>(
    context: &'t mut SafeNativeContext<'_, 'b, '_, '_>,
) -> Option<(&'b dyn DelayedFieldResolver, RefMut<'t, DelayedFieldData>)> {
    let aggregator_context = context.extensions().get::<NativeAggregatorContext>();
    if aggregator_context
        .delayed_field_resolver
        .is_delayed_field_optimization_capable()
        && context.aggregator_v2_delayed_fields_enabled()
    {
        Some((
            aggregator_context.delayed_field_resolver,
            aggregator_context.delayed_field_data.borrow_mut(),
        ))
    } else {
        None
    }
}

macro_rules! abort_if_not_enabled {
    ($context:expr) => {
        if !$context.aggregator_v2_api_enabled() {
            return Err(SafeNativeError::Abort {
                abort_code: EAGGREGATOR_API_NOT_ENABLED,
            });
        }
    };
}

fn create_aggregator_impl(
    context: &mut SafeNativeContext,
    ty_arg: &Type,
    max_value: u128,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    let value_field_value =
        if let Some((resolver, mut delayed_field_data)) = get_context_data(context) {
            let id = resolver.generate_delayed_field_id();
            delayed_field_data.create_new_aggregator(id);
            id.as_u64() as u128
        } else {
            0
        };

    Ok(smallvec![Value::struct_(Struct::pack(vec![
        create_value_by_type(ty_arg, value_field_value)?,
        create_value_by_type(ty_arg, max_value)?,
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
    abort_if_not_enabled!(context);

    debug_assert_eq!(args.len(), 1);
    debug_assert_eq!(ty_args.len(), 1);

    context.charge(AGGREGATOR_V2_CREATE_AGGREGATOR_BASE)?;
    let max_value = pop_value_by_type(&ty_args[0], &mut args)?;

    create_aggregator_impl(context, &ty_args[0], max_value)
}

/***************************************************************************************************
 * native fun create_unbounded_aggregator<IntElement: copy + drop>(): Aggregator<IntElement>;
 **************************************************************************************************/

fn native_create_unbounded_aggregator(
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    abort_if_not_enabled!(context);

    debug_assert_eq!(args.len(), 0);
    debug_assert_eq!(ty_args.len(), 1);

    context.charge(AGGREGATOR_V2_CREATE_AGGREGATOR_BASE)?;
    let max_value = {
        match &ty_args[0] {
            Type::U128 => u128::MAX,
            Type::U64 => u64::MAX as u128,
            _ => {
                return Err(SafeNativeError::Abort {
                    abort_code: EUNSUPPORTED_AGGREGATOR_TYPE,
                })
            },
        }
    };

    create_aggregator_impl(context, &ty_args[0], max_value)
}

/***************************************************************************************************
 * native fun try_add<IntElement>(aggregator: &mut Aggregator<IntElement>, value: IntElement): bool;
 **************************************************************************************************/
fn native_try_add(
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    abort_if_not_enabled!(context);

    debug_assert_eq!(args.len(), 2);
    debug_assert_eq!(ty_args.len(), 1);
    context.charge(AGGREGATOR_V2_TRY_ADD_BASE)?;

    let input = pop_value_by_type(&ty_args[0], &mut args)?;
    let agg_struct = safely_pop_arg!(args, StructRef);
    let (agg_value, agg_max_value) = get_aggregator_fields_by_type(&ty_args[0], &agg_struct)?;

    let result_value = if let Some((resolver, mut delayed_field_data)) = get_context_data(context) {
        let id = aggregator_value_field_as_id(agg_value, resolver)?;
        delayed_field_data.try_add_delta(
            id,
            agg_max_value,
            SignedU128::Positive(input),
            resolver,
        )?
    } else {
        let math = BoundedMath::new(agg_max_value);
        match math.unsigned_add(agg_value, input) {
            Ok(sum) => {
                set_aggregator_value_field(&agg_struct, create_value_by_type(&ty_args[0], sum)?)?;
                true
            },
            Err(_) => false,
        }
    };

    Ok(smallvec![Value::bool(result_value)])
}

/***************************************************************************************************
 * native fun try_sub<IntElement>(aggregator: &mut Aggregator<IntElement>, value: IntElement): bool;
 **************************************************************************************************/
fn native_try_sub(
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    abort_if_not_enabled!(context);

    debug_assert_eq!(args.len(), 2);
    debug_assert_eq!(ty_args.len(), 1);
    context.charge(AGGREGATOR_V2_TRY_SUB_BASE)?;

    let input = pop_value_by_type(&ty_args[0], &mut args)?;
    let agg_struct = safely_pop_arg!(args, StructRef);
    let (agg_value, agg_max_value) = get_aggregator_fields_by_type(&ty_args[0], &agg_struct)?;

    let result_value = if let Some((resolver, mut delayed_field_data)) = get_context_data(context) {
        let id = aggregator_value_field_as_id(agg_value, resolver)?;
        delayed_field_data.try_add_delta(
            id,
            agg_max_value,
            SignedU128::Negative(input),
            resolver,
        )?
    } else {
        let math = BoundedMath::new(agg_max_value);
        match math.unsigned_subtract(agg_value, input) {
            Ok(sum) => {
                set_aggregator_value_field(&agg_struct, create_value_by_type(&ty_args[0], sum)?)?;
                true
            },
            Err(_) => false,
        }
    };
    Ok(smallvec![Value::bool(result_value)])
}

/***************************************************************************************************
 * native fun read<IntElement>(aggregator: &Aggregator<IntElement>): IntElement;
 **************************************************************************************************/

fn native_read(
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    abort_if_not_enabled!(context);

    debug_assert_eq!(args.len(), 1);
    debug_assert_eq!(ty_args.len(), 1);
    context.charge(AGGREGATOR_V2_READ_BASE)?;

    let (agg_value, agg_max_value) =
        get_aggregator_fields_by_type(&ty_args[0], &safely_pop_arg!(args, StructRef))?;

    let result_value = if let Some((resolver, delayed_field_data)) = get_context_data(context) {
        let id = aggregator_value_field_as_id(agg_value, resolver)?;
        delayed_field_data.read_aggregator(id, resolver)?
    } else {
        agg_value
    };

    if result_value > agg_max_value {
        return Err(SafeNativeError::InvariantViolation(PartialVMError::new(
            StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
        )));
    };
    Ok(smallvec![create_value_by_type(&ty_args[0], result_value)?])
}

/***************************************************************************************************
 * native fun snapshot<IntElement>(aggregator: &Aggregator<IntElement>): AggregatorSnapshot<IntElement>;
 **************************************************************************************************/

fn native_snapshot(
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    abort_if_not_enabled!(context);

    debug_assert_eq!(args.len(), 1);
    debug_assert_eq!(ty_args.len(), 1);
    context.charge(AGGREGATOR_V2_SNAPSHOT_BASE)?;

    let (agg_value, agg_max_value) =
        get_aggregator_fields_by_type(&ty_args[0], &safely_pop_arg!(args, StructRef))?;

    let result_value = if let Some((resolver, mut delayed_field_data)) = get_context_data(context) {
        let aggregator_id = aggregator_value_field_as_id(agg_value, resolver)?;
        delayed_field_data
            .snapshot(aggregator_id, agg_max_value, resolver)?
            .as_u64() as u128
    } else {
        agg_value
    };

    Ok(smallvec![Value::struct_(Struct::pack(vec![
        create_value_by_type(&ty_args[0], result_value)?
    ]))])
}

/***************************************************************************************************
 * native fun create_snapshot<Element>(value: Element): AggregatorSnapshot<Element>
 **************************************************************************************************/

fn native_create_snapshot(
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    abort_if_not_enabled!(context);

    debug_assert_eq!(ty_args.len(), 1);
    debug_assert_eq!(args.len(), 1);
    context.charge(AGGREGATOR_V2_CREATE_SNAPSHOT_BASE)?;

    let snapshot_type = SnapshotType::from_ty_arg(context, &ty_args[0])?;
    let input = snapshot_type.pop_snapshot_value_by_type(&mut args)?;

    if let SnapshotValue::String(v) = &input {
        context.charge(AGGREGATOR_V2_CREATE_SNAPSHOT_PER_BYTE * NumBytes::new(v.len() as u64))?;
        if v.len() > STRING_SNAPSHOT_INPUT_MAX_LENGTH {
            return Err(SafeNativeError::Abort {
                abort_code: EINPUT_STRING_LENGTH_TOO_LARGE,
            });
        }
    }

    let result_value = if let Some((resolver, mut delayed_field_data)) = get_context_data(context) {
        let snapshot_id = delayed_field_data.create_new_snapshot(input, resolver);
        SnapshotValue::Integer(snapshot_id.as_u64() as u128)
    } else {
        input
    };

    Ok(smallvec![Value::struct_(Struct::pack(vec![
        snapshot_type.create_snapshot_value_by_type(result_value)?
    ]))])
}

/***************************************************************************************************
 * native fun copy_snapshot<Element>(snapshot: &AggregatorSnapshot<Element>): AggregatorSnapshot<Element>
 **************************************************************************************************/

fn native_copy_snapshot(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    _args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    abort_if_not_enabled!(context);

    Err(SafeNativeError::Abort {
        abort_code: EAGGREGATOR_FUNCTION_NOT_YET_SUPPORTED,
    })

    // debug_assert_eq!(ty_args.len(), 1);
    // debug_assert_eq!(args.len(), 1);
    // context.charge(AGGREGATOR_V2_COPY_SNAPSHOT_BASE)?;

    // let snapshot_type = SnapshotType::from_ty_arg(context, &ty_args[0])?;
    // let snapshot_value = snapshot_type.pop_snapshot_field_by_type(&mut args)?;

    // let result_value = if context.aggregator_execution_enabled() {
    //     let id = aggregator_snapshot_value_field_as_id(snapshot_value, resolver)?;

    //     // snapshots are immutable so we can just return the id
    //     SnapshotValue::Integer(id.id() as u128)
    // } else {
    //     snapshot_value
    // };

    // Ok(smallvec![Value::struct_(Struct::pack(vec![
    //     snapshot_type.create_snapshot_value_by_type(result_value)?
    // ]))])
}

/***************************************************************************************************
 * native fun read_snapshot<Element>(snapshot: &AggregatorSnapshot<Element>): Element;
 **************************************************************************************************/

fn native_read_snapshot(
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    abort_if_not_enabled!(context);

    debug_assert_eq!(ty_args.len(), 1);
    debug_assert_eq!(args.len(), 1);
    context.charge(AGGREGATOR_V2_READ_SNAPSHOT_BASE)?;

    let snapshot_type = SnapshotType::from_ty_arg(context, &ty_args[0])?;
    let snapshot_value = snapshot_type.pop_snapshot_field_by_type(&mut args)?;

    let result_value = if let Some((resolver, mut delayed_field_data)) = get_context_data(context) {
        let aggregator_id = aggregator_snapshot_value_field_as_id(snapshot_value, resolver)?;
        delayed_field_data.read_snapshot(aggregator_id, resolver)?
    } else {
        snapshot_value
    };

    Ok(smallvec![
        snapshot_type.create_snapshot_value_by_type(result_value)?
    ])
}

/***************************************************************************************************
 * native fun string_concat<IntElement>(before: String, snapshot: &AggregatorSnapshot<IntElement>, after: String): AggregatorSnapshot<String>;
 **************************************************************************************************/

fn native_string_concat(
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    abort_if_not_enabled!(context);

    debug_assert_eq!(ty_args.len(), 1);
    debug_assert_eq!(args.len(), 3);
    context.charge(AGGREGATOR_V2_STRING_CONCAT_BASE)?;

    let snapshot_input_type = SnapshotType::from_ty_arg(context, &ty_args[0])?;

    // Concat works only with integer snapshot types
    // This is to avoid unnecessary recursive snapshot dependencies
    if !matches!(snapshot_input_type, SnapshotType::U128 | SnapshotType::U64) {
        return Err(SafeNativeError::Abort {
            abort_code: EUNSUPPORTED_AGGREGATOR_SNAPSHOT_TYPE,
        });
    }

    // popping arguments from the end
    let suffix = string_to_bytes(safely_pop_arg!(args, Struct))?;
    context.charge(AGGREGATOR_V2_STRING_CONCAT_PER_BYTE * NumBytes::new(suffix.len() as u64))?;

    let snapshot_value = match snapshot_input_type.pop_snapshot_field_by_type(&mut args)? {
        SnapshotValue::Integer(v) => v,
        SnapshotValue::String(_) => {
            return Err(SafeNativeError::Abort {
                abort_code: EUNSUPPORTED_AGGREGATOR_SNAPSHOT_TYPE,
            })
        },
    };

    let prefix = string_to_bytes(safely_pop_arg!(args, Struct))?;
    context.charge(AGGREGATOR_V2_STRING_CONCAT_PER_BYTE * NumBytes::new(prefix.len() as u64))?;

    if prefix
        .len()
        .checked_add(suffix.len())
        .map_or(false, |v| v > STRING_SNAPSHOT_INPUT_MAX_LENGTH)
    {
        return Err(SafeNativeError::Abort {
            abort_code: EINPUT_STRING_LENGTH_TOO_LARGE,
        });
    }

    let result_value = if let Some((resolver, mut delayed_field_data)) = get_context_data(context) {
        let base_id = aggregator_value_field_as_id(snapshot_value, resolver)?;
        SnapshotValue::Integer(
            delayed_field_data
                .string_concat(base_id, prefix, suffix, resolver)?
                .as_u64() as u128,
        )
    } else {
        SnapshotValue::String(
            SnapshotToStringFormula::Concat { prefix, suffix }.apply_to(snapshot_value),
        )
    };

    Ok(smallvec![Value::struct_(Struct::pack(vec![
        SnapshotType::String.create_snapshot_value_by_type(result_value)?
    ]))])
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
        ("read", native_read),
        ("try_sub", native_try_sub),
        ("snapshot", native_snapshot),
        ("create_snapshot", native_create_snapshot),
        ("copy_snapshot", native_copy_snapshot),
        ("read_snapshot", native_read_snapshot),
        ("string_concat", native_string_concat),
    ];
    builder.make_named_natives(natives)
}
