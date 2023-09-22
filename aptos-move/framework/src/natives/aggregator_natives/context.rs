// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_aggregator::{
    aggregator_change_set::{AggregatorApplyChange, AggregatorChange},
    aggregator_extension::{AggregatorData, AggregatorSnapshotState, AggregatorState},
    delta_change_set::DeltaOp,
    delta_math::DeltaHistory,
    resolver::AggregatorResolver,
    types::{AggregatorID, AggregatorValue, AggregatorVersionedID, SnapshotValue},
};
use aptos_types::state_store::state_key::StateKey;
use better_any::{Tid, TidAble};
use std::{cell::RefCell, collections::HashMap};

/// Represents a single aggregator change.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum AggregatorChangeV1 {
    // A value should be written to storage.
    Write(u128),
    // A delta should be merged with the value from storage.
    Merge(DeltaOp),
    // A value should be deleted from the storage.
    Delete,
}

/// Represents changes made by all aggregators during this context. This change
/// set can be converted into appropriate `WriteSet` and `DeltaChangeSet` by the
/// user, e.g. VM session.
pub struct AggregatorChangeSet {
    pub aggregator_v1_changes: HashMap<StateKey, AggregatorChangeV1>,
    pub aggregator_v2_changes: HashMap<AggregatorID, AggregatorChange<AggregatorID>>,
}

/// Native context that can be attached to VM `NativeContextExtensions`.
///
/// Note: table resolver is reused for fine-grained storage access.
#[derive(Tid)]
pub struct NativeAggregatorContext<'a> {
    txn_hash: [u8; 32],
    pub(crate) resolver: &'a dyn AggregatorResolver,
    pub(crate) aggregator_data: RefCell<AggregatorData>,
}

impl<'a> NativeAggregatorContext<'a> {
    /// Creates a new instance of a native aggregator context. This must be
    /// passed into VM session.
    pub fn new(txn_hash: [u8; 32], resolver: &'a dyn AggregatorResolver) -> Self {
        Self {
            txn_hash,
            resolver,
            aggregator_data: Default::default(),
        }
    }

    /// Returns the hash of transaction associated with this context.
    pub fn txn_hash(&self) -> [u8; 32] {
        self.txn_hash
    }

    /// Returns all changes made within this context (i.e. by a single
    /// transaction).
    pub fn into_change_set(self) -> AggregatorChangeSet {
        let NativeAggregatorContext {
            aggregator_data, ..
        } = self;
        let (_, destroyed_aggregators, aggregators, snapshots) =
            aggregator_data.into_inner().into();

        let mut aggregator_v1_changes = HashMap::new();
        let mut aggregator_v2_changes = HashMap::new();

        // First process all snapshots (they need access to aggregators)
        for (id, snapshot) in snapshots {
            let state = snapshot.into();
            let change = match state {
                AggregatorSnapshotState::Create {
                    value: SnapshotValue::Integer(value),
                } => Some(AggregatorChange::Create(AggregatorValue::Integer(value))),
                AggregatorSnapshotState::Create {
                    value: SnapshotValue::String(value),
                } => Some(AggregatorChange::Create(AggregatorValue::String(value))),
                AggregatorSnapshotState::Delta {
                    base_aggregator,
                    delta,
                } => {
                    let delta_op = aggregators.get(&AggregatorVersionedID::V2(base_aggregator)).map_or_else(
                        || DeltaOp::new(delta, u128::MAX, DeltaHistory::new()),
                        |v| match v.state {
                            AggregatorState::Create { .. } => unreachable!("Aggregator that snapshot in Delta state depends on cannot be in Create state"),
                            AggregatorState::Delta { history, .. } => DeltaOp::new(delta, v.max_value, history),
                        }
                    );
                    Some(AggregatorChange::Apply(
                        AggregatorApplyChange::SnapshotDelta {
                            base_aggregator,
                            delta: delta_op,
                        },
                    ))
                },
                AggregatorSnapshotState::Derived {
                    base_snapshot,
                    formula,
                } => Some(AggregatorChange::Apply(
                    AggregatorApplyChange::SnapshotDerived {
                        base_snapshot,
                        formula,
                    },
                )),
                // Not a write
                AggregatorSnapshotState::Reference { .. } => None,
            };
            if let Some(change) = change {
                aggregator_v2_changes.insert(id, change);
            }
        }

        // Second, process all aggregators.
        for (id, aggregator) in aggregators {
            let (max_value, state) = aggregator.into();
            match id {
                AggregatorVersionedID::V1(state_key) => {
                    let change = match state {
                        AggregatorState::Create { value } => AggregatorChangeV1::Write(value),
                        AggregatorState::Delta { delta, history, .. } => {
                            let delta_op = DeltaOp::new(delta, max_value, history);
                            AggregatorChangeV1::Merge(delta_op)
                        },
                    };
                    aggregator_v1_changes.insert(state_key, change);
                },
                AggregatorVersionedID::V2(id) => {
                    let change = match state {
                        AggregatorState::Create { value } => {
                            Some(AggregatorChange::Create(AggregatorValue::Integer(value)))
                        },
                        AggregatorState::Delta { delta, history, .. } => {
                            if delta.is_zero() && history.is_empty() {
                                // not a write
                                None
                            } else {
                                Some(AggregatorChange::Apply(
                                    AggregatorApplyChange::AggregatorDelta {
                                        delta: DeltaOp::new(delta, max_value, history),
                                    },
                                ))
                            }
                        },
                    };
                    if let Some(change) = change {
                        aggregator_v2_changes.insert(id, change);
                    }
                },
            }
        }

        // Additionally, do not forget to delete destroyed values from storage.
        for id in destroyed_aggregators {
            aggregator_v1_changes.insert(id, AggregatorChangeV1::Delete);
        }

        AggregatorChangeSet {
            aggregator_v1_changes,
            aggregator_v2_changes,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use aptos_aggregator::{
        aggregator_v1_id_for_test, aggregator_v1_state_key_for_test, bounded_math::SignedU128,
        delta_math::DeltaHistory, FakeAggregatorView,
    };
    use claims::{assert_matches, assert_ok, assert_some_eq};

    fn get_test_resolver() -> FakeAggregatorView {
        let mut state_view = FakeAggregatorView::default();
        state_view.set_from_state_key(aggregator_v1_state_key_for_test(500), 150);
        state_view.set_from_state_key(aggregator_v1_state_key_for_test(600), 100);
        state_view.set_from_state_key(aggregator_v1_state_key_for_test(700), 200);
        state_view.set_from_aggregator_id(AggregatorID::new(900), 300);
        state_view.set_from_aggregator_id(AggregatorID::new(1000), 400);
        state_view
    }

    // All aggregators are initialized deterministically based on their ID,
    // with the following spec.
    //
    //     +-------+---------------+-----------+-----+---------+
    //     |  key  | storage value |  create   | get | remove  |
    //     +-------+---------------+-----------+-----+---------+
    //     |  100  |               |   yes     | yes |   yes   |
    //     |  200  |               |   yes     | yes |         |
    //     |  300  |               |   yes     |     |   yes   |
    //     |  400  |               |   yes     |     |         |
    //     |  500  |               |           | yes |   yes   |
    //     |  600  |               |           | yes |         |
    //     |  700  |               |           | yes |         |
    //     |  800  |               |           |     |   yes   |
    //     |  900  |               |           |     |         |
    //     | 1000  |               |           |     |         |
    //     +-------+---------------+-----------+-----+---------+
    fn test_set_up_v1(context: &NativeAggregatorContext) {
        let mut aggregator_data = context.aggregator_data.borrow_mut();

        aggregator_data.create_new_aggregator(aggregator_v1_id_for_test(100), 100);
        aggregator_data.create_new_aggregator(aggregator_v1_id_for_test(200), 200);
        aggregator_data.create_new_aggregator(aggregator_v1_id_for_test(300), 300);
        aggregator_data.create_new_aggregator(aggregator_v1_id_for_test(400), 400);

        assert_ok!(aggregator_data.get_aggregator(aggregator_v1_id_for_test(100), 100));
        assert_ok!(aggregator_data.get_aggregator(aggregator_v1_id_for_test(200), 200));
        assert!(aggregator_data
            .get_aggregator(aggregator_v1_id_for_test(500), 500)
            .unwrap()
            .try_add(context.resolver, 150)
            .unwrap());
        assert!(aggregator_data
            .get_aggregator(aggregator_v1_id_for_test(600), 600)
            .unwrap()
            .try_add(context.resolver, 100)
            .unwrap());
        assert!(aggregator_data
            .get_aggregator(aggregator_v1_id_for_test(700), 700)
            .unwrap()
            .try_add(context.resolver, 200)
            .unwrap());

        aggregator_data.remove_aggregator_v1(aggregator_v1_id_for_test(100));
        aggregator_data.remove_aggregator_v1(aggregator_v1_id_for_test(300));
        aggregator_data.remove_aggregator_v1(aggregator_v1_id_for_test(500));
        aggregator_data.remove_aggregator_v1(aggregator_v1_id_for_test(800));
    }

    #[test]
    fn test_v1_into_change_set() {
        let resolver = get_test_resolver();
        let context = NativeAggregatorContext::new([0; 32], &resolver);
        test_set_up_v1(&context);

        let AggregatorChangeSet {
            aggregator_v1_changes,
            ..
        } = context.into_change_set();

        assert!(!aggregator_v1_changes.contains_key(&aggregator_v1_state_key_for_test(100)));
        assert_matches!(
            aggregator_v1_changes
                .get(&aggregator_v1_state_key_for_test(200))
                .unwrap(),
            AggregatorChangeV1::Write(0)
        );
        assert!(!aggregator_v1_changes.contains_key(&aggregator_v1_state_key_for_test(300)));
        assert_matches!(
            aggregator_v1_changes
                .get(&aggregator_v1_state_key_for_test(400))
                .unwrap(),
            AggregatorChangeV1::Write(0)
        );
        assert_matches!(
            aggregator_v1_changes
                .get(&aggregator_v1_state_key_for_test(500))
                .unwrap(),
            AggregatorChangeV1::Delete
        );
        let delta_100 = DeltaOp::new(SignedU128::Positive(100), 600, DeltaHistory {
            max_achieved_positive_delta: 100,
            min_achieved_negative_delta: 0,
            min_overflow_positive_delta: None,
            max_underflow_negative_delta: None,
        });
        assert_eq!(
            *aggregator_v1_changes
                .get(&aggregator_v1_state_key_for_test(600))
                .unwrap(),
            AggregatorChangeV1::Merge(delta_100)
        );
        let delta_200 = DeltaOp::new(SignedU128::Positive(200), 700, DeltaHistory {
            max_achieved_positive_delta: 200,
            min_achieved_negative_delta: 0,
            min_overflow_positive_delta: None,
            max_underflow_negative_delta: None,
        });
        assert_eq!(
            *aggregator_v1_changes
                .get(&aggregator_v1_state_key_for_test(700))
                .unwrap(),
            AggregatorChangeV1::Merge(delta_200)
        );
        assert_matches!(
            aggregator_v1_changes
                .get(&aggregator_v1_state_key_for_test(800))
                .unwrap(),
            AggregatorChangeV1::Delete
        );
    }

    fn test_set_up_v2(context: &NativeAggregatorContext) {
        let mut aggregator_data = context.aggregator_data.borrow_mut();

        assert!(aggregator_data
            .get_aggregator(AggregatorVersionedID::v2(900), 900)
            .unwrap()
            .try_add(context.resolver, 200)
            .unwrap());
        assert!(!aggregator_data
            .get_aggregator(AggregatorVersionedID::v2(900), 900)
            .unwrap()
            .try_add(context.resolver, 401)
            .unwrap());
        assert!(!aggregator_data
            .get_aggregator(AggregatorVersionedID::v2(900), 900)
            .unwrap()
            .try_sub(context.resolver, 501)
            .unwrap());

        aggregator_data.create_new_aggregator(AggregatorVersionedID::v2(1100), 1100);
        assert!(aggregator_data
            .get_aggregator(AggregatorVersionedID::v2(1100), 1100)
            .unwrap()
            .try_add(context.resolver, 200)
            .unwrap());
        assert!(!aggregator_data
            .get_aggregator(AggregatorVersionedID::v2(1100), 1100)
            .unwrap()
            .try_add(context.resolver, 1000)
            .unwrap());
        assert!(!aggregator_data
            .get_aggregator(AggregatorVersionedID::v2(1100), 1100)
            .unwrap()
            .try_sub(context.resolver, 201)
            .unwrap());
    }

    #[test]
    fn test_v2_into_change_set() {
        let resolver = get_test_resolver();
        let context = NativeAggregatorContext::new([0; 32], &resolver);
        test_set_up_v2(&context);
        let AggregatorChangeSet {
            aggregator_v2_changes,
            ..
        } = context.into_change_set();
        assert!(aggregator_v2_changes.contains_key(&AggregatorID::new(900)));
        assert!(!aggregator_v2_changes.contains_key(&AggregatorID::new(1000)));
        assert!(aggregator_v2_changes.contains_key(&AggregatorID::new(1100)));
        assert_some_eq!(
            aggregator_v2_changes.get(&AggregatorID::new(900)),
            &AggregatorChange::Apply(AggregatorApplyChange::AggregatorDelta {
                delta: DeltaOp::new(SignedU128::Positive(200), 900, DeltaHistory {
                    max_achieved_positive_delta: 200,
                    min_achieved_negative_delta: 0,
                    min_overflow_positive_delta: Some(601),
                    max_underflow_negative_delta: Some(301),
                },),
            }),
        );
        assert_some_eq!(
            aggregator_v2_changes.get(&AggregatorID::new(1100)),
            &AggregatorChange::Create(AggregatorValue::Integer(200)),
        );
    }
}
