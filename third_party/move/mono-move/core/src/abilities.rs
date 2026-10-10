// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Ability computation over interned types.
//!
//! A [`Type::Nominal`] carries no abilities, so they are looked up.
//!
//! Resolve through the *asking* module's own declaration, which may carry fewer
//! abilities and fewer phantom markers than the definition. Only that
//! declaration governs what the module may do with the type, and it can narrow
//! the definition's set but never widen it.
//!
//! TODO(correctness): this relies on the declaration being compatible with the
//! definition. Revisit whether that reliance is acceptable, and whether
//! abilities should also be confirmed against the definition somewhere it is
//! already resolved.
//!
//! Note: this is an independent implementation, used in translation validation.
//! So, it should not reuse existing bytecode verifier implementations.

use crate::{
    interner::{InternedIdentifier, InternedModuleId},
    types::{view_type, view_type_list, InternedType, Type},
    ExecutionErrorKind, IntoExecutionError, PreparedModule,
};
use move_binary_format::file_format::StructHandle;
use move_core_types::ability::AbilitySet;
use shared_dsa::UnorderedMap;
use thiserror::Error;

/// Computes abilities of interned types under a fixed type-parameter scope.
///
/// `lookup` resolves a nominal to the [`StructHandle`] declaring it, which
/// carries the abilities and phantom markers this computation needs.
///
/// The memo is keyed on the interned type alone. That is sound only because
/// `ty_param_ctx` and `lookup` are fixed for the calculator's lifetime: both
/// change the abilities of the same type. Construct a new calculator whenever
/// either changes.
pub struct AbilityCalculator<'a, L> {
    lookup: L,
    /// Constraints of the enclosing scope's type parameters, indexed by
    /// [`Type::TypeParam`]'s `idx`.
    ty_param_ctx: &'a [AbilitySet],
    memo: UnorderedMap<InternedType, AbilitySet>,
}

impl<'a, L> AbilityCalculator<'a, L>
where
    L: FnMut(InternedModuleId, InternedIdentifier) -> Result<&'a StructHandle, AbilityError>,
{
    pub fn new(lookup: L, ty_param_ctx: &'a [AbilitySet]) -> Self {
        Self {
            lookup,
            ty_param_ctx,
            memo: UnorderedMap::new(),
        }
    }

    /// Abilities of `ty` in this calculator's scope.
    ///
    /// Inherits safety contract of [`view_type`].
    ///
    /// TODO(metering): the memo bounds repeated work but not stack depth, which
    /// is still linear in type nesting. Same family as the `TODO(metering)` on
    /// `is_closed_type`.
    pub fn abilities_of(&mut self, ty: InternedType) -> Result<AbilitySet, AbilityError> {
        if let Some(cached) = self.memo.get(&ty) {
            return Ok(*cached);
        }
        let abilities = self.compute(ty)?;
        self.memo.insert(ty, abilities);
        Ok(abilities)
    }

    fn compute(&mut self, ty: InternedType) -> Result<AbilitySet, AbilityError> {
        Ok(match view_type(ty) {
            Type::Bool
            | Type::U8
            | Type::U16
            | Type::U32
            | Type::U64
            | Type::U128
            | Type::U256
            | Type::I8
            | Type::I16
            | Type::I32
            | Type::I64
            | Type::I128
            | Type::I256
            | Type::Address => AbilitySet::PRIMITIVES,

            Type::Signer => AbilitySet::SIGNER,

            Type::ImmutRef { .. } | Type::MutRef { .. } => AbilitySet::REFERENCES,

            Type::Function { abilities, .. } => *abilities,

            Type::TypeParam { idx } => {
                *self
                    .ty_param_ctx
                    .get(*idx as usize)
                    .ok_or(AbilityError::TypeParamOutOfRange {
                        idx: *idx,
                        num_params: self.ty_param_ctx.len(),
                    })?
            },

            // A vector is copy/drop/store exactly when its element is; the
            // element is stored inside it, so it is never phantom.
            Type::Vector { elem } => {
                let elem_abilities = self.abilities_of(*elem)?;
                instantiated_abilities(AbilitySet::VECTOR, [(false, elem_abilities)])
            },

            Type::Nominal {
                module_id,
                name,
                ty_args,
            } => {
                let handle = (self.lookup)(*module_id, *name)?;
                let ty_args = view_type_list(*ty_args);
                if handle.type_parameters.len() != ty_args.len() {
                    return Err(AbilityError::TypeArgumentArityMismatch {
                        declared: handle.type_parameters.len(),
                        actual: ty_args.len(),
                    });
                }
                let mut arg_abilities = Vec::with_capacity(ty_args.len());
                for ty_arg in ty_args {
                    arg_abilities.push(self.abilities_of(*ty_arg)?);
                }
                instantiated_abilities(
                    handle.abilities,
                    handle
                        .type_parameters
                        .iter()
                        .map(|param| param.is_phantom)
                        .zip(arg_abilities),
                )
            },
        })
    }
}

/// Abilities of a polymorphic type once instantiated.
///
/// Each declared ability imposes a requirement on every non-phantom argument:
/// copy, drop, and store require themselves; key requires store. The ability is
/// kept only if every such argument meets it. Phantom arguments are not part of
/// the value, so they impose nothing.
///
/// Callers must pass one argument per declared type parameter.
fn instantiated_abilities(
    declared: AbilitySet,
    type_arguments: impl IntoIterator<Item = (bool, AbilitySet)>,
) -> AbilitySet {
    let mut result = declared;
    for (is_phantom, argument) in type_arguments {
        if is_phantom {
            continue;
        }
        for ability in declared.iter() {
            if !argument.has_ability(ability.requires()) {
                result = result.remove(ability);
            }
        }
    }
    result
}

/// Builds a lookup closure backed by one [`PreparedModule`]'s handle table.
///
/// The table holds every nominal the module can name, so a miss is
/// [`AbilityError::UnknownNominal`].
pub fn module_nominal_lookup<'m>(
    module: &'m PreparedModule,
) -> impl FnMut(InternedModuleId, InternedIdentifier) -> Result<&'m StructHandle, AbilityError> + 'm
{
    |module_id, name| {
        module
            .nominal_handle(module_id, name)
            .ok_or(AbilityError::UnknownNominal)
    }
}

#[derive(Debug, Error)]
pub enum AbilityError {
    #[error("no ability information for nominal type in scope")]
    UnknownNominal,

    #[error("type parameter {idx} out of range: {num_params} parameter(s) in scope")]
    TypeParamOutOfRange { idx: u16, num_params: usize },

    #[error("nominal declares {declared} type parameter(s) but the type carries {actual}")]
    TypeArgumentArityMismatch { declared: usize, actual: usize },
}

impl IntoExecutionError for AbilityError {
    fn kind(&self) -> ExecutionErrorKind {
        use AbilityError::*;
        match self {
            // Each is established before translation: nominal resolution,
            // type-parameter scoping, and instantiation arity.
            UnknownNominal | TypeParamOutOfRange { .. } | TypeArgumentArityMismatch { .. } => {
                ExecutionErrorKind::InvariantViolation
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use move_core_types::ability::Ability;

    /// Every `AbilitySet`: the domain is 4 bits, so "exhaustive" is 16 values.
    fn all_ability_sets() -> Vec<AbilitySet> {
        (0u8..16)
            .map(|bits| AbilitySet::from_u8(bits).expect("4-bit set"))
            .collect()
    }

    /// Pins the derivation to `polymorphic_abilities` over the entire input
    /// domain for arities 0..=2: all 16 declared sets against every
    /// `(phantom, argument)` pair drawn from 2 x 16.
    #[test]
    fn agrees_with_the_verifier_on_every_input() {
        let sets = all_ability_sets();
        let mut checked = 0usize;

        for &declared in &sets {
            // Arity 0.
            assert_eq!(
                instantiated_abilities(declared, []),
                AbilitySet::polymorphic_abilities(declared, [], []).expect("arity 0"),
            );
            checked += 1;

            for &phantom_a in &[false, true] {
                for &arg_a in &sets {
                    assert_eq!(
                        instantiated_abilities(declared, [(phantom_a, arg_a)]),
                        AbilitySet::polymorphic_abilities(declared, [phantom_a], [arg_a])
                            .expect("arity 1"),
                        "declared={declared:?} phantom={phantom_a} arg={arg_a:?}",
                    );
                    checked += 1;

                    for &phantom_b in &[false, true] {
                        for &arg_b in &sets {
                            assert_eq!(
                                instantiated_abilities(declared, [
                                    (phantom_a, arg_a),
                                    (phantom_b, arg_b)
                                ]),
                                AbilitySet::polymorphic_abilities(
                                    declared,
                                    [phantom_a, phantom_b],
                                    [arg_a, arg_b]
                                )
                                .expect("arity 2"),
                                "declared={declared:?} args=[{arg_a:?}, {arg_b:?}]",
                            );
                            checked += 1;
                        }
                    }
                }
            }
        }
        // 16 * (1 + 32 * (1 + 32)) = 16 * 1057
        assert_eq!(checked, 16 * (1 + 32 * (1 + 32)));
    }

    #[test]
    fn key_is_predicated_on_store_not_on_key() {
        // The one asymmetric rule: `a.requires()` is `a` for copy/drop/store
        // but `store` for key, so a `key` struct needs `store` arguments.
        let key_only = AbilitySet::EMPTY | Ability::Key;
        let store_only = AbilitySet::EMPTY | Ability::Store;

        assert_eq!(
            instantiated_abilities(key_only, [(false, store_only)]),
            key_only,
            "a store-only argument keeps key"
        );
        assert_eq!(
            instantiated_abilities(key_only, [(false, key_only)]),
            AbilitySet::EMPTY,
            "a key-only argument does not supply store, so key is stripped"
        );
    }

    #[test]
    fn phantom_arguments_never_predicate() {
        // A phantom parameter holds no runtime value, so even the empty set
        // takes nothing away.
        for &declared in &all_ability_sets() {
            assert_eq!(
                instantiated_abilities(declared, [(true, AbilitySet::EMPTY)]),
                declared,
            );
        }
    }

    #[test]
    fn abilities_are_only_ever_removed() {
        // Instantiation is monotone downward: an argument can strip a declared
        // ability but never add one.
        let sets = all_ability_sets();
        for &declared in &sets {
            for &arg in &sets {
                let result = instantiated_abilities(declared, [(false, arg)]);
                assert!(
                    result.is_subset(declared),
                    "{result:?} not subset {declared:?}"
                );
            }
        }
    }
}
