// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_types::{
    account_config::{
        CoinStoreResource, ConcurrentSupplyResource, FungibleStoreResource, ObjectGroupResource,
    },
    contract_event::ContractEvent,
    fee_statement::FeeStatement,
    state_store::state_key::StateKey,
    transaction::{ExecutionStatus, TransactionOutput},
    write_set::{TransactionWrite, WriteOp, WriteSet, TOTAL_SUPPLY_STATE_KEY},
    AptosCoinType,
};
use claims::assert_ok;
use move_core_types::{
    account_address::AccountAddress, language_storage::TypeTag, move_resource::MoveStructType,
};
use std::{collections::BTreeMap, str::FromStr};

/// Different parts of [TransactionOutput] that can be different:
///   1. gas used,
///   2. status (must be kept since transactions are replayed),
///   3. events,
///   4. writes.
/// Note that fine-grained comparison allows for some differences to be okay, e.g., using more gas
/// implies that the fee statement event, the account balance of the fee payer, and the total token
/// supply are different.
#[derive(Clone, Eq, PartialEq)]
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
        state_key: StateKey,
        left: Option<WriteOp>,
        right: Option<WriteOp>,
    },
}

/// Holds all differences for a pair of transaction outputs.
pub(crate) struct TransactionDiff {
    diffs: Vec<Diff>,
}

impl TransactionDiff {
    pub(crate) fn is_empty(&self) -> bool {
        self.diffs.is_empty()
    }

    pub(crate) fn println(&self) {
        if self.is_empty() {
            return;
        }

        use colored::Colorize;

        for diff in &self.diffs {
            println!("{}", "<<<<<<< BEFORE".yellow());
            match diff {
                Diff::GasUsed { left, right } => {
                    println!("{}", format!("gas_used: {:?}", left).green());
                    println!("{}", "========".yellow());
                    println!("{}", format!("gas_used: {:?}", right).red());
                },
                Diff::ExecutionStatus { left, right } => {
                    println!("{}", format!("execution_status: {:?}", left).green());
                    println!("{}", "========".yellow());
                    println!("{}", format!("execution_status: {:?}", right).red());
                },
                Diff::Event { left, right } => {
                    let left = left.as_ref();
                    let right = right.as_ref();

                    if left.is_none() {
                        let event_name = right.unwrap().type_tag().to_canonical_string();
                        println!("{}", "========".yellow());
                        println!("{}", format!("event {:?} emitted", event_name).red());
                    } else if right.is_none() {
                        let event_name = left.unwrap().type_tag().to_canonical_string();
                        println!("{}", format!("event {:?} emitted", event_name).green());
                        println!("{}", "========".yellow());
                    } else {
                        let event_name = left.unwrap().type_tag().to_canonical_string();
                        println!(
                            "{}",
                            format!(
                                "event {:?} data: {:?}",
                                event_name,
                                left.unwrap().event_data()
                            )
                            .green()
                        );
                        println!("{}", "========".yellow());
                        println!(
                            "{}",
                            format!(
                                "event {:?} data: {:?}",
                                event_name,
                                right.unwrap().event_data()
                            )
                            .red()
                        );
                    }
                },
                Diff::WriteSet {
                    state_key,
                    left,
                    right,
                } => {
                    let left = left.as_ref();
                    let right = right.as_ref();

                    if left.is_none() {
                        println!("{}", "========".yellow());
                        println!("{}", format!("write {:?}", state_key).red());
                    } else if right.is_none() {
                        println!("{}", format!("write {:?}", state_key).green());
                        println!("{}", "========".yellow());
                    } else {
                        println!(
                            "{}",
                            format!("write {:?} op {:?}", state_key, left.unwrap()).green()
                        );
                        println!("{}", "========".yellow());
                        println!(
                            "{}",
                            format!("write {:?} op {:?}", state_key, right.unwrap()).red()
                        );
                    }
                },
            }
            println!("{}", ">>>>>>> AFTER".yellow());
        }
    }
}

/// Builds [TransactionDiff]s for transaction outputs. The builder can be configured to ignore the
/// differences in outputs sometimes.
pub(crate) struct TransactionDiffBuilder {
    /// If true, differences related to the gas usage are ignored. These include:
    ///   - total gas used is not compared,
    ///   - `EmitFeeStatement` event is not compared,
    ///   - total APT supply is not compared,
    ///   - account balances are no compared.
    allow_different_gas_usage: bool,
}

impl TransactionDiffBuilder {
    pub(crate) fn new(allow_different_gas_usage: bool) -> Self {
        Self {
            allow_different_gas_usage,
        }
    }

    /// Given a pair of transaction outputs, computes its [TransactionDiff] that includes the gas
    /// used, execution status, events and write sets.
    pub(crate) fn build_from_outputs(
        &self,
        left: TransactionOutput,
        right: TransactionOutput,
        fee_payer: Option<AccountAddress>,
    ) -> TransactionDiff {
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

        if left_gas_used != right_gas_used && !self.allow_different_gas_usage {
            diffs.push(Diff::GasUsed {
                left: left_gas_used,
                right: right_gas_used,
            });
        }

        diffs.extend(self.diff_events(left_events, right_events));
        diffs.extend(self.diff_write_sets(left_write_set, right_write_set, fee_payer));

        TransactionDiff { diffs }
    }

    /// Computes the differences between a pair of event vectors.
    fn diff_events(&self, left: Vec<ContractEvent>, right: Vec<ContractEvent>) -> Vec<Diff> {
        let event_vec_to_map = |events: Vec<ContractEvent>| {
            events
                .into_iter()
                .map(|event| (event.type_tag().clone(), event))
                .collect::<BTreeMap<_, _>>()
        };

        let left = event_vec_to_map(left);
        let mut right = event_vec_to_map(right);

        let mut diffs = vec![];
        for (left_ty_tag, left_event) in left {
            let maybe_right_event = right.remove(&left_ty_tag);
            if maybe_right_event
                .as_ref()
                .is_some_and(|right_event| left_event.event_data() == right_event.event_data())
            {
                continue;
            }

            // If there are two fee statement events, and we allow different gas usage - ignore the
            // comparison.
            if self.allow_different_gas_usage
                && left_ty_tag == TypeTag::Struct(Box::new(FeeStatement::struct_tag()))
                && maybe_right_event.is_some()
            {
                continue;
            }

            diffs.push(Diff::Event {
                left: Some(left_event),
                right: maybe_right_event,
            });
        }

        for right_event in right.into_values() {
            diffs.push(Diff::Event {
                left: None,
                right: Some(right_event),
            });
        }
        diffs
    }

    /// Computes the differences between a pair of write sets.
    fn diff_write_sets(
        &self,
        left: WriteSet,
        right: WriteSet,
        fee_payer: Option<AccountAddress>,
    ) -> Vec<Diff> {
        let mut left = left.into_mut().into_inner();
        let mut right = right.into_mut().into_inner();

        let filter_gas_related_ops = |ops: &mut BTreeMap<StateKey, WriteOp>| {
            // Skip total coin APT supply comparisons.
            ops.remove(&*TOTAL_SUPPLY_STATE_KEY);

            // Total supply for fungible store. Note that this sadly does not work well for
            // comparisons between FA and non-FA since we write the full group, so even if supply
            // changes are removed, we still fail comparison on the rest of the group members...
            patch_object_group(ops, &AccountAddress::from_str("0xa").unwrap(), |group| {
                group.group.remove(&ConcurrentSupplyResource::struct_tag());
            });

            if let Some(fee_payer) = fee_payer {
                // Skip changes to fee payer's coin balance.
                let coin_resource_key = StateKey::resource(
                    &fee_payer,
                    &CoinStoreResource::<AptosCoinType>::struct_tag(),
                )
                .unwrap();
                ops.remove(&coin_resource_key);

                // Skip changes to fee payer's FA balance.
                patch_object_group(ops, &fee_payer, |group| {
                    group.group.remove(&FungibleStoreResource::struct_tag());
                });
            }
        };

        // For comparison without gas, we can simply evict all gas-related ops (and inner ops in
        // resource groups).
        if self.allow_different_gas_usage {
            filter_gas_related_ops(&mut left);
            filter_gas_related_ops(&mut right);
        }

        let mut diffs = vec![];
        for (state_key, left_write_op) in left {
            let maybe_right_write_op = right.remove(&state_key);
            if maybe_right_write_op
                .as_ref()
                .is_some_and(|right_write_op| right_write_op == &left_write_op)
            {
                // Both write ops exist and are the same.
                continue;
            }

            diffs.push(Diff::WriteSet {
                state_key,
                left: Some(left_write_op),
                right: maybe_right_write_op,
            });
        }

        for (state_key, right_write_op) in right {
            diffs.push(Diff::WriteSet {
                state_key,
                left: None,
                right: Some(right_write_op),
            });
        }
        diffs
    }
}

fn patch_object_group<F>(ops: &mut BTreeMap<StateKey, WriteOp>, addr: &AccountAddress, action: F)
where
    F: FnOnce(&mut ObjectGroupResource),
{
    let object_group_key = StateKey::resource_group(addr, &ObjectGroupResource::struct_tag());
    if let Some(w) = ops.get_mut(&object_group_key) {
        if let Some(bytes) = w.bytes().cloned() {
            let mut group = bcs::from_bytes::<ObjectGroupResource>(&bytes).unwrap();
            action(&mut group);
            let patched_bytes = bcs::to_bytes(&group).unwrap();
            w.set_bytes(patched_bytes.into());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aptos_types::{
        on_chain_config::CurrentTimeMicroseconds,
        state_store::state_value::StateValueMetadata,
        transaction::{TransactionAuxiliaryData, TransactionStatus},
        write_set::WriteSetMut,
    };

    #[test]
    fn test_diff_gas_used() {
        let output_1 = TransactionOutput::new(
            WriteSet::new(vec![]).unwrap(),
            vec![],
            1,
            TransactionStatus::Keep(ExecutionStatus::Success),
            TransactionAuxiliaryData::None,
        );
        let output_2 = TransactionOutput::new(
            WriteSet::new(vec![]).unwrap(),
            vec![],
            2,
            TransactionStatus::Keep(ExecutionStatus::Success),
            TransactionAuxiliaryData::None,
        );

        let diff = TransactionDiffBuilder::new(false).build_from_outputs(
            output_1.clone(),
            output_2.clone(),
            None,
        );
        assert_eq!(diff.diffs.len(), 1);
        assert!(diff.diffs[0].clone() == Diff::GasUsed { left: 1, right: 2 });

        let diff = TransactionDiffBuilder::new(true).build_from_outputs(output_1, output_2, None);
        assert!(diff.diffs.is_empty());
    }

    #[test]
    fn test_diff_status() {
        let output_1 = TransactionOutput::new(
            WriteSet::new(vec![]).unwrap(),
            vec![],
            1,
            TransactionStatus::Keep(ExecutionStatus::Success),
            TransactionAuxiliaryData::None,
        );
        let output_2 = TransactionOutput::new(
            WriteSet::new(vec![]).unwrap(),
            vec![],
            1,
            TransactionStatus::Keep(ExecutionStatus::OutOfGas),
            TransactionAuxiliaryData::None,
        );

        let diff = TransactionDiffBuilder::new(false).build_from_outputs(
            output_1.clone(),
            output_2.clone(),
            None,
        );
        assert_eq!(diff.diffs.len(), 1);
        assert!(
            diff.diffs[0].clone()
                == Diff::ExecutionStatus {
                    left: ExecutionStatus::Success,
                    right: ExecutionStatus::OutOfGas
                }
        );
    }

    #[test]
    fn test_diff_events() {
        let events_1 = vec![
            ContractEvent::new_v2_with_type_tag_str("0x1::event::EventA", vec![0, 1, 2]),
            ContractEvent::new_v2_with_type_tag_str("0x1::event::EventB", vec![0, 1, 2]),
            ContractEvent::new_v2_with_type_tag_str("0x1::event::EventD", vec![0, 1, 2]),
        ];

        let events_2 = vec![
            ContractEvent::new_v2_with_type_tag_str("0x1::event::EventA", vec![0, 1, 2]),
            ContractEvent::new_v2_with_type_tag_str("0x1::event::EventC", vec![0, 1, 2]),
            ContractEvent::new_v2_with_type_tag_str("0x1::event::EventD", vec![0, 1, 3]),
        ];

        let expected_diffs = vec![
            Diff::Event {
                left: Some(ContractEvent::new_v2_with_type_tag_str(
                    "0x1::event::EventB",
                    vec![0, 1, 2],
                )),
                right: None,
            },
            Diff::Event {
                left: None,
                right: Some(ContractEvent::new_v2_with_type_tag_str(
                    "0x1::event::EventC",
                    vec![0, 1, 2],
                )),
            },
            Diff::Event {
                left: Some(ContractEvent::new_v2_with_type_tag_str(
                    "0x1::event::EventD",
                    vec![0, 1, 2],
                )),
                right: Some(ContractEvent::new_v2_with_type_tag_str(
                    "0x1::event::EventD",
                    vec![0, 1, 3],
                )),
            },
        ];

        let diffs = TransactionDiffBuilder::new(false).diff_events(events_1, events_2);
        assert_eq!(diffs.len(), 3);
        assert!(diffs.iter().all(|diff| expected_diffs.contains(diff)));
    }

    #[test]
    fn test_diff_events_allow_different_gas_usage() {
        let fee_statement_tag = TypeTag::Struct(Box::new(FeeStatement::struct_tag()));

        let events_1 =
            vec![ContractEvent::new_v2(fee_statement_tag.clone(), vec![0, 1, 2]).unwrap()];
        let events_2 = vec![ContractEvent::new_v2(fee_statement_tag, vec![0, 1, 3]).unwrap()];

        let diffs = TransactionDiffBuilder::new(true).diff_events(events_1.clone(), events_2);
        assert!(diffs.is_empty());

        let diffs = TransactionDiffBuilder::new(true).diff_events(events_1, vec![]);
        assert_eq!(diffs.len(), 1);
    }

    #[test]
    fn test_diff_write_sets() {
        let write_set_1 = WriteSetMut::new(vec![
            // Same in 2nd write set.
            (
                StateKey::raw(b"key-1"),
                WriteOp::legacy_creation(vec![0, 1, 2].into()),
            ),
            // Does not exist in 2nd write set.
            (
                StateKey::raw(b"key-2"),
                WriteOp::legacy_creation(vec![0, 1, 2].into()),
            ),
            // Different from 2nd write-set.
            (
                StateKey::raw(b"key-4"),
                WriteOp::legacy_creation(vec![0, 1, 2].into()),
            ),
            (
                StateKey::raw(b"key-5"),
                WriteOp::legacy_creation(vec![0, 1, 2].into()),
            ),
            (
                StateKey::raw(b"key-6"),
                WriteOp::creation(
                    vec![0, 1, 2].into(),
                    StateValueMetadata::new(1, 2, &CurrentTimeMicroseconds { microseconds: 100 }),
                ),
            ),
        ])
        .freeze()
        .unwrap();

        let write_set_2 = WriteSetMut::new(vec![
            // Same in 1st write set.
            (
                StateKey::raw(b"key-1"),
                WriteOp::legacy_creation(vec![0, 1, 2].into()),
            ),
            // Does nto exist in 1st write set.
            (
                StateKey::raw(b"key-3"),
                WriteOp::legacy_creation(vec![0, 1, 2].into()),
            ),
            // Different from 1st write-set.
            (
                StateKey::raw(b"key-4"),
                WriteOp::legacy_creation(vec![0, 1, 3].into()),
            ),
            (
                StateKey::raw(b"key-5"),
                WriteOp::legacy_modification(vec![0, 1, 2].into()),
            ),
            (
                StateKey::raw(b"key-6"),
                WriteOp::creation(
                    vec![0, 1, 2].into(),
                    StateValueMetadata::new(1, 2, &CurrentTimeMicroseconds { microseconds: 200 }),
                ),
            ),
        ])
        .freeze()
        .unwrap();

        let expected_diffs = vec![
            Diff::WriteSet {
                state_key: StateKey::raw(b"key-2"),
                left: Some(WriteOp::legacy_creation(vec![0, 1, 2].into())),
                right: None,
            },
            Diff::WriteSet {
                state_key: StateKey::raw(b"key-3"),
                left: None,
                right: Some(WriteOp::legacy_creation(vec![0, 1, 2].into())),
            },
            Diff::WriteSet {
                state_key: StateKey::raw(b"key-4"),
                left: Some(WriteOp::legacy_creation(vec![0, 1, 2].into())),
                right: Some(WriteOp::legacy_creation(vec![0, 1, 3].into())),
            },
            Diff::WriteSet {
                state_key: StateKey::raw(b"key-5"),
                left: Some(WriteOp::legacy_creation(vec![0, 1, 2].into())),
                right: Some(WriteOp::legacy_modification(vec![0, 1, 2].into())),
            },
            Diff::WriteSet {
                state_key: StateKey::raw(b"key-6"),
                left: Some(WriteOp::creation(
                    vec![0, 1, 2].into(),
                    StateValueMetadata::new(1, 2, &CurrentTimeMicroseconds { microseconds: 100 }),
                )),
                right: Some(WriteOp::creation(
                    vec![0, 1, 2].into(),
                    StateValueMetadata::new(1, 2, &CurrentTimeMicroseconds { microseconds: 200 }),
                )),
            },
        ];

        let diffs =
            TransactionDiffBuilder::new(false).diff_write_sets(write_set_1, write_set_2, None);
        assert_eq!(diffs.len(), 5);
        assert!(diffs.iter().all(|diff| expected_diffs.contains(diff)));
    }
}
