// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0
//! Loaded representation for runtime types.

use crate::{limit::Limiter, print_if};
use move_binary_format::errors::{PartialVMError, PartialVMResult};
use move_core_types::{
    ability::AbilitySet,
    account_address::AccountAddress,
    identifier::Identifier,
    language_storage::{FunctionParamOrReturnTag, FunctionTag, StructTag, TypeTag},
    value::{MoveStructLayout, MoveTypeLayout},
    vm_status::StatusCode,
};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{convert::TryInto, sync::Arc, time::Instant};

#[derive(Debug, Clone, Copy)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct FatStructType {
    pub address: AccountAddress,
    pub module: Identifier,
    pub name: Identifier,
    pub abilities: WrappedAbilitySet,
    pub ty_args: Vec<FatType>,
    pub layout: FatStructLayout,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) enum FatStructLayout {
    Singleton(Vec<FatType>),
    Variants(Vec<Vec<FatType>>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct FatFunctionType {
    pub args: Vec<FatType>,
    pub results: Vec<FatType>,
    pub abilities: AbilitySet,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) enum FatType {
    Bool,
    U8,
    U64,
    U128,
    Address,
    Signer,
    Vector(Arc<FatType>),
    Struct(Arc<FatStructType>),
    Reference(Arc<FatType>),
    MutableReference(Arc<FatType>),
    TyParam(usize),
    // NOTE: Added in bytecode version v6, do not reorder!
    U16,
    U32,
    U256,
    // NOTE: Added in bytecode version v8, do not reorder!
    Function(Arc<FatFunctionType>),
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

impl FatStructType {
    /// Check if this struct type contains any type parameters that need substitution.
    fn contains_ty_param(&self) -> bool {
        self.ty_args.iter().any(|ty| ty.contains_ty_param())
            || match &self.layout {
                FatStructLayout::Singleton(fields) => {
                    fields.iter().any(|ty| ty.contains_ty_param())
                },
                FatStructLayout::Variants(variants) => variants
                    .iter()
                    .any(|fields| fields.iter().any(|ty| ty.contains_ty_param())),
            }
    }

    /// Mutate this struct type in place, substituting type parameters.
    /// Returns true if any mutations were made.
    fn subst_in_place(
        &mut self,
        ty_args: &[FatType],
        limiter: &mut Limiter,
    ) -> PartialVMResult<bool> {
        let mut changed = false;

        // Mutate ty_args in place
        for ty in &mut self.ty_args {
            if ty.contains_ty_param() {
                *ty = ty.subst(ty_args, limiter)?;
                changed = true;
            }
        }

        // Mutate layout in place
        match &mut self.layout {
            FatStructLayout::Singleton(fields) => {
                for ty in fields.iter_mut() {
                    if ty.contains_ty_param() {
                        *ty = ty.subst(ty_args, limiter)?;
                        changed = true;
                    }
                }
            },
            FatStructLayout::Variants(variants) => {
                for fields in variants.iter_mut() {
                    for ty in fields.iter_mut() {
                        if ty.contains_ty_param() {
                            *ty = ty.subst(ty_args, limiter)?;
                            changed = true;
                        }
                    }
                }
            },
        }

        Ok(changed)
    }

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
        // Optimization: Early exit if no type parameters are present
        if !self.contains_ty_param() {
            // Still charge for the metadata, but avoid recursive substitution
            limiter.charge(std::mem::size_of::<AccountAddress>())?;
            limiter.charge(self.module.as_bytes().len())?;
            limiter.charge(self.name.as_bytes().len())?;
            return Ok(Self {
                address: self.address,
                module: self.module.clone(),
                name: self.name.clone(),
                abilities: self.abilities,
                ty_args: self.ty_args.clone(),
                layout: self.layout.clone(),
            });
        }

        limiter.charge(std::mem::size_of::<AccountAddress>())?;
        limiter.charge(self.module.as_bytes().len())?;
        limiter.charge(self.name.as_bytes().len())?;

        // Clone and mutate in place to avoid creating new Vec allocations unnecessarily
        let mut result = Self {
            address: self.address,
            module: self.module.clone(),
            name: self.name.clone(),
            abilities: self.abilities,
            ty_args: self.ty_args.clone(),
            layout: self.layout.clone(),
        };

        // Mutate in place
        result.subst_in_place(ty_args, limiter)?;

        Ok(result)
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
    /// Check if this function type contains any type parameters that need substitution.
    fn contains_ty_param(&self) -> bool {
        self.args.iter().any(|ty| ty.contains_ty_param())
            || self.results.iter().any(|ty| ty.contains_ty_param())
    }

    /// Mutate this function type in place, substituting type parameters.
    /// Returns true if any mutations were made.
    fn subst_in_place(
        &mut self,
        ty_args: &[FatType],
        limiter: &mut Limiter,
    ) -> PartialVMResult<bool> {
        let mut changed = false;

        // Mutate args in place
        for ty in &mut self.args {
            if ty.contains_ty_param() {
                *ty = ty.subst(ty_args, limiter)?;
                changed = true;
            }
        }

        // Mutate results in place
        for ty in &mut self.results {
            if ty.contains_ty_param() {
                *ty = ty.subst(ty_args, limiter)?;
                changed = true;
            }
        }

        Ok(changed)
    }

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
        // Optimization: Early exit if no type parameters are present
        if !self.contains_ty_param() {
            return Ok(FatFunctionType {
                args: self.args.clone(),
                results: self.results.clone(),
                abilities: self.abilities,
            });
        }

        // Clone and mutate in place to avoid creating new Vec allocations unnecessarily
        let mut result = Self {
            args: self.args.clone(),
            results: self.results.clone(),
            abilities: self.abilities,
        };

        // Mutate in place
        result.subst_in_place(ty_args, limiter)?;

        Ok(result)
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
    /// Check if this type contains any type parameters that need substitution.
    /// This allows early-exit optimizations in substitution operations.
    pub(crate) fn contains_ty_param(&self) -> bool {
        use FatType::*;
        match self {
            TyParam(_) => true,
            Bool | U8 | U16 | U32 | U64 | U128 | U256 | I8 | I16 | I32 | I64 | I128 | I256
            | Address | Signer => false,
            Vector(ty) | Reference(ty) | MutableReference(ty) => ty.contains_ty_param(),
            Struct(struct_ty) => struct_ty.contains_ty_param(),
            Function(fun_ty) => fun_ty.contains_ty_param(),
            Runtime(tys) => tys.iter().any(|ty| ty.contains_ty_param()),
            RuntimeVariants(vars) => vars
                .iter()
                .any(|tys| tys.iter().any(|ty| ty.contains_ty_param())),
        }
    }

    fn clone_with_limit(&self, _limit: &mut Limiter) -> PartialVMResult<Self> {
        use FatType::*;
        // Note: limit is not charged here as Arc cloning is cheap, but it's part of the signature
        // for consistency with other clone_with_limit methods
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
            Vector(ty) => Vector(ty.clone()),
            Reference(ty) => Reference(ty.clone()),
            MutableReference(ty) => MutableReference(ty.clone()),
            Struct(struct_ty) => Struct(struct_ty.clone()),
            Function(fun_ty) => Function(fun_ty.clone()),
            Runtime(tys) => Runtime(tys.clone()),
            RuntimeVariants(vars) => RuntimeVariants(vars.clone()),
        })
    }

    fn clone_with_limit_slice(tys: &[Self], limit: &mut Limiter) -> PartialVMResult<Vec<Self>> {
        tys.iter().map(|ty| ty.clone_with_limit(limit)).collect()
    }

    pub fn subst(&self, ty_args: &[FatType], limit: &mut Limiter) -> PartialVMResult<FatType> {
        use FatType::*;

        let res = match self {
            TyParam(idx) => match ty_args.get(*idx) {
                Some(ty) => {
                    // Optimization: If the replacement type contains no TyParam, we can return it directly
                    // without cloning, avoiding unnecessary allocations
                    if !ty.contains_ty_param() {
                        return Ok(ty.clone());
                    }
                    // Otherwise, clone and continue (clone is needed to maintain ownership)
                    ty.clone_with_limit(limit)?
                },
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
            // Optimization: Reuse Arc when no substitution needed
            Vector(ty) => {
                if ty.contains_ty_param() {
                    Vector(Arc::new(ty.subst(ty_args, limit)?))
                } else {
                    // No substitution needed, reuse the existing Arc
                    Vector(ty.clone())
                }
            },
            Reference(ty) => {
                if ty.contains_ty_param() {
                    Reference(Arc::new(ty.subst(ty_args, limit)?))
                } else {
                    Reference(ty.clone())
                }
            },
            MutableReference(ty) => {
                if ty.contains_ty_param() {
                    MutableReference(Arc::new(ty.subst(ty_args, limit)?))
                } else {
                    MutableReference(ty.clone())
                }
            },

            Struct(struct_ty) => {
                if struct_ty.contains_ty_param() {
                    // Clone the inner type once and mutate in place to avoid intermediate allocations
                    let mut cloned = struct_ty.as_ref().clone();
                    cloned.subst_in_place(ty_args, limit)?;
                    Struct(Arc::new(cloned))
                } else {
                    // No substitution needed, reuse the existing Arc
                    Struct(struct_ty.clone())
                }
            },

            Function(fun_ty) => {
                if fun_ty.contains_ty_param() {
                    // Clone the inner type once and mutate in place to avoid intermediate allocations
                    let mut cloned = fun_ty.as_ref().clone();
                    cloned.subst_in_place(ty_args, limit)?;
                    Function(Arc::new(cloned))
                } else {
                    Function(fun_ty.clone())
                }
            },
            Runtime(tys) => {
                // Check if any type needs substitution
                if tys.iter().any(|ty| ty.contains_ty_param()) {
                    // Try to mutate in place if we own the Vec (by cloning and unwrapping)
                    // Since Runtime contains Vec directly (not Arc), we need to clone it first
                    // but we can mutate the cloned Vec in place
                    let mut result = tys.clone();
                    // Mutate elements in place
                    for ty in result.iter_mut() {
                        if ty.contains_ty_param() {
                            *ty = ty.subst(ty_args, limit)?;
                        }
                    }
                    Runtime(result)
                } else {
                    // No substitution needed, reuse the existing vector
                    Runtime(tys.clone())
                }
            },
            RuntimeVariants(vars) => {
                // Check if any variant needs substitution
                let needs_subst = vars
                    .iter()
                    .any(|tys| tys.iter().any(|ty| ty.contains_ty_param()));
                if needs_subst {
                    // Clone and mutate in place
                    let mut result = vars.clone();
                    for tys in result.iter_mut() {
                        for ty in tys.iter_mut() {
                            if ty.contains_ty_param() {
                                *ty = ty.subst(ty_args, limit)?;
                            }
                        }
                    }
                    RuntimeVariants(result)
                } else {
                    // No substitution needed, reuse the existing vector
                    RuntimeVariants(vars.clone())
                }
            },
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
            Vector(ty) => FatType::Vector(Arc::new(Self::from_runtime_layout(ty, limit)?)),
            Struct(MoveStructLayout::Runtime(tys)) => {
                FatType::Runtime(Self::from_layout_slice(tys, limit)?)
            },
            Struct(MoveStructLayout::RuntimeVariants(vars)) => FatType::RuntimeVariants(
                vars.iter()
                    .map(|tys| Self::from_layout_slice(tys, limit))
                    .collect::<PartialVMResult<Vec<Vec<_>>>>()?,
            ),
            Function => {
                // We cannot derive the actual type from layout, however, a dummy
                // function type will do since annotation of closures is not depending
                // actually on their type, but only their (hidden) captured arguments.
                // Currently, `from_runtime_layout` is only used to annotate captured arguments
                // of closures.
                FatType::Function(Arc::new(FatFunctionType {
                    args: vec![],
                    results: vec![],
                    abilities: AbilitySet::EMPTY,
                }))
            },
            Native(..) | Struct(_) => {
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

impl From<&TypeTag> for FatType {
    fn from(tag: &TypeTag) -> FatType {
        use FatType::*;
        match tag {
            TypeTag::Bool => Bool,
            TypeTag::U8 => U8,
            TypeTag::U16 => U16,
            TypeTag::U32 => U32,
            TypeTag::U64 => U64,
            TypeTag::U128 => U128,
            TypeTag::I8 => I8,
            TypeTag::I16 => I16,
            TypeTag::I32 => I32,
            TypeTag::I64 => I64,
            TypeTag::I128 => I128,
            TypeTag::I256 => I256,
            TypeTag::Address => Address,
            TypeTag::Signer => Signer,
            TypeTag::Vector(inner) => Vector(Arc::new(inner.as_ref().into())),
            TypeTag::Struct(inner) => Struct(Arc::new(inner.as_ref().into())),
            TypeTag::Function(inner) => Function(Arc::new(inner.as_ref().into())),
            TypeTag::U256 => U256,
        }
    }
}

impl From<&StructTag> for FatStructType {
    fn from(struct_tag: &StructTag) -> FatStructType {
        FatStructType {
            address: struct_tag.address,
            module: struct_tag.module.clone(),
            name: struct_tag.name.clone(),
            abilities: WrappedAbilitySet(AbilitySet::EMPTY), // We can't get abilities from a struct tag
            ty_args: struct_tag
                .type_args
                .iter()
                .map(|inner| inner.into())
                .collect(),
            layout: FatStructLayout::Singleton(vec![]), // We can't get field types from struct tag
        }
    }
}

impl From<&FunctionParamOrReturnTag> for FatType {
    fn from(tag: &FunctionParamOrReturnTag) -> FatType {
        use FatType::*;
        match tag {
            FunctionParamOrReturnTag::Reference(tag) => Reference(Arc::new(tag.into())),
            FunctionParamOrReturnTag::MutableReference(tag) => {
                MutableReference(Arc::new(tag.into()))
            },
            FunctionParamOrReturnTag::Value(tag) => tag.into(),
        }
    }
}

impl From<&FunctionTag> for FatFunctionType {
    fn from(fun_tag: &FunctionTag) -> FatFunctionType {
        let into_slice = |tys: &[FunctionParamOrReturnTag]| {
            tys.iter().map(|ty| ty.into()).collect::<Vec<FatType>>()
        };
        FatFunctionType {
            args: into_slice(&fun_tag.args),
            results: into_slice(&fun_tag.results),
            abilities: fun_tag.abilities,
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
            FatType::Reference(_) | FatType::MutableReference(_) | FatType::TyParam(_) => {
                return Err(PartialVMError::new(StatusCode::ABORT_TYPE_MISMATCH_ERROR))
            },
        })
    }
}
