// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Comparison of the two VMs' transaction outputs. Both replays run
//! configured to be gas-free, so allow for byte-for-byte equivalence.

use aptos_types::{
    contract_event::ContractEvent,
    state_store::state_key::StateKey,
    transaction::TransactionOutput,
    write_set::{TransactionWrite, WriteOp, WriteOpKind},
};
use std::collections::BTreeMap;

/// The verdict of comparing the two VMs' outputs.
pub enum Correctness {
    /// The outputs agree at the level we check.
    Match,
    /// The outputs disagree. Includes the case where V2 could not execute the
    /// transaction at all.
    Mismatch { detail: String },
}

/// Compares V2's result against the reference (V1's output).
///
/// No-ops (modifications whose bytes equal the pre-transaction state) are pruned
/// from both write sets before comparison.
pub fn compare_outputs(
    v1: &TransactionOutput,
    v2: Result<&TransactionOutput, &str>,
    pre_state: impl Fn(&StateKey) -> Option<Vec<u8>>,
) -> Correctness {
    let v2 = match v2 {
        Ok(v2) => v2,
        Err(reason) => {
            return Correctness::Mismatch {
                detail: format!("V2 could not execute the transaction: {}", reason),
            }
        },
    };

    if v1.status() != v2.status() {
        return Correctness::Mismatch {
            detail: format!(
                "statuses differ: V1={:?}, V2={:?}",
                v1.status(),
                v2.status()
            ),
        };
    }

    // Both replays are gas-free; a nonzero charge means the zero-gas plumbing
    // broke on one side.
    if v1.gas_used() != v2.gas_used() {
        return Correctness::Mismatch {
            detail: format!(
                "gas used differs under zero-gas replay: V1={}, V2={}",
                v1.gas_used(),
                v2.gas_used()
            ),
        };
    }

    match compare_write_sets(&real_writes(v1, &pre_state), &real_writes(v2, &pre_state)) {
        Correctness::Match => compare_events(v1.events(), v2.events()),
        mismatch => mismatch,
    }
}

/// The output's write set with no-op modifications pruned.
fn real_writes<'a>(
    output: &'a TransactionOutput,
    pre_state: &impl Fn(&StateKey) -> Option<Vec<u8>>,
) -> BTreeMap<&'a StateKey, &'a WriteOp> {
    output
        .write_set()
        .write_op_iter()
        .filter(|(key, op)| !is_noop_modification(key, op, pre_state))
        .collect()
}

/// Whether the write is a modification whose bytes match the pre-transaction
/// state, i.e. not a real state change.
fn is_noop_modification(
    key: &StateKey,
    op: &WriteOp,
    pre_state: impl Fn(&StateKey) -> Option<Vec<u8>>,
) -> bool {
    matches!(op.write_op_kind(), WriteOpKind::Modification)
        && op
            .bytes()
            .is_some_and(|new| pre_state(key).is_some_and(|old| new.as_ref() == old.as_slice()))
}

/// Compares the two write sets: the same keys, and per key the same op kind
/// and bytes. Metadata is gas bookkeeping (slot deposits, creation
/// timestamps), which the gas-free replay does not model, so it is
/// deliberately not compared.
fn compare_write_sets(
    v1: &BTreeMap<&StateKey, &WriteOp>,
    v2: &BTreeMap<&StateKey, &WriteOp>,
) -> Correctness {
    for (key, op1) in v1 {
        let Some(op2) = v2.get(*key) else {
            return Correctness::Mismatch {
                detail: format!("write to {:?} present in V1 but not V2", key),
            };
        };
        if op1.write_op_kind() != op2.write_op_kind() || op1.bytes() != op2.bytes() {
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
    if let Some(key) = v2.keys().find(|key| !v1.contains_key(*key)) {
        return Correctness::Mismatch {
            detail: format!("write to {:?} present in V2 but not V1", key),
        };
    }
    Correctness::Match
}

/// Compares the events emitted by the two VMs. Events are emitted in a
/// deterministic order, so the sequences must agree element-for-element.
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
    use aptos_types::{
        transaction::{ExecutionStatus, TransactionAuxiliaryData, TransactionStatus},
        write_set::WriteSetMut,
    };

    fn key(name: &str) -> StateKey {
        StateKey::raw(name.as_bytes())
    }

    fn output(
        status: TransactionStatus,
        writes: Vec<(&str, WriteOp)>,
        events: Vec<ContractEvent>,
    ) -> TransactionOutput {
        let write_set = WriteSetMut::new(writes.into_iter().map(|(n, op)| (key(n), op)))
            .freeze()
            .expect("write set freezes");
        TransactionOutput::new(
            write_set,
            events,
            0,
            status,
            TransactionAuxiliaryData::default(),
        )
    }

    fn success(writes: Vec<(&str, WriteOp)>, events: Vec<ContractEvent>) -> TransactionOutput {
        output(
            TransactionStatus::Keep(ExecutionStatus::Success),
            writes,
            events,
        )
    }

    fn event(payload: Vec<u8>) -> ContractEvent {
        use std::str::FromStr;
        let type_tag = move_core_types::language_storage::TypeTag::from_str("0x1::test::Event")
            .expect("valid type tag");
        ContractEvent::new_v2(type_tag, payload).expect("valid event")
    }

    fn is_match(v1: &TransactionOutput, v2: &TransactionOutput) -> bool {
        matches!(compare_outputs(v1, Ok(v2), |_| None), Correctness::Match)
    }

    #[test]
    fn identical_outputs_match() {
        let make = || {
            success(
                vec![("A", WriteOp::legacy_creation(vec![1, 2, 3].into()))],
                vec![event(vec![7])],
            )
        };
        assert!(is_match(&make(), &make()));
    }

    #[test]
    fn status_difference_is_a_mismatch() {
        let v1 = success(vec![], vec![]);
        let v2 = output(
            TransactionStatus::Keep(ExecutionStatus::MiscellaneousError(None)),
            vec![],
            vec![],
        );
        assert!(!is_match(&v1, &v2));
    }

    #[test]
    fn missing_key_is_a_mismatch() {
        let v1 = success(
            vec![("A", WriteOp::legacy_creation(vec![1].into()))],
            vec![],
        );
        let v2 = success(vec![], vec![]);
        assert!(!is_match(&v1, &v2));
        assert!(!is_match(&v2, &v1));
    }

    #[test]
    fn differing_bytes_are_a_mismatch() {
        let v1 = success(
            vec![("A", WriteOp::legacy_modification(vec![1, 2, 3].into()))],
            vec![],
        );
        let v2 = success(
            vec![("A", WriteOp::legacy_modification(vec![1, 2, 4].into()))],
            vec![],
        );
        assert!(!is_match(&v1, &v2));
    }

    #[test]
    fn differing_op_kind_is_a_mismatch() {
        let v1 = success(
            vec![("A", WriteOp::legacy_creation(vec![1].into()))],
            vec![],
        );
        let v2 = success(
            vec![("A", WriteOp::legacy_modification(vec![1].into()))],
            vec![],
        );
        assert!(!is_match(&v1, &v2));
    }

    #[test]
    fn differing_event_payload_is_a_mismatch() {
        let v1 = success(vec![], vec![event(vec![1])]);
        let v2 = success(vec![], vec![event(vec![2])]);
        assert!(!is_match(&v1, &v2));
    }

    #[test]
    fn v2_not_running_is_a_mismatch() {
        let v1 = success(vec![], vec![]);
        let Correctness::Mismatch { detail } = compare_outputs(&v1, Err("it broke"), |_| None)
        else {
            panic!("expected Mismatch");
        };
        assert!(detail.contains("it broke"), "{detail}");
    }

    #[test]
    fn one_sided_noop_modification_is_pruned() {
        // V2 over-approximates: it also "writes" B, but with the pre-state
        // bytes. Pruning treats that as no write, so the outputs match.
        let v1 = success(
            vec![("A", WriteOp::legacy_modification(vec![9].into()))],
            vec![],
        );
        let v2 = success(
            vec![
                ("A", WriteOp::legacy_modification(vec![9].into())),
                ("B", WriteOp::legacy_modification(vec![1, 2, 3].into())),
            ],
            vec![],
        );
        let pre_state = |k: &StateKey| (*k == key("B")).then(|| vec![1, 2, 3]);
        assert!(matches!(
            compare_outputs(&v1, Ok(&v2), pre_state),
            Correctness::Match
        ));

        // A changed write to B (bytes differ from pre-state) is still real.
        let v2_changed = success(
            vec![
                ("A", WriteOp::legacy_modification(vec![9].into())),
                ("B", WriteOp::legacy_modification(vec![4, 5].into())),
            ],
            vec![],
        );
        assert!(matches!(
            compare_outputs(&v1, Ok(&v2_changed), pre_state),
            Correctness::Mismatch { .. }
        ));
    }
}
