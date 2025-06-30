// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Context;
use aptos_config::config::{StorageConfig, StorageDirPaths, NO_OP_STORAGE_PRUNER_CONFIG};
use aptos_db::AptosDB;
use aptos_storage_interface::{
    state_store::state_view::db_state_view::{DbStateView, DbStateViewAtVersion},
    DbReader,
};
use aptos_types::state_store::state_key::StateKey;
use either::Either;
use std::{ops::Deref, path::PathBuf, sync::Arc};

pub struct Storage(Arc<dyn DbReader>);

impl Storage {
    pub fn open(path: &PathBuf) -> anyhow::Result<Self> {
        let config = StorageConfig::default();
        let aptos_db = AptosDB::open(
            StorageDirPaths::from_path(path),
            true,
            NO_OP_STORAGE_PRUNER_CONFIG,
            Default::default(),
            false,
            config.buffered_state_target_items,
            config.max_num_nodes_per_lru_cache_shard,
            None,
        )
        .context("failed to open aptos db")?;

        Ok(Self(Arc::new(aptos_db)))
    }

    /// Gets an [Arc] to the db reader.
    fn db_reader(&self) -> Arc<dyn DbReader> {
        self.0.clone()
    }

    /// Gets the latest version of the ledger.
    pub fn latest_ledger_version(&self) -> Result<u64, anyhow::Error> {
        let latest_ledger_info = self
            .db_reader()
            .get_latest_ledger_info()
            .context("failed to get latest ledger info")?;

        Ok(latest_ledger_info.ledger_info().version())
    }

    /// Gets the state view at a given version.
    pub fn state_view_at_version(
        &self,
        version: Option<u64>,
    ) -> Result<DbStateView, anyhow::Error> {
        let state_view = self.db_reader().state_view_at_version(version)?;

        Ok(state_view)
    }

    /// Gets the all [StateKey]s in the global storage dating back to an original version. None is treated as 0 or all versions.
    pub fn global_state_keys_from_version(&self, version: Option<u64>) -> GlobalStateKeyIterable {
        GlobalStateKeyIterable {
            db_reader: self.db_reader(),
            version: version.unwrap_or(0),
        }
    }
}

pub struct MovementStorage(Storage);

impl MovementStorage {
    pub fn open(path: &PathBuf) -> anyhow::Result<Self> {
        let storage = Storage::open(path)?;
        Ok(Self(storage))
    }
}

impl Deref for MovementStorage {
    type Target = Storage;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
pub struct MovementAptosStorage(Storage);

impl MovementAptosStorage {
    pub fn open(path: &PathBuf) -> anyhow::Result<Self> {
        let storage = Storage::open(path)?;
        Ok(Self(storage))
    }
}

impl Deref for MovementAptosStorage {
    type Target = Storage;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// An iterable of [StateKey]s in the global storage dating back to an original version.
///
/// This helps deal with lifetime issues.
pub struct GlobalStateKeyIterable {
    db_reader: Arc<dyn DbReader>,
    version: u64,
}

const MAX_WRITE_SET_SIZE: u64 = 20_000;

impl GlobalStateKeyIterable {
    pub fn iter(
        &self,
    ) -> Result<Box<dyn Iterator<Item = Result<StateKey, anyhow::Error>> + '_>, anyhow::Error> {
        let write_set_iterator = self
            .db_reader
            .get_write_set_iterator(self.version, MAX_WRITE_SET_SIZE)?;

        // We want to iterate lazily over the write set iterator because there could be a lot of them.
        let iter = write_set_iterator.flat_map(move |res| match res {
            Ok(write_set) => {
                // It should be okay to collect because there should not be that many state keys in a write set.
                let items: Vec<_> = write_set
                    .expect_v0()
                    .iter()
                    .map(|(key, _)| Ok(key.clone()))
                    .collect();
                Either::Left(items.into_iter())
            },
            Err(e) => Either::Right(std::iter::once(Err(e.into()))),
        });

        Ok(Box::new(iter))
    }
}
