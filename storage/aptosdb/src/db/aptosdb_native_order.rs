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
    /// Orders have no state root and no snapshot kind for state sync yet,
    /// so fast sync and restore leave the order store as opened: a node
    /// restored that way holds only the orders written after its target.
    ///
    /// For the same reason nothing catches a first enable on a ledger that
    /// already carries order writes: catch-up replays only past a store that
    /// has committed before, so the store opens empty. Enable the storage
    /// before `NATIVE_ORDERBOOK`.
    ///
    /// TODO[native_order](root): once the order root exists, refuse to open
    /// when the store disagrees with the committed root, as
    /// `ensure_position_root_matches_ledger` does for positions.
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
