// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use aptos_state_view::{StateViewId, TStateView};
use aptos_types::{
    state_store::{
        state_key::StateKey, state_storage_usage::StateStorageUsage, state_value::StateValue,
    },
    write_set::TransactionWrite,
};
use aptos_types::write_set::WriteSet;

pub struct DeltaStateView<'a, 'b, S> {
    base: &'a S,
    writes: &'b WriteSet,
}

impl<'a, 'b, S> DeltaStateView<'a, 'b, S> {
    pub fn new(base: &'a S, writes: &'b WriteSet) -> Self {
        Self { base, writes }
    }
}

impl<'a, 'b, S> TStateView for DeltaStateView<'a, 'b, S>
where
    S: TStateView<Key = StateKey>,
{
    type Key = StateKey;

    fn id(&self) -> StateViewId {
        self.base.id()
    }

    fn get_state_value(&self, state_key: &StateKey) -> Result<Option<StateValue>> {
        match self.writes.get(state_key) {
            Some(write_op) => Ok(write_op.as_state_value()),
            None => self.base.get_state_value(state_key),
        }
    }

    fn is_genesis(&self) -> bool {
        self.base.is_genesis()
    }

    fn get_usage(&self) -> Result<StateStorageUsage> {
        // TODO(Gas): Check if this is correct
        self.base.get_usage()
    }
}
