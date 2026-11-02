// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Native order index: a complete [`OrderBase`] with a speculative
//! [`OrderOverlay`] over it, both sharded on `(exchange, market)` so a
//! market's orders never split across shards.

#![forbid(unsafe_code)]

use crate::state_store::native_overlay::{
    NativeOverlay, NativeParent, NativeWrites, ShardedKey, ShardedNativeLayers,
    NUM_NATIVE_INDEX_SHARDS,
};
use ahash::AHashMap;
use aptos_experimental_layered_map::MapLayer;
use aptos_infallible::RwLock;
use aptos_types::{
    account_address::AccountAddress,
    state_store::{
        native_order::{NativeBulkOrder, NativeSingleOrder},
        state_key::{
            inner::{StateKeyInner, TradingNativeKey},
            StateKey,
        },
        state_value::StateValue,
    },
    transaction::Version,
};
use arr_macro::arr;
use std::collections::BTreeMap;

pub const NUM_ORDER_SHARDS: usize = NUM_NATIVE_INDEX_SHARDS;

/// The `(exchange, market)` a book's orders group under.
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd, Debug)]
pub struct MarketKey {
    pub exchange: AccountAddress,
    pub market: AccountAddress,
}

/// One order, keyed as the write set keys it.
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd, Debug)]
pub enum OrderKey {
    Single {
        exchange: AccountAddress,
        market: AccountAddress,
        order_id: u128,
    },
    /// An account holds at most one bulk order per market.
    Bulk {
        exchange: AccountAddress,
        market: AccountAddress,
        account: AccountAddress,
    },
}

impl OrderKey {
    pub fn market(&self) -> MarketKey {
        match *self {
            OrderKey::Single {
                exchange, market, ..
            }
            | OrderKey::Bulk {
                exchange, market, ..
            } => MarketKey { exchange, market },
        }
    }
}

/// Shard on the market, which is the base's outer key, so a market's
/// orders never split across shards and grouping needs no coordination
/// between them.
pub fn shard_of(market: &MarketKey) -> usize {
    let last = AccountAddress::LENGTH - 1;
    let byte = market.market.as_ref()[last] ^ market.exchange.as_ref()[last];
    usize::from(byte) % NUM_ORDER_SHARDS
}

impl ShardedKey for OrderKey {
    fn shard(&self) -> usize {
        shard_of(&self.market())
    }
}

/// An order value. Its kind always matches its key's.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NativeOrder {
    Single(NativeSingleOrder),
    Bulk(NativeBulkOrder),
}

pub type ShardedOrderLayers = ShardedNativeLayers<OrderKey, NativeOrder>;

/// A block's order writes. `None` deletes.
pub type OrderWrites = NativeWrites<OrderKey, NativeOrder>;

/// Speculative overlay: one layer per shard per chunk, chained from the
/// parent's.
pub type OrderOverlay = NativeOverlay<OrderKey, NativeOrder>;

pub type OrderParent<'a> = NativeParent<'a, OrderKey, NativeOrder>;

/// A market's resting orders.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct MarketOrders {
    pub singles: BTreeMap<u128, NativeSingleOrder>,
    /// By account.
    pub bulks: BTreeMap<AccountAddress, NativeBulkOrder>,
}

impl MarketOrders {
    pub fn is_empty(&self) -> bool {
        self.singles.is_empty() && self.bulks.is_empty()
    }

    pub fn len(&self) -> usize {
        self.singles.len() + self.bulks.len()
    }

    pub fn get(&self, key: &OrderKey) -> Option<NativeOrder> {
        match key {
            OrderKey::Single { order_id, .. } => {
                self.singles.get(order_id).cloned().map(NativeOrder::Single)
            },
            OrderKey::Bulk { account, .. } => {
                self.bulks.get(account).cloned().map(NativeOrder::Bulk)
            },
        }
    }

    /// Applies one write blind; the caller has checked the key and value
    /// kinds agree.
    fn apply(&mut self, key: &OrderKey, write: Option<NativeOrder>) {
        match (key, write) {
            (OrderKey::Single { order_id, .. }, Some(NativeOrder::Single(order))) => {
                self.singles.insert(*order_id, order);
            },
            (OrderKey::Bulk { account, .. }, Some(NativeOrder::Bulk(order))) => {
                self.bulks.insert(*account, order);
            },
            (OrderKey::Single { order_id, .. }, None) => {
                self.singles.remove(order_id);
            },
            (OrderKey::Bulk { account, .. }, None) => {
                self.bulks.remove(account);
            },
            (key, Some(order)) => {
                unreachable!("order kind does not match key {key:?}: {order:?}")
            },
        }
    }
}

struct OrderBaseInner {
    version: Option<Version>,
    shards: [BTreeMap<MarketKey, MarketOrders>; NUM_ORDER_SHARDS],
    /// Layer handles at `version` — the view base handed to new blocks.
    layers: ShardedOrderLayers,
}

impl std::fmt::Debug for OrderBaseInner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let len: usize = self.shards.iter().map(|s| s.len()).sum();
        write!(
            f,
            "OrderBaseInner(version={:?}, {len} markets)",
            self.version
        )
    }
}

/// Resolves overlay misses. Resident and complete, because the durable
/// store is hash-keyed and cannot enumerate a market's orders.
///
/// Single-version, so it must not move while a block executes: the
/// executor advances it under its execution lock.
#[derive(Debug)]
pub struct OrderBase {
    inner: RwLock<OrderBaseInner>,
}

/// Guarded view of an [`OrderBase`], scope-bound to a closure so no caller
/// can park a lock and stall folds.
pub struct OrderBaseView<'a> {
    inner: &'a OrderBaseInner,
}

impl OrderBaseView<'_> {
    pub fn version(&self) -> Option<Version> {
        self.inner.version
    }

    /// `None` when the market holds no orders — the fold prunes a market
    /// whose last order is deleted.
    pub fn market_orders(&self, market: &MarketKey) -> Option<&MarketOrders> {
        self.inner.shards[shard_of(market)].get(market)
    }

    pub fn get(&self, key: &OrderKey) -> Option<NativeOrder> {
        self.market_orders(&key.market())
            .and_then(|orders| orders.get(key))
    }

    pub fn shard(&self, shard: usize) -> &BTreeMap<MarketKey, MarketOrders> {
        &self.inner.shards[shard]
    }

    pub fn layers(&self) -> &ShardedOrderLayers {
        &self.inner.layers
    }

    pub fn num_markets(&self) -> usize {
        self.inner.shards.iter().map(|s| s.len()).sum()
    }

    pub fn num_orders(&self) -> usize {
        self.inner
            .shards
            .iter()
            .map(|s| s.values().map(MarketOrders::len).sum::<usize>())
            .sum()
    }
}

impl OrderBase {
    pub fn new_empty(family: &'static str) -> Self {
        Self::new_at_version(None, family, Vec::new())
    }

    pub fn new_at_version(
        version: Option<Version>,
        family: &'static str,
        seed: impl IntoIterator<Item = (OrderKey, NativeOrder)>,
    ) -> Self {
        Self {
            inner: RwLock::new(
                Self::build(
                    version,
                    family,
                    seed.into_iter().map(Ok::<_, crate::AptosDbError>),
                )
                .expect("Infallible rows."),
            ),
        }
    }

    /// Cold-load from undecoded durable rows, so the decoded set is never
    /// materialized alongside the base it feeds.
    pub fn new_from_rows<I>(
        version: Option<Version>,
        family: &'static str,
        rows: I,
    ) -> crate::Result<Self>
    where
        I: IntoIterator<Item = crate::Result<(StateKey, StateValue)>>,
    {
        Ok(Self {
            inner: RwLock::new(Self::build(
                version,
                family,
                rows.into_iter().map(decode_order_row),
            )?),
        })
    }

    /// Rebuild in place from durable rows. Starts a new layer family, so
    /// overlays on the old base stop descending from it and must be
    /// re-seated.
    pub fn reset_from_rows<I>(
        &self,
        version: Option<Version>,
        family: &'static str,
        rows: I,
    ) -> crate::Result<()>
    where
        I: IntoIterator<Item = crate::Result<(StateKey, StateValue)>>,
    {
        let rebuilt = Self::build(version, family, rows.into_iter().map(decode_order_row))?;
        *self.inner.write() = rebuilt;
        Ok(())
    }

    fn build<E>(
        version: Option<Version>,
        family: &'static str,
        rows: impl Iterator<Item = std::result::Result<(OrderKey, NativeOrder), E>>,
    ) -> std::result::Result<OrderBaseInner, E> {
        let mut shards: [BTreeMap<MarketKey, MarketOrders>; NUM_ORDER_SHARDS] =
            arr![BTreeMap::new(); 16];
        for row in rows {
            let (key, order) = row?;
            let market = key.market();
            shards[shard_of(&market)]
                .entry(market)
                .or_default()
                .apply(&key, Some(order));
        }
        Ok(OrderBaseInner {
            version,
            shards,
            layers: arr![MapLayer::new_family(family); 16],
        })
    }

    pub fn version(&self) -> Option<Version> {
        self.inner.read().version
    }

    /// Layer handles at the current base version. New overlays extend from
    /// these, which is what lets layers below them be freed once the blocks
    /// holding older handles are pruned.
    pub fn layers(&self) -> ShardedOrderLayers {
        self.inner.read().layers.clone()
    }

    pub fn read<R>(&self, f: impl FnOnce(&OrderBaseView<'_>) -> R) -> R {
        let guard = self.inner.read();
        f(&OrderBaseView { inner: &guard })
    }

    /// Empty overlay sitting directly on the base's current layers.
    pub fn new_overlay(&self) -> OrderOverlay {
        let inner = self.inner.read();
        OrderOverlay::new(inner.version.map_or(0, |v| v + 1), inner.layers.clone())
    }

    /// The guard [`Self::fold`] and [`Self::with_view`] require. Walks the
    /// ancestry rather than using `can_view_after`, which an abandoned
    /// sibling fork also satisfies while sharing no ancestry.
    pub fn is_ancestor_of(&self, overlay: &OrderOverlay) -> bool {
        overlay.descends_from(&self.inner.read().layers)
    }

    /// Runs `f` against `overlay` merged over this base.
    ///
    /// Holds the read lock for the scope, so a fold cannot land mid-view and
    /// leave the two halves at different versions. The base must not have
    /// moved past `overlay`.
    pub fn with_view<R>(&self, overlay: &OrderOverlay, f: impl FnOnce(&OrderView<'_>) -> R) -> R {
        self.read(|base_view| {
            // Under the lock, so the layers checked are the ones merged
            // against; checking before locking lets a fold land in between.
            assert!(
                overlay.descends_from(base_view.layers()),
                "order overlay does not descend from the base it would be merged over",
            );
            let mut by_market: OverlayByMarket = AHashMap::new();
            for (key, write) in overlay.writes_since(base_view.layers()) {
                by_market
                    .entry(key.market())
                    .or_default()
                    .push((key, write));
            }
            f(&OrderView {
                base: base_view,
                by_market,
            })
        })
    }

    /// Applies everything written since the base layers. Returns whether
    /// the base moved; declines an overlay at or behind the base, or one not
    /// descended from it, re-checked here under the write lock.
    pub fn fold(&self, overlay: &OrderOverlay) -> bool {
        let mut inner = self.inner.write();
        if inner.version.is_some() && overlay.next_version() <= inner.version.map_or(0, |v| v + 1) {
            return false;
        }
        if !overlay.descends_from(&inner.layers) {
            return false;
        }
        for shard in 0..NUM_ORDER_SHARDS {
            let view = overlay.shards()[shard].view_layers_after(&inner.layers[shard]);
            for (key, write) in view.iter() {
                let market = key.market();
                let markets = &mut inner.shards[shard];
                match write {
                    Some(order) => markets.entry(market).or_default().apply(&key, Some(order)),
                    None => {
                        // Drop the market with its last order, or it
                        // lingers empty and reads as a live book.
                        if let Some(orders) = markets.get_mut(&market) {
                            orders.apply(&key, None);
                            if orders.is_empty() {
                                markets.remove(&market);
                            }
                        }
                    },
                }
            }
        }
        inner.layers = overlay.shards().clone();
        inner.version = overlay.version();
        true
    }
}

/// The overlay's writes grouped by market. Built once per view, affordable
/// because the overlay holds only writes since the last fold.
type OverlayByMarket = AHashMap<MarketKey, Vec<(OrderKey, Option<NativeOrder>)>>;

/// An [`OrderOverlay`] merged over the base it sits on, valid for the scope
/// of [`OrderBase::with_view`].
pub struct OrderView<'a> {
    base: &'a OrderBaseView<'a>,
    by_market: OverlayByMarket,
}

impl OrderView<'_> {
    /// The version the base is folded to.
    pub fn base_version(&self) -> Option<Version> {
        self.base.version()
    }

    /// Overlay first, then the base. `None` for a tombstone or a miss.
    pub fn get(&self, key: &OrderKey) -> Option<NativeOrder> {
        match self.by_market.get(&key.market()).and_then(|writes| {
            writes
                .iter()
                .find(|(k, _)| k == key)
                .map(|(_, write)| write.clone())
        }) {
            Some(write) => write,
            None => self.base.get(key),
        }
    }

    /// The market's orders, base merged with the overlay's writes.
    pub fn market_orders(&self, market: &MarketKey) -> MarketOrders {
        let mut orders = self.base.market_orders(market).cloned().unwrap_or_default();
        if let Some(writes) = self.by_market.get(market) {
            for (key, write) in writes {
                orders.apply(key, write.clone());
            }
        }
        orders
    }
}

/// Decodes a block's native-order write set into the form both the overlay
/// and the durable commit take.
pub fn decode_order_writes<'a, I>(writes: I) -> crate::Result<OrderWrites>
where
    I: IntoIterator<Item = (&'a StateKey, Option<&'a StateValue>)>,
{
    writes
        .into_iter()
        .map(|(state_key, value)| {
            let key = order_key_of(state_key)?;
            let order = value.map(|sv| decode_order(&key, sv.bytes())).transpose()?;
            Ok((key, order))
        })
        .collect()
}

pub fn order_key_of(state_key: &StateKey) -> crate::Result<OrderKey> {
    match state_key.inner() {
        StateKeyInner::TradingNative(TradingNativeKey::Order {
            exchange,
            market,
            order_id,
        }) => Ok(OrderKey::Single {
            exchange: *exchange,
            market: *market,
            order_id: *order_id,
        }),
        StateKeyInner::TradingNative(TradingNativeKey::BulkOrder {
            exchange,
            market,
            account,
        }) => Ok(OrderKey::Bulk {
            exchange: *exchange,
            market: *market,
            account: *account,
        }),
        other => Err(crate::AptosDbError::Other(format!(
            "non-order native StateKey: {other:?}"
        ))),
    }
}

/// Decodes `bytes` as the order kind `key` names.
fn decode_order(key: &OrderKey, bytes: &[u8]) -> crate::Result<NativeOrder> {
    let decoded = match key {
        OrderKey::Single { .. } => NativeSingleOrder::deserialize(bytes).map(NativeOrder::Single),
        OrderKey::Bulk { .. } => NativeBulkOrder::deserialize(bytes).map(NativeOrder::Bulk),
    };
    decoded.map_err(|e| {
        crate::AptosDbError::Other(format!("native order at {key:?} failed to decode: {e}"))
    })
}

/// Decodes one durable JMT row into an [`OrderBase`] entry.
fn decode_order_row(
    row: crate::Result<(StateKey, StateValue)>,
) -> crate::Result<(OrderKey, NativeOrder)> {
    let (state_key, state_value) = row?;
    let key = order_key_of(&state_key)?;
    let order = decode_order(&key, state_value.bytes())?;
    Ok((key, order))
}

#[cfg(test)]
mod tests {
    use super::*;
    use aptos_types::state_store::native_order::{BulkOrderV1, SingleOrderV1, TimeInForce};

    fn addr(byte: u8) -> AccountAddress {
        let mut a = [0u8; AccountAddress::LENGTH];
        a[AccountAddress::LENGTH - 1] = byte;
        AccountAddress::new(a)
    }

    fn single(order_id: u128, price: u64) -> NativeOrder {
        NativeOrder::Single(NativeSingleOrder::V1(SingleOrderV1 {
            account: addr(9),
            order_id,
            client_order_id: None,
            price,
            orig_size: 10,
            remaining_size: 10,
            is_bid: true,
            trigger_condition: None,
            time_in_force: TimeInForce::Gtc,
            creation_time_micros: 0,
            unique_priority_idx: order_id,
            metadata: vec![].into(),
        }))
    }

    fn bulk(account: u8, price: u64) -> NativeOrder {
        NativeOrder::Bulk(NativeBulkOrder::V1(BulkOrderV1 {
            account: addr(account),
            order_id: 1,
            sequence_number: 0,
            bid_prices: vec![price],
            bid_sizes: vec![1],
            ask_prices: vec![],
            ask_sizes: vec![],
            unique_priority_idx: 1,
            creation_time_micros: 0,
            metadata: vec![].into(),
        }))
    }

    fn single_key(market: u8, order_id: u128) -> OrderKey {
        OrderKey::Single {
            exchange: addr(1),
            market: addr(market),
            order_id,
        }
    }

    fn bulk_key(market: u8, account: u8) -> OrderKey {
        OrderKey::Bulk {
            exchange: addr(1),
            market: addr(market),
            account: addr(account),
        }
    }

    fn market(market: u8) -> MarketKey {
        MarketKey {
            exchange: addr(1),
            market: addr(market),
        }
    }

    fn base_with(seed: Vec<(OrderKey, NativeOrder)>) -> OrderBase {
        OrderBase::new_at_version(Some(0), "test", seed)
    }

    fn read(base: &OrderBase, overlay: &OrderOverlay, key: &OrderKey) -> Option<NativeOrder> {
        base.with_view(overlay, |view| view.get(key))
    }

    fn row(key: OrderKey, order: &NativeOrder) -> crate::Result<(StateKey, StateValue)> {
        let (state_key, bytes) = match (key, order) {
            (
                OrderKey::Single {
                    exchange,
                    market,
                    order_id,
                },
                NativeOrder::Single(order),
            ) => (
                StateKey::order(exchange, market, order_id),
                order.serialize().unwrap(),
            ),
            (
                OrderKey::Bulk {
                    exchange,
                    market,
                    account,
                },
                NativeOrder::Bulk(order),
            ) => (
                StateKey::bulk_order(exchange, market, account),
                order.serialize().unwrap(),
            ),
            (key, order) => panic!("mismatched {key:?} {order:?}"),
        };
        Ok((state_key, StateValue::new_legacy(bytes.into())))
    }

    #[test]
    fn reads_fall_through_to_base() {
        let base = base_with(vec![
            (single_key(7, 1), single(1, 100)),
            (bulk_key(7, 3), bulk(3, 99)),
        ]);
        let overlay = base.new_overlay();
        assert_eq!(
            read(&base, &overlay, &single_key(7, 1)),
            Some(single(1, 100))
        );
        assert_eq!(read(&base, &overlay, &bulk_key(7, 3)), Some(bulk(3, 99)));
        assert_eq!(read(&base, &overlay, &single_key(7, 2)), None);
        assert_eq!(overlay.next_version(), 1);
    }

    #[test]
    fn overlay_shadows_base_and_tombstones_delete() {
        let base = base_with(vec![
            (single_key(7, 1), single(1, 100)),
            (bulk_key(7, 3), bulk(3, 99)),
        ]);
        let overlay = base.new_overlay().extend(&base.layers(), 1, vec![
            (single_key(7, 1), Some(single(1, 101))),
            (bulk_key(7, 3), None),
        ]);
        assert_eq!(
            read(&base, &overlay, &single_key(7, 1)),
            Some(single(1, 101))
        );
        assert_eq!(read(&base, &overlay, &bulk_key(7, 3)), None);
    }

    #[test]
    fn market_orders_merges_the_overlay_over_the_base() {
        let base = base_with(vec![
            (single_key(7, 1), single(1, 100)),
            (single_key(7, 2), single(2, 101)),
            (single_key(8, 1), single(1, 500)),
        ]);
        let overlay = base.new_overlay().extend(&base.layers(), 1, vec![
            (single_key(7, 2), None),
            (single_key(7, 3), Some(single(3, 102))),
            (bulk_key(7, 4), Some(bulk(4, 98))),
        ]);
        let orders = base.with_view(&overlay, |view| view.market_orders(&market(7)));
        assert_eq!(orders.singles.keys().copied().collect::<Vec<_>>(), vec![
            1, 3
        ]);
        assert_eq!(orders.bulks.keys().copied().collect::<Vec<_>>(), vec![
            addr(4)
        ]);
        let other = base.with_view(&overlay, |view| view.market_orders(&market(8)));
        assert_eq!(other.len(), 1, "other markets are untouched");
    }

    #[test]
    fn fold_applies_writes_and_prunes_an_emptied_market() {
        let base = base_with(vec![
            (single_key(7, 1), single(1, 100)),
            (single_key(8, 1), single(1, 500)),
        ]);
        let overlay = base.new_overlay().extend(&base.layers(), 1, vec![
            (single_key(7, 1), None),
            (bulk_key(8, 3), Some(bulk(3, 499))),
        ]);
        assert!(base.fold(&overlay));
        assert_eq!(base.version(), Some(1));
        base.read(|view| {
            assert!(
                view.market_orders(&market(7)).is_none(),
                "empty market pruned"
            );
            assert_eq!(
                view.market_orders(&market(8)).map(MarketOrders::len),
                Some(2)
            );
            assert_eq!(view.num_markets(), 1);
            assert_eq!(view.num_orders(), 2);
        });
        let fresh = base.new_overlay();
        assert!(fresh.writes_since(&base.layers()).is_empty());
        assert_eq!(read(&base, &fresh, &bulk_key(8, 3)), Some(bulk(3, 499)));
    }

    #[test]
    fn fold_declines_a_sibling_fork() {
        let base = base_with(Vec::new());
        let floor = base.layers();
        let parent = base
            .new_overlay()
            .extend(&floor, 1, vec![(single_key(7, 1), Some(single(1, 1)))]);
        let certified = parent.extend(&floor, 2, vec![(single_key(7, 2), Some(single(2, 2)))]);
        let abandoned = parent.extend(&floor, 2, vec![(single_key(7, 3), Some(single(3, 3)))]);
        assert!(base.fold(&certified));
        assert!(!base.is_ancestor_of(&abandoned));
        assert!(!base.fold(&abandoned));
    }

    /// Single and bulk orders of one market share a shard, whatever the order
    /// id or account, so a market's book never splits.
    #[test]
    fn a_market_never_splits_across_shards() {
        let shard = market(7);
        for id in 0..100u128 {
            assert_eq!(single_key(7, id).shard(), shard_of(&shard));
        }
        for account in 0..100u8 {
            assert_eq!(bulk_key(7, account).shard(), shard_of(&shard));
        }
    }

    #[test]
    fn rows_round_trip_through_cold_load_and_reset() {
        let seed = [
            (single_key(7, 1), single(1, 100)),
            (bulk_key(7, 3), bulk(3, 99)),
        ];
        let base = OrderBase::new_from_rows(
            Some(4),
            "test",
            seed.iter().map(|(key, order)| row(*key, order)),
        )
        .unwrap();
        base.read(|view| {
            assert_eq!(view.version(), Some(4));
            assert_eq!(view.get(&single_key(7, 1)), Some(single(1, 100)));
            assert_eq!(view.get(&bulk_key(7, 3)), Some(bulk(3, 99)));
        });

        let stale = base.new_overlay();
        base.reset_from_rows(Some(9), "test", vec![row(single_key(8, 5), &single(5, 7))])
            .unwrap();
        assert!(!base.is_ancestor_of(&stale), "reset starts a new family");
        base.read(|view| {
            assert_eq!(view.version(), Some(9));
            assert_eq!(view.num_orders(), 1);
            assert_eq!(view.get(&single_key(8, 5)), Some(single(5, 7)));
        });
    }

    #[test]
    fn decode_reads_each_kind_and_rejects_other_keys() {
        let (single_state_key, single_value) = row(single_key(7, 1), &single(1, 100)).unwrap();
        let (bulk_state_key, bulk_value) = row(bulk_key(7, 3), &bulk(3, 99)).unwrap();
        let writes = decode_order_writes([
            (&single_state_key, Some(&single_value)),
            (&bulk_state_key, Some(&bulk_value)),
            (&single_state_key, None),
        ])
        .unwrap();
        assert_eq!(writes, vec![
            (single_key(7, 1), Some(single(1, 100))),
            (bulk_key(7, 3), Some(bulk(3, 99))),
            (single_key(7, 1), None),
        ]);

        let position = StateKey::position(addr(1), addr(2), addr(7));
        assert!(decode_order_writes([(&position, None)]).is_err());
    }
}
