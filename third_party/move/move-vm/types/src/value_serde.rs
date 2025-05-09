// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    delayed_values::delayed_field_id::DelayedFieldID,
    values::{
        AbstractFunction, DeserializationSeed, SerializationReadyValue, SerializedFunctionData,
        Value,
    },
};
#[cfg(test)]
use mockall::automock;
use move_binary_format::errors::{PartialVMError, PartialVMResult};
use move_core_types::{
    value::{IdentifierMappingKind, MoveTypeLayout},
    vm_status::StatusCode,
};
use std::cell::RefCell;

/// An extension to (de)serialize information about function values.
#[cfg_attr(test, automock)]
pub trait FunctionValueExtension {
    /// Create an implementation of an `AbstractFunction` from the serialization data.
    fn create_from_serialization_data(
        &self,
        data: SerializedFunctionData,
    ) -> PartialVMResult<Box<dyn AbstractFunction>>;

    /// Get serialization data from an `AbstractFunction`.
    fn get_serialization_data(
        &self,
        fun: &dyn AbstractFunction,
    ) -> PartialVMResult<SerializedFunctionData>;
}

/// An extension to (de)serializer to lookup information about delayed fields.
pub(crate) struct DelayedFieldsExtension<'a> {
    /// Number of delayed fields (de)serialized, capped.
    pub(crate) delayed_fields_count: RefCell<usize>,
    /// Optional mapping to ids/values. The mapping is used to replace ids with values at
    /// serialization time and values with ids at deserialization time. If [None], ids and values
    /// are serialized as is.
    pub(crate) mapping: Option<&'a dyn ValueToIdentifierMapping>,
}

impl DelayedFieldsExtension<'_> {
    // Temporarily limit the number of delayed fields per resource, until proper charges are
    // implemented.
    // TODO[agg_v2](clean):
    //   Propagate up, so this value is controlled by the gas schedule version.
    const MAX_DELAYED_FIELDS_PER_RESOURCE: usize = 10;

    /// Increments the delayed fields count, and checks if there are too many of them. If so, an
    /// error is returned.
    pub(crate) fn inc_and_check_delayed_fields_count(&self) -> PartialVMResult<()> {
        *self.delayed_fields_count.borrow_mut() += 1;
        if *self.delayed_fields_count.borrow() > Self::MAX_DELAYED_FIELDS_PER_RESOURCE {
            return Err(PartialVMError::new(StatusCode::TOO_MANY_DELAYED_FIELDS)
                .with_message("Too many Delayed fields in a single resource.".to_string()));
        }
        Ok(())
    }
}

/// A (de)serializer context for a single Move [Value], containing optional extensions. If
/// extension is not provided, but required at (de)serialization time, (de)serialization fails.
pub struct ValueSerDeContext<'a> {
    #[allow(dead_code)]
    pub(crate) function_extension: Option<&'a dyn FunctionValueExtension>,
    pub(crate) delayed_fields_extension: Option<DelayedFieldsExtension<'a>>,
    pub(crate) legacy_signer: bool,
}

impl<'a> ValueSerDeContext<'a> {
    /// Default (de)serializer that disallows delayed fields.
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            function_extension: None,
            delayed_fields_extension: None,
            legacy_signer: false,
        }
    }

    /// Serialize signer with legacy format to maintain backwards compatibility.
    pub fn with_legacy_signer(mut self) -> Self {
        self.legacy_signer = true;
        self
    }

    /// Custom (de)serializer such that supports lookup of the argument types of a function during
    /// function value deserialization.
    pub fn with_func_args_deserialization(
        mut self,
        function_extension: &'a dyn FunctionValueExtension,
    ) -> Self {
        self.function_extension = Some(function_extension);
        self
    }

    pub fn required_function_extension(&self) -> PartialVMResult<&dyn FunctionValueExtension> {
        self.function_extension.ok_or_else(|| {
            PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR).with_message(
                "require function extension context for serialization of closures".to_string(),
            )
        })
    }

    /// Returns the same extension but without allowing the delayed fields.
    pub(crate) fn clone_without_delayed_fields(&self) -> Self {
        Self {
            function_extension: self.function_extension,
            delayed_fields_extension: None,
            legacy_signer: self.legacy_signer,
        }
    }

    /// Custom (de)serializer such that:
    ///   1. when serializing, the delayed value is replaced with a concrete value instance and
    ///      serialized instead;
    ///   2. when deserializing, the concrete value instance is replaced with a delayed id.
    pub fn with_delayed_fields_replacement(
        mut self,
        mapping: &'a dyn ValueToIdentifierMapping,
    ) -> Self {
        self.delayed_fields_extension = Some(DelayedFieldsExtension {
            delayed_fields_count: RefCell::new(0),
            mapping: Some(mapping),
        });
        self
    }

    /// Custom (de)serializer that allows delayed values to be (de)serialized as is. This means
    /// that when a delayed value is serialized, the deserialization must construct the delayed
    /// value back.
    pub fn with_delayed_fields_serde(mut self) -> Self {
        self.delayed_fields_extension = Some(DelayedFieldsExtension {
            delayed_fields_count: RefCell::new(0),
            mapping: None,
        });
        self
    }

    /// Serializes a [Value] based on the provided layout. For legacy reasons, all serialization
    /// errors are mapped to [None]. If [DelayedFieldsExtension] is set, and there are too many
    /// delayed fields, an error may be returned.
    pub fn serialize(
        self,
        value: &Value,
        layout: &MoveTypeLayout,
    ) -> PartialVMResult<Option<Vec<u8>>> {
        let value = SerializationReadyValue {
            ctx: &self,
            layout,
            value: &value.0,
        };

        match bcs::to_bytes(&value).ok() {
            Some(bytes) => Ok(Some(bytes)),
            None => {
                // Check if the error is due to too many delayed fields. If so, to be compatible
                // with the older implementation return an error.
                if let Some(delayed_fields_extension) = self.delayed_fields_extension {
                    if delayed_fields_extension.delayed_fields_count.into_inner()
                        > DelayedFieldsExtension::MAX_DELAYED_FIELDS_PER_RESOURCE
                    {
                        return Err(PartialVMError::new(StatusCode::TOO_MANY_DELAYED_FIELDS)
                            .with_message(
                                "Too many Delayed fields in a single resource.".to_string(),
                            ));
                    }
                }
                Ok(None)
            },
        }
    }

    /// Returns the serialized size of a [Value] with the associated layout. All errors are mapped
    /// to [StatusCode::VALUE_SERIALIZATION_ERROR].
    pub fn serialized_size(self, value: &Value, layout: &MoveTypeLayout) -> PartialVMResult<usize> {
        let value = SerializationReadyValue {
            ctx: &self,
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

    /// Deserializes the bytes using the provided layout into a Move [Value].
    pub fn deserialize(self, bytes: &[u8], layout: &MoveTypeLayout) -> Option<Value> {
        let seed = DeserializationSeed { ctx: &self, layout };
        bcs::from_bytes_seed(seed, bytes).ok()
    }

    /// Deserializes the bytes using the provided layout into a Move [Value], returning
    /// the proper underlying error on failure.
    pub fn deserialize_or_err(
        self,
        bytes: &[u8],
        layout: &MoveTypeLayout,
    ) -> PartialVMResult<Value> {
        let seed = DeserializationSeed { ctx: &self, layout };
        bcs::from_bytes_seed(seed, bytes).map_err(|e| {
            PartialVMError::new(StatusCode::FAILED_TO_DESERIALIZE_RESOURCE)
                .with_message(format!("deserializer error: {}", e))
        })
    }
}

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
