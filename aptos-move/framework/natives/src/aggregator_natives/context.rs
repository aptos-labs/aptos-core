// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_aggregator::{
    aggregator_v1_extension::{extension_error, AggregatorData, AggregatorID},
    delayed_change::DelayedChange,
    delayed_field_extension::DelayedFieldData,
    resolver::{AggregatorV1Resolver, DelayedFieldResolver},
};
use aptos_types::state_store::{state_key::StateKey, state_value::StateValueMetadata};
use better_any::{Tid, TidAble};
use move_binary_format::errors::PartialVMResult;
use move_core_types::value::MoveTypeLayout;
use move_vm_runtime::native_extensions::{NativeRuntimeRefCheckModelsCompleted, SessionListener};
use move_vm_types::delayed_values::delayed_field_id::DelayedFieldID;
use std::{
    cell::RefCell,
    collections::{BTreeMap, HashSet},
};
use triomphe::Arc as TriompheArc;

/// Represents a single aggregator change.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AggregatorChangeV1 {
    /// Used **ONLY** when delayed field optimization disabled.
    ///
    /// A concrete write of a known value (e.g., a new or read-materialized
    /// aggregator). Creation vs modification is decided at conversion from the
    /// existing storage slot.
    Write(u128),
    /// Used **ONLY** when delayed field optimization disabled.
    ///
    /// A delta resolved in place; the value is already materialized, and it
    /// will be written with legacy (no) metadata. Never charged gas.
    MaterializedDelta(u128),
    /// Used **ONLY** when delayed field optimization enabled.
    ///
    /// A write carrying the delayed field ID. The value behind the ID is stored
    /// separately and flows through the delayed field extension.
    WriteWithDelayedFields(DelayedFieldID),
    /// Used **ONLY** when delayed field optimization enabled.
    ///
    /// A delta to be resolved later; materialized by block executor.
    DelayedDelta,
    /// A deletion of the aggregator from storage. Independent of the delayed
    /// field optimization: a deletion carries no delayed field to exchange, so
    /// it is always a plain concrete write, just like a normal resource
    /// deletion.
    Delete,
}

/// Represents changes made by all aggregators during this context. This change
/// set can be converted into appropriate `WriteSet` and `DeltaChangeSet` by the
/// user, e.g. VM session.
pub struct AggregatorChangeSet {
    pub aggregator_v1_changes: BTreeMap<StateKey, AggregatorChangeV1>,
    pub delayed_field_changes: BTreeMap<DelayedFieldID, DelayedChange<DelayedFieldID>>,
    pub reads_needing_exchange:
        BTreeMap<StateKey, (StateValueMetadata, u64, TriompheArc<MoveTypeLayout>)>,
    pub group_reads_needing_exchange: BTreeMap<StateKey, (StateValueMetadata, u64)>,
}

/// Result of an aggregator read: either a delayed value (not yet known), or
/// a concrete u128 integer.
pub(crate) enum AggregatorValue {
    Delayed(DelayedFieldID),
    Concrete(u128),
}

/// Native context that can be attached to VM `NativeContextExtensions`.
///
/// Note: table resolver is reused for fine-grained storage access.
#[derive(Tid)]
pub struct NativeAggregatorContext<'a> {
    session_hash: [u8; 32],
    pub(crate) aggregator_v1_resolver: &'a dyn AggregatorV1Resolver,
    pub(crate) aggregator_v1_data: RefCell<AggregatorData>,
    pub(crate) delayed_field_optimization_enabled: bool,
    pub(crate) delayed_field_resolver: &'a dyn DelayedFieldResolver,
    pub(crate) delayed_field_data: RefCell<DelayedFieldData>,
}

impl<'a> SessionListener for NativeAggregatorContext<'a> {
    fn start(&mut self, session_hash: &[u8; 32], _script_hash: &[u8], _session_counter: u8) {
        self.session_hash = *session_hash;
        // TODO(sessions): implement
    }

    fn finish(&mut self) {
        // TODO(sessions): implement
    }

    fn abort(&mut self) {
        // TODO(sessions): implement
    }
}

impl<'a> NativeRuntimeRefCheckModelsCompleted for NativeAggregatorContext<'a> {
    // No native functions in this context return references, so no models to add.
}

impl<'a> NativeAggregatorContext<'a> {
    /// Creates a new instance of a native aggregator context. This must be
    /// passed into VM session.
    pub fn new(
        session_hash: [u8; 32],
        aggregator_v1_resolver: &'a dyn AggregatorV1Resolver,
        delayed_field_optimization_enabled: bool,
        delayed_field_resolver: &'a dyn DelayedFieldResolver,
    ) -> Self {
        Self {
            session_hash,
            aggregator_v1_resolver,
            aggregator_v1_data: Default::default(),
            delayed_field_resolver,
            delayed_field_optimization_enabled,
            delayed_field_data: Default::default(),
        }
    }

    /// Returns the hash of session associated with this context.
    pub fn session_hash(&self) -> [u8; 32] {
        self.session_hash
    }

    pub(crate) fn resolve_aggregator_value(
        &self,
        id: &AggregatorID,
    ) -> PartialVMResult<AggregatorValue> {
        Ok(if self.delayed_field_optimization_enabled {
            let existing = self.aggregator_v1_data.borrow().get_id(id);
            let delayed_field_id = match existing {
                Some(delayed_field_id) => delayed_field_id,
                None => {
                    let delayed_field_id = self
                        .aggregator_v1_resolver
                        .get_aggregator_v1_delayed_field_id(id.as_state_key())?
                        .ok_or_else(|| {
                            extension_error(format!(
                                "Aggregator V1 value not found in storage at {:?}",
                                id
                            ))
                        })?;
                    self.aggregator_v1_data
                        .borrow_mut()
                        .set_id(id.clone(), delayed_field_id);
                    delayed_field_id
                },
            };
            AggregatorValue::Delayed(delayed_field_id)
        } else {
            let existing = self.aggregator_v1_data.borrow().get_value(id);
            let value = match existing {
                Some(value) => value,
                None => self
                    .aggregator_v1_resolver
                    .get_aggregator_v1_value(id.as_state_key())?
                    .ok_or_else(|| {
                        extension_error(format!(
                            "Aggregator V1 value not found in storage at {:?}",
                            id
                        ))
                    })?,
            };
            AggregatorValue::Concrete(value)
        })
    }

    /// Returns all changes made within this context (i.e. by a single
    /// transaction).
    pub fn into_change_set(self) -> PartialVMResult<AggregatorChangeSet> {
        let NativeAggregatorContext {
            aggregator_v1_data,
            delayed_field_data,
            ..
        } = self;
        let (new_aggregators, destroyed_aggregators, read_aggregators, values, ids) =
            aggregator_v1_data.into_inner().into();

        let mut aggregator_v1_changes = BTreeMap::new();

        // An aggregator whose value is known when created in this transaction
        // or read in this transaction. In this case, it is treated as a write.
        // Non-writes are written with legacy (no) metadata and never charged,
        // so it is important to differentiate here.
        let is_write = |id: &AggregatorID| -> bool {
            new_aggregators.contains(id) || read_aggregators.contains(id)
        };

        // Optimization disabled: the value is a concrete u128 tracked in place.
        for (id, value) in values {
            let change = if is_write(&id) {
                AggregatorChangeV1::Write(value)
            } else {
                AggregatorChangeV1::MaterializedDelta(value)
            };
            aggregator_v1_changes.insert(id.0, change);
        }

        // Optimization enabled: the value is represented by a delayed field id, whose value flows
        // through delayed field extension.
        for (id, delayed_field_id) in ids {
            let change = if is_write(&id) {
                AggregatorChangeV1::WriteWithDelayedFields(delayed_field_id)
            } else {
                AggregatorChangeV1::DelayedDelta
            };
            aggregator_v1_changes.insert(id.0, change);
        }

        for id in destroyed_aggregators {
            aggregator_v1_changes.insert(id.0, AggregatorChangeV1::Delete);
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
        tests::types::FAKE_AGGREGATOR_VIEW_GEN_ID_START, types::DelayedFieldValue,
        FakeAggregatorView,
    };
    use aptos_types::delayed_fields::{
        calculate_width_for_integer_embedded_string, SnapshotToStringFormula,
    };
    use claims::{assert_matches, assert_ok, assert_ok_eq, assert_some_eq};

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

        // Created this transaction (value known, zero-initialized).
        for key in [100, 200, 300, 400] {
            aggregator_data.create_new_aggregator(aggregator_v1_id_for_test(key));
            aggregator_data.set_value(aggregator_v1_id_for_test(key), 0);
        }

        // Existing aggregators reached through add/sub: the running value is tracked in place
        // (600: 100 + 100, 700: 200 + 200).
        aggregator_data.set_value(aggregator_v1_id_for_test(600), 200);
        aggregator_data.set_value(aggregator_v1_id_for_test(700), 400);

        // 100 and 300 were created this transaction (elided); 500 and 800 existed before (deleted).
        aggregator_data.remove_aggregator(aggregator_v1_id_for_test(100));
        aggregator_data.remove_aggregator(aggregator_v1_id_for_test(300));
        aggregator_data.remove_aggregator(aggregator_v1_id_for_test(500));
        aggregator_data.remove_aggregator(aggregator_v1_id_for_test(800));
    }

    #[test]
    fn test_v1_into_change_set() {
        let resolver = get_test_resolver_v1();
        // The test drives `aggregator_v1_data` directly, i.e. the optimization-disabled path.
        let context = NativeAggregatorContext::new([0; 32], &resolver, false, &resolver);
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
        // Aggregators touched directly via `aggregator_v1_data` end the transaction in a delta
        // state, so `into_change_set` materializes them against the resolver (600: 100 + 100,
        // 700: 200 + 200).
        assert_matches!(
            aggregator_v1_changes
                .get(&aggregator_v1_state_key_for_test(600))
                .unwrap(),
            AggregatorChangeV1::MaterializedDelta(200)
        );
        assert_matches!(
            aggregator_v1_changes
                .get(&aggregator_v1_state_key_for_test(700))
                .unwrap(),
            AggregatorChangeV1::MaterializedDelta(400)
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
