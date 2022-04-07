// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::data_cache::{IntoMoveResolver, RemoteStorageOwned};
use aptos_parallel_executor::executor::MVHashMapView;
use aptos_state_view::{StateView, StateViewId};
use aptos_types::{state_store::state_key::StateKey, write_set::WriteOp};

pub(crate) struct VersionedView<'a, S: StateView> {
    base_view: &'a S,
    hashmap_view: &'a MVHashMapView<'a, StateKey, WriteOp>,
}

impl<'a, S: StateView> VersionedView<'a, S> {
    pub fn new_view(
        base_view: &'a S,
        hashmap_view: &'a MVHashMapView<'a, StateKey, WriteOp>,
    ) -> RemoteStorageOwned<VersionedView<'a, S>> {
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
            Some(v) => Ok(match v.as_ref() {
                WriteOp::Value(w) => Some(w.clone()),
                WriteOp::Deletion => None,
            }),
            None => self.base_view.get_state_value(state_key),
        }
    }

    fn is_genesis(&self) -> bool {
        self.base_view.is_genesis()
    }
}
