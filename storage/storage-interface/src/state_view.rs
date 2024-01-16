// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::DbReader;
use aptos_types::{
    state_store::{
        errors::StateviewError, state_key::StateKey, state_storage_usage::StateStorageUsage,
        state_value::StateValue, TStateView,
    },
    transaction::Version,
};
use std::sync::Arc;

type Result<T, E = StateviewError> = std::result::Result<T, E>;

pub struct DbStateView {
    pub db: Arc<dyn DbReader>,
    pub version: Option<Version>,
}

impl DbStateView {
    fn get(&self, key: &StateKey) -> Result<Option<StateValue>> {
        Ok(if let Some(version) = self.version {
            self.db
                .get_state_value_by_version(key, version)
                .map_err(Into::<StateviewError>::into)?
        } else {
            None
        })
    }
}

impl TStateView for DbStateView {
    type Key = StateKey;

    fn get_state_value(&self, state_key: &StateKey) -> Result<Option<StateValue>> {
        self.get(state_key).map_err(Into::into)
    }

    fn get_usage(&self) -> Result<StateStorageUsage> {
        self.db
            .get_state_storage_usage(self.version)
            .map_err(Into::into)
    }
}

pub trait LatestDbStateCheckpointView {
    fn latest_state_checkpoint_view(&self) -> Result<DbStateView>;
}

impl LatestDbStateCheckpointView for Arc<dyn DbReader> {
    fn latest_state_checkpoint_view(&self) -> Result<DbStateView> {
        Ok(DbStateView {
            db: self.clone(),
            version: self
                .get_latest_state_checkpoint_version()
                .map_err(Into::<StateviewError>::into)?,
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
