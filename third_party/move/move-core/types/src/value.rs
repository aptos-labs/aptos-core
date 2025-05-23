// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

// The below is to deal with a strange problem with derive(Dearbitrary), which creates warnings
// of unused variables in derived code which cannot be turned off by applying the attribute
// just at the type in question. (Here, MoveStructLayout.)
#![allow(unused_variables)]

use crate::{
    account_address::AccountAddress,
    function::{ClosureVisitor, MoveClosure},
    ident_str,
    identifier::Identifier,
    language_storage::{ModuleId, StructTag, TypeTag},
    u256,
};
use anyhow::{anyhow, bail, Result as AResult};
use once_cell::sync::Lazy;
use serde::{
    de::{EnumAccess, Error as DeError, VariantAccess},
    ser::{SerializeMap, SerializeSeq, SerializeStruct, SerializeTuple, SerializeTupleVariant},
    Deserialize, Serialize,
};
use std::{
    collections::{btree_map::Entry, BTreeMap},
    convert::TryInto,
    fmt::{self, Debug},
    sync::Mutex,
};

/// The maximal number of enum variants which are supported in values. This must align with
/// the configuration in the binary format, so the bytecode verifier checks its validness.
pub const VARIANT_COUNT_MAX: u64 = 127;

/// In the `WithTypes` configuration, a Move struct gets serialized into a Serde struct with this name
pub const MOVE_STRUCT_NAME: &str = "struct";

/// A Move enum gets serialized into a Serde struct with this name
pub const MOVE_ENUM_NAME: &str = "enum";

/// In the `WithTypes` configuration, a Move struct gets serialized into a Serde struct with this as the first field
pub const MOVE_STRUCT_TYPE: &str = "type";

/// In the `WithTypes` configuration, a Move struct gets serialized into a Serde struct with this as the second field
pub const MOVE_STRUCT_FIELDS: &str = "fields";

/// In the `WithVariant` configuration, a Move enum variant gets serialized into a Serde struct with this name
pub const MOVE_VARIANT_NAME: &str = "variant";

/// In the `WithVariant` configuration, a Move enum variant gets serialized into a Serde struct with this as the first field
pub const MOVE_VARIANT_NAME_FIELD: &str = "name";

/// In order to serialize enums with serde, we have to provide `&'static str` names for
/// variants. This static cache is used to generate those names, and contains
/// signatures of the form `["0", "1", "2", ..]`. The size of this cache
/// is bound by `VARIANT_COUNT_MAX * VARIANT_COUNT_MAX / 2` (8064 with max 127)
static VARIANT_NAME_PLACEHOLDER_CACHE: Lazy<Mutex<BTreeMap<usize, &'static [&'static str]>>> =
    Lazy::new(|| Mutex::new(Default::default()));

/// Returns variant name placeholders for providing dummy names in serde serialization.
pub fn variant_name_placeholder(len: usize) -> Result<&'static [&'static str], anyhow::Error> {
    if len > VARIANT_COUNT_MAX as usize {
        bail!("variant count is restricted to {}", VARIANT_COUNT_MAX);
    }
    let mutex = &VARIANT_NAME_PLACEHOLDER_CACHE;
    let mut lock = mutex.lock().expect("acquire index name lock");
    match lock.entry(len) {
        Entry::Vacant(e) => {
            let signature = Box::new(
                (0..len)
                    .map(|idx| Box::new(format!("{}", idx)).leak() as &str)
                    .collect::<Vec<_>>(),
            )
            .leak();
            e.insert(signature);
            Ok(signature)
        },
        Entry::Occupied(e) => Ok(e.get()),
    }
}

/// enum signer {
///     Master { account: address },
///     Permissioned { account: address, permissions_address: address },
/// }
/// enum variant tag for a master signer.
pub const MASTER_SIGNER_VARIANT: u16 = 0;
/// enum variant tag for a permissioned signer.
pub const PERMISSIONED_SIGNER_VARIANT: u16 = 1;
/// field offset of a master account address in a enum encoded signer.
pub const MASTER_ADDRESS_FIELD_OFFSET: usize = 1;
/// field offset of a permission storage address in a enum encoded permission signer.
pub const PERMISSION_ADDRESS_FIELD_OFFSET: usize = 2;

#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(
    any(test, feature = "fuzzing"),
    derive(arbitrary::Arbitrary, dearbitrary::Dearbitrary)
)]
pub enum MoveStruct {
    /// The representation used by the MoveVM
    Runtime(Vec<MoveValue>),
    /// The representation used by the MoveVM for a variant value.
    RuntimeVariant(u16, Vec<MoveValue>),
    /// A decorated representation with human-readable field names
    WithFields(Vec<(Identifier, MoveValue)>),
    /// An even more decorated representation with both types and human-readable field names
    WithTypes {
        _type_: StructTag,
        _fields: Vec<(Identifier, MoveValue)>,
    },
    /// A decorated representation of a variant, with the variant name, tag value, and field values.
    WithVariantFields(Identifier, u16, Vec<(Identifier, MoveValue)>),
}

#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(
    any(test, feature = "fuzzing"),
    derive(arbitrary::Arbitrary, dearbitrary::Dearbitrary)
)]
pub enum MoveValue {
    U8(u8),
    U64(u64),
    U128(u128),
    Bool(bool),
    Address(AccountAddress),
    Vector(Vec<MoveValue>),
    Struct(MoveStruct),
    // TODO: Signer is only used to construct arguments easily.
    //       Refactor the code to reflect the new permissioned signer schema.
    Signer(AccountAddress),
    // NOTE: Added in bytecode version v6, do not reorder!
    U16(u16),
    U32(u32),
    U256(u256::U256),
    // Added in bytecode version v8
    Closure(Box<MoveClosure>),
}

/// A layout associated with a named field
#[derive(Debug, Clone, Hash, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(
    any(test, feature = "fuzzing"),
    derive(arbitrary::Arbitrary, dearbitrary::Dearbitrary)
)]
pub struct MoveFieldLayout {
    pub name: Identifier,
    pub layout: MoveTypeLayout,
}

impl MoveFieldLayout {
    pub fn new(name: Identifier, layout: MoveTypeLayout) -> Self {
        Self { name, layout }
    }
}

#[derive(Debug, Clone, Hash, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(
    any(test, feature = "fuzzing"),
    derive(arbitrary::Arbitrary, dearbitrary::Dearbitrary)
)]
pub struct MoveVariantLayout {
    pub name: Identifier,
    pub fields: Vec<MoveFieldLayout>,
}

#[derive(Debug, Clone, Hash, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(
    any(test, feature = "fuzzing"),
    derive(arbitrary::Arbitrary, dearbitrary::Dearbitrary)
)]
pub enum MoveStructLayout {
    /// The representation used by the MoveVM for plain structs.
    Runtime(Vec<MoveTypeLayout>),
    /// The representation used by the MoveVM for plain struct variants.
    RuntimeVariants(Vec<Vec<MoveTypeLayout>>),
    /// A decorated representation with human-readable field names that can be used by clients.
    Decorated {
        struct_tag: StructTag,
        fields: Vec<MoveFieldLayout>,
    },
    /// A decorated representation of struct variants, containing variant and field names.
    DecoratedVariants {
        struct_tag: StructTag,
        variants: Vec<MoveVariantLayout>,
    },
}

/// Used to distinguish between aggregators ans snapshots.
#[derive(Debug, Clone, Hash, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(
    any(test, feature = "fuzzing"),
    derive(arbitrary::Arbitrary, dearbitrary::Dearbitrary)
)]
pub enum IdentifierMappingKind {
    Aggregator,
    Snapshot,
    DerivedString,
}

impl IdentifierMappingKind {
    /// If the struct identifier has a special mapping, return it.
    pub fn from_ident(
        module_id: &ModuleId,
        struct_id: &Identifier,
    ) -> Option<IdentifierMappingKind> {
        if module_id.address().eq(&AccountAddress::ONE)
            && module_id.name().eq(ident_str!("aggregator_v2"))
        {
            let ident_str = struct_id.as_ident_str();
            if ident_str.eq(ident_str!("Aggregator")) {
                Some(IdentifierMappingKind::Aggregator)
            } else if ident_str.eq(ident_str!("AggregatorSnapshot")) {
                Some(IdentifierMappingKind::Snapshot)
            } else if ident_str.eq(ident_str!("DerivedStringSnapshot")) {
                Some(IdentifierMappingKind::DerivedString)
            } else {
                None
            }
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Hash, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(
    any(test, feature = "fuzzing"),
    derive(arbitrary::Arbitrary),
    derive(dearbitrary::Dearbitrary)
)]
pub enum MoveTypeLayout {
    #[serde(rename(serialize = "bool", deserialize = "bool"))]
    Bool,
    #[serde(rename(serialize = "u8", deserialize = "u8"))]
    U8,
    #[serde(rename(serialize = "u64", deserialize = "u64"))]
    U64,
    #[serde(rename(serialize = "u128", deserialize = "u128"))]
    U128,
    #[serde(rename(serialize = "address", deserialize = "address"))]
    Address,
    #[serde(rename(serialize = "vector", deserialize = "vector"))]
    Vector(Box<MoveTypeLayout>),
    #[serde(rename(serialize = "struct", deserialize = "struct"))]
    Struct(MoveStructLayout),
    #[serde(rename(serialize = "signer", deserialize = "signer"))]
    Signer,

    // NOTE: Added in bytecode version v6, do not reorder!
    #[serde(rename(serialize = "u16", deserialize = "u16"))]
    U16,
    #[serde(rename(serialize = "u32", deserialize = "u32"))]
    U32,
    #[serde(rename(serialize = "u256", deserialize = "u256"))]
    U256,

    /// Represents an extension to layout which can be used by the runtime
    /// (MoveVM) to allow for custom serialization and deserialization of
    /// values.
    // TODO[agg_v2](cleanup): Shift to registry based implementation and
    //                        come up with a better name.
    // TODO[agg_v2](?): Do we need a layout here if we have custom serde
    //                  implementations available?
    Native(IdentifierMappingKind, Box<MoveTypeLayout>),

    // Added in bytecode version v8
    #[serde(rename(serialize = "fun", deserialize = "fun"))]
    Function,
}

impl MoveTypeLayout {
    /// Determines whether the layout is serialization compatible with the other layout
    /// (that is, any value serialized with this layout can be deserialized by the other).
    pub fn is_compatible_with(&self, other: &Self) -> bool {
        use MoveTypeLayout::*;
        match (self, other) {
            (Vector(t1), Vector(t2)) => t1.is_compatible_with(t2),
            (Struct(s1), Struct(s2)) => s1.is_compatible_with(s2),
            // For all other cases, equality is used
            (t1, t2) => t1 == t2,
        }
    }

    pub fn is_compatible_with_slice(this: &[Self], other: &[Self]) -> bool {
        this.len() == other.len()
            && this
                .iter()
                .zip(other)
                .all(|(t1, t2)| t1.is_compatible_with(t2))
    }
}

impl MoveValue {
    pub fn simple_deserialize(blob: &[u8], ty: &MoveTypeLayout) -> AResult<Self> {
        Ok(bcs::from_bytes_seed(ty, blob)?)
    }

    pub fn simple_serialize(&self) -> Option<Vec<u8>> {
        bcs::to_bytes(self).ok()
    }

    pub fn closure(c: MoveClosure) -> MoveValue {
        Self::Closure(Box::new(c))
    }

    pub fn vector_u8(v: Vec<u8>) -> Self {
        MoveValue::Vector(v.into_iter().map(MoveValue::U8).collect())
    }

    /// Converts the `Vec<MoveValue>` to a `Vec<u8>` if the inner `MoveValue` is a `MoveValue::U8`,
    /// or returns an error otherwise.
    pub fn vec_to_vec_u8(vec: Vec<MoveValue>) -> AResult<Vec<u8>> {
        let mut vec_u8 = Vec::with_capacity(vec.len());

        for byte in vec {
            match byte {
                MoveValue::U8(u8) => {
                    vec_u8.push(u8);
                },
                _ => {
                    return Err(anyhow!(
                        "Expected inner MoveValue in Vec<MoveValue> to be a MoveValue::U8"
                            .to_string(),
                    ));
                },
            }
        }
        Ok(vec_u8)
    }

    pub fn vector_address(v: Vec<AccountAddress>) -> Self {
        MoveValue::Vector(v.into_iter().map(MoveValue::Address).collect())
    }

    pub fn decorate(self, layout: &MoveTypeLayout) -> Self {
        match (self, layout) {
            (MoveValue::Struct(s), MoveTypeLayout::Struct(l)) => MoveValue::Struct(s.decorate(l)),
            (MoveValue::Vector(vals), MoveTypeLayout::Vector(t)) => {
                MoveValue::Vector(vals.into_iter().map(|v| v.decorate(t)).collect())
            },
            (v, _) => v,
        }
    }

    pub fn undecorate(self) -> Self {
        match self {
            Self::Struct(s) => MoveValue::Struct(s.undecorate()),
            Self::Vector(vals) => {
                MoveValue::Vector(vals.into_iter().map(MoveValue::undecorate).collect())
            },
            v => v,
        }
    }
}

pub fn serialize_values<'a, I>(vals: I) -> Vec<Vec<u8>>
where
    I: IntoIterator<Item = &'a MoveValue>,
{
    vals.into_iter()
        .map(|val| {
            val.simple_serialize()
                .expect("serialization should succeed")
        })
        .collect()
}

impl MoveStruct {
    pub fn new(value: Vec<MoveValue>) -> Self {
        Self::Runtime(value)
    }

    pub fn new_variant(tag: u16, value: Vec<MoveValue>) -> Self {
        Self::RuntimeVariant(tag, value)
    }

    pub fn with_fields(values: Vec<(Identifier, MoveValue)>) -> Self {
        Self::WithFields(values)
    }

    pub fn with_types(type_: StructTag, fields: Vec<(Identifier, MoveValue)>) -> Self {
        Self::WithTypes {
            _type_: type_,
            _fields: fields,
        }
    }

    pub fn simple_deserialize(blob: &[u8], ty: &MoveStructLayout) -> AResult<Self> {
        Ok(bcs::from_bytes_seed(ty, blob)?)
    }

    pub fn decorate(self, layout: &MoveStructLayout) -> Self {
        match (self, layout) {
            (MoveStruct::Runtime(vals), MoveStructLayout::Decorated { struct_tag, fields }) => {
                MoveStruct::WithTypes {
                    _type_: struct_tag.clone(),
                    _fields: vals
                        .into_iter()
                        .zip(fields)
                        .map(|(v, l)| (l.name.clone(), v.decorate(&l.layout)))
                        .collect(),
                }
            },
            (
                MoveStruct::RuntimeVariant(tag, vals),
                MoveStructLayout::DecoratedVariants { variants, .. },
            ) if (tag as usize) < variants.len() => {
                let MoveVariantLayout { name, fields } = &variants[tag as usize];
                MoveStruct::WithVariantFields(
                    name.clone(),
                    tag,
                    vals.into_iter()
                        .zip(fields)
                        .map(|(v, l)| (l.name.clone(), v.decorate(&l.layout)))
                        .collect(),
                )
            },
            (MoveStruct::WithFields(vals), MoveStructLayout::Decorated { struct_tag, fields }) => {
                MoveStruct::WithTypes {
                    _type_: struct_tag.clone(),
                    _fields: vals
                        .into_iter()
                        .zip(fields)
                        .map(|((fld, v), l)| (fld, v.decorate(&l.layout)))
                        .collect(),
                }
            },

            (v, _) => v, // already decorated (or invalid, in which case we ignore this as
                         // we cannot return a Result here)
        }
    }

    pub fn optional_variant_and_fields(&self) -> (Option<u16>, &[MoveValue]) {
        match self {
            Self::Runtime(vals) => (None, vals),
            Self::RuntimeVariant(tag, vals) => (Some(*tag), vals),
            Self::WithFields(_) | Self::WithTypes { .. } | Self::WithVariantFields(..) => {
                // It's not possible to implement this without changing the return type, and thus
                // panicking is the best move
                panic!("Getting fields for decorated representation")
            },
        }
    }

    pub fn into_optional_variant_and_fields(self) -> (Option<u16>, Vec<MoveValue>) {
        match self {
            Self::Runtime(vals) => (None, vals),
            Self::RuntimeVariant(tag, vals) => (Some(tag), vals),
            Self::WithFields(fields)
            | Self::WithTypes {
                _fields: fields, ..
            } => (None, fields.into_iter().map(|(_, f)| f).collect()),
            Self::WithVariantFields(_, tag, fields) => {
                (Some(tag), fields.into_iter().map(|(_, f)| f).collect())
            },
        }
    }

    pub fn undecorate(self) -> Self {
        match self {
            MoveStruct::WithFields(fields)
            | MoveStruct::WithTypes {
                _fields: fields, ..
            } => Self::Runtime(
                fields
                    .into_iter()
                    .map(|(_, v)| MoveValue::undecorate(v))
                    .collect(),
            ),
            MoveStruct::WithVariantFields(_, tag, fields) => Self::RuntimeVariant(
                tag,
                fields
                    .into_iter()
                    .map(|(_, v)| MoveValue::undecorate(v))
                    .collect(),
            ),
            _ => self,
        }
    }
}

impl MoveStructLayout {
    pub fn new(types: Vec<MoveTypeLayout>) -> Self {
        Self::Runtime(types)
    }

    pub fn new_variants(types: Vec<Vec<MoveTypeLayout>>) -> Self {
        Self::RuntimeVariants(types)
    }

    pub fn decorated(struct_tag: StructTag, fields: Vec<MoveFieldLayout>) -> Self {
        Self::Decorated { struct_tag, fields }
    }

    pub fn decorated_variants(struct_tag: StructTag, variants: Vec<MoveVariantLayout>) -> Self {
        Self::DecoratedVariants {
            struct_tag,
            variants,
        }
    }

    /// Determines whether the layout is serialization compatible with the other layout
    /// (that is, any value serialized with this layout can be deserialized by the other).
    /// This only will consider runtime variants, decorated variants are only compatible
    /// if equal.
    pub fn is_compatible_with(&self, other: &Self) -> bool {
        use MoveStructLayout::*;
        match (self, other) {
            (RuntimeVariants(variants1), RuntimeVariants(variants2)) => {
                variants1.len() <= variants2.len()
                    && variants1.iter().zip(variants2).all(|(fields1, fields2)| {
                        MoveTypeLayout::is_compatible_with_slice(fields1, fields2)
                    })
            },
            (Runtime(fields1), Runtime(fields2)) => {
                fields1.len() == fields2.len()
                    && fields1
                        .iter()
                        .zip(fields2)
                        .all(|(t1, t2)| t1.is_compatible_with(t2))
            },
            // All other cases require equality
            (s1, s2) => s1 == s2,
        }
    }

    pub fn fields(&self, variant: Option<usize>) -> &[MoveTypeLayout] {
        match self {
            Self::Runtime(vals) => vals,
            Self::RuntimeVariants(variants) => match variant {
                Some(idx) if idx < variants.len() => &variants[idx],
                _ => {
                    // API does not allow to return error, return empty fields instead of crashing
                    &[]
                },
            },
            Self::Decorated { .. } | Self::DecoratedVariants { .. } => {
                // It's not possible to implement this without changing the return type, and some
                // performance-critical VM serialization code uses the Runtime case of this.
                // panicking is the best move
                panic!("Getting fields for decorated representation")
            },
        }
    }

    pub fn into_fields(self, variant: Option<usize>) -> Vec<MoveTypeLayout> {
        match self {
            Self::Runtime(vals) => vals,
            Self::RuntimeVariants(mut variants) => {
                match variant {
                    Some(idx) if idx < variants.len() => variants.remove(idx),
                    _ => {
                        // be on the robust side and remove empty vec instead of crash
                        vec![]
                    },
                }
            },
            Self::Decorated { fields, .. } => fields.into_iter().map(|f| f.layout).collect(),
            Self::DecoratedVariants { mut variants, .. } => match variant {
                Some(idx) if idx < variants.len() => variants
                    .remove(idx)
                    .fields
                    .into_iter()
                    .map(|f| f.layout)
                    .collect(),
                _ => {
                    // be on the robust side and return empty vec instead of crash
                    vec![]
                },
            },
        }
    }

    pub fn signer_serialization_layout() -> Self {
        MoveStructLayout::RuntimeVariants(vec![vec![MoveTypeLayout::Address], vec![
            MoveTypeLayout::Address,
            MoveTypeLayout::Address,
        ]])
    }
}

impl<'d> serde::de::DeserializeSeed<'d> for &MoveTypeLayout {
    type Value = MoveValue;

    fn deserialize<D: serde::de::Deserializer<'d>>(
        self,
        deserializer: D,
    ) -> Result<Self::Value, D::Error> {
        match self {
            MoveTypeLayout::Bool => bool::deserialize(deserializer).map(MoveValue::Bool),
            MoveTypeLayout::U8 => u8::deserialize(deserializer).map(MoveValue::U8),
            MoveTypeLayout::U16 => u16::deserialize(deserializer).map(MoveValue::U16),
            MoveTypeLayout::U32 => u32::deserialize(deserializer).map(MoveValue::U32),
            MoveTypeLayout::U64 => u64::deserialize(deserializer).map(MoveValue::U64),
            MoveTypeLayout::U128 => u128::deserialize(deserializer).map(MoveValue::U128),
            MoveTypeLayout::U256 => u256::U256::deserialize(deserializer).map(MoveValue::U256),
            MoveTypeLayout::Address => {
                AccountAddress::deserialize(deserializer).map(MoveValue::Address)
            },
            MoveTypeLayout::Signer => Err(D::Error::custom("cannot deserialize signer")),
            MoveTypeLayout::Struct(ty) => Ok(MoveValue::Struct(ty.deserialize(deserializer)?)),
            MoveTypeLayout::Function => Ok(MoveValue::Closure(Box::new(
                deserializer.deserialize_seq(ClosureVisitor)?,
            ))),
            MoveTypeLayout::Vector(layout) => Ok(MoveValue::Vector(
                deserializer.deserialize_seq(VectorElementVisitor(layout))?,
            )),

            // This layout is only used by MoveVM, so we do not expect to see it here.
            MoveTypeLayout::Native(..) => {
                Err(D::Error::custom("Unsupported layout for Move value"))
            },
        }
    }
}

struct VectorElementVisitor<'a>(&'a MoveTypeLayout);

impl<'d> serde::de::Visitor<'d> for VectorElementVisitor<'_> {
    type Value = Vec<MoveValue>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("Vector")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'d>,
    {
        let mut vals = Vec::new();
        while let Some(elem) = seq.next_element_seed(self.0)? {
            vals.push(elem)
        }
        Ok(vals)
    }
}

struct DecoratedStructFieldVisitor<'a>(&'a [MoveFieldLayout]);

impl<'d> serde::de::Visitor<'d> for DecoratedStructFieldVisitor<'_> {
    type Value = Vec<(Identifier, MoveValue)>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("Struct")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'d>,
    {
        let mut vals = Vec::new();
        for (i, layout) in self.0.iter().enumerate() {
            match seq.next_element_seed(layout)? {
                Some(elem) => vals.push(elem),
                None => return Err(A::Error::invalid_length(i, &self)),
            }
        }
        Ok(vals)
    }
}

struct StructFieldVisitor<'a>(&'a [MoveTypeLayout]);

impl<'d> serde::de::Visitor<'d> for StructFieldVisitor<'_> {
    type Value = Vec<MoveValue>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("Struct")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'d>,
    {
        let mut val = Vec::new();
        for (i, field_type) in self.0.iter().enumerate() {
            match seq.next_element_seed(field_type)? {
                Some(elem) => val.push(elem),
                None => return Err(A::Error::invalid_length(i, &self)),
            }
        }
        Ok(val)
    }
}

struct StructVariantVisitor<'a>(&'a [Vec<MoveTypeLayout>]);

impl<'d> serde::de::Visitor<'d> for StructVariantVisitor<'_> {
    type Value = (u16, Vec<MoveValue>);

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("Variant")
    }

    fn visit_enum<A>(self, data: A) -> Result<Self::Value, A::Error>
    where
        A: EnumAccess<'d>,
    {
        let (tag, rest) = data.variant()?;
        if tag as usize >= self.0.len() {
            Err(A::Error::invalid_length(0, &self))
        } else {
            let fields = &self.0[tag as usize];
            match fields.len() {
                0 => {
                    rest.unit_variant()?;
                    Ok((tag, vec![]))
                },
                1 => {
                    let value = rest.newtype_variant_seed(&fields[0])?;
                    Ok((tag, vec![value]))
                },
                _ => {
                    let values = rest.tuple_variant(fields.len(), StructFieldVisitor(fields))?;
                    Ok((tag, values))
                },
            }
        }
    }
}

impl<'d> serde::de::DeserializeSeed<'d> for &MoveFieldLayout {
    type Value = (Identifier, MoveValue);

    fn deserialize<D: serde::de::Deserializer<'d>>(
        self,
        deserializer: D,
    ) -> Result<Self::Value, D::Error> {
        Ok((self.name.clone(), self.layout.deserialize(deserializer)?))
    }
}

impl<'d> serde::de::DeserializeSeed<'d> for &MoveStructLayout {
    type Value = MoveStruct;

    fn deserialize<D: serde::de::Deserializer<'d>>(
        self,
        deserializer: D,
    ) -> Result<Self::Value, D::Error> {
        match self {
            MoveStructLayout::Runtime(layout) => {
                let fields =
                    deserializer.deserialize_tuple(layout.len(), StructFieldVisitor(layout))?;
                Ok(MoveStruct::Runtime(fields))
            },
            MoveStructLayout::RuntimeVariants(variants) => {
                if variants.len() > (u16::MAX as usize) {
                    return Err(D::Error::custom("variant count out of range"));
                }
                let variant_names = variant_name_placeholder(variants.len())
                    .map_err(|e| D::Error::custom(format!("{}", e)))?;
                let (tag, fields) = deserializer.deserialize_enum(
                    MOVE_ENUM_NAME,
                    variant_names,
                    StructVariantVisitor(variants),
                )?;
                Ok(MoveStruct::RuntimeVariant(tag, fields))
            },
            MoveStructLayout::Decorated {
                struct_tag,
                fields: layout,
            } => {
                let fields = deserializer
                    .deserialize_tuple(layout.len(), DecoratedStructFieldVisitor(layout))?;
                Ok(MoveStruct::WithTypes {
                    _type_: struct_tag.clone(),
                    _fields: fields,
                })
            },
            MoveStructLayout::DecoratedVariants { variants, .. } => {
                // Downgrade the decorated variants to simple layouts to deserialize the fields.
                let variant_names = variant_name_placeholder(variants.len())
                    .map_err(|e| D::Error::custom(format!("{}", e)))?;
                let (tag, fields) = deserializer.deserialize_enum(
                    MOVE_ENUM_NAME,
                    variant_names,
                    StructVariantVisitor(
                        &variants
                            .iter()
                            .map(|v| v.fields.iter().map(|f| f.layout.clone()).collect())
                            .collect::<Vec<_>>(),
                    ),
                )?;
                // Now decorate the raw value. This is not optimally efficient but
                // decorated values should not be in the execution path.
                Ok(MoveStruct::RuntimeVariant(tag, fields).decorate(self))
            },
        }
    }
}

impl serde::Serialize for MoveValue {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            MoveValue::Struct(s) => s.serialize(serializer),
            MoveValue::Closure(c) => c.serialize(serializer),
            MoveValue::Bool(b) => serializer.serialize_bool(*b),
            MoveValue::U8(i) => serializer.serialize_u8(*i),
            MoveValue::U16(i) => serializer.serialize_u16(*i),
            MoveValue::U32(i) => serializer.serialize_u32(*i),
            MoveValue::U64(i) => serializer.serialize_u64(*i),
            MoveValue::U128(i) => serializer.serialize_u128(*i),
            MoveValue::U256(i) => i.serialize(serializer),
            MoveValue::Address(a) => a.serialize(serializer),
            MoveValue::Signer(a) => {
                // Runtime representation of signer looks following:
                // enum signer {
                //     Master { account: address },
                //     Permissioned { account: address, permissions_address: address },
                // }
                MoveStruct::new_variant(MASTER_SIGNER_VARIANT, vec![MoveValue::Address(*a)])
                    .serialize(serializer)
            },
            MoveValue::Vector(v) => {
                let mut t = serializer.serialize_seq(Some(v.len()))?;
                for val in v {
                    t.serialize_element(val)?;
                }
                t.end()
            },
        }
    }
}

struct MoveFields<'a>(&'a [(Identifier, MoveValue)]);

impl serde::Serialize for MoveFields<'_> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut t = serializer.serialize_map(Some(self.0.len()))?;
        for (f, v) in self.0.iter() {
            t.serialize_entry(f, v)?;
        }
        t.end()
    }
}

impl serde::Serialize for MoveStruct {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Runtime(values) => {
                let mut t = serializer.serialize_tuple(values.len())?;
                for v in values.iter() {
                    t.serialize_element(v)?;
                }
                t.end()
            },
            Self::RuntimeVariant(tag, values) => {
                // Variants need to be serialized as sequences, as the size is not statically known.
                let tag_idx = *tag as usize;
                let variant_tag = tag_idx as u32;
                let variant_names = variant_name_placeholder((tag + 1) as usize)
                    .map_err(|e| serde::ser::Error::custom(format!("{}", e)))?;
                let variant_name = variant_names[tag_idx];
                match values.len() {
                    0 => {
                        serializer.serialize_unit_variant(MOVE_ENUM_NAME, variant_tag, variant_name)
                    },
                    1 => serializer.serialize_newtype_variant(
                        MOVE_ENUM_NAME,
                        variant_tag,
                        variant_name,
                        &values[0],
                    ),
                    _ => {
                        let mut t = serializer.serialize_tuple_variant(
                            MOVE_ENUM_NAME,
                            variant_tag,
                            variant_name,
                            values.len(),
                        )?;
                        for v in values {
                            t.serialize_field(v)?
                        }
                        t.end()
                    },
                }
            },
            Self::WithFields(fields) => MoveFields(fields).serialize(serializer),
            Self::WithTypes {
                _type_: type_,
                _fields: fields,
            } => {
                // Serialize a Move struct as Serde struct type named `struct `with two fields named `type` and `fields`.
                // `fields` will get serialized as a Serde map.
                // Unfortunately, we can't serialize this in the logical way: as a Serde struct named `type` with a field for
                // each of `fields` because serde insists that struct and field names be `'static &str`'s
                let mut t = serializer.serialize_struct(MOVE_STRUCT_NAME, 2)?;
                // serialize type as string (e.g., 0x0::ModuleName::StructName<TypeArg1,TypeArg2>) instead of (e.g.
                // { address: 0x0...0, module: ModuleName, name: StructName, type_args: [TypeArg1, TypeArg2]})
                t.serialize_field(MOVE_STRUCT_TYPE, &type_.to_string())?;
                t.serialize_field(MOVE_STRUCT_FIELDS, &MoveFields(fields))?;
                t.end()
            },
            Self::WithVariantFields(name, _tag, fields) => {
                // Serialize a variant as Serde struct name `variant` with two fields `name` and
                // `fields`.
                let mut t = serializer.serialize_struct(MOVE_VARIANT_NAME, 2)?;
                t.serialize_field(MOVE_VARIANT_NAME_FIELD, &name.to_string())?;
                t.serialize_field(MOVE_STRUCT_FIELDS, &MoveFields(fields))?;
                t.end()
            },
        }
    }
}

impl fmt::Display for MoveFieldLayout {
    fn fmt(&self, f: &mut fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}: {}", self.name, self.layout)
    }
}

impl fmt::Display for MoveTypeLayout {
    fn fmt(&self, f: &mut fmt::Formatter) -> std::fmt::Result {
        use MoveTypeLayout::*;
        match self {
            Bool => write!(f, "bool"),
            U8 => write!(f, "u8"),
            U16 => write!(f, "u16"),
            U32 => write!(f, "u32"),
            U64 => write!(f, "u64"),
            U128 => write!(f, "u128"),
            U256 => write!(f, "u256"),
            Address => write!(f, "address"),
            Vector(typ) => write!(f, "vector<{}>", typ),
            Struct(s) => fmt::Display::fmt(s, f),
            Function => write!(f, "function"),
            Signer => write!(f, "signer"),
            // TODO[agg_v2](cleanup): consider printing the tag as well.
            Native(_, typ) => write!(f, "native<{}>", typ),
        }
    }
}

impl fmt::Display for MoveStructLayout {
    fn fmt(&self, f: &mut fmt::Formatter) -> std::fmt::Result {
        write!(f, "{{ ")?;
        match self {
            Self::Runtime(layouts) => {
                for (i, l) in layouts.iter().enumerate() {
                    write!(f, "{}: {}, ", i, l)?
                }
            },
            Self::RuntimeVariants(variants) => {
                for (i, v) in variants.iter().enumerate() {
                    write!(f, "#{}{{", i)?;
                    for (i, l) in v.iter().enumerate() {
                        write!(f, "{}: {}, ", i, l)?
                    }
                    write!(f, "}}")?;
                }
            },
            Self::Decorated { struct_tag, fields } => {
                write!(f, "Type: {}", struct_tag)?;
                write!(f, "Fields:")?;
                for field in fields {
                    write!(f, "{}, ", field)?
                }
            },
            Self::DecoratedVariants {
                struct_tag: _,
                variants,
            } => {
                for v in variants {
                    write!(f, "{}{{", v.name)?;
                    for layout in &v.fields {
                        write!(f, "{}", layout)?;
                    }
                    write!(f, "}}")?;
                }
            },
        }
        write!(f, "}}")
    }
}

impl TryInto<TypeTag> for &MoveTypeLayout {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<TypeTag, Self::Error> {
        Ok(match self {
            MoveTypeLayout::Address => TypeTag::Address,
            MoveTypeLayout::Bool => TypeTag::Bool,
            MoveTypeLayout::U8 => TypeTag::U8,
            MoveTypeLayout::U16 => TypeTag::U16,
            MoveTypeLayout::U32 => TypeTag::U32,
            MoveTypeLayout::U64 => TypeTag::U64,
            MoveTypeLayout::U128 => TypeTag::U128,
            MoveTypeLayout::U256 => TypeTag::U256,
            MoveTypeLayout::Signer => TypeTag::Signer,
            MoveTypeLayout::Vector(v) => TypeTag::Vector(Box::new(v.as_ref().try_into()?)),
            MoveTypeLayout::Struct(v) => TypeTag::Struct(Box::new(v.try_into()?)),

            // For function values, we cannot reconstruct the tag because we do not know the
            // argument and return types.
            MoveTypeLayout::Function => {
                bail!("Function layout cannot be constructed from type tag")
            },

            // Native layout variant is only used by MoveVM, and is irrelevant
            // for type tags which are used to key resources in the global state.
            MoveTypeLayout::Native(..) => bail!("Unsupported layout for type tag"),
        })
    }
}

impl TryInto<StructTag> for &MoveStructLayout {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<StructTag, Self::Error> {
        use MoveStructLayout::*;
        match self {
            Runtime(..) | RuntimeVariants(..) => bail!(
                "Invalid MoveTypeLayout -> StructTag conversion--needed MoveLayoutType::WithTypes"
            ),
            Decorated { struct_tag, .. } | DecoratedVariants { struct_tag, .. } => {
                Ok(struct_tag.clone())
            },
        }
    }
}

impl fmt::Display for MoveValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MoveValue::U8(u) => write!(f, "{}u8", u),
            MoveValue::U16(u) => write!(f, "{}u16", u),
            MoveValue::U32(u) => write!(f, "{}u32", u),
            MoveValue::U64(u) => write!(f, "{}u64", u),
            MoveValue::U128(u) => write!(f, "{}u128", u),
            MoveValue::U256(u) => write!(f, "{}u256", u),
            MoveValue::Bool(false) => write!(f, "false"),
            MoveValue::Bool(true) => write!(f, "true"),
            MoveValue::Address(a) => write!(f, "{}", a.to_hex_literal()),
            MoveValue::Signer(a) => write!(f, "signer({})", a.to_hex_literal()),
            MoveValue::Vector(v) => fmt_list(f, "vector[", v, "]"),
            MoveValue::Struct(s) => fmt::Display::fmt(s, f),
            MoveValue::Closure(c) => fmt::Display::fmt(c, f),
        }
    }
}

impl fmt::Display for MoveStruct {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MoveStruct::Runtime(v) => fmt_list(f, "struct[", v, "]"),
            MoveStruct::RuntimeVariant(tag, v) => fmt_list(f, &format!("variant#{}[", tag), v, "]"),
            MoveStruct::WithFields(fields) => {
                fmt_list(f, "{", fields.iter().map(DisplayFieldBinding), "}")
            },
            MoveStruct::WithTypes {
                _type_: type_,
                _fields: fields,
            } => {
                fmt::Display::fmt(type_, f)?;
                fmt_list(f, " {", fields.iter().map(DisplayFieldBinding), "}")
            },
            MoveStruct::WithVariantFields(name, _tag, fields) => fmt_list(
                f,
                &format!("{}{{", name),
                fields.iter().map(DisplayFieldBinding),
                "}",
            ),
        }
    }
}

struct DisplayFieldBinding<'a>(&'a (Identifier, MoveValue));

impl fmt::Display for DisplayFieldBinding<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let DisplayFieldBinding((field, value)) = self;
        write!(f, "{}: {}", field, value)
    }
}

fn fmt_list<T: fmt::Display>(
    f: &mut fmt::Formatter<'_>,
    begin: &str,
    items: impl IntoIterator<Item = T>,
    end: &str,
) -> fmt::Result {
    write!(f, "{}", begin)?;
    let mut items = items.into_iter();
    if let Some(x) = items.next() {
        write!(f, "{}", x)?;
        for x in items {
            write!(f, ", {}", x)?;
        }
    }
    write!(f, "{}", end)?;
    Ok(())
}
