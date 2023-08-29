// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
use crate::natives::aggregator_natives::helpers::get_aggregator_field;
use aptos_aggregator::aggregator_extension::extension_error;
use move_binary_format::errors::PartialVMResult;
use move_vm_types::values::{Struct, StructRef, Value};

const VALUE_FIELD_INDEX: usize = 0;

/// Returns ID of aggrgegator snapshot based on a reference to `AggregatorSnapshot` Move struct.
pub(crate) fn aggregator_snapshot_u128_info(
    aggregator_snapshot: &StructRef,
) -> PartialVMResult<u128> {
    let value = get_aggregator_field(aggregator_snapshot, VALUE_FIELD_INDEX)?.value_as::<u128>()?;
    Ok(value)
}

/// Returns ID of aggrgegator snapshot based on a reference to `AggregatorSnapshot` Move struct.
pub(crate) fn aggregator_snapshot_u64_info(
    aggregator_snapshot: &StructRef,
) -> PartialVMResult<u64> {
    let value = get_aggregator_field(aggregator_snapshot, VALUE_FIELD_INDEX)?.value_as::<u64>()?;
    Ok(value)
}

pub(crate) fn aggregator_snapshot_string_info(
    aggregator_snapshot: &StructRef,
) -> PartialVMResult<Vec<u8>> {
    get_aggregator_field(aggregator_snapshot, VALUE_FIELD_INDEX)?
        .value_as::<Struct>()?
        .unpack()?
        .collect::<Vec<Value>>()
        .pop()
        .map_or(
            Err(extension_error("unable to pop string field in snapshot")),
            |v| v.value_as::<Vec<u8>>(),
        )
}

pub(crate) fn string_value_info(string_value: Struct) -> PartialVMResult<Vec<u8>> {
    string_value.unpack()?.collect::<Vec<Value>>().pop().map_or(
        Err(extension_error("unable to extract string value")),
        |v| v.value_as::<Vec<u8>>(),
    )
}
