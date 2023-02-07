// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_types::state_store::state_key::StateKey;
use aptos_vm_types::{
    change_set::{ChangeSetContainer, WriteChange},
    data_cache::{CachedData, DataCache},
};
use move_core_types::value::MoveTypeLayout;

/// Data cache which additionally stores a change set.
pub struct DataCacheWithChanges<'a, 'b, S> {
    data_cache: &'a S,
    write_changes: &'b ChangeSetContainer<WriteChange>,
    // TODO: original DeltaStateView does not support deltas, but it is actually
    // easy to support, by resolving the value from delta first and
    // materializing.
    // delta_changes: &'c ChangeSetContainer<DeltaChange>,
}

impl<'a, 'b, S> DataCacheWithChanges<'a, 'b, S> {
    pub fn new(data_cache: &'a S, write_changes: &'b ChangeSetContainer<WriteChange>) -> Self {
        Self {
            data_cache,
            write_changes,
        }
    }
}

impl<'a, 'b, S: DataCache<Key = StateKey, DeserializerHint = MoveTypeLayout>> DataCache
    for DataCacheWithChanges<'a, 'b, S>
{
    type DeserializerHint = MoveTypeLayout;
    type Key = StateKey;

    fn get_value(
        &self,
        key: &Self::Key,
        hint: Option<&Self::DeserializerHint>,
    ) -> anyhow::Result<Option<CachedData>> {
        match self.write_changes.get(key) {
            Some(write_change) => Ok(write_change.cache()),
            None => self.data_cache.get_value(key, hint),
        }
    }
}
