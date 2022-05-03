// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::DbReader;
use anyhow::Result;
use aptos_state_view::StateView;
use aptos_types::{state_store::state_key::StateKey, transaction::Version};
use std::sync::Arc;

pub struct DbStateView {
    db: Arc<dyn DbReader>,
    version: Option<Version>,
}

impl DbStateView {
    fn get(&self, key: &StateKey) -> Result<Option<Vec<u8>>> {
        if let Some(version) = self.version {
            self.db
                .get_state_value_with_proof_by_version(key, version)
                .map(|(value_opt, _proof)| {
                    // Hack: `v.maybe_bytes == None` represents deleted value, deemed non-existent
                    value_opt.and_then(|value| value.maybe_bytes)
                })
        } else {
            Ok(None)
        }
    }
}

impl StateView for DbStateView {
    fn get_state_value(&self, state_key: &StateKey) -> Result<Option<Vec<u8>>> {
        self.get(state_key)
    }

    fn is_genesis(&self) -> bool {
        self.version.is_none()
    }
}

pub trait LatestDbStateView {
    fn latest_state_view(&self) -> Result<DbStateView>;
}

impl LatestDbStateView for Arc<dyn DbReader> {
    fn latest_state_view(&self) -> Result<DbStateView> {
        Ok(DbStateView {
            db: self.clone(),
            version: self.get_latest_version_option()?,
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
