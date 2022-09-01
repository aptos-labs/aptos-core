// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use aptos_state_view::{StateView, StateViewId};
use aptos_types::state_store::state_key::StateKey;
use aptos_types::state_store::state_storage_usage::StateStorageUsage;
use aptos_types::write_set::{WriteOp, WriteSet};

pub struct DeltaStateView<'a, 'b, S> {
    base: &'a S,
    write_set: &'b WriteSet,
}

impl<'a, 'b, S> DeltaStateView<'a, 'b, S> {
    pub fn new(base: &'a S, write_set: &'b WriteSet) -> Self {
        Self { base, write_set }
    }
}

impl<'a, 'b, S> StateView for DeltaStateView<'a, 'b, S>
where
    S: StateView,
{
    fn id(&self) -> StateViewId {
        self.base.id()
    }

    fn get_state_value(&self, state_key: &StateKey) -> Result<Option<Vec<u8>>> {
        match self.write_set.get(state_key) {
            Some(WriteOp::Creation(data) | WriteOp::Modification(data)) => Ok(Some(data.clone())),
            Some(WriteOp::Deletion) => Ok(None),
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
