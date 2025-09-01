// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    bounded_math::{BoundedMath, SignedU128},
    delayed_change::{ApplyBase, DelayedApplyChange, DelayedChange},
    delta_change_set::DeltaWithMax,
    resolver::DelayedFieldResolver,
    types::{DelayedFieldValue, DelayedFieldsSpeculativeError, ReadPosition},
};
use aptos_types::{
    delayed_fields::{
        calculate_width_for_constant_string, calculate_width_for_integer_embedded_string,
        SnapshotToStringFormula,
    },
    error::{code_invariant_error, expect_ok, PanicOr},
};
use move_binary_format::errors::PartialVMResult;
use move_vm_types::{
    delayed_values::delayed_field_id::DelayedFieldID,
    global_values::{VersionController, VersionedCell},
};
use std::collections::{btree_map::Entry, BTreeMap, HashSet};

fn get_delayed_field_value_from_storage(
    id: &DelayedFieldID,
    resolver: &dyn DelayedFieldResolver,
) -> Result<DelayedFieldValue, PanicOr<DelayedFieldsSpeculativeError>> {
    resolver.get_delayed_field_value(id)
}

/// Stores all information about aggregators (how many have been created or
/// removed), what are their states, etc. per single transaction).
#[derive(Default)]
pub struct DelayedFieldData {
    /// Stores the current version of data and controls saves/undos.
    version_controller: VersionController,
    /// All delayed field changes.
    ///
    /// Note: because delayed fields are not materialized, we use () for materialized value in the
    /// slot.
    delayed_fields: BTreeMap<DelayedFieldID, VersionedCell<DelayedChange<DelayedFieldID>>>,
}

impl DelayedFieldData {
    /// Records an undo request for the data. All undoing will be done lazily when aggregators are
    /// actually accessed.
    pub fn undo(&mut self) {
        self.version_controller.undo();
    }

    /// Records a save request for the data. All saving will be done lazily when aggregators are
    /// actually accessed.
    pub fn save(&mut self) {
        self.version_controller.save();
    }

    pub fn try_add_delta(
        &mut self,
        id: DelayedFieldID,
        max_value: u128,
        input: SignedU128,
        resolver: &dyn DelayedFieldResolver,
    ) -> PartialVMResult<bool> {
        self.try_add_or_check_delta(id, max_value, input, resolver, true)
    }

    pub fn try_add_or_check_delta(
        &mut self,
        id: DelayedFieldID,
        max_value: u128,
        input: SignedU128,
        resolver: &dyn DelayedFieldResolver,
        apply_delta: bool,
    ) -> PartialVMResult<bool> {
        // No need to record or check or try, if input value exceeds the bound.
        if input.abs() > max_value {
            return Ok(false);
        }

        let current_version = self.version_controller.current_version();
        let initialize_change = || -> PartialVMResult<_> {
            let result = resolver.delayed_field_try_add_delta_outcome(
                &id,
                &SignedU128::Positive(0),
                &input,
                max_value,
            )?;
            let slot = (result && apply_delta).then(|| {
                VersionedCell::new(
                    DelayedChange::Apply(DelayedApplyChange::AggregatorDelta {
                        delta: DeltaWithMax::new(input, max_value),
                    }),
                    current_version,
                )
            });
            Ok((result, slot))
        };

        match self.delayed_fields.entry(id) {
            Entry::Vacant(entry) => {
                let (result, slot) = initialize_change()?;
                if let Some(slot) = slot {
                    entry.insert(slot);
                }
                Ok(result)
            },
            Entry::Occupied(mut entry) => {
                let change = match entry.get_mut().latest_cow(current_version) {
                    Some(change) => change,
                    // This slot is actually empty - requires initialization.
                    None => {
                        let (result, slot) = initialize_change()?;
                        if let Some(slot) = slot {
                            entry.insert(slot);
                        }
                        return Ok(result);
                    },
                };

                let math = BoundedMath::new(max_value);
                match change {
                    DelayedChange::Create(DelayedFieldValue::Aggregator(value)) => {
                        match math.unsigned_add_delta(*value, &input) {
                            Ok(new_value) => {
                                if apply_delta {
                                    *value = new_value;
                                }
                                Ok(true)
                            },
                            Err(_) => Ok(false),
                        }
                    },
                    DelayedChange::Apply(DelayedApplyChange::AggregatorDelta {
                        delta: previous_delta,
                    }) => {
                        let result = resolver.delayed_field_try_add_delta_outcome(
                            &id,
                            &previous_delta.update,
                            &input,
                            previous_delta.max_value,
                        )?;
                        if result && apply_delta {
                            *previous_delta = expect_ok(DeltaWithMax::create_merged_delta(
                                previous_delta,
                                &DeltaWithMax::new(input, max_value),
                            ))?;
                        }
                        Ok(result)
                    },
                    _ => Err(code_invariant_error(
                        "Tried to add delta to a non-aggregator delayed field",
                    )
                    .into()),
                }
            },
        }
    }

    /// Creates and a new Aggregator with a given `id` and a `max_value`. The value
    /// of a new aggregator is always known, therefore it is created in a data
    /// state, with a zero-initialized value.
    pub fn create_new_aggregator(&mut self, id: DelayedFieldID) {
        let aggregator = DelayedChange::Create(DelayedFieldValue::Aggregator(0));
        let current_version = self.version_controller.current_version();
        // TODO: Assert no eviction?
        self.delayed_fields
            .insert(id, VersionedCell::new(aggregator, current_version));
    }

    /// Implements logic for doing a read on DelayedField.
    fn read_value(
        &mut self,
        id: DelayedFieldID,
        resolver: &dyn DelayedFieldResolver,
        read_position: ReadPosition,
    ) -> Result<DelayedFieldValue, PanicOr<DelayedFieldsSpeculativeError>> {
        match self.delayed_fields.get_mut(&id) {
            Some(slot) => {
                let current_version = self.version_controller.current_version();
                match slot.latest(current_version) {
                    Some(change) => {
                        match change {
                            DelayedChange::Create(value) => {
                                match read_position {
                                    ReadPosition::BeforeCurrentTxn => {
                                        Err(code_invariant_error(
                                            "Asking for aggregator value BeforeCurrentTxn that was created in this transaction",
                                        ).into())
                                    },
                                    ReadPosition::AfterCurrentTxn => {
                                        // If aggregator knows the value, return it.
                                        Ok(value.clone())
                                    },
                                }
                            },
                            DelayedChange::Apply(apply) => {
                                // TODO: avoid clone, need to fix multiple mut borrows here.
                                let apply = apply.clone();
                                let value = match apply.get_apply_base_id_option() {
                                    Some(base_id) => match base_id {
                                        ApplyBase::Previous(base_id) => self.read_value(
                                            base_id,
                                            resolver,
                                            ReadPosition::BeforeCurrentTxn,
                                        ),
                                        ApplyBase::Current(base_id) => self.read_value(
                                            base_id,
                                            resolver,
                                            ReadPosition::AfterCurrentTxn,
                                        ),
                                    },
                                    None => get_delayed_field_value_from_storage(&id, resolver),
                                }?;
                                match read_position {
                                    ReadPosition::BeforeCurrentTxn => Ok(value),
                                    ReadPosition::AfterCurrentTxn => {
                                        Ok(expect_ok(apply.apply_to_base(value))?)
                                    },
                                }
                            },
                        }
                    },
                    None => get_delayed_field_value_from_storage(&id, resolver),
                }
            },
            None => get_delayed_field_value_from_storage(&id, resolver),
        }
    }

    pub fn read_aggregator(
        &mut self,
        id: DelayedFieldID,
        resolver: &dyn DelayedFieldResolver,
    ) -> PartialVMResult<u128> {
        Ok(self
            .read_value(id, resolver, ReadPosition::AfterCurrentTxn)?
            .into_aggregator_value()?)
    }

    pub fn snapshot(
        &mut self,
        aggregator_id: DelayedFieldID,
        max_value: u128,
        width: u32,
        resolver: &dyn DelayedFieldResolver,
    ) -> PartialVMResult<DelayedFieldID> {
        let aggregator = self.delayed_fields.get_mut(&aggregator_id);

        let change = match aggregator {
            Some(slot) => {
                let current_version = self.version_controller.current_version();
                match slot.latest(current_version) {
                    Some(change) => {
                        match change {
                            // If aggregator is in Create state, we don't need to depend on it, and can just take the value.
                            DelayedChange::Create(DelayedFieldValue::Aggregator(value)) => {
                                DelayedChange::Create(DelayedFieldValue::Snapshot(*value))
                            },
                            DelayedChange::Apply(DelayedApplyChange::AggregatorDelta {
                                delta,
                                ..
                            }) => {
                                if max_value != delta.max_value {
                                    return Err(code_invariant_error(
                                        "Tried to snapshot an aggregator with a different max value",
                                    )
                                        .into());
                                }
                                DelayedChange::Apply(DelayedApplyChange::SnapshotDelta {
                                    base_aggregator: aggregator_id,
                                    delta: *delta,
                                })
                            },
                            _ => {
                                return Err(code_invariant_error(
                                    "Tried to snapshot a non-aggregator delayed field",
                                )
                                .into())
                            },
                        }
                    },
                    None => DelayedChange::Apply(DelayedApplyChange::SnapshotDelta {
                        base_aggregator: aggregator_id,
                        delta: DeltaWithMax {
                            update: SignedU128::Positive(0),
                            max_value,
                        },
                    }),
                }
            },
            None => DelayedChange::Apply(DelayedApplyChange::SnapshotDelta {
                base_aggregator: aggregator_id,
                delta: DeltaWithMax {
                    update: SignedU128::Positive(0),
                    max_value,
                },
            }),
        };

        let snapshot_id = resolver.generate_delayed_field_id(width);
        let current_version = self.version_controller.current_version();
        self.delayed_fields
            .insert(snapshot_id, VersionedCell::new(change, current_version));
        Ok(snapshot_id)
    }

    pub fn create_new_snapshot(
        &mut self,
        value: u128,
        width: u32,
        resolver: &dyn DelayedFieldResolver,
    ) -> DelayedFieldID {
        let change = DelayedChange::Create(DelayedFieldValue::Snapshot(value));
        let snapshot_id = resolver.generate_delayed_field_id(width);

        let current_version = self.version_controller.current_version();
        self.delayed_fields
            .insert(snapshot_id, VersionedCell::new(change, current_version));
        snapshot_id
    }

    pub fn create_new_derived(
        &mut self,
        value: Vec<u8>,
        resolver: &dyn DelayedFieldResolver,
    ) -> PartialVMResult<DelayedFieldID> {
        // cast shouldn't fail because we assert on low limit for value before this call.
        let width =
            u32::try_from(calculate_width_for_constant_string(value.len())).map_err(|_| {
                code_invariant_error("Calculated DerivedStringSnapshot width exceeds u32")
            })?;
        let change = DelayedChange::Create(DelayedFieldValue::Derived(value));
        let snapshot_id = resolver.generate_delayed_field_id(width);

        let current_version = self.version_controller.current_version();
        self.delayed_fields
            .insert(snapshot_id, VersionedCell::new(change, current_version));
        Ok(snapshot_id)
    }

    pub fn read_snapshot(
        &mut self,
        snapshot_id: DelayedFieldID,
        resolver: &dyn DelayedFieldResolver,
    ) -> PartialVMResult<u128> {
        Ok(self
            .read_value(snapshot_id, resolver, ReadPosition::AfterCurrentTxn)?
            .into_snapshot_value()?)
    }

    pub fn read_derived(
        &mut self,
        snapshot_id: DelayedFieldID,
        resolver: &dyn DelayedFieldResolver,
    ) -> PartialVMResult<Vec<u8>> {
        Ok(self
            .read_value(snapshot_id, resolver, ReadPosition::AfterCurrentTxn)?
            .into_derived_value()?)
    }

    pub fn derive_string_concat(
        &mut self,
        snapshot_id: DelayedFieldID,
        prefix: Vec<u8>,
        suffix: Vec<u8>,
        resolver: &dyn DelayedFieldResolver,
    ) -> PartialVMResult<DelayedFieldID> {
        // cast shouldn't fail because we assert on low limit for prefix and suffix before this call.
        let width = u32::try_from(calculate_width_for_integer_embedded_string(
            prefix.len() + suffix.len(),
            snapshot_id,
        )?)
        .map_err(|_| code_invariant_error("Calculated DerivedStringSnapshot width exceeds u32"))?;
        let formula = SnapshotToStringFormula::Concat { prefix, suffix };

        let change = match self.delayed_fields.get_mut(&snapshot_id) {
            Some(slot) => {
                let current_version = self.version_controller.current_version();
                match slot.latest(current_version) {
                    Some(change) => {
                        match change {
                            // If snapshot is in Create state, we don't need to depend on it, and can just take the value.
                            DelayedChange::Create(DelayedFieldValue::Snapshot(value)) => {
                                DelayedChange::Create(DelayedFieldValue::Derived(
                                    formula.apply_to(*value),
                                ))
                            },
                            DelayedChange::Apply(DelayedApplyChange::SnapshotDelta { .. }) => {
                                DelayedChange::Apply(DelayedApplyChange::SnapshotDerived {
                                    base_snapshot: snapshot_id,
                                    formula,
                                })
                            },
                            _ => {
                                return Err(code_invariant_error(
                                    "Tried to string_concat a non-snapshot delayed field",
                                )
                                .into())
                            },
                        }
                    },
                    None => DelayedChange::Apply(DelayedApplyChange::SnapshotDerived {
                        base_snapshot: snapshot_id,
                        formula,
                    }),
                }
            },
            None => DelayedChange::Apply(DelayedApplyChange::SnapshotDerived {
                base_snapshot: snapshot_id,
                formula,
            }),
        };

        let new_id = resolver.generate_delayed_field_id(width);
        let current_version = self.version_controller.current_version();
        self.delayed_fields
            .insert(new_id, VersionedCell::new(change, current_version));
        Ok(new_id)
    }

    pub fn into(self) -> BTreeMap<DelayedFieldID, DelayedChange<DelayedFieldID>> {
        let current_version = self.version_controller.current_version();
        self.delayed_fields
            .into_iter()
            .filter_map(|(id, mut slot)| {
                slot.take_latest(current_version).map(|change| (id, change))
            })
            .collect()
    }

    pub fn materialize(&mut self) -> HashSet<DelayedFieldID> {
        let mut ids = HashSet::new();
        let current_version = self.version_controller.current_version();
        for (id, slot) in self.delayed_fields.iter_mut() {
            if slot.latest(current_version).is_some() {
                ids.insert(*id);
            }
        }
        ids
    }

    pub fn take_latest(&mut self) -> BTreeMap<DelayedFieldID, DelayedChange<DelayedFieldID>> {
        let mut changes = BTreeMap::new();
        let current_version = self.version_controller.current_version();
        for (id, slot) in self.delayed_fields.iter_mut() {
            if let Some(change) = slot.take_latest(current_version) {
                changes.insert(*id, change);
            }
        }
        changes
    }
}

// ================================= Tests =================================

#[cfg(test)]
mod test {
    use super::*;
    use crate::FakeAggregatorView;
    use claims::{assert_err, assert_ok_eq};

    #[test]
    fn test_aggregator_not_in_storage() {
        let resolver = FakeAggregatorView::default();
        let mut data = DelayedFieldData::default();
        let id = DelayedFieldID::new_for_test_for_u64(200);
        let max_value = 700;

        assert_err!(data.read_aggregator(id, &resolver));
        assert_err!(data.try_add_delta(id, max_value, SignedU128::Positive(100), &resolver));
        assert_err!(data.try_add_delta(id, max_value, SignedU128::Negative(1), &resolver));
    }

    fn get_agg<'a>(
        d: &'a mut DelayedFieldData,
        id: &DelayedFieldID,
    ) -> &'a DelayedChange<DelayedFieldID> {
        let current_version = d.version_controller.current_version();
        d.delayed_fields
            .get_mut(id)
            .expect("Get aggregator failed")
            .latest(current_version)
            .expect("Latest aggregator is not found")
    }

    #[test]
    fn test_operations_on_new_aggregator() {
        let resolver = FakeAggregatorView::default();
        let mut data = DelayedFieldData::default();
        let id = DelayedFieldID::new_for_test_for_u64(200);
        let max_value = 200;

        data.create_new_aggregator(id);

        assert_eq!(
            get_agg(&mut data, &id),
            &DelayedChange::<DelayedFieldID>::Create(DelayedFieldValue::Aggregator(0))
        );
        assert_ok_eq!(
            data.try_add_or_check_delta(id, max_value, SignedU128::Positive(100), &resolver, false),
            true
        );
        assert_eq!(
            get_agg(&mut data, &id),
            &DelayedChange::<DelayedFieldID>::Create(DelayedFieldValue::Aggregator(0))
        );

        assert_ok_eq!(
            data.try_add_delta(id, max_value, SignedU128::Positive(100), &resolver),
            true
        );
        assert_eq!(
            get_agg(&mut data, &id),
            &DelayedChange::<DelayedFieldID>::Create(DelayedFieldValue::Aggregator(100))
        );

        assert_ok_eq!(
            data.try_add_or_check_delta(id, max_value, SignedU128::Positive(120), &resolver, false),
            false
        );
        assert_eq!(
            get_agg(&mut data, &id),
            &DelayedChange::<DelayedFieldID>::Create(DelayedFieldValue::Aggregator(100))
        );

        assert_ok_eq!(
            data.try_add_delta(id, max_value, SignedU128::Negative(50), &resolver),
            true
        );
        assert_eq!(
            get_agg(&mut data, &id),
            &DelayedChange::<DelayedFieldID>::Create(DelayedFieldValue::Aggregator(50))
        );
        assert_ok_eq!(
            data.try_add_delta(id, max_value, SignedU128::Negative(70), &resolver),
            false
        );
        assert_eq!(
            get_agg(&mut data, &id),
            &DelayedChange::<DelayedFieldID>::Create(DelayedFieldValue::Aggregator(50))
        );
        assert_ok_eq!(
            data.try_add_delta(id, max_value, SignedU128::Positive(170), &resolver),
            false
        );
        assert_eq!(
            get_agg(&mut data, &id),
            &DelayedChange::Create(DelayedFieldValue::Aggregator(50))
        );
        assert_ok_eq!(data.read_aggregator(id, &resolver), 50);
    }

    fn aggregator_delta_change(delta: i128, max_value: u128) -> DelayedChange<DelayedFieldID> {
        DelayedChange::Apply(DelayedApplyChange::AggregatorDelta {
            delta: DeltaWithMax {
                update: if delta > 0 {
                    SignedU128::Positive(delta as u128)
                } else {
                    SignedU128::Negative((-delta) as u128)
                },
                max_value,
            },
        })
    }

    #[test]
    fn test_successful_operations_in_delta_mode() {
        let mut resolver = FakeAggregatorView::default();
        let mut data = DelayedFieldData::default();
        let id = DelayedFieldID::new_for_test_for_u64(200);
        let max_value = 600;

        resolver.set_from_aggregator_id(id, 100);

        assert_ok_eq!(
            data.try_add_or_check_delta(id, max_value, SignedU128::Positive(400), &resolver, false),
            true
        );
        // checks only add to captured reads, not to writes
        assert!(!data.delayed_fields.contains_key(&id));

        assert_ok_eq!(
            data.try_add_or_check_delta(id, max_value, SignedU128::Positive(550), &resolver, false),
            false
        );
        // checks only add to captured reads, not to writes
        assert!(!data.delayed_fields.contains_key(&id));

        assert_ok_eq!(
            data.try_add_delta(id, max_value, SignedU128::Positive(400), &resolver),
            true
        );
        assert_eq!(
            get_agg(&mut data, &id),
            &aggregator_delta_change(400, max_value)
        );

        assert_ok_eq!(
            data.try_add_or_check_delta(id, max_value, SignedU128::Negative(100), &resolver, false),
            true
        );
        assert_eq!(
            get_agg(&mut data, &id),
            &aggregator_delta_change(400, max_value)
        );

        assert_ok_eq!(
            data.try_add_delta(id, max_value, SignedU128::Negative(470), &resolver),
            true
        );
        assert_eq!(
            get_agg(&mut data, &id),
            &aggregator_delta_change(-70, max_value)
        );

        assert_ok_eq!(data.read_aggregator(id, &resolver), 30);
    }

    #[test]
    fn test_aggregator_overflows() {
        let mut resolver = FakeAggregatorView::default();
        let mut data = DelayedFieldData::default();
        let id = DelayedFieldID::new_for_test_for_u64(600);
        let max_value = 600;

        resolver.set_from_aggregator_id(id, 100);

        assert_ok_eq!(
            data.try_add_delta(id, max_value, SignedU128::Positive(400), &resolver),
            true
        );
        assert_ok_eq!(
            data.try_add_delta(id, max_value, SignedU128::Negative(450), &resolver),
            true
        );
        assert_ok_eq!(
            data.try_add_delta(id, max_value, SignedU128::Positive(601), &resolver),
            false
        );
        assert_ok_eq!(
            data.try_add_delta(id, max_value, SignedU128::Positive(575), &resolver),
            false
        );
        assert_ok_eq!(
            data.try_add_delta(id, max_value, SignedU128::Positive(551), &resolver),
            false
        );
        assert_ok_eq!(
            data.try_add_delta(id, max_value, SignedU128::Positive(570), &resolver),
            false
        );
    }

    #[test]
    fn test_aggregator_underflows() {
        let mut resolver = FakeAggregatorView::default();
        let mut data = DelayedFieldData::default();
        let id = DelayedFieldID::new_for_test_for_u64(600);
        let max_value = 600;

        resolver.set_from_aggregator_id(id, 200);

        assert_ok_eq!(
            data.try_add_delta(id, max_value, SignedU128::Positive(300), &resolver),
            true
        );
        assert_eq!(
            get_agg(&mut data, &id),
            &aggregator_delta_change(300, max_value)
        );

        assert_ok_eq!(
            data.try_add_delta(id, max_value, SignedU128::Negative(650), &resolver),
            false
        );
        assert_eq!(
            get_agg(&mut data, &id),
            &aggregator_delta_change(300, max_value)
        );

        assert_ok_eq!(
            data.try_add_delta(id, max_value, SignedU128::Negative(550), &resolver),
            false
        );
        assert_eq!(
            get_agg(&mut data, &id),
            &aggregator_delta_change(300, max_value)
        );

        assert_ok_eq!(
            data.try_add_delta(id, max_value, SignedU128::Negative(525), &resolver),
            false
        );
        assert_eq!(
            get_agg(&mut data, &id),
            &aggregator_delta_change(300, max_value)
        );

        assert_ok_eq!(
            data.try_add_delta(id, max_value, SignedU128::Negative(540), &resolver),
            false
        );
        assert_eq!(
            get_agg(&mut data, &id),
            &aggregator_delta_change(300, max_value)
        );

        assert_ok_eq!(
            data.try_add_delta(id, max_value, SignedU128::Negative(501), &resolver),
            false
        );
        assert_eq!(
            get_agg(&mut data, &id),
            &aggregator_delta_change(300, max_value)
        );
    }
}
