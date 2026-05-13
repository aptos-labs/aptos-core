// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

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
//!
//! Range patterns are unified with literal number patterns: a literal `n` is treated as the
//! half-open interval `[n, n+1)`, and all range comparisons use interval arithmetic.

use itertools::Itertools;
use move_model::{
    ast::{ExpData, MatchArm, Pattern, Value},
    model::{GlobalEnv, NodeId, QualifiedId, StructId},
    symbol::Symbol,
    ty::{PrimitiveType, Type},
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
    /// Half-open interval [lo, hi). `None` = type-based boundary.
    /// `Number(n)` is represented as `Range(Some(n), Some(n+1))`.
    Range(Option<BigInt>, Option<BigInt>),
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
/// `additional_missing` counts how many further constructors at this
/// level are also missing (beyond the one shown).
#[derive(Clone, Debug)]
enum WitnessPat {
    Wild,
    Ctor {
        ctor: MConstructor,
        args: Vec<WitnessPat>,
        additional_missing: usize,
    },
}

/// Get the PrimitiveType from a pattern's type, stripping references.
fn get_prim_type(env: &GlobalEnv, pat: &Pattern) -> Option<PrimitiveType> {
    let ty = env.get_node_type(pat.node_id());
    match ty.skip_reference() {
        Type::Primitive(p) => Some(*p),
        _ => None,
    }
}

/// Normalize a half-open range [lo, hi) at type boundaries:
/// If `lo == type_min`, replace with `None` (= -inf for coverage purposes).
/// If `hi == type_max + 1`, replace with `None` (= +inf for coverage purposes).
fn normalize_range(
    lo: Option<BigInt>,
    hi: Option<BigInt>,
    prim: Option<&PrimitiveType>,
) -> (Option<BigInt>, Option<BigInt>) {
    let norm_lo = match (lo, prim) {
        (Some(l), Some(p)) => {
            if let Some(min) = p.get_min_value() {
                if l == min {
                    None
                } else {
                    Some(l)
                }
            } else {
                Some(l)
            }
        },
        (l, _) => l,
    };
    let norm_hi = match (hi, prim) {
        (Some(h), Some(p)) => {
            if let Some(max) = p.get_max_value() {
                let type_end = max + 1; // one past the max
                if h == type_end {
                    None
                } else {
                    Some(h)
                }
            } else {
                Some(h)
            }
        },
        (h, _) => h,
    };
    (norm_lo, norm_hi)
}

/// Convert an AST `Pattern` to a `MatPat`.
fn pattern_to_matpat(env: &GlobalEnv, pat: &Pattern) -> MatPat {
    match pat {
        Pattern::Var(_, _) | Pattern::Wildcard(_) | Pattern::Error(_) => MatPat::Wild,
        Pattern::Tuple(_, pats) => MatPat::Ctor(
            MConstructor::Tuple(pats.len()),
            pats.iter().map(|p| pattern_to_matpat(env, p)).collect(),
        ),
        Pattern::Struct(_, sid, variant, pats) => {
            let ctor = if let Some(v) = variant {
                MConstructor::Variant(sid.to_qualified_id(), *v)
            } else {
                MConstructor::Struct(sid.to_qualified_id())
            };
            MatPat::Ctor(
                ctor,
                pats.iter().map(|p| pattern_to_matpat(env, p)).collect(),
            )
        },
        Pattern::LiteralValue(_, val) => match val {
            Value::Bool(b) => MatPat::Ctor(MConstructor::Bool(*b), vec![]),
            Value::Number(n) => {
                // Represent literal `n` as the half-open interval [n, n+1).
                let lo = n.clone();
                let hi = n + 1;
                let prim = get_prim_type(env, pat);
                let (norm_lo, norm_hi) = normalize_range(Some(lo), Some(hi), prim.as_ref());
                MatPat::Ctor(MConstructor::Range(norm_lo, norm_hi), vec![])
            },
            Value::ByteArray(bytes) => MatPat::Ctor(MConstructor::ByteArray(bytes.clone()), vec![]),
            _ => unreachable!("unsupported literal pattern value"),
        },
        Pattern::Range(_, lo, hi, inclusive) => {
            // Normalize to half-open [lo, hi).
            let norm_lo = lo.as_ref().and_then(|v| v.to_bigint());
            let norm_hi = match (hi.as_ref(), inclusive) {
                (Some(val), true) => {
                    // inclusive upper: [lo, hi+1)
                    val.to_bigint().map(|n| n + 1)
                },
                (Some(val), false) => {
                    // exclusive upper: [lo, hi)
                    val.to_bigint()
                },
                (None, _) => None, // open-ended: [lo, +inf)
            };
            let prim = get_prim_type(env, pat);
            let (norm_lo, norm_hi) = normalize_range(norm_lo, norm_hi, prim.as_ref());
            MatPat::Ctor(MConstructor::Range(norm_lo, norm_hi), vec![])
        },
    }
}

/// Number of sub-fields a constructor carries.
fn constructor_arity(env: &GlobalEnv, ctor: &MConstructor) -> usize {
    match ctor {
        MConstructor::Variant(sid, v) => env.get_struct(*sid).get_fields_of_variant(*v).count(),
        MConstructor::Struct(sid) => env.get_struct(*sid).get_fields().count(),
        MConstructor::Bool(_) | MConstructor::Range(_, _) | MConstructor::ByteArray(_) => 0,
        MConstructor::Tuple(n) => *n,
    }
}

/// Check if a row range [rlo, rhi) contains the constructor range [clo, chi).
/// `None` for lo means -infinity, `None` for hi means +infinity.
fn range_contains(
    rlo: &Option<BigInt>,
    rhi: &Option<BigInt>,
    clo: &Option<BigInt>,
    chi: &Option<BigInt>,
) -> bool {
    // Check rlo <= clo
    let lo_ok = match (rlo, clo) {
        (None, _) => true,
        (Some(_), None) => false,
        (Some(r), Some(c)) => r <= c,
    };
    // Check chi <= rhi
    let hi_ok = match (rhi, chi) {
        (_, None) => rhi.is_none(),
        (None, _) => true,
        (Some(r), Some(c)) => c <= r,
    };
    lo_ok && hi_ok
}

/// Split `[qlo, qhi)` into disjoint sub-ranges at every boundary point from
/// Range constructors in the matrix's first column that falls strictly inside
/// the query range. Each resulting sub-range is either fully contained by any
/// given matrix range or disjoint from it.
fn split_range_at_matrix_boundaries(
    matrix: &[Vec<MatPat>],
    qlo: &Option<BigInt>,
    qhi: &Option<BigInt>,
) -> Vec<(Option<BigInt>, Option<BigInt>)> {
    let mut points = BTreeSet::new();
    for row in matrix {
        if let MatPat::Ctor(MConstructor::Range(lo, hi), _) = &row[0] {
            if let Some(l) = lo {
                points.insert(l.clone());
            }
            if let Some(h) = hi {
                points.insert(h.clone());
            }
        }
    }
    // Keep only points strictly inside (qlo, qhi).
    let split_points: Vec<&BigInt> = points
        .iter()
        .filter(|p| {
            let after_lo = match qlo {
                None => true,
                Some(l) => *p > l,
            };
            let before_hi = match qhi {
                None => true,
                Some(h) => *p < h,
            };
            after_lo && before_hi
        })
        .collect();
    if split_points.is_empty() {
        return vec![(qlo.clone(), qhi.clone())];
    }
    let mut result = Vec::new();
    let mut current_lo = qlo.clone();
    for p in split_points {
        result.push((current_lo, Some(p.clone())));
        current_lo = Some(p.clone());
    }
    result.push((current_lo, qhi.clone()));
    result
}

/// Collect Range intervals from a set of constructors and sort by lower bound
/// (None = -infinity comes first).
fn collect_and_sort_intervals(
    seen: &BTreeSet<MConstructor>,
) -> Vec<(&Option<BigInt>, &Option<BigInt>)> {
    let mut intervals: Vec<(&Option<BigInt>, &Option<BigInt>)> = seen
        .iter()
        .filter_map(|c| {
            if let MConstructor::Range(lo, hi) = c {
                Some((lo, hi))
            } else {
                None
            }
        })
        .collect();
    intervals.sort_by(|a, b| match (&a.0, &b.0) {
        (None, None) => std::cmp::Ordering::Equal,
        (None, Some(_)) => std::cmp::Ordering::Less,
        (Some(_), None) => std::cmp::Ordering::Greater,
        (Some(x), Some(y)) => x.cmp(y),
    });
    intervals
}

/// Sweep sorted intervals and find the first gap, if any.
/// Returns `None` if the intervals fully cover [-inf, +inf).
/// Returns `Some((gap_lo, gap_hi))` for the first uncovered sub-range.
///
/// Assumes intervals have been through `normalize_range`, where `None`
/// means "extends to the type boundary" (not "unresolved").
fn find_first_gap(
    intervals: &[(&Option<BigInt>, &Option<BigInt>)],
) -> Option<(Option<BigInt>, Option<BigInt>)> {
    let mut coverage_end: Option<BigInt> = None;
    let mut started = false;
    for (lo, hi) in intervals {
        if !started {
            if lo.is_some() {
                return Some((None, lo.as_ref().cloned()));
            }
            started = true;
        } else if let (Some(l), Some(end)) = (lo, &coverage_end)
            && l > end
        {
            return Some((Some(end.clone()), Some(l.clone())));
        }
        match (hi, &coverage_end) {
            (None, _) => return None, // covered to +inf
            (Some(h), None) => coverage_end = Some(h.clone()),
            (Some(h), Some(end)) => {
                if h > end {
                    coverage_end = Some(h.clone());
                }
            },
        }
    }
    if let Some(end) = coverage_end {
        Some((Some(end), None))
    } else if !started {
        Some((None, None))
    } else {
        None
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
        MConstructor::Range(_, _) => {
            let intervals = collect_and_sort_intervals(seen);
            if find_first_gap(&intervals).is_none() {
                Some(seen.iter().cloned().collect())
            } else {
                None
            }
        },
        MConstructor::ByteArray(_) => None, // byte arrays are never complete
    }
}

/// Return a constructor of the type that is NOT in `seen`, plus how
/// many *additional* constructors are also missing (beyond the one
/// returned).
fn find_missing_constructor(
    env: &GlobalEnv,
    seen: &BTreeSet<MConstructor>,
) -> Option<(MConstructor, usize)> {
    let first = seen.iter().next()?;
    match first {
        MConstructor::Bool(_) => {
            let missing: Vec<MConstructor> = [MConstructor::Bool(false), MConstructor::Bool(true)]
                .into_iter()
                .filter(|c| !seen.contains(c))
                .collect();
            let additional = missing.len().saturating_sub(1);
            missing.into_iter().next().map(|c| (c, additional))
        },
        MConstructor::Variant(sid, _) => {
            let missing: Vec<MConstructor> = env
                .get_struct(*sid)
                .get_variants()
                .map(|v| MConstructor::Variant(*sid, v))
                .filter(|c| !seen.contains(c))
                .collect();
            let additional = missing.len().saturating_sub(1);
            missing.into_iter().next().map(|c| (c, additional))
        },
        MConstructor::Range(_, _) => {
            let intervals = collect_and_sort_intervals(seen);
            find_first_gap(&intervals).map(|(lo, hi)| (MConstructor::Range(lo, hi), 0))
        },
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
        MatPat::Ctor(c, args) => {
            // For Range constructors, check containment instead of equality.
            if let (MConstructor::Range(clo, chi), MConstructor::Range(rlo, rhi)) = (ctor, c) {
                if range_contains(rlo, rhi, clo, chi) {
                    let mut new_row = args.clone();
                    new_row.extend_from_slice(&row[1..]);
                    return Some(new_row);
                }
                return None;
            }
            if c == ctor {
                let mut new_row = args.clone();
                new_row.extend_from_slice(&row[1..]);
                Some(new_row)
            } else {
                None
            }
        },
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
            // For Range constructors, split at matrix boundaries so each
            // sub-range is fully contained by individual matrix rows.
            // The query is useful iff any sub-range is useful.
            if let MConstructor::Range(qlo, qhi) = c {
                let sub_ranges = split_range_at_matrix_boundaries(matrix, qlo, qhi);
                return sub_ranges.iter().any(|(lo, hi)| {
                    let sub_ctor = MConstructor::Range(lo.clone(), hi.clone());
                    let spec = specialize(matrix, &sub_ctor, 0);
                    let mut new_q: Vec<MatPat> = sub.clone();
                    new_q.extend_from_slice(&q[1..]);
                    is_useful(env, &spec, &new_q)
                });
            }
            let arity = sub.len();
            let spec = specialize(matrix, c, arity);
            let mut new_q: Vec<MatPat> = sub.clone();
            new_q.extend_from_slice(&q[1..]);
            is_useful(env, &spec, &new_q)
        },
        MatPat::Wild => {
            if let Some(all) = all_constructors_if_complete(env, &head_ctors) {
                // For ranges, use atomic intervals to handle partial overlaps.
                if matches!(all.first(), Some(MConstructor::Range(_, _))) {
                    let atomic = split_range_at_matrix_boundaries(matrix, &None, &None);
                    atomic.iter().any(|(lo, hi)| {
                        let sub_ctor = MConstructor::Range(lo.clone(), hi.clone());
                        let spec = specialize(matrix, &sub_ctor, 0);
                        is_useful(env, &spec, &q[1..])
                    })
                } else {
                    all.iter().any(|c| {
                        let arity = constructor_arity(env, c);
                        let spec = specialize(matrix, c, arity);
                        let mut new_q = vec![MatPat::Wild; arity];
                        new_q.extend_from_slice(&q[1..]);
                        is_useful(env, &spec, &new_q)
                    })
                }
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
            // Range splitting is only implemented in `is_useful`; this branch
            // is currently unreachable for Range constructors because the entry
            // point always passes `[Wild]`.
            assert!(
                !matches!(c, MConstructor::Range(_, _)),
                "collect_witnesses should not be called with a Range Ctor query; \
                 range splitting is only implemented in is_useful"
            );
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
            // For Range constructors, split into atomic intervals to
            // correctly handle partial overlaps between matrix rows.
            let effective_ctors: Vec<MConstructor> = if head_ctors
                .iter()
                .any(|c| matches!(c, MConstructor::Range(_, _)))
            {
                let mut atomic = BTreeSet::new();
                for c in &head_ctors {
                    if let MConstructor::Range(lo, hi) = c {
                        for (slo, shi) in split_range_at_matrix_boundaries(matrix, lo, hi) {
                            atomic.insert(MConstructor::Range(slo, shi));
                        }
                    }
                }
                atomic.into_iter().collect()
            } else {
                head_ctors.iter().cloned().collect()
            };
            for c in &effective_ctors {
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
                        Some((ctor, additional_missing)) => {
                            let a = constructor_arity(env, &ctor);
                            WitnessPat::Ctor {
                                ctor,
                                args: vec![WitnessPat::Wild; a],
                                additional_missing,
                            }
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
    let mut out = vec![WitnessPat::Ctor {
        ctor: ctor.clone(),
        args: sub.to_vec(),
        additional_missing: 0,
    }];
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
        let mp = pattern_to_matpat(env, &arm.pattern);
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
        let notes: Vec<String> = witnesses
            .iter()
            .map(|w| {
                assert_eq!(w.len(), 1, "witness length must equal query length");
                let pat = display_witness_pat(env, &w[0]);
                let additional = match &w[0] {
                    WitnessPat::Ctor {
                        additional_missing, ..
                    } => *additional_missing,
                    _ => 0,
                };
                let is_range_witness = contains_range_witness(&w[0]);
                if additional > 0 {
                    format!("missing `{}` (and {} more)", pat, additional)
                } else if is_range_witness {
                    format!("missing at least `{}`", pat)
                } else {
                    format!("missing `{}`", pat)
                }
            })
            .collect();
        env.error_with_notes(
            &env.get_node_loc(disc_node_id),
            "match not exhaustive",
            notes,
        );
    }
}

// ---- Witness display --------------------------------------------------------------------

/// Check whether a witness pattern contains a range constructor anywhere
/// (including nested inside structs/tuples).
fn contains_range_witness(w: &WitnessPat) -> bool {
    match w {
        WitnessPat::Wild => false,
        WitnessPat::Ctor { ctor, args, .. } => {
            matches!(ctor, MConstructor::Range(_, _)) || args.iter().any(contains_range_witness)
        },
    }
}

fn display_witness_pat(env: &GlobalEnv, w: &WitnessPat) -> String {
    match w {
        WitnessPat::Wild => "_".to_string(),
        WitnessPat::Ctor { ctor, args, .. } => match ctor {
            MConstructor::Bool(b) => format!("{}", b),
            MConstructor::Range(lo, hi) => display_range_witness(lo, hi),
            MConstructor::ByteArray(bytes) => {
                if let Ok(s) = std::str::from_utf8(bytes) {
                    format!("b\"{}\"", s)
                } else {
                    let hex: String = bytes.iter().map(|b| format!("{:02x}", b)).collect();
                    format!("x\"{}\"", hex)
                }
            },
            MConstructor::Tuple(_) => {
                let inner = args.iter().map(|a| display_witness_pat(env, a)).join(", ");
                format!("({})", inner)
            },
            MConstructor::Variant(sid, var) => display_witness_struct(env, *sid, Some(*var), args),
            MConstructor::Struct(sid) => display_witness_struct(env, *sid, None, args),
        },
    }
}

/// Display a range witness in a human-readable form.
fn display_range_witness(lo: &Option<BigInt>, hi: &Option<BigInt>) -> String {
    match (lo, hi) {
        (Some(l), Some(h)) => {
            // Check if it's a single value [n, n+1).
            let one = BigInt::from(1);
            if h == &(l + &one) {
                format!("{}", l)
            } else {
                format!("{}..{}", l, h)
            }
        },
        (Some(l), None) => format!("{}..", l),
        (None, Some(h)) => format!("..{}", h),
        (None, None) => "_".to_string(),
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
    // Display fields: positional uses `(..)` / `(a, b)`, named uses `{..}` / `{f: a}`.
    if var.is_none() || !args.is_empty() {
        let positional = struct_env
            .get_fields_optional_variant(var)
            .any(|f| f.is_positional());
        if args.iter().all(|a| matches!(a, WitnessPat::Wild)) {
            if positional {
                s.push_str("(..)");
            } else {
                s.push_str("{..}");
            }
        } else {
            let fields: Vec<String> = struct_env
                .get_fields_optional_variant(var)
                .zip(args.iter())
                .map(|(f, a)| {
                    if positional {
                        display_witness_pat(env, a)
                    } else {
                        let name = f.get_name().display(env.symbol_pool()).to_string();
                        format!("{}: {}", name, display_witness_pat(env, a))
                    }
                })
                .collect();
            if positional {
                s.push_str(&format!("({})", fields.join(", ")));
            } else {
                s.push_str(&format!("{{{}}}", fields.join(", ")));
            }
        }
    }
    s
}
