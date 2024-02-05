// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::helpers_v1::set_aggregator_field;
use move_binary_format::errors::PartialVMResult;
use move_vm_types::values::{StructRef, Value};

/// Indices of `value` and `limit` fields in the `Aggregator` Move
/// struct.
pub(crate) const AGG_VALUE_FIELD_INDEX: usize = 0;
pub(crate) const AGG_MAX_VALUE_FIELD_INDEX: usize = 1;

pub(crate) const AGG_SNAPSHOT_VALUE_FIELD_INDEX: usize = 0;

pub(crate) const DERIVED_STRING_VALUE_FIELD_INDEX: usize = 0;
// pub (crate) const DERIVED_STRING_PADDING_FIELD_INDEX: usize = 1;

pub(crate) fn set_aggregator_value_field(
    aggregator: &StructRef,
    value: Value,
) -> PartialVMResult<()> {
    set_aggregator_field(aggregator, AGG_VALUE_FIELD_INDEX, value)
}
