// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Lint check for `#[view]` public functions that (directly or transitively)
//! call state-mutating global operations (`borrow_global_mut`, `move_to`,
//! `move_from`).
//!
//! View functions are expected to be read-only. A view that mutates state
//! could lead to bugs.
//!
//! The checker lazily precomputes a set of all functions that transitively
//! mutate global state using reverse call-graph propagation. Each
//! `check_function` call then does a simple set lookup.

use move_binary_format::file_format::Bytecode as FileFormatBytecode;
use move_compiler_v2::external_checks::FunctionChecker;
use move_model::{
    ast::{Attribute, ExpData, Operation},
    model::{FunId, FunctionEnv, GlobalEnv, QualifiedId},
    ty::ReferenceKind,
};
use std::{
    cell::OnceCell,
    collections::{BTreeMap, BTreeSet},
};

const CHECKER_NAME: &str = "mutable_view_function";
const VIEW_ATTRIBUTE: &str = "view";

pub struct MutableViewFunction {
    /// Lazily computed set of all functions that (directly or transitively)
    /// call a state-mutating global operation.
    mutating_funs: OnceCell<BTreeSet<QualifiedId<FunId>>>,
}

impl MutableViewFunction {
    pub fn new() -> Self {
        Self {
            mutating_funs: OnceCell::new(),
        }
    }
}

impl FunctionChecker for MutableViewFunction {
    fn get_name(&self) -> String {
        CHECKER_NAME.to_string()
    }

    fn check_function(&self, func: &FunctionEnv) {
        let env = &func.module_env.env;

        let mutating_funs = self
            .mutating_funs
            .get_or_init(|| compute_mutating_funs(env));

        if func.visibility().is_public()
            && has_view_attribute(func)
            && mutating_funs.contains(&func.get_qualified_id())
        {
            let name = func.get_name_str();
            let msg = format!(
                "view function `{name}` should not modify state, but this function \
                 (or one of its callees) calls a state-mutating operation \
                 (`borrow_global_mut`, `move_to`, or `move_from`).",
            );
            self.report(env, &func.get_id_loc(), &msg);
        }
    }
}

fn has_view_attribute(func: &FunctionEnv) -> bool {
    let env = &func.module_env.env;
    let view_sym = env.symbol_pool().make(VIEW_ATTRIBUTE);
    func.has_attribute(|attr| matches!(attr, Attribute::Apply(_, name, _) if *name == view_sym))
}

/// Compute the set of all functions that directly or transitively call a
/// state-mutating global operation.
///
/// Algorithm:
/// 1. Build a reverse call-graph (callee -> set of callers) using
///    `get_called_functions()` which is available for all functions.
/// 2. Seed the worklist with functions that directly mutate global state.
///    - Primary targets (have AST): scan `ExpData` for
///      `BorrowGlobal(Mutable)`, `MoveTo`, `MoveFrom`.
///    - Dependencies (have compiled bytecode): scan `file_format::Bytecode`
///      for `MutBorrowGlobal*`, `MoveTo*`, `MoveFrom*`.
/// 3. Propagate: any caller of a mutating function is also mutating.
fn compute_mutating_funs(env: &GlobalEnv) -> BTreeSet<QualifiedId<FunId>> {
    let mut reverse_callees: BTreeMap<QualifiedId<FunId>, BTreeSet<QualifiedId<FunId>>> =
        BTreeMap::new();
    let mut worklist: Vec<QualifiedId<FunId>> = Vec::new();

    for module in env.get_modules() {
        for func in module.get_functions() {
            let fun_id = func.get_qualified_id();
            if func.is_native() {
                continue;
            }

            if mutates_global_state_directly(&func) {
                worklist.push(fun_id);
            }

            if let Some(callees) = func.get_called_functions() {
                for callee in callees {
                    reverse_callees.entry(*callee).or_default().insert(fun_id);
                }
            }
        }
    }

    let mut mutating_funs = BTreeSet::new();
    while let Some(current) = worklist.pop() {
        if !mutating_funs.insert(current) {
            continue;
        }
        if let Some(callers) = reverse_callees.get(&current) {
            for caller in callers {
                if !mutating_funs.contains(caller) {
                    worklist.push(*caller);
                }
            }
        }
    }

    mutating_funs
}

/// Check if a function directly mutates global state.
///
/// Uses AST scanning for primary targets (which have `get_def()`) and
/// file-format bytecode scanning for dependencies (which have
/// `get_bytecode()` from their pre-compiled module).
fn mutates_global_state_directly(func: &FunctionEnv) -> bool {
    if let Some(def) = func.get_def() {
        let mut found = false;
        def.as_ref().visit_pre_order(&mut |exp| {
            if found {
                return false;
            }
            if let ExpData::Call(_, op, _) = exp
                && matches!(
                    op,
                    Operation::BorrowGlobal(ReferenceKind::Mutable)
                        | Operation::MoveTo
                        | Operation::MoveFrom
                )
            {
                found = true;
                return false;
            }
            true
        });
        return found;
    }

    if let Some(code) = func.get_bytecode() {
        return code.iter().any(|bc| {
            matches!(
                bc,
                FileFormatBytecode::MutBorrowGlobal(_)
                    | FileFormatBytecode::MutBorrowGlobalGeneric(_)
                    | FileFormatBytecode::MoveTo(_)
                    | FileFormatBytecode::MoveToGeneric(_)
                    | FileFormatBytecode::MoveFrom(_)
                    | FileFormatBytecode::MoveFromGeneric(_)
            )
        });
    }

    false
}
