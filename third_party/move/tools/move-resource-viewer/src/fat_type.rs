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
    language_storage::{FunctionTag, StructTag, TypeTag},
    value::{MoveStructLayout, MoveTypeLayout},
    vm_status::StatusCode,
};
use std::convert::TryInto;

/// VM representation of a struct type in Move.
#[derive(Debug, Clone)]
pub(crate) struct FatStructType {
    pub address: AccountAddress,
    pub module: Identifier,
    pub name: Identifier,
    pub abilities: AbilitySet,
    pub ty_args: Vec<FatType>,
    pub layout: FatStructLayout,
}

#[derive(Debug, Clone)]
pub(crate) enum FatStructLayout {
    Singleton(Vec<FatType>),
    Variants(Vec<Vec<FatType>>),
}

#[derive(Debug, Clone)]
pub(crate) struct FatFunctionType {
    pub args: Vec<FatType>,
    pub results: Vec<FatType>,
    pub abilities: AbilitySet,
}

#[derive(Debug, Clone)]
pub(crate) enum FatType {
    Bool,
    U8,
    U16,
    U32,
    U64,
    U128,
    U256,
    Address,
    Signer,
    Vector(Box<FatType>),
    Struct(Box<FatStructType>),
    TyParam(usize),
    Function(Box<FatFunctionType>),
    Runtime(Vec<FatType>),
    RuntimeVariants(Vec<Vec<FatType>>),
}

impl FatStructType {
    fn clone_with_limit(&self, limit: &mut Limiter) -> PartialVMResult<Self> {
        limit.charge(std::mem::size_of::<AccountAddress>())?;
        limit.charge(self.module.as_bytes().len())?;
        limit.charge(self.name.as_bytes().len())?;

        Ok(Self {
            address: self.address,
            module: self.module.clone(),
            name: self.name.clone(),
            abilities: self.abilities,
            ty_args: self
                .ty_args
                .iter()
                .map(|ty| ty.clone_with_limit(limit))
                .collect::<PartialVMResult<_>>()?,
            layout: match &self.layout {
                FatStructLayout::Singleton(fields) => FatStructLayout::Singleton(
                    fields
                        .iter()
                        .map(|ty| ty.clone_with_limit(limit))
                        .collect::<PartialVMResult<_>>()?,
                ),
                FatStructLayout::Variants(variants) => FatStructLayout::Variants(
                    variants
                        .iter()
                        .map(|fields| {
                            fields
                                .iter()
                                .map(|ty| ty.clone_with_limit(limit))
                                .collect::<PartialVMResult<Vec<_>>>()
                        })
                        .collect::<PartialVMResult<_>>()?,
                ),
            },
        })
    }

    pub fn subst(
        &self,
        ty_args: &[FatType],
        limiter: &mut Limiter,
    ) -> PartialVMResult<FatStructType> {
        limiter.charge(std::mem::size_of::<AccountAddress>())?;
        limiter.charge(self.module.as_bytes().len())?;
        limiter.charge(self.name.as_bytes().len())?;
        Ok(Self {
            address: self.address,
            module: self.module.clone(),
            name: self.name.clone(),
            abilities: self.abilities,
            ty_args: self
                .ty_args
                .iter()
                .map(|ty| ty.subst(ty_args, limiter))
                .collect::<PartialVMResult<_>>()?,
            layout: match &self.layout {
                FatStructLayout::Singleton(fields) => FatStructLayout::Singleton(
                    fields
                        .iter()
                        .map(|ty| ty.subst(ty_args, limiter))
                        .collect::<PartialVMResult<_>>()?,
                ),
                FatStructLayout::Variants(variants) => FatStructLayout::Variants(
                    variants
                        .iter()
                        .map(|fields| {
                            fields
                                .iter()
                                .map(|ty| ty.subst(ty_args, limiter))
                                .collect::<PartialVMResult<_>>()
                        })
                        .collect::<PartialVMResult<_>>()?,
                ),
            },
        })
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

    pub fn subst(&self, ty_args: &[FatType], limiter: &mut Limiter) -> PartialVMResult<Self> {
        let subst_slice = |limiter: &mut Limiter, tys: &[FatType]| {
            tys.iter()
                .map(|ty| ty.subst(ty_args, limiter))
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
                .map(|ty| ty.type_tag(limiter))
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
            Address => Address,
            Signer => Signer,
            Vector(ty) => Vector(Box::new(ty.clone_with_limit(limit)?)),
            Struct(struct_ty) => Struct(Box::new(struct_ty.clone_with_limit(limit)?)),
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

    pub fn subst(&self, ty_args: &[FatType], limit: &mut Limiter) -> PartialVMResult<FatType> {
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
            Address => Address,
            Signer => Signer,
            Vector(ty) => Vector(Box::new(ty.subst(ty_args, limit)?)),

            Struct(struct_ty) => Struct(Box::new(struct_ty.subst(ty_args, limit)?)),

            Function(fun_ty) => Function(Box::new(fun_ty.subst(ty_args, limit)?)),
            Runtime(tys) => Runtime(
                tys.iter()
                    .map(|ty| ty.subst(ty_args, limit))
                    .collect::<PartialVMResult<Vec<_>>>()?,
            ),
            RuntimeVariants(vars) => RuntimeVariants(
                vars.iter()
                    .map(|tys| {
                        tys.iter()
                            .map(|ty| ty.subst(ty_args, limit))
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
            Address => TypeTag::Address,
            Signer => TypeTag::Signer,
            Vector(ty) => TypeTag::Vector(Box::new(ty.type_tag(limit)?)),
            Struct(struct_ty) => TypeTag::Struct(Box::new(struct_ty.struct_tag(limit)?)),
            Function(fun_ty) => TypeTag::Function(Box::new(fun_ty.fun_tag(limit)?)),

            TyParam(_) | RuntimeVariants(_) | Runtime(..) => {
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
            Address => FatType::Address,
            Signer => FatType::Signer,
            Vector(ty) => FatType::Vector(Box::new(Self::from_runtime_layout(ty, limit)?)),
            Struct(MoveStructLayout::Runtime(tys)) => {
                FatType::Runtime(Self::from_layout_slice(tys, limit)?)
            },
            Struct(MoveStructLayout::RuntimeVariants(vars)) => FatType::RuntimeVariants(
                vars.iter()
                    .map(|tys| Self::from_layout_slice(tys, limit))
                    .collect::<PartialVMResult<Vec<Vec<_>>>>()?,
            ),
            // TODO(#15664): get rid of fat type to support captured functions.
            Native(..) | Struct(_) | Function => {
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
            FatType::Bool => MoveTypeLayout::Bool,
            FatType::Vector(v) => MoveTypeLayout::Vector(Box::new(v.as_ref().try_into()?)),
            FatType::Struct(s) => MoveTypeLayout::Struct(s.as_ref().try_into()?),
            FatType::Function(_) => MoveTypeLayout::Function,
            FatType::Runtime(tys) => {
                MoveTypeLayout::Struct(MoveStructLayout::Runtime(slice_into(tys)?))
            },
            FatType::RuntimeVariants(vars) => {
                MoveTypeLayout::Struct(MoveStructLayout::RuntimeVariants(
                    vars.iter()
                        .map(|tys| slice_into(tys))
                        .collect::<Result<Vec<_>, _>>()?,
                ))
            },
            FatType::Signer => MoveTypeLayout::Signer,
            FatType::TyParam(_) => {
                return Err(PartialVMError::new(StatusCode::ABORT_TYPE_MISMATCH_ERROR))
            },
        })
    }
}
