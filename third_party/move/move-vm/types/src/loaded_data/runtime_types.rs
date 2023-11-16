// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use derivative::Derivative;
use move_binary_format::{
    errors::{PartialVMError, PartialVMResult},
    file_format::{
        AbilitySet, SignatureToken, StructHandle, StructTypeParameter, TypeParameterIndex,
    },
};
use move_core_types::{identifier::Identifier, language_storage::ModuleId, vm_status::StatusCode};
use smallbitvec::SmallBitVec;
use std::{cmp::max, collections::BTreeMap, fmt::Debug, sync::Arc};

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
    Vector(Arc<Type>),
    Struct {
        idx: StructNameIndex,
        ability: AbilityInfo,
    },
    StructInstantiation {
        idx: StructNameIndex,
        ty_args: Arc<Vec<Type>>,
        ability: AbilityInfo,
    },
    Reference(Box<Type>),
    MutableReference(Box<Type>),
    Instantiated(Box<Type>, usize),
    TyParam(u16),
    U16,
    U32,
    U256,
}

#[derive(Debug, Clone)]
pub struct TypeStack {
    stack: Vec<Arc<Vec<Type>>>,
    current_top: Vec<usize>,
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
    fn clone_impl(&self, depth: usize) -> PartialVMResult<Type> {
        self.apply_subst(|idx, _| Ok(Type::TyParam(idx)), depth)
    }

    fn apply_subst<F>(&self, subst: F, depth: usize) -> PartialVMResult<Type>
    where
        F: Fn(u16, usize) -> PartialVMResult<Type> + Copy,
    {
        if depth > TYPE_DEPTH_MAX {
            return Err(PartialVMError::new(StatusCode::VM_MAX_TYPE_DEPTH_REACHED));
        }
        let res = match self {
            Type::TyParam(idx) => subst(*idx, depth)?,
            Type::Bool => Type::Bool,
            Type::U8 => Type::U8,
            Type::U16 => Type::U16,
            Type::U32 => Type::U32,
            Type::U64 => Type::U64,
            Type::U128 => Type::U128,
            Type::U256 => Type::U256,
            Type::Address => Type::Address,
            Type::Signer => Type::Signer,
            Type::Vector(ty) => Type::Vector(Arc::new(ty.apply_subst(subst, depth + 1)?)),
            Type::Reference(ty) => Type::Reference(Box::new(ty.apply_subst(subst, depth + 1)?)),
            Type::MutableReference(ty) => {
                Type::MutableReference(Box::new(ty.apply_subst(subst, depth + 1)?))
            },
            Type::Instantiated(_, _) => return Err(PartialVMError::new(StatusCode::INTERNAL_TYPE_ERROR).with_message("Substitution can only applied to canonicalized type".to_string())),
            Type::Struct { idx, ability } => Type::Struct {
                idx: *idx,
                ability: ability.clone(),
            },
            Type::StructInstantiation {
                idx,
                ty_args: instantiation,
                ability,
            } => {
                let mut inst = vec![];
                for ty in instantiation.iter() {
                    inst.push(ty.apply_subst(subst, depth + 1)?)
                }
                Type::StructInstantiation {
                    idx: *idx,
                    ty_args: Arc::new(inst),
                    ability: ability.clone(),
                }
            },
        };
        Ok(res)
    }

    pub fn subst(&self, ty_args: &[Type]) -> PartialVMResult<Type> {
        self.apply_subst(
            |idx, depth| match ty_args.get(idx as usize) {
                Some(ty) => ty.clone_impl(depth),
                None => Err(
                    PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                        .with_message(format!(
                            "type substitution failed: index out of bounds -- len {} got {}",
                            ty_args.len(),
                            idx
                        )),
                ),
            },
            1,
        )
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

            Type::Instantiated(_, _) => Err(PartialVMError::new(StatusCode::UNREACHABLE).with_message(
                "Abilities function can only be invoked on canonicalized types".to_string(),
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

    pub fn from_const_signature(constant_signature: &SignatureToken) -> PartialVMResult<Self> {
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
            S::Vector(inner) => L::Vector(Arc::new(Self::from_const_signature(inner)?)),
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
}

impl TypeStack {
    pub fn new(inital_ty_args: Arc<Vec<Type>>) -> Self {
        Self {
            stack: vec![inital_ty_args],
            current_top: vec![0],
        }
    }

    pub fn push(&mut self, tys: Arc<Vec<Type>>) {
        self.stack.push(tys);
        self.current_top.push(self.stack.len() - 1);
    }

    pub fn push_instantiation(&mut self, tys: Arc<Vec<Type>>) -> usize {
        self.stack.push(tys);
        self.stack.len() - 1
    }

    pub fn pop(&mut self) {
        self.current_top.pop();
    }

    pub fn instantiate_type_at(&self, ty: &Type, depth: usize) -> Type {
        match ty {
            Type::Bool
            | Type::U8
            | Type::U16
            | Type::U32
            | Type::U64
            | Type::U128
            | Type::U256
            | Type::Address
            | Type::Signer
            | Type::Struct { .. } => ty.clone(),
            Type::TyParam(idx) => Type::Instantiated(Box::new(Type::TyParam(*idx)), depth),
            Type::Reference(ty) => Type::Reference(Box::new(self.instantiate_type_at(ty, depth))),
            Type::MutableReference(ty) => {
                Type::MutableReference(Box::new(self.instantiate_type_at(ty, depth)))
            },
            Type::Instantiated(_, _) | Type::StructInstantiation { .. } | Type::Vector(_)=> {
                Type::Instantiated(Box::new(ty.clone()), depth)
            },
        }
    }

    pub fn instantiate_type(&self, ty: &Type) -> Type {
        self.instantiate_type_at(ty, self.current_frame())
    }

    fn current_frame(&self) -> usize {
        *self.current_top.last().unwrap()
    }

    pub fn ty_eq(&self, lhs: &Type, rhs: &Type) -> bool {
        self.check_ty_eq_impl(lhs, self.current_frame(), rhs, self.current_frame())
    }

    fn check_ty_eq_impl(
        &self,
        lhs: &Type,
        lhs_context_depth: usize,
        rhs: &Type,
        rhs_context_depth: usize,
    ) -> bool {
        match (lhs, rhs) {
            (Type::Bool, Type::Bool)
            | (Type::U8, Type::U8)
            | (Type::U16, Type::U16)
            | (Type::U32, Type::U32)
            | (Type::U64, Type::U64)
            | (Type::U128, Type::U128)
            | (Type::U256, Type::U256)
            | (Type::Address, Type::Address)
            | (Type::Signer, Type::Signer) => true,
            (Type::Reference(lhs), Type::Reference(rhs)) => {
                self.check_ty_eq_impl(lhs, lhs_context_depth, rhs, rhs_context_depth)
            },
            (Type::MutableReference(lhs), Type::MutableReference(rhs)) => {
                self.check_ty_eq_impl(lhs, lhs_context_depth, rhs, rhs_context_depth)
            },
            (Type::Vector(lhs), Type::Vector(rhs)) => {
                self.check_ty_eq_impl(lhs, lhs_context_depth, rhs, rhs_context_depth)
            },
            (Type::Instantiated(lhs, lhs_context_depth), rhs) => {
                self.check_ty_eq_impl(lhs, *lhs_context_depth, rhs, rhs_context_depth)
            },
            (lhs, Type::Instantiated(rhs, rhs_context_depth)) => {
                self.check_ty_eq_impl(lhs, lhs_context_depth, rhs, *rhs_context_depth)
            },
            (Type::Struct { idx: lhs_idx, .. }, Type::Struct { idx: rhs_idx, .. }) => {
                lhs_idx == rhs_idx
            },
            (Type::TyParam(idx), rhs) => self.check_ty_eq_impl(
                &self.stack[lhs_context_depth][*idx as usize],
                lhs_context_depth,
                rhs,
                rhs_context_depth,
            ),
            (lhs, Type::TyParam(idx)) => self.check_ty_eq_impl(
                lhs,
                lhs_context_depth,
                &self.stack[rhs_context_depth][*idx as usize],
                rhs_context_depth,
            ),
            (
                Type::StructInstantiation {
                    idx: lhs_idx,
                    ty_args: lhs_args,
                    ..
                },
                Type::StructInstantiation {
                    idx: rhs_idx,
                    ty_args: rhs_args,
                    ..
                },
            ) => {
                lhs_idx == rhs_idx
                    && lhs_args.len() == rhs_args.len()
                    && lhs_args.iter().zip(rhs_args.iter()).all(|(lhs, rhs)| {
                        self.check_ty_eq_impl(lhs, lhs_context_depth, rhs, rhs_context_depth)
                    })
            },
            _ => false,
        }
    }

    pub fn abilities(&self, ty: &Type) -> PartialVMResult<AbilitySet> {
        self.abilities_impl(ty, self.current_frame())
    }

    pub fn canonicalize(&self, ty: &Type) -> Type {
        self.canonicalize_impl(ty, self.current_frame())
    }

    fn canonicalize_impl(&self, ty: &Type, context_depth: usize) -> Type {
        match ty {
            Type::Bool
            | Type::U8
            | Type::U16
            | Type::U32
            | Type::U64
            | Type::U128
            | Type::U256
            | Type::Address
            | Type::Signer
            | Type::Struct { .. } => ty.clone(),
            Type::Reference(ty) => {
                Type::Reference(Box::new(self.canonicalize_impl(ty, context_depth)))
            },
            Type::MutableReference(ty) => {
                Type::MutableReference(Box::new(self.canonicalize_impl(ty, context_depth)))
            },
            Type::Vector(ty) => Type::Vector(Arc::new(self.canonicalize_impl(ty, context_depth))),
            Type::Instantiated(ty, idx) => self.canonicalize_impl(ty, *idx),
            Type::TyParam(idx) => self.canonicalize_impl(&self.stack[context_depth][*idx as usize].clone(), context_depth),
            Type::StructInstantiation {
                idx,
                ty_args,
                ability,
            } => Type::StructInstantiation {
                idx: *idx,
                ty_args: Arc::new(
                    ty_args
                        .iter()
                        .map(|ty| self.canonicalize_impl(ty, context_depth))
                        .collect(),
                ),
                ability: ability.clone(),
            },
        }
    }

    fn abilities_impl(&self, ty: &Type, context_depth: usize) -> PartialVMResult<AbilitySet> {
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
            Type::TyParam(i) => {
                self.abilities_impl(&self.stack[context_depth][*i as usize], context_depth)
            },
            Type::Instantiated(ty, depth) => self.abilities_impl(ty, *depth),
            Type::Vector(ty) => {
                AbilitySet::polymorphic_abilities(AbilitySet::VECTOR, vec![false], vec![
                    self.abilities_impl(ty, context_depth)?
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
                    .map(|arg| self.abilities_impl(arg, context_depth))
                    .collect::<PartialVMResult<Vec<_>>>()?;
                AbilitySet::polymorphic_abilities(
                    *base_ability_set,
                    phantom_ty_args_mask.iter(),
                    type_argument_abilities,
                )
            },
        }
    }

    pub fn check_vec_ref(
        &self,
        lhs: &Type,
        inner_ty: Type,
        is_mut: bool,
    ) -> PartialVMResult<Type> {
        match lhs {
            Type::MutableReference(inner) => {
                self.check_eq(inner, &Type::Vector(Arc::new(inner_ty.clone())))?;
                Ok(inner_ty)
            },
            Type::Reference(inner) if !is_mut => {
                self.check_eq(inner, &Type::Vector(Arc::new(inner_ty.clone())))?;
                Ok(inner_ty)
            },
            _ => Err(
                PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                    .with_message("VecMutBorrow expects a vector reference".to_string())
                    .with_sub_status(move_core_types::vm_status::sub_status::unknown_invariant_violation::EPARANOID_FAILURE),
            ),
        }
    }

    pub fn check_eq(&self, lhs: &Type, rhs: &Type) -> PartialVMResult<()> {
        if !self.ty_eq(lhs, rhs) {
            return Err(
                PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                    .with_message(format!(
                        "Type mismatch: expected {:?}, got {:?}",
                        lhs, rhs
                    ))
                    .with_sub_status(move_core_types::vm_status::sub_status::unknown_invariant_violation::EPARANOID_FAILURE),
            );
        }
        Ok(())
    }

    pub fn check_ref_eq(&self, lhs: &Type, expected_inner: &Type) -> PartialVMResult<()> {
        match lhs {
            Type::MutableReference(inner) | Type::Reference(inner) => {
                self.check_eq(inner, expected_inner)
            },
            _ => Err(
                PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                    .with_message("VecMutBorrow expects a vector reference".to_string()),
            ),
        }
    }

    pub fn count_type_nodes(&self, ty: &Type) -> u64 {
        let mut todo = vec![(ty, self.current_frame())];
        let mut result = 0;
        while let Some((ty, context_depth)) = todo.pop() {
            match ty {
                Type::Vector(ty)  => {
                    result += 1;
                    todo.push((ty, context_depth));
                },
                Type::Reference(ty) | Type::MutableReference(ty) => {
                    result += 1;
                    todo.push((ty, context_depth));
                },
                Type::StructInstantiation { ty_args, .. } => {
                    result += 1;
                    todo.extend(ty_args.iter().map(|ty| (ty, context_depth)));
                },
                Type::Instantiated(ty, idx) =>  todo.push((ty, *idx)),
                Type::TyParam(idx) => todo.push((&self.stack[context_depth][*idx as usize], context_depth)),
                _ => {
                    result += 1;
                },
            }
        }
        result
    }
}
