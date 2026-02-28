// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Primitive match transformation: converts match expressions over primitive types
//! (booleans, integers, byte strings, and tuples of primitives) into if-else chains.
//! Also handles "mixed tuple" matches where a tuple discriminator contains both
//! primitive and non-primitive (enum/struct) elements.

use crate::env_pipeline::rewrite_target::{
    RewriteState, RewriteTarget, RewriteTargets, RewritingScope,
};
use move_model::{
    ast::{AbortKind, Exp, ExpData, MatchArm, Operation, Pattern, Value},
    exp_rewriter::ExpRewriterFunctions,
    metadata::LanguageVersion,
    model::{GlobalEnv, Loc, NodeId},
    symbol::Symbol,
    ty::{PrimitiveType, Type},
    well_known,
};
use num::BigInt;
use std::collections::BTreeSet;

// ================================================================================================
// Main Entry Point

/// Transform primitive pattern matches in all target functions.
pub fn transform(env: &mut GlobalEnv) {
    let mut transformer = MatchTransformer { env };
    let mut targets = RewriteTargets::create(env, RewritingScope::CompilationTarget);
    let todo: BTreeSet<_> = targets.keys().collect();
    for target in todo {
        if let RewriteTarget::MoveFun(func_id) = target {
            let func_env = transformer.env.get_function(func_id);
            if let Some(def) = func_env.get_def().cloned() {
                let new_def = transformer.rewrite_exp(def.clone());
                if !ExpData::ptr_eq(&new_def, &def) {
                    *targets.state_mut(&target) = RewriteState::Def(new_def);
                }
            }
        }
    }
    targets.write_to_env(env);
}

// ================================================================================================
// Rewriter (primitive match transformation)

struct MatchTransformer<'env> {
    env: &'env GlobalEnv,
}

impl ExpRewriterFunctions for MatchTransformer<'_> {
    fn rewrite_match(&mut self, id: NodeId, discriminator: &Exp, arms: &[MatchArm]) -> Option<Exp> {
        // Transform primitive matches to if-else chains or combinations of match with guards.
        let fully_transformable = is_match_fully_transformable(self.env, discriminator, arms);
        let mixed_tuple =
            !fully_transformable && is_mixed_tuple_match(self.env, discriminator, arms);
        // Matches over primitive types (and mixed tuples containing them) require
        // language version 2.4+.
        if (fully_transformable || mixed_tuple)
            && !check_primitive_match_version(self.env, discriminator)
        {
            return None;
        }
        if fully_transformable {
            Some(generate_if_else_chain(self.env, id, discriminator, arms, 0))
        } else if mixed_tuple {
            Some(transform_mixed_tuple_match(
                self.env,
                id,
                discriminator,
                arms,
            ))
        } else {
            None
        }
    }
}

/// Check that the language version supports primitive match expressions.
/// Returns `true` if the version is sufficient, `false` after emitting an error otherwise.
fn check_primitive_match_version(env: &GlobalEnv, discriminator: &Exp) -> bool {
    if env.language_version().is_at_least(LanguageVersion::V2_4) {
        return true;
    }
    env.error(
        &env.get_node_loc(discriminator.node_id()),
        "match over integers, booleans, or byte strings \
         is not supported before language version 2.4",
    );
    false
}

// ================================================================================================
// Primitive Match Detection

/// Check if a match expression is fully transformable to an if-else chain.
fn is_match_fully_transformable(env: &GlobalEnv, discriminator: &Exp, arms: &[MatchArm]) -> bool {
    // Check if discriminator is a suitable type
    let discriminator_ty = env.get_node_type(discriminator.node_id());
    if !is_suitable_type(&discriminator_ty) {
        return false;
    }

    // Check if all patterns are suitable patterns (literals, wildcards, vars, or tuples thereof)
    arms.iter().all(|arm| is_suitable_pattern(&arm.pattern))
}

/// Check if a type is suitable (bool, integer, byte string, or tuple of suitable types).
fn is_suitable_type(ty: &Type) -> bool {
    match ty {
        Type::Primitive(prim) => matches!(
            prim,
            PrimitiveType::Bool
                | PrimitiveType::U8
                | PrimitiveType::U16
                | PrimitiveType::U32
                | PrimitiveType::U64
                | PrimitiveType::U128
                | PrimitiveType::U256
                | PrimitiveType::I8
                | PrimitiveType::I16
                | PrimitiveType::I32
                | PrimitiveType::I64
                | PrimitiveType::I128
                | PrimitiveType::I256
        ),
        Type::Vector(inner) => matches!(inner.as_ref(), Type::Primitive(PrimitiveType::U8)),
        Type::Tuple(tys) => tys.iter().all(is_suitable_type),
        _ => false,
    }
}

/// Check if a pattern is suitable (literals, wildcards, vars, or tuples thereof).
fn is_suitable_pattern(pat: &Pattern) -> bool {
    match pat {
        Pattern::Wildcard(_) | Pattern::Var(_, _) | Pattern::LiteralValue(_, _) => true,
        Pattern::Tuple(_, pats) => pats.iter().all(is_suitable_pattern),
        Pattern::Struct(..) | Pattern::Error(_) => false,
    }
}

/// Check if a pattern unconditionally matches any value (wildcard, variable,
/// or a tuple of all catch-all patterns).
fn is_catch_all_pattern(pat: &Pattern) -> bool {
    match pat {
        Pattern::Wildcard(_) | Pattern::Var(_, _) => true,
        Pattern::Tuple(_, pats) => pats.iter().all(is_catch_all_pattern),
        _ => false,
    }
}

/// Check if a pattern binds any variables (at any nesting depth).
fn pattern_has_vars(pat: &Pattern) -> bool {
    match pat {
        Pattern::Var(..) => true,
        Pattern::Tuple(_, pats) => pats.iter().any(pattern_has_vars),
        _ => false,
    }
}

// ================================================================================================
// Match to If-Else Transformation

/// Recursively generate if-else chain for match arms, starting from `arm_idx`.
fn generate_if_else_chain(
    env: &GlobalEnv,
    result_id: NodeId,
    discriminator: &Exp,
    arms: &[MatchArm],
    arm_idx: usize,
) -> Exp {
    if arm_idx >= arms.len() {
        return generate_abort(env, result_id);
    }

    let arm = &arms[arm_idx];

    // Unguarded catch-all: terminal arm, just return body with bindings.
    if arm.condition.is_none() && is_catch_all_pattern(&arm.pattern) {
        return maybe_bind_pattern(env, discriminator, &arm.pattern, &arm.body);
    }

    let else_branch = generate_if_else_chain(env, result_id, discriminator, arms, arm_idx + 1);
    let result_ty = env.get_node_type(result_id);
    let if_id = env.new_node(env.get_node_loc(result_id), result_ty);
    let (condition, then_branch) = generate_arm_test(env, result_id, discriminator, arm);

    ExpData::IfElse(if_id, condition, then_branch, else_branch).into_exp()
}

/// Generate the condition and body expressions for a single match arm.
///
/// When a guard references pattern variables, the bindings are scoped
/// separately around the guard and body so they don't leak into the else
/// branch. Otherwise, the pattern condition and guard are combined directly.
fn generate_arm_test(
    env: &GlobalEnv,
    result_id: NodeId,
    discriminator: &Exp,
    arm: &MatchArm,
) -> (Exp, Exp) {
    // When a guard is present and the pattern binds variables, scope those
    // bindings to the guard and body *separately*.
    if let Some(guard) = &arm.condition {
        if is_catch_all_pattern(&arm.pattern) || pattern_has_vars(&arm.pattern) {
            let scoped_guard = maybe_bind_pattern(env, discriminator, &arm.pattern, guard);
            let scoped_body = maybe_bind_pattern(env, discriminator, &arm.pattern, &arm.body);
            let condition = if is_catch_all_pattern(&arm.pattern) {
                scoped_guard
            } else {
                let pattern_cond = generate_pattern_condition(env, discriminator, &arm.pattern);
                let bool_ty = Type::Primitive(PrimitiveType::Bool);
                let and_id = env.new_node(env.get_node_loc(result_id), bool_ty);
                ExpData::Call(and_id, Operation::And, vec![pattern_cond, scoped_guard]).into_exp()
            };
            return (condition, scoped_body);
        }
    }

    // Non-guarded or guarded without pattern variables: combine directly.
    let pattern_cond = generate_pattern_condition(env, discriminator, &arm.pattern);
    let condition = if let Some(guard_exp) = &arm.condition {
        let loc = env.get_node_loc(discriminator.node_id());
        let and_id = env.new_node(loc, Type::Primitive(PrimitiveType::Bool));
        ExpData::Call(and_id, Operation::And, vec![
            pattern_cond,
            guard_exp.clone(),
        ])
        .into_exp()
    } else {
        pattern_cond
    };
    let body = maybe_bind_pattern(env, discriminator, &arm.pattern, &arm.body);
    (condition, body)
}

/// Generate a boolean condition that tests if discriminator matches pattern.
fn generate_pattern_condition(env: &GlobalEnv, discriminator: &Exp, pattern: &Pattern) -> Exp {
    let loc = env.get_node_loc(discriminator.node_id());
    let bool_ty = Type::Primitive(PrimitiveType::Bool);
    let bool_id = env.new_node(loc.clone(), bool_ty.clone());

    match pattern {
        Pattern::Wildcard(_) | Pattern::Var(_, _) => {
            // Wildcard/var always matches - return true
            ExpData::Value(bool_id, Value::Bool(true)).into_exp()
        },

        Pattern::LiteralValue(_, val) => {
            // Generate: discriminator == value
            let discriminator_ty = env.get_node_type(discriminator.node_id());
            let val_id = env.new_node(loc.clone(), discriminator_ty);
            let val_exp = ExpData::Value(val_id, val.clone()).into_exp();

            ExpData::Call(bool_id, Operation::Eq, vec![discriminator.clone(), val_exp]).into_exp()
        },

        Pattern::Tuple(_, pats) => {
            // For tuple patterns, generate conjunctions of component checks
            let discriminator_ty = env.get_node_type(discriminator.node_id());
            if let Type::Tuple(tys) = discriminator_ty {
                generate_tuple_condition(env, discriminator, &tys, pats)
            } else {
                ExpData::Invalid(bool_id).into_exp()
            }
        },

        Pattern::Struct(..) | Pattern::Error(_) => {
            // Precluded by is_match_fully_transformable / is_suitable_pattern.
            ExpData::Invalid(bool_id).into_exp()
        },
    }
}

/// Generate condition for tuple pattern matching.
///
/// For tuple patterns, we always bind the tuple to temporary variables first,
/// then compare individual components. This avoids issues with tuple comparison
/// semantics in the bytecode generator.
fn generate_tuple_condition(
    env: &GlobalEnv,
    tuple_exp: &Exp,
    tys: &[Type],
    patterns: &[Pattern],
) -> Exp {
    let loc = env.get_node_loc(tuple_exp.node_id());
    let bool_ty = Type::Primitive(PrimitiveType::Bool);
    let bool_id = env.new_node(loc.clone(), bool_ty.clone());

    if patterns.is_empty() {
        // Empty tuple always matches
        return ExpData::Value(bool_id, Value::Bool(true)).into_exp();
    }

    // Check if all patterns are either Value patterns or wildcards/vars
    let all_wildcards = patterns
        .iter()
        .all(|p| matches!(p, Pattern::Wildcard(_) | Pattern::Var(_, _)));

    if all_wildcards {
        // All wildcards - always matches
        return ExpData::Value(bool_id, Value::Bool(true)).into_exp();
    }

    // Always use the binding approach:
    // Generate:
    // {
    //     let (tmp0, tmp1, ...) = tuple_exp;
    //     tmp0 == val0 && tmp1 == val1 && ...
    // }

    // First, create temp variable symbols for each tuple element.
    // Prefix with `_` for positions where the original pattern is a wildcard/var
    // to avoid unused-variable warnings.
    let temp_patterns: Vec<Pattern> = tys
        .iter()
        .enumerate()
        .map(|(idx, ty)| {
            let prefix = if matches!(patterns[idx], Pattern::Wildcard(_) | Pattern::Var(_, _)) {
                "_"
            } else {
                ""
            };
            let sym = env
                .symbol_pool()
                .make(&format!("{}$tuple_elem_{}", prefix, idx));
            let pat_id = env.new_node(loc.clone(), ty.clone());
            Pattern::Var(pat_id, sym)
        })
        .collect();

    let tuple_pattern_id = env.new_node(loc.clone(), Type::Tuple(tys.to_vec()));
    let tuple_pattern = Pattern::Tuple(tuple_pattern_id, temp_patterns.clone());

    // Generate conditions for each non-wildcard pattern
    let mut conditions = vec![];
    for (idx, pat) in patterns.iter().enumerate() {
        if matches!(pat, Pattern::Wildcard(_) | Pattern::Var(_, _)) {
            continue; // Skip wildcards and vars
        }

        if let Pattern::LiteralValue(_, val) = pat {
            // temp_patterns are always Pattern::Var (built above).
            let Pattern::Var(var_id, sym) = &temp_patterns[idx] else {
                unreachable!()
            };
            let var_exp = ExpData::LocalVar(*var_id, *sym).into_exp();
            let val_id = env.new_node(loc.clone(), tys[idx].clone());
            let val_exp = ExpData::Value(val_id, val.clone()).into_exp();
            let cmp_id = env.new_node(loc.clone(), bool_ty.clone());
            conditions
                .push(ExpData::Call(cmp_id, Operation::Eq, vec![var_exp, val_exp]).into_exp());
        }
    }

    // Combine conditions with AND
    let combined_cond = if conditions.is_empty() {
        ExpData::Value(bool_id, Value::Bool(true)).into_exp()
    } else {
        conditions
            .into_iter()
            .reduce(|acc, cond| {
                let and_id = env.new_node(loc.clone(), bool_ty.clone());
                ExpData::Call(and_id, Operation::And, vec![acc, cond]).into_exp()
            })
            .unwrap()
    };

    // Wrap in a block that binds the tuple
    let block_id = env.new_node(loc, bool_ty);
    ExpData::Block(
        block_id,
        tuple_pattern,
        Some(tuple_exp.clone()),
        combined_cond,
    )
    .into_exp()
}

/// Wrap expression with pattern bindings if needed (for variable patterns).
fn maybe_bind_pattern(env: &GlobalEnv, discriminator: &Exp, pattern: &Pattern, body: &Exp) -> Exp {
    match pattern {
        Pattern::Var(var_id, _sym) => {
            // Bind the variable to the discriminator value
            let loc = env.get_node_loc(*var_id);
            let block_id = env.new_node(loc, env.get_node_type(body.node_id()));

            ExpData::Block(
                block_id,
                pattern.clone(),
                Some(discriminator.clone()),
                body.clone(),
            )
            .into_exp()
        },
        Pattern::Tuple(tuple_id, pats) => {
            // Check if any sub-pattern has variables
            let has_vars = pats.iter().any(|p| matches!(p, Pattern::Var(..)));
            if has_vars {
                // Replace non-variable sub-patterns (literals, wildcards) with wildcards
                // so the binding pattern only contains variables.  The literal checks
                // are already handled by generate_pattern_condition.
                let bind_pats: Vec<Pattern> = pats
                    .iter()
                    .map(|p| match p {
                        Pattern::Var(..) => p.clone(),
                        _ => Pattern::Wildcard(p.node_id()),
                    })
                    .collect();
                let bind_pattern = Pattern::Tuple(*tuple_id, bind_pats);
                let loc = env.get_node_loc(pattern.node_id());
                let block_id = env.new_node(loc, env.get_node_type(body.node_id()));

                ExpData::Block(
                    block_id,
                    bind_pattern,
                    Some(discriminator.clone()),
                    body.clone(),
                )
                .into_exp()
            } else {
                body.clone()
            }
        },
        _ => body.clone(),
    }
}

/// Generate an abort expression.
fn generate_abort(env: &GlobalEnv, id: NodeId) -> Exp {
    let loc = env.get_node_loc(id);
    let result_ty = env.get_node_type(id);
    let abort_id = env.new_node(loc.clone(), result_ty);
    let code_id = env.new_node(loc, Type::Primitive(PrimitiveType::U64));

    ExpData::Call(abort_id, Operation::Abort(AbortKind::Code), vec![
        ExpData::Value(
            code_id,
            Value::Number(BigInt::from(well_known::INCOMPLETE_MATCH_ABORT_CODE)),
        )
        .into_exp(),
    ])
    .into_exp()
}

// ================================================================================================
// Mixed Tuple Match Detection and Transformation

/// Check if a match has a tuple discriminator mixing primitive and non-primitive types.
///
/// This detects cases like `match ((enum_val, 1, 2)) { (Variant(a), 1, 2) => ..., _ => ... }`
/// where the tuple contains both enum/struct elements and primitive elements.
fn is_mixed_tuple_match(env: &GlobalEnv, discriminator: &Exp, arms: &[MatchArm]) -> bool {
    // Discriminator must be an explicit tuple construction.
    let disc_args = match discriminator.as_ref() {
        ExpData::Call(_, Operation::Tuple, args) => args,
        _ => return false,
    };

    // Get tuple element types
    let disc_ty = env.get_node_type(discriminator.node_id());
    let elem_tys = match &disc_ty {
        Type::Tuple(tys) => tys,
        _ => return false,
    };

    if elem_tys.len() != disc_args.len() {
        return false;
    }

    // Must have at least one primitive and at least one non-primitive element
    let has_primitive = elem_tys.iter().any(is_suitable_type);
    let has_non_primitive = elem_tys.iter().any(|ty| !is_suitable_type(ty));
    if !has_primitive || !has_non_primitive {
        return false;
    }

    // All arms must have valid patterns for this transformation
    arms.iter().all(|arm| {
        match &arm.pattern {
            Pattern::Tuple(_, pats) => {
                if pats.len() != elem_tys.len() {
                    return false;
                }
                // Check each position
                pats.iter().enumerate().all(|(idx, pat)| {
                    if is_suitable_type(&elem_tys[idx]) {
                        // Primitive positions: must be literal, wildcard, or var
                        matches!(
                            pat,
                            Pattern::LiteralValue(..) | Pattern::Wildcard(_) | Pattern::Var(_, _)
                        )
                    } else {
                        // Non-primitive positions: must be struct, wildcard, or var
                        matches!(
                            pat,
                            Pattern::Struct(..) | Pattern::Wildcard(_) | Pattern::Var(_, _)
                        )
                    }
                })
            },
            // Top-level catch-all is fine
            Pattern::Wildcard(_) | Pattern::Var(_, _) => true,
            _ => false,
        }
    })
}

/// Transform a mixed tuple match by extracting primitive conditions to guards.
///
/// Transforms:
/// ```text
/// match ((x, y, z)) {
///     (Data::V1(a, b), 1, 2) => a + b + 10,
///     (Data::V2(a, b, c), 5, 6) => a + b,
///     _ => 99,
/// }
/// ```
/// Into:
/// ```text
/// { let $prim_0 = y; let $prim_1 = z;
///   match (x) {
///     Data::V1(a, b) if $prim_0 == 1 && $prim_1 == 2 => a + b + 10,
///     Data::V2(a, b, c) if $prim_0 == 5 && $prim_1 == 6 => a + b,
///     _ => 99,
///   }
/// }
/// ```
fn transform_mixed_tuple_match(
    env: &GlobalEnv,
    match_id: NodeId,
    discriminator: &Exp,
    arms: &[MatchArm],
) -> Exp {
    let loc = env.get_node_loc(match_id);

    // Extract tuple args from discriminator
    let disc_args = match discriminator.as_ref() {
        ExpData::Call(_, Operation::Tuple, args) => args,
        _ => unreachable!("is_mixed_tuple_match verified this"),
    };

    let disc_ty = env.get_node_type(discriminator.node_id());
    let elem_tys = match &disc_ty {
        Type::Tuple(tys) => tys.clone(),
        _ => unreachable!("is_mixed_tuple_match verified this"),
    };

    // Classify positions
    let primitive_positions: Vec<usize> = elem_tys
        .iter()
        .enumerate()
        .filter(|(_, ty)| is_suitable_type(ty))
        .map(|(i, _)| i)
        .collect();
    let non_primitive_positions: Vec<usize> = elem_tys
        .iter()
        .enumerate()
        .filter(|(_, ty)| !is_suitable_type(ty))
        .map(|(i, _)| i)
        .collect();

    // Create temp variables for primitive-position discriminator args
    let prim_temps: Vec<(Symbol, Exp)> = primitive_positions
        .iter()
        .enumerate()
        .map(|(seq, &pos)| {
            let sym = env.symbol_pool().make(&format!("$prim_{}", seq));
            let arg = disc_args[pos].clone();
            (sym, arg)
        })
        .collect();

    // Build new discriminator from non-primitive elements only
    let new_disc = if non_primitive_positions.len() == 1 {
        let pos = non_primitive_positions[0];
        disc_args[pos].clone()
    } else {
        let np_args: Vec<Exp> = non_primitive_positions
            .iter()
            .map(|&pos| disc_args[pos].clone())
            .collect();
        let np_tys: Vec<Type> = non_primitive_positions
            .iter()
            .map(|&pos| elem_tys[pos].clone())
            .collect();
        let tuple_id = env.new_node(loc.clone(), Type::Tuple(np_tys));
        ExpData::Call(tuple_id, Operation::Tuple, np_args).into_exp()
    };

    // Transform each arm
    let new_arms: Vec<MatchArm> = arms
        .iter()
        .map(|arm| {
            transform_mixed_arm(
                env,
                arm,
                &elem_tys,
                &primitive_positions,
                &non_primitive_positions,
                &prim_temps,
            )
        })
        .collect();

    // Build the new match expression
    let match_result_ty = env.get_node_type(match_id);
    let new_match_id = env.new_node(loc.clone(), match_result_ty.clone());
    let match_exp = ExpData::Match(new_match_id, new_disc, new_arms).into_exp();

    // Wrap in blocks that bind primitive temps: { let $prim_0 = y; { let $prim_1 = z; match ... } }
    // Build from inside out
    prim_temps
        .iter()
        .enumerate()
        .rev()
        .fold(match_exp, |inner, (seq, (sym, arg))| {
            let pos = primitive_positions[seq];
            let ty = elem_tys[pos].clone();
            let pat_id = env.new_node(loc.clone(), ty);
            let pattern = Pattern::Var(pat_id, *sym);
            let block_id = env.new_node(loc.clone(), match_result_ty.clone());
            ExpData::Block(block_id, pattern, Some(arg.clone()), inner).into_exp()
        })
}

/// Transform a single arm of a mixed tuple match.
fn transform_mixed_arm(
    env: &GlobalEnv,
    arm: &MatchArm,
    elem_tys: &[Type],
    primitive_positions: &[usize],
    non_primitive_positions: &[usize],
    prim_temps: &[(Symbol, Exp)],
) -> MatchArm {
    match &arm.pattern {
        Pattern::Tuple(_, pats) => {
            let loc = env.get_node_loc(arm.pattern.node_id());

            // Build new pattern from non-primitive sub-patterns only
            let new_pattern = if non_primitive_positions.len() == 1 {
                let pos = non_primitive_positions[0];
                pats[pos].clone()
            } else {
                let np_pats: Vec<Pattern> = non_primitive_positions
                    .iter()
                    .map(|&pos| pats[pos].clone())
                    .collect();
                let np_tys: Vec<Type> = non_primitive_positions
                    .iter()
                    .map(|&pos| elem_tys[pos].clone())
                    .collect();
                let tuple_id = env.new_node(loc.clone(), Type::Tuple(np_tys));
                Pattern::Tuple(tuple_id, np_pats)
            };

            // Generate guard conditions from primitive positions
            let bool_ty = Type::Primitive(PrimitiveType::Bool);
            let mut conditions: Vec<Exp> = Vec::new();
            let mut var_bindings: Vec<(Symbol, usize)> = Vec::new();

            for (seq, &pos) in primitive_positions.iter().enumerate() {
                let pat = &pats[pos];
                match pat {
                    Pattern::LiteralValue(_, val) => {
                        // Generate: $prim_seq == val
                        let (sym, _) = &prim_temps[seq];
                        let var_id = env.new_node(loc.clone(), elem_tys[pos].clone());
                        let var_exp = ExpData::LocalVar(var_id, *sym).into_exp();
                        let val_id = env.new_node(loc.clone(), elem_tys[pos].clone());
                        let val_exp = ExpData::Value(val_id, val.clone()).into_exp();
                        let cmp_id = env.new_node(loc.clone(), bool_ty.clone());
                        conditions.push(
                            ExpData::Call(cmp_id, Operation::Eq, vec![var_exp, val_exp]).into_exp(),
                        );
                    },
                    Pattern::Var(_, var_sym) => {
                        // Remember to bind this variable in the body
                        var_bindings.push((*var_sym, seq));
                    },
                    Pattern::Wildcard(_) => {
                        // No condition needed
                    },
                    _ => unreachable!("is_mixed_tuple_match verified pattern types"),
                }
            }

            // Combine primitive conditions with AND
            let prim_guard = conditions.into_iter().reduce(|acc, cond| {
                let and_id = env.new_node(loc.clone(), bool_ty.clone());
                ExpData::Call(and_id, Operation::And, vec![acc, cond]).into_exp()
            });

            // Wrap the user's guard with primitive-position var bindings so they
            // are in scope: { let y = $prim_0; guard }
            let wrapped_user_guard = arm.condition.as_ref().map(|og| {
                wrap_with_prim_bindings(
                    env,
                    &loc,
                    elem_tys,
                    primitive_positions,
                    prim_temps,
                    &var_bindings,
                    og.clone(),
                )
            });

            // Combine with existing guard: prim_guard && wrapped_user_guard
            let new_condition = match (&prim_guard, &wrapped_user_guard) {
                (Some(pg), Some(wg)) => {
                    let and_id = env.new_node(loc.clone(), bool_ty.clone());
                    Some(
                        ExpData::Call(and_id, Operation::And, vec![pg.clone(), wg.clone()])
                            .into_exp(),
                    )
                },
                (Some(pg), None) => Some(pg.clone()),
                (None, Some(wg)) => Some(wg.clone()),
                (None, None) => None,
            };

            // Wrap the body with primitive-position var bindings
            let new_body = wrap_with_prim_bindings(
                env,
                &loc,
                elem_tys,
                primitive_positions,
                prim_temps,
                &var_bindings,
                arm.body.clone(),
            );

            MatchArm {
                loc: arm.loc.clone(),
                pattern: new_pattern,
                condition: new_condition,
                body: new_body,
            }
        },
        Pattern::Wildcard(_) => {
            // Keep as wildcard catch-all
            arm.clone()
        },
        Pattern::Var(..) => {
            // A top-level Var pattern on a mixed tuple match would require binding a
            // tuple-typed local. The type checker's NoTuple constraint rejects this
            // before the env pipeline runs, so this branch is unreachable.
            unreachable!("top-level Var pattern on mixed tuple: rejected by type checker")
        },
        _ => arm.clone(),
    }
}

/// Wrap an expression with nested let-bindings for primitive-position variables.
///
/// For each `(var_sym, seq)` in `var_bindings`, generates:
/// `{ let var_sym = $prim_seq; inner }`
///
/// Returns `inner` unchanged when `var_bindings` is empty.
fn wrap_with_prim_bindings(
    env: &GlobalEnv,
    loc: &Loc,
    elem_tys: &[Type],
    primitive_positions: &[usize],
    prim_temps: &[(Symbol, Exp)],
    var_bindings: &[(Symbol, usize)],
    inner: Exp,
) -> Exp {
    var_bindings
        .iter()
        .rev()
        .fold(inner, |acc, (var_sym, seq)| {
            let pos = primitive_positions[*seq];
            let (prim_sym, _) = &prim_temps[*seq];
            let var_pat_id = env.new_node(loc.clone(), elem_tys[pos].clone());
            let pattern = Pattern::Var(var_pat_id, *var_sym);
            let prim_var_id = env.new_node(loc.clone(), elem_tys[pos].clone());
            let prim_ref = ExpData::LocalVar(prim_var_id, *prim_sym).into_exp();
            let block_id = env.new_node(loc.clone(), env.get_node_type(acc.node_id()));
            ExpData::Block(block_id, pattern, Some(prim_ref), acc).into_exp()
        })
}
