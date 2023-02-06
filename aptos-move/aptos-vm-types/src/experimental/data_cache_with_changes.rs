// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! New implementation of DeltaStateView.

use crate::{
    change_set::{DeltaChangeSet, WriteChange, WriteChangeSet},
    data_cache::{CachedData, DataCache},
};
use aptos_types::state_store::state_key::StateKey;

pub struct DataCacheWithChanges<'a, 'b, S> {
    data_cache: &'a S,
    write_changes: &'b WriteChangeSet,
    // TODO: original DeltaStateView does not support deltas, but it is actually
    // easy to support.
    // delta_changes: &'b DeltaChangeSet,
}

impl<'a, 'b, S> DataCacheWithChanges<'a, 'b, S> {
    pub fn new(data_cache: &'a S, write_changes: &'b WriteChangeSet) -> Self {
        Self {
            data_cache,
            write_changes,
        }
    }
}

impl<'a, 'b, S: DataCache<Key = StateKey>> DataCache for DataCacheWithChanges<'a, 'b, S> {
    type Key = StateKey;

    fn get_value(&self, key: &Self::Key) -> anyhow::Result<Option<CachedData>> {
        match self.write_changes.get(key) {
            Some(WriteChange::Creation(data) | WriteChange::Modification(data)) => {
                Ok(Some(data.as_cached_data()))
            },
            Some(WriteChange::Deletion) => Ok(None),
            None => self.data_cache.get_value(key),
        }
    }
}
