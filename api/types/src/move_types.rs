// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{Address, Bytecode};

use anyhow::format_err;
use diem_types::{event::EventKey, transaction::Module};
use move_binary_format::{
    access::ModuleAccess,
    file_format::{
        Ability, AbilitySet, CompiledModule, CompiledScript, StructTypeParameter, Visibility,
    },
};
use move_core_types::{
    account_address::AccountAddress,
    identifier::Identifier,
    language_storage::{ModuleId, StructTag, TypeTag},
    parser::{parse_struct_tag, parse_type_tag},
    transaction_argument::TransactionArgument,
};
use resource_viewer::{AnnotatedMoveStruct, AnnotatedMoveValue};

use serde::{de::Error as _, Deserialize, Deserializer, Serialize, Serializer};
use std::{
    collections::BTreeMap,
    convert::{From, Into, TryFrom, TryInto},
    fmt,
    result::Result,
    str::FromStr,
};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MoveResource {
    #[serde(rename = "type")]
    pub typ: MoveStructTag,
    pub data: MoveStructValue,
}

impl TryFrom<AnnotatedMoveStruct> for MoveResource {
    type Error = anyhow::Error;

    fn try_from(s: AnnotatedMoveStruct) -> anyhow::Result<Self> {
        Ok(Self {
            typ: s.type_.clone().into(),
            data: s.try_into()?,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Copy)]
pub struct U64(u64);

impl U64 {
    pub fn inner(&self) -> &u64 {
        &self.0
    }
}

impl From<u64> for U64 {
    fn from(d: u64) -> Self {
        Self(d)
    }
}

impl From<U64> for warp::http::header::HeaderValue {
    fn from(d: U64) -> Self {
        d.0.into()
    }
}

impl From<U64> for u64 {
    fn from(d: U64) -> Self {
        d.0
    }
}

impl From<U64> for move_core_types::value::MoveValue {
    fn from(d: U64) -> Self {
        move_core_types::value::MoveValue::U64(d.0)
    }
}

impl fmt::Display for U64 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", &self.0)
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

#[derive(Clone, Debug, PartialEq, Copy)]
pub struct U128(u128);

impl U128 {
    pub fn inner(&self) -> &u128 {
        &self.0
    }
}

impl From<u128> for U128 {
    fn from(d: u128) -> Self {
        Self(d)
    }
}

impl From<U128> for u128 {
    fn from(d: U128) -> Self {
        d.0
    }
}

impl From<U128> for move_core_types::value::MoveValue {
    fn from(d: U128) -> Self {
        move_core_types::value::MoveValue::U128(d.0)
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

impl HexEncodedBytes {
    pub fn json(&self) -> anyhow::Result<serde_json::Value> {
        Ok(serde_json::to_value(self)?)
    }
}

impl FromStr for HexEncodedBytes {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> anyhow::Result<Self, anyhow::Error> {
        if let Some(hex) = s.strip_prefix("0x") {
            Ok(Self(hex::decode(&hex)?))
        } else {
            Ok(Self(hex::decode(&s)?))
        }
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
        Ok(EventKey::from_bytes(bytes.0)?)
    }
}

impl HexEncodedBytes {
    pub fn inner(&self) -> &[u8] {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MoveStructValue(BTreeMap<Identifier, serde_json::Value>);

impl TryFrom<AnnotatedMoveStruct> for MoveStructValue {
    type Error = anyhow::Error;
    fn try_from(s: AnnotatedMoveStruct) -> anyhow::Result<Self> {
        let mut map = BTreeMap::new();
        for (id, val) in s.value {
            map.insert(id, MoveValue::try_from(val)?.json()?);
        }
        Ok(Self(map))
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

impl MoveValue {
    pub fn json(&self) -> anyhow::Result<serde_json::Value> {
        Ok(serde_json::to_value(self)?)
    }
}

impl TryFrom<AnnotatedMoveValue> for MoveValue {
    type Error = anyhow::Error;

    fn try_from(val: AnnotatedMoveValue) -> anyhow::Result<Self> {
        Ok(match val {
            AnnotatedMoveValue::U8(v) => MoveValue::U8(v),
            AnnotatedMoveValue::U64(v) => MoveValue::U64(U64(v)),
            AnnotatedMoveValue::U128(v) => MoveValue::U128(U128(v)),
            AnnotatedMoveValue::Bool(v) => MoveValue::Bool(v),
            AnnotatedMoveValue::Address(v) => MoveValue::Address(v.into()),
            AnnotatedMoveValue::Vector(_, vals) => MoveValue::Vector(
                vals.into_iter()
                    .map(MoveValue::try_from)
                    .collect::<anyhow::Result<_>>()?,
            ),
            AnnotatedMoveValue::Bytes(v) => MoveValue::Bytes(HexEncodedBytes(v)),
            AnnotatedMoveValue::Struct(v) => MoveValue::Struct(v.try_into()?),
        })
    }
}

impl From<TransactionArgument> for MoveValue {
    fn from(val: TransactionArgument) -> Self {
        match val {
            TransactionArgument::U8(v) => MoveValue::U8(v),
            TransactionArgument::U64(v) => MoveValue::U64(U64(v)),
            TransactionArgument::U128(v) => MoveValue::U128(U128(v)),
            TransactionArgument::Bool(v) => MoveValue::Bool(v),
            TransactionArgument::Address(v) => MoveValue::Address(v.into()),
            TransactionArgument::U8Vector(bytes) => MoveValue::Bytes(HexEncodedBytes(bytes)),
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

#[derive(Clone, Debug, PartialEq)]
pub struct MoveStructTag {
    pub address: Address,
    pub module: Identifier,
    pub name: Identifier,
    pub generic_type_params: Vec<MoveType>,
}

impl MoveStructTag {
    pub fn new(
        address: Address,
        module: Identifier,
        name: Identifier,
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
            module: tag.module,
            name: tag.name,
            generic_type_params: tag.type_params.into_iter().map(MoveType::from).collect(),
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

impl TryFrom<MoveStructTag> for StructTag {
    type Error = anyhow::Error;
    fn try_from(tag: MoveStructTag) -> anyhow::Result<Self> {
        Ok(Self {
            address: tag.address.into(),
            module: tag.module,
            name: tag.name,
            type_params: tag
                .generic_type_params
                .into_iter()
                .map(|p| p.try_into())
                .collect::<anyhow::Result<Vec<TypeTag>>>()?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum MoveType {
    Bool,
    U8,
    U64,
    U128,
    Address,
    Signer,
    Vector { items: Box<MoveType> },
    Struct(MoveStructTag),
    GenericTypeParam { index: u16 },
    Reference { mutable: bool, to: Box<MoveType> },
}

impl fmt::Display for MoveType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MoveType::U8 => write!(f, "u8"),
            MoveType::U64 => write!(f, "u64"),
            MoveType::U128 => write!(f, "u128"),
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
            }
        }
    }
}

impl FromStr for MoveType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(parse_type_tag(s)?.into())
    }
}

impl Serialize for MoveType {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.to_string().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for MoveType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let data = <String>::deserialize(deserializer)?;
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

impl From<TypeTag> for MoveType {
    fn from(tag: TypeTag) -> Self {
        match tag {
            TypeTag::Bool => MoveType::Bool,
            TypeTag::U8 => MoveType::U8,
            TypeTag::U64 => MoveType::U64,
            TypeTag::U128 => MoveType::U128,
            TypeTag::Address => MoveType::Address,
            TypeTag::Signer => MoveType::Signer,
            TypeTag::Vector(v) => MoveType::Vector {
                items: Box::new(MoveType::from(*v)),
            },
            TypeTag::Struct(v) => MoveType::Struct(v.into()),
        }
    }
}

impl TryFrom<MoveType> for TypeTag {
    type Error = anyhow::Error;
    fn try_from(tag: MoveType) -> anyhow::Result<Self> {
        let ret = match tag {
            MoveType::Bool => TypeTag::Bool,
            MoveType::U8 => TypeTag::U8,
            MoveType::U64 => TypeTag::U64,
            MoveType::U128 => TypeTag::U128,
            MoveType::Address => TypeTag::Address,
            MoveType::Signer => TypeTag::Signer,
            MoveType::Vector { items } => TypeTag::Vector(Box::new((*items).try_into()?)),
            MoveType::Struct(v) => TypeTag::Struct(v.try_into()?),
            _ => {
                return Err(anyhow::anyhow!(
                    "invalid move type for converting into `TypeTag`: {:?}",
                    &tag
                ))
            }
        };
        Ok(ret)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MoveModule {
    pub address: Address,
    pub name: Identifier,
    pub friends: Vec<MoveModuleId>,
    pub exposed_functions: Vec<MoveFunction>,
    pub structs: Vec<MoveStruct>,
}

impl From<CompiledModule> for MoveModule {
    fn from(m: CompiledModule) -> Self {
        let (address, name) = <(AccountAddress, Identifier)>::from(m.self_id());
        Self {
            address: address.into(),
            name,
            friends: m
                .immediate_friends()
                .into_iter()
                .map(|f| f.into())
                .collect(),
            exposed_functions: m
                .function_defs
                .iter()
                .filter(|def| match def.visibility {
                    Visibility::Public | Visibility::Friend | Visibility::Script => true,
                    Visibility::Private => false,
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

#[derive(Clone, Debug, PartialEq)]
pub struct MoveModuleId {
    pub address: Address,
    pub name: Identifier,
}

impl From<ModuleId> for MoveModuleId {
    fn from(id: ModuleId) -> Self {
        let (address, name) = <(AccountAddress, Identifier)>::from(id);
        Self {
            address: address.into(),
            name,
        }
    }
}

impl From<MoveModuleId> for ModuleId {
    fn from(id: MoveModuleId) -> Self {
        ModuleId::new(id.address.into(), id.name)
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
fn invalid_move_module_id(s: &str) -> anyhow::Error {
    format_err!("invalid Move module id: {}", s)
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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MoveStruct {
    pub name: Identifier,
    pub is_native: bool,
    pub abilities: Vec<MoveAbility>,
    pub generic_type_params: Vec<MoveStructGenericTypeParam>,
    pub fields: Vec<MoveStructField>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MoveAbility(Ability);

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
            _ => return Err(anyhow::anyhow!("invalid ability string: {}", ability)),
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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MoveStructGenericTypeParam {
    pub constraints: Vec<MoveAbility>,
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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MoveStructField {
    pub name: Identifier,
    #[serde(rename = "type")]
    pub typ: MoveType,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MoveFunction {
    pub name: Identifier,
    pub visibility: MoveFunctionVisibility,
    pub generic_type_params: Vec<MoveFunctionGenericTypeParam>,
    pub params: Vec<MoveType>,
    #[serde(rename = "return")]
    pub return_: Vec<MoveType>,
}

impl From<&CompiledScript> for MoveFunction {
    fn from(script: &CompiledScript) -> Self {
        Self {
            name: Identifier::new("main").unwrap(),
            visibility: MoveFunctionVisibility::Script,
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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MoveFunctionVisibility {
    Private,
    Public,
    Script,
    Friend,
}

impl From<Visibility> for MoveFunctionVisibility {
    fn from(v: Visibility) -> Self {
        match &v {
            Visibility::Private => Self::Private,
            Visibility::Public => Self::Public,
            Visibility::Script => Self::Script,
            Visibility::Friend => Self::Friend,
        }
    }
}

impl From<MoveFunctionVisibility> for Visibility {
    fn from(v: MoveFunctionVisibility) -> Self {
        match &v {
            MoveFunctionVisibility::Private => Self::Private,
            MoveFunctionVisibility::Public => Self::Public,
            MoveFunctionVisibility::Script => Self::Script,
            MoveFunctionVisibility::Friend => Self::Friend,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MoveFunctionGenericTypeParam {
    pub constraints: Vec<MoveAbility>,
}

impl From<&AbilitySet> for MoveFunctionGenericTypeParam {
    fn from(constraints: &AbilitySet) -> Self {
        Self {
            constraints: constraints.into_iter().map(MoveAbility::from).collect(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MoveModuleBytecode {
    pub bytecode: HexEncodedBytes,
    pub abi: Option<MoveModule>,
}

impl MoveModuleBytecode {
    pub fn new(bytes: Vec<u8>) -> Self {
        Self {
            bytecode: bytes.into(),
            abi: None,
        }
    }

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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MoveScriptBytecode {
    pub bytecode: HexEncodedBytes,
    pub abi: Option<MoveFunction>,
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

#[derive(Clone, Debug, PartialEq)]
pub struct ScriptFunctionId {
    pub module: MoveModuleId,
    pub name: Identifier,
}

impl FromStr for ScriptFunctionId {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some((module, name)) = s.rsplit_once("::") {
            return Ok(Self {
                module: module.parse().map_err(|_| invalid_script_function_id(s))?,
                name: name.parse().map_err(|_| invalid_script_function_id(s))?,
            });
        }
        Err(invalid_script_function_id(s))
    }
}

#[inline]
fn invalid_script_function_id(s: &str) -> anyhow::Error {
    format_err!("invalid script function id: {}", s)
}

impl Serialize for ScriptFunctionId {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.to_string().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ScriptFunctionId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let script_fun_id = <String>::deserialize(deserializer)?;
        script_fun_id.parse().map_err(D::Error::custom)
    }
}

impl fmt::Display for ScriptFunctionId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}::{}", self.module, self.name)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        move_types::ScriptFunctionId, HexEncodedBytes, MoveModuleId, MoveResource, MoveType, U128,
        U64,
    };

    use diem_types::account_address::AccountAddress;
    use move_binary_format::file_format::AbilitySet;
    use move_core_types::{
        identifier::Identifier,
        language_storage::{StructTag, TypeTag},
    };
    use resource_viewer::{AnnotatedMoveStruct, AnnotatedMoveValue};

    use serde::{de::DeserializeOwned, Serialize};
    use serde_json::{json, to_value, Value};
    use std::{boxed::Box, convert::TryFrom, fmt::Debug};

    #[test]
    fn test_serialize_move_type_tag() {
        use TypeTag::*;
        fn assert_serialize(t: TypeTag, expected: Value) {
            let value = to_value(MoveType::from(t)).unwrap();
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
            Struct(create_nested_struct()),
            json!("0x1::Home::ABC<address, 0x1::Account::Base<u128, vector<u64>, vector<0x1::Type::String>, 0x1::Type::String>>"),
        );
    }

    #[test]
    fn test_serialize_move_resource() {
        use AnnotatedMoveValue::*;

        let res = MoveResource::try_from(annotated_move_struct(
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
        ))
        .unwrap();
        let value = to_value(&res).unwrap();
        assert_json(
            value,
            json!({
                "type": "0x1::Type::Values",
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
        let res = MoveResource::try_from(annotated_move_struct(
            "Values",
            vec![(
                identifier("address_0x0"),
                AnnotatedMoveValue::Address(address("0x0")),
            )],
        ))
        .unwrap();
        let value = to_value(&res).unwrap();
        assert_json(
            value,
            json!({
                "type": "0x1::Type::Values",
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
                name: "Diem".parse().unwrap(),
            },
            json!("0x1::Diem"),
        );
    }

    #[test]
    fn test_parse_invalid_move_module_id_string() {
        assert_eq!(
            "invalid Move module id: 0x1",
            "0x1".parse::<MoveModuleId>().err().unwrap().to_string()
        );
        assert_eq!(
            "invalid Move module id: 0x1:",
            "0x1:".parse::<MoveModuleId>().err().unwrap().to_string()
        );
        assert_eq!(
            "invalid Move module id: 0x1:::",
            "0x1:::".parse::<MoveModuleId>().err().unwrap().to_string()
        );
        assert_eq!(
            "invalid Move module id: 0x1::???",
            "0x1::???"
                .parse::<MoveModuleId>()
                .err()
                .unwrap()
                .to_string()
        );
        assert_eq!(
            "invalid Move module id: Diem::Diem",
            "Diem::Diem"
                .parse::<MoveModuleId>()
                .err()
                .unwrap()
                .to_string()
        );
        assert_eq!(
            "invalid Move module id: 0x1::Diem::Diem",
            "0x1::Diem::Diem"
                .parse::<MoveModuleId>()
                .err()
                .unwrap()
                .to_string()
        );
    }

    #[test]
    fn test_serialize_deserialize_move_script_function_id() {
        test_serialize_deserialize(
            ScriptFunctionId {
                module: MoveModuleId {
                    address: "0x1".parse().unwrap(),
                    name: "Diem".parse().unwrap(),
                },
                name: "Add".parse().unwrap(),
            },
            json!("0x1::Diem::Add"),
        );
    }

    #[test]
    fn test_parse_invalid_move_script_function_id_string() {
        assert_eq!(
            "invalid script function id: 0x1",
            "0x1".parse::<ScriptFunctionId>().err().unwrap().to_string()
        );
        assert_eq!(
            "invalid script function id: 0x1:",
            "0x1:"
                .parse::<ScriptFunctionId>()
                .err()
                .unwrap()
                .to_string()
        );
        assert_eq!(
            "invalid script function id: 0x1:::",
            "0x1:::"
                .parse::<ScriptFunctionId>()
                .err()
                .unwrap()
                .to_string()
        );
        assert_eq!(
            "invalid script function id: 0x1::???",
            "0x1::???"
                .parse::<ScriptFunctionId>()
                .err()
                .unwrap()
                .to_string()
        );
        assert_eq!(
            "invalid script function id: Diem::Diem",
            "Diem::Diem"
                .parse::<ScriptFunctionId>()
                .err()
                .unwrap()
                .to_string()
        );
        assert_eq!(
            "invalid script function id: Diem::Diem::??",
            "Diem::Diem::??"
                .parse::<ScriptFunctionId>()
                .err()
                .unwrap()
                .to_string()
        );
        assert_eq!(
            "invalid script function id: 0x1::Diem::Diem::Diem",
            "0x1::Diem::Diem::Diem"
                .parse::<ScriptFunctionId>()
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
