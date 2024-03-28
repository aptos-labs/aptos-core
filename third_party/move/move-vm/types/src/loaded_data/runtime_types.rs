// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

#![allow(clippy::non_canonical_partial_ord_impl)]

use crate::loaded_data::IndexMap;
use super::tuple_helper::KeyPair;
use dashmap::DashMap;
use derivative::Derivative;
use move_binary_format::{
    binary_views::BinaryIndexedView,
    errors::{PartialVMError, PartialVMResult},
    file_format::{
        AbilitySet, SignatureToken, StructHandle, StructTypeParameter, TypeParameterIndex,
    },
};
use move_core_types::{identifier::Identifier, language_storage::ModuleId, vm_status::StatusCode};
use smallbitvec::SmallBitVec;
use smallvec::{smallvec, SmallVec};
use std::{
    cell::RefCell,
    cmp::max,
    collections::{btree_map, BTreeMap},
    fmt::Debug,
};

pub const TYPE_DEPTH_MAX: usize = 256;

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
    pub fields: Vec<Type>,
    pub ty_idx: TypeIndex,
    pub field_idxs: Vec<TypeIndex>,
    pub field_names: Vec<Identifier>,
    pub phantom_ty_args_mask: SmallBitVec,
    pub abilities: AbilitySet,
    pub type_parameters: Vec<StructTypeParameter>,
    pub name: Identifier,
    pub module: ModuleId,
}

impl StructType {
    pub fn type_param_constraints(&self) -> impl ExactSizeIterator<Item = &AbilitySet> {
        self.type_parameters.iter().map(|param| &param.constraints)
    }

    // Check if the local struct handle is compatible with the defined struct type.
    pub fn check_compatibility(&self, struct_handle: &StructHandle) -> PartialVMResult<()> {
        if !struct_handle.abilities.is_subset(self.abilities) {
            return Err(
                PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                    .with_message("Ability definition of module mismatch".to_string()),
            );
        }

        if self.phantom_ty_args_mask.len() != struct_handle.type_parameters.len()
            || !self
                .phantom_ty_args_mask
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
#[derive(Debug, Copy, Clone, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct StructInstantiationIndex(pub usize);
#[derive(Debug, Copy, Clone, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct TypeIndex(pub usize);

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
    Vector(TypeIndex),
    Struct {
        idx: StructNameIndex,
        ability: AbilityInfo,
    },
    StructInstantiation {
        idx: StructInstantiationIndex,
        ability: AbilityInfo,
    },
    Reference(TypeIndex),
    MutableReference(TypeIndex),
    TyParam(u16),
    U16,
    U32,
    U256,
}

#[derive(Debug)]
pub struct TypeContext {
    identifier_cache: IndexMap<StructIdentifier>,
    instantiation_map: IndexMap<(StructNameIndex, Vec<Type>)>,
    type_id_map: IndexMap<Type>,
    type_inst: DashMap<(StructInstantiationIndex, Vec<Type>), StructInstantiationIndex>,
}

pub struct TypePreorderTraversalIter<'a> {
    context: &'a TypeContext,
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
                        self.stack.push(self.context.type_id_map.get_by_index(ty.0));
                    },

                    Vector(ty) => {
                        self.stack.push(self.context.type_id_map.get_by_index(ty.0));
                    },

                    StructInstantiation { idx, .. } => self
                        .stack
                        .extend(self.context.instantiation_map.get_by_index(idx.0).1.iter()),
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

impl Type {
    pub fn check_eq(&self, other: &Self) -> PartialVMResult<()> {
        if self != other {
            return Err(
                PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                    .with_message(format!(
                        "Type mismatch: expected {:?}, got {:?}",
                        self, other
                    ))
                    .with_sub_status(move_core_types::vm_status::sub_status::unknown_invariant_violation::EPARANOID_FAILURE),
            );
        }
        Ok(())
    }
}

impl TypeContext {
    pub fn new() -> Self {
        TypeContext {
            identifier_cache: IndexMap::new(),
            instantiation_map: IndexMap::new(),
            type_id_map: IndexMap::new(),
            type_inst: DashMap::new(),
        }
    }

    pub fn get_idx_by_identifier(&self, struct_identifier: StructIdentifier) -> StructNameIndex {
        StructNameIndex(self.identifier_cache.get_or_insert(struct_identifier))
    }

    pub fn get_identifier_by_idx(&self, struct_name_index: StructNameIndex) -> &StructIdentifier {
        self.identifier_cache.get_by_index(struct_name_index.0)
    }

    pub fn get_type_by_index(&self, ty_idx: TypeIndex) -> &Type {
        self.type_id_map.get_by_index(ty_idx.0)
    }

    pub fn get_idx_by_type(&self, ty: Type) -> TypeIndex {
        TypeIndex(self.type_id_map.get_or_insert(ty))
    }

    pub fn get_instantiation_by_index(
        &self,
        inst_idx: StructInstantiationIndex,
    ) -> &(StructNameIndex, Vec<Type>) {
        self.instantiation_map.get_by_index(inst_idx.0)
    }

    pub fn get_idx_by_instantiation(
        &self,
        idx: StructNameIndex,
        ty_args: Vec<Type>,
    ) -> StructInstantiationIndex {
        StructInstantiationIndex(self.instantiation_map.get_or_insert((idx, ty_args)))
    }

    pub fn load_signature_token(
        &self,
        module: BinaryIndexedView,
        tok: &SignatureToken,
        struct_name_table: &[StructNameIndex],
    ) -> PartialVMResult<Type> {
        let res = match tok {
            SignatureToken::Bool => Type::Bool,
            SignatureToken::U8 => Type::U8,
            SignatureToken::U16 => Type::U16,
            SignatureToken::U32 => Type::U32,
            SignatureToken::U64 => Type::U64,
            SignatureToken::U128 => Type::U128,
            SignatureToken::U256 => Type::U256,
            SignatureToken::Address => Type::Address,
            SignatureToken::Signer => Type::Signer,
            SignatureToken::TypeParameter(idx) => Type::TyParam(*idx),
            SignatureToken::Vector(inner_tok) => {
                let inner_type = self.load_signature_token(module, inner_tok, struct_name_table)?;
                Type::Vector(TypeIndex(self.type_id_map.get_or_insert(inner_type)))
            },
            SignatureToken::Reference(inner_tok) => {
                let inner_type = self.load_signature_token(module, inner_tok, struct_name_table)?;
                Type::Reference(TypeIndex(self.type_id_map.get_or_insert(inner_type)))
            },
            SignatureToken::MutableReference(inner_tok) => {
                let inner_type = self.load_signature_token(module, inner_tok, struct_name_table)?;
                Type::MutableReference(TypeIndex(self.type_id_map.get_or_insert(inner_type)))
            },
            SignatureToken::Struct(sh_idx) => {
                let struct_handle = module.struct_handle_at(*sh_idx);
                Type::Struct {
                    idx: struct_name_table[sh_idx.0 as usize],
                    ability: AbilityInfo::struct_(struct_handle.abilities),
                }
            },
            SignatureToken::StructInstantiation(sh_idx, tys) => {
                let type_args: Vec<_> = tys
                    .iter()
                    .map(|tok| self.load_signature_token(module, tok, struct_name_table))
                    .collect::<PartialVMResult<_>>()?;
                let struct_handle = module.struct_handle_at(*sh_idx);
                let inst_idx = self
                    .instantiation_map
                    .get_or_insert((struct_name_table[sh_idx.0 as usize], type_args));
                Type::StructInstantiation {
                    idx: StructInstantiationIndex(inst_idx),
                    ability: AbilityInfo::generic_struct(
                        struct_handle.abilities,
                        struct_handle
                            .type_parameters
                            .iter()
                            .map(|ty| ty.is_phantom)
                            .collect(),
                    ),
                }
            },
        };
        Ok(res)
    }

    pub fn subst(&self, ty: &Type, ty_args: &[Type]) -> PartialVMResult<Type> {
        self.subst_impl(ty, ty_args, 0)
    }

    fn subst_impl(&self, ty: &Type, ty_args: &[Type], depth: usize) -> PartialVMResult<Type> {
        if depth > TYPE_DEPTH_MAX {
            return Err(PartialVMError::new(StatusCode::VM_MAX_TYPE_DEPTH_REACHED));
        }
        Ok(match ty {
            Type::TyParam(idx) => ty_args[*idx as usize].clone(),
            Type::Bool => Type::Bool,
            Type::U8 => Type::U8,
            Type::U16 => Type::U16,
            Type::U32 => Type::U32,
            Type::U64 => Type::U64,
            Type::U128 => Type::U128,
            Type::U256 => Type::U256,
            Type::Address => Type::Address,
            Type::Signer => Type::Signer,
            Type::Struct { idx, ability } => Type::Struct {
                idx: *idx,
                ability: ability.clone(),
            },
            Type::Vector(ty) => Type::Vector(self.subst_ty_idx(*ty, ty_args, depth + 1)?),
            Type::Reference(ty) => Type::Reference(self.subst_ty_idx(*ty, ty_args, depth + 1)?),
            Type::MutableReference(ty) => {
                Type::MutableReference(self.subst_ty_idx(*ty, ty_args, depth + 1)?)
            },
            Type::StructInstantiation { idx, ability } => Type::StructInstantiation {
                idx: self.subst_struct_inst(*idx, ty_args, depth + 1)?,
                ability: ability.clone(),
            },
        })
    }

    fn subst_ty_idx(
        &self,
        idx: TypeIndex,
        ty_args: &[Type],
        depth: usize,
    ) -> PartialVMResult<TypeIndex> {
        let ty = self.subst_impl(self.type_id_map.get_by_index(idx.0), ty_args, depth)?;
        Ok(TypeIndex(self.type_id_map.get_or_insert(ty)))
    }

    fn subst_struct_inst(
        &self,
        idx: StructInstantiationIndex,
        ty_args: &[Type],
        depth: usize,
    ) -> PartialVMResult<StructInstantiationIndex> {
        let (struct_name, ty_inst) = self.instantiation_map.get_by_index(idx.0);
        if let Some(result) = self.type_inst.get(&(&idx, ty_args) as &dyn KeyPair) {
            return Ok(*result);
        }
        let substituted_tys = ty_inst
            .iter()
            .map(|ty| self.subst_impl(ty, ty_args, depth))
            .collect::<PartialVMResult<Vec<_>>>()?;
        let idx = StructInstantiationIndex(
            self.instantiation_map
                .get_or_insert((*struct_name, substituted_tys)),
        );
        self.type_inst.insert((idx, ty_args.to_vec()), idx);
        Ok(idx)
    }

    pub fn from_const_signature(
        &self,
        constant_signature: &SignatureToken,
    ) -> PartialVMResult<Type> {
        use SignatureToken as S;
        use Type as L;

        Ok(match constant_signature {
            S::Bool => L::Bool,
            S::U8 => L::U8,
            S::U16 => L::U16,
            S::U32 => L::U32,
            S::U64 => L::U64,
            S::U128 => L::U128,
            S::U256 => L::U256,
            S::Address => L::Address,
            S::Vector(inner) => L::Vector(TypeIndex(
                self.type_id_map
                    .get_or_insert(self.from_const_signature(inner)?),
            )),
            // Not yet supported
            S::Struct(_) | S::StructInstantiation(_, _) => {
                return Err(
                    PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                        .with_message("Unable to load const type signature".to_string()),
                )
            },
            // Not allowed/Not meaningful
            S::TypeParameter(_) | S::Reference(_) | S::MutableReference(_) | S::Signer => {
                return Err(
                    PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                        .with_message("Unable to load const type signature".to_string()),
                )
            },
        })
    }

    pub fn check_vec_ref(
        &self,
        lhs: &Type,
        inner_ty: TypeIndex,
        is_mut: bool,
    ) -> PartialVMResult<Type> {
        match lhs {
            Type::MutableReference(inner) => match self.type_id_map.get_by_index(inner.0) {
                Type::Vector(inner) if *inner == inner_ty => {
                    let inner = self.type_id_map.get_by_index(inner.0);
                    Ok(inner.clone())
                },
                _ => Err(
                    PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                        .with_message("VecMutBorrow expects a vector reference".to_string())
                        .with_sub_status(move_core_types::vm_status::sub_status::unknown_invariant_violation::EPARANOID_FAILURE),
                ),
            },
            Type::Reference(inner) if !is_mut => match self.type_id_map.get_by_index(inner.0) {
                Type::Vector(inner) if *inner == inner_ty => {
                    let inner = self.type_id_map.get_by_index(inner.0);
                    Ok(inner.clone())
                },
                _ => Err(
                    PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                        .with_message("VecMutBorrow expects a vector reference".to_string())
                        .with_sub_status(move_core_types::vm_status::sub_status::unknown_invariant_violation::EPARANOID_FAILURE),
                ),
            },
            _ => Err(
                PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                    .with_message(format!("VecMutBorrow expects a vector reference {:?} {:?}", lhs, inner_ty))
                    .with_sub_status(move_core_types::vm_status::sub_status::unknown_invariant_violation::EPARANOID_FAILURE),
            ),
        }
    }

    pub fn check_ref_eq(&self, lhs: &Type, expected_inner: TypeIndex) -> PartialVMResult<()> {
        match lhs {
            Type::MutableReference(inner) | Type::Reference(inner) if *inner == expected_inner => {
                Ok(())
            },
            _ => Err(
                PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                    .with_message("VecMutBorrow expects a vector reference".to_string()),
            ),
        }
    }

    pub fn preorder_traversal<'a>(&'a self, ty: &'a Type) -> TypePreorderTraversalIter<'a> {
        TypePreorderTraversalIter {
            context: self,
            stack: smallvec![ty],
        }
    }

    pub fn abilities(&self, ty: &Type) -> PartialVMResult<AbilitySet> {
        match ty {
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
                    self.abilities(self.type_id_map.get_by_index(ty.0))?
                ])
            },
            Type::Struct { ability, .. } => Ok(ability.base_ability_set),
            Type::StructInstantiation {
                idx,
                ability:
                    AbilityInfo {
                        base_ability_set,
                        phantom_ty_args_mask,
                    },
            } => {
                let type_argument_abilities = self
                    .instantiation_map
                    .get_by_index(idx.0)
                    .1
                    .iter()
                    .map(|arg| self.abilities(arg))
                    .collect::<PartialVMResult<Vec<_>>>()?;
                AbilitySet::polymorphic_abilities(
                    *base_ability_set,
                    phantom_ty_args_mask.iter(),
                    type_argument_abilities,
                )
            },
        }
    }

    /// Returns the number of nodes the type has.
    ///
    /// For example
    ///   - `u64` has one node
    ///   - `vector<u64>` has two nodes -- one for the vector and one for the element type u64.
    ///   - `Foo<u64, Bar<u8, bool>>` has 5 nodes.
    pub fn num_nodes(&self, ty: &Type) -> usize {
        self.preorder_traversal(ty).count()
    }

    /// Calculates the number of nodes in the substituted type.
    pub fn num_nodes_in_subst(&self, ty: &Type, ty_args: &[Type]) -> PartialVMResult<usize> {
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
                        *entry.insert(self.num_nodes(ty))
                    },
                })
            };

            let mut n = 0;
            for ty in self.preorder_traversal(ty) {
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

    pub fn format_type(&self, ty: &Type) -> String {
        match ty {
            Type::Bool
            | Type::U8
            | Type::U16
            | Type::U32
            | Type::U64
            | Type::U128
            | Type::U256
            | Type::Address
            | Type::Signer => format!("{:?}", ty),
            Type::Reference(idx) => format!("&{}", self.format_type(self.get_type_by_index(*idx))),
            Type::MutableReference(idx) => {
                format!("&mut {}", self.format_type(self.get_type_by_index(*idx)))
            },
            Type::Vector(idx) => format!("Vec<{}>", self.format_type(self.get_type_by_index(*idx))),
            Type::Struct { idx, .. } => {
                let struct_identifier = self.get_identifier_by_idx(*idx);
                format!("{}::{}", struct_identifier.module, struct_identifier.name)
            },
            Type::TyParam(id) => format!("ty_{}", id),
            Type::StructInstantiation { idx, .. } => {
                let (struct_identifier_id, ty_args) = self.get_instantiation_by_index(*idx);
                let struct_identifier = self.get_identifier_by_idx(*struct_identifier_id);
                let mut result =
                    format!("{}::{}<", struct_identifier.module, struct_identifier.name);
                for ty in ty_args {
                    result = format!("{}{}, ", result, self.format_type(ty));
                }
                result += ">";
                result
            },
        }
    }
}

#[test]
fn size_of() {
    println!("{:?}", std::mem::size_of::<Type>());
}
