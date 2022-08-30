// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::DbReader;
use anyhow::Result;
use aptos_state_view::StateView;
use aptos_types::state_store::state_storage_usage::StateStorageUsage;
use aptos_types::{state_store::state_key::StateKey, transaction::Version};
use std::sync::Arc;

pub struct DbStateView {
    pub db: Arc<dyn DbReader>,
    pub version: Option<Version>,
}

impl DbStateView {
    fn get(&self, key: &StateKey) -> Result<Option<Vec<u8>>> {
        Ok(if let Some(version) = self.version {
            self.db
                .get_state_value_by_version(key, version)?
                .map(|value| value.into_bytes())
        } else {
            None
        })
    }
}

impl StateView for DbStateView {
    fn get_state_value(&self, state_key: &StateKey) -> Result<Option<Vec<u8>>> {
        self.get(state_key)
    }

    fn is_genesis(&self) -> bool {
        self.version.is_none()
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
