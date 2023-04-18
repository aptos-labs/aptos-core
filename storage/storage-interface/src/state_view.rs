// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::DbReader;
use anyhow::Result;
use aptos_state_view::TStateView;
use aptos_types::{
    state_store::{
        state_key::StateKey, state_storage_usage::StateStorageUsage, state_value::StateValue,
    },
    transaction::Version,
};
use aptos_vm_types::remote_cache::{TRemoteCache, TStateViewWithRemoteCache};
use move_vm_types::resolver::{Module, ModuleRef, Resource, ResourceRef};
use std::sync::Arc;

pub struct DbStateView {
    pub db: Arc<dyn DbReader>,
    pub version: Option<Version>,
}

impl DbStateView {
    fn get(&self, key: &StateKey) -> Result<Option<StateValue>> {
        Ok(if let Some(version) = self.version {
            self.db.get_state_value_by_version(key, version)?
        } else {
            None
        })
    }
}

impl TStateView for DbStateView {
    type Key = StateKey;

    fn get_state_value(&self, state_key: &StateKey) -> Result<Option<StateValue>> {
        self.get(state_key)
    }

    fn is_genesis(&self) -> bool {
        self.version.is_none()
    }

    fn get_usage(&self) -> Result<StateStorageUsage> {
        self.db.get_state_storage_usage(self.version)
    }
}

impl TRemoteCache for DbStateView {
    type Key = StateKey;

    fn get_move_module(&self, state_key: &Self::Key) -> anyhow::Result<Option<ModuleRef>> {
        // TODO: Should we deserialize on the call-site or here?
        Ok(self
            .get_state_value_bytes(state_key)?
            .map(|blob| ModuleRef::new(Module::Serialized(blob))))
    }

    fn get_move_resource(&self, state_key: &Self::Key) -> anyhow::Result<Option<ResourceRef>> {
        // TODO: Should we deserialize on the call-site or here?
        Ok(self
            .get_state_value_bytes(state_key)?
            .map(|blob| ResourceRef::new(Resource::Serialized(blob))))
    }

    fn get_aggregator_value(&self, state_key: &Self::Key) -> Result<Option<u128>> {
        Ok(match self.get_state_value_bytes(state_key)? {
            Some(blob) => Some(bcs::from_bytes(&blob)?),
            None => None,
        })
    }
}

impl TStateViewWithRemoteCache for DbStateView {
    type CommonKey = StateKey;
}

pub trait LatestDbStateCheckpointView {
    fn latest_state_checkpoint_view(&self) -> Result<DbStateView>;
}

impl LatestDbStateCheckpointView for Arc<dyn DbReader> {
    fn latest_state_checkpoint_view(&self) -> Result<DbStateView> {
        Ok(DbStateView {
            db: self.clone(),
            version: self.get_latest_state_checkpoint_version()?,
        })
    }
}

pub trait DbStateViewAtVersion {
    fn state_view_at_version(&self, version: Option<Version>) -> Result<DbStateView>;
}

impl DbStateViewAtVersion for Arc<dyn DbReader> {
    fn state_view_at_version(&self, version: Option<Version>) -> Result<DbStateView> {
        Ok(DbStateView {
            db: self.clone(),
            version,
        })
    }
}
