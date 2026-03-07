// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Reject literal patterns in unsupported positions.
//!
//! Literal patterns are currently supported only at the top level of
//! match arms or inside top-level tuples, where match transforms handle
//! them. They are NOT supported:
//!
//! 1. In let bindings or lambda parameters (refutable patterns in
//!    irrefutable contexts).
//! 2. Nested inside struct/enum variant patterns in match arms (not
//!    yet handled by match transforms).
//!
//! This pass runs on the model AST before match coverage checks and
//! match transforms.

use move_model::{
    ast::{ExpData, Pattern},
    model::GlobalEnv,
};

/// Check all target functions for literal patterns in unsupported
/// positions.
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

/// Walk an expression tree.  `visit_pre_order` only visits `ExpData`
/// nodes, not `Pattern` nodes, so we intercept the relevant
/// expression kinds and inspect their patterns explicitly.
fn check_exp(env: &GlobalEnv, exp: &move_model::ast::Exp) {
    exp.visit_pre_order(&mut |e| {
        match e {
            ExpData::Block(_, pat, _, _) | ExpData::Lambda(_, pat, _, _, _) => {
                check_no_literal(env, pat);
            },
            ExpData::Match(_, _, arms) => {
                for arm in arms {
                    check_match_arm_pattern(env, &arm.pattern);
                }
            },
            _ => {},
        }
        true // always continue traversal
    });
}

/// Check a single match arm pattern. Top-level literals and
/// wildcards/vars are fine. For tuples, recurse into each element
/// (still top-level context). For structs/enums, any nested literal
/// is invalid.
fn check_match_arm_pattern(env: &GlobalEnv, pat: &Pattern) {
    match pat {
        Pattern::LiteralValue(..)
        | Pattern::Var(..)
        | Pattern::Wildcard(..)
        | Pattern::Error(..) => {},
        Pattern::Tuple(_, pats) => {
            for p in pats {
                check_match_arm_pattern(env, p);
            }
        },
        Pattern::Struct(_, _, _, pats) => {
            for p in pats {
                check_no_literal(env, p);
            }
        },
    }
}

/// Report an error for every `Pattern::LiteralValue` reachable in
/// `pat`.
fn check_no_literal(env: &GlobalEnv, pat: &Pattern) {
    pat.visit_pre_post(&mut |is_post, p| {
        if !is_post {
            if let Pattern::LiteralValue(id, _) = p {
                env.error(&env.get_node_loc(*id), "literals are not allowed here");
            }
        }
    });
}
