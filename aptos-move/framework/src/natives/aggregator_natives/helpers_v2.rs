// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::helpers_v1::set_aggregator_field;
use aptos_aggregator::{resolver::DelayedFieldResolver, types::DelayedFieldID};
use aptos_types::delayed_fields::{from_utf8_bytes, u128_to_u64};
use move_binary_format::errors::PartialVMResult;
use move_vm_types::values::{StructRef, Value};

/// Indices of `value` and `limit` fields in the `Aggregator` Move
/// struct.
pub(crate) const AGG_VALUE_FIELD_INDEX: usize = 0;
pub(crate) const AGG_MAX_VALUE_FIELD_INDEX: usize = 1;

pub(crate) const AGG_SNAPSHOT_VALUE_FIELD_INDEX: usize = 0;

pub(crate) const DERIVED_STRING_VALUE_FIELD_INDEX: usize = 0;
// pub (crate) const DERIVED_STRING_PADDING_FIELD_INDEX: usize = 1;

pub(crate) fn aggregator_value_field_as_id(
    value: u128,
    resolver: &dyn DelayedFieldResolver,
) -> PartialVMResult<DelayedFieldID> {
    let value_u64 = u128_to_u64(value)?;
    Ok(resolver.validate_and_convert_delayed_field_id(value_u64)?)
}

pub(crate) fn aggregator_snapshot_value_field_as_id(
    value: u128,
    resolver: &dyn DelayedFieldResolver,
) -> PartialVMResult<DelayedFieldID> {
    aggregator_value_field_as_id(value, resolver)
}

pub(crate) fn derived_string_value_field_as_id(
    value: Vec<u8>,
    resolver: &dyn DelayedFieldResolver,
) -> PartialVMResult<DelayedFieldID> {
    let value_u64: u64 = from_utf8_bytes(value)?;
    Ok(resolver.validate_and_convert_delayed_field_id(value_u64)?)
}

pub(crate) fn set_aggregator_value_field(
    aggregator: &StructRef,
    value: Value,
) -> PartialVMResult<()> {
    set_aggregator_field(aggregator, AGG_VALUE_FIELD_INDEX, value)
}
