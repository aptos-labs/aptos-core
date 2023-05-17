// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_state_view::StateViewId;
use aptos_types::state_store::{state_key::StateKey, state_storage_usage::StateStorageUsage};
use aptos_vm_types::{
    vm_view::{AptosVMView, VMView},
    write_change_set::WriteChangeSet,
};
use move_vm_types::types::Store;

pub struct DeltaStateView<'a, 'b, S, R: Store, M: Store, A: Store> {
    base: &'a S,
    resource_writes: &'b WriteChangeSet<R>,
    module_writes: &'b WriteChangeSet<M>,
    aggregator_writes: &'b WriteChangeSet<A>,
}

impl<'a, 'b, S, R: Store, M: Store, A: Store> DeltaStateView<'a, 'b, S, R, M, A> {
    pub fn new(
        base: &'a S,
        resource_writes: &'b WriteChangeSet<R>,
        module_writes: &'b WriteChangeSet<M>,
        aggregator_writes: &'b WriteChangeSet<A>,
    ) -> Self {
        Self {
            base,
            resource_writes,
            module_writes,
            aggregator_writes,
        }
    }
}

impl<'a, 'b, S: AptosVMView, R: Store, M: Store, A: Store> VMView
    for DeltaStateView<'a, 'b, S, R, M, A>
{
    type Key = StateKey;

    fn id(&self) -> StateViewId {
        self.base.id()
    }

    fn get_move_module(&self, state_key: &Self::Key) -> anyhow::Result<Option<Vec<u8>>> {
        match self.module_writes.get(state_key) {
            // TODO: Make as_bytes return Result.
            Some(module_op) => Ok(module_op
                .ok()
                .map(|m| m.as_bytes().expect("serialisation should always succeed"))),
            None => self.base.get_move_module(state_key),
        }
    }

    fn get_move_resource(&self, state_key: &Self::Key) -> anyhow::Result<Option<Vec<u8>>> {
        match self.resource_writes.get(state_key) {
            // TODO: Make as_bytes return Result.
            Some(resource_op) => Ok(resource_op
                .ok()
                .map(|r| r.as_bytes().expect("serialisation should always succeed"))),
            None => self.base.get_move_resource(state_key),
        }
    }

    fn get_aggregator_value(&self, state_key: &Self::Key) -> anyhow::Result<Option<Vec<u8>>> {
        match self.aggregator_writes.get(state_key) {
            // TODO: Make as_bytes return Result.
            Some(aggregator_op) => Ok(aggregator_op
                .ok()
                .map(|a| a.as_bytes().expect("serialisation should always succeed"))),
            None => self.base.get_aggregator_value(state_key),
        }
    }

    fn get_storage_usage_at_epoch_end(&self) -> anyhow::Result<StateStorageUsage> {
        // TODO(Gas): Check if this is correct.
        self.base.get_storage_usage_at_epoch_end()
    }
}
