// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    data_cache::{IntoMoveResolver, RemoteStorageOwned},
    delta_ext::{deserialize, serialize},
};
use anyhow::Error;
use aptos_parallel_executor::executor::{MVHashMapView, ReadStatus};
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
            ReadStatus::Value(v) => Ok(match v.as_ref() {
                WriteOp::Value(w) => Some(w.clone()),
                WriteOp::Deletion => None,
            }),
            ReadStatus::ResolvedDelta(value) => Ok(Some(serialize(&value))),
            ReadStatus::UnresolvedDelta(delta) => {
                let value = self
                    .base_view
                    .get_state_value(state_key)?
                    .map_or(Err(Error::msg("value not found")), |bytes| {
                        Ok(deserialize(&bytes))
                    })?;

                // TODO: add proper handling of delta operations.
                let result = value
                    .checked_add(delta)
                    .map_or(Err(Error::msg("overflow")), |v| Ok(v))?;
                Ok(Some(serialize(&result)))
            }
            ReadStatus::None => self.base_view.get_state_value(state_key),
        }
    }

    fn is_genesis(&self) -> bool {
        self.base_view.is_genesis()
    }
}
