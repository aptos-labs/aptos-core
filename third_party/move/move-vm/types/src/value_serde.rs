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

pub trait CustomDeserialize {
    fn custom_deserialize<'d, D: Deserializer<'d>>(
        &self,
        deserializer: D,
        tag: &IdentifierMappingKind,
        layout: &MoveTypeLayout,
    ) -> Result<Value, D::Error>;
}

pub trait CustomSerialize {
    fn custom_serialize<S: Serializer>(
        &self,
        serializer: S,
        tag: &IdentifierMappingKind,
        layout: &MoveTypeLayout,
        sized_id: DelayedFieldID,
    ) -> Result<S::Ok, S::Error>;
}

pub struct NativeValueSimpleSerDe;

impl CustomDeserialize for NativeValueSimpleSerDe {
    fn custom_deserialize<'d, D: Deserializer<'d>>(
        &self,
        deserializer: D,
        _tag: &IdentifierMappingKind,
        layout: &MoveTypeLayout,
    ) -> Result<Value, D::Error> {
        let value = DeserializationSeed {
            native_deserializer: None::<&NativeValueSimpleSerDe>,
            layout,
        }
        .deserialize(deserializer)?;
        let (id, _width) = DelayedFieldID::try_from_move_value(layout, value, &())
            .map_err(|_| D::Error::custom("Failed deserialization"))?;
        Ok(Value::native_value(id))
    }
}

impl CustomSerialize for NativeValueSimpleSerDe {
    fn custom_serialize<S: Serializer>(
        &self,
        serializer: S,
        _tag: &IdentifierMappingKind,
        layout: &MoveTypeLayout,
        sized_id: DelayedFieldID,
    ) -> Result<S::Ok, S::Error> {
        let value = sized_id
            .try_into_move_value(layout)
            .map_err(|_| S::Error::custom("Failed serialization"))?;
        SerializationReadyValue {
            native_serializer: None::<&NativeValueSimpleSerDe>,
            layout,
            value: &value.0,
        }
        .serialize(serializer)
    }
}

pub fn deserialize_and_allow_native_values(bytes: &[u8], layout: &MoveTypeLayout) -> Option<Value> {
    let native_deserializer = NativeValueSimpleSerDe;
    let seed = DeserializationSeed {
        native_deserializer: Some(&native_deserializer),
        layout,
    };
    bcs::from_bytes_seed(seed, bytes).ok()
}

pub fn serialize_and_allow_native_values(
    value: &Value,
    layout: &MoveTypeLayout,
) -> Option<Vec<u8>> {
    let native_serializer = NativeValueSimpleSerDe;
    let value = SerializationReadyValue {
        native_serializer: Some(&native_serializer),
        layout,
        value: &value.0,
    };
    bcs::to_bytes(&value).ok()
}

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

pub struct NativeValueSerDeWithExchange<'a, I: From<u64> + ExtractWidth + ExtractUniqueIndex> {
    mapping: &'a dyn ValueToIdentifierMapping<Identifier = I>,
}

impl<'a, I: From<u64> + ExtractWidth + ExtractUniqueIndex> CustomSerialize
    for NativeValueSerDeWithExchange<'a, I>
{
    fn custom_serialize<S: Serializer>(
        &self,
        serializer: S,
        _tag: &IdentifierMappingKind,
        layout: &MoveTypeLayout,
        sized_id: DelayedFieldID,
    ) -> Result<S::Ok, S::Error> {
        let value = self
            .mapping
            // FIXME
            .identifier_to_value(layout, sized_id.as_u64().into())
            .map_err(|e| S::Error::custom(format!("{}", e)))?;
        SerializationReadyValue {
            native_serializer: None::<&NativeValueSimpleSerDe>,
            layout,
            value: &value.0,
        }
        .serialize(serializer)
    }
}

impl<'a, I: From<u64> + ExtractWidth + ExtractUniqueIndex> CustomDeserialize
    for NativeValueSerDeWithExchange<'a, I>
{
    fn custom_deserialize<'d, D: Deserializer<'d>>(
        &self,
        deserializer: D,
        tag: &IdentifierMappingKind,
        layout: &MoveTypeLayout,
    ) -> Result<Value, D::Error> {
        let value = DeserializationSeed {
            native_deserializer: None::<&NativeValueSimpleSerDe>,
            layout,
        }
        .deserialize(deserializer)?;
        let id = self
            .mapping
            .value_to_identifier(tag, layout, value)
            .map_err(|e| D::Error::custom(format!("{}", e)))?;
        Ok(Value::native_value(DelayedFieldID::new_with_width(
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
    let native_deserializer = NativeValueSerDeWithExchange { mapping };
    let seed = DeserializationSeed {
        native_deserializer: Some(&native_deserializer),
        layout,
    };
    bcs::from_bytes_seed(seed, bytes).ok()
}

pub fn serialize_and_replace_ids_with_values<I: From<u64> + ExtractWidth + ExtractUniqueIndex>(
    value: &Value,
    layout: &MoveTypeLayout,
    mapping: &impl ValueToIdentifierMapping<Identifier = I>,
) -> Option<Vec<u8>> {
    let native_serializer = NativeValueSerDeWithExchange { mapping };
    let value = SerializationReadyValue {
        native_serializer: Some(&native_serializer),
        layout,
        value: &value.0,
    };
    bcs::to_bytes(&value).ok()
}
