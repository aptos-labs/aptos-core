// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Loads resolved XIR declarations into the Move model.
//!
//! JSON decoding and stackless function bodies remain frontend concerns. This
//! loader owns the declaration-to-model boundary and shares the runtime data
//! constructors used by the binary module loader.

use crate::{
    ast::{Attribute, FriendDecl, ModuleName, Spec},
    model::{
        FieldData, FieldId, FunId, FunctionData, FunctionKind, GlobalEnv, Loc, Parameter,
        QualifiedId, StructData, StructId, StructVariant, TypeParameter,
    },
    symbol::Symbol,
    ty::Type,
};
use anyhow::{ensure, Result};
use move_binary_format::file_format::Visibility;
use move_core_types::ability::AbilitySet;
use std::collections::{BTreeMap, BTreeSet};

pub struct XirModuleData {
    pub loc: Loc,
    pub name: ModuleName,
    pub structs: Vec<XirStructData>,
    pub functions: Vec<XirFunctionData>,
    /// Modules this one grants friend access to. A `Friend`-visible
    /// declaration says only that it *is* friend-visible; this says to whom.
    pub friends: Vec<FriendDecl>,
}

pub struct XirStructData {
    pub name: Symbol,
    pub loc: Loc,
    pub abilities: AbilitySet,
    pub type_parameters: Vec<TypeParameter>,
    pub fields: Vec<FieldData>,
    pub variants: Option<Vec<XirVariantData>>,
    pub visibility: Visibility,
    pub attributes: Vec<Attribute>,
}

pub struct XirVariantData {
    pub name: Symbol,
    pub loc: Loc,
}

pub struct XirFunctionData {
    pub name: Symbol,
    pub loc: Loc,
    pub visibility: Visibility,
    pub is_native: bool,
    pub kind: FunctionKind,
    pub attributes: Vec<Attribute>,
    pub type_parameters: Vec<TypeParameter>,
    pub params: Vec<Parameter>,
    pub result_type: Type,
    pub acquired_structs: BTreeSet<StructId>,
    pub called_funs: BTreeSet<QualifiedId<FunId>>,
}

impl GlobalEnv {
    /// Adds a resolved XIR module declaration to this environment.
    pub fn load_xir_module(&mut self, module: XirModuleData) -> Result<crate::model::ModuleId> {
        ensure!(
            self.find_module(&module.name).is_none(),
            "duplicate module `{}`",
            module.name.display(self)
        );
        let mut structs = BTreeMap::new();
        for decl in module.structs {
            let id = StructId::new(decl.name);
            let mut fields = BTreeMap::new();
            for field in decl.fields {
                let field_id = if let Some(variant) = field.variant {
                    let pool = self.symbol_pool();
                    FieldId::new(pool.make(&FieldId::make_variant_field_id_str(
                        pool.string(variant).as_str(),
                        pool.string(field.name).as_str(),
                    )))
                } else {
                    FieldId::new(field.name)
                };
                ensure!(
                    fields.insert(field_id, field).is_none(),
                    "duplicate field in XIR struct"
                );
            }
            let variants = decl.variants.map(|variants| {
                variants
                    .into_iter()
                    .enumerate()
                    .map(|(order, variant)| {
                        (variant.name, StructVariant {
                            loc: variant.loc,
                            attributes: vec![],
                            order,
                        })
                    })
                    .collect()
            });
            ensure!(
                structs
                    .insert(
                        id,
                        StructData::new_runtime(
                            decl.name,
                            decl.loc,
                            decl.abilities,
                            decl.type_parameters,
                            fields,
                            variants,
                            false,
                            decl.visibility,
                            decl.attributes,
                        ),
                    )
                    .is_none(),
                "duplicate XIR struct declaration"
            );
        }

        let mut functions = BTreeMap::new();
        for decl in module.functions {
            let id = FunId::new(decl.name);
            ensure!(
                functions
                    .insert(
                        id,
                        FunctionData::new_runtime(
                            decl.name,
                            decl.loc,
                            decl.visibility,
                            decl.is_native,
                            decl.kind,
                            decl.attributes,
                            decl.type_parameters,
                            decl.params,
                            decl.result_type,
                            None,
                            Some(decl.acquired_structs),
                            Some(decl.called_funs),
                        ),
                    )
                    .is_none(),
                "duplicate XIR function declaration"
            );
        }

        // Resolve friend declarations against the modules already loaded.
        //
        // Source modules get this from `check_and_update_friend_info`, which
        // runs at the end of the model builder — after every module has an id.
        // An XIR module is added later than that, so it never passes through
        // it, and `GlobalEnv::add` leaves `friend_modules` empty. Since
        // file-format generation serializes friends from `friend_modules` and
        // not from `friend_decls`, skipping this would silently drop every
        // friend declaration from the bytecode of an XIR *target*.
        let mut friend_decls = module.friends;
        let mut friend_modules = BTreeSet::new();
        for friend_decl in friend_decls.iter_mut() {
            if let Some(friend) = self.find_module(&friend_decl.module_name) {
                friend_decl.module_id = Some(friend.get_id());
                friend_modules.insert(friend.get_id());
            }
        }

        let module_id = self.add(
            module.loc,
            module.name,
            vec![],
            vec![],
            friend_decls,
            BTreeMap::new(),
            structs,
            functions,
            vec![],
            vec![],
            vec![],
            Spec::default(),
            vec![],
        );
        // Unlike the source path, an unresolved friend is not an error here.
        // XIR modules are loaded one at a time and a friend is typically a
        // *dependent*, which need not be present — and an absent friend only
        // withholds an access grant, so being lenient cannot widen access.
        self.get_module_data_mut(module_id).friend_modules = friend_modules;
        Ok(module_id)
    }
}
