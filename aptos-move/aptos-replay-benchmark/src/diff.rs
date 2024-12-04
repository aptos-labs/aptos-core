// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_types::{
    contract_event::ContractEvent,
    state_store::state_key::StateKey,
    transaction::{ExecutionStatus, TransactionOutput},
    write_set::{WriteOp, WriteSet},
};
use claims::assert_ok;
use std::collections::BTreeMap;

/// Different parts of [TransactionOutput] that can be different:
///   1. gas used,
///   2. status (must be kept since transactions are replayed),
///   3. events,
///   4. writes.
/// Note that fine-grained comparison allows for some differences to be okay, e.g., using more gas
/// implies that the fee statement event, the account balance of the fee payer, and the total token
/// supply are different.
enum Diff {
    GasUsed {
        left: u64,
        right: u64,
    },
    ExecutionStatus {
        left: ExecutionStatus,
        right: ExecutionStatus,
    },
    Event {
        left: Option<ContractEvent>,
        right: Option<ContractEvent>,
    },
    WriteSet {
        left: Option<(StateKey, WriteOp)>,
        right: Option<(StateKey, WriteOp)>,
    },
}

/// Holds all differences for a pair of transaction outputs.
pub(crate) struct TransactionDiff {
    diffs: Vec<Diff>,
}

impl TransactionDiff {
    /// Returns an empty diff - there are no differences between transaction outputs.
    pub(crate) fn empty() -> Self {
        Self { diffs: vec![] }
    }

    /// Given a pair of transaction outputs, computes its [TransactionDiff] that includes the gas
    /// used, execution status, events and write sets.
    // TODO: Make comparison configurable, so we can skip gas differences, etc.
    pub(crate) fn from_outputs(left: TransactionOutput, right: TransactionOutput) -> Self {
        let (left_write_set, left_events, left_gas_used, left_transaction_status, _) =
            left.unpack();
        let (right_write_set, right_events, right_gas_used, right_transaction_status, _) =
            right.unpack();

        let mut diffs = vec![];

        // All statuses must be kept, since we are replaying transactions.
        let left_execution_status = assert_ok!(left_transaction_status.as_kept_status());
        let right_execution_status = assert_ok!(right_transaction_status.as_kept_status());
        if left_execution_status != right_execution_status {
            diffs.push(Diff::ExecutionStatus {
                left: left_execution_status,
                right: right_execution_status,
            });
        }

        if left_gas_used != right_gas_used {
            diffs.push(Diff::GasUsed {
                left: left_gas_used,
                right: right_gas_used,
            });
        }

        Self::diff_events(&mut diffs, left_events, right_events);
        Self::diff_write_sets(&mut diffs, left_write_set, right_write_set);

        Self { diffs }
    }

    /// Returns true if the diff is empty, and transaction outputs match.
    pub(crate) fn is_empty(&self) -> bool {
        self.diffs.is_empty()
    }

    /// Computes the differences between a pair of event vectors, and adds them to the diff.
    fn diff_events(diffs: &mut Vec<Diff>, left: Vec<ContractEvent>, right: Vec<ContractEvent>) {
        let event_vec_to_map = |events: Vec<ContractEvent>| {
            let mut ty_tagged_events = BTreeMap::new();
            for event in events {
                ty_tagged_events.insert(event.type_tag().clone(), event);
            }
            ty_tagged_events
        };

        let left = event_vec_to_map(left);
        let mut right = event_vec_to_map(right);

        for (left_ty_tag, left_event) in left {
            if let Some(right_event) = right.remove(&left_ty_tag) {
                if left_event.event_data() != right_event.event_data() {
                    diffs.push(Diff::Event {
                        left: Some(left_event),
                        right: Some(right_event),
                    });
                }
            } else {
                diffs.push(Diff::Event {
                    left: Some(left_event),
                    right: None,
                });
            }
        }

        for right_event in right.into_values() {
            diffs.push(Diff::Event {
                left: None,
                right: Some(right_event),
            });
        }
    }

    /// Computes the differences between a pair of write sets, and adds them to the diff.
    fn diff_write_sets(diffs: &mut Vec<Diff>, left: WriteSet, right: WriteSet) {
        let left = left.into_mut().into_inner();
        let mut right = right.into_mut().into_inner();

        for (left_state_key, left_write_op) in left {
            if let Some(right_write_op) = right.remove(&left_state_key) {
                if left_write_op != right_write_op {
                    diffs.push(Diff::WriteSet {
                        left: Some((left_state_key.clone(), left_write_op)),
                        right: Some((left_state_key, right_write_op)),
                    });
                }
            } else {
                diffs.push(Diff::WriteSet {
                    left: Some((left_state_key, left_write_op)),
                    right: None,
                });
            }
        }

        for (right_state_key, right_write_op) in right {
            diffs.push(Diff::WriteSet {
                left: None,
                right: Some((right_state_key, right_write_op)),
            });
        }
    }
}

impl std::fmt::Display for TransactionDiff {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, " >>>>> ")?;
        for diff in &self.diffs {
            match diff {
                Diff::GasUsed { left, right } => {
                    writeln!(f, "[gas used] before: {}, after: {}", left, right)?;
                },
                Diff::ExecutionStatus { left, right } => {
                    writeln!(
                        f,
                        "[execution status] before: {:?}, after: {:?}",
                        left, right
                    )?;
                },
                Diff::Event { left, right } => {
                    let left = left.as_ref();
                    let right = right.as_ref();

                    if left.is_none() {
                        writeln!(
                            f,
                            "[event] {} was not emitted before",
                            right.unwrap().type_tag().to_canonical_string()
                        )?;
                    } else if right.is_none() {
                        writeln!(
                            f,
                            "[event] {} is not emitted anymore",
                            left.unwrap().type_tag().to_canonical_string()
                        )?;
                    } else {
                        writeln!(
                            f,
                            "[event] {} has changed its data",
                            left.unwrap().type_tag().to_canonical_string()
                        )?;
                    }
                },
                Diff::WriteSet { left, right } => {
                    let left = left.as_ref();
                    let right = right.as_ref();

                    if left.is_none() {
                        writeln!(
                            f,
                            "[write] {:?} was not written to before",
                            &right.unwrap().0
                        )?;
                    } else if right.is_none() {
                        writeln!(
                            f,
                            "[write] {:?} is not written to anymore",
                            &left.unwrap().0
                        )?;
                    } else {
                        writeln!(f, "[write] {:?} has changed its value", &left.unwrap().0)?;
                    }
                },
            }
        }
        writeln!(f, " <<<<< ")
    }
}

// TODO: Add tests here once we allow fine-grained comparisons between events and other parts of
//       the outputs.
