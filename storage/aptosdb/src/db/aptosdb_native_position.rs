// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    db::AptosDB,
    native_metrics::{POSITION_BASE_LAG_VERSIONS, POSITION_OVERLAY_LAYERS},
    native_state_reader::InMemoryNativeStateReader,
    native_state_store::{decode_position_writes, PositionBase, PositionOverlay, PositionWrites},
    native_store::{
        bundle::{NativeStoreBundle, NativeStoreConfigs},
        kv_db::NUM_NATIVE_VALUE_SHARDS,
        state_sync::NativeStoreReader,
        NativeStoreSpec, PositionStore,
    },
    utils::truncation_helper::get_native_commit_progress,
};
use aptos_config::config::StorageDirPaths;
use aptos_crypto::HashValue;
use aptos_infallible::Mutex;
use aptos_logger::info;
use aptos_schemadb::{Cache, Env};
use aptos_storage_interface::{db_ensure as ensure, AptosDbError, Result};
use aptos_types::{transaction::Version, write_set::WriteSet};
use std::sync::Arc;

/// Metric label of the position index's layers, apart from the position
/// JMT pipeline's (`PositionStore::NAME`).
const POSITION_INDEX_FAMILY: &str = "position_index";

pub struct PositionBundle {
    pub(crate) store: NativeStoreBundle<PositionStore>,
    /// Complete account-grouped snapshot at the last folded version.
    /// Resolves overlay misses, which is what lets `positions` be
    /// a bounded delta rather than a chain pinned to a family root.
    /// Advanced by `advance_position_base`, which the executor calls
    /// while holding its execution lock so the base cannot move under a
    /// block's reads.
    pub(crate) position_base: Arc<PositionBase>,
    /// Tip overlay over `position_base`, extended once the position
    /// store's KV commit succeeds. In readonly mode it stays at
    /// cold-load. External callers reach this via [`NativeStateReader`]
    /// (see `native_state_reader()`), not through the bundle directly.
    ///
    /// That commit runs in `pre_commit_ledger`, which consensus gates on
    /// the block's order proof: ordered, but not yet commit-proven. No
    /// competing block can replace it; a crash discards it, along with the
    /// ledger rows above `OverallCommitProgress`.
    ///
    /// It also trails execution by however far the commit pipeline lags, so
    /// a consumer that needs a particular block's positions takes that
    /// block's overlay from the executor rather than reading here.
    pub(crate) positions: Arc<Mutex<PositionOverlay>>,
}

impl AptosDB {
    /// Sets the overlay depth gauges from the published tip and the base.
    pub(crate) fn observe_position_overlay(&self) {
        let Some(bundle) = self.position.as_ref() else {
            return;
        };
        let tip = bundle.positions.lock();
        let floor = bundle.position_base.layers();
        let base_next_version = bundle.position_base.version().map_or(0, |v| v + 1);
        POSITION_OVERLAY_LAYERS.set(tip.layers_above(&floor) as i64);
        POSITION_BASE_LAG_VERSIONS.set(tip.next_version().saturating_sub(base_next_version) as i64);
    }

    pub fn position(&self) -> Option<&Arc<PositionBundle>> {
        self.position.as_ref()
    }

    /// The position store's state-sync producers. Errors if native-position
    /// storage isn't attached.
    pub(crate) fn position_store_reader(&self) -> Result<NativeStoreReader<'_, PositionStore>> {
        let bundle = self.position.as_ref().ok_or_else(|| {
            AptosDbError::Other("native-position subsystem is not enabled".into())
        })?;
        Ok(bundle.store.reader(&self.ledger_db))
    }

    pub fn native_state_reader(&self) -> Option<InMemoryNativeStateReader> {
        let bundle = self.position.as_ref()?;
        Some(InMemoryNativeStateReader::new(
            Arc::clone(&bundle.position_base),
            Arc::clone(&bundle.positions),
        ))
    }

    /// Called automatically from `open_internal` when
    /// `rocksdb_configs.enable_trading_native` is set. Shares `env` and
    /// `block_cache` with the main AptosDB so RocksDB background
    /// threads and the block cache stay singleton.
    ///
    /// `for_restore` opens at the persisted snapshot without catching up
    /// to the ledger: a restore saves transactions ahead of the state it
    /// has restored, and re-seats the position store itself once the
    /// snapshot lands (`reset_position_store_to_ledger`).
    pub(crate) fn init_native_position(
        &mut self,
        db_paths: &StorageDirPaths,
        configs: &NativeStoreConfigs,
        env: &Env,
        block_cache: &Cache,
        readonly: bool,
        for_restore: bool,
    ) -> Result<()> {
        if self.position.is_some() {
            return Err(AptosDbError::Other(
                "init_native_position called twice; native-position subsystem is already \
                 attached to this AptosDB"
                    .to_string(),
            ));
        }

        let (store, snapshot_version) = NativeStoreBundle::<PositionStore>::open(
            db_paths,
            configs,
            env,
            block_cache,
            &self.ledger_db,
            readonly,
        )?;

        // Cold-load: stream the durable JMT snapshot into `PositionBase`,
        // decoding row by row so the live set is never materialized twice.
        // The gap `[snapshot_version + 1, chain_tip]` left by a crash
        // between JMT-snapshot and chain commit is closed by the catch-up
        // below.
        let position_base = match snapshot_version {
            Some(version) => {
                let base = PositionBase::new_from_rows(
                    Some(version),
                    POSITION_INDEX_FAMILY,
                    store.snapshot_rows(version)?,
                )?;
                info!(
                    snapshot_version = version,
                    n_accounts = base.read(|view| view.num_accounts()),
                    "Native-position cold-load complete."
                );
                base
            },
            None => PositionBase::new_empty(POSITION_INDEX_FAMILY),
        };
        let position_base = Arc::new(position_base);
        let positions = Arc::new(Mutex::new(position_base.new_overlay()));

        self.position = Some(Arc::new(PositionBundle {
            store,
            position_base,
            positions,
        }));

        if readonly {
            self.catch_up_readonly_positions(snapshot_version)?;
        } else if !for_restore {
            self.catch_up_position_store(snapshot_version)?;
            self.ensure_position_root_matches_ledger()?;
        }

        info!(
            num_shards = NUM_NATIVE_VALUE_SHARDS,
            readonly = readonly,
            "Native-position subsystem initialized."
        );

        Ok(())
    }

    /// Replays write sets past the JMT snapshot at `snapshot_version` so the
    /// pipeline, the JMT and the overlay catch up to the chain tip.
    fn catch_up_position_store(&self, snapshot_version: Option<Version>) -> Result<()> {
        let Some(bundle) = self.position.as_ref() else {
            return Ok(());
        };
        if let Some((target_version, write_sets)) =
            bundle.store.catch_up(&self.ledger_db, snapshot_version)?
        {
            extend_tip(bundle, target_version, &write_sets)?;
        }
        Ok(())
    }

    /// Readonly counterpart of `catch_up_position_store`: replays the tail
    /// past the snapshot into the overlay only, writing nothing. Main state
    /// needs no such step readonly because its reads go to the DB by version;
    /// the position index is resident, so without it the tail is missing.
    fn catch_up_readonly_positions(&self, snapshot_version: Option<Version>) -> Result<()> {
        let Some(bundle) = self.position.as_ref() else {
            return Ok(());
        };
        let Some(v_overall) = self.ledger_db.metadata_db().get_synced_version()? else {
            return Ok(());
        };
        let snapshot_next_version = snapshot_version.map_or(0, |v| v + 1);
        if get_native_commit_progress(&bundle.store.kv_db)?.is_none()
            || snapshot_next_version > v_overall
        {
            return Ok(());
        }
        let write_sets = NativeStoreBundle::<PositionStore>::write_sets_after_snapshot(
            &self.ledger_db,
            snapshot_next_version,
            v_overall + 1,
        )?;
        extend_tip(bundle, v_overall, &write_sets)
    }

    /// Bring the position store up to the ledger after a restore saved
    /// transactions past its snapshot without executing them — what
    /// `StateStore::reload` does for main state.
    pub(crate) fn reset_position_store_to_ledger(&self) -> Result<()> {
        let Some(bundle) = self.position.as_ref() else {
            return Ok(());
        };
        let v_overall = self.ledger_db.metadata_db().get_synced_version()?;
        let snapshot_version = bundle
            .store
            .merkle_db
            .latest_snapshot_version_at_or_before(v_overall.unwrap_or(Version::MAX))?;
        // With no snapshot the store is still as opened: empty, nothing
        // restored into it.
        if let Some(version) = snapshot_version {
            let root_hash = bundle.store.merkle_db.get_root_hash(version)?;
            self.reset_position_after_fast_sync(version, root_hash)?;
        }
        self.catch_up_position_store(snapshot_version)?;
        self.ensure_position_root_matches_ledger()
    }

    /// Why a position-summary read cannot be served: storage is off, or
    /// the DB was opened readonly, which keeps no position pipeline.
    pub(crate) fn position_storage_unavailable(&self) -> AptosDbError {
        let reason = if self.position.is_some() {
            "the DB was opened readonly"
        } else {
            "storage.rocksdb_configs.enable_trading_native is off"
        };
        AptosDbError::Other(format!(
            "COMPUTE_TRADING_NATIVE_STATE_ROOTS is enabled but native-position storage is not \
             available: {reason}"
        ))
    }

    /// Refuse to open when the ledger tip commits a position root the
    /// rebuilt store disagrees with — e.g. position storage enabled on a
    /// node whose history already carries positions it never stored. Left
    /// undetected, the node would diverge at its next block.
    fn ensure_position_root_matches_ledger(&self) -> Result<()> {
        let Some(store) = self
            .position
            .as_ref()
            .and_then(|bundle| bundle.store.state_store.as_ref())
        else {
            return Ok(());
        };
        let Some(v_overall) = self.ledger_db.metadata_db().get_synced_version()? else {
            return Ok(());
        };
        let Some(committed_root) = self
            .ledger_db
            .transaction_info_db()
            .get_transaction_info(v_overall)?
            .position_state_checkpoint_hash()
        else {
            return Ok(());
        };
        let latest = store.current_state().lock().latest().clone();
        let local_root = latest.root_hash();
        ensure!(
            local_root == committed_root,
            "Native-position store (version {:?}, root {local_root:?}) disagrees with the position \
             root {committed_root:?} committed at ledger version {v_overall}. Position storage was \
             likely enabled on a node whose history already carries positions; restore it from a \
             state snapshot (fast sync or db-tool restore) instead.",
            latest.version(),
        );
        Ok(())
    }

    /// Rebuild the position subsystem against the snapshot a fast sync just
    /// restored at `version`: the store, then the resident index and the
    /// overlay over it.
    pub(crate) fn reset_position_after_fast_sync(
        &self,
        version: Version,
        expected_root_hash: HashValue,
    ) -> Result<()> {
        let Some(bundle) = self.position.as_ref() else {
            return Ok(());
        };
        bundle.store.reset_after_fast_sync(
            &self.ledger_db,
            version,
            expected_root_hash,
            |rows| {
                // Held across the rebuild: `reset_from_rows` starts a new
                // layer family, so until the re-seat the published overlay
                // relates to nothing in the base. Tip then base, the order
                // `with_view` takes.
                let mut tip = bundle.positions.lock();
                bundle.position_base.reset_from_rows(
                    Some(version),
                    POSITION_INDEX_FAMILY,
                    rows.into_iter().flatten(),
                )?;
                *tip = bundle.position_base.new_overlay();
                Ok(())
            },
        )?;

        info!(
            version = version,
            n_accounts = bundle.position_base.read(|view| view.num_accounts()),
            "Position subsystem reset after fast sync."
        );
        Ok(())
    }
}

/// Extends the published overlay with the position writes in `write_sets`,
/// which end at `target_version`.
fn extend_tip(
    bundle: &PositionBundle,
    target_version: Version,
    write_sets: &[WriteSet],
) -> Result<()> {
    let ops: Vec<_> = write_sets
        .iter()
        .flat_map(PositionStore::write_set_ops)
        .map(|(key, op)| (key, op.as_state_value_opt()))
        .collect();
    let writes: PositionWrites = decode_position_writes(ops)?;
    if !writes.is_empty() {
        let mut positions = bundle.positions.lock();
        *positions = positions.extend(&bundle.position_base.layers(), target_version, writes);
    }
    Ok(())
}
