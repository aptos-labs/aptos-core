// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::Address;
use move_core_types::{
    identifier::Identifier,
    language_storage::{StructTag, TypeTag},
};
use resource_viewer::{AnnotatedMoveStruct, AnnotatedMoveValue};

use serde::{de::Error as _, Deserialize, Deserializer, Serialize, Serializer};
use std::{collections::BTreeMap, convert::From, result::Result};

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct MoveResource {
    #[serde(rename = "type")]
    pub typ: String,
    pub type_tag: MoveResourceType,
    pub value: MoveStructValue,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MoveResourceType {
    Struct(MoveStructTag),
}

impl From<AnnotatedMoveStruct> for MoveResource {
    fn from(s: AnnotatedMoveStruct) -> Self {
        Self {
            typ: s.type_.to_string(),
            type_tag: MoveResourceType::Struct(MoveStructTag::from(s.type_.clone())),
            value: MoveStructValue::from(s),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct U64(u64);

impl U64 {
    pub fn into_inner(&self) -> u64 {
        self.0
    }
}

impl From<u64> for U64 {
    fn from(d: u64) -> Self {
        Self(d)
    }
}

impl Serialize for U64 {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.to_string().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for U64 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = <String>::deserialize(deserializer)?;
        let data = s.parse::<u64>().map_err(D::Error::custom)?;

        Ok(U64(data))
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct U128(u128);

impl U128 {
    pub fn into_inner(&self) -> u128 {
        self.0
    }
}

impl From<u128> for U128 {
    fn from(d: u128) -> Self {
        Self(d)
    }
}

impl Serialize for U128 {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.to_string().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for U128 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = <String>::deserialize(deserializer)?;
        let data = s.parse::<u128>().map_err(D::Error::custom)?;

        Ok(U128(data))
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct HexEncodedBytes(Vec<u8>);

impl Serialize for HexEncodedBytes {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        format!("0x{}", &hex::encode(&self.0)).serialize(serializer)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MoveStructValue(BTreeMap<Identifier, MoveValue>);

impl Serialize for MoveStructValue {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}

impl From<AnnotatedMoveStruct> for MoveStructValue {
    fn from(s: AnnotatedMoveStruct) -> Self {
        let mut map = BTreeMap::new();
        for (id, val) in s.value {
            map.insert(id, MoveValue::from(val));
        }
        Self(map)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum MoveValue {
    U8(u8),
    U64(U64),
    U128(U128),
    Bool(bool),
    Address(Address),
    Vector(Vec<MoveValue>),
    Bytes(HexEncodedBytes),
    Struct(MoveStructValue),
}

impl From<AnnotatedMoveValue> for MoveValue {
    fn from(val: AnnotatedMoveValue) -> Self {
        match val {
            AnnotatedMoveValue::U8(v) => MoveValue::U8(v),
            AnnotatedMoveValue::U64(v) => MoveValue::U64(U64(v)),
            AnnotatedMoveValue::U128(v) => MoveValue::U128(U128(v)),
            AnnotatedMoveValue::Bool(v) => MoveValue::Bool(v),
            AnnotatedMoveValue::Address(v) => MoveValue::Address(Address::new(v)),
            AnnotatedMoveValue::Vector(_, vals) => {
                MoveValue::Vector(vals.into_iter().map(MoveValue::from).collect())
            }
            AnnotatedMoveValue::Bytes(v) => MoveValue::Bytes(HexEncodedBytes(v)),
            AnnotatedMoveValue::Struct(v) => MoveValue::Struct(MoveStructValue::from(v)),
        }
    }
}

impl Serialize for MoveValue {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match &self {
            MoveValue::U8(v) => v.serialize(serializer),
            MoveValue::U64(v) => v.serialize(serializer),
            MoveValue::U128(v) => v.serialize(serializer),
            MoveValue::Bool(v) => v.serialize(serializer),
            MoveValue::Address(v) => v.serialize(serializer),
            MoveValue::Vector(v) => v.serialize(serializer),
            MoveValue::Bytes(v) => v.serialize(serializer),
            MoveValue::Struct(v) => v.serialize(serializer),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct MoveStructTag {
    pub address: Address,
    pub module: Identifier,
    pub name: Identifier,
    pub type_params: Vec<MoveTypeTag>,
}

impl From<StructTag> for MoveStructTag {
    fn from(tag: StructTag) -> Self {
        Self {
            address: Address::new(tag.address),
            module: tag.module,
            name: tag.name,
            type_params: tag.type_params.into_iter().map(MoveTypeTag::from).collect(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MoveTypeTag {
    Bool,
    U8,
    U64,
    U128,
    Address,
    Signer,
    Vector { items: Box<MoveTypeTag> },
    Struct(MoveStructTag),
}

impl From<TypeTag> for MoveTypeTag {
    fn from(tag: TypeTag) -> Self {
        match tag {
            TypeTag::Bool => MoveTypeTag::Bool,
            TypeTag::U8 => MoveTypeTag::U8,
            TypeTag::U64 => MoveTypeTag::U64,
            TypeTag::U128 => MoveTypeTag::U128,
            TypeTag::Address => MoveTypeTag::Address,
            TypeTag::Signer => MoveTypeTag::Signer,
            TypeTag::Vector(v) => MoveTypeTag::Vector {
                items: Box::new(MoveTypeTag::from(*v)),
            },
            TypeTag::Struct(v) => MoveTypeTag::Struct(MoveStructTag::from(v)),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{MoveResource, MoveTypeTag, U128, U64};

    use diem_types::account_address::AccountAddress;
    use move_binary_format::file_format::AbilitySet;
    use move_core_types::{
        identifier::Identifier,
        language_storage::{StructTag, TypeTag},
    };
    use resource_viewer::{AnnotatedMoveStruct, AnnotatedMoveValue};

    use serde_json::{json, to_value, Value};
    use std::boxed::Box;

    #[test]
    fn test_serialize_move_type_tag() {
        use TypeTag::*;
        fn assert_serialize(t: TypeTag, expected: Value) {
            let value = to_value(MoveTypeTag::from(t)).unwrap();
            assert_eq!(value, expected, "{}", pretty(&value))
        }
        assert_serialize(Bool, json!({"type": "bool"}));
        assert_serialize(U8, json!({"type": "u8"}));
        assert_serialize(U64, json!({"type": "u64"}));
        assert_serialize(U128, json!({"type": "u128"}));
        assert_serialize(Address, json!({"type": "address"}));
        assert_serialize(Signer, json!({"type": "signer"}));

        assert_serialize(
            Vector(Box::new(U8)),
            json!({"type": "vector", "items": {"type": "u8"}}),
        );

        assert_serialize(
            Struct(create_nested_struct()),
            json!({
                "type": "struct",
                "address": "0x1",
                "module": "Home",
                "name": "ABC",
                "type_params": [
                    {
                        "type": "address"
                    },
                    {
                        "type": "struct",
                        "address": "0x1",
                        "module": "Account",
                        "name": "Base",
                        "type_params": [
                            {
                                "type": "u128"
                            },
                            {
                                "type": "vector",
                                "items": {
                                    "type": "u64"
                                }
                            },
                            {
                                "type": "vector",
                                "items": {
                                    "type": "struct",
                                    "address": "0x1",
                                    "module": "Type",
                                    "name": "String",
                                    "type_params": []
                                }
                            },
                            {
                                "type": "struct",
                                "address": "0x1",
                                "module": "Type",
                                "name": "String",
                                "type_params": []
                            }
                        ]
                    }
                ]
            }),
        );
    }

    #[test]
    fn test_serialize_move_resource() {
        use AnnotatedMoveValue::*;

        let res = MoveResource::from(annotated_move_struct(
            "Values",
            vec![
                (identifier("field_u8"), U8(7)),
                (identifier("field_u64"), U64(7)),
                (identifier("field_u128"), U128(7)),
                (identifier("field_bool"), Bool(true)),
                (identifier("field_address"), Address(address("0xdd"))),
                (
                    identifier("field_vector"),
                    Vector(TypeTag::U128, vec![U128(128)]),
                ),
                (identifier("field_bytes"), Bytes(vec![9, 9])),
                (
                    identifier("field_struct"),
                    Struct(annotated_move_struct(
                        "Nested",
                        vec![(
                            identifier("nested_vector"),
                            Vector(
                                TypeTag::Struct(type_struct("Host")),
                                vec![Struct(annotated_move_struct(
                                    "String",
                                    vec![
                                        (identifier("address1"), Address(address("0x0"))),
                                        (identifier("address2"), Address(address("0x123"))),
                                    ],
                                ))],
                            ),
                        )],
                    )),
                ),
            ],
        ));
        let value = to_value(&res).unwrap();
        assert_eq!(
            value,
            json!({
                "type": "0x1::Type::Values",
                "type_tag": {
                    "type": "struct",
                    "address": "0x1",
                    "module": "Type",
                    "name": "Values",
                    "type_params": []
                },
                "value": {
                    "field_u8": 7,
                    "field_u64": "7",
                    "field_u128": "7",
                    "field_bool": true,
                    "field_address": "0xdd",
                    "field_vector": ["128"],
                    "field_bytes": "0x0909",
                    "field_struct": {"nested_vector": [{"address1": "0x0", "address2": "0x123"}]},
                }
            }),
            "{}",
            pretty(&value)
        );
    }

    #[test]
    fn test_serialize_move_resource_with_address_0x0() {
        let res = MoveResource::from(annotated_move_struct(
            "Values",
            vec![(
                identifier("address_0x0"),
                AnnotatedMoveValue::Address(address("0x0")),
            )],
        ));
        let value = to_value(&res).unwrap();
        assert_eq!(
            value,
            json!({
                "type": "0x1::Type::Values",
                "type_tag": {
                    "type": "struct",
                    "address": "0x1",
                    "module": "Type",
                    "name": "Values",
                    "type_params": []
                },
                "value": {
                    "address_0x0": "0x0",
                }
            }),
            "{}",
            pretty(&value)
        );
    }

    #[test]
    fn test_serialize_deserialize_u64() {
        let val = to_value(&U64::from(u64::MAX)).unwrap();
        assert_eq!(val, json!(u64::MAX.to_string()));

        let data: U64 = serde_json::from_value(json!(u64::MAX.to_string())).unwrap();
        assert_eq!(data.into_inner(), u64::MAX);
    }

    #[test]
    fn test_serialize_deserialize_u128() {
        let val = to_value(&U128::from(u128::MAX)).unwrap();
        assert_eq!(val, json!(u128::MAX.to_string()));

        let data: U128 = serde_json::from_value(json!(u128::MAX.to_string())).unwrap();
        assert_eq!(data.into_inner(), u128::MAX);
    }

    fn create_nested_struct() -> StructTag {
        let account = create_generic_type_struct();
        StructTag {
            address: address("0x1"),
            module: identifier("Home"),
            name: identifier("ABC"),
            type_params: vec![TypeTag::Address, TypeTag::Struct(account)],
        }
    }

    fn create_generic_type_struct() -> StructTag {
        StructTag {
            address: address("0x1"),
            module: identifier("Account"),
            name: identifier("Base"),
            type_params: vec![
                TypeTag::U128,
                TypeTag::Vector(Box::new(TypeTag::U64)),
                TypeTag::Vector(Box::new(TypeTag::Struct(type_struct("String")))),
                TypeTag::Struct(type_struct("String")),
            ],
        }
    }

    fn type_struct(t: &str) -> StructTag {
        StructTag {
            address: address("0x1"),
            module: identifier("Type"),
            name: identifier(t),
            type_params: vec![],
        }
    }

    fn address(hex: &str) -> AccountAddress {
        AccountAddress::from_hex_literal(hex).unwrap()
    }

    fn annotated_move_struct(
        typ: &str,
        values: Vec<(Identifier, AnnotatedMoveValue)>,
    ) -> AnnotatedMoveStruct {
        AnnotatedMoveStruct {
            abilities: AbilitySet::EMPTY,
            type_: type_struct(typ),
            value: values,
        }
    }

    fn identifier(id: &str) -> Identifier {
        Identifier::new(id).unwrap()
    }

    fn pretty(val: &Value) -> String {
        serde_json::to_string_pretty(val).unwrap()
    }
}
