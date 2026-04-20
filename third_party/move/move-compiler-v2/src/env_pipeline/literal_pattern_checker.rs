// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Reject literal and range patterns in unsupported positions.
//!
//! Literal and range patterns are currently supported in match arms:
//!
//! 1. At the top level.
//! 2. Inside top-level tuples.
//! 3. Nested inside struct/enum variant patterns.
//!
//! They are NOT supported in let bindings or lambda parameters
//! (refutable patterns in irrefutable contexts).
//!
//! This pass also validates range bounds (empty/inverted ranges).
//!
//! This pass runs on the model AST before match coverage checks and
//! match transforms.

use move_model::{
    ast::{ExpData, Pattern},
    metadata::lang_feature_versions::LANGUAGE_VERSION_FOR_PRIMITIVE_MATCH,
    model::{GlobalEnv, NodeId},
    ty::{PrimitiveType, Type},
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
                check_no_literal_or_range(
                    env,
                    pat,
                    "literal and range patterns are not allowed here",
                );
            },
            ExpData::Match(_, _, arms) => {
                for arm in arms {
                    check_match_arm_pattern(env, &arm.pattern);
                    check_range_validity(env, &arm.pattern);
                }
            },
            _ => {},
        }
        true // always continue traversal
    });
}

/// Check a single match arm pattern. Top-level literals/ranges and
/// wildcards/vars are fine. For tuples, recurse into each element
/// (still top-level context). For structs/enums, nested literals/ranges are
/// allowed starting from `LANGUAGE_VERSION_FOR_PRIMITIVE_MATCH`;
/// on earlier versions any nested literal/range is invalid.
fn check_match_arm_pattern(env: &GlobalEnv, pat: &Pattern) {
    match pat {
        Pattern::LiteralValue(..)
        | Pattern::Range(..)
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
                    check_no_literal_or_range(env, p, &format!(
                        "literal and range patterns inside struct/enum variants require language version {} or later",
                        LANGUAGE_VERSION_FOR_PRIMITIVE_MATCH
                    ));
                }
            }
        },
    }
}

/// Report an error for every `Pattern::LiteralValue` or `Pattern::Range`
/// reachable in `pat`.
fn check_no_literal_or_range(env: &GlobalEnv, pat: &Pattern, msg: &str) {
    pat.visit_pre_post(&mut |is_post, p| {
        if !is_post {
            match p {
                Pattern::LiteralValue(id, _) | Pattern::Range(id, _, _, _) => {
                    env.error(&env.get_node_loc(*id), msg);
                },
                _ => {},
            }
        }
    });
}

/// Get the PrimitiveType from a pattern's type, stripping references.
fn get_prim_type(env: &GlobalEnv, id: &NodeId) -> Option<PrimitiveType> {
    let ty = env.get_node_type(*id);
    match ty.skip_reference() {
        Type::Primitive(p) => Some(*p),
        _ => None,
    }
}

/// Validate range bounds: detect empty or inverted ranges.
fn check_range_validity(env: &GlobalEnv, pat: &Pattern) {
    pat.visit_pre_post(&mut |is_post, p| {
        if is_post {
            return;
        }
        // A bare `..` (no bounds) is redundant -> use `_` instead.
        if let Pattern::Range(id, None, None, false) = p {
            env.error(
                &env.get_node_loc(*id),
                "unbounded range pattern `..` is not allowed; use `_` instead",
            );
        }
        // Inclusive ranges must have an explicit upper bound (`..=` and `lo..=` are invalid).
        if let Pattern::Range(id, _, None, true) = p {
            env.error(
                &env.get_node_loc(*id),
                "inclusive range pattern requires an upper bound",
            );
        }
        if let Pattern::Range(id, lo_opt, hi_opt, inclusive) = p {
            // Resolve effective bounds and flag errors for empty ranges.
            let prim = get_prim_type(env, id);
            let effective_lo = match lo_opt {
                Some(v) => v.to_bigint(),
                None => prim.as_ref().and_then(|p| p.get_min_value()),
            };
            let effective_hi = match hi_opt {
                Some(v) => v.to_bigint(),
                None => prim.as_ref().and_then(|p| p.get_max_value()).map(|m| m + 1),
            };
            if let (Some(lo_n), Some(hi_n)) = (&effective_lo, &effective_hi) {
                let is_empty = if *inclusive && hi_opt.is_some() {
                    lo_n > hi_n
                } else {
                    // Half-open comparison (exclusive upper, or open-ended resolved to max+1)
                    lo_n >= hi_n
                };
                if is_empty {
                    env.error(&env.get_node_loc(*id), "empty range pattern");
                }
            }
        }
    });
}
