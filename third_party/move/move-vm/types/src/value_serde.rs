// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    delayed_values::delayed_field_id::DelayedFieldID,
    values::{DeserializationSeed, SerializationReadyValue, Value},
};
use move_binary_format::errors::{PartialVMError, PartialVMResult};
use move_core_types::{
    identifier::IdentStr,
    language_storage::ModuleId,
    value::{IdentifierMappingKind, MoveTypeLayout},
    vm_status::StatusCode,
};
use std::cell::RefCell;

pub trait FunctionExtension {
    fn get_function_layout(
        &self,
        module_id: &ModuleId,
        function_name: &IdentStr,
    ) -> PartialVMResult<()>;
}

pub(crate) struct DelayedFieldsExtension<'a> {
    pub(crate) delayed_fields_count: RefCell<usize>,
    pub(crate) mapping: Option<&'a dyn ValueToIdentifierMapping>,
}

pub struct ValueSerDeContext<'a> {
    #[allow(dead_code)]
    pub(crate) function_extension: Option<&'a dyn FunctionExtension>,
    pub(crate) delayed_fields_extension: Option<DelayedFieldsExtension<'a>>,
}

impl<'a> ValueSerDeContext<'a> {
    /// Default (de)serializer that disallows delayed fields.
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            function_extension: None,
            delayed_fields_extension: None,
        }
    }

    /// Custom (de)serializer such that:
    ///   1. when serializing, the delayed value is replaced with a concrete value instance and
    ///      serialized instead;
    ///   2. when deserializing, the concrete value instance is replaced with a delayed id.
    pub fn new_with_delayed_fields_replacement(mapping: &'a dyn ValueToIdentifierMapping) -> Self {
        let delayed_fields_extension = Some(DelayedFieldsExtension {
            delayed_fields_count: RefCell::new(0),
            mapping: Some(mapping),
        });
        Self {
            function_extension: None,
            delayed_fields_extension,
        }
    }

    /// Custom (de)serializer that allows delayed values to be (de)serialized as is. This means
    /// that when a delayed value is serialized, the deserialization must construct the delayed
    /// value back.
    pub fn new_with_delayed_fields_serde() -> Self {
        let delayed_fields_extension = Some(DelayedFieldsExtension {
            delayed_fields_count: RefCell::new(0),
            mapping: None,
        });
        Self {
            function_extension: None,
            delayed_fields_extension,
        }
    }

    pub fn serialize(
        &self,
        value: &Value,
        layout: &MoveTypeLayout,
    ) -> PartialVMResult<Option<Vec<u8>>> {
        let value = SerializationReadyValue {
            ctx: self,
            layout,
            value: &value.0,
        };

        match bcs::to_bytes(&value).ok() {
            Some(bytes) => Ok(Some(bytes)),
            None => {
                // Check if the error is due to too many delayed fields. If so, to be compatible
                // with the older implementation return an error.
                if self.delayed_fields_extension.as_ref().is_some_and(|ext| {
                    *ext.delayed_fields_count.borrow() > MAX_DELAYED_FIELDS_PER_RESOURCE
                }) {
                    Err(PartialVMError::new(StatusCode::TOO_MANY_DELAYED_FIELDS)
                        .with_message("Too many Delayed fields in a single resource.".to_string()))
                } else {
                    Ok(None)
                }
            },
        }
    }

    pub fn serialized_size(
        &self,
        value: &Value,
        layout: &MoveTypeLayout,
    ) -> PartialVMResult<usize> {
        let value = SerializationReadyValue {
            ctx: self,
            layout,
            value: &value.0,
        };
        bcs::serialized_size(&value).map_err(|e| {
            PartialVMError::new(StatusCode::VALUE_SERIALIZATION_ERROR).with_message(format!(
                "failed to compute serialized size of a value: {:?}",
                e
            ))
        })
    }

    pub fn deserialize(&self, bytes: &[u8], layout: &MoveTypeLayout) -> Option<Value> {
        let seed = DeserializationSeed { ctx: self, layout };
        bcs::from_bytes_seed(seed, bytes).ok()
    }
}

// TODO[agg_v2](clean): propagate up, so this value is controlled by the gas schedule version.
// Temporarily limit the number of delayed fields per resource,
// until proper charges are implemented.
pub const MAX_DELAYED_FIELDS_PER_RESOURCE: usize = 10;

/// Allow conversion between values and identifiers (delayed values). For example,
/// this trait can be implemented to fetch a concrete Move value from the global
/// state based on the identifier stored inside a delayed value.
pub trait ValueToIdentifierMapping {
    fn value_to_identifier(
        &self,
        // We need kind to distinguish between aggregators and snapshots
        // of the same type.
        kind: &IdentifierMappingKind,
        layout: &MoveTypeLayout,
        value: Value,
    ) -> PartialVMResult<DelayedFieldID>;

    fn identifier_to_value(
        &self,
        layout: &MoveTypeLayout,
        identifier: DelayedFieldID,
    ) -> PartialVMResult<Value>;
}
