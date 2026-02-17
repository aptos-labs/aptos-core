// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Match exhaustiveness checking, unreachable arm detection, and primitive match transformation.
//!
//! This pass performs two tasks in a single traversal:
//! 1. **Coverage check**: verifies that match expressions are exhaustive and that all arms are
//!    reachable (no arm is shadowed by a previous arm).
//! 2. **Primitive transform**: converts match expressions over primitive types (booleans, integers,
//!    byte strings, and tuples of primitives) into if-else chains.
//!    It also handles "mixed tuple" matches where a tuple discriminator contains both primitive and
//!    non-primitive (enum/struct) elements.
//!
//! Both tasks happen inside `rewrite_match`: coverage is checked first, then the match is
//! optionally transformed if it involves primitives.
//!
//! ## Coverage algorithm
//!
//! Coverage analysis uses the matrix-based algorithm from "Warnings for pattern matching"
//! (Maranget, JFP 2007). The core idea is the *usefulness* predicate: a pattern vector `q`
//! is useful w.r.t. a pattern matrix `P` if there exists a value matched by `q` that is
//! not matched by any row in `P`.
//!
//! - **Exhaustiveness**: a wildcard `_` is useful against the full matrix implies the match
//!   is not exhaustive. Witness patterns showing the missing values are collected and
//!   reported.
//! - **Reachability**: arm `i` is useful against arms `0..i` implies arm `i` is reachable.
//!   Arms that are not useful are reported as unreachable.
//!
//! The algorithm recursively decomposes the pattern matrix column-by-column, specializing
//! by each constructor. This avoids a cross-product explosion that would occur with value
//! enumeration on deeply nested tuple patterns.

use crate::env_pipeline::rewrite_target::{
    RewriteState, RewriteTarget, RewriteTargets, RewritingScope,
};
use itertools::Itertools;
use move_model::{
    ast::{AbortKind, Exp, ExpData, MatchArm, Operation, Pattern, Value},
    exp_rewriter::ExpRewriterFunctions,
    metadata::LanguageVersion,
    model::{GlobalEnv, NodeId, QualifiedId, StructId},
    symbol::Symbol,
    ty::{PrimitiveType, Type},
    well_known,
};
use num::BigInt;
use std::collections::BTreeSet;

// ================================================================================================
// Main Entry Point

/// Check match coverage and transform primitive pattern matches in all target functions.
pub fn check_and_transform(env: &mut GlobalEnv) {
    let mut rewriter = MatchRewriter { env };
    let mut targets = RewriteTargets::create(env, RewritingScope::CompilationTarget);
    let todo: BTreeSet<_> = targets.keys().collect();
    for target in todo {
        if let RewriteTarget::MoveFun(func_id) = target {
            let func_env = rewriter.env.get_function(func_id);
            if let Some(def) = func_env.get_def().cloned() {
                let new_def = rewriter.rewrite_exp(def.clone());
                if !ExpData::ptr_eq(&new_def, &def) {
                    *targets.state_mut(&target) = RewriteState::Def(new_def);
                }
            }
        }
    }
    targets.write_to_env(env);
}

// ================================================================================================
// Rewriter (combines coverage check + primitive transformation)

struct MatchRewriter<'env> {
    env: &'env GlobalEnv,
}

impl ExpRewriterFunctions for MatchRewriter<'_> {
    fn rewrite_match(&mut self, id: NodeId, discriminator: &Exp, arms: &[MatchArm]) -> Option<Exp> {
        // First, check match coverage (exhaustiveness and reachability).
        analyze_match_coverage(self.env, discriminator.node_id(), arms);

        // Then, transform primitive matches to if-else chains or combinations of match with guards.
        // Matches over primitive types (and mixed tuples containing them) require
        // language version 2.4+.
        let fully_transformable = is_match_fully_transformable(self.env, discriminator, arms);
        let mixed_tuple =
            !fully_transformable && is_mixed_tuple_match(self.env, discriminator, arms);
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
// Coverage analysis: matrix algorithm

/// A constructor appearing in patterns.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum MConstructor {
    Variant(QualifiedId<StructId>, Symbol),
    Struct(QualifiedId<StructId>),
    Bool(bool),
    Number(BigInt),
    ByteArray(Vec<u8>),
    Tuple(usize),
}

/// A pattern in the matrix representation.
#[derive(Clone, Debug)]
enum MatPat {
    /// Wildcard / variable — matches anything.
    Wild,
    /// Constructor applied to sub-patterns.
    Ctor(MConstructor, Vec<MatPat>),
}

/// A witness pattern for displaying missing values.
#[derive(Clone, Debug)]
enum WitnessPat {
    Wild,
    Ctor(MConstructor, Vec<WitnessPat>),
}

/// Convert an AST `Pattern` to a `MatPat`.
fn pattern_to_matpat(pat: &Pattern) -> MatPat {
    match pat {
        Pattern::Var(_, _) | Pattern::Wildcard(_) | Pattern::Error(_) => MatPat::Wild,
        Pattern::Tuple(_, pats) => MatPat::Ctor(
            MConstructor::Tuple(pats.len()),
            pats.iter().map(pattern_to_matpat).collect(),
        ),
        Pattern::Struct(_, sid, variant, pats) => {
            let ctor = if let Some(v) = variant {
                MConstructor::Variant(sid.to_qualified_id(), *v)
            } else {
                MConstructor::Struct(sid.to_qualified_id())
            };
            MatPat::Ctor(ctor, pats.iter().map(pattern_to_matpat).collect())
        },
        Pattern::LiteralValue(_, val) => match val {
            Value::Bool(b) => MatPat::Ctor(MConstructor::Bool(*b), vec![]),
            Value::Number(n) => MatPat::Ctor(MConstructor::Number(n.clone()), vec![]),
            Value::ByteArray(bytes) => MatPat::Ctor(MConstructor::ByteArray(bytes.clone()), vec![]),
            _ => MatPat::Wild,
        },
    }
}

/// Number of sub-fields a constructor carries.
fn constructor_arity(env: &GlobalEnv, ctor: &MConstructor) -> usize {
    match ctor {
        MConstructor::Variant(sid, v) => env.get_struct(*sid).get_fields_of_variant(*v).count(),
        MConstructor::Struct(sid) => env.get_struct(*sid).get_fields().count(),
        MConstructor::Bool(_) | MConstructor::Number(_) | MConstructor::ByteArray(_) => 0,
        MConstructor::Tuple(n) => *n,
    }
}

/// If the `seen` constructors form a *complete* set for their type, return all
/// constructors of that type. Otherwise return `None`.
fn all_constructors_if_complete(
    env: &GlobalEnv,
    seen: &BTreeSet<MConstructor>,
) -> Option<Vec<MConstructor>> {
    let first = seen.iter().next()?;
    match first {
        MConstructor::Bool(_) => {
            let all = vec![MConstructor::Bool(false), MConstructor::Bool(true)];
            all.iter().all(|c| seen.contains(c)).then_some(all)
        },
        MConstructor::Variant(sid, _) => {
            let all: Vec<_> = env
                .get_struct(*sid)
                .get_variants()
                .map(|v| MConstructor::Variant(*sid, v))
                .collect();
            all.iter().all(|c| seen.contains(c)).then_some(all)
        },
        MConstructor::Struct(_) | MConstructor::Tuple(_) => {
            // Exactly one constructor — always complete.
            Some(seen.iter().cloned().collect())
        },
        MConstructor::Number(_) | MConstructor::ByteArray(_) => None, // integers and byte arrays are never complete
    }
}

/// Return a constructor of the type that is NOT in `seen`, for witness display.
fn find_missing_constructor(
    env: &GlobalEnv,
    seen: &BTreeSet<MConstructor>,
) -> Option<MConstructor> {
    let first = seen.iter().next()?;
    match first {
        MConstructor::Bool(_) => [MConstructor::Bool(false), MConstructor::Bool(true)]
            .into_iter()
            .find(|c| !seen.contains(c)),
        MConstructor::Variant(sid, _) => env
            .get_struct(*sid)
            .get_variants()
            .map(|v| MConstructor::Variant(*sid, v))
            .find(|c| !seen.contains(c)),
        _ => None,
    }
}

// ---- Matrix operations ------------------------------------------------------------------

/// Specialize matrix by constructor `c`: keep rows whose head matches `c`, expand sub-patterns.
fn specialize(matrix: &[Vec<MatPat>], ctor: &MConstructor, arity: usize) -> Vec<Vec<MatPat>> {
    matrix
        .iter()
        .filter_map(|row| specialize_row(row, ctor, arity))
        .collect()
}

fn specialize_row(row: &[MatPat], ctor: &MConstructor, arity: usize) -> Option<Vec<MatPat>> {
    match &row[0] {
        MatPat::Ctor(c, args) if c == ctor => {
            let mut new_row = args.clone();
            new_row.extend_from_slice(&row[1..]);
            Some(new_row)
        },
        MatPat::Ctor(_, _) => None,
        MatPat::Wild => {
            let mut new_row = vec![MatPat::Wild; arity];
            new_row.extend_from_slice(&row[1..]);
            Some(new_row)
        },
    }
}

/// Default matrix: keep only wildcard rows, remove the first column.
fn default_matrix(matrix: &[Vec<MatPat>]) -> Vec<Vec<MatPat>> {
    matrix
        .iter()
        .filter_map(|row| {
            if matches!(&row[0], MatPat::Wild) {
                Some(row[1..].to_vec())
            } else {
                None
            }
        })
        .collect()
}

// ---- Core usefulness check (short-circuits, for reachability) ---------------------------

/// Returns `true` when `q` is useful w.r.t. `matrix` (standard Maranget algorithm).
fn is_useful(env: &GlobalEnv, matrix: &[Vec<MatPat>], q: &[MatPat]) -> bool {
    if q.is_empty() {
        return matrix.is_empty();
    }
    let head_ctors: BTreeSet<MConstructor> = matrix
        .iter()
        .filter_map(|row| {
            if let MatPat::Ctor(c, _) = &row[0] {
                Some(c.clone())
            } else {
                None
            }
        })
        .collect();

    match &q[0] {
        MatPat::Ctor(c, sub) => {
            let arity = sub.len();
            let spec = specialize(matrix, c, arity);
            let mut new_q: Vec<MatPat> = sub.clone();
            new_q.extend_from_slice(&q[1..]);
            is_useful(env, &spec, &new_q)
        },
        MatPat::Wild => {
            if let Some(all) = all_constructors_if_complete(env, &head_ctors) {
                all.iter().any(|c| {
                    let arity = constructor_arity(env, c);
                    let spec = specialize(matrix, c, arity);
                    let mut new_q = vec![MatPat::Wild; arity];
                    new_q.extend_from_slice(&q[1..]);
                    is_useful(env, &spec, &new_q)
                })
            } else {
                let def = default_matrix(matrix);
                is_useful(env, &def, &q[1..])
            }
        },
    }
}

// ---- Witness collection (explores all branches, for exhaustiveness) ---------------------

/// Collect every witness pattern-vector demonstrating that `q` is useful w.r.t. `matrix`.
/// Returns an empty vec when `q` is not useful.
fn collect_witnesses(
    env: &GlobalEnv,
    matrix: &[Vec<MatPat>],
    q: &[MatPat],
) -> Vec<Vec<WitnessPat>> {
    if q.is_empty() {
        return if matrix.is_empty() {
            vec![vec![]] // one empty witness
        } else {
            vec![]
        };
    }
    let head_ctors: BTreeSet<MConstructor> = matrix
        .iter()
        .filter_map(|row| {
            if let MatPat::Ctor(c, _) = &row[0] {
                Some(c.clone())
            } else {
                None
            }
        })
        .collect();

    match &q[0] {
        MatPat::Ctor(c, sub) => {
            let arity = sub.len();
            let spec = specialize(matrix, c, arity);
            let mut new_q: Vec<MatPat> = sub.clone();
            new_q.extend_from_slice(&q[1..]);
            collect_witnesses(env, &spec, &new_q)
                .into_iter()
                .map(|w| reconstruct_witness(c, arity, w))
                .collect()
        },
        MatPat::Wild => {
            let mut all_witnesses = Vec::new();
            // Check each seen constructor (may have internal gaps).
            for c in &head_ctors {
                let arity = constructor_arity(env, c);
                let spec = specialize(matrix, c, arity);
                let mut new_q = vec![MatPat::Wild; arity];
                new_q.extend_from_slice(&q[1..]);
                for w in collect_witnesses(env, &spec, &new_q) {
                    all_witnesses.push(reconstruct_witness(c, arity, w));
                }
            }
            // Check unseen constructors via the default matrix.
            if all_constructors_if_complete(env, &head_ctors).is_none() {
                let def = default_matrix(matrix);
                for w in collect_witnesses(env, &def, &q[1..]) {
                    let head = match find_missing_constructor(env, &head_ctors) {
                        Some(c) => {
                            let a = constructor_arity(env, &c);
                            WitnessPat::Ctor(c, vec![WitnessPat::Wild; a])
                        },
                        None => WitnessPat::Wild,
                    };
                    let mut full = vec![head];
                    full.extend(w);
                    all_witnesses.push(full);
                }
            }
            all_witnesses
        },
    }
}

/// Wrap the first `arity` elements of a witness vector into a constructor.
fn reconstruct_witness(
    ctor: &MConstructor,
    arity: usize,
    witness: Vec<WitnessPat>,
) -> Vec<WitnessPat> {
    let (sub, rest) = witness.split_at(arity);
    let mut out = vec![WitnessPat::Ctor(ctor.clone(), sub.to_vec())];
    out.extend_from_slice(rest);
    out
}

// ---- Entry point for coverage analysis --------------------------------------------------

/// Analyze match coverage: check reachability of all arms and exhaustiveness.
///
/// Reachability is checked for every arm (guarded or not) against the matrix of
/// preceding *unconditional* arms. A guarded arm does not consume values (the guard
/// might be false), so it is not added to the matrix. An arm whose pattern is not
/// useful against the unconditional matrix is unreachable regardless of any guard.
///
/// Exhaustiveness is checked using only unconditional arms, since guarded arms
/// cannot guarantee coverage.
fn analyze_match_coverage(env: &GlobalEnv, disc_node_id: NodeId, arms: &[MatchArm]) {
    // Build the unconditional matrix incrementally for reachability checking.
    let mut uncond_matrix: Vec<Vec<MatPat>> = Vec::new();
    for arm in arms {
        let mp = pattern_to_matpat(&arm.pattern);
        let q = vec![mp.clone()];
        if !is_useful(env, &uncond_matrix, &q) {
            env.error(
                &env.get_node_loc(arm.pattern.node_id()),
                "unreachable pattern",
            );
        }
        // Only unconditional arms consume values in the matrix.
        if arm.condition.is_none() {
            uncond_matrix.push(q);
        }
    }

    // Exhaustiveness: check if a wildcard is still useful against the unconditional matrix.
    let witnesses = collect_witnesses(env, &uncond_matrix, &[MatPat::Wild]);
    if !witnesses.is_empty() {
        env.error_with_notes(
            &env.get_node_loc(disc_node_id),
            "match not exhaustive",
            witnesses
                .iter()
                .map(|w| {
                    assert_eq!(w.len(), 1);
                    format!("missing `{}`", display_witness_pat(env, &w[0]))
                })
                .collect(),
        );
    }
}

// ---- Witness display --------------------------------------------------------------------

fn display_witness_pat(env: &GlobalEnv, w: &WitnessPat) -> String {
    match w {
        WitnessPat::Wild => "_".to_string(),
        WitnessPat::Ctor(MConstructor::Bool(b), _) => format!("{}", b),
        WitnessPat::Ctor(MConstructor::Number(n), _) => format!("{}", n),
        WitnessPat::Ctor(MConstructor::ByteArray(bytes), _) => {
            let hex: String = bytes.iter().map(|b| format!("{:02x}", b)).collect();
            format!("x\"{}\"", hex)
        },
        WitnessPat::Ctor(MConstructor::Tuple(_), args) => {
            let inner = args.iter().map(|a| display_witness_pat(env, a)).join(",");
            format!("({})", inner)
        },
        WitnessPat::Ctor(MConstructor::Variant(sid, var), args) => {
            display_witness_struct(env, *sid, Some(*var), args)
        },
        WitnessPat::Ctor(MConstructor::Struct(sid), args) => {
            display_witness_struct(env, *sid, None, args)
        },
    }
}

fn display_witness_struct(
    env: &GlobalEnv,
    sid: QualifiedId<StructId>,
    var: Option<Symbol>,
    args: &[WitnessPat],
) -> String {
    let struct_env = env.get_struct(sid);
    let mut s = struct_env.get_name().display(env.symbol_pool()).to_string();
    if let Some(v) = var {
        s.push_str(&format!("::{}", v.display(env.symbol_pool())));
    }
    // Display struct fields, or {..} if all wildcards.
    if var.is_none() || !args.is_empty() {
        if args.iter().all(|a| matches!(a, WitnessPat::Wild)) {
            s.push_str("{..}");
        } else {
            let fields: Vec<String> = struct_env
                .get_fields_optional_variant(var)
                .map(|f| f.get_name().display(env.symbol_pool()).to_string())
                .zip(args.iter())
                .map(|(f, a)| format!("{}: {}", f, display_witness_pat(env, a)))
                .collect();
            s.push_str(&format!("{{{}}}", fields.join(", ")));
        }
    }
    s
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
        // No more arms - generate abort for incomplete match
        return generate_abort(env, result_id);
    }

    let arm = &arms[arm_idx];

    if arm.condition.is_none() && is_catch_all_pattern(&arm.pattern) {
        // Catch-all pattern - just return the body (with variable binding if needed)
        return maybe_bind_pattern(env, discriminator, &arm.pattern, &arm.body);
    }

    // Generate condition for this arm
    let condition = generate_arm_condition(env, discriminator, &arm.pattern, &arm.condition);

    // Generate the body with pattern bindings
    let then_branch = maybe_bind_pattern(env, discriminator, &arm.pattern, &arm.body);

    // Generate else branch (remaining arms)
    let else_branch = generate_if_else_chain(env, result_id, discriminator, arms, arm_idx + 1);

    // Create if-else expression
    let result_ty = env.get_node_type(result_id);
    let if_id = env.new_node(env.get_node_loc(result_id), result_ty);

    ExpData::IfElse(if_id, condition, then_branch, else_branch).into_exp()
}

/// Generate a boolean condition that tests if discriminator matches the pattern and guard.
fn generate_arm_condition(
    env: &GlobalEnv,
    discriminator: &Exp,
    pattern: &Pattern,
    guard: &Option<Exp>,
) -> Exp {
    let pattern_cond = generate_pattern_condition(env, discriminator, pattern);

    // Combine pattern condition with guard (if present)
    if let Some(guard_exp) = guard {
        let loc = env.get_node_loc(discriminator.node_id());
        let and_id = env.new_node(loc, Type::Primitive(PrimitiveType::Bool));
        ExpData::Call(and_id, Operation::And, vec![
            pattern_cond,
            guard_exp.clone(),
        ])
        .into_exp()
    } else {
        pattern_cond
    }
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
                // Type mismatch - return false
                ExpData::Value(bool_id, Value::Bool(false)).into_exp()
            }
        },

        Pattern::Struct(..) | Pattern::Error(_) => {
            // Should not reach here for primitive matches
            ExpData::Value(bool_id, Value::Bool(false)).into_exp()
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
            // Get the temp variable for this index
            let temp_pat = &temp_patterns[idx];
            if let Pattern::Var(var_id, sym) = temp_pat {
                // Generate: tmp_idx == val
                let var_exp = ExpData::LocalVar(*var_id, *sym).into_exp();
                let val_id = env.new_node(loc.clone(), tys[idx].clone());
                let val_exp = ExpData::Value(val_id, val.clone()).into_exp();
                let cmp_id = env.new_node(loc.clone(), bool_ty.clone());
                conditions
                    .push(ExpData::Call(cmp_id, Operation::Eq, vec![var_exp, val_exp]).into_exp());
            }
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
        Pattern::Tuple(_, pats) => {
            // Check if any sub-pattern has variables
            let has_vars = pats.iter().any(|p| matches!(p, Pattern::Var(..)));
            if has_vars {
                // Need to bind tuple components
                let loc = env.get_node_loc(pattern.node_id());
                let block_id = env.new_node(loc, env.get_node_type(body.node_id()));

                ExpData::Block(
                    block_id,
                    pattern.clone(),
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

            // Combine with existing guard: prim_guard && original_guard
            let new_condition = match (&prim_guard, &arm.condition) {
                (Some(pg), Some(og)) => {
                    let and_id = env.new_node(loc.clone(), bool_ty.clone());
                    Some(
                        ExpData::Call(and_id, Operation::And, vec![pg.clone(), og.clone()])
                            .into_exp(),
                    )
                },
                (Some(pg), None) => Some(pg.clone()),
                (None, Some(og)) => Some(og.clone()),
                (None, None) => None,
            };

            // If there are var bindings at primitive positions, wrap the body
            let new_body =
                var_bindings
                    .iter()
                    .rev()
                    .fold(arm.body.clone(), |body, (var_sym, seq)| {
                        let pos = primitive_positions[*seq];
                        let (prim_sym, _) = &prim_temps[*seq];
                        let body_ty = env.get_node_type(body.node_id());
                        let var_pat_id = env.new_node(loc.clone(), elem_tys[pos].clone());
                        let pattern = Pattern::Var(var_pat_id, *var_sym);
                        let prim_var_id = env.new_node(loc.clone(), elem_tys[pos].clone());
                        let prim_ref = ExpData::LocalVar(prim_var_id, *prim_sym).into_exp();
                        let block_id = env.new_node(loc.clone(), body_ty);
                        ExpData::Block(block_id, pattern, Some(prim_ref), body).into_exp()
                    });

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
