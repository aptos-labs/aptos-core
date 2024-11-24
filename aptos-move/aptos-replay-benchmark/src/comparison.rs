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

struct Write {
    state_key: StateKey,
    #[allow(dead_code)]
    write_op: WriteOp,
}

impl Write {
    fn new(state_key: StateKey, write_op: WriteOp) -> Self {
        Self {
            state_key,
            write_op,
        }
    }
}

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
        left: Option<Write>,
        right: Option<Write>,
    },
}

/// Holds all differences for a pair of transaction outputs.
pub(crate) struct Comparison {
    diffs: Vec<Diff>,
}

impl Comparison {
    /// Given a  pair of transaction outputs, computes its diff for gas used, status, events and
    /// write sets.
    pub(crate) fn diff(left: TransactionOutput, right: TransactionOutput) -> Self {
        let (left_write_set, left_events, left_gas_used, left_transaction_status, _) =
            left.unpack();
        let (right_write_set, right_events, right_gas_used, right_transaction_status, _) =
            right.unpack();

        let mut diffs = vec![];

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

    fn diff_events(diffs: &mut Vec<Diff>, left: Vec<ContractEvent>, right: Vec<ContractEvent>) {
        let mut left_ty_tags = BTreeMap::new();
        for (idx, event) in left.iter().enumerate() {
            left_ty_tags.insert(event.type_tag().clone(), idx);
        }

        let mut right_ty_tags = BTreeMap::new();
        for (idx, event) in right.iter().enumerate() {
            right_ty_tags.insert(event.type_tag().clone(), idx);
        }

        for (left_ty_tag, left_idx) in left_ty_tags {
            if let Some(right_idx) = right_ty_tags.remove(&left_ty_tag) {
                let left_data = left[left_idx].event_data();
                let right_data = right[right_idx].event_data();
                if left_data != right_data {
                    diffs.push(Diff::Event {
                        left: Some(left[left_idx].clone()),
                        right: Some(right[right_idx].clone()),
                    });
                }
            } else {
                diffs.push(Diff::Event {
                    left: Some(left[left_idx].clone()),
                    right: None,
                });
            }
        }

        for (_, right_idx) in right_ty_tags {
            diffs.push(Diff::Event {
                left: None,
                right: Some(right[right_idx].clone()),
            });
        }
    }

    fn diff_write_sets(diffs: &mut Vec<Diff>, left: WriteSet, right: WriteSet) {
        let left = left.into_mut().into_inner();
        let mut right = right.into_mut().into_inner();

        for (left_state_key, left_write_op) in left {
            if let Some(right_write_op) = right.remove(&left_state_key) {
                if left_write_op != right_write_op {
                    diffs.push(Diff::WriteSet {
                        left: Some(Write::new(left_state_key.clone(), left_write_op)),
                        right: Some(Write::new(left_state_key, right_write_op)),
                    });
                }
            } else {
                diffs.push(Diff::WriteSet {
                    left: Some(Write::new(left_state_key, left_write_op)),
                    right: None,
                });
            }
        }

        for (right_state_key, right_write_op) in right {
            diffs.push(Diff::WriteSet {
                left: None,
                right: Some(Write::new(right_state_key, right_write_op)),
            });
        }
    }

    pub(crate) fn is_ok(&self) -> bool {
        self.diffs.is_empty()
    }
}

impl std::fmt::Display for Comparison {
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
                            &right.unwrap().state_key
                        )?;
                    } else if right.is_none() {
                        writeln!(
                            f,
                            "[write] {:?} is not written to anymore",
                            &left.unwrap().state_key
                        )?;
                    } else {
                        writeln!(
                            f,
                            "[write] {:?} has changed its value",
                            &left.unwrap().state_key
                        )?;
                    }
                },
            }
        }
        writeln!(f, " <<<<< ")
    }
}
