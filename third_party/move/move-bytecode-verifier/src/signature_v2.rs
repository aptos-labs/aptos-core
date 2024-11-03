// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::VerifierConfig;
use move_binary_format::{
    binary_views::BinaryIndexedView,
    errors::{Location, PartialVMError, PartialVMResult, VMResult},
    file_format::{
        Ability, AbilitySet, Bytecode, CodeUnit, CompiledModule, CompiledScript, FieldDefinition,
        FieldInstantiationIndex, FunctionDefinition, FunctionHandle, FunctionHandleIndex,
        FunctionInstantiationIndex, SignatureIndex, SignatureToken, StructDefInstantiationIndex,
        StructDefinition, StructDefinitionIndex, StructFieldInformation, StructHandle,
        StructTypeParameter, StructVariantInstantiationIndex, TypeParameterIndex,
        VariantFieldInstantiationIndex,
    },
    IndexKind,
};
use move_core_types::vm_status::StatusCode;
use std::{
    cell::RefCell,
    collections::{btree_map, BTreeMap},
    fmt::Debug,
};
use typed_arena::Arena;

const NUM_ABILITIES: usize = 4;
const NUM_PARAMS_PER_WORD: usize = 64 / NUM_ABILITIES;

#[derive(Debug)]
struct BitsetTypeParameterConstraints<const N: usize> {
    words: [u64; N],
}

impl<const N: usize> FromIterator<(TypeParameterIndex, AbilitySet)>
    for BitsetTypeParameterConstraints<N>
{
    fn from_iter<T: IntoIterator<Item = (TypeParameterIndex, AbilitySet)>>(iter: T) -> Self {
        let mut constraints = Self::new();
        for (idx, abilities) in iter.into_iter() {
            constraints.insert(idx, abilities);
        }
        constraints
    }
}

impl<'a, const N: usize> From<&'a [AbilitySet]> for BitsetTypeParameterConstraints<N> {
    fn from(abilities: &'a [AbilitySet]) -> Self {
        abilities
            .iter()
            .enumerate()
            .map(|(idx, abilities)| (idx as TypeParameterIndex, *abilities))
            .collect()
    }
}

/// This defines a generic collection of type parameter constraints, which is used
/// by the signature checking algorithm.
///
/// The Bitset-based implementation allow for super-fast merge and subset operations.
impl<const N: usize> BitsetTypeParameterConstraints<N> {
    /// Creates an empty set of type parameter constraints.
    fn new() -> Self {
        Self { words: [0; N] }
    }

    /// Inserts an additional constraint to the set.
    fn insert(&mut self, ty_param_idx: TypeParameterIndex, required_abilities: AbilitySet) {
        assert!(
            (ty_param_idx as usize) < N * NUM_PARAMS_PER_WORD,
            "Type parameter index out of bounds. \
             The current Bitset implementation is only configured to handle \
             {} type parameters at max.",
            N * NUM_PARAMS_PER_WORD
        );

        if required_abilities == AbilitySet::EMPTY {
            return;
        }

        let ty_param_idx = ty_param_idx as usize;

        let word_idx = ty_param_idx / NUM_PARAMS_PER_WORD;
        let offset_in_word = (ty_param_idx % NUM_PARAMS_PER_WORD) * NUM_ABILITIES;

        self.words[word_idx] |= (required_abilities.into_u8() as u64) << offset_in_word;
    }

    /// Merges the constraints in another set into this one.
    fn merge(&mut self, other: &Self) {
        for i in 0..N {
            self.words[i] |= other.words[i]
        }
    }

    /// Checks if all constraints are satisfied within a given context.
    ///
    /// The context is represented as a reference to the associated type `PreferredAbilityContext`,
    /// which the implementations can choose based on what is optimal.
    fn check_in_context(&self, context: &Self) -> PartialVMResult<()> {
        for i in 0..N {
            if self.words[i] | context.words[i] != context.words[i] {
                return Err(PartialVMError::new(StatusCode::CONSTRAINT_NOT_SATISFIED));
            }
        }

        Ok(())
    }
}

/// Checks if the given type is well formed and has the required abilities.
///
/// This function does NOT require the context (type parameter abilities) to be specified.
/// Instead, it will construct a minimal set of ability constraints each type parameter needs to satisfy,
/// which the caller can then verify in individual contexts.
/// This allows us to cache the results in the extreme case where a signature is being
/// referenced many many times.
///
/// Time Complexity: `O(size_of_type * num_of_abilities * log(num_of_ty_params))`.
/// Can be treated as `O(size_of_type)` if there are only a handful of abilities and type params.
fn check_ty<const N: usize>(
    struct_handles: &[StructHandle],
    ty: &SignatureToken,
    allow_ref: bool,
    required_abilities: AbilitySet,
    param_constraints: &mut BitsetTypeParameterConstraints<N>,
) -> PartialVMResult<()> {
    use SignatureToken::*;

    let assert_abilities = |abilities: AbilitySet, required: AbilitySet| -> PartialVMResult<()> {
        if !required.is_subset(abilities) {
            Err(
                PartialVMError::new(StatusCode::CONSTRAINT_NOT_SATISFIED).with_message(format!(
                    "expected type with abilities {:?} got type actual {:?} with incompatible \
                abilities {:?}",
                    required, ty, abilities
                )),
            )
        } else {
            Ok(())
        }
    };

    match ty {
        TypeParameter(param_idx) => param_constraints.insert(*param_idx, required_abilities),
        U8 | U16 | U32 | U64 | U128 | U256 | Bool | Address => {
            assert_abilities(AbilitySet::PRIMITIVES, required_abilities)?;
        },
        Signer => {
            assert_abilities(AbilitySet::SIGNER, required_abilities)?;
        },
        Reference(ty) | MutableReference(ty) => {
            if allow_ref {
                assert_abilities(AbilitySet::REFERENCES, required_abilities)?;
                check_ty(
                    struct_handles,
                    ty,
                    false,
                    AbilitySet::EMPTY,
                    param_constraints,
                )?;
            } else {
                return Err(PartialVMError::new(StatusCode::INVALID_SIGNATURE_TOKEN)
                    .with_message("reference not allowed".to_string()));
            }
        },
        Vector(ty) => {
            assert_abilities(AbilitySet::VECTOR, required_abilities)?;
            check_ty(
                struct_handles,
                ty,
                false,
                required_abilities.requires(),
                param_constraints,
            )?;
        },
        Function(args, result, abilities) => {
            assert_abilities(*abilities, required_abilities)?;
            for ty in args.iter().chain(result.iter()) {
                check_ty(
                    struct_handles,
                    ty,
                    false,
                    required_abilities.requires(),
                    param_constraints,
                )?;
            }
        },
        Struct(sh_idx) => {
            let handle = &struct_handles[sh_idx.0 as usize];
            assert_abilities(handle.abilities, required_abilities)?;
        },
        StructInstantiation(sh_idx, ty_args) => {
            let handle = &struct_handles[sh_idx.0 as usize];

            assert_abilities(handle.abilities, required_abilities)?;

            // TODO: is this needed?
            if handle.type_parameters.len() != ty_args.len() {
                return Err(
                    PartialVMError::new(StatusCode::NUMBER_OF_TYPE_ARGUMENTS_MISMATCH)
                        .with_message(format!(
                            "expected {} type argument(s), got {}",
                            handle.type_parameters.len(),
                            ty_args.len()
                        )),
                );
            }
            for (ty_param, ty_arg) in handle.type_parameters.iter().zip(ty_args.iter()) {
                let required_abilities = if ty_param.is_phantom {
                    ty_param.constraints
                } else {
                    ty_param.constraints.union(required_abilities.requires())
                };

                check_ty(
                    struct_handles,
                    ty_arg,
                    false,
                    required_abilities,
                    param_constraints,
                )?;
            }
        },
    }

    Ok(())
}

/// Checks if the given type is well formed and has the required abilities within a context.
///
/// Calls `check_ty` internally.
///
/// Time complexity: same as `check_ty`.
fn check_ty_in_context<const N: usize>(
    struct_handles: &[StructHandle],
    ability_context: &BitsetTypeParameterConstraints<N>,
    ty: &SignatureToken,
    allow_ref: bool,
    required_abilities: AbilitySet,
) -> PartialVMResult<()> {
    let mut param_constraints = BitsetTypeParameterConstraints::new();

    check_ty(
        struct_handles,
        ty,
        allow_ref,
        required_abilities,
        &mut param_constraints,
    )?;

    param_constraints
        .check_in_context(ability_context)
        .map_err(|err| {
            err.with_message(format!(
                "expected type with abilities {:?} got type {:?} within context {:?}",
                required_abilities, ty, ability_context,
            ))
        })
}

/// Helper function to check if a type contains phantom type parameters in non-phantom positions.
///
/// Time complexity: `O(size_of_type)`
fn check_phantom_params(
    struct_handles: &[StructHandle],
    context: &[StructTypeParameter],
    is_phantom_pos: bool,
    ty: &SignatureToken,
) -> PartialVMResult<()> {
    use SignatureToken::*;

    match ty {
        Vector(ty) => check_phantom_params(struct_handles, context, false, ty)?,
        Function(args, result, _) => {
            for ty in args.iter().chain(result) {
                check_phantom_params(struct_handles, context, false, ty)?
            }
        },
        StructInstantiation(idx, type_arguments) => {
            let sh = &struct_handles[idx.0 as usize];
            for (i, ty) in type_arguments.iter().enumerate() {
                check_phantom_params(
                    struct_handles,
                    context,
                    sh.type_parameters[i].is_phantom,
                    ty,
                )?;
            }
        },
        TypeParameter(idx) => {
            if context[*idx as usize].is_phantom && !is_phantom_pos {
                return Err(
                    PartialVMError::new(StatusCode::INVALID_PHANTOM_TYPE_PARAM_POSITION)
                        .with_message(
                            "phantom type parameter cannot be used in non-phantom position"
                                .to_string(),
                        ),
                );
            }
        },

        Struct(_) | Reference(_) | MutableReference(_) | Bool | U8 | U16 | U32 | U64 | U128
        | U256 | Address | Signer => {},
    }

    Ok(())
}

struct SignatureChecker<'a, const N: usize> {
    resolver: BinaryIndexedView<'a>,

    // Here the arena is used as a scoped interner, allowing us to store references in the
    // caches below.
    //
    // TODO: Now that we since have fully migrated to the bitset based implementation, we
    // may want to consider removing the arena and store owned copies of the constraint
    // sets directly in the caches below.
    constraints: &'a Arena<BitsetTypeParameterConstraints<N>>,

    // Cached results of the context-less checks.
    //
    // The interior mutability pattern (RefCell) is used because it is hard to prove to the
    // borrow checker that the partial borrows the algorithm makes are disjoint.
    //
    // TODO: Right now looking up ty_results can be a major bottleneck.
    //       Can we make it faster?
    ty_results: RefCell<
        BTreeMap<(SignatureIndex, usize, AbilitySet), &'a BitsetTypeParameterConstraints<N>>,
    >,
    sig_results: RefCell<BTreeMap<SignatureIndex, &'a BitsetTypeParameterConstraints<N>>>,
    func_inst_results: RefCell<
        BTreeMap<(FunctionHandleIndex, SignatureIndex), &'a BitsetTypeParameterConstraints<N>>,
    >,
    struct_inst_results: RefCell<
        BTreeMap<(StructDefInstantiationIndex, AbilitySet), &'a BitsetTypeParameterConstraints<N>>,
    >,
    field_inst_results:
        RefCell<BTreeMap<FieldInstantiationIndex, &'a BitsetTypeParameterConstraints<N>>>,
    variant_field_inst_results:
        RefCell<BTreeMap<VariantFieldInstantiationIndex, &'a BitsetTypeParameterConstraints<N>>>,
    struct_variant_inst_results: RefCell<
        BTreeMap<
            (StructVariantInstantiationIndex, AbilitySet),
            &'a BitsetTypeParameterConstraints<N>,
        >,
    >,
}

impl<'a, const N: usize> SignatureChecker<'a, N> {
    fn new(
        constraints: &'a Arena<BitsetTypeParameterConstraints<N>>,
        resolver: BinaryIndexedView<'a>,
    ) -> Self {
        Self {
            resolver,
            constraints,

            ty_results: RefCell::new(BTreeMap::new()),
            sig_results: RefCell::new(BTreeMap::new()),
            func_inst_results: RefCell::new(BTreeMap::new()),
            struct_inst_results: RefCell::new(BTreeMap::new()),
            field_inst_results: RefCell::new(BTreeMap::new()),
            variant_field_inst_results: RefCell::new(BTreeMap::new()),
            struct_variant_inst_results: RefCell::new(BTreeMap::new()),
        }
    }

    /// Checks if particular type in a signature is well-formed and has the required abilities, in a
    /// context-less fashion.
    ///
    /// Returns the minimal set of constraints the type parameters need to satisfy, with the result
    /// being cached.
    ///
    /// Time complexity: `O(size_of_type)` if not cached.
    fn verify_type_in_signature_contextless(
        &self,
        sig_idx: SignatureIndex,
        ty_idx: usize,
        required_abilities: AbilitySet,
    ) -> PartialVMResult<&'a BitsetTypeParameterConstraints<N>> {
        let r = match self
            .ty_results
            .borrow_mut()
            .entry((sig_idx, ty_idx, required_abilities))
        {
            btree_map::Entry::Occupied(entry) => *entry.into_mut(),
            btree_map::Entry::Vacant(entry) => {
                let mut param_constraints = BitsetTypeParameterConstraints::new();
                let sig = self.resolver.signature_at(sig_idx);
                let ty = &sig.0[ty_idx];
                check_ty(
                    self.resolver.struct_handles(),
                    ty,
                    true,
                    required_abilities,
                    &mut param_constraints,
                )
                .map_err(|err| {
                    err.append_message_with_separator(' ', format!("at type {}", ty_idx))
                        .at_index(IndexKind::Signature, sig_idx.0)
                })?;

                let r = self.constraints.alloc(param_constraints);

                *entry.insert(r)
            },
        };

        Ok(r)
    }

    /// Checks if a signature (list of types) is well-formed, in a context-less fashion.
    /// This does not impose additional ability requirements on individual types.
    ///
    /// Returns the minimal set of constraints the type parameters need to satisfy, with the result
    /// being cached.
    ///
    /// Time complexity: `O(total_size_of_all_types)` if not cached.
    fn verify_signature_contextless(
        &self,
        sig_idx: SignatureIndex,
    ) -> PartialVMResult<&'a BitsetTypeParameterConstraints<N>> {
        let r = match self.sig_results.borrow_mut().entry(sig_idx) {
            btree_map::Entry::Occupied(entry) => *entry.into_mut(),
            btree_map::Entry::Vacant(entry) => {
                let mut constraints = BitsetTypeParameterConstraints::new();

                for ty_idx in 0..self.resolver.signature_at(sig_idx).len() {
                    constraints.merge(self.verify_type_in_signature_contextless(
                        sig_idx,
                        ty_idx,
                        AbilitySet::EMPTY,
                    )?)
                }

                *entry.insert(self.constraints.alloc(constraints))
            },
        };

        Ok(r)
    }

    /// Verifies that all signatures in the signature pool are well-formed, in a context-less fashion.
    fn verify_signature_pool_contextless(&self) -> PartialVMResult<()> {
        for sig_idx in 0..self.resolver.signatures().len() {
            self.verify_signature_contextless(SignatureIndex(sig_idx as u16))?;
        }
        Ok(())
    }

    /// Checks if a signature is well-formed within a specific context.
    ///
    /// Time complexity: same as `verify_signature`.
    fn verify_signature_in_context(
        &self,
        ability_context: &BitsetTypeParameterConstraints<N>,
        sig_idx: SignatureIndex,
    ) -> PartialVMResult<()> {
        let constraints = self.verify_signature_contextless(sig_idx)?;
        constraints.check_in_context(ability_context)?;
        Ok(())
    }

    /// Checks if a function instantiation is well-formed, in a context-less fashion.
    ///
    /// A function instantiation is well-formed if
    /// - There are no references in the type arguments
    /// - All type arguments are well-formed and have declared abilities
    ///
    /// Returns the minimal set of constraints the type parameters need to satisfy, with the result
    /// being cached.
    ///
    /// Time complexity: `O(total_size_of_all_type_args)` if not cached.
    fn verify_function_instantiation_contextless(
        &self,
        func_inst_idx: FunctionInstantiationIndex,
    ) -> PartialVMResult<&'a BitsetTypeParameterConstraints<N>> {
        let func_inst = self.resolver.function_instantiation_at(func_inst_idx);
        let ty_args_idx = func_inst.type_parameters;

        let r = match self
            .func_inst_results
            .borrow_mut()
            .entry((func_inst.handle, ty_args_idx))
        {
            btree_map::Entry::Occupied(entry) => *entry.into_mut(),
            btree_map::Entry::Vacant(entry) => {
                let func_handle = self.resolver.function_handle_at(func_inst.handle);
                let ty_args = &self.resolver.signature_at(ty_args_idx).0;

                // TODO: is this needed?
                if func_handle.type_parameters.len() != ty_args.len() {
                    return Err(
                        PartialVMError::new(StatusCode::NUMBER_OF_TYPE_ARGUMENTS_MISMATCH)
                            .with_message(format!(
                                "expected {} type argument(s), got {}",
                                func_handle.type_parameters.len(),
                                ty_args.len()
                            ))
                            .at_index(IndexKind::FunctionInstantiation, func_inst_idx.0),
                    );
                }

                let mut constraints = BitsetTypeParameterConstraints::new();
                for (ty_idx, ty) in ty_args.iter().enumerate() {
                    if ty.is_reference() {
                        return Err(PartialVMError::new(StatusCode::INVALID_SIGNATURE_TOKEN)
                            .with_message("reference not allowed".to_string())
                            .at_index(IndexKind::FunctionInstantiation, func_inst_idx.0));
                    }

                    constraints.merge(self.verify_type_in_signature_contextless(
                        ty_args_idx,
                        ty_idx,
                        func_handle.type_parameters[ty_idx],
                    )?)
                }

                *entry.insert(self.constraints.alloc(constraints))
            },
        };

        Ok(r)
    }

    /// Checks if all function instantiations are well-formed, in a context-less fashion.
    ///
    /// Time complexity: `O(total_size_of_all_type_args_in_all_instantiations)`
    fn verify_function_instantiations_contextless(&self) -> PartialVMResult<()> {
        for func_inst_idx in 0..self.resolver.function_instantiations().len() {
            self.verify_function_instantiation_contextless(FunctionInstantiationIndex(
                func_inst_idx as u16,
            ))?;
        }
        Ok(())
    }

    /// Checks if a struct instantiation is well-formed, in a context-less fashion,
    /// with the result being cached.
    fn verify_struct_instantiation_contextless(
        &self,
        struct_inst_idx: StructDefInstantiationIndex,
        required_abilities: AbilitySet,
    ) -> PartialVMResult<&'a BitsetTypeParameterConstraints<N>> {
        let r = match self
            .struct_inst_results
            .borrow_mut()
            .entry((struct_inst_idx, required_abilities))
        {
            btree_map::Entry::Occupied(entry) => *entry.into_mut(),
            btree_map::Entry::Vacant(entry) => {
                let struct_inst = self.resolver.struct_instantiation_at(struct_inst_idx)?;
                let constraints = self
                    .verify_struct_type_params(
                        required_abilities,
                        struct_inst.def,
                        struct_inst.type_parameters,
                    )
                    .map_err(|err| {
                        err.at_index(IndexKind::StructDefInstantiation, struct_inst_idx.0)
                    })?;
                *entry.insert(self.constraints.alloc(constraints))
            },
        };

        Ok(r)
    }

    /// Checks if a struct variant instantiation is well-formed, in a context-less fashion,
    /// with the result being cached.
    fn verify_struct_variant_instantiation_contextless(
        &self,
        struct_variant_inst_idx: StructVariantInstantiationIndex,
        required_abilities: AbilitySet,
    ) -> PartialVMResult<&'a BitsetTypeParameterConstraints<N>> {
        let r = match self
            .struct_variant_inst_results
            .borrow_mut()
            .entry((struct_variant_inst_idx, required_abilities))
        {
            btree_map::Entry::Occupied(entry) => *entry.into_mut(),
            btree_map::Entry::Vacant(entry) => {
                let struct_variant_inst = self
                    .resolver
                    .struct_variant_instantiation_at(struct_variant_inst_idx)?;
                let struct_variant_handle = self
                    .resolver
                    .struct_variant_handle_at(struct_variant_inst.handle)?;
                let constraints = self
                    .verify_struct_type_params(
                        required_abilities,
                        struct_variant_handle.struct_index,
                        struct_variant_inst.type_parameters,
                    )
                    .map_err(|err| {
                        err.at_index(
                            IndexKind::StructVariantInstantiation,
                            struct_variant_inst_idx.0,
                        )
                    })?;
                *entry.insert(self.constraints.alloc(constraints))
            },
        };

        Ok(r)
    }

    /// Checks if a struct instantiation is well-formed, in a context-less fashion.
    ///
    /// A struct instantiation is well-formed if
    /// - There are no references in the type arguments
    /// - All type arguments are well-formed and have declared abilities
    ///
    /// Returns the minimal set of constraints the type parameters need to satisfy.
    ///
    /// Time complexity: `O(total_size_of_all_type_args)` if not cached.
    fn verify_struct_type_params(
        &self,
        required_abilities: AbilitySet,
        struct_def_idx: StructDefinitionIndex,
        ty_args_idx: SignatureIndex,
    ) -> PartialVMResult<BitsetTypeParameterConstraints<N>> {
        let struct_def = self.resolver.struct_def_at(struct_def_idx)?;
        let struct_handle = self.resolver.struct_handle_at(struct_def.struct_handle);
        let ty_args = &self.resolver.signature_at(ty_args_idx).0;

        if struct_handle.type_parameters.len() != ty_args.len() {
            return Err(
                PartialVMError::new(StatusCode::NUMBER_OF_TYPE_ARGUMENTS_MISMATCH).with_message(
                    format!(
                        "expected {} type argument(s), got {}",
                        struct_handle.type_parameters.len(),
                        ty_args.len()
                    ),
                ),
            );
        }

        if !required_abilities.is_subset(struct_handle.abilities) {
            return Err(
                PartialVMError::new(StatusCode::CONSTRAINT_NOT_SATISFIED).with_message(format!(
                    "expected struct with abilities {:?} got {:?}",
                    required_abilities, struct_handle.abilities
                )),
            );
        }

        let mut constraints = BitsetTypeParameterConstraints::new();
        for (ty_idx, ty) in ty_args.iter().enumerate() {
            if ty.is_reference() {
                return Err(PartialVMError::new(StatusCode::INVALID_SIGNATURE_TOKEN)
                    .with_message("reference not allowed".to_string()));
            }

            let arg_abilities = if struct_handle.type_parameters[ty_idx].is_phantom {
                struct_handle.type_parameters[ty_idx].constraints
            } else {
                struct_handle.type_parameters[ty_idx]
                    .constraints
                    .union(required_abilities.requires())
            };

            constraints.merge(self.verify_type_in_signature_contextless(
                ty_args_idx,
                ty_idx,
                arg_abilities,
            )?);
        }
        Ok(constraints)
    }

    /// Checks if all struct instantiations are well-formed, in a context-less fashion.
    ///
    /// Time complexity: `O(total_size_of_all_type_args_in_all_instantiations)`
    fn verify_struct_instantiations_contextless(&self) -> PartialVMResult<()> {
        for struct_inst_idx in 0..self.resolver.struct_instantiations().unwrap().len() {
            self.verify_struct_instantiation_contextless(
                StructDefInstantiationIndex(struct_inst_idx as u16),
                AbilitySet::EMPTY,
            )?;
        }
        Ok(())
    }

    /// Checks if all struct variant instantiations are well-formed, in a context-less fashion.
    ///
    /// Time complexity: `O(total_size_of_all_type_args_in_all_instantiations)`
    fn verify_struct_variant_instantiations_contextless(&self) -> PartialVMResult<()> {
        for struct_inst_idx in 0..self.resolver.struct_variant_instantiations().unwrap().len() {
            self.verify_struct_variant_instantiation_contextless(
                StructVariantInstantiationIndex(struct_inst_idx as u16),
                AbilitySet::EMPTY,
            )?;
        }
        Ok(())
    }

    /// Checks if a field instantiation is well-formed, in a context-less fashion,
    /// with the result being cached.
    fn verify_field_instantiation_contextless(
        &self,
        field_inst_idx: FieldInstantiationIndex,
    ) -> PartialVMResult<&'a BitsetTypeParameterConstraints<N>> {
        let r = match self.field_inst_results.borrow_mut().entry(field_inst_idx) {
            btree_map::Entry::Occupied(entry) => *entry.into_mut(),
            btree_map::Entry::Vacant(entry) => {
                let field_inst = self.resolver.field_instantiation_at(field_inst_idx)?;
                let field_handle = self.resolver.field_handle_at(field_inst.handle)?;
                let constraints = self
                    .verify_struct_type_params(
                        AbilitySet::EMPTY,
                        field_handle.owner,
                        field_inst.type_parameters,
                    )
                    .map_err(|err| err.at_index(IndexKind::FieldInstantiation, field_inst_idx.0))?;
                *entry.insert(self.constraints.alloc(constraints))
            },
        };
        Ok(r)
    }

    /// Same like `verify_field_instantiation_contextless` but for variant fields.
    fn verify_variant_field_instantiation_contextless(
        &self,
        field_inst_idx: VariantFieldInstantiationIndex,
    ) -> PartialVMResult<&'a BitsetTypeParameterConstraints<N>> {
        let r = match self
            .variant_field_inst_results
            .borrow_mut()
            .entry(field_inst_idx)
        {
            btree_map::Entry::Occupied(entry) => *entry.into_mut(),
            btree_map::Entry::Vacant(entry) => {
                let variant_field_inst = self
                    .resolver
                    .variant_field_instantiation_at(field_inst_idx)?;
                let field_handle = self
                    .resolver
                    .variant_field_handle_at(variant_field_inst.handle)?;
                let constraints = self
                    .verify_struct_type_params(
                        AbilitySet::EMPTY,
                        field_handle.struct_index,
                        variant_field_inst.type_parameters,
                    )
                    .map_err(|err| {
                        err.at_index(IndexKind::VariantFieldInstantiation, field_inst_idx.0)
                    })?;
                *entry.insert(self.constraints.alloc(constraints))
            },
        };
        Ok(r)
    }

    /// Checks if all field instantiations are well-formed, in a context-less fashion.
    ///
    /// Time complexity: `O(total_size_of_all_type_args_in_all_instantiations)`
    fn verify_field_instantiations_contextless(&self) -> PartialVMResult<()> {
        for field_inst_idx in 0..self.resolver.field_instantiations().unwrap().len() {
            self.verify_field_instantiation_contextless(FieldInstantiationIndex(
                field_inst_idx as u16,
            ))?;
        }
        Ok(())
    }

    /// Checks if all variant field instantiations are well-formed, in a context-less fashion.
    ///
    /// Time complexity: `O(total_size_of_all_type_args_in_all_instantiations)`
    fn verify_variant_field_instantiations_contextless(&self) -> PartialVMResult<()> {
        for field_inst_idx in 0..self.resolver.variant_field_instantiations().unwrap().len() {
            self.verify_variant_field_instantiation_contextless(VariantFieldInstantiationIndex(
                field_inst_idx as u16,
            ))?;
        }
        Ok(())
    }

    /// Checks if a function handle is well-formed.
    ///
    /// A function handle is well-formed if all parameter and return types are well-formed, with references
    /// being allowed.
    ///
    /// Time complexity: `O(total_size_of_all_types)` if not cached.
    fn verify_function_handle(&self, fh: &FunctionHandle) -> PartialVMResult<()> {
        let ability_context = BitsetTypeParameterConstraints::from(fh.type_parameters.as_slice());
        self.verify_signature_in_context(&ability_context, fh.parameters)?;
        self.verify_signature_in_context(&ability_context, fh.return_)?;
        Ok(())
    }

    /// Checks if all function handles are well-formed.
    fn verify_function_handles(&self) -> PartialVMResult<()> {
        for (idx, fh) in self.resolver.function_handles().iter().enumerate() {
            self.verify_function_handle(fh)
                .map_err(|err| err.at_index(IndexKind::FunctionHandle, idx as u16))?;
        }
        Ok(())
    }

    /// Checks if a code unit is well-formed.
    ///
    /// A code unit is well-formed if
    /// - The locals are well-formed within the context. (References are allowed.)
    /// - All instantiations (function, struct, field & vector) are well-formed within the context and do not
    ///   contain references.
    ///
    /// Time complexity: `O(num_of_ty_params * num_of_instantiations)`,
    ///                  assuming that the verification results for the instantiations have been cached.
    fn verify_code(&self, ability_context: &[AbilitySet], code: &CodeUnit) -> PartialVMResult<()> {
        use Bytecode::*;

        let ability_context = BitsetTypeParameterConstraints::from(ability_context);

        self.verify_signature_in_context(&ability_context, code.locals)
            .map_err(|err| err.at_index(IndexKind::Signature, code.locals.0))?;

        // Local caches to avoid re-verifying identical instantiations within the same context.
        let mut checked_func_insts = BTreeMap::<FunctionInstantiationIndex, ()>::new();
        let mut checked_struct_def_insts = BTreeMap::<StructDefInstantiationIndex, ()>::new();
        let mut checked_struct_def_insts_with_key =
            BTreeMap::<StructDefInstantiationIndex, ()>::new();
        let mut checked_vec_insts = BTreeMap::<SignatureIndex, ()>::new();
        let mut checked_field_insts = BTreeMap::<FieldInstantiationIndex, ()>::new();
        let mut checked_variant_field_insts = BTreeMap::<VariantFieldInstantiationIndex, ()>::new();
        let mut checked_struct_variant_insts =
            BTreeMap::<StructVariantInstantiationIndex, ()>::new();

        for (offset, instr) in code.code.iter().enumerate() {
            let map_err = |res: PartialVMResult<()>| {
                res.map_err(|err| {
                    err.append_message_with_separator(
                        ' ',
                        format!(
                            "missing abilities for `{:?}` at code offset {}",
                            instr, offset
                        ),
                    )
                })
            };
            match instr {
                CallGeneric(idx) | ClosPackGeneric(idx, _) => {
                    if let btree_map::Entry::Vacant(entry) = checked_func_insts.entry(*idx) {
                        let constraints = self.verify_function_instantiation_contextless(*idx)?;
                        map_err(constraints.check_in_context(&ability_context))?;
                        entry.insert(());
                    }
                },
                PackGeneric(idx) | UnpackGeneric(idx) => {
                    if let btree_map::Entry::Vacant(entry) = checked_struct_def_insts.entry(*idx) {
                        let constraints =
                            self.verify_struct_instantiation_contextless(*idx, AbilitySet::EMPTY)?;
                        map_err(constraints.check_in_context(&ability_context))?;
                        entry.insert(());
                    }
                },
                PackVariantGeneric(idx) | UnpackVariantGeneric(idx) | TestVariantGeneric(idx) => {
                    if let btree_map::Entry::Vacant(entry) =
                        checked_struct_variant_insts.entry(*idx)
                    {
                        let constraints = self.verify_struct_variant_instantiation_contextless(
                            *idx,
                            AbilitySet::EMPTY,
                        )?;
                        map_err(constraints.check_in_context(&ability_context))?;
                        entry.insert(());
                    }
                },
                ExistsGeneric(idx)
                | MoveFromGeneric(idx)
                | MoveToGeneric(idx)
                | ImmBorrowGlobalGeneric(idx)
                | MutBorrowGlobalGeneric(idx) => {
                    if let btree_map::Entry::Vacant(entry) =
                        checked_struct_def_insts_with_key.entry(*idx)
                    {
                        let constraints = self.verify_struct_instantiation_contextless(
                            *idx,
                            AbilitySet::singleton(Ability::Key),
                        )?;
                        map_err(constraints.check_in_context(&ability_context))?;
                        entry.insert(());
                    }
                },
                ImmBorrowFieldGeneric(idx) | MutBorrowFieldGeneric(idx) => {
                    if let btree_map::Entry::Vacant(entry) = checked_field_insts.entry(*idx) {
                        let constraints = self.verify_field_instantiation_contextless(*idx)?;
                        map_err(constraints.check_in_context(&ability_context))?;
                        entry.insert(());
                    }
                },
                ImmBorrowVariantFieldGeneric(idx) | MutBorrowVariantFieldGeneric(idx) => {
                    if let btree_map::Entry::Vacant(entry) = checked_variant_field_insts.entry(*idx)
                    {
                        let constraints =
                            self.verify_variant_field_instantiation_contextless(*idx)?;
                        map_err(constraints.check_in_context(&ability_context))?;
                        entry.insert(());
                    }
                },
                ClosEval(idx) => {
                    let sign = self.resolver.signature_at(*idx);
                    if sign.len() != 1 || !matches!(&sign.0[0], SignatureToken::Function(..)) {
                        return map_err(Err(PartialVMError::new(
                            StatusCode::CLOSURE_EVAL_REQUIRES_FUNCTION,
                        )));
                    }
                },
                VecPack(idx, _)
                | VecLen(idx)
                | VecImmBorrow(idx)
                | VecMutBorrow(idx)
                | VecPushBack(idx)
                | VecPopBack(idx)
                | VecUnpack(idx, _)
                | VecSwap(idx) => {
                    if let btree_map::Entry::Vacant(entry) = checked_vec_insts.entry(*idx) {
                        let ty_args = &self.resolver.signature_at(*idx).0;
                        if ty_args.len() != 1 {
                            return map_err(Err(PartialVMError::new(
                                StatusCode::NUMBER_OF_TYPE_ARGUMENTS_MISMATCH,
                            )
                            .with_message(format!(
                                "expected 1 type token for vector operations, got {}",
                                ty_args.len()
                            ))));
                        }

                        if ty_args[0].is_reference() {
                            return map_err(Err(PartialVMError::new(
                                StatusCode::INVALID_SIGNATURE_TOKEN,
                            )
                            .with_message("reference not allowed".to_string())));
                        }
                        map_err(self.verify_signature_in_context(&ability_context, *idx))?;

                        entry.insert(());
                    }
                },

                // List out the other options explicitly so there's a compile error if a new
                // bytecode gets added.
                Pop
                | Ret
                | Branch(_)
                | BrTrue(_)
                | BrFalse(_)
                | LdU8(_)
                | LdU16(_)
                | LdU32(_)
                | LdU64(_)
                | LdU128(_)
                | LdU256(_)
                | LdConst(_)
                | CastU8
                | CastU16
                | CastU32
                | CastU64
                | CastU128
                | CastU256
                | LdTrue
                | LdFalse
                | Call(_)
                | ClosPack(..)
                | Pack(_)
                | Unpack(_)
                | TestVariant(_)
                | PackVariant(_)
                | UnpackVariant(_)
                | ReadRef
                | WriteRef
                | FreezeRef
                | Add
                | Sub
                | Mul
                | Mod
                | Div
                | BitOr
                | BitAnd
                | Xor
                | Shl
                | Shr
                | Or
                | And
                | Not
                | Eq
                | Neq
                | Lt
                | Gt
                | Le
                | Ge
                | CopyLoc(_)
                | MoveLoc(_)
                | StLoc(_)
                | MutBorrowLoc(_)
                | ImmBorrowLoc(_)
                | MutBorrowField(_)
                | ImmBorrowField(_)
                | MutBorrowVariantField(_)
                | ImmBorrowVariantField(_)
                | MutBorrowGlobal(_)
                | ImmBorrowGlobal(_)
                | Exists(_)
                | MoveTo(_)
                | MoveFrom(_)
                | Abort
                | Nop => (),
            }
        }

        Ok(())
    }

    /// Checks if a function definition is well-formed.
    fn verify_function_def(&self, fdef: &FunctionDefinition) -> PartialVMResult<()> {
        let code = match &fdef.code {
            Some(code) => code,
            None => return Ok(()), // No need to check native functions
        };

        let fh = self.resolver.function_handle_at(fdef.function);

        self.verify_code(&fh.type_parameters, code)
    }

    /// Checks if all function defs are well-formed.
    fn verify_function_defs(&self) -> PartialVMResult<()> {
        for (idx, fdef) in self.resolver.function_defs().unwrap().iter().enumerate() {
            self.verify_function_def(fdef)
                .map_err(|err| err.at_index(IndexKind::FunctionDefinition, idx as u16))?;
        }
        Ok(())
    }

    /// Checks if a struct definition is well-formed.
    ///
    /// A struct definition is well-formed if
    /// - All field types are well-formed within the struct context.
    /// - No phantom type parameters appear in non-phantom positions.
    ///
    /// Time complexity: `O(total_size_of_field_types)`
    fn verify_struct_def(&self, struct_def: &StructDefinition) -> PartialVMResult<()> {
        let struct_handle = self.resolver.struct_handle_at(struct_def.struct_handle);
        let context = struct_handle
            .type_param_constraints()
            .enumerate()
            .map(|(idx, abilities)| (idx as TypeParameterIndex, abilities))
            .collect::<BitsetTypeParameterConstraints<N>>();

        let required_abilities_conditional = struct_handle.abilities.requires();
        let context_all_abilities = (0..struct_handle.type_parameters.len())
            .map(|idx| (idx as TypeParameterIndex, AbilitySet::ALL))
            .collect::<BitsetTypeParameterConstraints<N>>();

        match &struct_def.field_information {
            StructFieldInformation::Native => Ok(()),
            StructFieldInformation::Declared(fields) => self.verify_fields_of_struct(
                &struct_handle,
                &context,
                required_abilities_conditional,
                &context_all_abilities,
                fields.iter(),
            ),
            StructFieldInformation::DeclaredVariants(variants) => self.verify_fields_of_struct(
                &struct_handle,
                &context,
                required_abilities_conditional,
                &context_all_abilities,
                variants.iter().flat_map(|v| v.fields.iter()),
            ),
        }
    }

    fn verify_fields_of_struct<'l>(
        &self,
        struct_handle: &&StructHandle,
        context: &BitsetTypeParameterConstraints<{ N }>,
        required_abilities_conditional: AbilitySet,
        context_all_abilities: &BitsetTypeParameterConstraints<{ N }>,
        fields: impl Iterator<Item = &'l FieldDefinition>,
    ) -> Result<(), PartialVMError> {
        for field_def in fields {
            let field_ty = &field_def.signature.0;

            // Check if the field type itself is well-formed.
            check_ty_in_context(
                self.resolver.struct_handles(),
                context,
                field_ty,
                false,
                AbilitySet::EMPTY,
            )?;

            // Check if the field type satisfies the conditional ability requirements.
            check_ty_in_context(
                self.resolver.struct_handles(),
                context_all_abilities,
                field_ty,
                false,
                required_abilities_conditional,
            )
            .map_err(|_err| PartialVMError::new(StatusCode::FIELD_MISSING_TYPE_ABILITY))?;

            check_phantom_params(
                self.resolver.struct_handles(),
                &struct_handle.type_parameters,
                false,
                field_ty,
            )?;
        }
        Ok(())
    }

    /// Checks if all struct defs are well-formed.
    fn verify_struct_defs(&self) -> PartialVMResult<()> {
        for (idx, struct_def) in self.resolver.struct_defs().unwrap().iter().enumerate() {
            self.verify_struct_def(struct_def)
                .map_err(|err| err.at_index(IndexKind::StructDefinition, idx as u16))?;
        }
        Ok(())
    }
}

fn verify_module_impl<const N: usize>(module: &CompiledModule) -> PartialVMResult<()> {
    let arena = Arena::<BitsetTypeParameterConstraints<N>>::new();
    let checker = SignatureChecker::new(&arena, BinaryIndexedView::Module(module));

    // Check if all signatures & instantiations are well-formed without any specific contexts.
    // This is only needed if we want to keep the binary format super clean.
    checker.verify_signature_pool_contextless()?;
    checker.verify_function_instantiations_contextless()?;
    checker.verify_struct_instantiations_contextless()?;
    checker.verify_field_instantiations_contextless()?;
    checker.verify_struct_variant_instantiations_contextless()?;
    checker.verify_variant_field_instantiations_contextless()?;

    checker.verify_function_handles()?;
    checker.verify_function_defs()?;
    checker.verify_struct_defs()?;

    Ok(())
}

fn verify_script_impl<const N: usize>(script: &CompiledScript) -> PartialVMResult<()> {
    let arena = Arena::<BitsetTypeParameterConstraints<N>>::new();
    let checker = SignatureChecker::new(&arena, BinaryIndexedView::Script(script));

    // Check if all signatures & instantiations are well-formed without any specific contexts.
    // This is only needed if we want to keep the binary format super clean.
    checker.verify_signature_pool_contextless()?;
    checker.verify_function_instantiations_contextless()?;

    checker.verify_function_handles()?;
    checker.verify_signature_in_context(
        &BitsetTypeParameterConstraints::from(script.type_parameters.as_slice()),
        script.parameters,
    )?;
    checker.verify_code(&script.type_parameters, &script.code)?;

    Ok(())
}

fn max_num_of_ty_params_or_args(resolver: BinaryIndexedView) -> usize {
    let mut n = 0;

    for fh in resolver.function_handles() {
        n = n.max(fh.type_parameters.len())
    }

    for sh in resolver.struct_handles() {
        n = n.max(sh.type_parameters.len())
    }

    for sig in resolver.signatures() {
        for ty in &sig.0 {
            for ty in ty.preorder_traversal() {
                if let SignatureToken::TypeParameter(ty_param_idx) = ty {
                    n = n.max(*ty_param_idx as usize + 1)
                }
            }
        }
    }

    if let Some(struct_defs) = resolver.struct_defs() {
        for struct_def in struct_defs {
            match &struct_def.field_information {
                StructFieldInformation::Native => {},
                StructFieldInformation::Declared(fields) => {
                    for field in fields {
                        for ty in field.signature.0.preorder_traversal() {
                            if let SignatureToken::TypeParameter(ty_param_idx) = ty {
                                n = n.max(*ty_param_idx as usize + 1)
                            }
                        }
                    }
                },
                StructFieldInformation::DeclaredVariants(variants) => {
                    for variant in variants {
                        for field in &variant.fields {
                            for ty in field.signature.0.preorder_traversal() {
                                if let SignatureToken::TypeParameter(ty_param_idx) = ty {
                                    n = n.max(*ty_param_idx as usize + 1)
                                }
                            }
                        }
                    }
                },
            }
        }
    }

    n
}

pub fn verify_module(module: &CompiledModule) -> VMResult<()> {
    let max_num = max_num_of_ty_params_or_args(BinaryIndexedView::Module(module));

    let res = if max_num <= NUM_PARAMS_PER_WORD {
        verify_module_impl::<1>(module)
    } else if max_num <= NUM_PARAMS_PER_WORD * 2 {
        verify_module_impl::<2>(module)
    } else if max_num <= NUM_PARAMS_PER_WORD * 16 {
        verify_module_impl::<16>(module)
    } else {
        return Err(
            PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                .with_message("too many type parameters/arguments in the program".to_string())
                .finish(Location::Undefined),
        );
    };

    res.map_err(|e| e.finish(Location::Module(module.self_id())))
}

pub fn verify_script(config: &VerifierConfig, script: &CompiledScript) -> VMResult<()> {
    let mut max_num = max_num_of_ty_params_or_args(BinaryIndexedView::Script(script));
    if config.sig_checker_v2_fix_script_ty_param_count {
        max_num = max_num.max(script.type_parameters.len());
    }

    let res = if max_num <= NUM_PARAMS_PER_WORD {
        verify_script_impl::<1>(script)
    } else if max_num <= NUM_PARAMS_PER_WORD * 2 {
        verify_script_impl::<2>(script)
    } else if max_num <= NUM_PARAMS_PER_WORD * 16 {
        verify_script_impl::<16>(script)
    } else {
        return Err(
            PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                .with_message("too many type parameters/arguments in the program".to_string())
                .finish(Location::Undefined),
        );
    };

    res.map_err(|e| e.finish(Location::Script))
}
