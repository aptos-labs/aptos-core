// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::natives::aggregator_natives::aggregator_v2::{
    EUNSUPPORTED_AGGREGATOR_SNAPSHOT_TYPE, EUNSUPPORTED_AGGREGATOR_TYPE,
};
use aptos_aggregator::resolver::DelayedFieldResolver;
use aptos_native_interface::{safely_get_struct_field_as, SafeNativeError, SafeNativeResult};
use move_binary_format::errors::PartialVMError;
use move_vm_types::{
    delayed_values::{delayed_field_id::DelayedFieldID, derived_string_snapshot::string_to_bytes},
    loaded_data::runtime_types::Type,
    values::{Reference, Struct, StructRef, Value},
};

// Field indices for aggregator Move struct.
const AGGREGATOR_VALUE_FIELD_INDEX: usize = 0;
const AGGREGATOR_MAX_VALUE_FIELD_INDEX: usize = 1;

// Field indices for aggregator snapshot Move struct.
const AGGREGATOR_SNAPSHOT_VALUE_FIELD_INDEX: usize = 0;

// Field indices for derived string snapshot Move struct.
const DERIVED_STRING_SNAPSHOT_VALUE_FIELD_INDEX: usize = 0;
const _DERIVED_STRING_SNAPSHOT_PADDING_FIELD_INDEX: usize = 1;

macro_rules! get_value_impl {
    ($func_name:ident, $idx:expr_2021, $e:expr_2021) => {
        pub(crate) fn $func_name(struct_ref: &StructRef, ty: &Type) -> SafeNativeResult<u128> {
            Ok(match ty {
                Type::U128 => safely_get_struct_field_as!(struct_ref, $idx, u128),
                Type::U64 => safely_get_struct_field_as!(struct_ref, $idx, u64) as u128,
                _ => return Err(SafeNativeError::Abort { abort_code: $e }),
            })
        }
    };
}

get_value_impl!(
    get_aggregator_max_value,
    AGGREGATOR_MAX_VALUE_FIELD_INDEX,
    EUNSUPPORTED_AGGREGATOR_TYPE
);

get_value_impl!(
    get_aggregator_value,
    AGGREGATOR_VALUE_FIELD_INDEX,
    EUNSUPPORTED_AGGREGATOR_TYPE
);

get_value_impl!(
    get_snapshot_value,
    AGGREGATOR_SNAPSHOT_VALUE_FIELD_INDEX,
    EUNSUPPORTED_AGGREGATOR_SNAPSHOT_TYPE
);

macro_rules! get_value_as_id_impl {
    ($func_name:ident, $idx:expr_2021, $e:expr_2021) => {
        pub(crate) fn $func_name(
            struct_ref: &StructRef,
            ty: &Type,
            resolver: &dyn DelayedFieldResolver,
        ) -> SafeNativeResult<DelayedFieldID> {
            let id = match ty {
                Type::U64 | Type::U128 => {
                    safely_get_struct_field_as!(struct_ref, $idx, DelayedFieldID)
                },
                _ => return Err(SafeNativeError::Abort { abort_code: $e }),
            };

            // Make sure we validate generated id is correct, i.e. it lies within the
            // right bounds, etc. to catch bugs early.
            resolver
                .validate_delayed_field_id(&id)
                .map_err(|e| SafeNativeError::InvariantViolation(PartialVMError::from(e)))?;
            Ok(id)
        }
    };
}

get_value_as_id_impl!(
    get_aggregator_value_as_id,
    AGGREGATOR_VALUE_FIELD_INDEX,
    EUNSUPPORTED_AGGREGATOR_TYPE
);

get_value_as_id_impl!(
    get_snapshot_value_as_id,
    AGGREGATOR_SNAPSHOT_VALUE_FIELD_INDEX,
    EUNSUPPORTED_AGGREGATOR_SNAPSHOT_TYPE
);

pub(crate) fn set_aggregator_value(aggregator: &StructRef, value: Value) -> SafeNativeResult<()> {
    aggregator
        .borrow_field(AGGREGATOR_VALUE_FIELD_INDEX)
        .map_err(SafeNativeError::InvariantViolation)?
        .value_as::<Reference>()
        .map_err(SafeNativeError::InvariantViolation)?
        .write_ref(value)
        .map_err(SafeNativeError::InvariantViolation)
}

pub(crate) fn unbounded_aggregator_max_value(ty: &Type) -> SafeNativeResult<u128> {
    Ok(match ty {
        Type::U128 => u128::MAX,
        Type::U64 => u64::MAX as u128,
        _ => {
            return Err(SafeNativeError::Abort {
                abort_code: EUNSUPPORTED_AGGREGATOR_TYPE,
            })
        },
    })
}

pub(crate) fn get_derived_string_snapshot_value(
    derived_string_snapshot: &StructRef,
) -> SafeNativeResult<Vec<u8>> {
    let derived_string_snapshot_value = safely_get_struct_field_as!(
        derived_string_snapshot,
        DERIVED_STRING_SNAPSHOT_VALUE_FIELD_INDEX,
        Struct
    );
    string_to_bytes(derived_string_snapshot_value).map_err(SafeNativeError::InvariantViolation)
}

pub(crate) fn get_derived_string_snapshot_value_as_id(
    derived_string_snapshot: Reference,
    resolver: &dyn DelayedFieldResolver,
) -> SafeNativeResult<DelayedFieldID> {
    let id = derived_string_snapshot
        .read_ref()
        .map_err(SafeNativeError::InvariantViolation)?
        .value_as::<DelayedFieldID>()
        .map_err(SafeNativeError::InvariantViolation)?;
    resolver
        .validate_delayed_field_id(&id)
        .map_err(|e| SafeNativeError::InvariantViolation(PartialVMError::from(e)))?;
    Ok(id)
}
