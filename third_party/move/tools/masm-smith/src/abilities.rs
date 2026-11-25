// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Computes which abilities (`copy`, `drop`, `store`, `key`) a type has, by Move's rules. Generic
//! types follow `AbilitySet::polymorphic_abilities`.

use crate::ir::{Module, StructDecl, Type};
use move_core_types::{
    ability::{Ability, AbilitySet},
    language_storage::ModuleId,
};

pub struct AbilityCtx<'a> {
    /// Identity of the module containing the *local* struct decls in `structs`. When `None`
    /// (e.g. during early struct-decl construction), local lookups are disabled.
    pub local_module_id: Option<&'a ModuleId>,
    /// Local struct declarations (this module). Looked up by name when `qs.module` matches
    /// `local_module_id`.
    pub structs: &'a [StructDecl],
    /// Already-published modules whose structs may appear in type positions. Looked up by
    /// `qs.module` then by `name` for foreign-struct ability resolution.
    pub external_modules: &'a [Module],
    pub type_param_constraints: &'a [AbilitySet],
}

impl<'a> AbilityCtx<'a> {
    /// Resolve a struct reference to its declaration, searching local then external pools.
    pub fn lookup_struct(&self, qs: &crate::ir::QualifiedStruct) -> Option<&'a StructDecl> {
        if self.local_module_id == Some(&qs.module) {
            return self.structs.iter().find(|d| d.name == qs.name);
        }
        let m = self.external_modules.iter().find(|m| m.id == qs.module)?;
        m.structs.iter().find(|d| d.name == qs.name)
    }
}

pub fn type_abilities(ty: &Type, ctx: &AbilityCtx<'_>) -> AbilitySet {
    match ty {
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
        Type::Vector(inner) => {
            // `vector<T>` is declared as `has copy, drop, store` parameterized over T.
            AbilitySet::polymorphic_abilities(
                AbilitySet::VECTOR,
                std::iter::once(false),
                std::iter::once(type_abilities(inner, ctx)),
            )
            .unwrap_or(AbilitySet::EMPTY)
        },
        Type::Reference(_, _) => AbilitySet::REFERENCES,
        Type::TypeParam(idx) => ctx
            .type_param_constraints
            .get(*idx as usize)
            .copied()
            .unwrap_or(AbilitySet::EMPTY),
        Type::Function { abilities, .. } => *abilities,
        Type::Struct(qs, type_args) => {
            let Some(decl) = ctx.lookup_struct(qs) else {
                return AbilitySet::EMPTY;
            };
            let phantoms: Vec<bool> = decl.type_params.iter().map(|p| p.is_phantom).collect();
            let args: Vec<AbilitySet> = type_args.iter().map(|t| type_abilities(t, ctx)).collect();
            if phantoms.len() != args.len() {
                return AbilitySet::EMPTY;
            }
            AbilitySet::polymorphic_abilities(decl.abilities, phantoms, args)
                .unwrap_or(AbilitySet::EMPTY)
        },
    }
}

pub fn has_drop(ty: &Type, ctx: &AbilityCtx<'_>) -> bool {
    type_abilities(ty, ctx).has_drop()
}

pub fn has_copy(ty: &Type, ctx: &AbilityCtx<'_>) -> bool {
    type_abilities(ty, ctx).has_copy()
}

pub fn has_key(ty: &Type, ctx: &AbilityCtx<'_>) -> bool {
    type_abilities(ty, ctx).has_key()
}

/// Returns the largest set of declared abilities supported by every field in the layout.
pub fn implied_struct_abilities(decl: &StructDecl, ctx: &AbilityCtx<'_>) -> AbilitySet {
    let mut implied = AbilitySet::ALL;
    for f in decl.layout.all_fields() {
        let f_abilities = type_abilities(&f.ty, ctx);
        // Each non-key declared ability must be supported by the field type. `Key`, however,
        // requires `Store` on each field — that's the standard Move rule.
        let mut field_capability = f_abilities;
        if !f_abilities.has_ability(Ability::Store) {
            field_capability = field_capability.remove(Ability::Key);
        }
        // Re-add `Key` if `Store` is present on the field; `Key` itself isn't required on
        // field types, only `Store`.
        if f_abilities.has_ability(Ability::Store) {
            field_capability = field_capability.add(Ability::Key);
        }
        implied = implied.intersect(field_capability);
    }
    implied
}
