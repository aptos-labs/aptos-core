// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    file_format::{
        AbilitySet, FieldDefinition, IdentifierIndex, ModuleHandleIndex, SignatureToken,
        StructDefinition, StructFieldInformation, StructHandle, StructHandleIndex,
        StructTypeParameter, TableIndex, TypeSignature, VariantDefinition,
    },
    internals::ModuleIndex,
    proptest_types::{
        prop_index_avoid,
        signature::{AbilitySetGen, SignatureTokenGen},
    },
};
use proptest::{
    collection::{vec, SizeRange},
    option,
    prelude::*,
    sample::Index as PropIndex,
    std_facade::hash_set::HashSet,
};
use std::collections::BTreeSet;

#[allow(dead_code)]
#[derive(Debug)]
struct TypeSignatureIndex(u16);

#[derive(Debug)]
pub struct StDefnMaterializeState {
    pub self_module_handle_idx: ModuleHandleIndex,
    pub identifiers_len: usize,
    pub struct_handles: Vec<StructHandle>,
    pub new_handles: BTreeSet<(ModuleHandleIndex, IdentifierIndex)>,
}

impl StDefnMaterializeState {
    pub fn new(
        self_module_handle_idx: ModuleHandleIndex,
        identifiers_len: usize,
        struct_handles: Vec<StructHandle>,
    ) -> Self {
        Self {
            self_module_handle_idx,
            identifiers_len,
            struct_handles,
            new_handles: BTreeSet::new(),
        }
    }

    fn add_struct_handle(&mut self, handle: StructHandle) -> Option<StructHandleIndex> {
        if self.new_handles.insert((handle.module, handle.name)) {
            self.struct_handles.push(handle);
            Some(StructHandleIndex((self.struct_handles.len() - 1) as u16))
        } else {
            None
        }
    }

    fn potential_abilities(&self, ty: &SignatureToken) -> AbilitySet {
        use SignatureToken::*;

        match ty {
            Bool | U8 | U16 | U32 | U64 | U128 | U256 | Address => AbilitySet::PRIMITIVES,

            Reference(_) | MutableReference(_) => AbilitySet::REFERENCES,
            Signer => AbilitySet::SIGNER,
            TypeParameter(_) => AbilitySet::ALL,
            Vector(ty) => {
                let inner = self.potential_abilities(ty);
                inner.intersect(AbilitySet::VECTOR)
            },
            Function(_, _, a) => *a,
            Struct(idx) => {
                let sh = &self.struct_handles[idx.0 as usize];
                sh.abilities
            },
            StructInstantiation(idx, type_args) => {
                let sh = &self.struct_handles[idx.0 as usize];

                // Gather the abilities of the type actuals.
                let type_args_abilities = type_args.iter().map(|ty| self.potential_abilities(ty));
                type_args_abilities.fold(sh.abilities, |acc, ty_arg_abilities| {
                    acc.intersect(ty_arg_abilities)
                })
            },
        }
    }
}

#[derive(Clone, Debug)]
pub struct StructHandleGen {
    module_idx: PropIndex,
    name_idx: PropIndex,
    abilities: AbilitySetGen,
    type_parameters: Vec<(AbilitySetGen, bool)>,
}

impl StructHandleGen {
    pub fn strategy(ability_count: impl Into<SizeRange>) -> impl Strategy<Value = Self> {
        let ability_count = ability_count.into();
        (
            any::<PropIndex>(),
            any::<PropIndex>(),
            AbilitySetGen::strategy(),
            vec((AbilitySetGen::strategy(), any::<bool>()), ability_count),
        )
            .prop_map(|(module_idx, name_idx, abilities, type_parameters)| Self {
                module_idx,
                name_idx,
                abilities,
                type_parameters,
            })
    }

    pub fn materialize(
        self,
        self_module_handle_idx: ModuleHandleIndex,
        module_len: usize,
        identifiers_len: usize,
    ) -> StructHandle {
        let idx = prop_index_avoid(
            self.module_idx,
            self_module_handle_idx.into_index(),
            module_len,
        );
        let type_parameters = self
            .type_parameters
            .into_iter()
            .map(|(constraints, is_phantom)| StructTypeParameter {
                constraints: constraints.materialize(),
                is_phantom,
            })
            .collect();
        StructHandle {
            module: ModuleHandleIndex(idx as TableIndex),
            name: IdentifierIndex(self.name_idx.index(identifiers_len) as TableIndex),
            abilities: self.abilities.materialize(),
            type_parameters,
        }
    }
}

#[derive(Clone, Debug)]
pub struct StructDefinitionGen {
    name_idx: PropIndex,
    abilities: AbilitySetGen,
    type_parameters: Vec<(AbilitySetGen, bool)>,
    #[allow(dead_code)]
    is_public: bool,
    // Variants to generate, with prop index the name of the variant.
    variants: Vec<PropIndex>,
    field_defs: Option<Vec<FieldDefinitionGen>>,
}

impl StructDefinitionGen {
    pub fn strategy(
        field_count: impl Into<SizeRange>,
        type_parameter_count: impl Into<SizeRange>,
    ) -> impl Strategy<Value = Self> {
        (
            any::<PropIndex>(),
            AbilitySetGen::strategy(),
            vec(
                (AbilitySetGen::strategy(), any::<bool>()),
                type_parameter_count,
            ),
            any::<bool>(),
            vec(any::<PropIndex>(), 0..4), // Generate up to 4 variants (0 is pure struct)
            option::of(vec(FieldDefinitionGen::strategy(), field_count)),
        )
            .prop_map(
                |(name_idx, abilities, type_parameters, is_public, variants, field_defs)| Self {
                    name_idx,
                    abilities,
                    type_parameters,
                    is_public,
                    variants,
                    field_defs,
                },
            )
    }

    pub fn materialize(self, state: &mut StDefnMaterializeState) -> Option<StructDefinition> {
        let mut field_names = HashSet::new();
        let mut fields = vec![];
        match self.field_defs {
            None => (),
            Some(field_defs_gen) => {
                for fd_gen in field_defs_gen {
                    let field = fd_gen.materialize(state);
                    if field_names.insert(field.name) {
                        fields.push(field);
                    }
                }
            },
        };
        let abilities = fields
            .iter()
            .fold(self.abilities.materialize(), |acc, field| {
                acc.intersect(state.potential_abilities(&field.signature.0))
            });

        let type_parameters = self
            .type_parameters
            .into_iter()
            .map(|(constraints, is_phantom)| StructTypeParameter {
                constraints: constraints.materialize(),
                is_phantom,
            })
            .collect();
        let handle = StructHandle {
            module: state.self_module_handle_idx,
            name: IdentifierIndex(self.name_idx.index(state.identifiers_len) as TableIndex),
            abilities,
            type_parameters,
        };
        match state.add_struct_handle(handle) {
            Some(struct_handle) => {
                let field_information = if self.variants.is_empty() {
                    if fields.is_empty() {
                        StructFieldInformation::Native
                    } else {
                        StructFieldInformation::Declared(fields)
                    }
                } else {
                    // partition fields among variants
                    let mut variant_fields: Vec<Vec<FieldDefinition>> =
                        (0..self.variants.len()).map(|_| vec![]).collect();
                    for (i, fd) in fields.into_iter().enumerate() {
                        variant_fields[i % self.variants.len()].push(fd)
                    }
                    let mut seen_names = BTreeSet::new();
                    StructFieldInformation::DeclaredVariants(
                        variant_fields
                            .into_iter()
                            .zip(self.variants.iter())
                            .filter_map(|(fields, name)| {
                                let variant_name = name.index(state.identifiers_len) as TableIndex;
                                // avoid duplicates
                                if seen_names.insert(variant_name) {
                                    Some(VariantDefinition {
                                        name: IdentifierIndex(variant_name),
                                        fields,
                                    })
                                } else {
                                    None
                                }
                            })
                            .collect(),
                    )
                };
                Some(StructDefinition {
                    struct_handle,
                    field_information,
                })
            },
            None => None,
        }
    }
}

#[derive(Clone, Debug)]
struct FieldDefinitionGen {
    name_idx: PropIndex,
    signature_gen: SignatureTokenGen,
}

impl FieldDefinitionGen {
    fn strategy() -> impl Strategy<Value = Self> {
        (any::<PropIndex>(), SignatureTokenGen::atom_strategy()).prop_map(
            |(name_idx, signature_gen)| Self {
                name_idx,
                signature_gen,
            },
        )
    }

    fn materialize(self, state: &StDefnMaterializeState) -> FieldDefinition {
        FieldDefinition {
            name: IdentifierIndex(self.name_idx.index(state.identifiers_len) as TableIndex),
            signature: TypeSignature(self.signature_gen.materialize(&state.struct_handles)),
        }
    }
}
