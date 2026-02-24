// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0
//! Loaded representation for runtime types.

use crate::limit::Limiter;
use move_binary_format::errors::{PartialVMError, PartialVMResult};
use move_core_types::{
    ability::AbilitySet,
    account_address::AccountAddress,
    identifier::Identifier,
    language_storage::{
        FunctionParamOrReturnTag, FunctionTag, StructTag, TypeTag, TABLE_MODULE_ID,
        TABLE_STRUCT_NAME,
    },
    value::{MoveStructLayout, MoveTypeLayout},
    vm_status::StatusCode,
};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{cmp::Ordering, convert::TryInto, ops::Deref, rc::Rc};

#[derive(Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq)]
pub(crate) struct WrappedAbilitySet(pub AbilitySet);

impl Serialize for WrappedAbilitySet {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.into_u8().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for WrappedAbilitySet {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let byte = u8::deserialize(deserializer)?;
        Ok(WrappedAbilitySet(AbilitySet::from_u8(byte).ok_or_else(
            || serde::de::Error::custom(format!("Invalid ability set: {:X}", byte)),
        )?))
    }
}

/// VM representation of a struct type in Move.
#[derive(Debug, Clone, Serialize, Deserialize, PartialOrd, Ord, PartialEq, Eq)]
pub(crate) struct FatStructType {
    pub address: AccountAddress,
    pub module: Identifier,
    pub name: Identifier,
    pub abilities: WrappedAbilitySet,
    pub ty_args: Vec<FatType>,
    pub layout: FatStructLayout,
    // Whether this struct transitively contains 0x1::table::Table types. This
    // is true if this here is a table itself. Extends to the type arguments and
    // the layout.
    pub contains_tables: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialOrd, Ord, PartialEq, Eq)]
pub(crate) enum FatStructLayout {
    Singleton(Vec<FatType>),
    Variants(Vec<Vec<FatType>>),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialOrd, Ord, PartialEq, Eq)]
pub(crate) struct FatFunctionType {
    pub args: Vec<FatType>,
    pub results: Vec<FatType>,
    pub abilities: AbilitySet,
}

// INVARIANT: this type need to stay crate local. See discussion at `FatStructRef`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialOrd, Ord, PartialEq, Eq)]
pub(crate) enum FatType {
    Bool,
    U8,
    U64,
    U128,
    Address,
    Signer,
    Vector(Box<FatType>),
    Struct(FatStructRef),
    Reference(Box<FatType>),
    MutableReference(Box<FatType>),
    TyParam(usize),
    // NOTE: Added in bytecode version v6, do not reorder!
    U16,
    U32,
    U256,
    // NOTE: Added in bytecode version v8, do not reorder!
    Function(Box<FatFunctionType>),
    // `Runtime` and `RuntimeVariants` are used for typing
    // captured structures in closures, for which we only know
    // the raw layout (no struct name, no field names).
    Runtime(Vec<FatType>),
    RuntimeVariants(Vec<Vec<FatType>>),
    // NOTE: Added in bytecode version v9, do not reorder!
    I8,
    I16,
    I32,
    I64,
    I128,
    I256,
}

/// A representation for fat structs which assumes
/// that they are interned, enabling fast pointer comparison.
/// We can do this since `FatType` is crate private and we
/// know ordering is only used for caching; otherwise we
/// would need to be concerned for `ptr(rc1) != ptr(rc2)`
/// not representing structural disequality. But it is
/// only (?) a cache miss.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub(crate) struct FatStructRef {
    rc: Rc<FatStructType>,
}

/// Used as a key for caching struct related info.
#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub(crate) struct StructName {
    pub address: AccountAddress,
    pub module: Identifier,
    pub name: Identifier,
}

impl FatStructRef {
    pub(crate) fn new(data: FatStructType) -> Self {
        FatStructRef { rc: Rc::new(data) }
    }
}

impl AsRef<FatStructType> for FatStructRef {
    #[inline]
    fn as_ref(&self) -> &FatStructType {
        self.rc.as_ref()
    }
}
impl Deref for FatStructRef {
    type Target = FatStructType;

    #[inline]
    fn deref(&self) -> &FatStructType {
        self.rc.as_ref()
    }
}

impl PartialEq for FatStructRef {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        // Notice we could also implement full semantics, but it's likely more expensive than
        // accepting cache misses.
        Rc::ptr_eq(&self.rc, &other.rc)
    }
}
impl Eq for FatStructRef {}

impl PartialOrd for FatStructRef {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for FatStructRef {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        (Rc::as_ptr(&self.rc) as usize).cmp(&(Rc::as_ptr(&other.rc) as usize))
    }
}

impl FatStructType {
    pub fn subst(
        &self,
        ty_args: &[FatType],
        subst_struct: &impl Fn(
            &FatStructType,
            &[FatType],
            &mut Limiter,
        ) -> PartialVMResult<FatStructRef>,
        limiter: &mut Limiter,
    ) -> PartialVMResult<FatStructType> {
        limiter.charge(std::mem::size_of::<AccountAddress>())?;
        limiter.charge(self.module.as_bytes().len())?;
        limiter.charge(self.name.as_bytes().len())?;
        // self.contains_tables already reflects tables directly used in field types, we
        // only need to combine it here with tables used in type arguments.
        let contains_tables = self.contains_tables || ty_args.iter().any(|t| t.contains_tables());
        Ok(Self {
            address: self.address,
            module: self.module.clone(),
            name: self.name.clone(),
            abilities: self.abilities,
            ty_args: self
                .ty_args
                .iter()
                .map(|ty| ty.subst(ty_args, subst_struct, limiter))
                .collect::<PartialVMResult<_>>()?,
            layout: match &self.layout {
                FatStructLayout::Singleton(fields) => FatStructLayout::Singleton(
                    fields
                        .iter()
                        .map(|ty| ty.subst(ty_args, subst_struct, limiter))
                        .collect::<PartialVMResult<_>>()?,
                ),
                FatStructLayout::Variants(variants) => FatStructLayout::Variants(
                    variants
                        .iter()
                        .map(|fields| {
                            fields
                                .iter()
                                .map(|ty| ty.subst(ty_args, subst_struct, limiter))
                                .collect::<PartialVMResult<_>>()
                        })
                        .collect::<PartialVMResult<_>>()?,
                ),
            },
            contains_tables,
        })
    }

    pub fn struct_name(&self) -> StructName {
        StructName {
            address: self.address,
            module: self.module.clone(),
            name: self.name.clone(),
        }
    }

    pub fn struct_tag(&self, limiter: &mut Limiter) -> PartialVMResult<StructTag> {
        let ty_args = self
            .ty_args
            .iter()
            .map(|ty| ty.type_tag(limiter))
            .collect::<PartialVMResult<Vec<_>>>()?;

        limiter.charge(std::mem::size_of::<AccountAddress>())?;
        limiter.charge(self.module.as_bytes().len())?;
        limiter.charge(self.name.as_bytes().len())?;

        Ok(StructTag {
            address: self.address,
            module: self.module.clone(),
            name: self.name.clone(),
            type_args: ty_args,
        })
    }

    pub fn is_table(&self) -> bool {
        let table_id = &*TABLE_MODULE_ID;
        self.address == table_id.address
            && self.module == table_id.name
            && self.name == *TABLE_STRUCT_NAME
    }
}

impl FatFunctionType {
    fn clone_with_limit(&self, limiter: &mut Limiter) -> PartialVMResult<Self> {
        let clone_slice = |limiter: &mut Limiter, tys: &[FatType]| {
            tys.iter()
                .map(|ty| ty.clone_with_limit(limiter))
                .collect::<PartialVMResult<Vec<_>>>()
        };
        Ok(FatFunctionType {
            args: clone_slice(limiter, &self.args)?,
            results: clone_slice(limiter, &self.results)?,
            abilities: self.abilities,
        })
    }

    pub fn subst(
        &self,
        ty_args: &[FatType],
        subst_struct: &impl Fn(
            &FatStructType,
            &[FatType],
            &mut Limiter,
        ) -> PartialVMResult<FatStructRef>,
        limiter: &mut Limiter,
    ) -> PartialVMResult<Self> {
        let subst_slice = |limiter: &mut Limiter, tys: &[FatType]| {
            tys.iter()
                .map(|ty| ty.subst(ty_args, subst_struct, limiter))
                .collect::<PartialVMResult<Vec<_>>>()
        };
        Ok(FatFunctionType {
            args: subst_slice(limiter, &self.args)?,
            results: subst_slice(limiter, &self.results)?,
            abilities: self.abilities,
        })
    }

    pub fn fun_tag(&self, limiter: &mut Limiter) -> PartialVMResult<FunctionTag> {
        let tag_slice = |limiter: &mut Limiter, tys: &[FatType]| {
            tys.iter()
                .map(|ty| {
                    Ok(match ty {
                        FatType::Reference(ty) => {
                            FunctionParamOrReturnTag::Reference(ty.type_tag(limiter)?)
                        },
                        FatType::MutableReference(ty) => {
                            FunctionParamOrReturnTag::MutableReference(ty.type_tag(limiter)?)
                        },
                        ty => FunctionParamOrReturnTag::Value(ty.type_tag(limiter)?),
                    })
                })
                .collect::<PartialVMResult<Vec<_>>>()
        };
        Ok(FunctionTag {
            args: tag_slice(limiter, &self.args)?,
            results: tag_slice(limiter, &self.results)?,
            abilities: self.abilities,
        })
    }
}

impl FatType {
    fn clone_with_limit(&self, limit: &mut Limiter) -> PartialVMResult<Self> {
        use FatType::*;
        Ok(match self {
            TyParam(idx) => TyParam(*idx),
            Bool => Bool,
            U8 => U8,
            U16 => U16,
            U32 => U32,
            U64 => U64,
            U128 => U128,
            U256 => U256,
            I8 => I8,
            I16 => I16,
            I32 => I32,
            I64 => I64,
            I128 => I128,
            I256 => I256,
            Address => Address,
            Signer => Signer,
            Vector(ty) => Vector(Box::new(ty.clone_with_limit(limit)?)),
            Reference(ty) => Reference(Box::new(ty.clone_with_limit(limit)?)),
            MutableReference(ty) => MutableReference(Box::new(ty.clone_with_limit(limit)?)),
            Struct(struct_ty) => Struct(struct_ty.clone()),
            Function(fun_ty) => Function(Box::new(fun_ty.clone_with_limit(limit)?)),
            Runtime(tys) => Runtime(Self::clone_with_limit_slice(tys, limit)?),
            RuntimeVariants(vars) => RuntimeVariants(
                vars.iter()
                    .map(|tys| Self::clone_with_limit_slice(tys, limit))
                    .collect::<PartialVMResult<Vec<_>>>()?,
            ),
        })
    }

    fn clone_with_limit_slice(tys: &[Self], limit: &mut Limiter) -> PartialVMResult<Vec<Self>> {
        tys.iter().map(|ty| ty.clone_with_limit(limit)).collect()
    }

    pub fn subst(
        &self,
        ty_args: &[FatType],
        subst_struct: &impl Fn(
            &FatStructType,
            &[FatType],
            &mut Limiter,
        ) -> PartialVMResult<FatStructRef>,
        limit: &mut Limiter,
    ) -> PartialVMResult<FatType> {
        use FatType::*;

        let res = match self {
            TyParam(idx) => match ty_args.get(*idx) {
                Some(ty) => ty.clone_with_limit(limit)?,
                None => {
                    return Err(
                        PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                            .with_message(format!(
                            "fat type substitution failed: index out of bounds -- len {} got {}",
                            ty_args.len(),
                            idx
                        )),
                    );
                },
            },

            Bool => Bool,
            U8 => U8,
            U16 => U16,
            U32 => U32,
            U64 => U64,
            U128 => U128,
            U256 => U256,
            I8 => I8,
            I16 => I16,
            I32 => I32,
            I64 => I64,
            I128 => I128,
            I256 => I256,
            Address => Address,
            Signer => Signer,
            Vector(ty) => Vector(Box::new(ty.subst(ty_args, subst_struct, limit)?)),
            Reference(ty) => Reference(Box::new(ty.subst(ty_args, subst_struct, limit)?)),
            MutableReference(ty) => {
                MutableReference(Box::new(ty.subst(ty_args, subst_struct, limit)?))
            },

            Struct(struct_ty) => {
                if struct_ty.ty_args.is_empty() {
                    // If the struct has no type parameters, it's field types cannot be effected
                    // by type substitution.
                    Struct(struct_ty.clone())
                } else {
                    Struct((*subst_struct)(struct_ty, ty_args, limit)?)
                }
            },

            Function(fun_ty) => Function(Box::new(fun_ty.subst(ty_args, subst_struct, limit)?)),
            Runtime(tys) => Runtime(
                tys.iter()
                    .map(|ty| ty.subst(ty_args, subst_struct, limit))
                    .collect::<PartialVMResult<Vec<_>>>()?,
            ),
            RuntimeVariants(vars) => RuntimeVariants(
                vars.iter()
                    .map(|tys| {
                        tys.iter()
                            .map(|ty| ty.subst(ty_args, subst_struct, limit))
                            .collect::<PartialVMResult<Vec<_>>>()
                    })
                    .collect::<PartialVMResult<Vec<Vec<_>>>>()?,
            ),
        };

        Ok(res)
    }

    pub fn type_tag(&self, limit: &mut Limiter) -> PartialVMResult<TypeTag> {
        use FatType::*;

        let res = match self {
            Bool => TypeTag::Bool,
            U8 => TypeTag::U8,
            U16 => TypeTag::U16,
            U32 => TypeTag::U32,
            U64 => TypeTag::U64,
            U128 => TypeTag::U128,
            U256 => TypeTag::U256,
            I8 => TypeTag::I8,
            I16 => TypeTag::I16,
            I32 => TypeTag::I32,
            I64 => TypeTag::I64,
            I128 => TypeTag::I128,
            I256 => TypeTag::I256,
            Address => TypeTag::Address,
            Signer => TypeTag::Signer,
            Vector(ty) => TypeTag::Vector(Box::new(ty.type_tag(limit)?)),
            Struct(struct_ty) => TypeTag::Struct(Box::new(struct_ty.struct_tag(limit)?)),
            Function(fun_ty) => TypeTag::Function(Box::new(fun_ty.fun_tag(limit)?)),

            Reference(_) | MutableReference(_) | TyParam(_) | RuntimeVariants(_) | Runtime(..) => {
                return Err(
                    PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                        .with_message(format!("cannot derive type tag for {:?}", self)),
                )
            },
        };

        Ok(res)
    }

    pub(crate) fn from_runtime_layout(
        layout: &MoveTypeLayout,
        limit: &mut Limiter,
    ) -> PartialVMResult<FatType> {
        use MoveTypeLayout::*;
        Ok(match layout {
            Bool => FatType::Bool,
            U8 => FatType::U8,
            U16 => FatType::U16,
            U32 => FatType::U32,
            U64 => FatType::U64,
            U128 => FatType::U128,
            U256 => FatType::U256,
            I8 => FatType::I8,
            I16 => FatType::I16,
            I32 => FatType::I32,
            I64 => FatType::I64,
            I128 => FatType::I128,
            I256 => FatType::I256,
            Address => FatType::Address,
            Signer => FatType::Signer,
            Vector(ty) => FatType::Vector(Box::new(Self::from_runtime_layout(ty, limit)?)),
            Struct(struct_layout) => match struct_layout.as_ref() {
                MoveStructLayout::Runtime(tys) => {
                    FatType::Runtime(Self::from_layout_slice(tys, limit)?)
                },
                MoveStructLayout::RuntimeVariants(vars) => FatType::RuntimeVariants(
                    vars.iter()
                        .map(|tys| Self::from_layout_slice(tys, limit))
                        .collect::<PartialVMResult<Vec<Vec<_>>>>()?,
                ),
                _ => {
                    return Err(PartialVMError::new_invariant_violation(format!(
                        "cannot derive fat type for {:?}",
                        layout
                    )))
                },
            },
            Function => {
                // We cannot derive the actual type from layout, however, a dummy
                // function type will do since annotation of closures is not depending
                // actually on their type, but only their (hidden) captured arguments.
                // Currently, `from_runtime_layout` is only used to annotate captured arguments
                // of closures.
                FatType::Function(Box::new(FatFunctionType {
                    args: vec![],
                    results: vec![],
                    abilities: AbilitySet::EMPTY,
                }))
            },
            Native(..) => {
                return Err(PartialVMError::new_invariant_violation(format!(
                    "cannot derive fat type for {:?}",
                    layout
                )))
            },
        })
    }

    fn from_layout_slice(
        layouts: &[MoveTypeLayout],
        limit: &mut Limiter,
    ) -> PartialVMResult<Vec<FatType>> {
        layouts
            .iter()
            .map(|l| Self::from_runtime_layout(l, limit))
            .collect()
    }

    pub(crate) fn contains_tables(&self) -> bool {
        match self {
            FatType::Struct(st) => st.contains_tables,
            FatType::MutableReference(ty) | FatType::Reference(ty) | FatType::Vector(ty) => {
                ty.contains_tables()
            },
            FatType::Runtime(tys) => tys.iter().any(|ty| ty.contains_tables()),
            FatType::RuntimeVariants(vars) => vars.iter().flatten().any(|ty| ty.contains_tables()),
            FatType::Bool
            | FatType::U8
            | FatType::U64
            | FatType::U128
            | FatType::Address
            | FatType::Signer
            | FatType::TyParam(_)
            | FatType::U16
            | FatType::U32
            | FatType::U256
            | FatType::Function(_)
            | FatType::I8
            | FatType::I16
            | FatType::I32
            | FatType::I64
            | FatType::I128
            | FatType::I256 => false,
        }
    }
}

impl TryInto<MoveStructLayout> for &FatStructType {
    type Error = PartialVMError;

    fn try_into(self) -> Result<MoveStructLayout, Self::Error> {
        Ok(match &self.layout {
            FatStructLayout::Singleton(fields) => MoveStructLayout::new(into_types(fields.iter())?),
            FatStructLayout::Variants(variants) => MoveStructLayout::new_variants(
                variants
                    .iter()
                    .map(|fields| into_types(fields.iter()))
                    .collect::<PartialVMResult<_>>()?,
            ),
        })
    }
}

fn into_types<'a>(
    types: impl Iterator<Item = &'a FatType>,
) -> PartialVMResult<Vec<MoveTypeLayout>> {
    types
        .map(|ty| ty.try_into())
        .collect::<PartialVMResult<Vec<_>>>()
}

impl TryInto<MoveTypeLayout> for &FatType {
    type Error = PartialVMError;

    fn try_into(self) -> Result<MoveTypeLayout, Self::Error> {
        let slice_into = |tys: &[FatType]| {
            tys.iter()
                .map(|ty| ty.try_into())
                .collect::<PartialVMResult<Vec<MoveTypeLayout>>>()
        };
        Ok(match self {
            FatType::Address => MoveTypeLayout::Address,
            FatType::U8 => MoveTypeLayout::U8,
            FatType::U16 => MoveTypeLayout::U16,
            FatType::U32 => MoveTypeLayout::U32,
            FatType::U64 => MoveTypeLayout::U64,
            FatType::U128 => MoveTypeLayout::U128,
            FatType::U256 => MoveTypeLayout::U256,
            FatType::I8 => MoveTypeLayout::I8,
            FatType::I16 => MoveTypeLayout::I16,
            FatType::I32 => MoveTypeLayout::I32,
            FatType::I64 => MoveTypeLayout::I64,
            FatType::I128 => MoveTypeLayout::I128,
            FatType::I256 => MoveTypeLayout::I256,
            FatType::Bool => MoveTypeLayout::Bool,
            FatType::Vector(v) => MoveTypeLayout::Vector(Box::new(v.as_ref().try_into()?)),
            FatType::Struct(s) => MoveTypeLayout::new_struct(s.as_ref().try_into()?),
            FatType::Function(_) => MoveTypeLayout::Function,
            FatType::Runtime(tys) => {
                MoveTypeLayout::new_struct(MoveStructLayout::Runtime(slice_into(tys)?))
            },
            FatType::RuntimeVariants(vars) => {
                MoveTypeLayout::new_struct(MoveStructLayout::RuntimeVariants(
                    vars.iter()
                        .map(|tys| slice_into(tys))
                        .collect::<Result<Vec<_>, _>>()?,
                ))
            },
            FatType::Signer => MoveTypeLayout::Signer,
            FatType::Reference(_) | FatType::MutableReference(_) | FatType::TyParam(_) => {
                return Err(PartialVMError::new(StatusCode::ABORT_TYPE_MISMATCH_ERROR))
            },
        })
    }
}
