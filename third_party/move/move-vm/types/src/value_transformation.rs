// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::values::{AnnotatedValue, SeedWrapper, Value};
use move_binary_format::errors::PartialVMError;
use move_core_types::value::{IdentifierMappingKind, LayoutTag, MoveTypeLayout};
use std::fmt::{Display, Formatter};

/// Type for errors occurred while transforming values.
#[derive(Debug)]
pub struct TransformationError(pub String);

impl TransformationError {
    pub fn new(s: impl ToString) -> Self {
        Self(s.to_string())
    }
}

impl Display for TransformationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Error during value transformation: {}.", self.0)
    }
}

impl From<PartialVMError> for TransformationError {
    fn from(err: PartialVMError) -> Self {
        Self::new(format!("{}", err))
    }
}

pub type TransformationResult<T> = Result<T, TransformationError>;

/// Trait to allow transforming one Move value into another Move value
/// during serialization and deserialization.
/// The transformation only takes place if `matches` returns true.
pub trait OnTagTransformation {
    fn matches(&self, tag: &LayoutTag) -> bool;

    // Called after `value_to_transform` has been deserialized. The matched
    // tag is passed to allow custom implementations based on tag values,
    // without the need for a new tag.
    fn post_deserialization_transform(
        &self,
        matched_tag: &LayoutTag,
        layout: &MoveTypeLayout,
        value_to_transform: Value,
    ) -> TransformationResult<Value>;

    // Called before `value_to_transform` is serialized.
    fn pre_serialization_transform(
        &self,
        matched_tag: &LayoutTag,
        layout: &MoveTypeLayout,
        value_to_transform: Value,
    ) -> TransformationResult<Value>;
}

/// Allows the replacement of aggregator and snapshot values with unique
/// identifiers. The identifiers are used after deserialization, and have
/// an identical layout to the values they replace. Before serialization,
/// the identifiers are replaced with the actual values.
pub trait ValueToIdentifierMapping {
    fn value_to_identifier(
        &self,
        // We need kind to distinguish between aggregators and snapshots
        // of the same type.
        kind: &IdentifierMappingKind,
        layout: &MoveTypeLayout,
        value: Value,
    ) -> TransformationResult<Value>;

    fn identifier_to_value(
        &self,
        layout: &MoveTypeLayout,
        identifier: Value,
    ) -> TransformationResult<Value>;
}

impl<T: ValueToIdentifierMapping> OnTagTransformation for T {
    fn matches(&self, tag: &LayoutTag) -> bool {
        matches!(tag, LayoutTag::IdentifierMapping(_))
    }

    fn post_deserialization_transform(
        &self,
        matched_tag: &LayoutTag,
        layout: &MoveTypeLayout,
        value_to_transform: Value,
    ) -> TransformationResult<Value> {
        debug_assert!(matches!(matched_tag, LayoutTag::IdentifierMapping(_)));
        let LayoutTag::IdentifierMapping(kind) = matched_tag;
        self.value_to_identifier(kind, layout, value_to_transform)
    }

    fn pre_serialization_transform(
        &self,
        matched_tag: &LayoutTag,
        layout: &MoveTypeLayout,
        value_to_transform: Value,
    ) -> TransformationResult<Value> {
        debug_assert!(matches!(matched_tag, LayoutTag::IdentifierMapping(_)));
        self.identifier_to_value(layout, value_to_transform)
    }
}

pub fn deserialize_and_replace_values_with_ids(
    bytes: &[u8],
    layout: &MoveTypeLayout,
    mapping: &impl ValueToIdentifierMapping,
) -> Option<Value> {
    let seed = SeedWrapper {
        transformation: Some(mapping),
        layout,
    };
    bcs::from_bytes_seed(seed, bytes).ok()
}

pub fn serialize_and_replace_ids_with_values(
    value: &Value,
    layout: &MoveTypeLayout,
    mapping: &impl ValueToIdentifierMapping,
) -> Option<Vec<u8>> {
    let value = AnnotatedValue {
        transformation: Some(mapping),
        layout,
        val: &value.0,
    };
    bcs::to_bytes(&value).ok()
}
