// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Lint check for `#[view]` functions that (directly or transitively)
//! call state-mutating global operations (`borrow_global_mut`, `move_to`,
//! `move_from`).
//!
//! View functions are expected to be read-only. A view that mutates state
//! could lead to bugs.
//!
//! The checker uses recursive memoized traversal starting from view functions,
//! so only functions reachable from view functions are ever visited.

use move_compiler_v2::external_checks::FunctionChecker;
use move_model::{
    ast::{Attribute, ExpData, Operation},
    model::{FunId, FunctionEnv, QualifiedId},
    ty::ReferenceKind,
};
use std::{cell::RefCell, collections::BTreeMap};

const CHECKER_NAME: &str = "mutable_view_function";
const VIEW_ATTRIBUTE: &str = "view";

pub struct MutableViewFunction {
    memo: RefCell<BTreeMap<QualifiedId<FunId>, bool>>,
}

impl MutableViewFunction {
    pub fn new() -> Self {
        Self {
            memo: RefCell::new(BTreeMap::new()),
        }
    }

    fn transitively_mutates(&self, func: &FunctionEnv) -> bool {
        let fun_id = func.get_qualified_id();

        if let Some(&result) = self.memo.borrow().get(&fun_id) {
            return result;
        }

        if func.is_native() {
            self.memo.borrow_mut().insert(fun_id, false);
            return false;
        }

        if mutates_global_state_directly(func) {
            self.memo.borrow_mut().insert(fun_id, true);
            return true;
        }

        // Insert false before recursing to handle cycles.
        self.memo.borrow_mut().insert(fun_id, false);

        let mut result = false;
        if let Some(callees) = func.get_called_functions() {
            let env = &func.module_env.env;
            for callee_id in callees {
                if let Some(callee_func) = env.get_function_opt(*callee_id) {
                    if self.transitively_mutates(&callee_func) {
                        result = true;
                        break;
                    }
                }
            }
        }

        self.memo.borrow_mut().insert(fun_id, result);
        result
    }
}

impl FunctionChecker for MutableViewFunction {
    fn get_name(&self) -> String {
        CHECKER_NAME.to_string()
    }

    fn check_function(&self, func: &FunctionEnv) {
        if has_view_attribute(func) && self.transitively_mutates(func) {
            let env = &func.module_env.env;
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

/// Check if a function directly mutates global state by scanning its AST.
fn mutates_global_state_directly(func: &FunctionEnv) -> bool {
    func.get_def().is_some_and(|def| {
        def.any(&mut |exp| {
            matches!(
                exp,
                ExpData::Call(
                    _,
                    Operation::BorrowGlobal(ReferenceKind::Mutable)
                        | Operation::MoveTo
                        | Operation::MoveFrom,
                    _
                )
            )
        })
    })
}
