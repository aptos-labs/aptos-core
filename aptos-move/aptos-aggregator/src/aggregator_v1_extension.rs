// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_types::{
    state_store::{state_key::StateKey, table::TableHandle},
    PeerId,
};
use move_binary_format::errors::PartialVMError;
use move_core_types::{
    value::{IdentifierMappingKind, MoveTypeLayout},
    vm_status::StatusCode,
};
use move_vm_types::delayed_values::delayed_field_id::DelayedFieldID;
use once_cell::sync::Lazy;
use std::collections::{BTreeMap, BTreeSet};
use triomphe::Arc;

#[derive(Debug, Clone, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct AggregatorID(pub StateKey);

impl AggregatorID {
    pub fn new(handle: TableHandle, key: PeerId) -> Self {
        let state_key = StateKey::table_item(&handle, key.as_ref());
        AggregatorID(state_key)
    }

    pub fn as_state_key(&self) -> &StateKey {
        &self.0
    }

    pub fn into_state_key(self) -> StateKey {
        self.0
    }
}

pub static AGGREGATOR_V1_LAYOUT: Lazy<Arc<MoveTypeLayout>> = Lazy::new(|| {
    Arc::new(MoveTypeLayout::Native(
        IdentifierMappingKind::Aggregator,
        Box::new(MoveTypeLayout::U128),
    ))
});

pub const AGGREGATOR_V1_SIZE: u32 = 16;

/// Tracks every aggregator V1 touched by a single transaction.
///
/// Aggregator lifecycle (which state items are created, read, or destroyed) is
/// tracked the same way regardless of the delayed field optimization. Only the
/// value representation differs:
///   - optimization disabled: a concrete u128, read from storage on first touch
///     and updated in place;
///   - optimization enabled: a stable [`DelayedFieldID`], whose value (or delta)
///     is tracked by the delayed field extension.
#[derive(Default)]
pub struct AggregatorData {
    /// All aggregators that were created in the current transaction.
    // within a single transaction.
    new_aggregators: BTreeSet<AggregatorID>,
    /// All aggregators that were destroyed in the current transaction.
    destroyed_aggregators: BTreeSet<AggregatorID>,
    /// All aggregators read in this transaction.
    read_aggregators: BTreeSet<AggregatorID>,
    /// Concrete values (delayed field optimization disabled).
    values: BTreeMap<AggregatorID, u128>,
    /// Delayed field ids (delayed field optimization enabled).
    ids: BTreeMap<AggregatorID, DelayedFieldID>,
}

impl AggregatorData {
    /// Marks an aggregator as created in this transaction. The caller
    /// initializes its value (zero) or its delayed field id separately.
    pub fn create_new_aggregator(&mut self, id: AggregatorID) {
        self.new_aggregators.insert(id);
    }

    /// Marks an aggregator as read in this transaction, fixing it to a
    /// concrete (written) value.
    pub fn mark_read(&mut self, id: AggregatorID) {
        self.read_aggregators.insert(id);
    }

    /// Returns the concrete value tracked for an aggregator.
    ///
    /// Called only when delayed field optimization is not enabled.
    pub fn get_value(&self, id: &AggregatorID) -> Option<u128> {
        self.values.get(id).copied()
    }

    /// Records the concrete value of an aggregator.
    ///
    /// Called only when delayed field optimization is not enabled.
    pub fn set_value(&mut self, id: AggregatorID, value: u128) {
        self.values.insert(id, value);
    }

    /// Returns the [`DelayedFieldID`] tracked for an aggregator.
    ///
    /// Called only when delayed field optimization is enabled.
    pub fn get_id(&self, id: &AggregatorID) -> Option<DelayedFieldID> {
        self.ids.get(id).copied()
    }

    /// Records the [`DelayedFieldID`] of an aggregator.
    ///
    /// Called only when delayed field optimization is enabled.
    pub fn set_id(&mut self, id: AggregatorID, delayed_field_id: DelayedFieldID) {
        self.ids.insert(id, delayed_field_id);
    }

    /// Returns the number of aggregators touched in this transaction. Used
    /// for deterministic key derivation, so it counts every touched aggregator
    /// regardless of the optimization (only one of the two value maps is
    /// populated at a time).
    pub fn num_aggregators(&self) -> u128 {
        debug_assert!(self.values.is_empty() || self.ids.is_empty());
        (self.values.len() + self.ids.len()) as u128
    }

    /// Destroys an aggregator. If it was created in this transaction, the create and destroy cancel
    /// out and nothing is written; otherwise it is marked for deletion from storage.
    pub fn remove_aggregator(&mut self, id: AggregatorID) {
        self.values.remove(&id);
        self.ids.remove(&id);
        self.read_aggregators.remove(&id);

        // If the aggregator was created in this transaction, the create and destroy cancel out;
        // otherwise it has to be deleted from storage.
        if !self.new_aggregators.remove(&id) {
            self.destroyed_aggregators.insert(id);
        }
    }

    /// Unpacks the tracked aggregators: (new, destroyed, read, values, ids).
    pub fn into(
        self,
    ) -> (
        BTreeSet<AggregatorID>,
        BTreeSet<AggregatorID>,
        BTreeSet<AggregatorID>,
        BTreeMap<AggregatorID, u128>,
        BTreeMap<AggregatorID, DelayedFieldID>,
    ) {
        (
            self.new_aggregators,
            self.destroyed_aggregators,
            self.read_aggregators,
            self.values,
            self.ids,
        )
    }
}

/// Returns partial VM error on extension failure.
pub fn extension_error(message: impl ToString) -> PartialVMError {
    PartialVMError::new(StatusCode::VM_EXTENSION_ERROR).with_message(message.to_string())
}

// ================================= Tests =================================

#[cfg(test)]
mod test {
    use super::*;
    use crate::aggregator_v1_id_for_test;

    #[test]
    fn create_then_destroy_within_txn_is_elided() {
        let mut data = AggregatorData::default();
        let id = aggregator_v1_id_for_test(1);

        data.create_new_aggregator(id.clone());
        data.set_value(id.clone(), 0);
        data.remove_aggregator(id.clone());

        let (new, destroyed, read, values, ids) = data.into();
        assert!(new.is_empty());
        // Created and destroyed in the same transaction: nothing is written.
        assert!(destroyed.is_empty());
        assert!(read.is_empty());
        assert!(values.is_empty());
        assert!(ids.is_empty());
    }

    #[test]
    fn destroy_existing_aggregator_is_recorded() {
        let mut data = AggregatorData::default();
        let id = aggregator_v1_id_for_test(1);

        data.remove_aggregator(id.clone());

        let (new, destroyed, _read, _values, _ids) = data.into();
        assert!(new.is_empty());
        assert_eq!(destroyed.len(), 1);
        assert!(destroyed.contains(&id));
    }

    #[test]
    fn num_aggregators_counts_touched() {
        let mut value_data = AggregatorData::default();
        let mut id_data = AggregatorData::default();

        assert_eq!(value_data.num_aggregators(), 0);
        assert_eq!(id_data.num_aggregators(), 0);

        value_data.set_value(aggregator_v1_id_for_test(1), 10);
        id_data.set_id(
            aggregator_v1_id_for_test(2),
            DelayedFieldID::new_with_width(5, 16),
        );

        assert_eq!(value_data.num_aggregators(), 1);
        assert_eq!(id_data.num_aggregators(), 1);
    }
}
