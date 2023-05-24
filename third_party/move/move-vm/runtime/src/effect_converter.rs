// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use move_binary_format::errors::{PartialVMError, PartialVMResult};
use move_core_types::{value::MoveTypeLayout, vm_status::StatusCode};
use move_vm_types::values::Value;

/// Trait that defines a generic interface for converting effects of a
/// transaction into a desired type. This allows clients of the Move VM
/// to process effects in their own way.
pub trait EffectConverter<R> {
    fn convert_resource(&self, value: Value, layout: MoveTypeLayout) -> PartialVMResult<R>;
}

/// Default effects converter which serializes all changes.
pub struct StandardEffectConverter;

impl EffectConverter<Vec<u8>> for StandardEffectConverter {
    fn convert_resource(&self, value: Value, layout: MoveTypeLayout) -> PartialVMResult<Vec<u8>> {
        value.simple_serialize(&layout).ok_or_else(|| {
            PartialVMError::new(StatusCode::INTERNAL_TYPE_ERROR)
                .with_message(format!("Error when serializing resource {}.", value))
        })
    }
}
