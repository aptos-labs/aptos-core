// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::values::{DeserializationSeed, SerializationReadyValue, SizedID, Value};
use move_core_types::value::{LayoutTag, MoveTypeLayout};
use serde::{Deserialize, Deserializer, Serializer};

pub trait CustomDeserialize {
    fn custom_deserialize<D: serde::Deserializer>(
        &self,
        deserializer: D,
        tag: &LayoutTag,
        layout: &MoveTypeLayout,
    ) -> Result<Value, D::Error>;
}

pub trait CustomSerialize {
    fn custom_serialize<S: serde::Serializer>(
        &self,
        serializer: S,
        tag: &LayoutTag,
        layout: &MoveTypeLayout,
        id: SizedID,
    ) -> Result<S::Ok, S::Error>;
}

pub struct NativeValueSimpleSerDe;

impl CustomDeserialize for NativeValueSimpleSerDe {
    fn custom_deserialize<D: Deserializer>(
        &self,
        deserializer: D,
        _tag: &LayoutTag,
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
        _tag: &LayoutTag,
        _layout: &MoveTypeLayout,
        id: SizedID,
    ) -> Result<S::Ok, S::Error> {
        let value: u64 = id.into();
        Ok(serializer.serialize_u64(value))
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

pub struct NativeValueSerDeWithExchange {
    
}
