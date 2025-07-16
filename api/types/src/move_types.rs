// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{Address, Bytecode, IdentifierWrapper, VerifyInput, VerifyInputWithRecursion};
use anyhow::{bail, format_err};
use aptos_resource_viewer::{
    AnnotatedMoveClosure, AnnotatedMoveStruct, AnnotatedMoveValue, RawMoveStruct,
};
use aptos_types::{account_config::CORE_CODE_ADDRESS, event::EventKey, transaction::Module};
use move_binary_format::{
    access::ModuleAccess,
    file_format::{CompiledModule, CompiledScript, StructTypeParameter, Visibility},
};
use move_core_types::{
    ability::{Ability, AbilitySet},
    account_address::AccountAddress,
    identifier::Identifier,
    language_storage::{FunctionParamOrReturnTag, FunctionTag, ModuleId, StructTag, TypeTag},
    parser::{parse_struct_tag, parse_type_tag},
    transaction_argument::TransactionArgument,
};
use poem_openapi::{types::Type, Enum, Object, Union};
use serde::{de::Error as _, Deserialize, Deserializer, Serialize, Serializer};
use std::{
    collections::BTreeMap,
    convert::{From, Into, TryFrom, TryInto},
    fmt,
    fmt::Display,
    result::Result,
    str::FromStr,
};

pub type ResourceGroup = BTreeMap<StructTag, Vec<u8>>;

/// A parsed Move resource
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Object)]
pub struct MoveResource {
    #[serde(rename = "type")]
    #[oai(rename = "type")]
    pub typ: MoveStructTag,
    pub data: MoveStructValue,
}

impl TryFrom<AnnotatedMoveStruct> for MoveResource {
    type Error = anyhow::Error;

    fn try_from(s: AnnotatedMoveStruct) -> anyhow::Result<Self> {
        Ok(Self {
            typ: s.ty_tag.clone().into(),
            data: s.try_into()?,
        })
    }
}

macro_rules! define_integer_type {
    ($n:ident, $t:ty, $d:literal) => {
        #[doc = $d]
        #[doc = "Encoded as a string to encode into JSON."]
        #[derive(Clone, Debug, Default, Eq, PartialEq, Copy)]
        pub struct $n(pub $t);

        impl $n {
            pub fn inner(&self) -> &$t {
                &self.0
            }
        }

        impl From<$t> for $n {
            fn from(d: $t) -> Self {
                Self(d)
            }
        }

        impl From<$n> for $t {
            fn from(d: $n) -> Self {
                d.0
            }
        }

        impl From<$n> for move_core_types::value::MoveValue {
            fn from(d: $n) -> Self {
                move_core_types::value::MoveValue::$n(d.0)
            }
        }

        impl fmt::Display for $n {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "{}", &self.0)
            }
        }

        impl Serialize for $n {
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                self.0.to_string().serialize(serializer)
            }
        }

        impl<'de> Deserialize<'de> for $n {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                let s = <String>::deserialize(deserializer)?;
                s.parse().map_err(D::Error::custom)
            }
        }

        impl FromStr for $n {
            type Err = anyhow::Error;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                let data = s.parse::<$t>().map_err(|e| {
                    format_err!(
                        "Parsing {} string {:?} failed, caused by error: {}",
                        stringify!($t),
                        s,
                        e
                    )
                })?;

                Ok($n(data))
            }
        }
    };
}

define_integer_type!(U64, u64, "A string encoded U64.");
define_integer_type!(U128, u128, "A string encoded U128.");
define_integer_type!(U256, move_core_types::u256::U256, "A string encoded U256.");

/// Hex encoded bytes to allow for having bytes represented in JSON
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HexEncodedBytes(pub Vec<u8>);

impl HexEncodedBytes {
    pub fn json(&self) -> anyhow::Result<serde_json::Value> {
        Ok(serde_json::to_value(self)?)
    }
}

impl FromStr for HexEncodedBytes {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> anyhow::Result<Self, anyhow::Error> {
        let hex_str = if let Some(hex) = s.strip_prefix("0x") {
            hex
        } else {
            s
        };
        Ok(Self(hex::decode(hex_str).map_err(|e| {
            format_err!(
                "decode hex-encoded string({:?}) failed, caused by error: {}",
                s,
                e
            )
        })?))
    }
}

impl fmt::Display for HexEncodedBytes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x{}", hex::encode(&self.0))?;
        Ok(())
    }
}

impl Serialize for HexEncodedBytes {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.to_string().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for HexEncodedBytes {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = <String>::deserialize(deserializer)?;
        s.parse().map_err(D::Error::custom)
    }
}

impl From<Vec<u8>> for HexEncodedBytes {
    fn from(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }
}

impl From<HexEncodedBytes> for Vec<u8> {
    fn from(bytes: HexEncodedBytes) -> Self {
        bytes.0
    }
}

impl From<HexEncodedBytes> for move_core_types::value::MoveValue {
    fn from(d: HexEncodedBytes) -> Self {
        move_core_types::value::MoveValue::Vector(
            d.0.into_iter()
                .map(move_core_types::value::MoveValue::U8)
                .collect(),
        )
    }
}

impl TryFrom<HexEncodedBytes> for EventKey {
    type Error = anyhow::Error;

    fn try_from(bytes: HexEncodedBytes) -> anyhow::Result<Self> {
        Ok(bcs::from_bytes(&bytes.0)?)
    }
}

impl HexEncodedBytes {
    pub fn inner(&self) -> &[u8] {
        &self.0
    }
}

/// A JSON map representation of a Move struct's or closure's inner values
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MoveStructValue(pub BTreeMap<IdentifierWrapper, serde_json::Value>);

impl TryFrom<AnnotatedMoveStruct> for MoveStructValue {
    type Error = anyhow::Error;

    fn try_from(s: AnnotatedMoveStruct) -> anyhow::Result<Self> {
        let mut map = BTreeMap::new();
        if let Some((_, name)) = s.variant_info {
            map.insert(
                IdentifierWrapper::from_str("__variant__")?,
                MoveValue::String(name.to_string()).json()?,
            );
        }
        for (id, val) in s.value {
            map.insert(id.into(), MoveValue::try_from(val)?.json()?);
        }
        Ok(Self(map))
    }
}

impl TryFrom<RawMoveStruct> for MoveStructValue {
    type Error = anyhow::Error;

    fn try_from(s: RawMoveStruct) -> anyhow::Result<Self> {
        let mut map = BTreeMap::new();
        if let Some(tag) = s.variant_info {
            map.insert(
                IdentifierWrapper::from_str("__variant_tag__")?,
                MoveValue::U16(tag).json()?,
            );
        }
        for (pos, val) in s.field_values.into_iter().enumerate() {
            map.insert(
                IdentifierWrapper::from_str(&pos.to_string())?,
                MoveValue::try_from(val)?.json()?,
            );
        }
        Ok(Self(map))
    }
}

impl TryFrom<AnnotatedMoveClosure> for MoveStructValue {
    type Error = anyhow::Error;

    fn try_from(s: AnnotatedMoveClosure) -> anyhow::Result<Self> {
        let mut map = BTreeMap::new();
        let AnnotatedMoveClosure {
            module_id,
            fun_id,
            ty_args,
            mask,
            captured,
        } = s;
        map.insert(
            IdentifierWrapper::from_str("__fun_name__")?,
            MoveValue::String(format!(
                "0x{}::{}::{}",
                module_id.address.short_str_lossless(),
                module_id.name,
                fun_id
            ))
            .json()?,
        );
        if !ty_args.is_empty() {
            map.insert(
                IdentifierWrapper::from_str("__ty_args__")?,
                MoveValue::Vector(
                    ty_args
                        .iter()
                        .map(|ty| MoveValue::String(ty.to_canonical_string()))
                        .collect(),
                )
                .json()?,
            );
        }
        map.insert(
            IdentifierWrapper::from_str("__mask__")?,
            MoveValue::String(mask.to_string()).json()?,
        );
        if !captured.is_empty() {
            map.insert(
                IdentifierWrapper::from_str("__captured__")?,
                MoveValue::Vector(
                    captured
                        .into_iter()
                        .map(MoveValue::try_from)
                        .collect::<anyhow::Result<Vec<_>>>()?,
                )
                .json()?,
            );
        }
        Ok(Self(map))
    }
}

/// An enum of the possible Move value types
#[derive(Clone, Debug, PartialEq, Union)]
pub enum MoveValue {
    /// A u8 Move type
    U8(u8),
    U16(u16),
    U32(u32),
    U64(U64),
    U128(U128),
    U256(U256),
    /// A bool Move type
    Bool(bool),
    Address(Address),
    /// A vector Move type.  May have any other [`MoveValue`] nested inside it
    Vector(Vec<MoveValue>),
    Bytes(HexEncodedBytes),
    Struct(MoveStructValue),
    /// A string Move type
    String(String),
}

impl MoveValue {
    pub fn json(&self) -> anyhow::Result<serde_json::Value> {
        Ok(serde_json::to_value(self)?)
    }

    pub fn is_utf8_string(st: &StructTag) -> bool {
        st.address == CORE_CODE_ADDRESS
            && st.name.to_string() == "String"
            && st.module.to_string() == "string"
    }

    pub fn convert_utf8_string(v: AnnotatedMoveStruct) -> anyhow::Result<MoveValue> {
        if let Some((_, AnnotatedMoveValue::Bytes(bytes))) = v.value.into_iter().next() {
            match String::from_utf8(bytes.clone()) {
                Ok(string) => Ok(MoveValue::String(string)),
                Err(_) => {
                    // There's no real use in logging the error, since this is only done on output conversion
                    Ok(MoveValue::String(format!(
                        "Unparsable utf-8 {}",
                        HexEncodedBytes(bytes)
                    )))
                },
            }
        } else {
            bail!("expect string::String, but failed to decode struct value");
        }
    }
}

impl TryFrom<AnnotatedMoveValue> for MoveValue {
    type Error = anyhow::Error;

    fn try_from(val: AnnotatedMoveValue) -> anyhow::Result<Self> {
        Ok(match val {
            AnnotatedMoveValue::U8(v) => MoveValue::U8(v),
            AnnotatedMoveValue::U16(v) => MoveValue::U16(v),
            AnnotatedMoveValue::U32(v) => MoveValue::U32(v),
            AnnotatedMoveValue::U64(v) => MoveValue::U64(U64(v)),
            AnnotatedMoveValue::U128(v) => MoveValue::U128(U128(v)),
            AnnotatedMoveValue::U256(v) => MoveValue::U256(U256(v)),
            AnnotatedMoveValue::Bool(v) => MoveValue::Bool(v),
            AnnotatedMoveValue::Address(v) => MoveValue::Address(v.into()),
            AnnotatedMoveValue::Vector(_, vals) => MoveValue::Vector(
                vals.into_iter()
                    .map(MoveValue::try_from)
                    .collect::<anyhow::Result<_>>()?,
            ),
            AnnotatedMoveValue::Bytes(v) => MoveValue::Bytes(HexEncodedBytes(v)),
            AnnotatedMoveValue::Struct(v) => {
                if MoveValue::is_utf8_string(&v.ty_tag) {
                    MoveValue::convert_utf8_string(v)?
                } else {
                    MoveValue::Struct(v.try_into()?)
                }
            },
            AnnotatedMoveValue::RawStruct(v) => MoveValue::Struct(v.try_into()?),
            AnnotatedMoveValue::Closure(c) => MoveValue::Struct(c.try_into()?),
        })
    }
}

impl From<TransactionArgument> for MoveValue {
    fn from(val: TransactionArgument) -> Self {
        match val {
            TransactionArgument::U8(v) => MoveValue::U8(v),
            TransactionArgument::U16(v) => MoveValue::U16(v),
            TransactionArgument::U32(v) => MoveValue::U32(v),
            TransactionArgument::U64(v) => MoveValue::U64(U64(v)),
            TransactionArgument::U128(v) => MoveValue::U128(U128(v)),
            TransactionArgument::U256(v) => MoveValue::U256(U256(v)),
            TransactionArgument::Bool(v) => MoveValue::Bool(v),
            TransactionArgument::Address(v) => MoveValue::Address(v.into()),
            TransactionArgument::U8Vector(bytes) => MoveValue::Bytes(HexEncodedBytes(bytes)),
            TransactionArgument::Serialized(bytes) => MoveValue::Bytes(HexEncodedBytes(bytes)),
        }
    }
}

impl Serialize for MoveValue {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match &self {
            MoveValue::U8(v) => v.serialize(serializer),
            MoveValue::U16(v) => v.serialize(serializer),
            MoveValue::U32(v) => v.serialize(serializer),
            MoveValue::U64(v) => v.serialize(serializer),
            MoveValue::U128(v) => v.serialize(serializer),
            MoveValue::U256(v) => v.serialize(serializer),
            MoveValue::Bool(v) => v.serialize(serializer),
            MoveValue::Address(v) => v.serialize(serializer),
            MoveValue::Vector(v) => v.serialize(serializer),
            MoveValue::Bytes(v) => v.serialize(serializer),
            MoveValue::Struct(v) => v.serialize(serializer),
            MoveValue::String(v) => v.serialize(serializer),
        }
    }
}

/// A Move struct tag for referencing an onchain struct type
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MoveStructTag {
    pub address: Address,
    pub module: IdentifierWrapper,
    pub name: IdentifierWrapper,
    /// Generic type parameters associated with the struct
    pub generic_type_params: Vec<MoveType>,
}

impl VerifyInputWithRecursion for MoveStructTag {
    fn verify(&self, recursion_count: u8) -> anyhow::Result<()> {
        if recursion_count > MAX_RECURSIVE_TYPES_ALLOWED {
            bail!(
                "Move struct tag {} has gone over the limit of recursive types {}",
                self,
                MAX_RECURSIVE_TYPES_ALLOWED
            );
        }
        verify_module_identifier(self.module.as_str())
            .map_err(|_| anyhow::anyhow!("invalid struct tag: {}", self))?;
        verify_identifier(self.name.as_str())
            .map_err(|_| anyhow::anyhow!("invalid struct tag: {}", self))?;
        for param in self.generic_type_params.iter() {
            param.verify(recursion_count + 1).map_err(|err| {
                anyhow::anyhow!(
                    "Invalid struct tag for generic type params: {} {}",
                    self,
                    err
                )
            })?;
        }

        Ok(())
    }
}

impl MoveStructTag {
    pub fn new(
        address: Address,
        module: IdentifierWrapper,
        name: IdentifierWrapper,
        generic_type_params: Vec<MoveType>,
    ) -> Self {
        Self {
            address,
            module,
            name,
            generic_type_params,
        }
    }
}

impl FromStr for MoveStructTag {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> anyhow::Result<Self, anyhow::Error> {
        Ok(parse_struct_tag(s)?.into())
    }
}

impl From<StructTag> for MoveStructTag {
    fn from(tag: StructTag) -> Self {
        Self {
            address: tag.address.into(),
            module: tag.module.into(),
            name: tag.name.into(),
            generic_type_params: tag.type_args.iter().map(MoveType::from).collect(),
        }
    }
}

impl From<&StructTag> for MoveStructTag {
    fn from(tag: &StructTag) -> Self {
        Self {
            address: tag.address.into(),
            module: IdentifierWrapper::from(&tag.module),
            name: IdentifierWrapper::from(&tag.name),
            generic_type_params: tag.type_args.iter().map(MoveType::from).collect(),
        }
    }
}

impl fmt::Display for MoveStructTag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}::{}::{}", self.address, self.module, self.name)?;
        if let Some(first_ty) = self.generic_type_params.first() {
            write!(f, "<")?;
            write!(f, "{}", first_ty)?;
            for ty in self.generic_type_params.iter().skip(1) {
                write!(f, ", {}", ty)?;
            }
            write!(f, ">")?;
        }
        Ok(())
    }
}

impl Serialize for MoveStructTag {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.to_string().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for MoveStructTag {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let data = <String>::deserialize(deserializer)?;
        data.parse().map_err(D::Error::custom)
    }
}

impl TryFrom<&MoveStructTag> for StructTag {
    type Error = anyhow::Error;

    fn try_from(tag: &MoveStructTag) -> anyhow::Result<Self> {
        Ok(Self {
            address: (&tag.address).into(),
            module: (&tag.module).into(),
            name: (&tag.name).into(),
            type_args: tag
                .generic_type_params
                .iter()
                .map(|p| p.try_into())
                .collect::<anyhow::Result<Vec<TypeTag>>>()?,
        })
    }
}

/// An enum of Move's possible types on-chain
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MoveType {
    /// A bool type
    Bool,
    /// An 8-bit unsigned int
    U8,
    /// A 16-bit unsigned int
    U16,
    /// A 32-bit unsigned int
    U32,
    /// A 64-bit unsigned int
    U64,
    /// A 128-bit unsigned int
    U128,
    /// A 256-bit unsigned int
    U256,
    /// A 32-byte account address
    Address,
    /// An account signer
    Signer,
    /// A Vector of [`MoveType`]
    Vector { items: Box<MoveType> },
    /// A struct of [`MoveStructTag`]
    Struct(MoveStructTag),
    /// A function
    Function {
        args: Vec<MoveType>,
        results: Vec<MoveType>,
        abilities: AbilitySet,
    },
    /// A generic type param with index
    GenericTypeParam { index: u16 },
    /// A reference
    Reference { mutable: bool, to: Box<MoveType> },
    /// A move type that couldn't be parsed
    ///
    /// This prevents the parser from just throwing an error because one field
    /// was unparsable, and gives the value in it.
    Unparsable(String),
}

/// Maximum number of recursive types - Same as (non-public)
/// move_core_types::safe_serialize::MAX_TYPE_TAG_NESTING
pub const MAX_RECURSIVE_TYPES_ALLOWED: u8 = 8;

impl VerifyInputWithRecursion for MoveType {
    fn verify(&self, recursion_count: u8) -> anyhow::Result<()> {
        if recursion_count > MAX_RECURSIVE_TYPES_ALLOWED {
            bail!(
                "Move type {} has gone over the limit of recursive types {}",
                self,
                MAX_RECURSIVE_TYPES_ALLOWED
            );
        }
        match self {
            MoveType::Vector { items } => items.verify(recursion_count + 1),
            MoveType::Struct(struct_tag) => struct_tag.verify(recursion_count + 1),
            MoveType::Function { args, results, .. } => {
                for ty in args.iter().chain(results) {
                    ty.verify(recursion_count + 1)?
                }
                Ok(())
            },
            MoveType::GenericTypeParam { .. } => Ok(()),
            MoveType::Reference { to, .. } => to.verify(recursion_count + 1),
            MoveType::Unparsable(inner) => bail!("Unable to parse move type {}", inner),
            _ => Ok(()),
        }
    }
}

impl MoveType {
    /// Returns corresponding JSON data type for the value of `MoveType`
    ///
    /// This type notation here, is just to explain to the user in error messages the type that needs
    /// to be passed in to represent the value.  So it is represented as `JsonType<MoveType>`, where
    /// `JsonType` is the value to be passed in as JSON, and `MoveType` is the move type it is converting
    /// into.
    pub fn json_type_name(&self) -> String {
        match self {
            MoveType::U8 => "integer<u8>".to_owned(),
            MoveType::U16 => "integer<u16>".to_owned(),
            MoveType::U32 => "integer<u32>".to_owned(),
            MoveType::U64 => "string<u64>".to_owned(),
            MoveType::U128 => "string<u128>".to_owned(),
            MoveType::U256 => "string<u256>".to_owned(),
            MoveType::Signer | MoveType::Address => "string<address>".to_owned(),
            MoveType::Bool => "boolean".to_owned(),
            MoveType::Vector { items } => {
                if matches!(**items, MoveType::U8) {
                    "string<hex>".to_owned()
                } else {
                    format!("array<{}>", items.json_type_name())
                }
            },
            MoveType::Struct(_) | MoveType::GenericTypeParam { index: _ } => {
                "string<move_struct_tag_id>".to_owned()
            },
            MoveType::Function { .. } => {
                // TODO(#15664): what to put here for functions?
                "string<move_function_id>".to_owned()
            },
            MoveType::Reference { mutable: _, to } => to.json_type_name(),
            MoveType::Unparsable(string) => string.to_string(),
        }
    }
}

impl fmt::Display for MoveType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MoveType::U8 => write!(f, "u8"),
            MoveType::U16 => write!(f, "u16"),
            MoveType::U32 => write!(f, "u32"),
            MoveType::U64 => write!(f, "u64"),
            MoveType::U128 => write!(f, "u128"),
            MoveType::U256 => write!(f, "u256"),
            MoveType::Address => write!(f, "address"),
            MoveType::Signer => write!(f, "signer"),
            MoveType::Bool => write!(f, "bool"),
            MoveType::Vector { items } => write!(f, "vector<{}>", items),
            MoveType::Struct(s) => write!(f, "{}", s),
            MoveType::GenericTypeParam { index } => write!(f, "T{}", index),
            MoveType::Reference { mutable, to } => {
                if *mutable {
                    write!(f, "&mut {}", to)
                } else {
                    write!(f, "&{}", to)
                }
            },
            MoveType::Function { args, results, .. } => {
                write!(
                    f,
                    "|{}|{}",
                    args.iter()
                        .map(|ty| ty.to_string())
                        .collect::<Vec<_>>()
                        .join(","),
                    results
                        .iter()
                        .map(|ty| ty.to_string())
                        .collect::<Vec<_>>()
                        .join(",")
                )
            },
            MoveType::Unparsable(string) => write!(f, "unparsable<{}>", string),
        }
    }
}

// This function cannot handle the full range of types that MoveType can
// represent. Internally, it uses parse_type_tag, which cannot handle references
// or generic type parameters. This function adds nominal support for references
// on top of parse_type_tag, but it still does not work for generic type params.
// For that, we have the Unparsable variant of MoveType, so the deserialization
// doesn't fail when dealing with these values.
impl FromStr for MoveType {
    type Err = anyhow::Error;

    fn from_str(mut s: &str) -> Result<Self, Self::Err> {
        let mut is_ref = false;
        let mut is_mut = false;
        if s.starts_with('&') {
            s = &s[1..];
            is_ref = true;
        }
        if is_ref && s.starts_with("mut ") {
            s = &s[4..];
            is_mut = true;
        }
        // Previously this would just crap out, but this meant the API could
        // return a serialized version of an object and not be able to
        // deserialize it using that same object.
        let inner = match parse_type_tag(s) {
            Ok(inner) => (&inner).into(),
            Err(_e) => MoveType::Unparsable(s.to_string()),
        };
        if is_ref {
            Ok(MoveType::Reference {
                mutable: is_mut,
                to: Box::new(inner),
            })
        } else {
            Ok(inner)
        }
    }
}

impl Serialize for MoveType {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.to_string().serialize(serializer)
    }
}

// This deserialization has limitations, see the FromStr impl for MoveType.
impl<'de> Deserialize<'de> for MoveType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let data = <String>::deserialize(deserializer)
            .map_err(|e| D::Error::custom(format_err!("deserialize Move type failed, {}", e)))?;
        data.parse().map_err(D::Error::custom)
    }
}

impl MoveType {
    pub fn is_signer(&self) -> bool {
        match self {
            MoveType::Signer => true,
            MoveType::Reference { mutable: _, to } => to.is_signer(),
            _ => false,
        }
    }
}

impl From<&TypeTag> for MoveType {
    fn from(tag: &TypeTag) -> Self {
        match tag {
            TypeTag::Bool => MoveType::Bool,
            TypeTag::U8 => MoveType::U8,
            TypeTag::U16 => MoveType::U16,
            TypeTag::U32 => MoveType::U32,
            TypeTag::U64 => MoveType::U64,
            TypeTag::U128 => MoveType::U128,
            TypeTag::U256 => MoveType::U256,
            TypeTag::Address => MoveType::Address,
            TypeTag::Signer => MoveType::Signer,
            TypeTag::Vector(v) => MoveType::Vector {
                items: Box::new(MoveType::from(v.as_ref())),
            },
            TypeTag::Struct(v) => MoveType::Struct(v.as_ref().into()),
            TypeTag::Function(f) => from_function_tag(f),
        }
    }
}

fn from_function_tag(f: &FunctionTag) -> MoveType {
    let FunctionTag {
        args,
        results,
        abilities,
    } = f;
    let from_vec = |ts: &[FunctionParamOrReturnTag]| {
        ts.iter()
            .map(|t| match t {
                FunctionParamOrReturnTag::Reference(t) => MoveType::Reference {
                    mutable: false,
                    to: Box::new(MoveType::from(t)),
                },
                FunctionParamOrReturnTag::MutableReference(t) => MoveType::Reference {
                    mutable: true,
                    to: Box::new(MoveType::from(t)),
                },
                FunctionParamOrReturnTag::Value(t) => MoveType::from(t),
            })
            .collect::<Vec<_>>()
    };
    MoveType::Function {
        args: from_vec(args),
        results: from_vec(results),
        abilities: *abilities,
    }
}

impl TryFrom<&MoveType> for TypeTag {
    type Error = anyhow::Error;

    fn try_from(tag: &MoveType) -> anyhow::Result<Self> {
        let ret = match tag {
            MoveType::Bool => TypeTag::Bool,
            MoveType::U8 => TypeTag::U8,
            MoveType::U16 => TypeTag::U16,
            MoveType::U32 => TypeTag::U32,
            MoveType::U64 => TypeTag::U64,
            MoveType::U128 => TypeTag::U128,
            MoveType::U256 => TypeTag::U256,
            MoveType::Address => TypeTag::Address,
            MoveType::Signer => TypeTag::Signer,
            MoveType::Vector { items } => TypeTag::Vector(Box::new(items.as_ref().try_into()?)),
            MoveType::Struct(v) => TypeTag::Struct(Box::new(v.try_into()?)),
            MoveType::Function {
                args,
                results,
                abilities,
            } => {
                let try_vec = |tys: &[MoveType]| {
                    tys.iter()
                        .map(|t| {
                            Ok(match t {
                                MoveType::Reference { mutable, to } => {
                                    let tag = to.as_ref().try_into()?;
                                    if *mutable {
                                        FunctionParamOrReturnTag::MutableReference(tag)
                                    } else {
                                        FunctionParamOrReturnTag::Reference(tag)
                                    }
                                },
                                t => FunctionParamOrReturnTag::Value(t.try_into()?),
                            })
                        })
                        .collect::<anyhow::Result<_>>()
                };
                TypeTag::Function(Box::new(FunctionTag {
                    args: try_vec(args)?,
                    results: try_vec(results)?,
                    abilities: *abilities,
                }))
            },
            MoveType::GenericTypeParam { index: _ } => TypeTag::Address, // Dummy type, allows for Object<T>
            MoveType::Reference { .. } | MoveType::Unparsable(_) => {
                return Err(anyhow::anyhow!(
                    "Invalid move type for converting into `TypeTag`: {:?}",
                    &tag
                ))
            },
        };
        Ok(ret)
    }
}

/// A Move module
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Object)]
pub struct MoveModule {
    pub address: Address,
    pub name: IdentifierWrapper,
    /// Friends of the module
    pub friends: Vec<MoveModuleId>,
    /// Public functions of the module
    pub exposed_functions: Vec<MoveFunction>,
    /// Structs of the module
    pub structs: Vec<MoveStruct>,
}

impl From<CompiledModule> for MoveModule {
    fn from(m: CompiledModule) -> Self {
        let (address, name) = <(AccountAddress, Identifier)>::from(m.self_id());
        Self {
            address: address.into(),
            name: name.into(),
            friends: m
                .immediate_friends()
                .into_iter()
                .map(|f| f.into())
                .collect(),
            exposed_functions: m
                .function_defs
                .iter()
                // Return all entry or public functions.
                // Private entry functions are still callable by entry function transactions so
                // they should be included.
                .filter(|def| {
                    def.is_entry
                        || match def.visibility {
                            Visibility::Public | Visibility::Friend => true,
                            Visibility::Private => false,
                        }
                })
                .map(|def| m.new_move_function(def))
                .collect(),
            structs: m
                .struct_defs
                .iter()
                .map(|def| m.new_move_struct(def))
                .collect(),
        }
    }
}

/// A Move module Id
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct MoveModuleId {
    pub address: Address,
    pub name: IdentifierWrapper,
}

impl VerifyInput for MoveModuleId {
    fn verify(&self) -> anyhow::Result<()> {
        self.name.verify().map_err(|_| invalid_move_module_id(self))
    }
}

impl From<ModuleId> for MoveModuleId {
    fn from(id: ModuleId) -> Self {
        let (address, name) = <(AccountAddress, Identifier)>::from(id);
        Self {
            address: address.into(),
            name: name.into(),
        }
    }
}

impl From<MoveModuleId> for ModuleId {
    fn from(id: MoveModuleId) -> Self {
        ModuleId::new(id.address.into(), id.name.into())
    }
}

impl fmt::Display for MoveModuleId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}::{}", self.address, self.name)
    }
}

impl FromStr for MoveModuleId {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some((address, name)) = s.split_once("::") {
            return Ok(Self {
                address: address.parse().map_err(|_| invalid_move_module_id(s))?,
                name: name.parse().map_err(|_| invalid_move_module_id(s))?,
            });
        }
        Err(invalid_move_module_id(s))
    }
}

#[inline]
fn invalid_move_module_id<S: Display + Sized>(s: S) -> anyhow::Error {
    format_err!("Invalid Move module ID: {}", s)
}

impl Serialize for MoveModuleId {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.to_string().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for MoveModuleId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let module_id = <String>::deserialize(deserializer)?;
        module_id.parse().map_err(D::Error::custom)
    }
}

/// A move struct
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Object)]
pub struct MoveStruct {
    pub name: IdentifierWrapper,
    /// Whether the struct is a native struct of Move
    pub is_native: bool,
    /// Whether the struct is marked with the #[event] annotation
    pub is_event: bool,
    /// Abilities associated with the struct
    pub abilities: Vec<MoveAbility>,
    /// Generic types associated with the struct
    pub generic_type_params: Vec<MoveStructGenericTypeParam>,
    /// Fields associated with the struct
    pub fields: Vec<MoveStructField>,
}

/// A move ability e.g. drop, store
// TODO: Consider finding a way to derive NewType here instead of using the
// custom macro, since some of the enum type information (such as the
// variants) is currently being lost.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MoveAbility(pub Ability);

impl From<Ability> for MoveAbility {
    fn from(a: Ability) -> Self {
        Self(a)
    }
}

impl From<MoveAbility> for Ability {
    fn from(a: MoveAbility) -> Self {
        a.0
    }
}

impl fmt::Display for MoveAbility {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let st = match self.0 {
            Ability::Copy => "copy",
            Ability::Drop => "drop",
            Ability::Store => "store",
            Ability::Key => "key",
        };
        write!(f, "{}", st)
    }
}

impl FromStr for MoveAbility {
    type Err = anyhow::Error;

    fn from_str(ability: &str) -> Result<Self, Self::Err> {
        Ok(Self(match ability {
            "copy" => Ability::Copy,
            "drop" => Ability::Drop,
            "store" => Ability::Store,
            "key" => Ability::Key,
            _ => return Err(anyhow::anyhow!("Invalid ability string: {}", ability)),
        }))
    }
}

impl Serialize for MoveAbility {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.to_string().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for MoveAbility {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let ability = <String>::deserialize(deserializer)?;
        ability.parse().map_err(D::Error::custom)
    }
}

/// Move generic type param
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Object)]
pub struct MoveStructGenericTypeParam {
    /// Move abilities tied to the generic type param and associated with the type that uses it
    pub constraints: Vec<MoveAbility>,
    /// Whether the type is a phantom type
    #[oai(skip)]
    pub is_phantom: bool,
}

impl From<&StructTypeParameter> for MoveStructGenericTypeParam {
    fn from(param: &StructTypeParameter) -> Self {
        Self {
            constraints: param
                .constraints
                .into_iter()
                .map(MoveAbility::from)
                .collect(),
            is_phantom: param.is_phantom,
        }
    }
}

/// Move struct field
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Object)]
pub struct MoveStructField {
    pub name: IdentifierWrapper,
    #[serde(rename = "type")]
    #[oai(rename = "type")]
    pub typ: MoveType,
}

/// Move function
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Object)]
pub struct MoveFunction {
    pub name: IdentifierWrapper,
    pub visibility: MoveFunctionVisibility,
    /// Whether the function can be called as an entry function directly in a transaction
    pub is_entry: bool,
    /// Whether the function is a view function or not
    pub is_view: bool,
    /// Generic type params associated with the Move function
    pub generic_type_params: Vec<MoveFunctionGenericTypeParam>,
    /// Parameters associated with the move function
    pub params: Vec<MoveType>,
    /// Return type of the function
    #[serde(rename = "return")]
    #[oai(rename = "return")]
    pub return_: Vec<MoveType>,
}

impl From<&CompiledScript> for MoveFunction {
    fn from(script: &CompiledScript) -> Self {
        Self {
            name: Identifier::new("main").unwrap().into(),
            visibility: MoveFunctionVisibility::Public,
            is_entry: true,
            is_view: false,
            generic_type_params: script
                .type_parameters
                .iter()
                .map(MoveFunctionGenericTypeParam::from)
                .collect(),
            params: script
                .signature_at(script.parameters)
                .0
                .iter()
                .map(|s| script.new_move_type(s))
                .collect(),
            return_: vec![],
        }
    }
}

/// Move function visibility
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Enum)]
#[serde(rename_all = "snake_case")]
#[oai(rename_all = "snake_case")]
pub enum MoveFunctionVisibility {
    /// Visible only by this module
    Private,
    /// Visible by all modules
    Public,
    /// Visible by friend modules
    Friend,
}

impl From<Visibility> for MoveFunctionVisibility {
    fn from(v: Visibility) -> Self {
        match &v {
            Visibility::Private => Self::Private,
            Visibility::Public => Self::Public,
            Visibility::Friend => Self::Friend,
        }
    }
}

impl From<MoveFunctionVisibility> for Visibility {
    fn from(v: MoveFunctionVisibility) -> Self {
        match &v {
            MoveFunctionVisibility::Private => Self::Private,
            MoveFunctionVisibility::Public => Self::Public,
            MoveFunctionVisibility::Friend => Self::Friend,
        }
    }
}

/// Move function generic type param
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Object)]
pub struct MoveFunctionGenericTypeParam {
    /// Move abilities tied to the generic type param and associated with the function that uses it
    pub constraints: Vec<MoveAbility>,
}

impl From<&AbilitySet> for MoveFunctionGenericTypeParam {
    fn from(constraints: &AbilitySet) -> Self {
        Self {
            constraints: constraints.into_iter().map(MoveAbility::from).collect(),
        }
    }
}

/// Move module bytecode along with it's ABI
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Object)]
pub struct MoveModuleBytecode {
    pub bytecode: HexEncodedBytes,
    // We don't need deserialize MoveModule as it should be serialized
    // from `bytecode`.
    #[serde(skip_deserializing)]
    pub abi: Option<MoveModule>,
}

impl VerifyInput for MoveModuleBytecode {
    fn verify(&self) -> anyhow::Result<()> {
        if self.bytecode.is_empty() {
            bail!("Move module bytecode is empty")
        }

        Ok(())
    }
}

impl MoveModuleBytecode {
    pub fn new(bytes: Vec<u8>) -> Self {
        Self {
            bytecode: bytes.into(),
            abi: None,
        }
    }

    #[allow(clippy::unnecessary_fallible_conversions)]
    pub fn try_parse_abi(mut self) -> anyhow::Result<Self> {
        if self.abi.is_none() {
            // Ignore error, because it is possible a transaction module payload contains
            // invalid bytecode.
            // So we ignore the error and output bytecode without abi.
            if let Ok(module) = CompiledModule::deserialize(self.bytecode.inner()) {
                self.abi = Some(module.try_into()?);
            }
        }
        Ok(self)
    }
}

impl From<Module> for MoveModuleBytecode {
    fn from(m: Module) -> Self {
        Self::new(m.into_inner())
    }
}

/// Move script bytecode
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Object)]
pub struct MoveScriptBytecode {
    pub bytecode: HexEncodedBytes,
    // We don't need deserialize MoveModule as it should be serialized
    // from `bytecode`.
    #[serde(skip_deserializing)]
    pub abi: Option<MoveFunction>,
}

impl VerifyInput for MoveScriptBytecode {
    fn verify(&self) -> anyhow::Result<()> {
        if self.bytecode.is_empty() {
            bail!("Move script bytecode is empty")
        }

        Ok(())
    }
}

impl MoveScriptBytecode {
    pub fn new(bytes: Vec<u8>) -> Self {
        Self {
            bytecode: bytes.into(),
            abi: None,
        }
    }

    pub fn try_parse_abi(mut self) -> Self {
        if self.abi.is_none() {
            // ignore error, because it is possible a transaction script payload contains
            // invalid bytecode.
            // So we ignore the error and output bytecode without abi.
            if let Ok(script) = CompiledScript::deserialize(self.bytecode.inner()) {
                self.abi = Some((&script).into());
            }
        }
        self
    }
}

/// Entry function id
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EntryFunctionId {
    pub module: MoveModuleId,
    pub name: IdentifierWrapper,
}

impl VerifyInput for EntryFunctionId {
    fn verify(&self) -> anyhow::Result<()> {
        self.module
            .verify()
            .map_err(|_| invalid_entry_function_id(self))?;
        self.name
            .verify()
            .map_err(|_| invalid_entry_function_id(self))
    }
}

impl FromStr for EntryFunctionId {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some((module, name)) = s.rsplit_once("::") {
            return Ok(Self {
                module: module.parse().map_err(|_| invalid_entry_function_id(s))?,
                name: name.parse().map_err(|_| invalid_entry_function_id(s))?,
            });
        }
        Err(invalid_entry_function_id(s))
    }
}

#[inline]
fn invalid_entry_function_id<S: Display + Sized>(s: S) -> anyhow::Error {
    format_err!("Invalid entry function ID {}", s)
}

impl Serialize for EntryFunctionId {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.to_string().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for EntryFunctionId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let entry_fun_id = <String>::deserialize(deserializer)?;
        entry_fun_id.parse().map_err(D::Error::custom)
    }
}

impl fmt::Display for EntryFunctionId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}::{}", self.module, self.name)
    }
}

pub fn verify_function_identifier(function: &str) -> anyhow::Result<()> {
    verify_identifier(function).map_err(|_| format_err!("invalid Move function name: {}", function))
}
pub fn verify_module_identifier(module: &str) -> anyhow::Result<()> {
    verify_identifier(module).map_err(|_| format_err!("invalid Move module name: {}", module))
}

pub fn verify_field_identifier(field: &str) -> anyhow::Result<()> {
    verify_identifier(field).map_err(|_| format_err!("invalid Move field name: {}", field))
}

pub fn verify_identifier(identifier: &str) -> anyhow::Result<()> {
    if identifier.contains("::") {
        Err(format_err!(
            "Identifier should not contain '::' {}",
            identifier
        ))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aptos_types::account_address::AccountAddress;
    use move_core_types::{
        ability::AbilitySet,
        identifier::Identifier,
        language_storage::{StructTag, TypeTag},
    };
    use serde::{de::DeserializeOwned, Serialize};
    use serde_json::{json, to_value, Value};
    use std::{boxed::Box, convert::TryFrom, fmt::Debug};

    #[test]
    fn test_serialize_move_type_tag() {
        use TypeTag::*;
        fn assert_serialize(t: TypeTag, expected: Value) {
            let value = to_value(MoveType::from(&t)).unwrap();
            assert_json(value, expected)
        }
        assert_serialize(Bool, json!("bool"));
        assert_serialize(U8, json!("u8"));
        assert_serialize(U64, json!("u64"));
        assert_serialize(U128, json!("u128"));
        assert_serialize(Address, json!("address"));
        assert_serialize(Signer, json!("signer"));

        assert_serialize(Vector(Box::new(U8)), json!("vector<u8>"));

        assert_serialize(
            Struct(Box::new(create_nested_struct())),
            json!("0x1::Home::ABC<address, 0x1::account::Base<u128, vector<u64>, vector<0x1::type::String>, 0x1::type::String>>"),
        );
    }

    #[test]
    fn test_serialize_move_resource() {
        use AnnotatedMoveValue::*;

        let res = MoveResource::try_from(annotated_move_struct("Values", vec![
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
                Struct(annotated_move_struct("Nested", vec![(
                    identifier("nested_vector"),
                    Vector(TypeTag::Struct(Box::new(type_struct("Host"))), vec![
                        Struct(annotated_move_struct("String", vec![
                            (identifier("address1"), Address(address("0x0"))),
                            (identifier("address2"), Address(address("0x123"))),
                        ])),
                    ]),
                )])),
            ),
        ]))
        .unwrap();
        let value = to_value(&res).unwrap();
        assert_json(
            value,
            json!({
                "type": "0x1::type::Values",
                "data": {
                    "field_u8": 7,
                    "field_u64": "7",
                    "field_u128": "7",
                    "field_bool": true,
                    "field_address": "0xdd",
                    "field_vector": ["128"],
                    "field_bytes": "0x0909",
                    "field_struct": {
                        "nested_vector": [{"address1": "0x0", "address2": "0x123"}]
                    },
                }
            }),
        );
    }

    #[test]
    fn test_serialize_move_resource_with_address_0x0() {
        let res = MoveResource::try_from(annotated_move_struct("Values", vec![(
            identifier("address_0x0"),
            AnnotatedMoveValue::Address(address("0x0")),
        )]))
        .unwrap();
        let value = to_value(&res).unwrap();
        assert_json(
            value,
            json!({
                "type": "0x1::type::Values",
                "data": {
                    "address_0x0": "0x0",
                }
            }),
        );
    }

    #[test]
    fn test_serialize_deserialize_u64() {
        test_serialize_deserialize(U64::from(u64::MAX), json!(u64::MAX.to_string()))
    }

    #[test]
    fn test_serialize_deserialize_u128() {
        test_serialize_deserialize(U128::from(u128::MAX), json!(u128::MAX.to_string()))
    }

    #[test]
    fn test_serialize_deserialize_move_module_id() {
        test_serialize_deserialize(
            MoveModuleId {
                address: "0x1".parse().unwrap(),
                name: "Aptos".parse().unwrap(),
            },
            json!("0x1::Aptos"),
        );
    }

    #[test]
    fn test_parse_invalid_move_module_id_string() {
        assert_eq!(
            "Invalid Move module ID: 0x1",
            "0x1".parse::<MoveModuleId>().err().unwrap().to_string()
        );
        assert_eq!(
            "Invalid Move module ID: 0x1:",
            "0x1:".parse::<MoveModuleId>().err().unwrap().to_string()
        );
        assert_eq!(
            "Invalid Move module ID: 0x1:::",
            "0x1:::".parse::<MoveModuleId>().err().unwrap().to_string()
        );
        assert_eq!(
            "Invalid Move module ID: 0x1::???",
            "0x1::???"
                .parse::<MoveModuleId>()
                .err()
                .unwrap()
                .to_string()
        );
        assert_eq!(
            "Invalid Move module ID: Aptos::Aptos",
            "Aptos::Aptos"
                .parse::<MoveModuleId>()
                .err()
                .unwrap()
                .to_string()
        );
        assert_eq!(
            "Invalid Move module ID: 0x1::Aptos::Aptos",
            "0x1::Aptos::Aptos"
                .parse::<MoveModuleId>()
                .err()
                .unwrap()
                .to_string()
        );
    }

    #[test]
    fn test_serialize_deserialize_move_entry_function_id() {
        test_serialize_deserialize(
            EntryFunctionId {
                module: MoveModuleId {
                    address: "0x1".parse().unwrap(),
                    name: "Aptos".parse().unwrap(),
                },
                name: "Add".parse().unwrap(),
            },
            json!("0x1::Aptos::Add"),
        );
    }

    #[test]
    fn test_parse_invalid_move_entry_function_id_string() {
        assert_eq!(
            "Invalid entry function ID 0x1",
            "0x1".parse::<EntryFunctionId>().err().unwrap().to_string()
        );
        assert_eq!(
            "Invalid entry function ID 0x1:",
            "0x1:".parse::<EntryFunctionId>().err().unwrap().to_string()
        );
        assert_eq!(
            "Invalid entry function ID 0x1:::",
            "0x1:::"
                .parse::<EntryFunctionId>()
                .err()
                .unwrap()
                .to_string()
        );
        assert_eq!(
            "Invalid entry function ID 0x1::???",
            "0x1::???"
                .parse::<EntryFunctionId>()
                .err()
                .unwrap()
                .to_string()
        );
        assert_eq!(
            "Invalid entry function ID Aptos::Aptos",
            "Aptos::Aptos"
                .parse::<EntryFunctionId>()
                .err()
                .unwrap()
                .to_string()
        );
        assert_eq!(
            "Invalid entry function ID Aptos::Aptos::??",
            "Aptos::Aptos::??"
                .parse::<EntryFunctionId>()
                .err()
                .unwrap()
                .to_string()
        );
        assert_eq!(
            "Invalid entry function ID 0x1::Aptos::Aptos::Aptos",
            "0x1::Aptos::Aptos::Aptos"
                .parse::<EntryFunctionId>()
                .err()
                .unwrap()
                .to_string()
        );
    }

    #[test]
    fn test_serialize_deserialize_hex_encoded_bytes() {
        let bytes = hex::decode("abcd").unwrap();
        test_serialize_deserialize(HexEncodedBytes::from(bytes), json!("0xabcd"))
    }

    fn test_serialize_deserialize<O>(obj: O, expected: Value)
    where
        O: Serialize + DeserializeOwned + PartialEq + Debug,
    {
        let val = serde_json::to_value(&obj).unwrap();
        assert_eq!(val, expected);

        let data: O = serde_json::from_value(val).unwrap();
        assert_eq!(data, obj);
    }

    fn create_nested_struct() -> StructTag {
        let account = create_generic_type_struct();
        StructTag {
            address: address("0x1"),
            module: identifier("Home"),
            name: identifier("ABC"),
            type_args: vec![TypeTag::Address, TypeTag::Struct(Box::new(account))],
        }
    }

    fn create_generic_type_struct() -> StructTag {
        StructTag {
            address: address("0x1"),
            module: identifier("account"),
            name: identifier("Base"),
            type_args: vec![
                TypeTag::U128,
                TypeTag::Vector(Box::new(TypeTag::U64)),
                TypeTag::Vector(Box::new(TypeTag::Struct(Box::new(type_struct("String"))))),
                TypeTag::Struct(Box::new(type_struct("String"))),
            ],
        }
    }

    fn type_struct(t: &str) -> StructTag {
        StructTag {
            address: address("0x1"),
            module: identifier("type"),
            name: identifier(t),
            type_args: vec![],
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
            ty_tag: type_struct(typ),
            variant_info: None,
            value: values,
        }
    }

    fn identifier(id: &str) -> Identifier {
        Identifier::new(id).unwrap()
    }

    fn assert_json(ret: Value, expected: Value) {
        assert_eq!(
            &ret,
            &expected,
            "\nexpected: {}, \nbut got: {}",
            pretty(&expected),
            pretty(&ret)
        )
    }

    fn pretty(val: &Value) -> String {
        serde_json::to_string_pretty(val).unwrap()
    }
}
