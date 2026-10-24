// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Comparison of the two VMs' transaction outputs. Both replays run
//! configured to be gas-free, so allow for byte-for-byte equivalence.

use aptos_types::{
    contract_event::ContractEvent,
    state_store::state_key::StateKey,
    transaction::{TransactionOutput, TransactionStatus},
    write_set::{TransactionWrite, WriteOp, WriteOpKind},
};
use std::collections::{BTreeMap, BTreeSet};

/// One way V2's output differs from the reference (V1's).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DiffField {
    Status {
        v1: TransactionStatus,
        v2: TransactionStatus,
    },
    /// Both replays are gas-free; a nonzero difference means the zero-gas plumbing broke on one
    /// side.
    GasUsed { v1: u64, v2: u64 },
    /// A write present on one side only, or with a different kind, bytes or metadata.
    Write {
        key: StateKey,
        v1: Option<WriteOp>,
        v2: Option<WriteOp>,
    },
    /// A native-position write (the trading-native bucket the storage applier commits beside the
    /// main write set) present on one side only, or different. Compared strictly: none is known.
    NativePosition {
        key: StateKey,
        v1: Option<WriteOp>,
        v2: Option<WriteOp>,
    },
    /// Known: the same write, but only V1's carries state-value metadata. MonoMove does not emit
    /// metadata yet (`aptos-transaction-executor` `txn_output.rs`).
    MissingMetadata { key: StateKey },
    /// Known: V2 alone writes a key with the bytes it already held. MonoMove emits every
    /// copied-on-write entry as a write (`aptos-transaction-executor` `txn_output.rs`).
    UnchangedWrite { key: StateKey },
    /// Keys made hot on one side only.
    HotState {
        v1: Vec<StateKey>,
        v2: Vec<StateKey>,
    },
    /// The event sequences differ in length, so they are not compared element by element.
    EventCount { v1: usize, v2: usize },
    /// Events are emitted in a deterministic order, so the sequences must agree element by
    /// element.
    Event {
        index: usize,
        v1: ContractEvent,
        v2: ContractEvent,
    },
}

impl DiffField {
    /// Whether this is a difference MonoMove is known to have: the outputs then agree on what the
    /// transaction did, but are not byte-identical.
    pub fn is_known(&self) -> bool {
        match self {
            DiffField::MissingMetadata { .. } | DiffField::UnchangedWrite { .. } => true,
            DiffField::Status { .. }
            | DiffField::GasUsed { .. }
            | DiffField::Write { .. }
            | DiffField::NativePosition { .. }
            | DiffField::HotState { .. }
            | DiffField::EventCount { .. }
            | DiffField::Event { .. } => false,
        }
    }

    pub fn describe(&self) -> String {
        match self {
            DiffField::Status { v1, v2 } => format!("statuses differ: V1={:?}, V2={:?}", v1, v2),
            DiffField::GasUsed { v1, v2 } => format!(
                "gas used differs under zero-gas replay: V1={}, V2={}",
                v1, v2
            ),
            DiffField::Write { key, v1, v2 } => match (v1, v2) {
                (Some(_), None) => format!("write to {:?} present in V1 but not V2", key),
                (None, Some(_)) => format!("write to {:?} present in V2 but not V1", key),
                (v1, v2) => format!(
                    "write to {:?} differs: V1 {}, V2 {}",
                    key,
                    describe_op(v1.as_ref()),
                    describe_op(v2.as_ref())
                ),
            },
            DiffField::NativePosition { key, v1, v2 } => format!(
                "native-position write to {:?} differs: V1 {}, V2 {}",
                key,
                describe_op(v1.as_ref()),
                describe_op(v2.as_ref())
            ),
            DiffField::MissingMetadata { key } => {
                format!("write to {:?} carries metadata in V1 only (known)", key)
            },
            DiffField::UnchangedWrite { key } => format!(
                "write to {:?} in V2 only, with the bytes it already held (known)",
                key
            ),
            DiffField::HotState { v1, v2 } => {
                format!("hot-state keys differ: V1 only {:?}, V2 only {:?}", v1, v2)
            },
            DiffField::EventCount { v1, v2 } => format!(
                "different event counts: V1 emitted {}, V2 emitted {}",
                v1, v2
            ),
            DiffField::Event { index, v1, v2 } => format!(
                "event {} differs: V1 {}, V2 {}",
                index,
                describe_event(v1),
                describe_event(v2)
            ),
        }
    }
}

/// How the differences between two outputs judge them.
pub enum Verdict {
    /// No difference.
    Match,
    /// Only [known MonoMove differences](DiffField::is_known).
    KnownDifference,
    /// At least one other difference.
    Mismatch,
}

/// The [`Verdict`] of `diffs`.
pub fn classify(diffs: &[DiffField]) -> Verdict {
    if diffs.is_empty() {
        Verdict::Match
    } else if diffs.iter().all(DiffField::is_known) {
        Verdict::KnownDifference
    } else {
        Verdict::Mismatch
    }
}

/// Every difference between V2's output and the reference (V1's), in the order status, gas used,
/// writes (by key), native-position writes (by key), hot-state keys, events. Nothing is pruned: a difference MonoMove is known to
/// have is reported as such (see [`DiffField::is_known`]), so the caller decides what it means.
pub fn diff_outputs(
    v1: &TransactionOutput,
    v2: &TransactionOutput,
    pre_state: impl Fn(&StateKey) -> Option<Vec<u8>>,
) -> Vec<DiffField> {
    let mut diffs = vec![];
    if v1.status() != v2.status() {
        diffs.push(DiffField::Status {
            v1: v1.status().clone(),
            v2: v2.status().clone(),
        });
    }
    if v1.gas_used() != v2.gas_used() {
        diffs.push(DiffField::GasUsed {
            v1: v1.gas_used(),
            v2: v2.gas_used(),
        });
    }
    let writes = |output: &TransactionOutput| -> BTreeMap<StateKey, WriteOp> {
        output
            .write_set()
            .write_op_iter()
            .map(|(key, op)| (key.clone(), op.clone()))
            .collect()
    };
    diff_write_sets(writes(v1), writes(v2), &pre_state, &mut diffs);
    // The native-position bucket is committed too, beside the main write set: compared strictly.
    let positions = |output: &TransactionOutput| -> BTreeMap<StateKey, WriteOp> {
        output
            .write_set()
            .native_position_iter()
            .map(|(key, op)| (key.clone(), op.as_write_op().clone()))
            .collect()
    };
    diffs.extend(
        paired(positions(v1), positions(v2))
            .filter(|(_, op1, op2)| op1 != op2)
            .map(|(key, v1, v2)| DiffField::NativePosition { key, v1, v2 }),
    );
    let hot = |output: &TransactionOutput| -> BTreeSet<StateKey> {
        output.write_set().hotness_keys().cloned().collect()
    };
    let (hot1, hot2) = (hot(v1), hot(v2));
    if hot1 != hot2 {
        diffs.push(DiffField::HotState {
            v1: hot1.difference(&hot2).cloned().collect(),
            v2: hot2.difference(&hot1).cloned().collect(),
        });
    }
    diff_events(v1.events(), v2.events(), &mut diffs);
    diffs
}

/// Every key of `v1` or `v2`, in key order, with its value on each side.
fn paired<V>(
    mut v1: BTreeMap<StateKey, V>,
    mut v2: BTreeMap<StateKey, V>,
) -> impl Iterator<Item = (StateKey, Option<V>, Option<V>)> {
    let keys: BTreeSet<StateKey> = v1.keys().chain(v2.keys()).cloned().collect();
    keys.into_iter().map(move |key| {
        let (a, b) = (v1.remove(&key), v2.remove(&key));
        (key, a, b)
    })
}

/// Every key either VM writes, compared by kind, bytes and metadata.
fn diff_write_sets(
    v1: BTreeMap<StateKey, WriteOp>,
    v2: BTreeMap<StateKey, WriteOp>,
    pre_state: &impl Fn(&StateKey) -> Option<Vec<u8>>,
    diffs: &mut Vec<DiffField>,
) {
    for (key, op1, op2) in paired(v1, v2) {
        let diff = match (op1, op2) {
            (Some(op1), Some(op2)) => {
                let same_value =
                    op1.write_op_kind() == op2.write_op_kind() && op1.bytes() == op2.bytes();
                if same_value && op1.metadata() == op2.metadata() {
                    None
                } else if same_value && op2.metadata().is_none() {
                    Some(DiffField::MissingMetadata { key })
                } else {
                    Some(DiffField::Write {
                        key,
                        v1: Some(op1),
                        v2: Some(op2),
                    })
                }
            },
            (None, Some(op2)) if is_unchanged_modification(&key, &op2, pre_state) => {
                Some(DiffField::UnchangedWrite { key })
            },
            (op1, op2) => Some(DiffField::Write {
                key,
                v1: op1,
                v2: op2,
            }),
        };
        diffs.extend(diff);
    }
}

/// Whether the write is a modification whose bytes equal the pre-transaction value.
fn is_unchanged_modification(
    key: &StateKey,
    op: &WriteOp,
    pre_state: &impl Fn(&StateKey) -> Option<Vec<u8>>,
) -> bool {
    matches!(op.write_op_kind(), WriteOpKind::Modification)
        && op
            .bytes()
            .is_some_and(|new| pre_state(key).is_some_and(|old| new.as_ref() == old.as_slice()))
}

fn diff_events(v1: &[ContractEvent], v2: &[ContractEvent], diffs: &mut Vec<DiffField>) {
    if v1.len() != v2.len() {
        diffs.push(DiffField::EventCount {
            v1: v1.len(),
            v2: v2.len(),
        });
        return;
    }
    for (index, (e1, e2)) in v1.iter().zip(v2).enumerate() {
        if e1 != e2 {
            diffs.push(DiffField::Event {
                index,
                v1: e1.clone(),
                v2: e2.clone(),
            });
        }
    }
}

fn describe_event(event: &ContractEvent) -> String {
    format!(
        "{} ({} B)",
        event.type_tag().to_canonical_string(),
        event.event_data().len()
    )
}

/// How a write op changes its slot.
pub(crate) fn op_kind_name(kind: WriteOpKind) -> &'static str {
    match kind {
        WriteOpKind::Creation => "creation",
        WriteOpKind::Modification => "modification",
        WriteOpKind::Deletion => "deletion",
    }
}

fn describe_op(op: Option<&WriteOp>) -> String {
    let Some(op) = op else {
        return "absent".to_string();
    };
    let kind = op_kind_name(op.write_op_kind());
    let metadata = if op.metadata().is_none() {
        ""
    } else {
        " with metadata"
    };
    match op.bytes() {
        Some(bytes) => format!("{kind} ({} B){metadata}", bytes.len()),
        None => format!("{kind}{metadata}"),
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

    fn verdict(
        v1: &TransactionOutput,
        v2: &TransactionOutput,
        pre_state: impl Fn(&StateKey) -> Option<Vec<u8>>,
    ) -> Verdict {
        classify(&diff_outputs(v1, v2, pre_state))
    }

    fn is_match(v1: &TransactionOutput, v2: &TransactionOutput) -> bool {
        matches!(verdict(v1, v2, |_| None), Verdict::Match)
    }

    fn is_mismatch(v1: &TransactionOutput, v2: &TransactionOutput) -> bool {
        matches!(verdict(v1, v2, |_| None), Verdict::Mismatch)
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
        assert!(is_mismatch(&v1, &v2));
    }

    #[test]
    fn missing_key_is_a_mismatch() {
        let v1 = success(
            vec![("A", WriteOp::legacy_creation(vec![1].into()))],
            vec![],
        );
        let v2 = success(vec![], vec![]);
        assert!(is_mismatch(&v1, &v2));
        assert!(is_mismatch(&v2, &v1));
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
        assert!(is_mismatch(&v1, &v2));
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
        assert!(is_mismatch(&v1, &v2));
    }

    #[test]
    fn native_position_writes_are_compared() {
        use aptos_types::write_set::NativePositionOp;
        let with_position = |bytes: Vec<u8>| {
            let mut write_set = WriteSetMut::new(vec![])
                .freeze()
                .expect("write set freezes");
            write_set.add_native_positions(
                [(
                    key("P"),
                    NativePositionOp::from_write_op(WriteOp::legacy_modification(bytes.into())),
                )]
                .into_iter()
                .collect(),
            );
            TransactionOutput::new(
                write_set,
                vec![],
                0,
                TransactionStatus::Keep(ExecutionStatus::Success),
                TransactionAuxiliaryData::default(),
            )
        };
        // The same position on both sides matches; a different one, or one on one side only, is a
        // mismatch that is never known.
        assert!(is_match(&with_position(vec![1]), &with_position(vec![1])));
        let diffs = diff_outputs(&with_position(vec![1]), &with_position(vec![2]), |_| None);
        assert!(matches!(diffs.as_slice(), [
            DiffField::NativePosition { .. }
        ]));
        assert!(!diffs[0].is_known());
        assert!(is_mismatch(
            &with_position(vec![1]),
            &success(vec![], vec![])
        ));
        assert!(is_mismatch(
            &success(vec![], vec![]),
            &with_position(vec![1])
        ));
    }

    #[test]
    fn differing_event_payload_is_a_mismatch() {
        let v1 = success(vec![], vec![event(vec![1])]);
        let v2 = success(vec![], vec![event(vec![2])]);
        assert!(is_mismatch(&v1, &v2));
    }

    #[test]
    fn one_sided_noop_modification_is_a_known_difference() {
        // V2 over-approximates: it also "writes" B, but with the pre-state
        // bytes. That is a known difference, never a match.
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
            verdict(&v1, &v2, pre_state),
            Verdict::KnownDifference
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
            verdict(&v1, &v2_changed, pre_state),
            Verdict::Mismatch
        ));
    }

    #[test]
    fn every_difference_is_reported() {
        let v1 = success(
            vec![
                ("A", WriteOp::legacy_modification(vec![1].into())),
                ("B", WriteOp::legacy_creation(vec![2].into())),
            ],
            vec![event(vec![1]), event(vec![2])],
        );
        let v2 = output(
            TransactionStatus::Keep(ExecutionStatus::MiscellaneousError(None)),
            vec![
                ("A", WriteOp::legacy_modification(vec![9].into())),
                ("C", WriteOp::legacy_creation(vec![3].into())),
            ],
            vec![event(vec![1]), event(vec![7])],
        );
        let diffs = diff_outputs(&v1, &v2, |_| None);
        let kinds: Vec<_> = diffs
            .iter()
            .map(|d| match d {
                DiffField::Status { .. } => "status".to_string(),
                DiffField::GasUsed { .. } => "gas".to_string(),
                DiffField::Write { key, .. } => format!("write {:?}", key),
                DiffField::NativePosition { key, .. } => format!("native position {:?}", key),
                DiffField::MissingMetadata { key } => format!("metadata {:?}", key),
                DiffField::UnchangedWrite { key } => format!("unchanged {:?}", key),
                DiffField::HotState { .. } => "hot state".to_string(),
                DiffField::EventCount { .. } => "event count".to_string(),
                DiffField::Event { index, .. } => format!("event {}", index),
            })
            .collect();
        assert_eq!(kinds, vec![
            "status".to_string(),
            format!("write {:?}", key("A")),
            format!("write {:?}", key("B")),
            format!("write {:?}", key("C")),
            "event 1".to_string(),
        ]);
        assert!(matches!(classify(&diffs), Verdict::Mismatch));
    }

    #[test]
    fn write_v1_alone_makes_is_a_mismatch_even_if_unchanged() {
        // V2 omitting a write V1 commits is a real difference, unlike the known reverse.
        let v1 = success(
            vec![("B", WriteOp::legacy_modification(vec![1, 2, 3].into()))],
            vec![],
        );
        let v2 = success(vec![], vec![]);
        let pre_state = |k: &StateKey| (*k == key("B")).then(|| vec![1, 2, 3]);
        let diffs = diff_outputs(&v1, &v2, pre_state);
        assert_eq!(diffs.len(), 1);
        assert!(!diffs[0].is_known());
        assert!(matches!(verdict(&v1, &v2, pre_state), Verdict::Mismatch));
    }

    #[test]
    fn metadata_only_v1_has_is_known_and_the_reverse_is_not() {
        use aptos_types::{
            on_chain_config::CurrentTimeMicroseconds, state_store::state_value::StateValueMetadata,
        };
        let metadata =
            StateValueMetadata::placeholder(&CurrentTimeMicroseconds { microseconds: 7 });
        let with = || WriteOp::creation(vec![1].into(), metadata.clone());
        let without = || WriteOp::legacy_creation(vec![1].into());

        let diffs = diff_outputs(
            &success(vec![("A", with())], vec![]),
            &success(vec![("A", without())], vec![]),
            |_| None,
        );
        assert_eq!(diffs, vec![DiffField::MissingMetadata { key: key("A") }]);
        assert!(diffs[0].is_known());

        let diffs = diff_outputs(
            &success(vec![("A", without())], vec![]),
            &success(vec![("A", with())], vec![]),
            |_| None,
        );
        assert!(matches!(diffs.as_slice(), [DiffField::Write { .. }]));
        assert!(!diffs[0].is_known());
    }
}
