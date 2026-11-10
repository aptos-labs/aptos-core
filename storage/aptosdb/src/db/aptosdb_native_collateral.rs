// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    db::AptosDB,
    ledger_db::LedgerDb,
    native_store::{
        bundle::{NativeStoreBundle, NativeStoreConfigs},
        state_sync::NativeStoreReader,
        CollateralStore, NativeStoreSpec,
    },
    utils::truncation_helper::get_native_commit_progress,
};
use aptos_config::config::StorageDirPaths;
use aptos_crypto::HashValue;
use aptos_infallible::Mutex;
use aptos_logger::info;
use aptos_schemadb::{Cache, Env};
use aptos_storage_interface::{
    db_ensure as ensure,
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
    /// `for_restore` opens without catching up to the ledger; the restore
    /// brings the store up once it has saved its transactions
    /// (`reset_collateral_store_to_ledger`).
    ///
    /// Collateral has no snapshot kind for state sync yet, so fast sync and
    /// restore leave the collateral store as opened, and catch-up replays
    /// only past a store that has committed before. A node whose store
    /// disagrees with the collateral root the ledger tip commits refuses to
    /// open rather than diverge at its next block. Nothing catches a ledger
    /// that carries collateral but no collateral root yet, so enable the
    /// storage before `NATIVE_COLLATERAL`.
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
            ensure_collateral_root_matches_ledger(&bundle, &self.ledger_db)?;
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

impl AptosDB {
    pub(crate) fn collateral_store_reader(&self) -> Result<NativeStoreReader<'_, CollateralStore>> {
        let bundle = self
            .collateral
            .as_ref()
            .ok_or_else(|| AptosDbError::Other("native-collateral store is not enabled".into()))?;
        Ok(bundle.store.reader(&self.ledger_db))
    }

    /// Rebuild the collateral store against the snapshot a fast sync just
    /// restored at `version`: the store, then the resident index and the
    /// overlay over it.
    pub(crate) fn reset_collateral_after_fast_sync(
        &self,
        version: Version,
        expected_root_hash: HashValue,
    ) -> Result<()> {
        let Some(bundle) = self.collateral.as_ref() else {
            return Ok(());
        };
        bundle.store.reset_after_fast_sync(
            &self.ledger_db,
            version,
            expected_root_hash,
            |rows| {
                // Held across the rebuild: until the re-seat the published
                // overlay relates to nothing in the base.
                let mut tip = bundle.collateral.lock();
                bundle.collateral_base.reset_from_rows(
                    Some(version),
                    COLLATERAL_INDEX_FAMILY,
                    rows.into_iter().flatten(),
                )?;
                *tip = bundle.collateral_base.new_overlay();
                Ok(())
            },
        )?;

        info!(
            version = version,
            n_accounts = bundle.collateral_base.read(|view| view.num_accounts()),
            "Collateral store reset after fast sync."
        );
        Ok(())
    }

    /// Bring the collateral store up to the ledger after a restore saved
    /// transactions past its snapshot without executing them.
    pub(crate) fn reset_collateral_store_to_ledger(&self) -> Result<()> {
        let Some(bundle) = self.collateral.as_ref() else {
            return Ok(());
        };
        let v_overall = self.ledger_db.metadata_db().get_synced_version()?;
        let snapshot_version = bundle
            .store
            .merkle_db
            .latest_snapshot_version_at_or_before(v_overall.unwrap_or(Version::MAX))?;
        if let Some((target_version, write_sets)) =
            bundle.store.catch_up(&self.ledger_db, snapshot_version)?
        {
            extend_tip(bundle, target_version, &write_sets)?;
        }
        ensure_collateral_root_matches_ledger(bundle, &self.ledger_db)
    }

    /// Why a collateral-summary read cannot be served: storage is off, or the
    /// DB was opened readonly, which keeps no collateral pipeline.
    pub(crate) fn collateral_storage_unavailable(&self) -> AptosDbError {
        let reason = if self.collateral.is_some() {
            "the DB was opened readonly"
        } else {
            "storage.rocksdb_configs.enable_trading_native is off"
        };
        AptosDbError::Other(format!(
            "COMPUTE_TRADING_NATIVE_STATE_ROOTS is enabled but native-collateral storage is not \
             available: {reason}"
        ))
    }
}

/// Refuse to open when the ledger tip commits a collateral root the store
/// disagrees with, as `ensure_position_root_matches_ledger` does for
/// positions.
fn ensure_collateral_root_matches_ledger(
    bundle: &CollateralBundle,
    ledger_db: &LedgerDb,
) -> Result<()> {
    let Some(store) = bundle.store.state_store.as_ref() else {
        return Ok(());
    };
    let Some(v_overall) = ledger_db.metadata_db().get_synced_version()? else {
        return Ok(());
    };
    let Some(committed_root) = ledger_db
        .transaction_info_db()
        .get_transaction_info(v_overall)?
        .collateral_state_checkpoint_hash()
    else {
        return Ok(());
    };
    let latest = store.current_state().lock().last_checkpoint().clone();
    let local_root = latest.root_hash();
    ensure!(
        local_root == committed_root,
        "Native-collateral store (version {:?}, root {local_root:?}) disagrees with the \
         collateral root {committed_root:?} committed at ledger version {v_overall}. Collateral \
         storage was likely enabled on a node whose history already carries collateral, or the \
         node was restored without it.",
        latest.version(),
    );
    Ok(())
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
