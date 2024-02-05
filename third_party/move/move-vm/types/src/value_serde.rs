// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::values::{DeserializationSeed, SerializationReadyValue, SizedID, Value};
use move_binary_format::errors::PartialVMResult;
use move_core_types::value::{IdentifierMappingKind, MoveTypeLayout};
use serde::{
    de::{DeserializeSeed, Error as DeError},
    ser::Error as SerError,
    Deserialize, Deserializer, Serialize, Serializer,
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
        id: SizedID,
    ) -> Result<S::Ok, S::Error>;
}

pub struct NativeValueSimpleSerDe;

impl CustomDeserialize for NativeValueSimpleSerDe {
    fn custom_deserialize<'d, D: Deserializer<'d>>(
        &self,
        deserializer: D,
        _tag: &IdentifierMappingKind,
        _layout: &MoveTypeLayout,
    ) -> Result<Value, D::Error> {
        let value = u64::deserialize(deserializer)?;
        Ok(Value::native_value(value.into()))
    }
}

impl CustomSerialize for NativeValueSimpleSerDe {
    fn custom_serialize<S: Serializer>(
        &self,
        serializer: S,
        _tag: &IdentifierMappingKind,
        _layout: &MoveTypeLayout,
        id: SizedID,
    ) -> Result<S::Ok, S::Error> {
        serializer.serialize_u64(id.into())
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
    fn value_to_identifier(
        &self,
        // We need kind to distinguish between aggregators and snapshots
        // of the same type.
        kind: &IdentifierMappingKind,
        layout: &MoveTypeLayout,
        value: Value,
    ) -> PartialVMResult<SizedID>;

    fn identifier_to_value(
        &self,
        layout: &MoveTypeLayout,
        identifier: SizedID,
    ) -> PartialVMResult<Value>;
}

pub struct NativeValueSerDeWithExchange<'a> {
    mapping: &'a dyn ValueToIdentifierMapping,
}

impl<'a> CustomSerialize for NativeValueSerDeWithExchange<'a> {
    fn custom_serialize<S: Serializer>(
        &self,
        serializer: S,
        _tag: &IdentifierMappingKind,
        layout: &MoveTypeLayout,
        id: SizedID,
    ) -> Result<S::Ok, S::Error> {
        let value = self
            .mapping
            .identifier_to_value(layout, id)
            .map_err(|e| S::Error::custom(format!("{}", e)))?;
        SerializationReadyValue {
            native_serializer: None::<&NativeValueSimpleSerDe>,
            layout,
            value: &value.0,
        }
        .serialize(serializer)
    }
}

impl<'a> CustomDeserialize for NativeValueSerDeWithExchange<'a> {
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
        let size_id = self
            .mapping
            .value_to_identifier(tag, layout, value)
            .map_err(|e| D::Error::custom(format!("{}", e)))?;
        Ok(Value::native_value(size_id))
    }
}

pub fn deserialize_and_replace_values_with_ids(
    bytes: &[u8],
    layout: &MoveTypeLayout,
    mapping: &impl ValueToIdentifierMapping,
) -> Option<Value> {
    let native_deserializer = NativeValueSerDeWithExchange { mapping };
    let seed = DeserializationSeed {
        native_deserializer: Some(&native_deserializer),
        layout,
    };
    bcs::from_bytes_seed(seed, bytes).ok()
}

pub fn serialize_and_replace_ids_with_values(
    value: &Value,
    layout: &MoveTypeLayout,
    mapping: &impl ValueToIdentifierMapping,
) -> Option<Vec<u8>> {
    let native_serializer = NativeValueSerDeWithExchange { mapping };
    let value = SerializationReadyValue {
        native_serializer: Some(&native_serializer),
        layout,
        value: &value.0,
    };
    bcs::to_bytes(&value).ok()
}
