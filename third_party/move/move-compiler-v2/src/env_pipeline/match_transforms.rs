// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Primitive match transformation: converts match expressions over primitive types
//! (booleans, integers, byte strings, and tuples of primitives) into if-else chains.
//! Also handles "mixed tuple" matches where a tuple discriminator contains both
//! primitive and non-primitive (enum/struct) elements.

use crate::env_pipeline::rewrite_target::{
    RewriteState, RewriteTarget, RewriteTargets, RewritingScope,
};
use codespan_reporting::diagnostic::Severity;
use move_model::{
    ast::{AbortKind, Exp, ExpData, MatchArm, Operation, Pattern, Value},
    exp_builder::ExpBuilder,
    exp_rewriter::ExpRewriterFunctions,
    metadata::lang_feature_versions::LANGUAGE_VERSION_FOR_PRIMITIVE_MATCH,
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
// Temporary Symbol Names
//
// All temporary symbols introduced by this pass are created through this enum,
// making it easy to audit for name clashes.

/// Kinds of temporary variables introduced during match transformation.
enum TempSymbol {
    /// Scalar discriminator binding: `_$disc`.
    Discriminator,
    /// Tuple discriminator element binding: `_$disc_0`, `_$disc_1`, etc.
    TupleDiscriminatorElement(usize),
    /// Primitive-position temp in a mixed tuple: `_$prim_0`, `_$prim_1`, etc.
    PrimitiveTemp(usize),
    /// Non-primitive-position temp in a mixed tuple: `_$np_0`, `_$np_1`, etc.
    NonPrimitiveTemp(usize),
}

impl TempSymbol {
    fn create(self, env: &GlobalEnv) -> Symbol {
        match self {
            TempSymbol::Discriminator => env.symbol_pool().make("_$disc"),
            TempSymbol::TupleDiscriminatorElement(idx) => {
                env.symbol_pool().make(&format!("_$disc_{}", idx))
            },
            TempSymbol::PrimitiveTemp(seq) => env.symbol_pool().make(&format!("_$prim_{}", seq)),
            TempSymbol::NonPrimitiveTemp(seq) => env.symbol_pool().make(&format!("_$np_{}", seq)),
        }
    }
}

// ================================================================================================
// Expression Building Helpers

/// Create a `LocalVar` expression.
fn make_local_var(env: &GlobalEnv, loc: &Loc, ty: Type, sym: Symbol) -> Exp {
    let id = env.new_node(loc.clone(), ty);
    ExpData::LocalVar(id, sym).into_exp()
}

/// Create an equality comparison expression: `lhs == rhs`.
fn make_eq(env: &GlobalEnv, loc: &Loc, lhs: Exp, rhs: Exp) -> Exp {
    let id = env.new_node(loc.clone(), Type::Primitive(PrimitiveType::Bool));
    ExpData::Call(id, Operation::Eq, vec![lhs, rhs]).into_exp()
}

/// Combine a list of boolean conditions with `&&`. Returns `None` if the list is empty.
fn conjoin(env: &GlobalEnv, loc: &Loc, conditions: Vec<Exp>) -> Option<Exp> {
    conditions.into_iter().reduce(|acc, cond| {
        let id = env.new_node(loc.clone(), Type::Primitive(PrimitiveType::Bool));
        ExpData::Call(id, Operation::And, vec![acc, cond]).into_exp()
    })
}

/// Create a boolean `true` literal.
fn make_true(env: &GlobalEnv, loc: &Loc) -> Exp {
    ExpBuilder::new(env).bool_const(loc, true)
}

/// Select elements from `slice` at the given `positions`.
fn select_positions<T: Clone>(slice: &[T], positions: &[usize]) -> Vec<T> {
    positions.iter().map(|&pos| slice[pos].clone()).collect()
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
        // a minimum language version.
        if (fully_transformable || mixed_tuple)
            && !check_primitive_match_version(self.env, discriminator)
        {
            return None;
        }
        if fully_transformable {
            let (new_disc, bind_pat, bind_init) = bind_discriminator(self.env, discriminator);
            let chain = generate_if_else_chain(self.env, id, &new_disc, arms, 0);
            Some(ExpBuilder::new(self.env).block(bind_pat, Some(bind_init), chain))
        } else if mixed_tuple {
            Some(transform_mixed_tuple_match(
                self.env,
                id,
                discriminator,
                arms,
            ))
        } else {
            // If neither transform applies but arms contain literal patterns, report
            // an error instead of letting them reach bytecode generation.
            // This should eventually never happen as we should be able to transform all literal
            // patterns that reach this stage. TODO(#19024).
            self.reject_unsupported_literals(arms);
            None
        }
    }
}

impl MatchTransformer<'_> {
    /// Report errors for any literal patterns in arms that won't be transformed.
    fn reject_unsupported_literals(&self, arms: &[MatchArm]) {
        for arm in arms {
            arm.pattern.visit_pre_post(&mut |is_post, pat| {
                if !is_post {
                    if let Pattern::LiteralValue(id, _) = pat {
                        self.env.error(
                            &self.env.get_node_loc(*id),
                            "literal patterns are not supported in this match expression",
                        );
                    }
                }
            });
        }
    }
}

/// Check that the language version supports primitive match expressions.
/// Returns `true` if the version is sufficient, `false` after emitting an error otherwise.
fn check_primitive_match_version(env: &GlobalEnv, discriminator: &Exp) -> bool {
    if env
        .language_version()
        .is_at_least(LANGUAGE_VERSION_FOR_PRIMITIVE_MATCH)
    {
        return true;
    }
    env.error(
        &env.get_node_loc(discriminator.node_id()),
        &format!(
            "match over integers, booleans, or byte strings \
             is not supported before language version {}",
            LANGUAGE_VERSION_FOR_PRIMITIVE_MATCH
        ),
    );
    false
}

// ================================================================================================
// Primitive Match Detection

/// Check if a match expression and its arms are fully transformable to an if-else chain (as opposed to
/// a match expression with guards).
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

/// Bind the discriminator to temporary variables so it is evaluated exactly once.
///
/// For scalar types, produces a single `_$disc` binding.
/// For tuple types, produces `(_$disc_0, _$disc_1, ..)` bindings and returns
/// a `Tuple(LocalVar(_$disc_0), ..)` as the new discriminator expression.
fn bind_discriminator(env: &GlobalEnv, discriminator: &Exp) -> (Exp, Pattern, Exp) {
    let loc = env.get_node_loc(discriminator.node_id());
    let disc_ty = env.get_node_type(discriminator.node_id());

    match &disc_ty {
        Type::Tuple(tys) => {
            let mut patterns = Vec::new();
            let mut var_exps = Vec::new();
            for (idx, ty) in tys.iter().enumerate() {
                let sym = TempSymbol::TupleDiscriminatorElement(idx).create(env);
                let pat_id = env.new_node(loc.clone(), ty.clone());
                patterns.push(Pattern::Var(pat_id, sym));
                var_exps.push(make_local_var(env, &loc, ty.clone(), sym));
            }
            let tuple_pat_id = env.new_node(loc.clone(), disc_ty.clone());
            let pattern = Pattern::Tuple(tuple_pat_id, patterns);
            let tuple_id = env.new_node(loc, disc_ty);
            let new_disc = ExpData::Call(tuple_id, Operation::Tuple, var_exps).into_exp();
            (new_disc, pattern, discriminator.clone())
        },
        _ => {
            let sym = TempSymbol::Discriminator.create(env);
            let pat_id = env.new_node(loc.clone(), disc_ty.clone());
            let pattern = Pattern::Var(pat_id, sym);
            let new_disc = make_local_var(env, &loc, disc_ty, sym);
            (new_disc, pattern, discriminator.clone())
        },
    }
}

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
    let (condition, then_branch) = generate_arm_test(env, result_id, discriminator, arm);

    let builder = ExpBuilder::new(env);
    builder.if_else(condition, then_branch, else_branch)
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
                let loc = env.get_node_loc(result_id);
                conjoin(env, &loc, vec![pattern_cond, scoped_guard]).unwrap()
            };
            return (condition, scoped_body);
        }
    }

    // Non-guarded or guarded without pattern variables: combine directly.
    let pattern_cond = generate_pattern_condition(env, discriminator, &arm.pattern);
    let condition = if let Some(guard_exp) = &arm.condition {
        let loc = env.get_node_loc(discriminator.node_id());
        conjoin(env, &loc, vec![pattern_cond, guard_exp.clone()]).unwrap()
    } else {
        pattern_cond
    };
    let body = maybe_bind_pattern(env, discriminator, &arm.pattern, &arm.body);
    (condition, body)
}

/// Generate a boolean condition that tests if discriminator matches pattern.
fn generate_pattern_condition(env: &GlobalEnv, discriminator: &Exp, pattern: &Pattern) -> Exp {
    let loc = env.get_node_loc(discriminator.node_id());

    match pattern {
        Pattern::Wildcard(_) | Pattern::Var(_, _) => make_true(env, &loc),

        Pattern::LiteralValue(_, val) => {
            let discriminator_ty = env.get_node_type(discriminator.node_id());
            let val_id = env.new_node(loc.clone(), discriminator_ty);
            let val_exp = ExpData::Value(val_id, val.clone()).into_exp();
            make_eq(env, &loc, discriminator.clone(), val_exp)
        },

        Pattern::Tuple(_, pats) => {
            let discriminator_ty = env.get_node_type(discriminator.node_id());
            if let Type::Tuple(tys) = discriminator_ty {
                generate_tuple_condition(env, discriminator, &tys, pats)
            } else {
                let bool_id = env.new_node(loc, Type::Primitive(PrimitiveType::Bool));
                ExpData::Invalid(bool_id).into_exp()
            }
        },

        Pattern::Struct(..) | Pattern::Error(_) => {
            let bool_id = env.new_node(loc, Type::Primitive(PrimitiveType::Bool));
            ExpData::Invalid(bool_id).into_exp()
        },
    }
}

/// Generate condition for tuple pattern matching.
///
/// After `bind_discriminator`, the tuple expression is always
/// `Call(_, Tuple, args)` with `LocalVar` elements, so element expressions
/// are extracted directly and compared against pattern literals.
fn generate_tuple_condition(
    env: &GlobalEnv,
    tuple_exp: &Exp,
    tys: &[Type],
    patterns: &[Pattern],
) -> Exp {
    let loc = env.get_node_loc(tuple_exp.node_id());

    if patterns.is_empty() {
        return make_true(env, &loc);
    }

    let all_wildcards = patterns
        .iter()
        .all(|p| matches!(p, Pattern::Wildcard(_) | Pattern::Var(_, _)));
    if all_wildcards {
        return make_true(env, &loc);
    }

    // Extract element expressions from the Tuple call.
    let elem_exps = match tuple_exp.as_ref() {
        ExpData::Call(_, Operation::Tuple, args) => args,
        _ => {
            env.diag(
                Severity::Bug,
                &loc,
                "unexpected non-tuple discriminator in tuple condition generation",
            );
            return make_true(env, &loc);
        },
    };

    // Generate Eq conditions for each literal pattern
    let conditions: Vec<Exp> = patterns
        .iter()
        .enumerate()
        .filter_map(|(idx, pat)| {
            if let Pattern::LiteralValue(_, val) = pat {
                let val_id = env.new_node(loc.clone(), tys[idx].clone());
                let val_exp = ExpData::Value(val_id, val.clone()).into_exp();
                Some(make_eq(env, &loc, elem_exps[idx].clone(), val_exp))
            } else {
                None
            }
        })
        .collect();

    conjoin(env, &loc, conditions).unwrap_or_else(|| make_true(env, &loc))
}

/// Wrap expression with pattern bindings if needed (for variable patterns).
///
/// Only introduces bindings for variables that are actually free in `body`,
/// to avoid unused-variable warnings on compiler-generated code.
fn maybe_bind_pattern(env: &GlobalEnv, discriminator: &Exp, pattern: &Pattern, body: &Exp) -> Exp {
    let builder = ExpBuilder::new(env);
    let free = body.as_ref().free_vars();
    match pattern {
        Pattern::Var(_, sym) => {
            if !free.contains(sym) {
                return body.clone();
            }
            builder.block(pattern.clone(), Some(discriminator.clone()), body.clone())
        },
        Pattern::Tuple(tuple_id, pats) => {
            // Only keep sub-patterns whose variable is actually used in the body.
            // Replace all others (literals, wildcards, unused vars) with wildcards.
            let bind_pats: Vec<Pattern> = pats
                .iter()
                .map(|p| match p {
                    Pattern::Var(_, sym) if free.contains(sym) => p.clone(),
                    _ => Pattern::Wildcard(p.node_id()),
                })
                .collect();
            let has_bindings = bind_pats.iter().any(|p| matches!(p, Pattern::Var(..)));
            if has_bindings {
                let bind_pattern = Pattern::Tuple(*tuple_id, bind_pats);
                builder.block(bind_pattern, Some(discriminator.clone()), body.clone())
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
            Pattern::Wildcard(_) => true,
            Pattern::Var(id, ..) => {
                env.diag(
                    Severity::Bug,
                    &env.get_node_loc(*id),
                    "top-level Var pattern on mixed tuple: rejected by type checker",
                );
                false
            },
            _ => false,
        }
    })
}

/// Transform a mixed tuple match by extracting primitive conditions to guards.
///
/// All tuple elements are bound to temporaries in left-to-right order to
/// preserve evaluation order and ensure each sub-expression is evaluated
/// exactly once.
///
/// ## Example
///
/// Given a mixed tuple match where position 0 is non-primitive (enum) and
/// position 1 is primitive (`u64`), with a user-written guard on one arm:
///
/// ```move
/// match ((make_data(), compute_x())) {
///     (Data::V1 { f }, 5) if (f > 10) => f + 1,
///     (Data::V2, y)                    => y,
///     _                                => 0,
/// }
/// ```
///
/// The transform binds each tuple element to a temporary (preserving
/// left-to-right evaluation), strips the primitive position from the
/// pattern, and moves its literal check into a guard.  User-written guards
/// are wrapped so that primitive-position variable bindings are in scope,
/// then combined with the synthesized primitive check via `&&`:
///
/// ```move
/// { let _$np_0 = make_data();       // non-prim temp (pos 0)
///   let _$prim_0 = compute_x();     // prim temp     (pos 1)
///   match (_$np_0) {
///     //  pattern: only non-prim positions remain
///     //  guard:   prim literal check && user_guard
///     Data::V1 { f } if (_$prim_0 == 5 && f > 10) => f + 1,
///     //  pattern: non-prim only; prim var `y` bound via let in body
///     Data::V2       => { let y = _$prim_0; y },
///     _              => 0,
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
        _ => {
            env.diag(
                Severity::Bug,
                &loc,
                "expected tuple discriminator in mixed tuple match",
            );
            return ExpData::Invalid(match_id).into_exp();
        },
    };

    let disc_ty = env.get_node_type(discriminator.node_id());
    let elem_tys = match &disc_ty {
        Type::Tuple(tys) => tys.clone(),
        _ => {
            env.diag(
                Severity::Bug,
                &loc,
                "expected tuple type for discriminator in mixed tuple match",
            );
            return ExpData::Invalid(match_id).into_exp();
        },
    };

    // Classify positions
    let mut primitive_positions = Vec::new();
    let mut non_primitive_positions = Vec::new();
    for (i, ty) in elem_tys.iter().enumerate() {
        if is_suitable_type(ty) {
            primitive_positions.push(i);
        } else {
            non_primitive_positions.push(i);
        }
    }

    // Create temp variables for primitive-position discriminator args
    let prim_temps: Vec<(Symbol, Exp)> = primitive_positions
        .iter()
        .enumerate()
        .map(|(seq, &pos)| {
            let sym = TempSymbol::PrimitiveTemp(seq).create(env);
            let arg = disc_args[pos].clone();
            (sym, arg)
        })
        .collect();

    // Create temp variables for non-primitive-position discriminator args
    let np_temps: Vec<(Symbol, Exp)> = non_primitive_positions
        .iter()
        .enumerate()
        .map(|(seq, &pos)| {
            let sym = TempSymbol::NonPrimitiveTemp(seq).create(env);
            let arg = disc_args[pos].clone();
            (sym, arg)
        })
        .collect();

    // Build new discriminator from non-primitive temp references
    let new_disc = if non_primitive_positions.len() == 1 {
        let (sym, _) = &np_temps[0];
        let pos = non_primitive_positions[0];
        make_local_var(env, &loc, elem_tys[pos].clone(), *sym)
    } else {
        let np_args: Vec<Exp> = np_temps
            .iter()
            .enumerate()
            .map(|(seq, (sym, _))| {
                let pos = non_primitive_positions[seq];
                make_local_var(env, &loc, elem_tys[pos].clone(), *sym)
            })
            .collect();
        let np_tys = select_positions(&elem_tys, &non_primitive_positions);
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

    // Collect all bindings sorted by position to preserve left-to-right evaluation order.
    let mut all_bindings: Vec<(usize, Symbol, Exp)> = Vec::new();
    for (seq, &pos) in primitive_positions.iter().enumerate() {
        all_bindings.push((pos, prim_temps[seq].0, prim_temps[seq].1.clone()));
    }
    for (seq, &pos) in non_primitive_positions.iter().enumerate() {
        all_bindings.push((pos, np_temps[seq].0, np_temps[seq].1.clone()));
    }
    all_bindings.sort_by_key(|(pos, _, _)| *pos);

    // Wrap in blocks binding all elements in left-to-right order (build inside-out)
    let builder = ExpBuilder::new(env);
    all_bindings
        .iter()
        .rev()
        .fold(match_exp, |inner, (pos, sym, arg)| {
            let ty = elem_tys[*pos].clone();
            let pat_id = env.new_node(loc.clone(), ty);
            let pattern = Pattern::Var(pat_id, *sym);
            builder.block(pattern, Some(arg.clone()), inner)
        })
}

/// Transform a single arm of a mixed tuple match.
///
/// For a `Pattern::Tuple` arm, the primitive sub-patterns are removed from
/// the pattern and converted into guard conditions:
///
/// - `LiteralValue(v)` -- `_$prim_N == v` added to the guard conjunction.
/// - `Var(sym)`        -- `let sym = _$prim_N` injected into the guard and body.
/// - `Wildcard`        -- no condition or binding.
///
/// When the arm already carries a user-written guard, the final guard is:
///
/// ```text
///   prim_check_0 && prim_check_1 && ... && { let y = _$prim_K; user_guard }
/// ```
///
/// The user guard is wrapped with any primitive-position variable bindings
/// so those names are in scope.  The body is wrapped identically.
///
/// `Pattern::Wildcard` arms are retyped to the non-primitive-only
/// discriminator.  Top-level `Pattern::Var` is unreachable (see comment).
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
                pats[non_primitive_positions[0]].clone()
            } else {
                let np_pats = select_positions(pats, non_primitive_positions);
                let np_tys = select_positions(elem_tys, non_primitive_positions);
                let tuple_id = env.new_node(loc.clone(), Type::Tuple(np_tys));
                Pattern::Tuple(tuple_id, np_pats)
            };

            // Generate guard conditions from primitive positions
            let mut conditions: Vec<Exp> = Vec::new();
            let mut var_bindings: Vec<(Symbol, usize)> = Vec::new();

            for (seq, &pos) in primitive_positions.iter().enumerate() {
                let pat = &pats[pos];
                match pat {
                    Pattern::LiteralValue(_, val) => {
                        let (sym, _) = &prim_temps[seq];
                        let var_exp = make_local_var(env, &loc, elem_tys[pos].clone(), *sym);
                        let val_id = env.new_node(loc.clone(), elem_tys[pos].clone());
                        let val_exp = ExpData::Value(val_id, val.clone()).into_exp();
                        conditions.push(make_eq(env, &loc, var_exp, val_exp));
                    },
                    Pattern::Var(_, var_sym) => {
                        var_bindings.push((*var_sym, seq));
                    },
                    Pattern::Wildcard(_) => {},
                    _ => {
                        env.diag(
                            Severity::Bug,
                            &env.get_node_loc(pat.node_id()),
                            "unexpected pattern in primitive position of mixed tuple match",
                        );
                    },
                }
            }

            let prim_guard = conjoin(env, &loc, conditions);

            // Wrap the user's guard with primitive-position var bindings so they
            // are in scope: { let y = _$prim_0; guard }
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
            let guard_parts: Vec<Exp> = prim_guard.into_iter().chain(wrapped_user_guard).collect();
            let new_condition = conjoin(env, &loc, guard_parts);

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
            // Retype wildcard to match the non-primitive-only discriminator.
            let np_ty = if non_primitive_positions.len() == 1 {
                elem_tys[non_primitive_positions[0]].clone()
            } else {
                Type::Tuple(select_positions(elem_tys, non_primitive_positions))
            };
            let loc = env.get_node_loc(arm.pattern.node_id());
            let wc_id = env.new_node(loc, np_ty);
            MatchArm {
                loc: arm.loc.clone(),
                pattern: Pattern::Wildcard(wc_id),
                condition: arm.condition.clone(),
                body: arm.body.clone(),
            }
        },
        Pattern::Var(id, ..) => {
            // A top-level Var pattern on a mixed tuple match would require binding a
            // tuple-typed local. The type checker's NoTuple constraint rejects this
            // before the env pipeline runs, so this branch should be unreachable.
            env.diag(
                Severity::Bug,
                &env.get_node_loc(*id),
                "top-level Var pattern on mixed tuple: rejected by type checker",
            );
            arm.clone()
        },
        _ => arm.clone(),
    }
}

/// Wrap an expression with nested let-bindings for primitive-position variables.
///
/// For each `(var_sym, seq)` in `var_bindings`, generates:
/// `{ let var_sym = _$prim_seq; inner }`
///
/// Only introduces bindings for variables that are actually free in `inner`,
/// to avoid unused-variable warnings. Returns `inner` unchanged when no
/// bindings are needed.
fn wrap_with_prim_bindings(
    env: &GlobalEnv,
    loc: &Loc,
    elem_tys: &[Type],
    primitive_positions: &[usize],
    prim_temps: &[(Symbol, Exp)],
    var_bindings: &[(Symbol, usize)],
    inner: Exp,
) -> Exp {
    let builder = ExpBuilder::new(env);
    let free = inner.as_ref().free_vars();
    var_bindings
        .iter()
        .rev()
        .filter(|(var_sym, _)| free.contains(var_sym))
        .fold(inner, |acc, (var_sym, seq)| {
            let pos = primitive_positions[*seq];
            let (prim_sym, _) = &prim_temps[*seq];
            let var_pat_id = env.new_node(loc.clone(), elem_tys[pos].clone());
            let pattern = Pattern::Var(var_pat_id, *var_sym);
            let prim_ref = make_local_var(env, loc, elem_tys[pos].clone(), *prim_sym);
            builder.block(pattern, Some(prim_ref), acc)
        })
}
