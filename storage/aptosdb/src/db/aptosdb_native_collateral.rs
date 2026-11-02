// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    db::AptosDB,
    native_store::{
        bundle::{NativeStoreBundle, NativeStoreConfigs},
        CollateralStore,
    },
};
use aptos_config::config::StorageDirPaths;
use aptos_logger::info;
use aptos_schemadb::{Cache, Env};
use aptos_storage_interface::{AptosDbError, Result};
use std::sync::Arc;

pub type CollateralBundle = NativeStoreBundle<CollateralStore>;

impl AptosDB {
    /// Called from `open_internal` beside `init_native_order` when
    /// `rocksdb_configs.enable_trading_native` is set.
    ///
    /// Collateral has no state root and no snapshot kind for state sync yet,
    /// so fast sync and restore leave the collateral store as opened: a node
    /// restored that way holds only the collateral written after its target.
    ///
    /// For the same reason nothing catches a first enable on a ledger that
    /// already carries collateral writes: catch-up replays only past a store
    /// that has committed before, so the store opens empty. Enable the storage
    /// before `NATIVE_COLLATERAL`.
    ///
    /// TODO[native_collateral](root): once the collateral root is computed,
    /// refuse to open when the store disagrees with the committed root, as
    /// `ensure_position_root_matches_ledger` does for positions.
    pub(crate) fn init_native_collateral(
        &mut self,
        db_paths: &StorageDirPaths,
        configs: &NativeStoreConfigs,
        env: &Env,
        block_cache: &Cache,
        readonly: bool,
        for_restore: bool,
    ) -> Result<()> {
        if self.collateral.is_some() {
            return Err(AptosDbError::Other(
                "init_native_collateral called twice; the collateral store is already attached \
                 to this AptosDB"
                    .to_string(),
            ));
        }
        let (store, snapshot_version) = CollateralBundle::open(
            db_paths,
            configs,
            env,
            block_cache,
            &self.ledger_db,
            readonly,
        )?;
        if !readonly && !for_restore {
            store.catch_up(&self.ledger_db, snapshot_version)?;
        }
        self.collateral = Some(Arc::new(store));
        info!(
            snapshot_version = ?snapshot_version,
            readonly = readonly,
            "Native-collateral store initialized."
        );
        Ok(())
    }
}
