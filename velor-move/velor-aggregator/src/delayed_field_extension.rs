// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    bounded_math::{BoundedMath, SignedU128},
    delayed_change::{ApplyBase, DelayedApplyChange, DelayedChange},
    delta_change_set::DeltaWithMax,
    resolver::DelayedFieldResolver,
    types::{DelayedFieldValue, DelayedFieldsSpeculativeError, ReadPosition},
};
use velor_types::{
    delayed_fields::{
        calculate_width_for_constant_string, calculate_width_for_integer_embedded_string,
        SnapshotToStringFormula,
    },
    error::{code_invariant_error, expect_ok, PanicOr},
};
use move_binary_format::errors::PartialVMResult;
use move_vm_types::delayed_values::delayed_field_id::DelayedFieldID;
use std::collections::{btree_map::Entry, BTreeMap};

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
    // All aggregator instances that exist in the current transaction.
    delayed_fields: BTreeMap<DelayedFieldID, DelayedChange<DelayedFieldID>>,
}

impl DelayedFieldData {
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

        match self.delayed_fields.entry(id) {
            Entry::Vacant(entry) => {
                let result = resolver.delayed_field_try_add_delta_outcome(
                    &id,
                    &SignedU128::Positive(0),
                    &input,
                    max_value,
                )?;
                if result && apply_delta {
                    entry.insert(DelayedChange::Apply(DelayedApplyChange::AggregatorDelta {
                        delta: DeltaWithMax::new(input, max_value),
                    }));
                }
                Ok(result)
            },
            Entry::Occupied(mut entry) => {
                let math = BoundedMath::new(max_value);
                match entry.get_mut() {
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
        self.delayed_fields.insert(id, aggregator);
    }

    /// Implements logic for doing a read on DelayedField.
    fn read_value(
        &self,
        id: DelayedFieldID,
        resolver: &dyn DelayedFieldResolver,
        read_position: ReadPosition,
    ) -> Result<DelayedFieldValue, PanicOr<DelayedFieldsSpeculativeError>> {
        match self.delayed_fields.get(&id) {
            Some(DelayedChange::Create(value)) => {
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
            Some(DelayedChange::Apply(apply)) => {
                let value = apply.get_apply_base_id_option().map_or_else(
                    || get_delayed_field_value_from_storage(&id, resolver),
                    |base_id| match base_id {
                        ApplyBase::Previous(base_id) => {
                            self.read_value(base_id, resolver, ReadPosition::BeforeCurrentTxn)
                        },
                        ApplyBase::Current(base_id) => {
                            self.read_value(base_id, resolver, ReadPosition::AfterCurrentTxn)
                        },
                    },
                )?;
                match read_position {
                    ReadPosition::BeforeCurrentTxn => Ok(value),
                    ReadPosition::AfterCurrentTxn => Ok(expect_ok(apply.apply_to_base(value))?),
                }
            },
            None => get_delayed_field_value_from_storage(&id, resolver),
        }
    }

    pub fn read_aggregator(
        &self,
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
        let aggregator = self.delayed_fields.get(&aggregator_id);

        let change = match aggregator {
            // If aggregator is in Create state, we don't need to depend on it, and can just take the value.
            Some(DelayedChange::Create(DelayedFieldValue::Aggregator(value))) => {
                DelayedChange::Create(DelayedFieldValue::Snapshot(*value))
            },
            Some(DelayedChange::Apply(DelayedApplyChange::AggregatorDelta { delta, .. })) => {
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
            None => DelayedChange::Apply(DelayedApplyChange::SnapshotDelta {
                base_aggregator: aggregator_id,
                delta: DeltaWithMax {
                    update: SignedU128::Positive(0),
                    max_value,
                },
            }),
            _ => {
                return Err(code_invariant_error(
                    "Tried to snapshot a non-aggregator delayed field",
                )
                .into())
            },
        };

        let snapshot_id = resolver.generate_delayed_field_id(width);
        self.delayed_fields.insert(snapshot_id, change);
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

        self.delayed_fields.insert(snapshot_id, change);
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

        self.delayed_fields.insert(snapshot_id, change);
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
        let snapshot = self.delayed_fields.get(&snapshot_id);
        // cast shouldn't fail because we assert on low limit for prefix and suffix before this call.
        let width = u32::try_from(calculate_width_for_integer_embedded_string(
            prefix.len() + suffix.len(),
            snapshot_id,
        )?)
        .map_err(|_| code_invariant_error("Calculated DerivedStringSnapshot width exceeds u32"))?;
        let formula = SnapshotToStringFormula::Concat { prefix, suffix };

        let change = match snapshot {
            // If snapshot is in Create state, we don't need to depend on it, and can just take the value.
            Some(DelayedChange::Create(DelayedFieldValue::Snapshot(value))) => {
                DelayedChange::Create(DelayedFieldValue::Derived(formula.apply_to(*value)))
            },
            Some(DelayedChange::Apply(DelayedApplyChange::SnapshotDelta { .. })) | None => {
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
        };

        let new_id = resolver.generate_delayed_field_id(width);
        self.delayed_fields.insert(new_id, change);
        Ok(new_id)
    }

    /// Unpacks aggregator data.
    pub fn into(self) -> BTreeMap<DelayedFieldID, DelayedChange<DelayedFieldID>> {
        self.delayed_fields
    }
}

// ================================= Tests =================================

#[cfg(test)]
mod test {
    use super::*;
    use crate::FakeAggregatorView;
    use claims::{assert_err, assert_none, assert_ok_eq};

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
        d: &'a DelayedFieldData,
        id: &DelayedFieldID,
    ) -> &'a DelayedChange<DelayedFieldID> {
        d.delayed_fields.get(id).expect("Get aggregator failed")
    }

    #[test]
    fn test_operations_on_new_aggregator() {
        let resolver = FakeAggregatorView::default();
        let mut data = DelayedFieldData::default();
        let id = DelayedFieldID::new_for_test_for_u64(200);
        let max_value = 200;

        data.create_new_aggregator(id);

        assert_eq!(
            get_agg(&data, &id),
            &DelayedChange::<DelayedFieldID>::Create(DelayedFieldValue::Aggregator(0))
        );
        assert_ok_eq!(
            data.try_add_or_check_delta(id, max_value, SignedU128::Positive(100), &resolver, false),
            true
        );
        assert_eq!(
            get_agg(&data, &id),
            &DelayedChange::<DelayedFieldID>::Create(DelayedFieldValue::Aggregator(0))
        );

        assert_ok_eq!(
            data.try_add_delta(id, max_value, SignedU128::Positive(100), &resolver),
            true
        );
        assert_eq!(
            get_agg(&data, &id),
            &DelayedChange::<DelayedFieldID>::Create(DelayedFieldValue::Aggregator(100))
        );

        assert_ok_eq!(
            data.try_add_or_check_delta(id, max_value, SignedU128::Positive(120), &resolver, false),
            false
        );
        assert_eq!(
            get_agg(&data, &id),
            &DelayedChange::<DelayedFieldID>::Create(DelayedFieldValue::Aggregator(100))
        );

        assert_ok_eq!(
            data.try_add_delta(id, max_value, SignedU128::Negative(50), &resolver),
            true
        );
        assert_eq!(
            get_agg(&data, &id),
            &DelayedChange::<DelayedFieldID>::Create(DelayedFieldValue::Aggregator(50))
        );
        assert_ok_eq!(
            data.try_add_delta(id, max_value, SignedU128::Negative(70), &resolver),
            false
        );
        assert_eq!(
            get_agg(&data, &id),
            &DelayedChange::<DelayedFieldID>::Create(DelayedFieldValue::Aggregator(50))
        );
        assert_ok_eq!(
            data.try_add_delta(id, max_value, SignedU128::Positive(170), &resolver),
            false
        );
        assert_eq!(
            get_agg(&data, &id),
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
        assert_none!(data.delayed_fields.get(&id));

        assert_ok_eq!(
            data.try_add_or_check_delta(id, max_value, SignedU128::Positive(550), &resolver, false),
            false
        );
        // checks only add to captured reads, not to writes
        assert_none!(data.delayed_fields.get(&id));

        assert_ok_eq!(
            data.try_add_delta(id, max_value, SignedU128::Positive(400), &resolver),
            true
        );
        assert_eq!(
            get_agg(&data, &id),
            &aggregator_delta_change(400, max_value)
        );

        assert_ok_eq!(
            data.try_add_or_check_delta(id, max_value, SignedU128::Negative(100), &resolver, false),
            true
        );
        assert_eq!(
            get_agg(&data, &id),
            &aggregator_delta_change(400, max_value)
        );

        assert_ok_eq!(
            data.try_add_delta(id, max_value, SignedU128::Negative(470), &resolver),
            true
        );
        assert_eq!(
            get_agg(&data, &id),
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
            get_agg(&data, &id),
            &aggregator_delta_change(300, max_value)
        );

        assert_ok_eq!(
            data.try_add_delta(id, max_value, SignedU128::Negative(650), &resolver),
            false
        );
        assert_eq!(
            get_agg(&data, &id),
            &aggregator_delta_change(300, max_value)
        );

        assert_ok_eq!(
            data.try_add_delta(id, max_value, SignedU128::Negative(550), &resolver),
            false
        );
        assert_eq!(
            get_agg(&data, &id),
            &aggregator_delta_change(300, max_value)
        );

        assert_ok_eq!(
            data.try_add_delta(id, max_value, SignedU128::Negative(525), &resolver),
            false
        );
        assert_eq!(
            get_agg(&data, &id),
            &aggregator_delta_change(300, max_value)
        );

        assert_ok_eq!(
            data.try_add_delta(id, max_value, SignedU128::Negative(540), &resolver),
            false
        );
        assert_eq!(
            get_agg(&data, &id),
            &aggregator_delta_change(300, max_value)
        );

        assert_ok_eq!(
            data.try_add_delta(id, max_value, SignedU128::Negative(501), &resolver),
            false
        );
        assert_eq!(
            get_agg(&data, &id),
            &aggregator_delta_change(300, max_value)
        );
    }
}
