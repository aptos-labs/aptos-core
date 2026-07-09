// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! VM-agnostic execution outcomes ([`ExecOutcome`]) and the coarse three-category correctness
//! check: both succeeding is a match, Move abort compares the abort code and message, other
//! failures compare by kind, and anything else is a non-match.

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
    /// The entry function returned.
    /// TODO(completeness): also compare write sets.
    Success { events: Vec<String> },
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
            ExecOutcome::Success { events: v1_events },
            ExecOutcome::Success { events: v2_events },
        ) => compare_events(v1_events, v2_events),
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

/// Compares the events emitted by the two VMs. Each event is a normalized rendering (type, kind,
/// and payload) produced by the shared testsuite renderers; events are emitted in a deterministic
/// order, so the sequences must agree element-for-element.
fn compare_events(v1: &[String], v2: &[String]) -> Correctness {
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
                detail: format!("event {} differs: V1 [{}], V2 [{}]", i, e1, e2),
            };
        }
    }
    Correctness::Match
}
