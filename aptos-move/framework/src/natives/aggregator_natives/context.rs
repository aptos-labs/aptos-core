// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_aggregator::{
    aggregator_v1_extension::{AggregatorData, AggregatorState, MaterializedAggregator},
    bounded_math::SignedU128,
    delayed_change::DelayedChange,
    delayed_field_extension::DelayedFieldData,
    delta_change_set::DeltaOp,
    resolver::{AggregatorV1Resolver, DelayedFieldResolver},
};
use aptos_gas_meter::AptosGasMeter;
use aptos_types::{
    state_store::{state_key::StateKey, state_value::StateValueMetadata},
    write_set::{TransactionWrite, WriteOp},
};
use aptos_vm_types::{
    change_set::WriteOpInfo,
    storage::{change_set_configs::ChangeSetSizeTracker, space_pricing::ChargeAndRefund},
};
use better_any::{Tid, TidAble};
use move_binary_format::errors::{PartialVMResult, VMResult};
use move_core_types::value::MoveTypeLayout;
use move_vm_runtime::native_extensions::VersionControlledNativeExtension;
use move_vm_types::delayed_values::delayed_field_id::DelayedFieldID;
use std::{
    cell::RefCell,
    collections::{BTreeMap, HashSet},
    sync::Arc,
};

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
    pub aggregator_v1_changes: BTreeMap<StateKey, AggregatorChangeV1>,
    pub delayed_field_changes: BTreeMap<DelayedFieldID, DelayedChange<DelayedFieldID>>,
    pub reads_needing_exchange: BTreeMap<StateKey, (StateValueMetadata, u64, Arc<MoveTypeLayout>)>,
    pub group_reads_needing_exchange: BTreeMap<StateKey, (StateValueMetadata, u64)>,
}

/// Native context that can be attached to VM `NativeContextExtensions`.
///
/// Note: table resolver is reused for fine-grained storage access.
#[derive(Tid)]
pub struct NativeAggregatorContext<'a> {
    txn_hash: [u8; 32],
    pub(crate) aggregator_v1_resolver: &'a dyn AggregatorV1Resolver,
    pub(crate) aggregator_v1_data: RefCell<AggregatorData>,
    pub(crate) delayed_field_optimization_enabled: bool,
    pub(crate) delayed_field_resolver: &'a dyn DelayedFieldResolver,
    pub(crate) delayed_field_data: RefCell<DelayedFieldData>,
}

impl<'a> VersionControlledNativeExtension for NativeAggregatorContext<'a> {
    fn undo(&mut self) {
        self.delayed_field_data.borrow_mut().undo();
        self.aggregator_v1_data.borrow_mut().undo();
    }

    fn save(&mut self) {
        self.delayed_field_data.borrow_mut().undo();
        self.aggregator_v1_data.borrow_mut().save();
    }

    fn update(&mut self, txn_hash: &[u8; 32], _script_hash: &[u8]) {
        // Note: nothing to update for delayed fields.
        self.txn_hash = *txn_hash;
        self.aggregator_v1_data.borrow_mut().update();
    }
}

impl<'a> NativeAggregatorContext<'a> {
    /// Creates a new instance of a native aggregator context. This must be
    /// passed into VM session.
    pub fn new(
        txn_hash: [u8; 32],
        aggregator_v1_resolver: &'a dyn AggregatorV1Resolver,
        delayed_field_optimization_enabled: bool,
        delayed_field_resolver: &'a dyn DelayedFieldResolver,
    ) -> Self {
        Self {
            txn_hash,
            aggregator_v1_resolver,
            aggregator_v1_data: Default::default(),
            delayed_field_resolver,
            delayed_field_optimization_enabled,
            delayed_field_data: Default::default(),
        }
    }

    /// Returns the hash of transaction associated with this context.
    pub fn txn_hash(&self) -> [u8; 32] {
        self.txn_hash
    }

    pub fn materialize(
        &self,
        new_slot_metadata: &Option<StateValueMetadata>,
        inherit_metadata: bool,
    ) -> PartialVMResult<HashSet<DelayedFieldID>> {
        self.aggregator_v1_data.borrow_mut().materialize(
            self.aggregator_v1_resolver,
            new_slot_metadata,
            inherit_metadata,
        )?;
        let delayed_field_ids = self.delayed_field_data.borrow_mut().materialize();
        Ok(delayed_field_ids)
    }

    pub fn charge_write_ops(
        &self,
        change_set_size_tracker: &mut ChangeSetSizeTracker,
        gas_meter: &mut impl AptosGasMeter,
    ) -> VMResult<()> {
        let mut aggregator_v1_data = self.aggregator_v1_data.borrow_mut();
        for res in aggregator_v1_data.write_ops_iter() {
            let (state_key, write_op, prev_size) = res?;
            if let Some(pricing) = change_set_size_tracker.disk_pricing {
                let ChargeAndRefund { charge, refund } = pricing.charge_refund_write_op(
                    change_set_size_tracker.txn_gas_params.unwrap(),
                    WriteOpInfo {
                        key: state_key,
                        op_size: write_op.write_op_size(),
                        prev_size,
                        metadata_mut: write_op.metadata_mut(),
                    },
                );
                change_set_size_tracker.write_fee += charge;
                change_set_size_tracker.total_refund += refund;
            }
            change_set_size_tracker.record_write_op(state_key, write_op.write_op_size())?;
            gas_meter.charge_io_gas_for_write(state_key, &write_op.write_op_size())?;
        }
        Ok(())
    }

    pub fn take_writes(
        &self,
    ) -> VMResult<(
        BTreeMap<DelayedFieldID, DelayedChange<DelayedFieldID>>,
        BTreeMap<StateKey, WriteOp>,
        BTreeMap<StateKey, DeltaOp>,
    )> {
        let mut aggregator_v1_write_set = BTreeMap::new();
        let mut aggregator_v1_delta_set = BTreeMap::new();

        let mut aggregator_v1_data = self.aggregator_v1_data.borrow_mut();
        for res in aggregator_v1_data.take_materialized_writes() {
            let (state_key, write) = res?;
            match write {
                MaterializedAggregator::Write(write_op, _) => {
                    aggregator_v1_write_set.insert(state_key, write_op);
                },
                MaterializedAggregator::Delta(delta_op) => {
                    aggregator_v1_delta_set.insert(state_key, delta_op);
                },
            }
        }

        Ok((
            self.delayed_field_data.borrow_mut().take_latest(),
            aggregator_v1_write_set,
            aggregator_v1_delta_set,
        ))
    }

    /// Returns all changes made within this context (i.e. by a single
    /// transaction).
    pub fn into_change_set(self) -> PartialVMResult<AggregatorChangeSet> {
        let NativeAggregatorContext {
            aggregator_v1_data,
            delayed_field_data,
            ..
        } = self;
        let mut aggregator_v1_changes = BTreeMap::new();

        // First, process all writes and deltas.
        for (id, maybe_aggregator) in aggregator_v1_data.into_inner().into() {
            let aggregator = match maybe_aggregator {
                Some(aggregator) => aggregator,
                None => {
                    aggregator_v1_changes.insert(id.0, AggregatorChangeV1::Delete);
                    continue;
                },
            };

            let (value, state, limit, history) = aggregator.into();

            let change = match state {
                AggregatorState::Data => AggregatorChangeV1::Write(value),
                AggregatorState::PositiveDelta => {
                    let history = history.unwrap();
                    let plus = SignedU128::Positive(value);
                    let delta_op = DeltaOp::new(plus, limit, history);
                    AggregatorChangeV1::Merge(delta_op)
                },
                AggregatorState::NegativeDelta => {
                    let history = history.unwrap();
                    let minus = SignedU128::Negative(value);
                    let delta_op = DeltaOp::new(minus, limit, history);
                    AggregatorChangeV1::Merge(delta_op)
                },
            };
            aggregator_v1_changes.insert(id.0, change);
        }

        let delayed_field_changes = delayed_field_data.into_inner().into();
        let delayed_write_set_ids = delayed_field_changes
            .keys()
            .cloned()
            .collect::<HashSet<_>>();
        Ok(AggregatorChangeSet {
            aggregator_v1_changes,
            delayed_field_changes,
            // is_empty check covers both whether delayed fields are enabled or not, as well as whether there
            // are any changes that would require computing reads needing exchange.
            // TODO[agg_v2](optimize) we only later compute the write set, so cannot pass the correct skip values here.
            reads_needing_exchange: if delayed_write_set_ids.is_empty() {
                BTreeMap::new()
            } else {
                self.delayed_field_resolver
                    .get_reads_needing_exchange(&delayed_write_set_ids, &HashSet::new())?
            },
            group_reads_needing_exchange: if delayed_write_set_ids.is_empty() {
                BTreeMap::new()
            } else {
                self.delayed_field_resolver
                    .get_group_reads_needing_exchange(&delayed_write_set_ids, &HashSet::new())?
            },
        })
    }

    #[cfg(test)]
    fn into_delayed_fields(self) -> BTreeMap<DelayedFieldID, DelayedChange<DelayedFieldID>> {
        let NativeAggregatorContext {
            delayed_field_data, ..
        } = self;
        delayed_field_data.into_inner().into()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use aptos_aggregator::{
        aggregator_v1_id_for_test, aggregator_v1_state_key_for_test, bounded_math::SignedU128,
        delayed_change::DelayedApplyChange, delta_change_set::DeltaWithMax,
        delta_math::DeltaHistory, tests::types::FAKE_AGGREGATOR_VIEW_GEN_ID_START,
        types::DelayedFieldValue, FakeAggregatorView,
    };
    use aptos_types::delayed_fields::{
        calculate_width_for_integer_embedded_string, SnapshotToStringFormula,
    };
    use claims::{assert_err, assert_matches, assert_ok, assert_ok_eq, assert_some_eq};

    fn get_test_resolver_v1() -> FakeAggregatorView {
        let mut state_view = FakeAggregatorView::default();
        state_view.set_from_state_key(aggregator_v1_state_key_for_test(500), 150);
        state_view.set_from_state_key(aggregator_v1_state_key_for_test(600), 100);
        state_view.set_from_state_key(aggregator_v1_state_key_for_test(700), 200);
        state_view.set_from_aggregator_id(DelayedFieldID::new_with_width(900, 8), 300);
        state_view.set_from_aggregator_id(DelayedFieldID::new_with_width(1000, 8), 400);
        state_view
    }

    // All aggregators are initialized deterministically based on their ID,
    // with V1 key, with the following spec.
    //
    //     +-------+---------------+-----------+-----+---------+
    //     |  key  | storage value |  create   | get | remove  |
    //     +-------+---------------+-----------+-----+---------+
    //     |  100  |               |   yes     | yes |   yes   |
    //     |  200  |               |   yes     | yes |         |
    //     |  300  |               |   yes     |     |   yes   |
    //     |  400  |               |   yes     |     |         |
    //     |  500  |      150      |           | yes |   yes   |
    //     |  600  |      100      |           | yes |         |
    //     |  700  |      200      |           | yes |         |
    //     |  800  |               |           |     |   yes   |
    //     +-------+---------------+-----------+-----+---------+
    fn test_set_up_v1(context: &NativeAggregatorContext) {
        let mut aggregator_data = context.aggregator_v1_data.borrow_mut();

        assert_ok!(aggregator_data.create_new_aggregator(aggregator_v1_id_for_test(100), 100));
        assert_ok!(aggregator_data.create_new_aggregator(aggregator_v1_id_for_test(200), 200));
        assert_ok!(aggregator_data.create_new_aggregator(aggregator_v1_id_for_test(300), 300));
        assert_ok!(aggregator_data.create_new_aggregator(aggregator_v1_id_for_test(400), 400));

        assert_ok!(aggregator_data.get_aggregator(aggregator_v1_id_for_test(100), 100));
        assert_ok!(aggregator_data.get_aggregator(aggregator_v1_id_for_test(200), 200));
        assert_ok!(aggregator_data
            .get_aggregator(aggregator_v1_id_for_test(500), 500)
            .unwrap()
            .add(150));
        assert_ok!(aggregator_data
            .get_aggregator(aggregator_v1_id_for_test(600), 600)
            .unwrap()
            .add(100));
        assert_ok!(aggregator_data
            .get_aggregator(aggregator_v1_id_for_test(700), 700)
            .unwrap()
            .add(200));

        assert_ok!(aggregator_data.remove_aggregator(aggregator_v1_id_for_test(100)));
        assert_ok!(aggregator_data.remove_aggregator(aggregator_v1_id_for_test(300)));
        assert_ok!(aggregator_data.remove_aggregator(aggregator_v1_id_for_test(500)));
        assert_ok!(aggregator_data.remove_aggregator(aggregator_v1_id_for_test(800)));
        assert_err!(aggregator_data.remove_aggregator(aggregator_v1_id_for_test(800)));
    }

    #[test]
    fn test_v1_into_change_set() {
        let resolver = get_test_resolver_v1();
        let context = NativeAggregatorContext::new([0; 32], &resolver, true, &resolver);
        test_set_up_v1(&context);

        let AggregatorChangeSet {
            aggregator_v1_changes,
            ..
        } = context.into_change_set().unwrap();

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

    fn get_test_resolver_v2() -> FakeAggregatorView {
        let mut state_view = FakeAggregatorView::default();
        state_view.set_from_aggregator_id(DelayedFieldID::new_with_width(900, 8), 300);
        state_view.set_from_aggregator_id(DelayedFieldID::new_with_width(1000, 8), 400);
        state_view
    }

    fn id_from_fake_idx(idx: u32, width: u32) -> DelayedFieldID {
        DelayedFieldID::new_with_width(FAKE_AGGREGATOR_VIEW_GEN_ID_START + idx, width)
    }

    // All aggregators are initialized deterministically based on their ID,
    // with v2 id, with the following spec.
    //
    //   agg(900) : storage(300)  -> try_add(200)  -> failed try_sub(501)  -> try_add(300)    -> try_add(100)  -> failed try_add(51)
    //                                   |                                       |
    //                               snapshot(0)                              snapshot(1)
    //                                   |
    //                               string_concat(4)
    //   agg(1000): storage(400)
    //
    //   agg(2000):  create()    -> try_add (500) -> failed try_add(1700) -> failed try_sub(501)
    //                                 |
    //                              snapshot(2)
    //                                 |
    //                              string_concat(3)
    fn test_set_up_v2(context: &NativeAggregatorContext) {
        let mut delayed_field_data = context.delayed_field_data.borrow_mut();

        assert_ok_eq!(
            delayed_field_data.try_add_delta(
                DelayedFieldID::new_with_width(900, 8),
                900,
                SignedU128::Positive(200),
                context.delayed_field_resolver
            ),
            true
        );

        // failed because of wrong max_value
        assert!(delayed_field_data
            .snapshot(
                DelayedFieldID::new_with_width(900, 8),
                800,
                8,
                context.delayed_field_resolver,
            )
            .is_err());

        assert_ok_eq!(
            delayed_field_data.snapshot(
                DelayedFieldID::new_with_width(900, 8),
                900,
                8,
                context.delayed_field_resolver
            ),
            id_from_fake_idx(0, 8)
        );

        assert_ok_eq!(
            delayed_field_data.try_add_delta(
                DelayedFieldID::new_with_width(900, 8),
                900,
                SignedU128::Negative(501),
                context.delayed_field_resolver
            ),
            false
        );

        assert_ok_eq!(
            delayed_field_data.try_add_delta(
                DelayedFieldID::new_with_width(900, 8),
                900,
                SignedU128::Positive(300),
                context.delayed_field_resolver
            ),
            true
        );

        assert_ok_eq!(
            delayed_field_data.snapshot(
                DelayedFieldID::new_with_width(900, 8),
                900,
                8,
                context.delayed_field_resolver
            ),
            id_from_fake_idx(1, 8)
        );

        assert_ok_eq!(
            delayed_field_data.try_add_delta(
                DelayedFieldID::new_with_width(900, 8),
                900,
                SignedU128::Positive(100),
                context.delayed_field_resolver
            ),
            true
        );

        assert_ok_eq!(
            delayed_field_data.try_add_delta(
                DelayedFieldID::new_with_width(900, 8),
                900,
                SignedU128::Positive(51),
                context.delayed_field_resolver
            ),
            false
        );

        delayed_field_data.create_new_aggregator(DelayedFieldID::new_with_width(2000, 8));
        assert_ok_eq!(
            delayed_field_data.try_add_delta(
                DelayedFieldID::new_with_width(2000, 8),
                2000,
                SignedU128::Positive(500),
                context.delayed_field_resolver
            ),
            true
        );

        assert_ok_eq!(
            delayed_field_data.snapshot(
                DelayedFieldID::new_with_width(2000, 8),
                2000,
                8,
                context.delayed_field_resolver
            ),
            id_from_fake_idx(2, 8)
        );

        let derived_width = assert_ok!(calculate_width_for_integer_embedded_string(
            "prefixsuffix".len(),
            id_from_fake_idx(0, 8)
        )) as u32;

        assert_ok_eq!(
            delayed_field_data.derive_string_concat(
                id_from_fake_idx(2, 8),
                "prefix".as_bytes().to_vec(),
                "suffix".as_bytes().to_vec(),
                context.delayed_field_resolver,
            ),
            id_from_fake_idx(3, derived_width),
        );

        assert_ok_eq!(
            delayed_field_data.derive_string_concat(
                id_from_fake_idx(0, 8),
                "prefix".as_bytes().to_vec(),
                "suffix".as_bytes().to_vec(),
                context.delayed_field_resolver,
            ),
            id_from_fake_idx(4, derived_width),
        );

        assert_ok_eq!(
            delayed_field_data.try_add_delta(
                DelayedFieldID::new_with_width(2000, 8),
                2000,
                SignedU128::Positive(1700),
                context.delayed_field_resolver
            ),
            false
        );
        assert_ok_eq!(
            delayed_field_data.try_add_delta(
                DelayedFieldID::new_with_width(2000, 8),
                2000,
                SignedU128::Negative(501),
                context.delayed_field_resolver
            ),
            false
        );
    }

    #[test]
    fn test_v2_into_change_set() {
        let resolver = get_test_resolver_v2();
        let context = NativeAggregatorContext::new([0; 32], &resolver, true, &resolver);
        test_set_up_v2(&context);
        let delayed_field_changes = context.into_delayed_fields();
        assert!(!delayed_field_changes.contains_key(&DelayedFieldID::new_with_width(1000, 8)));
        assert_some_eq!(
            delayed_field_changes.get(&DelayedFieldID::new_with_width(900, 8)),
            &DelayedChange::Apply(DelayedApplyChange::AggregatorDelta {
                delta: DeltaWithMax::new(SignedU128::Positive(600), 900)
            }),
        );
        // Snapshots have full history (not just until their point),
        // So their validation validates full transaction, and it is not
        // needed to check aggregators too (i.e. when we do read_snapshot)
        assert_some_eq!(
            delayed_field_changes.get(&id_from_fake_idx(0, 8)),
            &DelayedChange::Apply(DelayedApplyChange::SnapshotDelta {
                base_aggregator: DelayedFieldID::new_with_width(900, 8),
                delta: DeltaWithMax::new(SignedU128::Positive(200), 900)
            }),
        );
        assert_some_eq!(
            delayed_field_changes.get(&id_from_fake_idx(1, 8)),
            &DelayedChange::Apply(DelayedApplyChange::SnapshotDelta {
                base_aggregator: DelayedFieldID::new_with_width(900, 8),
                delta: DeltaWithMax::new(SignedU128::Positive(500), 900)
            }),
        );

        assert_some_eq!(
            delayed_field_changes.get(&DelayedFieldID::new_with_width(2000, 8)),
            &DelayedChange::Create(DelayedFieldValue::Aggregator(500)),
        );

        assert_some_eq!(
            delayed_field_changes.get(&id_from_fake_idx(2, 8)),
            &DelayedChange::Create(DelayedFieldValue::Snapshot(500)),
        );

        let derived_width = assert_ok!(calculate_width_for_integer_embedded_string(
            "prefixsuffix".len(),
            id_from_fake_idx(0, 8)
        )) as u32;

        assert_some_eq!(
            delayed_field_changes.get(&id_from_fake_idx(3, derived_width)),
            &DelayedChange::Create(DelayedFieldValue::Derived(
                "prefix500suffix".as_bytes().to_vec()
            )),
        );
        assert_some_eq!(
            delayed_field_changes.get(&id_from_fake_idx(4, derived_width)),
            &DelayedChange::Apply(DelayedApplyChange::SnapshotDerived {
                base_snapshot: id_from_fake_idx(0, 8),
                formula: SnapshotToStringFormula::Concat {
                    prefix: "prefix".as_bytes().to_vec(),
                    suffix: "suffix".as_bytes().to_vec(),
                },
            }),
        );
    }
}
