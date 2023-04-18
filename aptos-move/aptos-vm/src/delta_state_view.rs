// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::{bail, Result};
use aptos_state_view::{StateViewId, TStateView};
use aptos_types::{
    state_store::{
        state_key::StateKey, state_storage_usage::StateStorageUsage, state_value::StateValue,
    },
    write_set::TransactionWrite,
};
use aptos_vm_types::{
    change_set::WriteChangeSet,
    effects::Op,
    remote_cache::{TRemoteCache, TStateViewWithRemoteCache},
    write::WriteOp,
};
use move_vm_types::resolver::{ModuleRef, ResourceRef};

pub struct DeltaStateView<'a, 'b, S> {
    base: &'a S,
    writes: &'b WriteChangeSet,
    // TODO: add deltas here!
}

impl<'a, 'b, S> DeltaStateView<'a, 'b, S> {
    pub fn new(base: &'a S, writes: &'b WriteChangeSet) -> Self {
        Self { base, writes }
    }
}

impl<'a, 'b, S> TStateViewWithRemoteCache for DeltaStateView<'a, 'b, S>
where
    S: TStateViewWithRemoteCache<CommonKey = StateKey>,
{
    type CommonKey = StateKey;
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
            Some(write) => Ok(write.clone().into_write_op()?.as_state_value()),
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

impl<'a, 'b, S> TRemoteCache for DeltaStateView<'a, 'b, S>
where
    S: TRemoteCache<Key = StateKey>,
{
    type Key = StateKey;

    fn get_move_module(&self, state_key: &Self::Key) -> anyhow::Result<Option<ModuleRef>> {
        Ok(match self.writes.get(state_key) {
            Some(WriteOp::ModuleWrite(op)) => match op {
                // TODO: avoid clone by storing the ref in the transaction output directly.
                Op::Creation(m) | Op::Modification(m) => Some(ModuleRef::new(m.clone())),
                Op::Deletion => None,
            },
            Some(_) => bail!("encountered non-module when reading a module"),
            None => self.base.get_move_module(state_key)?,
        })
    }

    fn get_move_resource(&self, state_key: &Self::Key) -> anyhow::Result<Option<ResourceRef>> {
        Ok(match self.writes.get(state_key) {
            Some(WriteOp::ResourceWrite(op)) => match op {
                // TODO: avoid clone by storing the ref in the transaction output directly.
                Op::Creation(r) | Op::Modification(r) => Some(ResourceRef::new(r.clone())),
                Op::Deletion => None,
            },
            Some(_) => bail!("encountered non-resource when reading a resource"),
            None => self.base.get_move_resource(state_key)?,
        })
    }

    fn get_aggregator_value(&self, state_key: &Self::Key) -> anyhow::Result<Option<u128>> {
        Ok(match self.writes.get(state_key) {
            Some(WriteOp::AggregatorWrite(value)) => value.clone(),
            Some(_) => bail!("encountered non-aggregator value when reading an aggregator"),
            None => self.base.get_aggregator_value(state_key)?,
        })
    }
}
