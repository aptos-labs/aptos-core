// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::DbReader;
use anyhow::{anyhow, Result};
use aptos_state_view::TStateView;
use aptos_types::{
    access_path::AccessPath,
    state_store::{
        state_key::StateKey, state_storage_usage::StateStorageUsage, state_value::StateValue,
    },
    transaction::Version,
};
use move_binary_format::CompiledModule;
use move_bytecode_utils::viewer::ModuleViewer;
use move_core_types::language_storage::ModuleId;
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

    fn get_usage(&self) -> Result<StateStorageUsage> {
        self.db.get_state_storage_usage(self.version)
    }
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

impl ModuleViewer for DbStateView {
    type Error = anyhow::Error;
    type Item = CompiledModule;

    fn view_module(&self, module_id: &ModuleId) -> Result<Self::Item, Self::Error> {
        let state_key = StateKey::access_path(AccessPath::from(module_id));
        let state_value = self
            .get_state_value(&state_key)?
            .ok_or_else(|| anyhow!("Module {:?} not found", module_id))?;
        CompiledModule::deserialize(state_value.bytes()).map_err(|e| {
            anyhow!(
                "Module {:?} failed to deserialize with error code {:?}",
                module_id,
                e,
            )
        })
    }
}
