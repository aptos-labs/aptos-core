// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

#![allow(clippy::non_canonical_partial_ord_impl)]

use crate::loaded_data::struct_name_indexing::StructNameIndex;
use derivative::Derivative;
use itertools::Itertools;
use move_binary_format::{
    errors::{Location, PartialVMError, PartialVMResult, VMResult},
    file_format::{
        SignatureToken, StructHandle, StructTypeParameter, TypeParameterIndex, VariantIndex,
    },
};
use move_core_types::{
    ability::{Ability, AbilitySet},
    identifier::Identifier,
    language_storage::{ModuleId, StructTag, TypeTag},
    vm_status::{sub_status::unknown_invariant_violation::EPARANOID_FAILURE, StatusCode},
};
use serde::Serialize;
use smallbitvec::SmallBitVec;
use smallvec::{smallvec, SmallVec};
use std::{
    cell::RefCell,
    cmp::max,
    collections::{btree_map, BTreeMap},
    fmt,
    fmt::Debug,
    sync::Arc,
};
use triomphe::Arc as TriompheArc;

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug)]
/// A formula describing the value depth of a type, using (the depths of) the type parameters as inputs.
///
/// It has the form of `max(CBase, T1 + C1, T2 + C2, ..)` where `Ti` is the depth of the ith type parameter
/// and `Ci` is just some constant.
///
/// This form has a special property: when you compute the max of multiple formulae, you can normalize
/// them into a single formula.
pub struct DepthFormula {
    pub terms: Vec<(TypeParameterIndex, u64)>, // Ti + Ci
    pub constant: Option<u64>,                 // Cbase
}

impl DepthFormula {
    pub fn constant(constant: u64) -> Self {
        Self {
            terms: vec![],
            constant: Some(constant),
        }
    }

    pub fn type_parameter(tparam: TypeParameterIndex) -> Self {
        Self {
            terms: vec![(tparam, 0)],
            constant: None,
        }
    }

    pub fn normalize(formulas: Vec<Self>) -> Self {
        let mut var_map = BTreeMap::new();
        let mut constant_acc = None;
        for formula in formulas {
            let Self { terms, constant } = formula;
            for (var, cur_factor) in terms {
                var_map
                    .entry(var)
                    .and_modify(|prev_factor| *prev_factor = max(cur_factor, *prev_factor))
                    .or_insert(cur_factor);
            }
            match (constant_acc, constant) {
                (_, None) => (),
                (None, Some(_)) => constant_acc = constant,
                (Some(c1), Some(c2)) => constant_acc = Some(max(c1, c2)),
            }
        }
        Self {
            terms: var_map.into_iter().collect(),
            constant: constant_acc,
        }
    }

    pub fn subst(
        &self,
        mut map: BTreeMap<TypeParameterIndex, DepthFormula>,
    ) -> PartialVMResult<DepthFormula> {
        let Self { terms, constant } = self;
        let mut formulas = vec![];
        if let Some(constant) = constant {
            formulas.push(DepthFormula::constant(*constant))
        }
        for (t_i, c_i) in terms {
            let Some(mut u_form) = map.remove(t_i) else {
                return Err(
                    PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                        .with_message(format!("{t_i:?} missing mapping")),
                );
            };
            u_form.scale(*c_i);
            formulas.push(u_form)
        }
        Ok(DepthFormula::normalize(formulas))
    }

    pub fn solve(&self, tparam_depths: &[u64]) -> u64 {
        let Self { terms, constant } = self;
        let mut depth = constant.as_ref().copied().unwrap_or(0);
        for (t_i, c_i) in terms {
            depth = max(depth, tparam_depths[*t_i as usize].saturating_add(*c_i))
        }
        depth
    }

    pub fn scale(&mut self, c: u64) {
        let Self { terms, constant } = self;
        for (_t_i, c_i) in terms {
            *c_i = (*c_i).saturating_add(c);
        }
        if let Some(cbase) = constant.as_mut() {
            *cbase = (*cbase).saturating_add(c);
        }
    }
}

#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub struct StructType {
    pub idx: StructNameIndex,
    pub layout: StructLayout,
    pub phantom_ty_params_mask: SmallBitVec,
    pub abilities: AbilitySet,
    pub ty_params: Vec<StructTypeParameter>,
}

#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub enum StructLayout {
    Single(Vec<(Identifier, Type)>),
    Variants(Vec<(Identifier, Vec<(Identifier, Type)>)>),
}

impl StructType {
    /// Get the fields from this struct type. If this is a proper struct, the `variant`
    /// must be None. Otherwise if its a variant struct, the variant for which the fields
    /// are requested must be given. For non-matching parameters, the function returns
    /// an empty list.
    pub fn fields(&self, variant: Option<VariantIndex>) -> PartialVMResult<&[(Identifier, Type)]> {
        match (&self.layout, variant) {
            (StructLayout::Single(fields), None) => Ok(fields.as_slice()),
            (StructLayout::Variants(variants), Some(variant))
                if (variant as usize) < variants.len() =>
            {
                Ok(variants[variant as usize].1.as_slice())
            },
            _ => Err(
                PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR).with_message(
                    "inconsistent struct field query: not a variant struct, or variant index out bounds"
                        .to_string(),
                ),
            ),
        }
    }

    /// Selects the field information from this struct type at the given offset. Returns
    /// error if field is not defined.
    pub fn field_at(
        &self,
        variant: Option<VariantIndex>,
        offset: usize,
    ) -> PartialVMResult<&(Identifier, Type)> {
        let slice = self.fields(variant)?;
        if offset < slice.len() {
            Ok(&slice[offset])
        } else {
            Err(
                PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR).with_message(
                    format!(
                        "field offset out of bounds -- len {} got {}",
                        slice.len(),
                        offset
                    ),
                ),
            )
        }
    }

    /// Same as `struct_type.fields(variant_opt).len()`
    pub fn field_count(&self, variant: Option<VariantIndex>) -> u16 {
        match (&self.layout, variant) {
            (StructLayout::Single(fields), None) => fields.len() as u16,
            (StructLayout::Variants(variants), Some(variant))
                if (variant as usize) < variants.len() =>
            {
                variants[variant as usize].1.len() as u16
            },
            _ => 0,
        }
    }

    /// Returns a string for the variant for error messages. If this is
    /// not a type with this variant, returns a string anyway indicating
    /// its undefined.
    pub fn variant_name_for_message(&self, variant: VariantIndex) -> String {
        let variant = variant as usize;
        match &self.layout {
            StructLayout::Variants(variants) if variant < variants.len() => {
                variants[variant].0.to_string()
            },
            _ => "<undefined>".to_string(),
        }
    }

    pub fn ty_param_constraints(&self) -> impl ExactSizeIterator<Item = &AbilitySet> {
        self.ty_params.iter().map(|param| &param.constraints)
    }

    // Check if the local struct handle is compatible with the defined struct type.
    pub fn check_compatibility(&self, struct_handle: &StructHandle) -> PartialVMResult<()> {
        if !struct_handle.abilities.is_subset(self.abilities) {
            return Err(
                PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                    .with_message("Ability definition of module mismatch".to_string()),
            );
        }

        if self.phantom_ty_params_mask.len() != struct_handle.type_parameters.len()
            || !self
                .phantom_ty_params_mask
                .iter()
                .zip(struct_handle.type_parameters.iter())
                .all(|(defined_is_phantom, local_type_parameter)| {
                    !local_type_parameter.is_phantom || defined_is_phantom
                })
        {
            return Err(
                PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR).with_message(
                    "Phantom type parameter definition of module mismatch".to_string(),
                ),
            );
        }

        Ok(())
    }

    #[cfg(test)]
    pub fn for_test() -> StructType {
        Self {
            idx: StructNameIndex::new(0),
            layout: StructLayout::Single(vec![]),
            phantom_ty_params_mask: SmallBitVec::new(),
            abilities: AbilitySet::EMPTY,
            ty_params: vec![],
        }
    }
}

#[derive(Debug, Clone, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct StructIdentifier {
    pub module: ModuleId,
    pub name: Identifier,
}

#[derive(Debug, Clone, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Type {
    Bool,
    U8,
    U64,
    U128,
    Address,
    Signer,
    Vector(TriompheArc<Type>),
    Struct {
        idx: StructNameIndex,
        ability: AbilityInfo,
    },
    StructInstantiation {
        idx: StructNameIndex,
        ty_args: TriompheArc<Vec<Type>>,
        ability: AbilityInfo,
    },
    Reference(Box<Type>),
    MutableReference(Box<Type>),
    TyParam(u16),
    U16,
    U32,
    U256,
}

pub struct TypePreorderTraversalIter<'a> {
    stack: SmallVec<[&'a Type; 32]>,
}

impl<'a> Iterator for TypePreorderTraversalIter<'a> {
    type Item = &'a Type;

    fn next(&mut self) -> Option<Self::Item> {
        use Type::*;

        match self.stack.pop() {
            Some(ty) => {
                match ty {
                    Signer
                    | Bool
                    | Address
                    | U8
                    | U16
                    | U32
                    | U64
                    | U128
                    | U256
                    | Struct { .. }
                    | TyParam(..) => (),

                    Reference(ty) | MutableReference(ty) => {
                        self.stack.push(ty);
                    },

                    Vector(ty) => {
                        self.stack.push(ty);
                    },

                    StructInstantiation { ty_args, .. } => self.stack.extend(ty_args.iter().rev()),
                }
                Some(ty)
            },
            None => None,
        }
    }
}

// Cache for the ability of struct. They will be ignored when comparing equality or Ord as they are just used for caching purpose.
#[derive(Derivative)]
#[derivative(Debug, Clone, Eq, Hash, PartialEq, Ord, PartialOrd)]
pub struct AbilityInfo {
    #[derivative(
        PartialEq = "ignore",
        Hash = "ignore",
        Ord = "ignore",
        PartialOrd = "ignore"
    )]
    base_ability_set: AbilitySet,

    #[derivative(
        PartialEq = "ignore",
        Hash = "ignore",
        Ord = "ignore",
        PartialOrd = "ignore"
    )]
    phantom_ty_args_mask: SmallBitVec,
}

impl AbilityInfo {
    pub fn struct_(ability: AbilitySet) -> Self {
        Self {
            base_ability_set: ability,
            phantom_ty_args_mask: SmallBitVec::new(),
        }
    }

    pub fn generic_struct(base_ability_set: AbilitySet, phantom_ty_args_mask: SmallBitVec) -> Self {
        Self {
            base_ability_set,
            phantom_ty_args_mask,
        }
    }
}

macro_rules! paranoid_failure {
    ($msg:ident) => {
        Err(
            PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                .with_message($msg)
                .with_sub_status(EPARANOID_FAILURE),
        )
    };
}

impl Type {
    pub fn verify_ty_arg_abilities<'a, I>(
        ty_param_abilities: I,
        ty_args: &[Self],
    ) -> PartialVMResult<()>
    where
        I: IntoIterator<Item = &'a AbilitySet>,
        I::IntoIter: ExactSizeIterator,
    {
        let ty_param_abilities = ty_param_abilities.into_iter();
        if ty_param_abilities.len() != ty_args.len() {
            return Err(PartialVMError::new(
                StatusCode::NUMBER_OF_TYPE_ARGUMENTS_MISMATCH,
            ));
        }
        for (ty, expected_ability_set) in ty_args.iter().zip(ty_param_abilities) {
            if !expected_ability_set.is_subset(ty.abilities()?) {
                return Err(PartialVMError::new(StatusCode::CONSTRAINT_NOT_SATISFIED));
            }
        }
        Ok(())
    }

    pub fn paranoid_check_is_bool_ty(&self) -> PartialVMResult<()> {
        if !matches!(self, Self::Bool) {
            let msg = format!("Expected boolean type, got {}", self);
            return paranoid_failure!(msg);
        }
        Ok(())
    }

    pub fn paranoid_check_is_u64_ty(&self) -> PartialVMResult<()> {
        if !matches!(self, Self::U64) {
            let msg = format!("Expected U64 type, got {}", self);
            return paranoid_failure!(msg);
        }
        Ok(())
    }

    pub fn paranoid_check_is_address_ty(&self) -> PartialVMResult<()> {
        if !matches!(self, Self::Address) {
            let msg = format!("Expected address type, got {}", self);
            return paranoid_failure!(msg);
        }
        Ok(())
    }

    pub fn paranoid_check_is_signer_ref_ty(&self) -> PartialVMResult<()> {
        if let Self::Reference(inner_ty) = self {
            if matches!(inner_ty.as_ref(), Self::Signer) {
                return Ok(());
            }
        }
        let msg = format!("Expected address type, got {}", self);
        paranoid_failure!(msg)
    }

    pub fn paranoid_check_has_ability(&self, ability: Ability) -> PartialVMResult<()> {
        if !self.abilities()?.has_ability(ability) {
            let msg = format!("Type {} does not have expected ability {}", self, ability);
            return paranoid_failure!(msg);
        }
        Ok(())
    }

    pub fn paranoid_check_abilities(&self, expected_abilities: AbilitySet) -> PartialVMResult<()> {
        let abilities = self.abilities()?;
        if !expected_abilities.is_subset(abilities) {
            let msg = format!(
                "Type {} has unexpected ability: expected {}, got {}",
                self, expected_abilities, abilities
            );
            return paranoid_failure!(msg);
        }
        Ok(())
    }

    pub fn paranoid_check_eq(&self, expected_ty: &Self) -> PartialVMResult<()> {
        if self != expected_ty {
            let msg = format!("Expected type {}, got {}", expected_ty, self);
            return paranoid_failure!(msg);
        }
        Ok(())
    }

    pub fn paranoid_check_is_vec_ty(&self, expected_elem_ty: &Self) -> PartialVMResult<()> {
        if let Self::Vector(elem_ty) = self {
            return elem_ty.paranoid_check_eq(expected_elem_ty);
        }

        let msg = format!("Expected vector type, got {}", self);
        paranoid_failure!(msg)
    }

    pub fn paranoid_check_is_vec_ref_ty<const IS_MUT: bool>(
        &self,
        expected_elem_ty: &Self,
    ) -> PartialVMResult<()> {
        if let Self::MutableReference(inner_ty) = self {
            if let Self::Vector(elem_ty) = inner_ty.as_ref() {
                elem_ty.paranoid_check_eq(expected_elem_ty)?;
                return Ok(());
            }
        }

        if let Self::Reference(inner_ty) = self {
            if !IS_MUT {
                if let Self::Vector(elem_ty) = inner_ty.as_ref() {
                    elem_ty.paranoid_check_eq(expected_elem_ty)?;
                    return Ok(());
                }
            }
        }

        let msg = format!(
            "Expected a (mutable: {}) vector reference, got {}",
            IS_MUT, self
        );
        paranoid_failure!(msg)
    }

    /// Returns an error if the type is not a (mutable) vector reference. Otherwise, returns
    /// a (mutable) reference to its element type.
    pub fn paranoid_check_and_get_vec_elem_ref_ty<const IS_MUT: bool>(
        &self,
        expected_elem_ty: &Self,
    ) -> PartialVMResult<Self> {
        self.paranoid_check_is_vec_ref_ty::<IS_MUT>(expected_elem_ty)?;
        let elem_ty = Box::new(self.get_vec_ref_elem_ty());

        // SAFETY: This type construction satisfies all constraints on size/depth because the parent
        //         vector reference type has been safely constructed.
        Ok(if IS_MUT {
            Type::MutableReference(elem_ty)
        } else {
            Type::Reference(elem_ty)
        })
    }

    /// Returns an error if the type is not a (mutable) vector reference. Otherwise, returns
    /// its element type.
    pub fn paranoid_check_and_get_vec_elem_ty<const IS_MUT: bool>(
        &self,
        expected_elem_ty: &Self,
    ) -> PartialVMResult<Self> {
        self.paranoid_check_is_vec_ref_ty::<IS_MUT>(expected_elem_ty)?;
        Ok(self.get_vec_ref_elem_ty())
    }

    fn get_vec_ref_elem_ty(&self) -> Self {
        match self {
            Self::Reference(inner_ty) | Self::MutableReference(inner_ty) => match inner_ty.as_ref()
            {
                Self::Vector(elem_ty) => elem_ty.as_ref().clone(),
                _ => unreachable!("The inner type must be a vector"),
            },
            _ => unreachable!("The top-level type must be a reference"),
        }
    }

    #[inline]
    pub fn paranoid_freeze_ref_ty(self) -> PartialVMResult<Type> {
        match self {
            Type::MutableReference(ty) => Ok(Type::Reference(ty)),
            _ => {
                let msg = format!("Expected a mutable reference to freeze, got {}", self);
                paranoid_failure!(msg)
            },
        }
    }

    #[inline]
    pub fn paranoid_read_ref(self) -> PartialVMResult<Type> {
        match self {
            Type::Reference(inner_ty) | Type::MutableReference(inner_ty) => {
                inner_ty.paranoid_check_has_ability(Ability::Copy)?;
                Ok(inner_ty.as_ref().clone())
            },
            _ => {
                let msg = format!("Expected a reference to read, got {}", self);
                paranoid_failure!(msg)
            },
        }
    }

    #[inline]
    pub fn paranoid_write_ref(&self, val_ty: &Type) -> PartialVMResult<()> {
        if let Type::MutableReference(inner_ty) = self {
            if inner_ty.as_ref() == val_ty {
                return inner_ty.paranoid_check_has_ability(Ability::Drop);
            }
        }

        let msg = format!("Cannot write type {} to type {}", val_ty, self);
        paranoid_failure!(msg)
    }

    pub fn paranoid_check_ref_eq(
        &self,
        expected_inner_ty: &Self,
        is_mut: bool,
    ) -> PartialVMResult<()> {
        match self {
            Type::MutableReference(inner_ty) => inner_ty.paranoid_check_eq(expected_inner_ty),
            Type::Reference(inner_ty) if !is_mut => inner_ty.paranoid_check_eq(expected_inner_ty),
            _ => {
                let msg = format!(
                    "Expected a (mutable: {}) reference type, got {}",
                    is_mut, self
                );
                paranoid_failure!(msg)
            },
        }
    }

    pub fn abilities(&self) -> PartialVMResult<AbilitySet> {
        match self {
            Type::Bool
            | Type::U8
            | Type::U16
            | Type::U32
            | Type::U64
            | Type::U128
            | Type::U256
            | Type::Address => Ok(AbilitySet::PRIMITIVES),

            // Technically unreachable but, no point in erroring if we don't have to
            Type::Reference(_) | Type::MutableReference(_) => Ok(AbilitySet::REFERENCES),
            Type::Signer => Ok(AbilitySet::SIGNER),

            Type::TyParam(_) => Err(PartialVMError::new(StatusCode::UNREACHABLE).with_message(
                "Unexpected TyParam type after translating from TypeTag to Type".to_string(),
            )),

            Type::Vector(ty) => {
                AbilitySet::polymorphic_abilities(AbilitySet::VECTOR, vec![false], vec![
                    ty.abilities()?
                ])
                .map_err(|e| {
                    PartialVMError::new(StatusCode::VERIFIER_INVARIANT_VIOLATION)
                        .with_message(e.to_string())
                })
            },
            Type::Struct { ability, .. } => Ok(ability.base_ability_set),
            Type::StructInstantiation {
                ty_args,
                ability:
                    AbilityInfo {
                        base_ability_set,
                        phantom_ty_args_mask,
                    },
                ..
            } => {
                let type_argument_abilities = ty_args
                    .iter()
                    .map(|arg| arg.abilities())
                    .collect::<PartialVMResult<Vec<_>>>()?;
                AbilitySet::polymorphic_abilities(
                    *base_ability_set,
                    phantom_ty_args_mask.iter(),
                    type_argument_abilities,
                )
                .map_err(|e| {
                    PartialVMError::new(StatusCode::VERIFIER_INVARIANT_VIOLATION)
                        .with_message(e.to_string())
                })
            },
        }
    }

    pub fn preorder_traversal(&self) -> TypePreorderTraversalIter<'_> {
        TypePreorderTraversalIter {
            stack: smallvec![self],
        }
    }

    /// Returns the number of nodes the type has.
    ///
    /// For example
    ///   - `u64` has one node
    ///   - `vector<u64>` has two nodes -- one for the vector and one for the element type u64.
    ///   - `Foo<u64, Bar<u8, bool>>` has 5 nodes.
    pub fn num_nodes(&self) -> usize {
        self.preorder_traversal().count()
    }

    /// Calculates the number of nodes in the substituted type.
    pub fn num_nodes_in_subst(&self, ty_args: &[Type]) -> PartialVMResult<usize> {
        use Type::*;

        thread_local! {
            static CACHE: RefCell<BTreeMap<usize, usize>> = const { RefCell::new(BTreeMap::new()) };
        }

        CACHE.with(|cache| {
            let mut cache = cache.borrow_mut();
            cache.clear();
            let mut num_nodes_in_arg = |idx: usize| -> PartialVMResult<usize> {
                Ok(match cache.entry(idx) {
                    btree_map::Entry::Occupied(entry) => *entry.into_mut(),
                    btree_map::Entry::Vacant(entry) => {
                        let ty = ty_args.get(idx).ok_or_else(|| {
                            PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                                .with_message(format!(
                                "type substitution failed: index out of bounds -- len {} got {}",
                                ty_args.len(),
                                idx
                            ))
                        })?;
                        *entry.insert(ty.num_nodes())
                    },
                })
            };

            let mut n = 0;
            for ty in self.preorder_traversal() {
                match ty {
                    TyParam(idx) => {
                        n += num_nodes_in_arg(*idx as usize)?;
                    },
                    Address
                    | Bool
                    | Signer
                    | U8
                    | U16
                    | U32
                    | U64
                    | U128
                    | U256
                    | Vector(..)
                    | Struct { .. }
                    | Reference(..)
                    | MutableReference(..)
                    | StructInstantiation { .. } => n += 1,
                }
            }

            Ok(n)
        })
    }
}

impl fmt::Display for StructIdentifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}::{}",
            self.module.short_str_lossless(),
            self.name.as_str()
        )
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Type::*;
        match self {
            Bool => f.write_str("bool"),
            U8 => f.write_str("u8"),
            U16 => f.write_str("u16"),
            U32 => f.write_str("u32"),
            U64 => f.write_str("u64"),
            U128 => f.write_str("u128"),
            U256 => f.write_str("u256"),
            Address => f.write_str("address"),
            Signer => f.write_str("signer"),
            Vector(et) => write!(f, "vector<{}>", et),
            Struct { idx, ability: _ } => write!(f, "s#{}", idx),
            StructInstantiation {
                idx,
                ty_args,
                ability: _,
            } => write!(
                f,
                "s#{}<{}>",
                idx,
                ty_args.iter().map(|t| t.to_string()).join(",")
            ),
            Reference(t) => write!(f, "&{}", t),
            MutableReference(t) => write!(f, "&mut {}", t),
            TyParam(no) => write!(f, "_{}", no),
        }
    }
}

/// Controls creation of runtime types, i.e., methods offered by this struct
/// should be the only way to construct any type.
#[derive(Clone, Serialize)]
pub struct TypeBuilder {
    // Maximum number of nodes a fully-instantiated type has.
    max_ty_size: u64,
    // Maximum depth (in terms of number of nodes) a fully-instantiated type has.
    max_ty_depth: u64,
}

impl TypeBuilder {
    pub fn with_limits(max_ty_size: u64, max_ty_depth: u64) -> Self {
        Self {
            max_ty_size,
            max_ty_depth,
        }
    }

    #[inline]
    pub fn create_bool_ty(&self) -> Type {
        Type::Bool
    }

    #[inline]
    pub fn create_u8_ty(&self) -> Type {
        Type::U8
    }

    #[inline]
    pub fn create_u16_ty(&self) -> Type {
        Type::U16
    }

    #[inline]
    pub fn create_u32_ty(&self) -> Type {
        Type::U32
    }

    #[inline]
    pub fn create_u64_ty(&self) -> Type {
        Type::U64
    }

    #[inline]
    pub fn create_u128_ty(&self) -> Type {
        Type::U128
    }

    #[inline]
    pub fn create_u256_ty(&self) -> Type {
        Type::U256
    }

    pub fn create_address_ty(&self) -> Type {
        Type::Address
    }

    pub fn create_signer_ty(&self) -> Type {
        Type::Signer
    }

    /// Creates a (possibly mutable) reference type from the given inner type.
    /// Returns an error if the type size or depth are too large.
    #[inline]
    pub fn create_ref_ty(&self, inner_ty: &Type, is_mut: bool) -> PartialVMResult<Type> {
        let mut count = 1;
        let check = |c: &mut u64, d: u64| self.check(c, d);
        let inner_ty = self
            .clone_impl(inner_ty, &mut count, 2, check)
            .map_err(|e| {
                e.append_message_with_separator(
                    '.',
                    format!(
                        "Failed to create a (mutable: {}) reference type with inner type {}",
                        is_mut, inner_ty
                    ),
                )
            })?;
        let inner_ty = Box::new(inner_ty);
        Ok(if is_mut {
            Type::MutableReference(inner_ty)
        } else {
            Type::Reference(inner_ty)
        })
    }

    /// Creates a vector type with the given element type, returning an error
    /// if the type size or depth are too large.
    #[inline]
    pub fn create_vec_ty(&self, elem_ty: &Type) -> PartialVMResult<Type> {
        let mut count = 1;
        let check = |c: &mut u64, d: u64| self.check(c, d);
        let elem_ty = self
            .clone_impl(elem_ty, &mut count, 2, check)
            .map_err(|e| {
                e.append_message_with_separator(
                    '.',
                    format!(
                        "Failed to create a vector type with element type {}",
                        elem_ty
                    ),
                )
            })?;
        Ok(Type::Vector(TriompheArc::new(elem_ty)))
    }

    #[inline]
    pub fn create_struct_ty(&self, idx: StructNameIndex, ability: AbilityInfo) -> Type {
        Type::Struct { idx, ability }
    }

    /// Creates a fully-instantiated struct type, performing the type substitution.
    /// Returns an error if the type size or depth are too large.
    #[inline]
    pub fn create_struct_instantiation_ty(
        &self,
        struct_ty: &StructType,
        ty_params: &[Type],
        ty_args: &[Type],
    ) -> PartialVMResult<Type> {
        // We cannot call substitution API directly because we have to take into
        // account struct type itself. We simply shift count and depth by 1 and
        // call inner APIs, to save extra cloning.
        let mut count = 1;
        let check = |c: &mut u64, d: u64| self.check(c, d);

        let ty_args = ty_params
            .iter()
            .map(|ty| {
                // Note that depth is 2 because we accounted for the parent struct type.
                self.subst_impl(ty, ty_args, &mut count, 2, check)
                    .map_err(|e| {
                        e.append_message_with_separator(
                            '.',
                            format!(
                                "Failed to instantiate a type {} with type arguments {:?}",
                                ty, ty_args
                            ),
                        )
                    })
            })
            .collect::<PartialVMResult<Vec<_>>>()?;

        Ok(Type::StructInstantiation {
            idx: struct_ty.idx,
            ty_args: triomphe::Arc::new(ty_args),
            ability: AbilityInfo::generic_struct(
                struct_ty.abilities,
                struct_ty.phantom_ty_params_mask.clone(),
            ),
        })
    }

    /// Creates a type for a Move constant. Note that constant types can be
    /// more restrictive and therefore have their own creation API.
    pub fn create_constant_ty(&self, const_tok: &SignatureToken) -> PartialVMResult<Type> {
        let mut count = 0;
        self.create_constant_ty_impl(const_tok, &mut count, 1)
            .map_err(|e| {
                e.append_message_with_separator(
                    '.',
                    format!(
                        "Failed to construct a type for constant token {:?}",
                        const_tok
                    ),
                )
            })
    }

    /// Creates a fully-instantiated type from its storage representation.
    pub fn create_ty<F>(&self, ty_tag: &TypeTag, mut resolver: F) -> VMResult<Type>
    where
        F: FnMut(&StructTag) -> VMResult<Arc<StructType>>,
    {
        let mut count = 0;
        self.create_ty_impl(ty_tag, &mut resolver, &mut count, 1)
    }

    /// Clones the given type, at the same time instantiating all its type parameters.
    pub fn create_ty_with_subst(&self, ty: &Type, ty_args: &[Type]) -> PartialVMResult<Type> {
        let mut count = 0;
        let check = |c: &mut u64, d: u64| self.check(c, d);
        self.subst_impl(ty, ty_args, &mut count, 1, check)
    }

    fn check(&self, count: &mut u64, depth: u64) -> PartialVMResult<()> {
        if *count >= self.max_ty_size {
            return Err(
                PartialVMError::new(StatusCode::TOO_MANY_TYPE_NODES).with_message(format!(
                    "Type size is larger than maximum {}",
                    self.max_ty_size
                )),
            );
        }
        if depth > self.max_ty_depth {
            return Err(
                PartialVMError::new(StatusCode::VM_MAX_TYPE_DEPTH_REACHED).with_message(format!(
                    "Type depth is larger than maximum {}",
                    self.max_ty_depth
                )),
            );
        }
        Ok(())
    }

    fn create_constant_ty_impl(
        &self,
        const_tok: &SignatureToken,
        count: &mut u64,
        depth: u64,
    ) -> PartialVMResult<Type> {
        use SignatureToken as S;
        use Type::*;

        self.check(count, depth)?;
        *count += 1;
        Ok(match const_tok {
            S::Bool => Bool,
            S::U8 => U8,
            S::U16 => U16,
            S::U32 => U32,
            S::U64 => U64,
            S::U128 => U128,
            S::U256 => U256,
            S::Address => Address,
            S::Vector(elem_tok) => {
                let elem_ty = self.create_constant_ty_impl(elem_tok, count, depth + 1)?;
                Vector(TriompheArc::new(elem_ty))
            },

            S::Struct(_) | S::StructInstantiation(_, _) => {
                return Err(
                    PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                        .with_message("Struct constants are not supported".to_string()),
                );
            },

            tok => {
                return Err(
                    PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                        .with_message(format!(
                            "{:?} is not allowed or is not a meaningful token for a constant",
                            tok
                        )),
                );
            },
        })
    }

    fn subst_impl<G>(
        &self,
        ty: &Type,
        ty_args: &[Type],
        count: &mut u64,
        depth: u64,
        check: G,
    ) -> PartialVMResult<Type>
    where
        G: Fn(&mut u64, u64) -> PartialVMResult<()> + Copy,
    {
        Self::apply_subst(
            ty,
            |idx, c, d| match ty_args.get(idx as usize) {
                Some(ty) => self.clone_impl(ty, c, d, check),
                None => Err(
                    PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                        .with_message(format!(
                        "Type substitution failed: index {} is out of bounds for {} type arguments",
                        idx,
                        ty_args.len()
                    )),
                ),
            },
            count,
            depth,
            check,
        )
    }

    fn clone_impl<G>(
        &self,
        ty: &Type,
        count: &mut u64,
        depth: u64,
        check: G,
    ) -> PartialVMResult<Type>
    where
        G: Fn(&mut u64, u64) -> PartialVMResult<()> + Copy,
    {
        Self::apply_subst(
            ty,
            |idx, _, _| {
                // The type cannot contain type parameters anymore (it also does not make
                // sense to have them!), and so it is the caller's responsibility to ensure
                // type substitution has been performed.
                Err(
                    PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                        .with_message(format!(
                        "There is an unresolved type parameter (index: {}) when cloning type {}",
                        idx, ty
                    )),
                )
            },
            count,
            depth,
            check,
        )
    }

    fn apply_subst<F, G>(
        ty: &Type,
        subst: F,
        count: &mut u64,
        depth: u64,
        check: G,
    ) -> PartialVMResult<Type>
    where
        F: Fn(u16, &mut u64, u64) -> PartialVMResult<Type> + Copy,
        G: Fn(&mut u64, u64) -> PartialVMResult<()> + Copy,
    {
        use Type::*;

        check(count, depth)?;
        *count += 1;
        Ok(match ty {
            TyParam(idx) => {
                // To avoid double-counting, revert counting the type parameter.
                *count -= 1;
                subst(*idx, count, depth)?
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
            Vector(elem_ty) => {
                let elem_ty = Self::apply_subst(elem_ty, subst, count, depth + 1, check)?;
                Vector(TriompheArc::new(elem_ty))
            },
            Reference(inner_ty) => {
                let inner_ty = Self::apply_subst(inner_ty, subst, count, depth + 1, check)?;
                Reference(Box::new(inner_ty))
            },
            MutableReference(inner_ty) => {
                let inner_ty = Self::apply_subst(inner_ty, subst, count, depth + 1, check)?;
                MutableReference(Box::new(inner_ty))
            },
            Struct { idx, ability } => Struct {
                idx: *idx,
                ability: ability.clone(),
            },
            StructInstantiation {
                idx,
                ty_args: non_instantiated_tys,
                ability,
            } => {
                let mut instantiated_tys = vec![];
                for ty in non_instantiated_tys.iter() {
                    let ty = Self::apply_subst(ty, subst, count, depth + 1, check)?;
                    instantiated_tys.push(ty);
                }
                StructInstantiation {
                    idx: *idx,
                    ty_args: TriompheArc::new(instantiated_tys),
                    ability: ability.clone(),
                }
            },
        })
    }

    fn create_ty_impl<F>(
        &self,
        ty_tag: &TypeTag,
        resolver: &mut F,
        count: &mut u64,
        depth: u64,
    ) -> VMResult<Type>
    where
        F: FnMut(&StructTag) -> VMResult<Arc<StructType>>,
    {
        use Type::*;
        use TypeTag as T;

        self.check(count, depth)
            .map_err(|e| e.finish(Location::Undefined))?;
        *count += 1;
        Ok(match ty_tag {
            T::Bool => Bool,
            T::U8 => U8,
            T::U16 => U16,
            T::U32 => U32,
            T::U64 => U64,
            T::U128 => U128,
            T::U256 => U256,
            T::Address => Address,
            T::Signer => Signer,
            T::Vector(elem_ty_tag) => {
                let elem_ty = self.create_ty_impl(elem_ty_tag, resolver, count, depth + 1)?;
                Vector(triomphe::Arc::new(elem_ty))
            },
            T::Struct(struct_tag) => {
                let struct_ty = resolver(struct_tag.as_ref())?;

                if struct_ty.ty_params.is_empty() && struct_tag.type_args.is_empty() {
                    Struct {
                        idx: struct_ty.idx,
                        ability: AbilityInfo::struct_(struct_ty.abilities),
                    }
                } else {
                    let mut ty_args = vec![];
                    for ty_arg in &struct_tag.type_args {
                        let ty_arg = self.create_ty_impl(ty_arg, resolver, count, depth + 1)?;
                        ty_args.push(ty_arg);
                    }
                    Type::verify_ty_arg_abilities(struct_ty.ty_param_constraints(), &ty_args)
                        .map_err(|e| e.finish(Location::Undefined))?;
                    StructInstantiation {
                        idx: struct_ty.idx,
                        ty_args: triomphe::Arc::new(ty_args),
                        ability: AbilityInfo::generic_struct(
                            struct_ty.abilities,
                            struct_ty.phantom_ty_params_mask.clone(),
                        ),
                    }
                }
            },
        })
    }

    #[cfg(test)]
    fn num_nodes_in_subst(&self, ty: &Type, ty_args: &[Type]) -> PartialVMResult<usize> {
        let mut count = 0;

        let check = |c: &mut u64, d: u64| self.check(c, d);
        self.subst_impl(ty, ty_args, &mut count, 1, check)?;
        Ok(count as usize)
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;
    use claims::{assert_err, assert_matches, assert_ok};
    use move_binary_format::file_format::StructHandleIndex;

    fn struct_instantiation_ty_for_test(ty_args: Vec<Type>) -> Type {
        Type::StructInstantiation {
            idx: StructNameIndex::new(0),
            ability: AbilityInfo::struct_(AbilitySet::EMPTY),
            ty_args: TriompheArc::new(ty_args),
        }
    }

    fn struct_ty_for_test() -> Type {
        Type::Struct {
            idx: StructNameIndex::new(0),
            ability: AbilityInfo::struct_(AbilitySet::EMPTY),
        }
    }

    fn nested_vec_for_test(ty_depth: u64) -> (Type, SignatureToken, TypeTag) {
        use SignatureToken as S;
        use Type::*;
        use TypeTag as T;

        let mut ty = U8;
        let mut tok = S::U8;
        let mut tag = T::U8;

        let mut depth = 1;
        while depth < ty_depth {
            ty = Vector(TriompheArc::new(ty.clone()));
            tok = S::Vector(Box::new(tok.clone()));
            tag = T::Vector(Box::new(tag.clone()));
            depth += 1;
        }
        (ty, tok, tag)
    }

    #[test]
    fn test_num_nodes_in_type() {
        use Type::*;

        let cases = [
            (U8, 1),
            (Vector(TriompheArc::new(U8)), 2),
            (Vector(TriompheArc::new(Vector(TriompheArc::new(U8)))), 3),
            (Reference(Box::new(Bool)), 2),
            (TyParam(0), 1),
            (struct_ty_for_test(), 1),
            (struct_instantiation_ty_for_test(vec![U8, U8]), 3),
            (
                struct_instantiation_ty_for_test(vec![
                    U8,
                    struct_instantiation_ty_for_test(vec![Bool, Bool, Bool]),
                    U8,
                ]),
                7,
            ),
        ];

        for (ty, expected) in cases {
            assert_eq!(ty.num_nodes(), expected);
        }
    }

    #[test]
    fn test_num_nodes_in_subst() {
        use Type::*;

        let ty_builder = TypeBuilder::with_limits(11, 5);
        let cases: Vec<(Type, Vec<Type>, usize)> = vec![
            (TyParam(0), vec![Bool], 1),
            (TyParam(0), vec![Vector(TriompheArc::new(Bool))], 2),
            (Bool, vec![], 1),
            (
                struct_instantiation_ty_for_test(vec![TyParam(0), TyParam(0)]),
                vec![Vector(TriompheArc::new(Bool))],
                5,
            ),
            (
                struct_instantiation_ty_for_test(vec![TyParam(0), TyParam(1)]),
                vec![
                    Vector(TriompheArc::new(Bool)),
                    Vector(TriompheArc::new(Vector(TriompheArc::new(Bool)))),
                ],
                6,
            ),
        ];

        for (ty, ty_args, expected_num_nodes) in cases {
            let num_nodes = assert_ok!(ty_builder.num_nodes_in_subst(&ty, &ty_args));
            assert_eq!(num_nodes, expected_num_nodes);
            assert_eq!(ty.num_nodes_in_subst(&ty_args).unwrap(), expected_num_nodes);
        }
    }

    #[test]
    fn test_substitution_large_depth() {
        use Type::*;

        let ty_builder = TypeBuilder::with_limits(11, 5);

        let ty = Vector(TriompheArc::new(Vector(TriompheArc::new(TyParam(0)))));
        let ty_arg = Vector(TriompheArc::new(Vector(TriompheArc::new(Bool))));
        assert_ok!(ty_builder.create_ty_with_subst(&ty, &[ty_arg.clone()]));

        let ty_arg = Vector(TriompheArc::new(ty_arg));
        let err = assert_err!(ty_builder.create_ty_with_subst(&ty, &[ty_arg]));
        assert_eq!(err.major_status(), StatusCode::VM_MAX_TYPE_DEPTH_REACHED);
    }

    #[test]
    fn test_substitution_large_count() {
        use Type::*;

        let ty_builder = TypeBuilder::with_limits(11, 5);

        let ty_params: Vec<Type> = (0..5).map(TyParam).collect();
        let ty = struct_instantiation_ty_for_test(ty_params);

        // Each type argument contributes 2 nodes, so in total the count is 11.
        let ty_args: Vec<Type> = (0..5).map(|_| Vector(TriompheArc::new(Bool))).collect();
        let num_nodes = assert_ok!(ty_builder.num_nodes_in_subst(&ty, &ty_args));
        assert_eq!(num_nodes, 11);

        let ty_args: Vec<Type> = (0..5)
            .map(|i| {
                if i == 4 {
                    // 3 nodes, to increase the total count to 12.
                    struct_instantiation_ty_for_test(vec![U64, struct_ty_for_test()])
                } else {
                    Vector(TriompheArc::new(Bool))
                }
            })
            .collect();
        let err = assert_err!(ty_builder.create_ty_with_subst(&ty, &ty_args));
        assert_eq!(err.major_status(), StatusCode::TOO_MANY_TYPE_NODES);
    }

    #[test]
    fn test_create_primitive_tys() {
        use Type::*;

        // Limits are irrelevant here.
        let ty_builder = TypeBuilder::with_limits(1, 1);

        assert_eq!(ty_builder.create_u8_ty(), U8);
        assert_eq!(ty_builder.create_u16_ty(), U16);
        assert_eq!(ty_builder.create_u32_ty(), U32);
        assert_eq!(ty_builder.create_u64_ty(), U64);
        assert_eq!(ty_builder.create_u128_ty(), U128);
        assert_eq!(ty_builder.create_u256_ty(), U256);
        assert_eq!(ty_builder.create_bool_ty(), Bool);
    }

    #[test]
    fn test_create_struct_ty() {
        let idx = StructNameIndex::new(0);
        let ability_info = AbilityInfo::struct_(AbilitySet::EMPTY);

        // Limits are not relevant here.
        let struct_ty = TypeBuilder::with_limits(1, 1).create_struct_ty(idx, ability_info.clone());
        assert_matches!(struct_ty, Type::Struct { .. });
    }

    #[test]
    fn test_create_struct_instantiation_ty() {
        use Type::*;

        let struct_ty = StructType::for_test();
        let ty_params = [TyParam(0), Bool, TyParam(1)];

        // Should succeed, type size limit is 5, and we have 5 nodes.
        let ty_builder = TypeBuilder::with_limits(5, 100);
        let ty_args = [Bool, Vector(TriompheArc::new(Bool))];
        assert_ok!(ty_builder.create_struct_instantiation_ty(&struct_ty, &ty_params, &ty_args));

        // Should fail, we have size of 6 now.
        let ty_args = [
            Vector(TriompheArc::new(Bool)),
            Vector(TriompheArc::new(Bool)),
        ];
        let err = assert_err!(
            ty_builder.create_struct_instantiation_ty(&struct_ty, &ty_params, &ty_args)
        );
        assert_eq!(err.major_status(), StatusCode::TOO_MANY_TYPE_NODES);

        // Should succeed, type depth limit is 4, and we have 4 nodes (3 in type parameter + struct).
        let nested_vec = Vector(TriompheArc::new(Vector(TriompheArc::new(Bool))));
        let ty_args = vec![Bool, nested_vec.clone()];
        let ty_builder = TypeBuilder::with_limits(100, 4);
        assert_ok!(ty_builder.create_struct_instantiation_ty(&struct_ty, &ty_params, &ty_args));

        // Should fail, we have depth of 5 now.
        let ty_params = vec![Bool, Vector(TriompheArc::new(nested_vec))];
        let err = assert_err!(
            ty_builder.create_struct_instantiation_ty(&struct_ty, &ty_params, &ty_args)
        );
        assert_eq!(err.major_status(), StatusCode::VM_MAX_TYPE_DEPTH_REACHED);
    }

    #[test]
    fn test_create_vec_ty() {
        let max_ty_depth = 5;
        let ty_builder = TypeBuilder::with_limits(100, max_ty_depth);

        let mut depth = 1;
        let mut ty = Type::Bool;
        while depth < max_ty_depth {
            ty = assert_ok!(ty_builder.create_vec_ty(&ty));
            assert_matches!(ty, Type::Vector(_));
            depth += 1;
        }
        assert_eq!(depth, max_ty_depth);

        // Type creation fails on exceeding the depth.
        let err = assert_err!(ty_builder.create_vec_ty(&ty));
        assert_eq!(err.major_status(), StatusCode::VM_MAX_TYPE_DEPTH_REACHED);

        // The checks are always ordered: first number of nodes, then depth. Using
        // a type builder with smaller than depth size limit must return a different
        // error code.
        let max_ty_size = 5;
        let ty_builder = TypeBuilder::with_limits(max_ty_size, 100);
        let err = assert_err!(ty_builder.create_vec_ty(&ty));
        assert_eq!(err.major_status(), StatusCode::TOO_MANY_TYPE_NODES);
    }

    #[test]
    fn test_create_ref_ty() {
        let max_ty_depth = 5;
        let ty_builder = TypeBuilder::with_limits(100, max_ty_depth);

        let mut depth = 1;
        let mut ty = Type::Bool;
        while depth < max_ty_depth {
            ty = assert_ok!(ty_builder.create_ref_ty(&ty, false));
            assert_matches!(ty, Type::Reference(_));
            depth += 1;
        }
        assert_eq!(depth, max_ty_depth);

        let err = assert_err!(ty_builder.create_ref_ty(&ty, false));
        assert_eq!(err.major_status(), StatusCode::VM_MAX_TYPE_DEPTH_REACHED);

        let max_ty_size = 5;
        let ty_builder = TypeBuilder::with_limits(max_ty_size, 100);
        let err = assert_err!(ty_builder.create_ref_ty(&ty, false));
        assert_eq!(err.major_status(), StatusCode::TOO_MANY_TYPE_NODES);
    }

    #[test]
    fn test_create_mut_ref_ty() {
        let max_ty_depth = 5;
        let ty_builder = TypeBuilder::with_limits(100, max_ty_depth);

        let mut depth = 1;
        let mut ty = Type::Bool;
        while depth < max_ty_depth {
            ty = assert_ok!(ty_builder.create_ref_ty(&ty, true));
            assert_matches!(ty, Type::MutableReference(_));
            depth += 1;
        }
        assert_eq!(depth, max_ty_depth);

        let err = assert_err!(ty_builder.create_ref_ty(&ty, true));
        assert_eq!(err.major_status(), StatusCode::VM_MAX_TYPE_DEPTH_REACHED);

        let max_ty_size = 5;
        let ty_builder = TypeBuilder::with_limits(max_ty_size, 100);
        let err = assert_err!(ty_builder.create_ref_ty(&ty, true));
        assert_eq!(err.major_status(), StatusCode::TOO_MANY_TYPE_NODES);
    }

    #[test]
    fn test_create_constant_ty() {
        use SignatureToken as S;
        use Type::*;

        let max_ty_depth = 5;
        let ty_builder = TypeBuilder::with_limits(100, max_ty_depth);

        assert_eq!(assert_ok!(ty_builder.create_constant_ty(&S::U8)), U8);
        assert_eq!(assert_ok!(ty_builder.create_constant_ty(&S::U16)), U16);
        assert_eq!(assert_ok!(ty_builder.create_constant_ty(&S::U32)), U32);
        assert_eq!(assert_ok!(ty_builder.create_constant_ty(&S::U64)), U64);
        assert_eq!(assert_ok!(ty_builder.create_constant_ty(&S::U128)), U128);
        assert_eq!(assert_ok!(ty_builder.create_constant_ty(&S::U256)), U256);
        assert_eq!(assert_ok!(ty_builder.create_constant_ty(&S::Bool)), Bool);
        assert_eq!(
            assert_ok!(ty_builder.create_constant_ty(&S::Address)),
            Address
        );

        // Vectors are special, because we limit their depth (and size).
        // Here, we test the boundary cases.

        for depth in [max_ty_depth - 1, max_ty_depth] {
            let (expected_ty, vec_tok, _) = nested_vec_for_test(depth);
            let ty = assert_ok!(ty_builder.create_constant_ty(&vec_tok));
            assert_eq!(&ty, &expected_ty);
        }

        let (_, vec_tok, _) = nested_vec_for_test(max_ty_depth + 1);
        let err = assert_err!(ty_builder.create_constant_ty(&vec_tok));
        assert_eq!(err.major_status(), StatusCode::VM_MAX_TYPE_DEPTH_REACHED);

        let max_ty_size = 5;
        let ty_builder = TypeBuilder::with_limits(max_ty_size, 100);

        for size in [max_ty_size - 1, max_ty_size] {
            let (expected_ty, vec_tok, _) = nested_vec_for_test(size);
            let ty = assert_ok!(ty_builder.create_constant_ty(&vec_tok));
            assert_eq!(&ty, &expected_ty);
        }

        let (_, vec_tok, _) = nested_vec_for_test(max_ty_size + 1);
        let err = assert_err!(ty_builder.create_constant_ty(&vec_tok));
        assert_eq!(err.major_status(), StatusCode::TOO_MANY_TYPE_NODES);

        // The following tokens cannot be constants:

        let struct_tok = S::Struct(StructHandleIndex::new(0));
        assert_err!(ty_builder.create_constant_ty(&struct_tok));

        let struct_instantiation_tok = S::StructInstantiation(StructHandleIndex::new(0), vec![]);
        assert_err!(ty_builder.create_constant_ty(&struct_instantiation_tok));

        assert_err!(ty_builder.create_constant_ty(&S::Signer));

        let ref_tok = S::Reference(Box::new(S::U8));
        assert_err!(ty_builder.create_constant_ty(&ref_tok));

        let mut_ref_tok = S::Reference(Box::new(S::U8));
        assert_err!(ty_builder.create_constant_ty(&mut_ref_tok));

        let ty_param_tok = S::TypeParameter(0);
        assert_err!(ty_builder.create_constant_ty(&ty_param_tok));
    }

    #[test]
    fn test_create_ty() {
        use Type::*;
        use TypeTag as T;

        let max_ty_size = 11;
        let max_ty_depth = 5;
        let ty_builder = TypeBuilder::with_limits(max_ty_size, max_ty_depth);

        let no_op = |_: &StructTag| unreachable!("Should not be called");

        // Primitive types.

        assert_eq!(assert_ok!(ty_builder.create_ty(&T::U8, no_op)), U8);
        assert_eq!(assert_ok!(ty_builder.create_ty(&T::U16, no_op)), U16);
        assert_eq!(assert_ok!(ty_builder.create_ty(&T::U32, no_op)), U32);
        assert_eq!(assert_ok!(ty_builder.create_ty(&T::U64, no_op)), U64);
        assert_eq!(assert_ok!(ty_builder.create_ty(&T::U128, no_op)), U128);
        assert_eq!(assert_ok!(ty_builder.create_ty(&T::U256, no_op)), U256);
        assert_eq!(assert_ok!(ty_builder.create_ty(&T::Bool, no_op)), Bool);
        assert_eq!(
            assert_ok!(ty_builder.create_ty(&T::Address, no_op)),
            Address
        );
        assert_eq!(assert_ok!(ty_builder.create_ty(&T::Signer, no_op)), Signer);

        // Vectors.

        for depth in [max_ty_depth - 1, max_ty_depth] {
            let (expected_ty, _, vec_tag) = nested_vec_for_test(depth);
            let ty = assert_ok!(ty_builder.create_ty(&vec_tag, no_op));
            assert_eq!(&ty, &expected_ty);
        }

        let (_, _, vec_tag) = nested_vec_for_test(max_ty_depth + 1);
        let err = assert_err!(ty_builder.create_ty(&vec_tag, no_op));
        assert_eq!(err.major_status(), StatusCode::VM_MAX_TYPE_DEPTH_REACHED);

        let max_ty_size = 5;
        let ty_builder = TypeBuilder::with_limits(max_ty_size, 100);

        for size in [max_ty_size - 1, max_ty_size] {
            let (expected_ty, _, vec_tag) = nested_vec_for_test(size);
            let ty = assert_ok!(ty_builder.create_ty(&vec_tag, no_op));
            assert_eq!(&ty, &expected_ty);
        }

        let (_, _, vec_tag) = nested_vec_for_test(max_ty_size + 1);
        let err = assert_err!(ty_builder.create_ty(&vec_tag, no_op));
        assert_eq!(err.major_status(), StatusCode::TOO_MANY_TYPE_NODES);
    }
}
