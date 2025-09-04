// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

//! Checks pureness (absence of side-effects) of expressions.
//! An expression is pure if
//!
//! - It does not use borrow_global_mut or `&mut e`.
//! - It does not use Mutate
//! - It does not call another impure function.
//!
//! In specification checking mode, in addition the following constructs are disallowed
//!
//! - No use of Assign
//! - Not use of Return
//! - No use of uninitialized let bindings
//!
//! The checker does a DFS search to figure whether transitive call chains are pure or not.

use crate::{
    ast::{Exp, ExpData, Operation, Spec},
    model::{FunId, GlobalEnv, NodeId, Parameter, QualifiedId},
    ty::ReferenceKind,
};
use std::{collections::BTreeMap, mem};

#[derive(Debug, PartialEq, Eq)]
pub enum FunctionPurenessCheckerMode {
    /// General checking mode, determining semantic impureness
    General,
    /// In addition to the general rules, specification specific rules
    Specification,
}

/// Pureness checker for expressions.
#[derive(Debug)]
pub struct FunctionPurenessChecker<F>
where
    F: FnMut(NodeId, &str, &[(QualifiedId<FunId>, NodeId)]),
{
    /// The checking mode
    mode: FunctionPurenessCheckerMode,
    /// An action which is invoked if impurity is detected. The first argument is
    /// the node where the issue was found, the 2nd a message explaining the issue,
    /// and the 3rd a stack of calls to functions which are currently analyzed, with
    /// the first element in the vector the most outer call.
    impure_action: F,
    /// Map from functions to their known pureness status
    pureness: BTreeMap<QualifiedId<FunId>, bool>,
    /// Stack of functions currently visiting
    visiting: Vec<(QualifiedId<FunId>, NodeId)>,
    /// Whether the expression visited so far is impure
    is_impure: bool,
}

impl<F> FunctionPurenessChecker<F>
where
    F: FnMut(NodeId, &str, &[(QualifiedId<FunId>, NodeId)]),
{
    /// Creates a new checker. The given function is invoke with diagnostic information
    /// if impurity is detected. It is up to this function whether an actual error is
    /// reported.
    pub fn new(mode: FunctionPurenessCheckerMode, impure_action: F) -> Self {
        Self {
            mode,
            impure_action,
            pureness: BTreeMap::default(),
            visiting: vec![],
            is_impure: false,
        }
    }

    /// Consumes the checker and returns a map from qualified function
    /// names to a boolean indicating whether they are pure (= true).
    pub fn into_map(self) -> BTreeMap<QualifiedId<FunId>, bool> {
        self.pureness
    }

    /// Checks whether the given expression is pure and returns true if so.
    pub fn check_exp(&mut self, env: &GlobalEnv, exp: &Exp) -> bool {
        // Reset before start of traversal
        self.is_impure = false;
        exp.visit_post_order(&mut |e| {
            use ExpData::*;
            use Operation::*;
            match e {
                Assign(id, ..) if self.mode == FunctionPurenessCheckerMode::Specification => {
                    (self.impure_action)(*id, "assigns variable", &self.visiting);
                    self.is_impure = true
                },
                Mutate(id, ..) => {
                    (self.impure_action)(*id, "mutates reference", &self.visiting);
                    self.is_impure = true;
                },
                Return(id, ..) if self.mode == FunctionPurenessCheckerMode::Specification => {
                    (self.impure_action)(
                        *id,
                        "return not allowed in specifications",
                        &self.visiting,
                    );
                },
                Block(id, _, None, _)
                    if self.mode == FunctionPurenessCheckerMode::Specification =>
                {
                    (self.impure_action)(
                        *id,
                        "uninitialized let not allowed in specifications",
                        &self.visiting,
                    );
                },
                Call(id, Borrow(ReferenceKind::Mutable), ..) => {
                    (self.impure_action)(*id, "mutably borrows value", &self.visiting);
                    self.is_impure = true;
                },
                Call(id, BorrowGlobal(ReferenceKind::Mutable), ..) => {
                    (self.impure_action)(
                        *id,
                        "mutably borrows from global storage",
                        &self.visiting,
                    );
                    self.is_impure = true;
                },
                Call(id, MoveFunction(mid, sid), ..) => {
                    let qid = mid.qualified(*sid);
                    // false positive: can't use entry because of borrow conflict
                    #[allow(clippy::map_entry)]
                    if !self.pureness.contains_key(&qid) {
                        self.visiting.push((qid, *id));
                        let old_impure = mem::take(&mut self.is_impure);
                        self.check_function(env, qid);
                        self.pureness.insert(qid, !self.is_impure);
                        self.visiting.pop();
                        self.is_impure |= old_impure;
                    }
                    if !self.pureness.get(&qid).unwrap() {
                        (self.impure_action)(
                            *id,
                            "calls a function which modifies state",
                            &self.visiting,
                        );
                        self.is_impure = true
                    }
                },
                _ => {},
            }
            // Stop traversal if we have shown the expression is impure
            !self.is_impure
        });
        !self.is_impure
    }

    /// Checks all the expressions in a spec block.
    pub fn check_spec(&mut self, env: &GlobalEnv, spec: &Spec) {
        // We map this to checking an expression, leveraging the existing
        // SpecBlock visitor logic.
        let spec_exp = ExpData::SpecBlock(env.new_node_id(), spec.clone());
        self.check_exp(env, &spec_exp.into_exp());
    }

    fn check_function(&mut self, env: &GlobalEnv, qid: QualifiedId<FunId>) {
        let fun = env.get_function(qid);
        if let Some(def) = fun.get_def() {
            // For breaking cycles, assume initially function is pure
            self.pureness.insert(qid, true);
            // Continue recursively
            self.check_exp(env, def);
        } else {
            // We consider a native as pure if it does not take or deliver a mutable reference.
            self.is_impure = fun
                .get_parameters()
                .iter()
                .any(|Parameter(_, ty, _)| ty.is_mutable_reference())
                || fun
                    .get_result_type()
                    .flatten()
                    .iter()
                    .any(|ty| ty.is_mutable_reference());
        }
    }
}
