// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Frame-slot layout for entry-function arguments.
//!
//! Mirrors MonoMove's frame layout: each parameter is placed at the next offset
//! aligned to its alignment, and advances the cursor by its size. Two kinds of
//! argument are supported:
//!
//!   - Scalars (`bool`/integers/`address`/`signer`, and `0x1::object::Object<T>`
//!     which is a one-field struct wrapping an address). The BCS encoding equals
//!     the raw little-endian frame bytes, so the same bytes serve both the
//!     legacy VM (as serialized args) and MonoMove (copied into a frame slot).
//!   - A hardcoded set of heap-boxed types needed by the benchmark
//!     (`vector<u64>`, `vector<vector<u8>>`, `0x1::string::String`). These
//!     occupy an 8-byte pointer slot and are placed by deserializing the BCS
//!     bytes into the heap (see `InterpreterContext::deserialize_arg`).
//!
//! The first `num_signers` parameters are `&signer`s, filled with the sender
//! address; the rest are the BCS args from the transaction. Any other argument
//! type is reported (by name) as unsupported and skipped.

use move_core_types::{
    account_address::AccountAddress,
    ident_str,
    language_storage::{StructTag, TypeTag},
};
use move_vm_types::loaded_data::{
    runtime_types::Type,
    struct_name_indexing::{StructNameIndex, StructNameIndexMap},
};

/// A scalar parameter kind with a fixed frame size and alignment.
#[derive(Copy, Clone, Debug)]
pub enum PrimitiveKind {
    Bool,
    U8,
    U16,
    U32,
    U64,
    U128,
    U256,
    I8,
    I16,
    I32,
    I64,
    I128,
    I256,
    Address,
    Signer,
}

impl PrimitiveKind {
    /// Classifies a scalar parameter type, or `None` if it is not a scalar.
    /// `0x1::object::Object<T>` is classified as an address (a one-field struct
    /// wrapping an address). References are unwrapped by the caller.
    pub fn from_type(ty: &Type, structs: &StructNameIndexMap) -> Option<Self> {
        Some(match ty {
            Type::Bool => Self::Bool,
            Type::U8 => Self::U8,
            Type::U16 => Self::U16,
            Type::U32 => Self::U32,
            Type::U64 => Self::U64,
            Type::U128 => Self::U128,
            Type::U256 => Self::U256,
            Type::I8 => Self::I8,
            Type::I16 => Self::I16,
            Type::I32 => Self::I32,
            Type::I64 => Self::I64,
            Type::I128 => Self::I128,
            Type::I256 => Self::I256,
            Type::Address => Self::Address,
            Type::Signer => Self::Signer,
            Type::Struct { idx, .. } | Type::StructInstantiation { idx, .. }
                if is_well_known(structs, *idx, "object", "Object") =>
            {
                Self::Address
            },
            Type::Struct { .. }
            | Type::StructInstantiation { .. }
            | Type::Vector(_)
            | Type::Reference(_)
            | Type::MutableReference(_)
            | Type::Function { .. }
            | Type::TyParam(_) => return None,
        })
    }

    pub fn size(self) -> u32 {
        match self {
            Self::Bool | Self::U8 | Self::I8 => 1,
            Self::U16 | Self::I16 => 2,
            Self::U32 | Self::I32 => 4,
            Self::U64 | Self::I64 => 8,
            Self::U128 | Self::I128 => 16,
            Self::U256 | Self::I256 | Self::Address | Self::Signer => 32,
        }
    }

    pub fn align(self) -> u32 {
        match self {
            Self::Bool | Self::U8 | Self::I8 => 1,
            Self::U16 | Self::I16 => 2,
            Self::U32 | Self::I32 => 4,
            Self::U64 | Self::I64 => 8,
            // Wide integers and addresses are 8-byte aligned in the frame.
            Self::U128 | Self::I128 | Self::U256 | Self::I256 | Self::Address | Self::Signer => 8,
        }
    }
}

/// The `TypeTag` for a value type MonoMove can place by deserialization, or
/// `None` if the type (or any component) is unsupported. Covers primitives,
/// `vector<T>` of any supported `T` (including nested vectors),
/// `0x1::string::String`, `0x1::option::Option<T>` for any supported `T`, and
/// `0x1::object::Object<T>` (a phantom wrapper over an address). Other structs,
/// function values, type parameters, and references are unsupported.
fn value_tag(ty: &Type, structs: &StructNameIndexMap) -> Option<TypeTag> {
    Some(match ty {
        Type::Bool => TypeTag::Bool,
        Type::U8 => TypeTag::U8,
        Type::U16 => TypeTag::U16,
        Type::U32 => TypeTag::U32,
        Type::U64 => TypeTag::U64,
        Type::U128 => TypeTag::U128,
        Type::U256 => TypeTag::U256,
        Type::I8 => TypeTag::I8,
        Type::I16 => TypeTag::I16,
        Type::I32 => TypeTag::I32,
        Type::I64 => TypeTag::I64,
        Type::I128 => TypeTag::I128,
        Type::I256 => TypeTag::I256,
        Type::Address => TypeTag::Address,
        Type::Signer => TypeTag::Signer,
        Type::Vector(inner) => TypeTag::Vector(Box::new(value_tag(inner, structs)?)),
        Type::Struct { idx, .. } | Type::StructInstantiation { idx, .. }
            if is_well_known(structs, *idx, "string", "String") =>
        {
            TypeTag::Struct(Box::new(string_struct_tag()))
        },
        Type::StructInstantiation { idx, ty_args, .. }
            if is_well_known(structs, *idx, "option", "Option") =>
        {
            // `Option<T>` is an enum with a single type argument `T`. Place it
            // by deserializing into the heap, recursing on the element type.
            let [elem] = &ty_args[..] else {
                return None;
            };
            TypeTag::Struct(Box::new(option_struct_tag(value_tag(elem, structs)?)))
        },
        Type::StructInstantiation { idx, ty_args, .. }
            if is_well_known(structs, *idx, "object", "Object") =>
        {
            // `Object<T>` is a phantom wrapper over an address (its sole field).
            // `T` is phantom and never deserialized, but it must match the
            // instantiation in the code so the resolved type has a published
            // layout — so it is converted with the general `type_to_tag`.
            let [phantom] = &ty_args[..] else {
                return None;
            };
            TypeTag::Struct(Box::new(object_struct_tag(type_to_tag(phantom, structs)?)))
        },
        Type::Struct { .. }
        | Type::StructInstantiation { .. }
        | Type::Reference(_)
        | Type::MutableReference(_)
        | Type::Function { .. }
        | Type::TyParam(_) => return None,
    })
}

/// Converts an arbitrary parameter type to its `TypeTag`, resolving struct
/// names through `structs`. Unlike [`value_tag`], it does not restrict to
/// MonoMove-placeable value types — it is used for the phantom type arguments
/// of `Object<T>`, which are only needed to name the nominal, never placed.
/// Returns `None` for references, function values, and type parameters.
fn type_to_tag(ty: &Type, structs: &StructNameIndexMap) -> Option<TypeTag> {
    Some(match ty {
        Type::Bool => TypeTag::Bool,
        Type::U8 => TypeTag::U8,
        Type::U16 => TypeTag::U16,
        Type::U32 => TypeTag::U32,
        Type::U64 => TypeTag::U64,
        Type::U128 => TypeTag::U128,
        Type::U256 => TypeTag::U256,
        Type::I8 => TypeTag::I8,
        Type::I16 => TypeTag::I16,
        Type::I32 => TypeTag::I32,
        Type::I64 => TypeTag::I64,
        Type::I128 => TypeTag::I128,
        Type::I256 => TypeTag::I256,
        Type::Address => TypeTag::Address,
        Type::Signer => TypeTag::Signer,
        Type::Vector(inner) => TypeTag::Vector(Box::new(type_to_tag(inner, structs)?)),
        Type::Struct { idx, .. } => TypeTag::Struct(Box::new(struct_tag_of(*idx, vec![], structs)?)),
        Type::StructInstantiation { idx, ty_args, .. } => {
            let args = ty_args
                .iter()
                .map(|t| type_to_tag(t, structs))
                .collect::<Option<Vec<_>>>()?;
            TypeTag::Struct(Box::new(struct_tag_of(*idx, args, structs)?))
        },
        Type::Reference(_)
        | Type::MutableReference(_)
        | Type::Function { .. }
        | Type::TyParam(_) => return None,
    })
}

/// Builds the `StructTag` for the named struct index with the given type
/// arguments.
fn struct_tag_of(
    idx: StructNameIndex,
    type_args: Vec<TypeTag>,
    structs: &StructNameIndexMap,
) -> Option<StructTag> {
    let name = structs.idx_to_struct_name(idx).ok()?;
    Some(StructTag {
        address: *name.module().address(),
        module: name.module().name().to_owned(),
        name: name.name().to_owned(),
        type_args,
    })
}

/// The struct tag for `0x1::string::String`.
fn string_struct_tag() -> StructTag {
    StructTag {
        address: AccountAddress::ONE,
        module: ident_str!("string").to_owned(),
        name: ident_str!("String").to_owned(),
        type_args: vec![],
    }
}

/// The struct tag for `0x1::option::Option<elem>`.
fn option_struct_tag(elem: TypeTag) -> StructTag {
    StructTag {
        address: AccountAddress::ONE,
        module: ident_str!("option").to_owned(),
        name: ident_str!("Option").to_owned(),
        type_args: vec![elem],
    }
}

/// The struct tag for `0x1::object::Object<phantom>`.
fn object_struct_tag(phantom: TypeTag) -> StructTag {
    StructTag {
        address: AccountAddress::ONE,
        module: ident_str!("object").to_owned(),
        name: ident_str!("Object").to_owned(),
        type_args: vec![phantom],
    }
}

/// How a single parameter is placed into the root frame.
#[derive(Clone, Debug)]
pub enum ArgKind {
    /// A scalar slot filled by copying the raw BCS bytes.
    Scalar(PrimitiveKind),
    /// A heap-boxed value (8-byte pointer slot) filled by deserializing the BCS
    /// bytes into the heap. Carries the type tag used to resolve its layout.
    Structured(TypeTag),
}

impl ArgKind {
    /// Classifies a parameter type, unwrapping a leading reference (entry
    /// functions take `&signer`). Returns `Err(name)` naming the type if it is
    /// neither a supported scalar nor a supported heap-boxed argument.
    pub fn from_type(ty: &Type, structs: &StructNameIndexMap) -> Result<Self, String> {
        if let Type::Reference(inner) | Type::MutableReference(inner) = ty {
            return Self::from_type(inner, structs);
        }
        if let Some(kind) = PrimitiveKind::from_type(ty, structs) {
            return Ok(Self::Scalar(kind));
        }
        if let Some(tag) = value_tag(ty, structs) {
            return Ok(Self::Structured(tag));
        }
        Err(describe_type(ty, structs))
    }

    pub fn size(&self) -> u32 {
        match self {
            Self::Scalar(kind) => kind.size(),
            // A heap-boxed value is a single pointer.
            Self::Structured(_) => 8,
        }
    }

    pub fn align(&self) -> u32 {
        match self {
            Self::Scalar(kind) => kind.align(),
            Self::Structured(_) => 8,
        }
    }
}

/// The placement of an entry function's non-signer arguments, plus how many
/// leading parameters are signers. `kinds` covers only the arguments (one per
/// transaction argument); the leading `num_signers` signer parameters are set
/// up separately by `InterpreterContext::set_root_signers`.
pub struct ArgLayout {
    pub kinds: Vec<ArgKind>,
    pub num_signers: usize,
}

impl ArgLayout {
    /// Builds a layout from the loaded function's parameter types and the count
    /// of non-signer arguments. The leading `num_signers = params - num_args`
    /// parameters are signers (handled by the interpreter); only the remaining
    /// argument parameters are classified. Returns `Err(reason)` naming the
    /// first argument that is not yet placeable in a MonoMove frame.
    pub fn from_param_tys(
        param_tys: &[Type],
        num_args: usize,
        structs: &StructNameIndexMap,
    ) -> Result<Self, String> {
        let num_signers = param_tys
            .len()
            .checked_sub(num_args)
            .ok_or_else(|| "more arguments than parameters".to_string())?;
        let mut kinds = Vec::with_capacity(num_args);
        for (i, ty) in param_tys[num_signers..].iter().enumerate() {
            let kind = ArgKind::from_type(ty, structs)
                .map_err(|name| format!("argument #{i} has unsupported type `{name}`"))?;
            kinds.push(kind);
        }
        Ok(Self { kinds, num_signers })
    }
}

/// Whether the indexed struct is `0x1::<module>::<name>`.
fn is_well_known(
    structs: &StructNameIndexMap,
    idx: StructNameIndex,
    module: &str,
    name: &str,
) -> bool {
    structs.idx_to_struct_name(idx).is_ok_and(|s| {
        s.module().address() == &AccountAddress::ONE
            && s.module().name().as_str() == module
            && s.name().as_str() == name
    })
}

/// A human-readable name for `ty`, used in "unsupported argument" errors.
fn describe_type(ty: &Type, structs: &StructNameIndexMap) -> String {
    match ty {
        Type::Bool => "bool".to_string(),
        Type::U8 => "u8".to_string(),
        Type::U16 => "u16".to_string(),
        Type::U32 => "u32".to_string(),
        Type::U64 => "u64".to_string(),
        Type::U128 => "u128".to_string(),
        Type::U256 => "u256".to_string(),
        Type::I8 => "i8".to_string(),
        Type::I16 => "i16".to_string(),
        Type::I32 => "i32".to_string(),
        Type::I64 => "i64".to_string(),
        Type::I128 => "i128".to_string(),
        Type::I256 => "i256".to_string(),
        Type::Address => "address".to_string(),
        Type::Signer => "signer".to_string(),
        Type::Vector(inner) => format!("vector<{}>", describe_type(inner, structs)),
        Type::Reference(inner) => format!("&{}", describe_type(inner, structs)),
        Type::MutableReference(inner) => format!("&mut {}", describe_type(inner, structs)),
        Type::Struct { idx, .. } | Type::StructInstantiation { idx, .. } => structs
            .idx_to_struct_name(*idx)
            .map_or_else(|_| "struct".to_string(), |s| s.to_string()),
        Type::Function { .. } => "function".to_string(),
        Type::TyParam(i) => format!("type parameter #{i}"),
    }
}
