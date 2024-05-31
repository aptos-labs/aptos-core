// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

#![allow(clippy::non_canonical_partial_ord_impl)]

use derivative::Derivative;
use itertools::Itertools;
use move_binary_format::{
    errors::{Location, PartialVMError, PartialVMResult, VMResult},
    file_format::{
        Ability, AbilitySet, SignatureToken, StructHandle, StructTypeParameter, TypeParameterIndex,
    },
};
use move_core_types::{
    identifier::Identifier,
    language_storage::{ModuleId, StructTag, TypeTag},
    vm_status::{
        sub_status::unknown_invariant_violation::EPARANOID_FAILURE, StatusCode,
        StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
    },
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
    pub field_tys: Vec<Type>,
    pub field_names: Vec<Identifier>,
    pub phantom_ty_params_mask: SmallBitVec,
    pub abilities: AbilitySet,
    pub ty_params: Vec<StructTypeParameter>,
    pub name: Identifier,
    pub module: ModuleId,
}

impl StructType {
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
}

#[derive(Debug, Copy, Clone, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct StructNameIndex(pub usize);

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

    pub fn paranoid_check_is_vec_ref_ty(
        &self,
        expected_elem_ty: &Self,
        is_mut: bool,
    ) -> PartialVMResult<()> {
        if let Self::MutableReference(inner_ty) = self {
            if let Self::Vector(elem_ty) = inner_ty.as_ref() {
                elem_ty.paranoid_check_eq(expected_elem_ty)?;
                return Ok(());
            }
        }

        if let Self::Reference(inner_ty) = self {
            if !is_mut {
                if let Self::Vector(elem_ty) = inner_ty.as_ref() {
                    elem_ty.paranoid_check_eq(expected_elem_ty)?;
                    return Ok(());
                }
            }
        }

        let msg = format!(
            "Expected a (mutable: {}) vector reference, got {}",
            is_mut, self
        );
        paranoid_failure!(msg)
    }

    /// Returns an error if the type is not a (mutable) vector reference. Otherwise, returns
    /// a (mutable) reference to its element type.
    pub fn paranoid_check_and_get_vec_elem_ref_ty(
        &self,
        expected_elem_ty: &Self,
        is_mut: bool,
    ) -> PartialVMResult<Self> {
        self.paranoid_check_is_vec_ref_ty(expected_elem_ty, is_mut)?;
        let elem_ty = Box::new(self.get_vec_ref_elem_ty());

        // SAFETY: This type construction satisfies all constraints on size/depth because the parent
        //         vector reference type has been safely constructed.
        Ok(if is_mut {
            Type::MutableReference(elem_ty)
        } else {
            Type::Reference(elem_ty)
        })
    }

    /// Returns an error if the type is not a (mutable) vector reference. Otherwise, returns
    /// its element type.
    pub fn paranoid_check_and_get_vec_elem_ty(
        &self,
        expected_elem_ty: &Self,
        is_mut: bool,
    ) -> PartialVMResult<Self> {
        self.paranoid_check_is_vec_ref_ty(expected_elem_ty, is_mut)?;
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
            static CACHE: RefCell<BTreeMap<usize, usize>> = RefCell::new(BTreeMap::new());
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
            Struct { idx, ability: _ } => write!(f, "s#{}", idx.0),
            StructInstantiation {
                idx,
                ty_args,
                ability: _,
            } => write!(
                f,
                "s#{}<{}>",
                idx.0,
                ty_args.iter().map(|t| t.to_string()).join(",")
            ),
            Reference(t) => write!(f, "&{}", t),
            MutableReference(t) => write!(f, "&mut {}", t),
            TyParam(no) => write!(f, "_{}", no),
        }
    }
}

#[derive(Clone, Serialize)]
pub struct TypeConfig {
    // Maximum number of nodes a fully-instantiated type has.
    max_ty_size: usize,
    // Maximum depth (in terms of number of nodes) a fully-instantiated type has.
    max_ty_depth: usize,
}

impl Default for TypeConfig {
    fn default() -> Self {
        Self {
            max_ty_size: 256,
            max_ty_depth: 256,
        }
    }
}

#[derive(Clone)]
pub struct TypeBuilder {
    max_ty_size: usize,
    max_ty_depth: usize,
}

impl TypeBuilder {
    pub fn new(ty_config: &TypeConfig) -> Self {
        Self {
            max_ty_size: ty_config.max_ty_size,
            max_ty_depth: ty_config.max_ty_depth,
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

    /// Creates a (possibly mutable) reference type from the given inner type.
    /// Returns an error if the type size or depth are too large.
    #[inline]
    pub fn create_ref_ty(&self, inner_ty: &Type, is_mut: bool) -> PartialVMResult<Type> {
        let mut count = 1;
        let inner_ty = self.clone_impl(inner_ty, &mut count, 2).map_err(|e| {
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
        let elem_ty = self.clone_impl(elem_ty, &mut count, 2).map_err(|e| {
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
        ty_params: Vec<Type>,
        ty_args: &[Type],
    ) -> PartialVMResult<Type> {
        let ty = Type::StructInstantiation {
            idx: struct_ty.idx,
            ty_args: triomphe::Arc::new(ty_params),
            ability: AbilityInfo::generic_struct(
                struct_ty.abilities,
                struct_ty.phantom_ty_params_mask.clone(),
            ),
        };

        // We need to count the struct type itself.
        let mut count = 1;
        self.subst_impl(&ty, ty_args, &mut count, 2).map_err(|e| {
            e.append_message_with_separator(
                '.',
                format!(
                    "Failed to instantiate a type {} with type arguments {:?}",
                    ty, ty_args
                ),
            )
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
        self.subst_impl(ty, ty_args, &mut count, 1)
    }

    fn check(&self, count: &mut usize, depth: usize) -> PartialVMResult<()> {
        if *count >= self.max_ty_size {
            return Err(PartialVMError::new(StatusCode::TOO_MANY_TYPE_NODES));
        }
        if depth > self.max_ty_depth {
            return Err(PartialVMError::new(StatusCode::VM_MAX_TYPE_DEPTH_REACHED));
        }
        Ok(())
    }

    fn create_constant_ty_impl(
        &self,
        const_tok: &SignatureToken,
        count: &mut usize,
        depth: usize,
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
                )
            },

            tok => {
                return Err(
                    PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                        .with_message(format!(
                            "{:?} is not allowed or is not a meaningful token for a constant",
                            tok
                        )),
                )
            },
        })
    }

    fn subst_impl(
        &self,
        ty: &Type,
        ty_args: &[Type],
        count: &mut usize,
        depth: usize,
    ) -> PartialVMResult<Type> {
        self.apply_subst(
            ty,
            |idx, c, d| match ty_args.get(idx as usize) {
                Some(ty) => self.clone_impl(ty, c, d),
                None => Err(
                    PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                        .with_message(format!(
                            "Type substitution failed: there are {} type arguments and index {} is out of bounds",
                            idx,
                            ty_args.len()
                        )),
                ),
            },
            count,
            depth,
        )
    }

    fn clone_impl(&self, ty: &Type, count: &mut usize, depth: usize) -> PartialVMResult<Type> {
        self.apply_subst(
            ty,
            |idx, _, _| {
                // The type cannot contain type parameters anymore (it also does not make
                // sense to have them!), and so it is the caller's responsibility to ensure
                // type substitution has been performed.
                Err(
                    PartialVMError::new(UNKNOWN_INVARIANT_VIOLATION_ERROR).with_message(format!(
                        "There is an unresolved type parameter (index: {}) when cloning type {}",
                        idx, ty
                    )),
                )
            },
            count,
            depth,
        )
    }

    fn apply_subst<F>(
        &self,
        ty: &Type,
        subst: F,
        count: &mut usize,
        depth: usize,
    ) -> PartialVMResult<Type>
    where
        F: Fn(u16, &mut usize, usize) -> PartialVMResult<Type> + Copy,
    {
        use Type::*;

        self.check(count, depth)?;
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
                let elem_ty = self.apply_subst(elem_ty, subst, count, depth + 1)?;
                Vector(TriompheArc::new(elem_ty))
            },
            Reference(inner_ty) => {
                let inner_ty = self.apply_subst(inner_ty, subst, count, depth + 1)?;
                Reference(Box::new(inner_ty))
            },
            MutableReference(inner_ty) => {
                let inner_ty = self.apply_subst(inner_ty, subst, count, depth + 1)?;
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
                for non_instantiated_ty in non_instantiated_tys.iter() {
                    let instantiated_ty =
                        self.apply_subst(non_instantiated_ty, subst, count, depth + 1)?;
                    instantiated_tys.push(instantiated_ty);
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
        count: &mut usize,
        depth: usize,
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
    pub fn new_for_test() -> Self {
        Self {
            max_ty_size: 11,
            max_ty_depth: 5,
        }
    }

    #[cfg(test)]
    fn num_nodes_in_subst(&self, ty: &Type, ty_args: &[Type]) -> PartialVMResult<usize> {
        let mut count = 0;
        self.subst_impl(ty, ty_args, &mut count, 1)?;
        Ok(count)
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;
    use claims::{assert_err, assert_ok};

    fn struct_instantiation_ty_for_test(ty_args: Vec<Type>) -> Type {
        Type::StructInstantiation {
            idx: StructNameIndex(0),
            ability: AbilityInfo::struct_(AbilitySet::EMPTY),
            ty_args: TriompheArc::new(ty_args),
        }
    }

    fn struct_ty_for_test() -> Type {
        Type::Struct {
            idx: StructNameIndex(0),
            ability: AbilityInfo::struct_(AbilitySet::EMPTY),
        }
    }

    fn vec_ty_for_test(mut depth: usize) -> Type {
        use Type::*;

        let mut ty = Address;
        depth -= 1;
        while depth > 0 {
            ty = Vector(TriompheArc::new(ty.clone()));
            depth -= 1;
        }
        ty
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

        let ty_builder = TypeBuilder::new_for_test();
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

        let ty_builder = TypeBuilder::new_for_test();

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

        let ty_builder = TypeBuilder::new_for_test();

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
    fn test_create_nested_tys() {
        let ty_builder = TypeBuilder::new_for_test();
        let ty = vec_ty_for_test(ty_builder.max_ty_depth - 1);

        // These types have the maximum possible depth!
        let vec_ty = assert_ok!(ty_builder.create_vec_ty(&ty));
        let ref_ty = assert_ok!(ty_builder.create_ref_ty(&ty, false));
        let mut_ref_ty = assert_ok!(ty_builder.create_ref_ty(&ty, true));

        let err = assert_err!(ty_builder.create_vec_ty(&vec_ty));
        assert_eq!(err.major_status(), StatusCode::VM_MAX_TYPE_DEPTH_REACHED);

        let err = assert_err!(ty_builder.create_ref_ty(&vec_ty, false));
        assert_eq!(err.major_status(), StatusCode::VM_MAX_TYPE_DEPTH_REACHED);

        let err = assert_err!(ty_builder.create_ref_ty(&vec_ty, true));
        assert_eq!(err.major_status(), StatusCode::VM_MAX_TYPE_DEPTH_REACHED);

        let err = assert_err!(ty_builder.create_vec_ty(&ref_ty));
        assert_eq!(err.major_status(), StatusCode::VM_MAX_TYPE_DEPTH_REACHED);

        let err = assert_err!(ty_builder.create_vec_ty(&mut_ref_ty));
        assert_eq!(err.major_status(), StatusCode::VM_MAX_TYPE_DEPTH_REACHED);
    }
}
