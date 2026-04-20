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
    /// `None`  = in-progress or provisional (cycle participant, result uncertain).
    /// `Some(b)` = finalized result.
    memo: RefCell<BTreeMap<QualifiedId<FunId>, Option<bool>>>,
}

impl MutableViewFunction {
    pub fn new() -> Self {
        Self {
            memo: RefCell::new(BTreeMap::new()),
        }
    }

    fn transitively_mutates(&self, func: &FunctionEnv) -> bool {
        self.transitively_mutates_inner(func).0
    }

    /// Returns `(mutates, hit_cycle)`.
    ///
    /// `hit_cycle` is true when the subtree traversed a back-edge to a
    /// `None` (in-progress) memo entry.  A `false` result with
    /// `hit_cycle = true` is provisional -- the node stays as `None` in
    /// the memo so it can be recomputed once its cycle partners are
    /// resolved.
    fn transitively_mutates_inner(&self, func: &FunctionEnv) -> (bool, bool) {
        let fun_id = func.get_qualified_id();

        match self.memo.borrow().get(&fun_id).copied() {
            Some(Some(result)) => return (result, false),
            Some(None) => return (false, true),
            None => {},
        }

        if func.is_native() {
            self.memo.borrow_mut().insert(fun_id, Some(false));
            return (false, false);
        }

        if mutates_global_state_directly(func) {
            self.memo.borrow_mut().insert(fun_id, Some(true));
            return (true, false);
        }

        self.memo.borrow_mut().insert(fun_id, None);

        let env = &func.module_env.env;
        let mut result = false;
        let mut any_cycle = false;
        for &callee_id in func.get_called_functions().into_iter().flatten() {
            let (r, c) = self.transitively_mutates_inner(&env.get_function(callee_id));
            any_cycle |= c;
            if r {
                result = true;
                break;
            }
        }

        // Only memoize `false` when no cycle back-edge was traversed;
        // otherwise a cycle partner may later resolve to true.
        if result || !any_cycle {
            self.memo.borrow_mut().insert(fun_id, Some(result));
        }

        (result, any_cycle)
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
                "view function `{name}` performs global state mutations (via `borrow_global_mut`, \
                 `move_to`, or `move_from`), either directly or through a callee. \
                 These mutations are silently discarded when called via the view API, \
                 but applied when called from Move code. This inconsistency can lead to \
                 unexpected behavior, reconsider the implementation.",
            );
            self.report(env, &func.get_id_loc(), &msg);
        }
        // Discard provisional (None) entries so cycle participants
        // are recomputed fresh for subsequent view function checks.
        self.memo.borrow_mut().retain(|_, v| v.is_some());
    }
}

fn has_view_attribute(func: &FunctionEnv) -> bool {
    let env = &func.module_env.env;
    let view_sym = env.symbol_pool().make(VIEW_ATTRIBUTE);
    func.has_attribute(|attr| matches!(attr, Attribute::Apply(_, name, _) if *name == view_sym))
}

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
