// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Reject literal patterns that appear outside of `match` arms.
//!
//! Literal patterns are only meaningful inside match expressions.
//! In let bindings and lambda parameters they are refutable patterns
//! in irrefutable contexts, so we reject them here.
//!
//! This pass runs on the model AST before match coverage checks and
//! match transforms, walking every function body and reporting an
//! error for any `Pattern::LiteralValue` that appears under a `Block`
//! (let binding) or `Lambda` node rather than under a `Match` node.

use move_model::{
    ast::{ExpData, Pattern},
    model::GlobalEnv,
};

/// Check all target functions for literal patterns outside match
/// arms.
pub fn check(env: &GlobalEnv) {
    for module in env.get_modules() {
        if module.is_target() {
            for func in module.get_functions() {
                if let Some(def) = func.get_def() {
                    check_exp(env, def);
                }
            }
        }
    }
}

/// Walk an expression tree, checking patterns on `Block` (let
/// bindings) and `Lambda` nodes for illegal literal patterns.
/// `visit_pre_order` only visits `ExpData` nodes, not `Pattern`
/// nodes, so match arm patterns are naturally skipped while guards
/// and bodies are still traversed.
fn check_exp(env: &GlobalEnv, exp: &move_model::ast::Exp) {
    exp.visit_pre_order(&mut |e| {
        match e {
            ExpData::Block(_, pat, _, _) | ExpData::Lambda(_, pat, _, _, _) => {
                check_no_literal(env, pat);
            },
            _ => {},
        }
        true // always continue traversal
    });
}

/// Report an error for every `Pattern::LiteralValue` reachable in
/// `pat`.
fn check_no_literal(env: &GlobalEnv, pat: &Pattern) {
    pat.visit_pre_post(&mut |is_post, p| {
        if !is_post {
            if let Pattern::LiteralValue(id, _) = p {
                env.error(
                    &env.get_node_loc(*id),
                    "literals are not allowed here",
                );
            }
        }
    });
}
