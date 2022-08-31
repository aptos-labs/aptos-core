// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::data_cache::{IntoMoveResolver, StorageAdapterOwned};
use aptos_aggregator::delta_change_set::{deserialize, serialize};
use aptos_parallel_executor::executor::{MVHashMapView, ReadResult};
use aptos_state_view::{StateView, StateViewId};
use aptos_types::state_store::state_storage_usage::StateStorageUsage;
use aptos_types::{
    state_store::state_key::StateKey,
    vm_status::{StatusCode, VMStatus},
    write_set::WriteOp,
};
use move_deps::move_binary_format::errors::Location;

pub(crate) struct VersionedView<'a, S: StateView> {
    base_view: &'a S,
    hashmap_view: &'a MVHashMapView<'a, StateKey, WriteOp>,
}

impl<'a, S: StateView> VersionedView<'a, S> {
    pub fn new_view(
        base_view: &'a S,
        hashmap_view: &'a MVHashMapView<'a, StateKey, WriteOp>,
    ) -> StorageAdapterOwned<VersionedView<'a, S>> {
        VersionedView {
            base_view,
            hashmap_view,
        }
        .into_move_resolver()
    }
}

impl<'a, S: StateView> StateView for VersionedView<'a, S> {
    fn id(&self) -> StateViewId {
        self.base_view.id()
    }

    // Get some data either through the cache or the `StateView` on a cache miss.
    fn get_state_value(&self, state_key: &StateKey) -> anyhow::Result<Option<Vec<u8>>> {
        match self.hashmap_view.read(state_key) {
            ReadResult::Value(v) => Ok(match v.as_ref() {
                WriteOp::Modification(w) | WriteOp::Creation(w) => Some(w.clone()),
                WriteOp::Deletion => None,
            }),
            ReadResult::U128(v) => Ok(Some(serialize(&v))),
            ReadResult::Unresolved(delta) => {
                let from_storage = self
                    .base_view
                    .get_state_value(state_key)?
                    .map_or(Err(VMStatus::Error(StatusCode::STORAGE_ERROR)), |bytes| {
                        Ok(deserialize(&bytes))
                    })?;
                let result = delta
                    .apply_to(from_storage)
                    .map_err(|pe| pe.finish(Location::Undefined).into_vm_status())?;
                Ok(Some(serialize(&result)))
            }
            ReadResult::None => self.base_view.get_state_value(state_key),
        }
    }

    fn is_genesis(&self) -> bool {
        self.base_view.is_genesis()
    }

    fn get_usage(&self) -> anyhow::Result<StateStorageUsage> {
        self.base_view.get_usage()
    }
}
