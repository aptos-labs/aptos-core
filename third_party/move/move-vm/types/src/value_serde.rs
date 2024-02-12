// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    delayed_values::delayed_field_id::{
        DelayedFieldID, ExtractUniqueIndex, ExtractWidth, TryFromMoveValue, TryIntoMoveValue,
    },
    values::{DeserializationSeed, SerializationReadyValue, Value},
};
use move_binary_format::errors::PartialVMResult;
use move_core_types::value::{IdentifierMappingKind, MoveTypeLayout};
use serde::{
    de::{DeserializeSeed, Error as DeError},
    ser::Error as SerError,
    Deserializer, Serialize, Serializer,
};

pub trait CustomDeserializer {
    fn custom_deserialize<'d, D: Deserializer<'d>>(
        &self,
        deserializer: D,
        kind: &IdentifierMappingKind,
        layout: &MoveTypeLayout,
    ) -> Result<Value, D::Error>;
}

pub trait CustomSerializer {
    fn custom_serialize<S: Serializer>(
        &self,
        serializer: S,
        kind: &IdentifierMappingKind,
        layout: &MoveTypeLayout,
        id: DelayedFieldID,
    ) -> Result<S::Ok, S::Error>;
}

/// Custom (de)serializer which allows delayed values to be (de)serialized as
/// is. This means that when a delayed value is serialized, the deserialization
/// must construct the delayed value back.
pub struct RelaxedCustomSerDe;

impl CustomDeserializer for RelaxedCustomSerDe {
    fn custom_deserialize<'d, D: Deserializer<'d>>(
        &self,
        deserializer: D,
        kind: &IdentifierMappingKind,
        layout: &MoveTypeLayout,
    ) -> Result<Value, D::Error> {
        let value = DeserializationSeed {
            custom_deserializer: None::<&RelaxedCustomSerDe>,
            layout,
        }
        .deserialize(deserializer)?;
        let (id, _width) =
            DelayedFieldID::try_from_move_value(layout, value, &()).map_err(|_| {
                D::Error::custom(format!(
                    "Custom deserialization failed for {:?} with layout {}",
                    kind, layout
                ))
            })?;
        Ok(Value::delayed_value(id))
    }
}

impl CustomSerializer for RelaxedCustomSerDe {
    fn custom_serialize<S: Serializer>(
        &self,
        serializer: S,
        kind: &IdentifierMappingKind,
        layout: &MoveTypeLayout,
        id: DelayedFieldID,
    ) -> Result<S::Ok, S::Error> {
        let value = id.try_into_move_value(layout).map_err(|_| {
            S::Error::custom(format!(
                "Custom serialization failed for {:?} with layout {}",
                kind, layout
            ))
        })?;
        SerializationReadyValue {
            custom_serializer: None::<&RelaxedCustomSerDe>,
            layout,
            value: &value.0,
        }
        .serialize(serializer)
    }
}

pub fn deserialize_and_allow_delayed_values(
    bytes: &[u8],
    layout: &MoveTypeLayout,
) -> Option<Value> {
    let native_deserializer = RelaxedCustomSerDe;
    let seed = DeserializationSeed {
        custom_deserializer: Some(&native_deserializer),
        layout,
    };
    bcs::from_bytes_seed(seed, bytes).ok()
}

pub fn serialize_and_allow_delayed_values(
    value: &Value,
    layout: &MoveTypeLayout,
) -> Option<Vec<u8>> {
    let native_serializer = RelaxedCustomSerDe;
    let value = SerializationReadyValue {
        custom_serializer: Some(&native_serializer),
        layout,
        value: &value.0,
    };
    bcs::to_bytes(&value).ok()
}

/// Allow conversion between values and identifiers (delayed values). For example,
/// this trait can be implemented to fetch a concrete Move value from the global
/// state based on the identifier stored inside a delayed value.
pub trait ValueToIdentifierMapping {
    type Identifier;

    fn value_to_identifier(
        &self,
        // We need kind to distinguish between aggregators and snapshots
        // of the same type.
        kind: &IdentifierMappingKind,
        layout: &MoveTypeLayout,
        value: Value,
    ) -> PartialVMResult<Self::Identifier>;

    fn identifier_to_value(
        &self,
        layout: &MoveTypeLayout,
        identifier: Self::Identifier,
    ) -> PartialVMResult<Value>;
}

/// Custom (de)serializer such that:
///   1. when encountering a delayed value, ir uses its id to replace it with a concrete
///      value instance and serialize it instead;
///   2. when deserializing, the concrete value instance is replaced with a delayed value.
pub struct CustomSerDeWithExchange<'a, I: From<u64> + ExtractWidth + ExtractUniqueIndex> {
    mapping: &'a dyn ValueToIdentifierMapping<Identifier = I>,
}

impl<'a, I: From<u64> + ExtractWidth + ExtractUniqueIndex> CustomSerializer
    for CustomSerDeWithExchange<'a, I>
{
    fn custom_serialize<S: Serializer>(
        &self,
        serializer: S,
        _kind: &IdentifierMappingKind,
        layout: &MoveTypeLayout,
        sized_id: DelayedFieldID,
    ) -> Result<S::Ok, S::Error> {
        let value = self
            .mapping
            .identifier_to_value(layout, sized_id.as_u64().into())
            .map_err(|e| S::Error::custom(format!("{}", e)))?;
        SerializationReadyValue {
            custom_serializer: None::<&RelaxedCustomSerDe>,
            layout,
            value: &value.0,
        }
        .serialize(serializer)
    }
}

impl<'a, I: From<u64> + ExtractWidth + ExtractUniqueIndex> CustomDeserializer
    for CustomSerDeWithExchange<'a, I>
{
    fn custom_deserialize<'d, D: Deserializer<'d>>(
        &self,
        deserializer: D,
        kind: &IdentifierMappingKind,
        layout: &MoveTypeLayout,
    ) -> Result<Value, D::Error> {
        let value = DeserializationSeed {
            custom_deserializer: None::<&RelaxedCustomSerDe>,
            layout,
        }
        .deserialize(deserializer)?;
        let id = self
            .mapping
            .value_to_identifier(kind, layout, value)
            .map_err(|e| D::Error::custom(format!("{}", e)))?;
        Ok(Value::delayed_value(DelayedFieldID::new_with_width(
            id.extract_unique_index(),
            id.extract_width(),
        )))
    }
}

pub fn deserialize_and_replace_values_with_ids<I: From<u64> + ExtractWidth + ExtractUniqueIndex>(
    bytes: &[u8],
    layout: &MoveTypeLayout,
    mapping: &impl ValueToIdentifierMapping<Identifier = I>,
) -> Option<Value> {
    let custom_deserializer = CustomSerDeWithExchange { mapping };
    let seed = DeserializationSeed {
        custom_deserializer: Some(&custom_deserializer),
        layout,
    };
    bcs::from_bytes_seed(seed, bytes).ok()
}

pub fn serialize_and_replace_ids_with_values<I: From<u64> + ExtractWidth + ExtractUniqueIndex>(
    value: &Value,
    layout: &MoveTypeLayout,
    mapping: &impl ValueToIdentifierMapping<Identifier = I>,
) -> Option<Vec<u8>> {
    let custom_serializer = CustomSerDeWithExchange { mapping };
    let value = SerializationReadyValue {
        custom_serializer: Some(&custom_serializer),
        layout,
        value: &value.0,
    };
    bcs::to_bytes(&value).ok()
}
