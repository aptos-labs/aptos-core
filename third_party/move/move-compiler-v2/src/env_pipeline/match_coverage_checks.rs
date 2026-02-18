// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Match exhaustiveness checking and unreachable arm detection.
//!
//! Uses the matrix-based algorithm from "Warnings for pattern matching"
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

use itertools::Itertools;
use move_model::{
    ast::{ExpData, MatchArm, Pattern, Value},
    model::{GlobalEnv, NodeId, QualifiedId, StructId},
    symbol::Symbol,
};
use num::BigInt;
use std::collections::BTreeSet;

// ================================================================================================
// Main Entry Point

/// Check match coverage (exhaustiveness and reachability) in all target functions.
pub fn check(env: &mut GlobalEnv) {
    for module in env.get_modules() {
        if module.is_target() {
            for func in module.get_functions() {
                if let Some(def) = func.get_def().cloned() {
                    def.visit_pre_order(&mut |e| {
                        if let ExpData::Match(_, discriminator, arms) = e {
                            analyze_match_coverage(env, discriminator.node_id(), arms);
                        }
                        true
                    });
                }
            }
        }
    }
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
