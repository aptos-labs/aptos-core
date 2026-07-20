// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! VM-agnostic execution outcomes ([`ExecOutcome`]) and the coarse three-category correctness
//! check: both succeeding is a match, Move abort compares the abort code and message, other
//! failures compare by kind, and anything else is a non-match.
//!
//! On success the two VMs are also compared on their **write set** — the writes each made, keyed
//! by [`StateKey`] so both sides use the same `TransactionOutput` types.

use aptos_types::{
    contract_event::ContractEvent,
    state_store::state_key::StateKey,
    write_set::{TransactionWrite, WriteOp, WriteOpKind},
};
use std::collections::BTreeMap;

/// A transaction's writes, keyed by [`StateKey`]. `BTreeMap` gives a canonical order for free.
pub type WriteSet = BTreeMap<StateKey, WriteOp>;

/// A normalized, VM-agnostic class of non-abort runtime failure. The two VMs use different error
/// types, so we match on the kind rather than a raw status code or message.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FailureKind {
    /// Ran out of the gas budget.
    OutOfGas,
    /// Arithmetic overflow/underflow, division by zero, bad shift, bad cast.
    Arithmetic,
    /// `borrow_global`/`move_from` on a missing resource.
    ResourceDoesNotExist,
    /// `move_to` over an existing resource.
    ResourceAlreadyExists,
    /// Vector out-of-bounds, pop-from-empty, etc.
    VectorError,
    /// A structural runtime limit (stack/heap/value depth) was exceeded.
    RuntimeLimitExceeded,
    /// Type / reference-safety violation (paranoid checks, enum variant mismatch, etc.).
    TypeOrReferenceSafety,
    /// Missing/incompatible module, function, or struct (linking).
    Linker,
    /// A "should never happen" VM invariant violation.
    InvariantViolation,
    /// Anything not covered above.
    Other,
}

impl std::fmt::Display for FailureKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// A VM-agnostic execution outcome, in one of the three comparable categories.
pub enum ExecOutcome {
    /// The entry function returned, emitting these events and producing this write set.
    Success {
        events: Vec<ContractEvent>,
        writes: WriteSet,
    },
    /// The function executed a Move `abort` with this code. `message` is the optional abort message
    /// (populated only for the message form of abort).
    Aborted { code: u64, message: Option<String> },
    /// A non-abort runtime failure, classified by kind (with detail for reporting).
    Failure { kind: FailureKind, detail: String },
}

/// The verdict of comparing the two VMs' outcomes.
pub enum Correctness {
    /// The outcomes agree at the level we check.
    Match,
    /// The outcomes disagree. Includes the case where V2 could not execute the transaction at all —
    /// per the brief, that is just another non-match, not a softer category.
    Mismatch { detail: String },
}

/// Compares V1's outcome (V1 is the reference and is expected to always produce an outcome) with
/// V2's result. `v2` is `Err` when V2 could not execute the transaction at all (the reason is
/// surfaced as a mismatch).
pub fn compare_outcomes(v1: &ExecOutcome, v2: Result<&ExecOutcome, &str>) -> Correctness {
    let v2 = match v2 {
        Ok(v2) => v2,
        Err(reason) => {
            return Correctness::Mismatch {
                detail: format!("V2 could not execute the transaction: {}", reason),
            }
        },
    };

    match (v1, v2) {
        (
            ExecOutcome::Success {
                events: v1_events,
                writes: v1_writes,
            },
            ExecOutcome::Success {
                events: v2_events,
                writes: v2_writes,
            },
        ) => match compare_events(v1_events, v2_events) {
            Correctness::Match => compare_write_sets(v1_writes, v2_writes),
            mismatch => mismatch,
        },
        (
            ExecOutcome::Aborted {
                code: c1,
                message: m1,
            },
            ExecOutcome::Aborted {
                code: c2,
                message: m2,
            },
        ) => {
            if c1 != c2 {
                Correctness::Mismatch {
                    detail: format!(
                        "both aborted but with different codes: V1={}, V2={}",
                        c1, c2
                    ),
                }
            } else if m1 != m2 {
                Correctness::Mismatch {
                    detail: format!(
                        "both aborted with code {} but different messages: V1={:?}, V2={:?}",
                        c1, m1, m2
                    ),
                }
            } else {
                Correctness::Match
            }
        },
        (ExecOutcome::Failure { kind: k1, .. }, ExecOutcome::Failure { kind: k2, .. }) => {
            if k1 == k2 {
                Correctness::Match
            } else {
                Correctness::Mismatch {
                    detail: format!("both failed but with different kinds: V1={}, V2={}", k1, k2),
                }
            }
        },
        (v1, v2) => Correctness::Mismatch {
            detail: format!(
                "different outcome categories: V1={}, V2={}",
                describe(v1),
                describe(v2)
            ),
        },
    }
}

fn describe(outcome: &ExecOutcome) -> String {
    match outcome {
        ExecOutcome::Success { .. } => "success".to_string(),
        ExecOutcome::Aborted { code, .. } => format!("abort(code={})", code),
        ExecOutcome::Failure { kind, .. } => format!("failure({})", kind),
    }
}

/// Compares the events emitted by the two VMs. Events are emitted in a deterministic order, so the
/// sequences must agree element-for-element on type tag and payload.
fn compare_events(v1: &[ContractEvent], v2: &[ContractEvent]) -> Correctness {
    if v1.len() != v2.len() {
        return Correctness::Mismatch {
            detail: format!(
                "different event counts: V1 emitted {}, V2 emitted {}",
                v1.len(),
                v2.len()
            ),
        };
    }
    for (i, (e1, e2)) in v1.iter().zip(v2).enumerate() {
        if e1 != e2 {
            return Correctness::Mismatch {
                detail: format!(
                    "event {} differs: V1 {}, V2 {}",
                    i,
                    describe_event(e1),
                    describe_event(e2)
                ),
            };
        }
    }
    Correctness::Match
}

fn describe_event(event: &ContractEvent) -> String {
    format!(
        "{} ({} B)",
        event.type_tag().to_canonical_string(),
        event.event_data().len()
    )
}

/// Drops writes that did not actually change state, so a write set reflects real modifications.
///
/// Both VMs over-approximate: `borrow_global_mut` marks a resource written even if the borrow never
/// mutates it, leaving a `Modification` whose new bytes equal the pre-transaction value. Pruning
/// both sides makes the comparison about real state changes rather than these no-op copies.
///
/// `pre_state` returns the pre-transaction bytes, or `None` if the slot did not exist. A write is
/// dropped only when the pre-state is known and byte-identical.
pub fn prune_unchanged_modifications(
    writes: &mut WriteSet,
    pre_state: impl Fn(&StateKey) -> Option<Vec<u8>>,
) {
    writes.retain(|key, op| {
        if !matches!(op.write_op_kind(), WriteOpKind::Modification) {
            return true;
        }
        let Some(new_bytes) = op.bytes() else {
            return true;
        };
        pre_state(key).is_none_or(|old_bytes| new_bytes.as_ref() != old_bytes.as_slice())
    });
}

/// Compares the two VMs' write sets. A mismatch is reported for a key present on only one side or a
/// differing write op (kind or bytes).
///
/// The comparison is strict: because both VMs over-approximate modifications, a write only one side
/// emits — even with identical bytes — is surfaced rather than hidden.
fn compare_write_sets(v1: &WriteSet, v2: &WriteSet) -> Correctness {
    for key in v1.keys() {
        if !v2.contains_key(key) {
            return Correctness::Mismatch {
                detail: format!("write to {:?} present in V1 but not V2", key),
            };
        }
    }
    for key in v2.keys() {
        if !v1.contains_key(key) {
            return Correctness::Mismatch {
                detail: format!("write to {:?} present in V2 but not V1", key),
            };
        }
    }
    for (key, op1) in v1 {
        let op2 = &v2[key];
        if op1 != op2 {
            return Correctness::Mismatch {
                detail: format!(
                    "write to {:?} differs: V1 {}, V2 {}",
                    key,
                    describe_op(op1),
                    describe_op(op2)
                ),
            };
        }
    }
    Correctness::Match
}

fn describe_op(op: &WriteOp) -> String {
    let kind = match op.write_op_kind() {
        WriteOpKind::Creation => "creation",
        WriteOpKind::Modification => "modification",
        WriteOpKind::Deletion => "deletion",
    };
    match op.bytes() {
        Some(bytes) => format!("{kind} ({} B)", bytes.len()),
        None => kind.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(name: &str) -> StateKey {
        StateKey::raw(name.as_bytes())
    }

    fn success(writes: WriteSet) -> ExecOutcome {
        ExecOutcome::Success {
            events: vec![],
            writes,
        }
    }

    fn ws(entries: Vec<(&str, WriteOp)>) -> WriteSet {
        entries.into_iter().map(|(n, op)| (key(n), op)).collect()
    }

    fn is_match(v1: &ExecOutcome, v2: &ExecOutcome) -> bool {
        matches!(compare_outcomes(v1, Ok(v2)), Correctness::Match)
    }

    #[test]
    fn identical_write_sets_match() {
        let w = || ws(vec![("A", WriteOp::legacy_creation(vec![1, 2, 3].into()))]);
        assert!(is_match(&success(w()), &success(w())));
    }

    #[test]
    fn missing_key_is_a_mismatch() {
        let v1 = success(ws(vec![("A", WriteOp::legacy_creation(vec![1].into()))]));
        let v2 = success(ws(vec![]));
        assert!(!is_match(&v1, &v2));
        assert!(!is_match(&v2, &v1));
    }

    #[test]
    fn differing_bytes_are_a_mismatch() {
        let v1 = success(ws(vec![(
            "A",
            WriteOp::legacy_modification(vec![1, 2, 3].into()),
        )]));
        let v2 = success(ws(vec![(
            "A",
            WriteOp::legacy_modification(vec![1, 2, 4].into()),
        )]));
        assert!(!is_match(&v1, &v2));
    }

    #[test]
    fn differing_op_kind_is_a_mismatch() {
        let v1 = success(ws(vec![("A", WriteOp::legacy_creation(vec![1].into()))]));
        let v2 = success(ws(vec![(
            "A",
            WriteOp::legacy_modification(vec![1].into()),
        )]));
        assert!(!is_match(&v1, &v2));
    }

    #[test]
    fn prune_drops_only_unchanged_modifications() {
        let mut writes = ws(vec![
            ("Same", WriteOp::legacy_modification(vec![1, 2, 3].into())), // no-op: dropped
            ("Changed", WriteOp::legacy_modification(vec![9].into())),    // real change: kept
            ("New", WriteOp::legacy_creation(vec![1, 2, 3].into())),      // creation: kept
            ("Gone", WriteOp::legacy_deletion()),                         // deletion: kept
            ("Unknown", WriteOp::legacy_modification(vec![7].into())),    // no pre-state: kept
        ]);
        prune_unchanged_modifications(&mut writes, |k| {
            if *k == key("Same") || *k == key("Changed") {
                Some(vec![1, 2, 3])
            } else {
                None
            }
        });

        assert!(!writes.contains_key(&key("Same")));
        assert!(writes.contains_key(&key("Changed")));
        assert!(writes.contains_key(&key("New")));
        assert!(writes.contains_key(&key("Gone")));
        assert!(writes.contains_key(&key("Unknown")));
        assert_eq!(writes.len(), 4);
    }
}
