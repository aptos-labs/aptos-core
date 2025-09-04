// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module contains some advanced algorithms for type based analysis. This is currently
//! used by the Move Prover.

use crate::ty::{
    NoUnificationContext, Substitution, Type, UnificationContext, Variance, WideningOrder,
};
use move_binary_format::file_format::TypeParameterIndex;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

/// Helper to unify types which stem from different generic contexts.
///
/// Both comparison side may have type parameters (equally named as #0, #1, ...).
/// The helper converts the type parameter from or both sides into variables
/// and then performs unification of the terms. The resulting substitution
/// is converted back to parameter instantiations.
///
/// Example: consider a function `f<X>` which uses memory `M<X, u64>`, and invariant
/// `invariant<X>` which uses memory `M<bool, X>`. Using this helper to unify both
/// memories will result in instantiations which when applied create `f<bool>`
/// and `invariant<u64>` respectively.
pub struct TypeUnificationAdapter {
    type_vars_map: BTreeMap<u32, (bool, TypeParameterIndex)>,
    types_adapted_lhs: Vec<Type>,
    types_adapted_rhs: Vec<Type>,
}

impl TypeUnificationAdapter {
    /// Initialize the context for the type unifier.
    ///
    /// If `treat_lhs_type_param_as_var_after_index` is set to P,
    /// - any type parameter on the LHS with index < P will be treated as concrete types and
    /// - only type parameters on the LHS with index >= P are treated as variables and thus,
    ///   participate in the type unification process.
    /// The same rule applies to the RHS parameters via `treat_rhs_type_param_as_var_after_index`.
    fn new<'a, I>(
        lhs_types: I,
        rhs_types: I,
        treat_lhs_type_param_as_var_after_index: Option<TypeParameterIndex>,
        treat_rhs_type_param_as_var_after_index: Option<TypeParameterIndex>,
    ) -> Self
    where
        I: Iterator<Item = &'a Type> + Clone,
    {
        debug_assert!(
            treat_lhs_type_param_as_var_after_index.is_some()
                || treat_rhs_type_param_as_var_after_index.is_some(),
            "At least one side of the unification must be treated as variable"
        );

        // Check the input types do not contain type variables.
        debug_assert!(
            lhs_types.clone().chain(rhs_types.clone()).all(|ty| {
                let mut b = true;
                ty.visit(&mut |t| b = b && !matches!(t, Type::Var(_)));
                b
            }),
            "unexpected type variable"
        );

        // Compute the number of type parameters for each side.
        let mut lhs_type_param_count = 0;
        let mut rhs_type_param_count = 0;
        let count_type_param = |t: &Type, current: &mut u16| {
            if let Type::TypeParameter(idx) = t {
                *current = (*current).max(*idx + 1);
            }
        };
        for ty in lhs_types.clone() {
            ty.visit(&mut |t| count_type_param(t, &mut lhs_type_param_count));
        }
        for ty in rhs_types.clone() {
            ty.visit(&mut |t| count_type_param(t, &mut rhs_type_param_count));
        }

        // Create a type variable instantiation for each side.
        let mut var_count = 0;
        let mut type_vars_map = BTreeMap::new();
        let lhs_inst = match treat_lhs_type_param_as_var_after_index {
            None => vec![],
            Some(boundary) => (0..boundary)
                .map(Type::TypeParameter)
                .chain((boundary..lhs_type_param_count).map(|i| {
                    let idx = var_count;
                    var_count += 1;
                    type_vars_map.insert(idx, (true, i));
                    Type::Var(idx)
                }))
                .collect(),
        };
        let rhs_inst = match treat_rhs_type_param_as_var_after_index {
            None => vec![],
            Some(boundary) => (0..boundary)
                .map(Type::TypeParameter)
                .chain((boundary..rhs_type_param_count).map(|i| {
                    let idx = var_count;
                    var_count += 1;
                    type_vars_map.insert(idx, (false, i));
                    Type::Var(idx)
                }))
                .collect(),
        };

        // Do the adaptation.
        let types_adapted_lhs = lhs_types.map(|t| t.instantiate(&lhs_inst)).collect();
        let types_adapted_rhs = rhs_types.map(|t| t.instantiate(&rhs_inst)).collect();

        Self {
            type_vars_map,
            types_adapted_lhs,
            types_adapted_rhs,
        }
    }

    /// Create a TypeUnificationAdapter with the goal of unifying a pair of types.
    ///
    /// If `treat_lhs_type_param_as_var` is True, treat all type parameters on the LHS as variables.
    /// If `treat_rhs_type_param_as_var` is True, treat all type parameters on the RHS as variables.
    pub fn new_pair(
        lhs_type: &Type,
        rhs_type: &Type,
        treat_lhs_type_param_as_var: bool,
        treat_rhs_type_param_as_var: bool,
    ) -> Self {
        Self::new(
            std::iter::once(lhs_type),
            std::iter::once(rhs_type),
            treat_lhs_type_param_as_var.then_some(0),
            treat_rhs_type_param_as_var.then_some(0),
        )
    }

    /// Create a TypeUnificationAdapter with the goal of unifying a pair of type tuples.
    ///
    /// If `treat_lhs_type_param_as_var` is True, treat all type parameters on the LHS as variables.
    /// If `treat_rhs_type_param_as_var` is True, treat all type parameters on the RHS as variables.
    pub fn new_vec(
        lhs_types: &[Type],
        rhs_types: &[Type],
        treat_lhs_type_param_as_var: bool,
        treat_rhs_type_param_as_var: bool,
    ) -> Self {
        Self::new(
            lhs_types.iter(),
            rhs_types.iter(),
            treat_lhs_type_param_as_var.then_some(0),
            treat_rhs_type_param_as_var.then_some(0),
        )
    }

    /// Consume the TypeUnificationAdapter and produce the unification result. If type unification
    /// is successful, return a pair of instantiations for type parameters on each side which
    /// unify the LHS and RHS respectively. If the LHS and RHS cannot unify, None is returned.
    pub fn unify(
        self,
        context: &mut impl UnificationContext,
        variance: Variance,
        shallow_subst: bool,
    ) -> Option<(BTreeMap<u16, Type>, BTreeMap<u16, Type>)> {
        let mut subst = Substitution::new();
        match subst.unify_vec(
            context,
            variance,
            WideningOrder::LeftToRight,
            None,
            &self.types_adapted_lhs,
            &self.types_adapted_rhs,
        ) {
            Ok(_) => {
                let mut inst_lhs = BTreeMap::new();
                let mut inst_rhs = BTreeMap::new();
                for (var_idx, (is_lhs, param_idx)) in &self.type_vars_map {
                    let subst_ty = match subst.get_substitution(*var_idx, shallow_subst) {
                        None => continue,
                        Some(Type::Var(subst_var_idx)) => {
                            match self.type_vars_map.get(&subst_var_idx) {
                                None => {
                                    // If the original types do not contain free type
                                    // variables, this should not happen.
                                    panic!("unexpected type variable");
                                },
                                Some((_, subs_param_idx)) => {
                                    // There can be either lhs or rhs type parameters left, but
                                    // not both sides, so it is unambiguous to just return it here.
                                    Type::TypeParameter(*subs_param_idx)
                                },
                            }
                        },
                        Some(subst_ty) => subst_ty.clone(),
                    };
                    let inst = if *is_lhs {
                        &mut inst_lhs
                    } else {
                        &mut inst_rhs
                    };
                    inst.insert(*param_idx, subst_ty);
                }

                Some((inst_lhs, inst_rhs))
            },
            Err(_) => None,
        }
    }
}

/// A helper to derive the set of instantiations for type parameters
pub struct TypeInstantiationDerivation {}

impl TypeInstantiationDerivation {
    /// Find what the instantiations should we have for the type parameter at `target_param_index`.
    ///
    /// The invariant is, forall type parameters whose index < target_param_index, it should either
    /// - be assigned with a concrete type already and hence, ceases to be a type parameter, or
    /// - does not have any matching instantiation and hence, either remains a type parameter or is
    ///   represented as a type error.
    /// But in anyway, these type parameters no longer participate in type unification anymore.
    ///
    /// If `target_lhs` is True, derive instantiations for the type parameter with
    /// `target_param_index` on the `lhs_types`. Otherwise, target the `rhs_types`.
    fn derive_instantiations_for_target_parameter(
        lhs_types: &BTreeSet<Type>,
        rhs_types: &BTreeSet<Type>,
        treat_lhs_type_param_as_var: bool,
        treat_rhs_type_param_as_var: bool,
        target_param_index: TypeParameterIndex,
        target_lhs: bool,
    ) -> BTreeSet<Type> {
        // progressively increase the boundary
        let treat_lhs_type_param_as_var_after_index =
            treat_lhs_type_param_as_var.then_some(if target_lhs { target_param_index } else { 0 });
        let treat_rhs_type_param_as_var_after_index =
            treat_rhs_type_param_as_var.then_some(if target_lhs { 0 } else { target_param_index });

        let mut target_param_insts = BTreeSet::new();
        for t_lhs in lhs_types {
            for t_rhs in rhs_types {
                // Try to unify the instantiations
                let adapter = TypeUnificationAdapter::new(
                    std::iter::once(t_lhs),
                    std::iter::once(t_rhs),
                    treat_lhs_type_param_as_var_after_index,
                    treat_rhs_type_param_as_var_after_index,
                );
                let rel = adapter.unify(&mut NoUnificationContext, Variance::SpecVariance, false);
                if let Some((subst_lhs, subst_rhs)) = rel {
                    let subst = if target_lhs { subst_lhs } else { subst_rhs };
                    for (param_idx, inst_ty) in subst.into_iter() {
                        if param_idx != target_param_index {
                            // this parameter will be unified at a later stage.
                            //
                            // NOTE: this code is inefficient when we have multiple type parameters,
                            // but a vast majority of Move code we see so far have at most one type
                            // parameter, so we trade-off efficiency with simplicity in code.
                            assert!(param_idx > target_param_index);
                            continue;
                        }
                        target_param_insts.insert(inst_ty);
                    }
                }
            }
        }
        target_param_insts
    }

    /// Find the set of valid instantiation combinations for all the type parameters.
    ///
    /// The algorithm is progressive. For a list of parameters with arity `params_arity = N`, it
    /// - first finds all possible instantiation for parameter at index 0 (`inst_param_0`) and,'
    /// - for each instantiation in `inst_param_0`,
    ///   - refines LHS or RHS types and
    ///   - finds all possible instantiations for parameter at index 1 (`inst_param_1`)
    ///   - for each instantiation in `inst_param_1`,
    ///     - refines LHS or RHS types and
    ///     - finds all possible instantiations for parameter at index 2 (`inst_param_2`)
    ///     - for each instantiation in `inst_param_2`,
    ///       - ......
    /// The process continues until all type parameters are analyzed (i.e., reaching the type
    /// parameter at index `N`).
    ///
    /// If `refine_lhs` is True, refine the `lhs_types` after each round; same for `refine_rhs`.
    ///
    /// If `target_lhs` is True, find instantiations for the type parameters in the `lhs_types`,
    /// otherwise, target the `rhs_types`.
    ///
    /// If `mark_irrelevant_param_as_error` is True, type parameters that do not have any valid
    /// instantiation will be marked as `Type::Error`. Otherwise, leave the type parameter as it is.
    pub fn progressive_instantiation<'a, I>(
        lhs_types: I,
        rhs_types: I,
        treat_lhs_type_param_as_var: bool,
        treat_rhs_type_param_as_var: bool,
        refine_lhs: bool,
        refine_rhs: bool,
        params_arity: usize,
        target_lhs: bool,
        mark_irrelevant_param_as_error: bool,
    ) -> BTreeSet<Vec<Type>>
    where
        I: Iterator<Item = &'a Type> + Clone,
    {
        let initial_param_insts: Vec<_> = (0..params_arity).map(Type::new_param).collect();

        let mut work_queue = VecDeque::new();
        work_queue.push_back(initial_param_insts);
        for target_param_index in 0..params_arity {
            let mut for_next_round = vec![];
            while let Some(param_insts) = work_queue.pop_front() {
                // refine the memory usage sets with current param instantiations
                let refined_lhs = lhs_types
                    .clone()
                    .map(|t| {
                        if refine_lhs {
                            t.instantiate(&param_insts)
                        } else {
                            t.clone()
                        }
                    })
                    .collect();
                let refined_rhs = rhs_types
                    .clone()
                    .map(|t| {
                        if refine_rhs {
                            t.instantiate(&param_insts)
                        } else {
                            t.clone()
                        }
                    })
                    .collect();

                // find type instantiations for the target parameter index
                let mut target_param_insts = Self::derive_instantiations_for_target_parameter(
                    &refined_lhs,
                    &refined_rhs,
                    treat_lhs_type_param_as_var,
                    treat_rhs_type_param_as_var,
                    target_param_index as TypeParameterIndex,
                    target_lhs,
                );

                // decide what to do with an irrelevant type parameter
                if target_param_insts.is_empty() {
                    let irrelevant_type = if mark_irrelevant_param_as_error {
                        Type::Error
                    } else {
                        Type::new_param(target_param_index)
                    };
                    target_param_insts.insert(irrelevant_type);
                }

                // instantiate the target type parameter in every possible way
                for inst in target_param_insts {
                    let mut next_insts = param_insts.clone();
                    next_insts[target_param_index] = inst;
                    for_next_round.push(next_insts);
                }
            }
            work_queue.extend(for_next_round);
        }

        // the final workqueue contains possible instantiations for all type parameters
        work_queue.into_iter().collect()
    }
}
