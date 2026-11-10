// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    db::AptosDB,
    ledger_db::LedgerDb,
    native_store::{
        bundle::{NativeStoreBundle, NativeStoreConfigs},
        NativeStoreSpec, OrderStore,
    },
    utils::truncation_helper::get_native_commit_progress,
};
use aptos_config::config::StorageDirPaths;
use aptos_infallible::Mutex;
use aptos_logger::info;
use aptos_schemadb::{Cache, Env};
use aptos_storage_interface::{
    db_ensure as ensure,
    state_store::orders::{decode_order_writes, OrderBase, OrderOverlay},
    AptosDbError, Result,
};
use aptos_types::{transaction::Version, write_set::WriteSet};
use std::sync::Arc;

/// Metric label of the order index's layers, apart from the order JMT
/// pipeline's (`OrderStore::NAME`).
const ORDER_INDEX_FAMILY: &str = "order_index";

pub struct OrderBundle {
    pub(crate) store: NativeStoreBundle<OrderStore>,
    /// Complete market-grouped snapshot at the last folded version.
    /// Advanced by `advance_order_base` under the executor's execution
    /// lock, as the position base is.
    pub(crate) order_base: Arc<OrderBase>,
    /// Tip overlay over `order_base`, extended once the order store's KV
    /// commit succeeds. In readonly mode it stays at cold-load.
    pub(crate) orders: Arc<Mutex<OrderOverlay>>,
}

impl AptosDB {
    pub fn order(&self) -> Option<&Arc<OrderBundle>> {
        self.order.as_ref()
    }

    /// Called from `open_internal` beside `init_native_position` when
    /// `rocksdb_configs.enable_trading_native` is set.
    ///
    /// `for_restore` opens without catching up to the ledger; the restore
    /// brings the store up once it has saved its transactions
    /// (`reset_order_store_to_ledger`).
    ///
    /// Orders have no snapshot kind for state sync yet, so fast sync and
    /// restore leave the order store as opened, and catch-up replays only
    /// past a store that has committed before. A node whose store disagrees
    /// with the order root the ledger tip commits refuses to open rather than
    /// diverge at its next block. Nothing catches a ledger that carries orders
    /// but no order root yet, so enable the storage before `NATIVE_ORDERBOOK`.
    pub(crate) fn init_native_order(
        &mut self,
        db_paths: &StorageDirPaths,
        configs: &NativeStoreConfigs,
        env: &Env,
        block_cache: &Cache,
        readonly: bool,
        for_restore: bool,
    ) -> Result<()> {
        if self.order.is_some() {
            return Err(AptosDbError::Other(
                "init_native_order called twice; the order store is already attached to this \
                 AptosDB"
                    .to_string(),
            ));
        }
        let (store, snapshot_version) = NativeStoreBundle::<OrderStore>::open(
            db_paths,
            configs,
            env,
            block_cache,
            &self.ledger_db,
            readonly,
        )?;

        // Cold-load: stream the durable JMT snapshot into `OrderBase`; the
        // catch-up below closes the gap past it.
        let order_base = match snapshot_version {
            Some(version) => {
                let base = OrderBase::new_from_rows(
                    Some(version),
                    ORDER_INDEX_FAMILY,
                    store.snapshot_rows(version)?,
                )?;
                info!(
                    snapshot_version = version,
                    n_orders = base.read(|view| view.num_orders()),
                    "Native-order cold-load complete."
                );
                base
            },
            None => OrderBase::new_empty(ORDER_INDEX_FAMILY),
        };
        let order_base = Arc::new(order_base);
        let orders = Arc::new(Mutex::new(order_base.new_overlay()));
        let bundle = OrderBundle {
            store,
            order_base,
            orders,
        };

        if readonly {
            catch_up_readonly(&bundle, &self.ledger_db, snapshot_version)?;
        } else if !for_restore {
            if let Some((target_version, write_sets)) =
                bundle.store.catch_up(&self.ledger_db, snapshot_version)?
            {
                extend_tip(&bundle, target_version, &write_sets)?;
            }
            ensure_order_root_matches_ledger(&bundle, &self.ledger_db)?;
        }
        self.order = Some(Arc::new(bundle));
        info!(
            snapshot_version = ?snapshot_version,
            readonly = readonly,
            "Native-order store initialized."
        );
        Ok(())
    }
}

impl AptosDB {
    /// Bring the order store up to the ledger after a restore saved
    /// transactions past its snapshot without executing them.
    pub(crate) fn reset_order_store_to_ledger(&self) -> Result<()> {
        let Some(bundle) = self.order.as_ref() else {
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
        ensure_order_root_matches_ledger(bundle, &self.ledger_db)
    }

    /// Why an order-summary read cannot be served: storage is off, or the DB
    /// was opened readonly, which keeps no order pipeline.
    pub(crate) fn order_storage_unavailable(&self) -> AptosDbError {
        let reason = if self.order.is_some() {
            "the DB was opened readonly"
        } else {
            "storage.rocksdb_configs.enable_trading_native is off"
        };
        AptosDbError::Other(format!(
            "COMPUTE_TRADING_NATIVE_STATE_ROOTS is enabled but native-order storage is not \
             available: {reason}"
        ))
    }
}

/// Refuse to open when the ledger tip commits an order root the store
/// disagrees with, as `ensure_position_root_matches_ledger` does for
/// positions.
fn ensure_order_root_matches_ledger(bundle: &OrderBundle, ledger_db: &LedgerDb) -> Result<()> {
    let Some(store) = bundle.store.state_store.as_ref() else {
        return Ok(());
    };
    let Some(v_overall) = ledger_db.metadata_db().get_synced_version()? else {
        return Ok(());
    };
    let Some(committed_root) = ledger_db
        .transaction_info_db()
        .get_transaction_info(v_overall)?
        .order_state_checkpoint_hash()
    else {
        return Ok(());
    };
    let latest = store.current_state().lock().last_checkpoint().clone();
    let local_root = latest.root_hash();
    ensure!(
        local_root == committed_root,
        "Native-order store (version {:?}, root {local_root:?}) disagrees with the order root \
         {committed_root:?} committed at ledger version {v_overall}. Order storage was likely \
         enabled on a node whose history already carries orders, or the node was restored \
         without them.",
        latest.version(),
    );
    Ok(())
}

/// Readonly counterpart of the catch-up: replays the tail past the snapshot
/// into the overlay only, writing nothing.
fn catch_up_readonly(
    bundle: &OrderBundle,
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
    let write_sets = NativeStoreBundle::<OrderStore>::write_sets_after_snapshot(
        ledger_db,
        snapshot_next_version,
        v_overall + 1,
    )?;
    extend_tip(bundle, v_overall, &write_sets)
}

/// Extends the published overlay with the order writes in `write_sets`, which
/// end at `target_version`.
pub(crate) fn extend_tip(
    bundle: &OrderBundle,
    target_version: Version,
    write_sets: &[WriteSet],
) -> Result<()> {
    let writes = decode_order_writes(
        write_sets
            .iter()
            .flat_map(OrderStore::write_set_ops)
            .map(|(key, op)| (key, op.as_state_value_opt())),
    )?;
    if !writes.is_empty() {
        let mut orders = bundle.orders.lock();
        *orders = orders.extend(&bundle.order_base.layers(), target_version, writes);
    }
    Ok(())
}
