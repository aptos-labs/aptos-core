// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    db::AptosDB,
    ledger_db::LedgerDb,
    native_store::{
        bundle::{NativeStoreBundle, NativeStoreConfigs},
        CollateralStore, NativeStoreSpec,
    },
    utils::truncation_helper::get_native_commit_progress,
};
use aptos_config::config::StorageDirPaths;
use aptos_infallible::Mutex;
use aptos_logger::info;
use aptos_schemadb::{Cache, Env};
use aptos_storage_interface::{
    state_store::collateral::{decode_collateral_writes, CollateralBase, CollateralOverlay},
    AptosDbError, Result,
};
use aptos_types::{transaction::Version, write_set::WriteSet};
use std::sync::Arc;

/// Metric label of the collateral index's layers, apart from the collateral
/// JMT pipeline's (`CollateralStore::NAME`).
const COLLATERAL_INDEX_FAMILY: &str = "collateral_index";

pub struct CollateralBundle {
    pub(crate) store: NativeStoreBundle<CollateralStore>,
    /// Complete account-grouped snapshot at the last folded version.
    /// Advanced by `advance_collateral_base` under the executor's execution
    /// lock, as the position base is.
    pub(crate) collateral_base: Arc<CollateralBase>,
    /// Tip overlay over `collateral_base`, extended once the collateral
    /// store's KV commit succeeds. In readonly mode it stays at cold-load.
    pub(crate) collateral: Arc<Mutex<CollateralOverlay>>,
}

impl AptosDB {
    pub fn collateral(&self) -> Option<&Arc<CollateralBundle>> {
        self.collateral.as_ref()
    }

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
        let (store, snapshot_version) = NativeStoreBundle::<CollateralStore>::open(
            db_paths,
            configs,
            env,
            block_cache,
            &self.ledger_db,
            readonly,
        )?;

        // Cold-load: stream the durable JMT snapshot into `CollateralBase`;
        // the catch-up below closes the gap past it.
        let collateral_base = match snapshot_version {
            Some(version) => {
                let base = CollateralBase::new_from_rows(
                    Some(version),
                    COLLATERAL_INDEX_FAMILY,
                    store.snapshot_rows(version)?,
                )?;
                info!(
                    snapshot_version = version,
                    n_accounts = base.read(|view| view.num_accounts()),
                    "Native-collateral cold-load complete."
                );
                base
            },
            None => CollateralBase::new_empty(COLLATERAL_INDEX_FAMILY),
        };
        let collateral_base = Arc::new(collateral_base);
        let collateral = Arc::new(Mutex::new(collateral_base.new_overlay()));
        let bundle = CollateralBundle {
            store,
            collateral_base,
            collateral,
        };

        if readonly {
            catch_up_readonly(&bundle, &self.ledger_db, snapshot_version)?;
        } else if !for_restore {
            if let Some((target_version, write_sets)) =
                bundle.store.catch_up(&self.ledger_db, snapshot_version)?
            {
                extend_tip(&bundle, target_version, &write_sets)?;
            }
        }
        self.collateral = Some(Arc::new(bundle));
        info!(
            snapshot_version = ?snapshot_version,
            readonly = readonly,
            "Native-collateral store initialized."
        );
        Ok(())
    }
}

/// Readonly counterpart of the catch-up: replays the tail past the snapshot
/// into the overlay only, writing nothing.
fn catch_up_readonly(
    bundle: &CollateralBundle,
    ledger_db: &LedgerDb,
    snapshot_version: Option<Version>,
) -> Result<()> {
    let Some(v_overall) = ledger_db.metadata_db().get_synced_version()? else {
        return Ok(());
    };
    let snapshot_next_version = snapshot_version.map_or(0, |v| v + 1);
    if get_native_commit_progress(&bundle.store.kv_db)?.is_none()
        || snapshot_next_version > v_overall
    {
        return Ok(());
    }
    let write_sets = NativeStoreBundle::<CollateralStore>::write_sets_after_snapshot(
        ledger_db,
        snapshot_next_version,
        v_overall + 1,
    )?;
    extend_tip(bundle, v_overall, &write_sets)
}

/// Extends the published overlay with the collateral writes in `write_sets`,
/// which end at `target_version`.
pub(crate) fn extend_tip(
    bundle: &CollateralBundle,
    target_version: Version,
    write_sets: &[WriteSet],
) -> Result<()> {
    let writes = decode_collateral_writes(
        write_sets
            .iter()
            .flat_map(CollateralStore::write_set_ops)
            .map(|(key, op)| (key, op.as_state_value_opt())),
    )?;
    if !writes.is_empty() {
        let mut collateral = bundle.collateral.lock();
        *collateral = collateral.extend(&bundle.collateral_base.layers(), target_version, writes);
    }
    Ok(())
}
