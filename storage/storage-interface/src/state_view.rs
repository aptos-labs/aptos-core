// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::DbReader;
use anyhow::Result;
use aptos_state_view::StateView;
use aptos_types::{state_store::state_key::StateKey, transaction::Version};

pub struct DbStateView<R: AsRef<dyn DbReader>> {
    db: R,
    version: Option<Version>,
}

impl<R: AsRef<dyn DbReader>> DbStateView<R> {
    fn get(&self, key: &StateKey) -> Result<Option<Vec<u8>>> {
        if let Some(version) = self.version {
            self.db
                .as_ref()
                .get_state_value_by_version(key, version)
                .map(|value_opt| {
                    // Hack: `v.maybe_bytes == None` represents deleted value, deemed non-existent
                    value_opt.and_then(|value| value.maybe_bytes)
                })
        } else {
            Ok(None)
        }
    }
}

impl<R: AsRef<dyn DbReader> + Sync> StateView for DbStateView<R> {
    fn get_state_value(&self, state_key: &StateKey) -> Result<Option<Vec<u8>>> {
        self.get(state_key)
    }

    fn is_genesis(&self) -> bool {
        self.version.is_none()
    }
}

pub trait LatestDbStateCheckpointView<R: AsRef<dyn DbReader>> {
    fn latest_state_checkpoint_view(&self) -> Result<DbStateView<R>>;
}

impl<R: AsRef<dyn DbReader> + Clone> LatestDbStateCheckpointView<R> for R {
    fn latest_state_checkpoint_view(&self) -> Result<DbStateView<R>> {
        Ok(DbStateView {
            db: self.clone(),
            version: self.as_ref().get_latest_state_snapshot()?.map(|(v, _)| v),
        })
    }
}

pub trait DbStateViewAtVersion<R: AsRef<dyn DbReader>> {
    fn state_view_at_version(&self, version: Option<Version>) -> Result<DbStateView<R>>;
}

impl<R: AsRef<dyn DbReader> + Clone> DbStateViewAtVersion<R> for R {
    fn state_view_at_version(&self, version: Option<Version>) -> Result<DbStateView<R>> {
        Ok(DbStateView {
            db: self.clone(),
            version,
        })
    }
}
