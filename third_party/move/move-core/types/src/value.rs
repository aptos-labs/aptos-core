// Parts of the file are Copyright (c) The Diem Core Contributors
// Parts of the file are Copyright (c) The Move Contributors
// Parts of the file are Copyright (c) Aptos Foundation
// All Aptos Foundation code and content is licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

// The below is to deal with a strange problem with derive(Dearbitrary), which creates warnings
// of unused variables and unused assignments in derived code which cannot be turned off by
// applying the attribute just at the type in question. (Here, MoveStructLayout.)
#![allow(unused_variables, unused_assignments)]

use crate::{
    account_address::AccountAddress,
    function::{ClosureVisitor, MoveClosure},
    ident_str,
    identifier::Identifier,
    int256,
    language_storage::{ModuleId, StructTag, TypeTag},
};
use anyhow::{anyhow, bail, Result as AResult};
use once_cell::sync::Lazy;
use serde::{
    de::{EnumAccess, Error as DeError, VariantAccess},
    ser::{SerializeMap, SerializeSeq, SerializeStruct, SerializeTuple, SerializeTupleVariant},
    Deserialize, Serialize,
};
use std::{
    collections::{HashMap, HashSet},
    convert::TryInto,
    fmt::{self, Debug},
    sync::Arc,
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
/// variants. This static is used to generate those names, and contains signatures of
/// the form `["0", "1", "2", ..]`.
static VARIANT_NAME_PLACEHOLDERS: Lazy<[&'static str; VARIANT_COUNT_MAX as usize]> =
    Lazy::new(|| {
        std::array::from_fn(|i| Box::leak(format!("{i}").into_boxed_str()) as &'static str)
    });

/// Returns variant name placeholders for providing dummy names in serde serialization.
pub fn variant_name_placeholder(len: usize) -> Result<&'static [&'static str], anyhow::Error> {
    if len > VARIANT_COUNT_MAX as usize {
        bail!("variant count is restricted to {}", VARIANT_COUNT_MAX);
    }
    Ok(&VARIANT_NAME_PLACEHOLDERS[..len])
}

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
    // A signer carries only an account address; its runtime representation is a bare address.
    Signer(AccountAddress),
    // NOTE: Added in bytecode version v6, do not reorder!
    U16(u16),
    U32(u32),
    U256(int256::U256),
    // Added in bytecode version v8
    Closure(Box<MoveClosure>),
    // Added in bytecode version v9
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    I128(i128),
    I256(int256::I256),
}

/// A layout associated with a named field
#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(
    any(test, feature = "fuzzing"),
    derive(arbitrary::Arbitrary, dearbitrary::Dearbitrary)
)]
pub struct MoveVariantLayout {
    pub name: Identifier,
    pub fields: Vec<MoveFieldLayout>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(
    any(test, feature = "fuzzing"),
    derive(arbitrary::Arbitrary, dearbitrary::Dearbitrary)
)]
pub enum MoveStructLayout {
    /// The representation used by the MoveVM for plain structs
    Runtime(Vec<MoveTypeLayout>),
    /// The representation used by the MoveVM for plain struct variants.
    RuntimeVariants(Vec<Vec<MoveTypeLayout>>),
    /// A decorated representation with human-readable field names that can be used by clients
    WithFields(Vec<MoveFieldLayout>),
    /// An even more decorated representation which carries the tag of struct this layout belongs
    /// to. This allows specialized rendering for framework types like strings.
    WithTypes {
        type_: StructTag,
        fields: Vec<MoveFieldLayout>,
    },
    /// A decorated representation of struct variants, containing variant and field names.
    /// Like WithTypes, this carries the type tag for proper type identification.
    WithVariants {
        type_: StructTag,
        variants: Vec<MoveVariantLayout>,
    },
}

/// Used to distinguish between aggregators ans snapshots.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(
    any(test, feature = "fuzzing"),
    derive(arbitrary::Arbitrary, dearbitrary::Dearbitrary)
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
    Struct(Arc<MoveStructLayout>),
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
    // Added in bytecode version v9
    #[serde(rename(serialize = "i8", deserialize = "i8"))]
    I8,
    #[serde(rename(serialize = "i16", deserialize = "i16"))]
    I16,
    #[serde(rename(serialize = "i32", deserialize = "i32"))]
    I32,
    #[serde(rename(serialize = "i64", deserialize = "i64"))]
    I64,
    #[serde(rename(serialize = "i128", deserialize = "i128"))]
    I128,
    #[serde(rename(serialize = "i256", deserialize = "i256"))]
    I256,
}

/// Limit on the number of nodes a [`MoveTypeLayout`] may unfold to when serialized.
pub const MAX_LAYOUT_NODES: u64 = 4096;

/// Returns an error if `layout` unfolds to more than [`MAX_LAYOUT_NODES`] nodes.
/// Use this before serializing a layout sourced from untrusted input to prevent
/// pathological DAGs from expanding into multi-megabyte BCS output.
pub fn check_layout_within_bounds<E: serde::ser::Error>(layout: &MoveTypeLayout) -> Result<(), E> {
    let count = layout.unfolded_node_count();
    if count > MAX_LAYOUT_NODES {
        return Err(E::custom(format!(
            "layout unfolds to {} nodes, exceeding the maximum of {}",
            count, MAX_LAYOUT_NODES
        )));
    }
    Ok(())
}

impl MoveTypeLayout {
    /// Creates a `MoveTypeLayout::Struct` wrapping the given layout in an `Arc`.
    pub fn new_struct(layout: MoveStructLayout) -> Self {
        Self::Struct(Arc::new(layout))
    }

    /// Counts every node in the layout; shared `Arc<MoveStructLayout>`
    /// occurrences are not deduped, each contributing its full subtree.
    pub fn unfolded_node_count(&self) -> u64 {
        layout_unfolded_node_count(self, &mut LayoutCountCache::new())
    }
}

type LayoutCountCache = HashMap<*const MoveStructLayout, u64>;

fn layout_unfolded_node_count(layout: &MoveTypeLayout, cache: &mut LayoutCountCache) -> u64 {
    use MoveTypeLayout::*;
    match layout {
        Bool | U8 | U16 | U32 | U64 | U128 | U256 | I8 | I16 | I32 | I64 | I128 | I256
        | Address | Signer | Function => 1,
        Vector(inner) | Native(_, inner) => {
            1u64.saturating_add(layout_unfolded_node_count(inner, cache))
        },
        Struct(inner) => {
            let ptr = Arc::as_ptr(inner);
            match cache.get(&ptr) {
                Some(cached) => *cached,
                None => {
                    let count =
                        1u64.saturating_add(struct_layout_unfolded_node_count(inner, cache));
                    cache.insert(ptr, count);
                    count
                },
            }
        },
    }
}

fn struct_layout_unfolded_node_count(
    layout: &MoveStructLayout,
    cache: &mut LayoutCountCache,
) -> u64 {
    use MoveStructLayout::*;
    match layout {
        Runtime(fields) => fields.iter().fold(0u64, |acc, l| {
            acc.saturating_add(layout_unfolded_node_count(l, cache))
        }),
        RuntimeVariants(variants) => variants.iter().flat_map(|v| v.iter()).fold(0u64, |acc, l| {
            acc.saturating_add(layout_unfolded_node_count(l, cache))
        }),
        WithFields(fields) | WithTypes { fields, .. } => fields.iter().fold(0u64, |acc, f| {
            acc.saturating_add(layout_unfolded_node_count(&f.layout, cache))
        }),
        WithVariants { variants, .. } => variants
            .iter()
            .flat_map(|v| v.fields.iter())
            .fold(0u64, |acc, f| {
                acc.saturating_add(layout_unfolded_node_count(&f.layout, cache))
            }),
    }
}

// Equality on the layout DAG with pointer-identity memoization on
// `Arc<MoveStructLayout>`. A derived `PartialEq` would compare the inner
// `MoveStructLayout` once per occurrence of a shared `Arc`, expanding the DAG
// into a tree; the impl below short-circuits on `Arc::ptr_eq` and caches
// structurally equal pairs so the work is `O(number of unique node pairs)`.
type LayoutEqCache = HashSet<(*const MoveStructLayout, *const MoveStructLayout)>;

impl PartialEq for MoveTypeLayout {
    fn eq(&self, other: &Self) -> bool {
        layout_eq(self, other, &mut LayoutEqCache::new())
    }
}

impl Eq for MoveTypeLayout {}

impl PartialEq for MoveStructLayout {
    fn eq(&self, other: &Self) -> bool {
        struct_layout_eq(self, other, &mut LayoutEqCache::new())
    }
}

impl Eq for MoveStructLayout {}

impl PartialEq for MoveFieldLayout {
    fn eq(&self, other: &Self) -> bool {
        field_layout_eq(self, other, &mut LayoutEqCache::new())
    }
}

impl Eq for MoveFieldLayout {}

impl PartialEq for MoveVariantLayout {
    fn eq(&self, other: &Self) -> bool {
        variant_layout_eq(self, other, &mut LayoutEqCache::new())
    }
}

impl Eq for MoveVariantLayout {}

fn layout_eq(a: &MoveTypeLayout, b: &MoveTypeLayout, cache: &mut LayoutEqCache) -> bool {
    use MoveTypeLayout::*;
    match (a, b) {
        (Bool, Bool)
        | (Address, Address)
        | (Signer, Signer)
        | (Function, Function)
        | (U8, U8)
        | (U16, U16)
        | (U32, U32)
        | (U64, U64)
        | (U128, U128)
        | (U256, U256)
        | (I8, I8)
        | (I16, I16)
        | (I32, I32)
        | (I64, I64)
        | (I128, I128)
        | (I256, I256) => true,
        (Vector(x), Vector(y)) => layout_eq(x, y, cache),
        (Native(ka, x), Native(kb, y)) => ka == kb && layout_eq(x, y, cache),
        (Struct(x), Struct(y)) => arc_struct_layout_eq(x, y, cache),
        // Listed exhaustively so adding a new variant forces an update here.
        (Bool, _)
        | (Address, _)
        | (Signer, _)
        | (Function, _)
        | (U8, _)
        | (U16, _)
        | (U32, _)
        | (U64, _)
        | (U128, _)
        | (U256, _)
        | (I8, _)
        | (I16, _)
        | (I32, _)
        | (I64, _)
        | (I128, _)
        | (I256, _)
        | (Vector(_), _)
        | (Native(_, _), _)
        | (Struct(_), _) => false,
    }
}

fn arc_struct_layout_eq(
    a: &Arc<MoveStructLayout>,
    b: &Arc<MoveStructLayout>,
    cache: &mut LayoutEqCache,
) -> bool {
    if Arc::ptr_eq(a, b) {
        return true;
    }
    let key = {
        let pa = Arc::as_ptr(a);
        let pb = Arc::as_ptr(b);
        if pa <= pb {
            (pa, pb)
        } else {
            (pb, pa)
        }
    };
    if cache.contains(&key) {
        return true;
    }
    let result = struct_layout_eq(a, b, cache);
    if result {
        cache.insert(key);
    }
    result
}

fn struct_layout_eq(a: &MoveStructLayout, b: &MoveStructLayout, cache: &mut LayoutEqCache) -> bool {
    use MoveStructLayout::*;
    match (a, b) {
        (Runtime(xs), Runtime(ys)) => {
            xs.len() == ys.len() && xs.iter().zip(ys).all(|(x, y)| layout_eq(x, y, cache))
        },
        (RuntimeVariants(xs), RuntimeVariants(ys)) => {
            xs.len() == ys.len()
                && xs.iter().zip(ys).all(|(vx, vy)| {
                    vx.len() == vy.len() && vx.iter().zip(vy).all(|(x, y)| layout_eq(x, y, cache))
                })
        },
        (WithFields(xs), WithFields(ys)) => {
            xs.len() == ys.len() && xs.iter().zip(ys).all(|(x, y)| field_layout_eq(x, y, cache))
        },
        (
            WithTypes {
                type_: ta,
                fields: fa,
            },
            WithTypes {
                type_: tb,
                fields: fb,
            },
        ) => {
            ta == tb
                && fa.len() == fb.len()
                && fa.iter().zip(fb).all(|(x, y)| field_layout_eq(x, y, cache))
        },
        (
            WithVariants {
                type_: ta,
                variants: va,
            },
            WithVariants {
                type_: tb,
                variants: vb,
            },
        ) => {
            ta == tb
                && va.len() == vb.len()
                && va
                    .iter()
                    .zip(vb)
                    .all(|(x, y)| variant_layout_eq(x, y, cache))
        },
        // Listed exhaustively so adding a new variant forces an update here.
        (Runtime(_), _)
        | (RuntimeVariants(_), _)
        | (WithFields(_), _)
        | (WithTypes { .. }, _)
        | (WithVariants { .. }, _) => false,
    }
}

fn field_layout_eq(a: &MoveFieldLayout, b: &MoveFieldLayout, cache: &mut LayoutEqCache) -> bool {
    a.name == b.name && layout_eq(&a.layout, &b.layout, cache)
}

fn variant_layout_eq(
    a: &MoveVariantLayout,
    b: &MoveVariantLayout,
    cache: &mut LayoutEqCache,
) -> bool {
    a.name == b.name
        && a.fields.len() == b.fields.len()
        && a.fields
            .iter()
            .zip(&b.fields)
            .all(|(x, y)| field_layout_eq(x, y, cache))
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
            (MoveStruct::Runtime(vals), MoveStructLayout::WithFields(layouts)) => {
                MoveStruct::WithFields(
                    vals.into_iter()
                        .zip(layouts)
                        .map(|(v, l)| (l.name.clone(), v.decorate(&l.layout)))
                        .collect(),
                )
            },
            (MoveStruct::Runtime(vals), MoveStructLayout::WithTypes { type_, fields }) => {
                MoveStruct::WithTypes {
                    _type_: type_.clone(),
                    _fields: vals
                        .into_iter()
                        .zip(fields)
                        .map(|(v, l)| (l.name.clone(), v.decorate(&l.layout)))
                        .collect(),
                }
            },
            (
                MoveStruct::RuntimeVariant(tag, vals),
                MoveStructLayout::WithVariants { variants, .. },
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
            (MoveStruct::WithFields(vals), MoveStructLayout::WithTypes { type_, fields }) => {
                MoveStruct::WithTypes {
                    _type_: type_.clone(),
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

    pub fn with_fields(types: Vec<MoveFieldLayout>) -> Self {
        Self::WithFields(types)
    }

    pub fn with_types(type_: StructTag, fields: Vec<MoveFieldLayout>) -> Self {
        Self::WithTypes { type_, fields }
    }

    pub fn with_variants(type_: StructTag, variants: Vec<MoveVariantLayout>) -> Self {
        Self::WithVariants { type_, variants }
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
            Self::WithFields(_) | Self::WithTypes { .. } | Self::WithVariants { .. } => {
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
            Self::WithFields(fields) | Self::WithTypes { fields, .. } => {
                fields.into_iter().map(|f| f.layout).collect()
            },
            Self::WithVariants { mut variants, .. } => match variant {
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
            MoveTypeLayout::U256 => int256::U256::deserialize(deserializer).map(MoveValue::U256),
            MoveTypeLayout::I8 => i8::deserialize(deserializer).map(MoveValue::I8),
            MoveTypeLayout::I16 => i16::deserialize(deserializer).map(MoveValue::I16),
            MoveTypeLayout::I32 => i32::deserialize(deserializer).map(MoveValue::I32),
            MoveTypeLayout::I64 => i64::deserialize(deserializer).map(MoveValue::I64),
            MoveTypeLayout::I128 => i128::deserialize(deserializer).map(MoveValue::I128),
            MoveTypeLayout::I256 => int256::I256::deserialize(deserializer).map(MoveValue::I256),
            MoveTypeLayout::Address => {
                AccountAddress::deserialize(deserializer).map(MoveValue::Address)
            },
            MoveTypeLayout::Signer => Err(D::Error::custom("cannot deserialize signer")),
            MoveTypeLayout::Struct(ty) => {
                Ok(MoveValue::Struct(ty.as_ref().deserialize(deserializer)?))
            },
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
            MoveStructLayout::WithFields(layout) => {
                let fields = deserializer
                    .deserialize_tuple(layout.len(), DecoratedStructFieldVisitor(layout))?;
                Ok(MoveStruct::WithFields(fields))
            },
            MoveStructLayout::WithTypes {
                type_,
                fields: layout,
            } => {
                let fields = deserializer
                    .deserialize_tuple(layout.len(), DecoratedStructFieldVisitor(layout))?;
                Ok(MoveStruct::WithTypes {
                    _type_: type_.clone(),
                    _fields: fields,
                })
            },
            MoveStructLayout::WithVariants {
                variants: decorated_variants,
                ..
            } => {
                // Downgrade the decorated variants to simple layouts to deserialize the fields.
                let variant_names = variant_name_placeholder(decorated_variants.len())
                    .map_err(|e| D::Error::custom(format!("{}", e)))?;
                let (tag, fields) = deserializer.deserialize_enum(
                    MOVE_ENUM_NAME,
                    variant_names,
                    StructVariantVisitor(
                        &decorated_variants
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
            MoveValue::I8(i) => serializer.serialize_i8(*i),
            MoveValue::I16(i) => serializer.serialize_i16(*i),
            MoveValue::I32(i) => serializer.serialize_i32(*i),
            MoveValue::I64(i) => serializer.serialize_i64(*i),
            MoveValue::I128(i) => serializer.serialize_i128(*i),
            MoveValue::I256(i) => i.serialize(serializer),
            MoveValue::Address(a) => a.serialize(serializer),
            // A signer serializes identically to its address. The runtime representation of a
            // signer is a bare address.
            MoveValue::Signer(a) => a.serialize(serializer),
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
                // serialize type as string (e.g., 0x0::ModuleName::StructName<TypeArg1, TypeArg2>) instead of (e.g.
                // { address: 0x0...0, module: ModuleName, name: StructName, type_args: [TypeArg1, TypeArg2]})
                t.serialize_field(MOVE_STRUCT_TYPE, &type_.to_canonical_string())?;
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
            MoveTypeLayout::I8 => TypeTag::I8,
            MoveTypeLayout::I16 => TypeTag::I16,
            MoveTypeLayout::I32 => TypeTag::I32,
            MoveTypeLayout::I64 => TypeTag::I64,
            MoveTypeLayout::I128 => TypeTag::I128,
            MoveTypeLayout::I256 => TypeTag::I256,
            MoveTypeLayout::Signer => TypeTag::Signer,
            MoveTypeLayout::Vector(v) => TypeTag::Vector(Box::new(v.as_ref().try_into()?)),
            MoveTypeLayout::Struct(v) => TypeTag::Struct(Box::new(v.as_ref().try_into()?)),

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
            Runtime(..) | RuntimeVariants(..) | WithFields(..) => bail!(
                "Invalid MoveTypeLayout -> StructTag conversion--needed MoveLayoutType::WithTypes or WithVariants"
            ),
            WithTypes { type_, .. } | WithVariants { type_, .. } => Ok(type_.clone()),
        }
    }
}

#[cfg(test)]
mod unfolded_node_count_tests {
    use super::*;

    /// Primitives are leaves and always count as 1.
    #[test]
    fn primitives_count_as_one() {
        assert_eq!(MoveTypeLayout::U8.unfolded_node_count(), 1);
        assert_eq!(MoveTypeLayout::Address.unfolded_node_count(), 1);
        assert_eq!(MoveTypeLayout::Function.unfolded_node_count(), 1);
    }

    /// Containers add one node for themselves and recurse into the element.
    #[test]
    fn containers_add_one_per_level() {
        let layout = MoveTypeLayout::Vector(Box::new(MoveTypeLayout::Vector(Box::new(
            MoveTypeLayout::U8,
        ))));
        assert_eq!(layout.unfolded_node_count(), 3);
    }

    /// Builds layouts for
    /// ```text
    /// struct S_0 { v: u8 }
    /// struct S_i { l: S_{i - 1}, r: S_{i - 1} }  // for i > 0
    /// ```
    /// The expanded node count is `c(N) = 3 * 2^N - 1`. Verifies that the count
    /// reflects the fully-expanded tree, not the deduplicated in-memory DAG.
    #[test]
    fn doubling_dag_counts_tree_expansion() {
        fn build(n: usize) -> MoveTypeLayout {
            let mut current = Arc::new(MoveStructLayout::Runtime(vec![MoveTypeLayout::U8]));
            for _ in 0..n {
                let child = MoveTypeLayout::Struct(current);
                current = Arc::new(MoveStructLayout::Runtime(vec![child; 2]));
            }
            MoveTypeLayout::Struct(current)
        }
        for n in 0..10 {
            let layout = build(n);
            assert_eq!(layout.unfolded_node_count(), 3u64 * (1u64 << n) - 1);
        }
    }
}
