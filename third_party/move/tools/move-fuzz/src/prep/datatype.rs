// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    deps::PkgKind,
    prep::{
        ident::DatatypeIdent,
        typing::{IntrinsicType, TypeBase, TypeItem, TypeRef, TypeTag},
    },
};
use move_binary_format::{
    binary_views::BinaryIndexedView,
    file_format::{SignatureToken, StructFieldInformation, StructHandle},
    CompiledModule,
};
use move_core_types::ability::AbilitySet;
use std::collections::{btree_map::Entry, BTreeMap};

/// Declaration of a datatype
#[derive(Clone, PartialEq, Eq)]
pub struct DatatypeDecl {
    pub ident: DatatypeIdent,
    pub generics: Vec<(AbilitySet, bool)>,
    pub abilities: AbilitySet,
    pub kind: PkgKind,
}

/// Content of a datatype
#[derive(Clone, PartialEq, Eq)]
pub enum DatatypeContent {
    Fields(Vec<TypeTag>),
    Variants(BTreeMap<String, Vec<TypeTag>>),
    Opaque,
}

/// A registry of datatypes
pub struct DatatypeRegistry {
    decls: BTreeMap<DatatypeIdent, DatatypeDecl>,
    contents: BTreeMap<DatatypeIdent, DatatypeContent>,
}

impl DatatypeRegistry {
    /// Create an empty registry
    pub fn new() -> Self {
        Self {
            decls: BTreeMap::new(),
            contents: BTreeMap::new(),
        }
    }

    /// Analyze a module and register datatypes found in this module
    pub fn analyze(&mut self, module: &CompiledModule, kind: PkgKind) {
        let binary = BinaryIndexedView::Module(module);

        // pass 1: register declarations
        for def in &module.struct_defs {
            let handle = binary.struct_handle_at(def.struct_handle);
            let ident = DatatypeIdent::from_struct_handle(&binary, handle);

            // skip intrinsic types
            if IntrinsicType::try_parse_ident(&ident).is_some() {
                continue;
            }

            // register the declaration
            self.insert_decl(
                Self::decl_from_handle(&binary, handle, kind),
                /* allow_opaque_upgrade */ true,
            );
        }

        // pass 2: fill in content
        for def in &module.struct_defs {
            let handle = binary.struct_handle_at(def.struct_handle);
            let ident = DatatypeIdent::from_struct_handle(&binary, handle);

            // skip intrinsic types
            if IntrinsicType::try_parse_ident(&ident).is_some() {
                continue;
            }

            // parse the content
            let content = match &def.field_information {
                StructFieldInformation::Native => DatatypeContent::Opaque,
                StructFieldInformation::Declared(fields) => {
                    let mut field_types = vec![];
                    for field_def in fields.iter() {
                        let tag =
                            match self.convert_signature_token(&binary, &field_def.signature.0) {
                                TypeRef::Base(tag) => tag,
                                TypeRef::ImmRef(_) | TypeRef::MutRef(_) => {
                                    panic!("unexpected reference type as struct field");
                                },
                            };
                        field_types.push(tag);
                    }
                    DatatypeContent::Fields(field_types)
                },
                StructFieldInformation::DeclaredVariants(variants) => {
                    let mut variant_table = BTreeMap::new();
                    for variant_def in variants {
                        let key = binary.identifier_at(variant_def.name).to_string();
                        let mut field_types = vec![];
                        for field_def in variant_def.fields.iter() {
                            let tag = match self
                                .convert_signature_token(&binary, &field_def.signature.0)
                            {
                                TypeRef::Base(tag) => tag,
                                TypeRef::ImmRef(_) | TypeRef::MutRef(_) => {
                                    panic!("unexpected reference type as enum variant");
                                },
                            };
                            field_types.push(tag);
                        }
                        let existing = variant_table.insert(key, field_types);
                        assert!(existing.is_none());
                    }
                    DatatypeContent::Variants(variant_table)
                },
            };

            // register the content
            self.insert_content(ident, content, /* allow_opaque_upgrade */ true);
        }

        // sanity check
        assert_eq!(self.decls.len(), self.contents.len());
        self.decls
            .keys()
            .zip(self.contents.keys())
            .for_each(|(ident_decl, ident_content)| assert_eq!(ident_decl, ident_content));
    }

    fn decl_from_handle(
        binary: &BinaryIndexedView,
        handle: &StructHandle,
        kind: PkgKind,
    ) -> DatatypeDecl {
        DatatypeDecl {
            ident: DatatypeIdent::from_struct_handle(binary, handle),
            generics: handle
                .type_parameters
                .iter()
                .map(|p| (p.constraints, p.is_phantom))
                .collect(),
            abilities: handle.abilities,
            kind,
        }
    }

    fn insert_decl(&mut self, decl: DatatypeDecl, allow_opaque_upgrade: bool) {
        let ident = decl.ident.clone();
        match self.decls.entry(ident.clone()) {
            Entry::Vacant(entry) => {
                entry.insert(decl);
            },
            Entry::Occupied(mut entry) => {
                if entry.get().generics == decl.generics && entry.get().abilities == decl.abilities
                {
                    let merged_kind = merge_pkg_kind(entry.get().kind, decl.kind);
                    if entry.get().kind != merged_kind {
                        entry.get_mut().kind = merged_kind;
                    }
                    return;
                }
                let can_upgrade = allow_opaque_upgrade
                    && matches!(self.contents.get(&ident), Some(DatatypeContent::Opaque));
                if can_upgrade {
                    entry.insert(decl);
                } else {
                    panic!("duplicate datatype declaration {ident}");
                }
            },
        }
    }

    fn insert_content(
        &mut self,
        ident: DatatypeIdent,
        content: DatatypeContent,
        allow_opaque_upgrade: bool,
    ) {
        match self.contents.entry(ident.clone()) {
            Entry::Vacant(entry) => {
                entry.insert(content);
            },
            Entry::Occupied(mut entry) => {
                if entry.get() == &content {
                    return;
                }
                let can_upgrade =
                    allow_opaque_upgrade && matches!(entry.get(), DatatypeContent::Opaque);
                if can_upgrade {
                    entry.insert(content);
                } else {
                    panic!("duplicate datatype content {ident}");
                }
            },
        }
    }

    fn ensure_decl_registered(
        &mut self,
        binary: &BinaryIndexedView,
        handle: &StructHandle,
    ) -> DatatypeIdent {
        let ident = DatatypeIdent::from_struct_handle(binary, handle);
        if !self.decls.contains_key(&ident) {
            let decl = Self::decl_from_handle(binary, handle, PkgKind::Dependency);
            self.insert_decl(decl, /* allow_opaque_upgrade */ false);
            self.insert_content(
                ident.clone(),
                DatatypeContent::Opaque,
                /* allow_opaque_upgrade */ false,
            );
        }
        ident
    }

    /// Iterate over all datatype declarations
    pub fn iter_decls(&self) -> impl Iterator<Item = &DatatypeDecl> {
        self.decls.values()
    }

    /// Lookup a datatype declaration
    pub fn lookup_decl(&self, ident: &DatatypeIdent) -> &DatatypeDecl {
        self.decls
            .get(ident)
            .unwrap_or_else(|| panic!("unregistered datatype {ident}"))
    }

    /// Lookup a datatype declaration
    pub fn lookup_decl_and_content(
        &self,
        ident: &DatatypeIdent,
    ) -> (&DatatypeDecl, &DatatypeContent) {
        let decl = self
            .decls
            .get(ident)
            .unwrap_or_else(|| panic!("unregistered datatype {ident}"));
        let content = self
            .contents
            .get(ident)
            .unwrap_or_else(|| panic!("unregistered datatype {ident}"));
        (decl, content)
    }

    /// Convert a signature token
    pub fn convert_signature_token(
        &mut self,
        binary: &BinaryIndexedView,
        token: &SignatureToken,
    ) -> TypeRef {
        match token {
            SignatureToken::Bool => TypeRef::Base(TypeTag::Bool),
            SignatureToken::U8 => TypeRef::Base(TypeTag::U8),
            SignatureToken::I8 => TypeRef::Base(TypeTag::I8),
            SignatureToken::U16 => TypeRef::Base(TypeTag::U16),
            SignatureToken::I16 => TypeRef::Base(TypeTag::I16),
            SignatureToken::U32 => TypeRef::Base(TypeTag::U32),
            SignatureToken::I32 => TypeRef::Base(TypeTag::I32),
            SignatureToken::U64 => TypeRef::Base(TypeTag::U64),
            SignatureToken::I64 => TypeRef::Base(TypeTag::I64),
            SignatureToken::U128 => TypeRef::Base(TypeTag::U128),
            SignatureToken::I128 => TypeRef::Base(TypeTag::I128),
            SignatureToken::U256 => TypeRef::Base(TypeTag::U256),
            SignatureToken::I256 => TypeRef::Base(TypeTag::I256),
            SignatureToken::Address => TypeRef::Base(TypeTag::Address),
            SignatureToken::Signer => TypeRef::Base(TypeTag::Signer),
            SignatureToken::Vector(element) => {
                let element_tag = match self.convert_signature_token(binary, element) {
                    TypeRef::Base(tag) => tag,
                    TypeRef::ImmRef(_) | TypeRef::MutRef(_) => {
                        panic!("reference type as vector element is not expected");
                    },
                };
                TypeRef::Base(TypeTag::Vector {
                    element: element_tag.into(),
                })
            },
            SignatureToken::Struct(idx) => {
                let handle = binary.struct_handle_at(*idx);
                let ident = DatatypeIdent::from_struct_handle(binary, handle);

                // first try to see if this is an intrinsic type
                match IntrinsicType::try_parse_ident(&ident) {
                    Some(IntrinsicType::Bitvec) => TypeRef::Base(TypeTag::Bitvec),
                    Some(IntrinsicType::String) => TypeRef::Base(TypeTag::String),
                    Some(IntrinsicType::Object) => {
                        panic!("Object<T> is cannot be `SignatureToken::Struct`");
                    },
                    None => {
                        // not an intrinsic type, locate the datatype
                        let ident = self.ensure_decl_registered(binary, handle);
                        let decl = self.lookup_decl(&ident);
                        assert!(decl.generics.is_empty());
                        TypeRef::Base(TypeTag::Datatype {
                            ident,
                            type_args: vec![],
                        })
                    },
                }
            },
            SignatureToken::StructInstantiation(idx, inst) => {
                let handle = binary.struct_handle_at(*idx);
                let ident = DatatypeIdent::from_struct_handle(binary, handle);

                // convert the type arguments
                let mut ty_args: Vec<_> = inst
                    .iter()
                    .map(|t| match self.convert_signature_token(binary, t) {
                        TypeRef::Base(tag) => tag,
                        TypeRef::ImmRef(_) | TypeRef::MutRef(_) => {
                            panic!("reference type as datatype instantiation is not expected");
                        },
                    })
                    .collect();

                // first try to see if this is an intrinsic type
                match IntrinsicType::try_parse_ident(&ident) {
                    Some(IntrinsicType::Bitvec) | Some(IntrinsicType::String) => {
                        panic!("basic intrinsic type is not expected to be `SignatureToken::StructInstantiation`");
                    },
                    Some(IntrinsicType::Object) => {
                        assert_eq!(ty_args.len(), 1);
                        match ty_args.pop().unwrap() {
                            TypeTag::Datatype { ident, type_args } => {
                                TypeRef::Base(TypeTag::ObjectKnown { ident, type_args })
                            },
                            TypeTag::Param(index) => TypeRef::Base(TypeTag::ObjectParam(index)),
                            _ => panic!("type argument for Object must be a datatype or parameter"),
                        }
                    },
                    None => {
                        // not an intrinsic type, locate the datatype
                        let ident = self.ensure_decl_registered(binary, handle);
                        let decl = self.lookup_decl(&ident);
                        assert_eq!(decl.generics.len(), ty_args.len());
                        TypeRef::Base(TypeTag::Datatype {
                            ident,
                            type_args: ty_args,
                        })
                    },
                }
            },
            SignatureToken::Reference(inner) => {
                let inner_tag = match self.convert_signature_token(binary, inner) {
                    TypeRef::Base(tag) => tag,
                    TypeRef::ImmRef(_) | TypeRef::MutRef(_) => {
                        panic!("reference type behind immutable borrow is not expected");
                    },
                };
                TypeRef::ImmRef(inner_tag)
            },
            SignatureToken::MutableReference(inner) => {
                let inner_tag = match self.convert_signature_token(binary, inner) {
                    TypeRef::Base(tag) => tag,
                    TypeRef::ImmRef(_) | TypeRef::MutRef(_) => {
                        panic!("reference type behind mutable borrow is not expected");
                    },
                };
                TypeRef::MutRef(inner_tag)
            },
            SignatureToken::Function(param_tokens, return_tokens, abilities) => {
                let params = param_tokens
                    .iter()
                    .map(|t| self.convert_signature_token(binary, t))
                    .collect();
                let returns = return_tokens
                    .iter()
                    .map(|t| self.convert_signature_token(binary, t))
                    .collect();
                TypeRef::Base(TypeTag::Function {
                    params,
                    returns,
                    abilities: *abilities,
                })
            },
            SignatureToken::TypeParameter(idx) => TypeRef::Base(TypeTag::Param(*idx as usize)),
        }
    }

    /// Instantiate type parameters in this type tag with the type arguments
    pub fn instantiate_type_tag(&self, tag: &TypeTag, ty_args: &[TypeBase]) -> TypeBase {
        match tag {
            TypeTag::Bool => TypeBase::Bool,
            TypeTag::U8 => TypeBase::U8,
            TypeTag::I8 => TypeBase::I8,
            TypeTag::U16 => TypeBase::U16,
            TypeTag::I16 => TypeBase::I16,
            TypeTag::U32 => TypeBase::U32,
            TypeTag::I32 => TypeBase::I32,
            TypeTag::U64 => TypeBase::U64,
            TypeTag::I64 => TypeBase::I64,
            TypeTag::U128 => TypeBase::U128,
            TypeTag::I128 => TypeBase::I128,
            TypeTag::U256 => TypeBase::U256,
            TypeTag::I256 => TypeBase::I256,
            TypeTag::Bitvec => TypeBase::Bitvec,
            TypeTag::String => TypeBase::String,
            TypeTag::Address => TypeBase::Address,
            TypeTag::Signer => TypeBase::Signer,
            TypeTag::Vector { element } => TypeBase::Vector {
                element: self.instantiate_type_tag(element, ty_args).into(),
            },
            TypeTag::Datatype { ident, type_args } => {
                let decl = self.lookup_decl(ident);
                debug_assert_eq!(type_args.len(), decl.generics.len());

                if type_args.is_empty() {
                    TypeBase::Datatype {
                        ident: ident.clone(),
                        type_args: vec![],
                        abilities: decl.abilities,
                    }
                } else {
                    let ty_args: Vec<_> = type_args
                        .iter()
                        .map(|t| self.instantiate_type_tag(t, ty_args))
                        .collect();
                    let actual_abilities = derive_actual_ability(decl, &ty_args);
                    TypeBase::Datatype {
                        ident: ident.clone(),
                        type_args: ty_args,
                        abilities: actual_abilities,
                    }
                }
            },
            TypeTag::Param(index) => ty_args
                .get(*index)
                .expect("type arguments in bound")
                .clone(),
            TypeTag::ObjectKnown { ident, type_args } => {
                let decl = self.lookup_decl(ident);
                assert_eq!(type_args.len(), decl.generics.len());

                if type_args.is_empty() {
                    TypeBase::ObjectKnown {
                        ident: ident.clone(),
                        type_args: vec![],
                        abilities: decl.abilities,
                    }
                } else {
                    let ty_args: Vec<_> = type_args
                        .iter()
                        .map(|t| self.instantiate_type_tag(t, ty_args))
                        .collect();
                    let actual_abilities = derive_actual_ability(decl, &ty_args);
                    TypeBase::ObjectKnown {
                        ident: ident.clone(),
                        type_args: ty_args,
                        abilities: actual_abilities,
                    }
                }
            },
            TypeTag::ObjectParam(index) => {
                match ty_args.get(*index).expect("type arguments in bound") {
                    TypeBase::Param { index, abilities } => TypeBase::ObjectParam {
                        index: *index,
                        abilities: *abilities,
                    },
                    TypeBase::Datatype {
                        ident,
                        type_args,
                        abilities,
                    } => TypeBase::ObjectKnown {
                        ident: ident.clone(),
                        type_args: type_args.clone(),
                        abilities: *abilities,
                    },
                    _ => panic!("expect a datatype or a parameter as the type argument for object"),
                }
            },
            TypeTag::Function {
                params,
                returns,
                abilities,
            } => TypeBase::Function {
                params: params
                    .iter()
                    .map(|t| self.instantiate_type_ref(t, ty_args))
                    .collect(),
                returns: returns
                    .iter()
                    .map(|t| self.instantiate_type_ref(t, ty_args))
                    .collect(),
                abilities: *abilities,
            },
        }
    }

    /// Instantiate type parameters in this type ref with the type arguments
    pub fn instantiate_type_ref(&self, t: &TypeRef, ty_args: &[TypeBase]) -> TypeItem {
        match t {
            TypeRef::Base(tag) => TypeItem::Base(self.instantiate_type_tag(tag, ty_args)),
            TypeRef::ImmRef(tag) => TypeItem::ImmRef(self.instantiate_type_tag(tag, ty_args)),
            TypeRef::MutRef(tag) => TypeItem::MutRef(self.instantiate_type_tag(tag, ty_args)),
        }
    }
}

/// Utility: derive the actual ability based on type arguments
fn derive_actual_ability(decl: &DatatypeDecl, ty_args: &[TypeBase]) -> AbilitySet {
    let mut provided_abilities = AbilitySet::ALL;
    for (t, (_, is_phantom)) in ty_args.iter().zip(decl.generics.iter()) {
        if *is_phantom {
            continue;
        }
        provided_abilities = provided_abilities.intersect(t.abilities());
    }

    let mut actual_abilities = AbilitySet::EMPTY;
    for ability in decl.abilities.iter() {
        let required = ability.requires();
        if provided_abilities.has_ability(required) {
            actual_abilities = actual_abilities | ability;
        }
    }
    actual_abilities
}

fn merge_pkg_kind(existing: PkgKind, incoming: PkgKind) -> PkgKind {
    match (existing, incoming) {
        (PkgKind::Primary, _) | (_, PkgKind::Primary) => PkgKind::Primary,
        (PkgKind::Dependency, _) | (_, PkgKind::Dependency) => PkgKind::Dependency,
        _ => PkgKind::Framework,
    }
}

#[cfg(test)]
mod tests {
    use super::{derive_actual_ability, DatatypeContent, DatatypeDecl, DatatypeRegistry};
    use crate::{
        deps::PkgKind,
        prep::{
            ident::DatatypeIdent,
            typing::{TypeBase, TypeRef, TypeTag},
        },
    };
    use move_core_types::{
        ability::{Ability, AbilitySet},
        account_address::AccountAddress,
        identifier::Identifier,
    };

    fn datatype(name: &str) -> DatatypeIdent {
        DatatypeIdent::from_struct_tuple(
            AccountAddress::ONE,
            Identifier::new("m").unwrap(),
            Identifier::new(name).unwrap(),
        )
    }

    fn registry_with_decl(
        ident: &DatatypeIdent,
        generics: Vec<(AbilitySet, bool)>,
    ) -> DatatypeRegistry {
        let mut registry = DatatypeRegistry::new();
        registry.decls.insert(ident.clone(), DatatypeDecl {
            ident: ident.clone(),
            generics,
            abilities: AbilitySet::EMPTY.add(Ability::Copy).add(Ability::Drop),
            kind: PkgKind::Primary,
        });
        registry
            .contents
            .insert(ident.clone(), DatatypeContent::Fields(vec![]));
        registry
    }

    #[test]
    fn test_derive_actual_ability_ignores_phantom_arguments() {
        let ident = datatype("Box");
        let decl = DatatypeDecl {
            ident,
            generics: vec![(AbilitySet::EMPTY, false), (AbilitySet::EMPTY, true)],
            abilities: AbilitySet::EMPTY.add(Ability::Copy).add(Ability::Drop),
            kind: PkgKind::Primary,
        };

        let actual = derive_actual_ability(&decl, &[TypeBase::U64, TypeBase::Signer]);
        assert_eq!(
            actual,
            AbilitySet::EMPTY.add(Ability::Copy).add(Ability::Drop)
        );
    }

    #[test]
    fn test_instantiate_type_tag_converts_object_param_to_known_object() {
        let ident = datatype("Vault");
        let registry = registry_with_decl(&ident, vec![]);
        let result =
            registry.instantiate_type_tag(&TypeTag::ObjectParam(0), &[TypeBase::Datatype {
                ident: ident.clone(),
                type_args: vec![],
                abilities: AbilitySet::EMPTY.add(Ability::Copy).add(Ability::Drop),
            }]);

        assert!(matches!(
            result,
            TypeBase::ObjectKnown {
                ident: ref actual,
                type_args,
                ..
            } if actual == &ident && type_args.is_empty()
        ));
    }

    #[test]
    fn test_instantiate_type_ref_preserves_reference_kind() {
        let ident = datatype("Vault");
        let registry = registry_with_decl(&ident, vec![(AbilitySet::EMPTY, false)]);

        let result = registry.instantiate_type_ref(
            &TypeRef::MutRef(TypeTag::Datatype {
                ident: ident.clone(),
                type_args: vec![TypeTag::Param(0)],
            }),
            &[TypeBase::Bool],
        );

        assert!(matches!(
            result,
            crate::prep::typing::TypeItem::MutRef(TypeBase::Datatype {
                ident: ref actual,
                ref type_args,
                ..
            }) if actual == &ident && type_args == &vec![TypeBase::Bool]
        ));
    }

    #[test]
    fn test_opaque_placeholder_can_be_upgraded_to_real_datatype() {
        let ident = datatype("Table");
        let mut registry = DatatypeRegistry::new();

        registry.insert_decl(
            DatatypeDecl {
                ident: ident.clone(),
                generics: vec![(AbilitySet::EMPTY, false)],
                abilities: AbilitySet::EMPTY.add(Ability::Store),
                kind: PkgKind::Dependency,
            },
            false,
        );
        registry.insert_content(ident.clone(), DatatypeContent::Opaque, false);

        registry.insert_decl(
            DatatypeDecl {
                ident: ident.clone(),
                generics: vec![(AbilitySet::EMPTY, false)],
                abilities: AbilitySet::EMPTY.add(Ability::Key),
                kind: PkgKind::Framework,
            },
            true,
        );
        registry.insert_content(ident.clone(), DatatypeContent::Fields(vec![]), true);

        let (decl, content) = registry.lookup_decl_and_content(&ident);
        assert_eq!(decl.kind, PkgKind::Framework);
        assert_eq!(decl.abilities, AbilitySet::EMPTY.add(Ability::Key));
        assert!(matches!(content, DatatypeContent::Fields(fields) if fields.is_empty()));
    }
}
