// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

//! Implements an AST pass that checks any struct `S` defined in the current module
//! cannot contain descendant of type `S`.
//! Preconditon: Cyclic module dependency has been checked.

use move_model::{
    model::{FieldEnv, GlobalEnv, Loc, ModuleEnv, StructEnv, StructId},
    symbol::Symbol,
    ty::Type,
};
use std::{collections::BTreeSet, rc::Rc};

/// Checks all modules in `env`
pub fn check_recursive_struct(env: &GlobalEnv) {
    for module in env.get_modules() {
        if module.is_target() {
            let checker = RecursiveStructChecker::new(module);
            checker.check();
        }
    }
}

/// Module checker state
struct RecursiveStructChecker<'a> {
    /// The module we are checking
    mod_env: ModuleEnv<'a>,
}

impl<'a> RecursiveStructChecker<'a> {
    pub fn new(mod_env: ModuleEnv<'a>) -> Self {
        Self { mod_env }
    }

    /// Checks all structs in the module
    fn check(&self) {
        let mut checked = BTreeSet::new();
        for struct_env in self.mod_env.get_structs() {
            self.check_struct(struct_env.get_id(), &mut checked);
        }
    }

    /// Checks the given struct and marks it as checked
    fn check_struct(&self, struct_id: StructId, checked: &mut BTreeSet<StructId>) {
        if !checked.contains(&struct_id) {
            let mut path = Vec::new();
            let struct_loc = self.mod_env.get_struct(struct_id).get_loc();
            self.check_struct_as_required_by(&mut path, struct_id, struct_loc, checked);
        }
    }

    /// Checks the given struct, and marks it as checked.
    /// `path`: checking the struct `path[i]` requires checking the struct `path[i + 1]`,
    /// because `path[i + 1]` has not been checked when checking `path[i]`,
    /// and is the field `path[i].1` at location `path[i].0`;
    /// checking the last element of `path[i]` (if exists) requires checking `struct_id`.
    /// We keep this parameter to avoid infinite recursion and for the diagnostics.
    /// precondition: the given struct has not been checked
    fn check_struct_as_required_by(
        &self,
        path: &mut Vec<(Loc, Symbol, StructId)>,
        struct_id: StructId,
        loc: Loc,
        checked: &mut BTreeSet<StructId>,
    ) {
        // check for cyclic dependencies
        for (i, (_ancestor_loc, _ancestor_name, ancestor)) in path.iter().enumerate() {
            if *ancestor == struct_id {
                self.report_cycle(&path[i..], struct_id);
                checked.insert(struct_id);
                return;
            }
        }
        let struct_env = self.mod_env.get_struct(struct_id);
        for field_env in struct_env.get_fields() {
            let field_name = field_env.get_name();
            path.push((loc.clone(), field_name, struct_id));
            match field_env.get_type() {
                Type::Struct(field_mod_id, field_struct_id, insts) => {
                    // check the field struct if it's not been checked, so that we only need to look at
                    // the type parameters later
                    if field_mod_id == self.mod_env.get_id() && !checked.contains(&field_struct_id)
                    {
                        self.check_struct_as_required_by(
                            path,
                            field_struct_id,
                            field_env.get_loc().clone(),
                            checked,
                        );
                    }
                    // check the type parameters of the fields cannot contain the struct we are checking
                    if insts.iter().any(|ty| {
                        self.ty_contains_struct(path, ty, loc.clone(), struct_id, checked)
                    }) {
                        self.report_invalid_field(&struct_env, &field_env);
                    }
                },
                Type::Vector(ty) => {
                    if self.ty_contains_struct(path, &ty, loc.clone(), struct_id, checked) {
                        self.report_invalid_field(&struct_env, &field_env);
                    }
                },
                Type::Fun(arg_ty, ret_ty, _) => {
                    if self.ty_contains_struct(path, &arg_ty, loc.clone(), struct_id, checked) {
                        self.report_invalid_field(&struct_env, &field_env);
                    }
                    if self.ty_contains_struct(path, &ret_ty, loc.clone(), struct_id, checked) {
                        self.report_invalid_field(&struct_env, &field_env);
                    }
                },
                Type::Reference(_, ty) => {
                    if self.ty_contains_struct(path, &ty, loc.clone(), struct_id, checked) {
                        self.report_invalid_field(&struct_env, &field_env);
                    }
                },
                Type::Primitive(_) | Type::TypeParameter(_) => {},
                _ => unreachable!("invalid field type"),
            }
            path.pop();
        }
        checked.insert(struct_id);
    }

    fn report_invalid_field(&self, struct_env: &StructEnv, field_env: &FieldEnv) {
        let struct_name = self.get_struct_name(struct_env.get_id());
        let note = format!(
            "invalid field `{}` of `{}` containing `{}` itself",
            self.mod_env.symbol_pool().string(field_env.get_name()),
            struct_name,
            struct_name,
        );
        self.mod_env
            .env
            .error_with_labels(&struct_env.get_loc(), "cyclic data", vec![(
                field_env.get_loc().clone(),
                note,
            )]);
    }

    /// Report cyclic dependency of structs
    /// `path`: `path[i]` contains field of type `path[i + 1]`,
    /// and the last element contains field of type `struct_id`
    /// precondition: `path` not empty, and `path[0] == struct_id`
    fn report_cycle(&self, path: &[(Loc, Symbol, StructId)], struct_id: StructId) {
        let notes = self.gen_error_msg_for_fields_cycle(path, struct_id);
        let root_struct_loc = self.mod_env.get_struct(path[0].2).get_loc();
        self.mod_env
            .env
            .error_with_notes(&root_struct_loc, "cyclic data", notes);
    }

    fn get_struct_name(&self, struct_id: StructId) -> Rc<String> {
        let symbol_pool = self.mod_env.symbol_pool();
        let struct_name = self.mod_env.get_struct(struct_id).get_name();
        symbol_pool.string(struct_name)
    }

    /// Generates error messages for recursive structs
    /// Parameters same as `report_cycle`
    fn gen_error_msg_for_fields_cycle(
        &self,
        path: &[(Loc, Symbol, StructId)],
        struct_id: StructId,
    ) -> Vec<String> {
        let mut cycle_note = Vec::new();
        for i in 0..path.len() - 1 {
            let parent_name = self.get_struct_name(path[i].2);
            let field_name = self.mod_env.symbol_pool().string(path[i].1);
            let child_name = self.get_struct_name(path[i + 1].2);
            cycle_note.push(format!(
                "field `{}` of `{}` contains `{}`",
                field_name, parent_name, child_name,
            ));
        }
        cycle_note.push(format!(
            "field `{}` of `{}` contains `{}`, which forms a cycle.",
            self.mod_env
                .symbol_pool()
                .string(path.last().expect("parent").1),
            self.get_struct_name(path.last().expect("parent").2),
            self.get_struct_name(struct_id),
        ));
        cycle_note
    }

    /// Checks whether the given type contains the given struct.
    fn ty_contains_struct(
        &self,
        path: &mut Vec<(Loc, Symbol, StructId)>,
        ty: &Type,
        loc: Loc,
        struct_id: StructId,
        checked: &mut BTreeSet<StructId>,
    ) -> bool {
        match ty {
            Type::Vector(ty) => self.ty_contains_struct(path, ty, loc, struct_id, checked),
            Type::Struct(mid, sid, insts) => {
                if *mid == self.mod_env.get_id() {
                    if *sid == struct_id {
                        return true;
                    }
                    if !checked.contains(sid) {
                        self.check_struct_as_required_by(path, *sid, loc.clone(), checked);
                    }
                }
                insts
                    .iter()
                    .any(|ty| self.ty_contains_struct(path, ty, loc.clone(), struct_id, checked))
            },
            Type::Tuple(ty_vec) => {
                for ty in ty_vec.iter() {
                    if self.ty_contains_struct(path, ty, loc.clone(), struct_id, checked) {
                        return true;
                    }
                }
                false
            },
            Type::Fun(arg_ty, ret_ty, _abi) => {
                self.ty_contains_struct(path, arg_ty, loc.clone(), struct_id, checked)
                    || self.ty_contains_struct(path, ret_ty, loc, struct_id, checked)
            },
            Type::Reference(_, ty) => self.ty_contains_struct(path, ty, loc, struct_id, checked),
            Type::Primitive(_) | Type::TypeParameter(_) => false,
            _ => panic!("ICE: {:?} used as a type parameter", ty),
        }
    }
}
