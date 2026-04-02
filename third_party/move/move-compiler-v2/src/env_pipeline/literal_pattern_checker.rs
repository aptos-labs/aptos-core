// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Reject literal patterns in unsupported positions.
//!
//! Literal patterns are currently supported in match arms:
//!
//! 1. At the top level.
//! 2. Inside top-level tuples.
//! 3. Nested inside struct/enum variant patterns.
//!
//! They are NOT supported in let bindings or lambda parameters
//! (refutable patterns in irrefutable contexts).
//!
//! This pass runs on the model AST before match coverage checks and
//! match transforms.

use move_model::{
    ast::{ExpData, Pattern},
    metadata::lang_feature_versions::LANGUAGE_VERSION_FOR_PRIMITIVE_MATCH,
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
                check_no_literal(env, pat, "literals are not allowed here");
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
/// (still top-level context). For structs/enums, nested literals are
/// allowed starting from `LANGUAGE_VERSION_FOR_PRIMITIVE_MATCH`;
/// on earlier versions any nested literal is invalid.
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
            if env
                .language_version()
                .is_at_least(LANGUAGE_VERSION_FOR_PRIMITIVE_MATCH)
            {
                for p in pats {
                    check_match_arm_pattern(env, p);
                }
            } else {
                for p in pats {
                    check_no_literal(env, p, &format!(
                        "literal patterns inside struct/enum variants require language version {} or later",
                        LANGUAGE_VERSION_FOR_PRIMITIVE_MATCH
                    ));
                }
            }
        },
    }
}

/// Report an error for every `Pattern::LiteralValue` reachable in
/// `pat`.
fn check_no_literal(env: &GlobalEnv, pat: &Pattern, msg: &str) {
    pat.visit_pre_post(&mut |is_post, p| {
        if !is_post {
            if let Pattern::LiteralValue(id, _) = p {
                env.error(&env.get_node_loc(*id), msg);
            }
        }
    });
}
