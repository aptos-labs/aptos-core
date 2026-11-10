// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Native order index: a complete [`OrderBase`] with a speculative
//! [`OrderOverlay`] over it, both sharded on `(exchange, market)` so a
//! market's orders never split across shards.

#![forbid(unsafe_code)]

use crate::state_store::{
    native_overlay::{
        NativeOverlay, NativeParent, NativeWrites, ShardedKey, ShardedNativeLayers,
        NUM_NATIVE_INDEX_SHARDS,
    },
    order_books::{BookDelta, BookView, MarketBook},
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
use itertools::Itertools;
use std::{collections::BTreeMap, sync::OnceLock};

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

/// A market's resting orders and the book derived from them. Only
/// [`MarketOrders::apply`] mutates either, which keeps them in step.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct MarketOrders {
    singles: BTreeMap<u128, NativeSingleOrder>,
    /// By account.
    bulks: BTreeMap<AccountAddress, NativeBulkOrder>,
    book: MarketBook,
}

impl MarketOrders {
    pub fn is_empty(&self) -> bool {
        self.singles.is_empty() && self.bulks.is_empty()
    }

    pub fn len(&self) -> usize {
        self.singles.len() + self.bulks.len()
    }

    pub fn singles(&self) -> &BTreeMap<u128, NativeSingleOrder> {
        &self.singles
    }

    pub fn bulks(&self) -> &BTreeMap<AccountAddress, NativeBulkOrder> {
        &self.bulks
    }

    pub fn book(&self) -> &MarketBook {
        &self.book
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
    /// kinds agree. The old value's book entries are retracted before the
    /// new value's go in, since the two can share keys.
    pub(crate) fn apply(&mut self, key: &OrderKey, write: Option<NativeOrder>) {
        match (key, write) {
            (OrderKey::Single { order_id, .. }, write) => {
                if let Some(old) = self.singles.remove(order_id) {
                    self.book.remove_single(&old);
                }
                if let Some(NativeOrder::Single(order)) = write {
                    self.book.insert_single(&order);
                    self.singles.insert(*order_id, order);
                } else if let Some(order) = write {
                    unreachable!("order kind does not match key {key:?}: {order:?}")
                }
            },
            (OrderKey::Bulk { account, .. }, write) => {
                if let Some(old) = self.bulks.remove(account) {
                    self.book.remove_bulk(&old);
                }
                if let Some(NativeOrder::Bulk(order)) = write {
                    self.book.insert_bulk(&order);
                    self.bulks.insert(*account, order);
                } else if let Some(order) = write {
                    unreachable!("order kind does not match key {key:?}: {order:?}")
                }
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
                    .writes
                    .insert(key, write);
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

/// The overlay's writes to one market, and the book delta they make, derived
/// by the first query that needs it and reused for the rest of the view.
#[derive(Default)]
struct MarketWrites {
    writes: AHashMap<OrderKey, Option<NativeOrder>>,
    delta: OnceLock<BookDelta>,
}

impl MarketWrites {
    fn delta(&self, base: Option<&MarketOrders>) -> &BookDelta {
        self.delta.get_or_init(|| {
            let mut delta = BookDelta::default();
            for (key, write) in &self.writes {
                match key {
                    OrderKey::Single { order_id, .. } => {
                        if let Some(old) = base.and_then(|orders| orders.singles().get(order_id)) {
                            delta.retracted.insert_single(old);
                        }
                    },
                    OrderKey::Bulk { account, .. } => {
                        if let Some(old) = base.and_then(|orders| orders.bulks().get(account)) {
                            delta.retracted.insert_bulk(old);
                        }
                    },
                }
                match write {
                    Some(NativeOrder::Single(order)) => delta.inserted.insert_single(order),
                    Some(NativeOrder::Bulk(order)) => delta.inserted.insert_bulk(order),
                    None => {},
                }
            }
            delta
        })
    }
}

/// The overlay's writes grouped by market. Built once per view, affordable
/// because the overlay holds only writes since the last fold.
type OverlayByMarket = AHashMap<MarketKey, MarketWrites>;

/// An [`OrderOverlay`] merged over the base it sits on, valid for the scope
/// of [`OrderBase::with_view`].
pub struct OrderView<'a> {
    base: &'a OrderBaseView<'a>,
    by_market: OverlayByMarket,
}

impl<'v> OrderView<'v> {
    /// The version the base is folded to.
    pub fn base_version(&self) -> Option<Version> {
        self.base.version()
    }

    /// Overlay first, then the base. `None` for a tombstone or a miss.
    pub fn get(&self, key: &OrderKey) -> Option<NativeOrder> {
        match self
            .by_market
            .get(&key.market())
            .and_then(|market| market.writes.get(key))
        {
            Some(write) => write.clone(),
            None => self.base.get(key),
        }
    }

    /// The market's book with the overlay's writes applied, without copying
    /// the base book: the written orders' old values are retracted and their
    /// new values inserted, by the same index rule the fold applies.
    pub fn book(&self, market: &MarketKey) -> BookView<'_> {
        let base = self.base.market_orders(market);
        let delta = match self.by_market.get(market) {
            Some(writes) => writes.delta(base),
            None => BookDelta::empty(),
        };
        BookView::new(base.map(MarketOrders::book), delta)
    }

    /// `(best bid, best ask)`; `None` for an empty side.
    pub fn best_prices(&self, market: &MarketKey) -> (Option<u64>, Option<u64>) {
        self.book(market).best_prices()
    }

    /// The best `max_levels` depth levels on a side, `(price, size)`.
    pub fn depth(&self, market: &MarketKey, is_bid: bool, max_levels: usize) -> Vec<(u64, u128)> {
        self.book(market).depth(is_bid, max_levels)
    }

    /// The market's single orders by order id, ascending: the base's with
    /// the overlay's writes applied, copying neither.
    pub fn singles<'s>(
        &'s self,
        market: &MarketKey,
    ) -> impl Iterator<Item = (u128, &'s NativeSingleOrder)> + use<'s, 'v> {
        let writes = self.by_market.get(market).map(|market| &market.writes);
        let market = *market;
        let mut written: Vec<(u128, &NativeSingleOrder)> = writes
            .into_iter()
            .flatten()
            .filter_map(|(key, write)| match (key, write) {
                (OrderKey::Single { order_id, .. }, Some(NativeOrder::Single(order))) => {
                    Some((*order_id, order))
                },
                (
                    OrderKey::Single { .. } | OrderKey::Bulk { .. },
                    Some(NativeOrder::Single(_) | NativeOrder::Bulk(_)) | None,
                ) => None,
            })
            .collect();
        written.sort_unstable_by_key(|(order_id, _)| *order_id);
        self.base
            .market_orders(&market)
            .into_iter()
            .flat_map(MarketOrders::singles)
            .filter(move |(order_id, _)| {
                let key = OrderKey::Single {
                    exchange: market.exchange,
                    market: market.market,
                    order_id: **order_id,
                };
                !writes.is_some_and(|writes| writes.contains_key(&key))
            })
            .map(|(order_id, order)| (*order_id, order))
            .merge_by(written, |(a, _), (b, _)| a < b)
    }

    /// The market's bulk orders by account, ascending, as [`Self::singles`].
    pub fn bulks<'s>(
        &'s self,
        market: &MarketKey,
    ) -> impl Iterator<Item = (AccountAddress, &'s NativeBulkOrder)> + use<'s, 'v> {
        let writes = self.by_market.get(market).map(|market| &market.writes);
        let market = *market;
        let mut written: Vec<(AccountAddress, &NativeBulkOrder)> = writes
            .into_iter()
            .flatten()
            .filter_map(|(key, write)| match (key, write) {
                (OrderKey::Bulk { account, .. }, Some(NativeOrder::Bulk(order))) => {
                    Some((*account, order))
                },
                (
                    OrderKey::Single { .. } | OrderKey::Bulk { .. },
                    Some(NativeOrder::Single(_) | NativeOrder::Bulk(_)) | None,
                ) => None,
            })
            .collect();
        written.sort_unstable_by_key(|(account, _)| *account);
        self.base
            .market_orders(&market)
            .into_iter()
            .flat_map(MarketOrders::bulks)
            .filter(move |(account, _)| {
                let key = OrderKey::Bulk {
                    exchange: market.exchange,
                    market: market.market,
                    account: **account,
                };
                !writes.is_some_and(|writes| writes.contains_key(&key))
            })
            .map(|(account, order)| (*account, order))
            .merge_by(written, |(a, _), (b, _)| a < b)
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
    fn view_queries_merge_the_overlay_over_the_base() {
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
        base.with_view(&overlay, |view| {
            assert_eq!(view.best_prices(&market(7)), (Some(102), None));
            assert_eq!(view.depth(&market(7), true, 10), vec![
                (102, 10),
                (100, 10),
                (98, 1)
            ]);
            let singles: Vec<_> = view
                .singles(&market(7))
                .map(|(order_id, order)| (order_id, NativeOrder::Single(order.clone())))
                .collect();
            assert_eq!(singles, vec![(1, single(1, 100)), (3, single(3, 102))]);
            let bulks: Vec<_> = view
                .bulks(&market(7))
                .map(|(account, order)| (account, NativeOrder::Bulk(order.clone())))
                .collect();
            assert_eq!(bulks, vec![(addr(4), bulk(4, 98))]);
            assert_eq!(
                view.best_prices(&market(8)),
                (Some(500), None),
                "other markets are untouched"
            );
        });
    }

    /// A written market's book delta is built by the first query that needs
    /// it and reused after; point reads never build one.
    #[test]
    fn book_delta_is_built_once_per_written_market() {
        let base = base_with(vec![
            (single_key(7, 1), single(1, 100)),
            (single_key(8, 1), single(1, 500)),
        ]);
        let overlay = base.new_overlay().extend(&base.layers(), 1, vec![(
            single_key(7, 2),
            Some(single(2, 101)),
        )]);
        base.with_view(&overlay, |view| {
            let built = || {
                view.by_market
                    .get(&market(7))
                    .expect("market 7 written")
                    .delta
                    .get()
                    .map(|delta| delta as *const BookDelta)
            };
            assert_eq!(view.get(&single_key(7, 2)), Some(single(2, 101)));
            assert_eq!(built(), None, "a point read builds no delta");

            assert_eq!(view.best_prices(&market(7)), (Some(101), None));
            let first = built().expect("built by the first book query");
            assert_eq!(view.depth(&market(7), true, 10), vec![(101, 10), (100, 10)]);
            assert_eq!(built(), Some(first), "reused by later queries");

            assert_eq!(view.best_prices(&market(8)), (Some(500), None));
            assert!(view.by_market.get(&market(8)).is_none());
        });
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

#[cfg(test)]
mod book_tests {
    use super::*;
    use crate::state_store::order_books::{BookDelta, BookEntry, BookOrder, BookView};
    use aptos_types::state_store::native_order::{
        BulkOrderV1, SingleOrderV1, TimeInForce, TriggerCondition,
    };
    use proptest::prelude::*;
    use std::cmp::Reverse;

    fn addr(byte: u8) -> AccountAddress {
        let mut a = [0u8; AccountAddress::LENGTH];
        a[AccountAddress::LENGTH - 1] = byte;
        AccountAddress::new(a)
    }

    fn single_key(order_id: u128) -> OrderKey {
        OrderKey::Single {
            exchange: addr(1),
            market: addr(7),
            order_id,
        }
    }

    fn bulk_key(account: u8) -> OrderKey {
        OrderKey::Bulk {
            exchange: addr(1),
            market: addr(7),
            account: addr(account),
        }
    }

    #[derive(Clone, Debug)]
    struct Single {
        order_id: u128,
        account: u8,
        price: u64,
        remaining_size: u64,
        is_bid: bool,
        trigger: Option<TriggerCondition>,
        with_coid: bool,
    }

    impl Single {
        fn order(&self) -> NativeOrder {
            NativeOrder::Single(NativeSingleOrder::V1(SingleOrderV1 {
                account: addr(self.account),
                order_id: self.order_id,
                // Unique per account, as the order book keeps it.
                client_order_id: self.with_coid.then(|| format!("coid-{}", self.order_id)),
                price: self.price,
                orig_size: 100,
                remaining_size: self.remaining_size,
                is_bid: self.is_bid,
                trigger_condition: self.trigger,
                time_in_force: TimeInForce::Gtc,
                creation_time_micros: 0,
                // Unique per order, as `IncreasingIdx` is.
                unique_priority_idx: self.order_id,
                metadata: vec![].into(),
            }))
        }

        fn active(order_id: u128, price: u64, remaining_size: u64, is_bid: bool) -> Self {
            Self {
                order_id,
                account: 2,
                price,
                remaining_size,
                is_bid,
                trigger: None,
                with_coid: false,
            }
        }
    }

    fn bulk(account: u8, bids: &[(u64, u64)], asks: &[(u64, u64)]) -> NativeOrder {
        NativeOrder::Bulk(NativeBulkOrder::V1(BulkOrderV1 {
            account: addr(account),
            order_id: 1000 + u128::from(account),
            sequence_number: 0,
            bid_prices: bids.iter().map(|(price, _)| *price).collect(),
            bid_sizes: bids.iter().map(|(_, size)| *size).collect(),
            ask_prices: asks.iter().map(|(price, _)| *price).collect(),
            ask_sizes: asks.iter().map(|(_, size)| *size).collect(),
            unique_priority_idx: 1000 + u128::from(account),
            creation_time_micros: 0,
            metadata: vec![].into(),
        }))
    }

    fn set_single(orders: &mut MarketOrders, single: &Single) {
        orders.apply(&single_key(single.order_id), Some(single.order()));
    }

    /// Rebuilt from the orders alone, in key order.
    fn rebuilt(orders: &MarketOrders) -> MarketOrders {
        let mut fresh = MarketOrders::default();
        for (order_id, order) in orders.singles() {
            fresh.apply(
                &single_key(*order_id),
                Some(NativeOrder::Single(order.clone())),
            );
        }
        for (account, order) in orders.bulks() {
            let key = OrderKey::Bulk {
                exchange: addr(1),
                market: addr(7),
                account: *account,
            };
            fresh.apply(&key, Some(NativeOrder::Bulk(order.clone())));
        }
        fresh
    }

    #[test]
    fn best_prices_follow_price_then_priority() {
        let mut orders = MarketOrders::default();
        set_single(&mut orders, &Single::active(3, 100, 5, true));
        set_single(&mut orders, &Single::active(1, 100, 5, true));
        set_single(&mut orders, &Single::active(2, 99, 5, true));
        set_single(&mut orders, &Single::active(5, 105, 5, false));
        set_single(&mut orders, &Single::active(4, 105, 5, false));
        set_single(&mut orders, &Single::active(6, 106, 5, false));
        let book = orders.book();
        assert_eq!(
            book.best_bid().map(|(price, entry)| (price, entry.order)),
            Some((100, BookOrder::Single { order_id: 1 }))
        );
        assert_eq!(
            book.best_ask().map(|(price, entry)| (price, entry.order)),
            Some((105, BookOrder::Single { order_id: 4 }))
        );
        assert_eq!(
            book.bid_depth
                .iter()
                .map(|(Reverse(p), s)| (*p, *s))
                .collect::<Vec<_>>(),
            vec![(100, 10), (99, 5)]
        );
    }

    /// A partial fill keeps the order's key, so it keeps its priority; a full
    /// fill or a delete takes every entry with it.
    #[test]
    fn fills_keep_priority_and_deletes_clear_entries() {
        let mut orders = MarketOrders::default();
        let mut order = Single::active(1, 100, 10, true);
        order.with_coid = true;
        set_single(&mut orders, &order);
        order.remaining_size = 4;
        set_single(&mut orders, &order);
        let book = orders.book();
        assert_eq!(
            book.bids.get(&(Reverse(100), 1)),
            Some(&BookEntry {
                order: BookOrder::Single { order_id: 1 },
                size: 4
            })
        );
        assert_eq!(book.bid_depth.get(&Reverse(100)), Some(&4));
        assert_eq!(book.client_order_ids.len(), 1);

        orders.apply(&single_key(1), None);
        assert_eq!(orders.book(), &MarketBook::default());
        assert!(orders.is_empty());
    }

    #[test]
    fn zero_remaining_rests_nowhere_but_stays_indexed_by_account() {
        let mut orders = MarketOrders::default();
        set_single(&mut orders, &Single::active(1, 100, 0, false));
        let book = orders.book();
        assert!(book.asks.is_empty() && book.ask_depth.is_empty());
        assert_eq!(
            book.account_orders.get(&addr(2)).map(|ids| ids.len()),
            Some(1)
        );
    }

    /// A pending order sits in its trigger's index only; once its trigger is
    /// cleared it moves to the price-time index.
    #[test]
    fn triggers_route_to_their_pending_index() {
        let mut orders = MarketOrders::default();
        for (order_id, trigger) in [
            (1, TriggerCondition::PriceMoveAbove(120)),
            (2, TriggerCondition::PriceMoveBelow(80)),
            (3, TriggerCondition::TimeBased(1_000)),
        ] {
            let mut order = Single::active(order_id, 100, 5, true);
            order.trigger = Some(trigger);
            set_single(&mut orders, &order);
        }
        let book = orders.book();
        assert_eq!(book.pending_up.get(&(120, 1)), Some(&1));
        assert_eq!(book.pending_down.get(&(Reverse(80), 2)), Some(&2));
        assert_eq!(book.pending_time.get(&(1_000, 3)), Some(&3));
        assert!(book.bids.is_empty() && book.bid_depth.is_empty());

        set_single(&mut orders, &Single::active(1, 100, 5, true));
        let book = orders.book();
        assert!(book.pending_up.is_empty());
        assert_eq!(book.best_bid().map(|(price, _)| price), Some(100));
    }

    /// Each pending index has the next order to fire at its front, in the
    /// order Move takes them: up and time by lowest trigger, down by highest,
    /// then first placed first.
    #[test]
    fn pending_indexes_put_the_next_to_fire_first() {
        let mut orders = MarketOrders::default();
        for (order_id, trigger) in [
            (1, TriggerCondition::PriceMoveBelow(80)),
            (2, TriggerCondition::PriceMoveBelow(90)),
            (3, TriggerCondition::PriceMoveBelow(90)),
            (4, TriggerCondition::PriceMoveAbove(120)),
            (5, TriggerCondition::PriceMoveAbove(110)),
            (6, TriggerCondition::PriceMoveAbove(110)),
            (7, TriggerCondition::TimeBased(20)),
            (8, TriggerCondition::TimeBased(10)),
            (9, TriggerCondition::TimeBased(10)),
        ] {
            let mut order = Single::active(order_id, 100, 5, true);
            order.trigger = Some(trigger);
            set_single(&mut orders, &order);
        }
        let book = orders.book();
        let ids = |ids: Vec<&u128>| ids.into_iter().copied().collect::<Vec<_>>();
        assert_eq!(ids(book.pending_down.values().collect()), vec![2, 3, 1]);
        assert_eq!(ids(book.pending_up.values().collect()), vec![5, 6, 4]);
        assert_eq!(ids(book.pending_time.values().collect()), vec![8, 9, 7]);
    }

    /// The price-time index holds a bulk order's first level on each side;
    /// consuming it moves the entry to the next level at the same priority.
    /// Depth holds every level.
    #[test]
    fn bulk_levels_progress() {
        let mut orders = MarketOrders::default();
        orders.apply(
            &bulk_key(3),
            Some(bulk(3, &[(100, 5), (99, 6)], &[(101, 7)])),
        );
        let book = orders.book();
        assert_eq!(
            book.bids.get(&(Reverse(100), 1003)).map(|entry| entry.size),
            Some(5)
        );
        assert_eq!(
            book.bid_depth
                .iter()
                .map(|(Reverse(p), s)| (*p, *s))
                .collect::<Vec<_>>(),
            vec![(100, 5), (99, 6)]
        );
        assert_eq!(book.bulk_order_ids.get(&1003), Some(&addr(3)));

        orders.apply(&bulk_key(3), Some(bulk(3, &[(99, 6)], &[(101, 7)])));
        let book = orders.book();
        assert!(!book.bids.contains_key(&(Reverse(100), 1003)));
        assert_eq!(
            book.bids.get(&(Reverse(99), 1003)).map(|entry| entry.size),
            Some(6)
        );
        assert_eq!(
            book.bid_depth
                .iter()
                .map(|(Reverse(p), s)| (*p, *s))
                .collect::<Vec<_>>(),
            vec![(99, 6)]
        );

        orders.apply(&bulk_key(3), None);
        assert_eq!(orders.book(), &MarketBook::default());
    }

    /// Single and bulk orders aggregate into one depth level per price.
    #[test]
    fn depth_aggregates_singles_and_bulk_levels() {
        let mut orders = MarketOrders::default();
        set_single(&mut orders, &Single::active(1, 101, 2, false));
        orders.apply(&bulk_key(3), Some(bulk(3, &[], &[(101, 3), (102, 4)])));
        assert_eq!(
            orders
                .book()
                .ask_depth
                .iter()
                .map(|(p, s)| (*p, *s))
                .collect::<Vec<_>>(),
            vec![(101, 5), (102, 4)]
        );
    }

    #[derive(Clone, Debug)]
    enum Op {
        SetSingle(Single),
        DeleteSingle(u128),
        SetBulk {
            account: u8,
            bids: Vec<(u64, u64)>,
            asks: Vec<(u64, u64)>,
        },
        DeleteBulk(u8),
    }

    fn trigger() -> impl Strategy<Value = Option<TriggerCondition>> {
        prop_oneof![
            3 => Just(None),
            1 => (90u64..110).prop_map(|p| Some(TriggerCondition::PriceMoveAbove(p))),
            1 => (90u64..110).prop_map(|p| Some(TriggerCondition::PriceMoveBelow(p))),
            1 => (0u64..5).prop_map(|t| Some(TriggerCondition::TimeBased(t))),
        ]
    }

    fn levels() -> impl Strategy<Value = Vec<(u64, u64)>> {
        prop::collection::vec((95u64..105, 0u64..5), 0..4)
    }

    fn op() -> impl Strategy<Value = Op> {
        prop_oneof![
            4 => (0u128..12, 0u8..3, 95u64..105, 0u64..5, any::<bool>(), trigger(), any::<bool>())
                .prop_map(|(order_id, account, price, remaining_size, is_bid, trigger, with_coid)| {
                    Op::SetSingle(Single {
                        order_id,
                        account,
                        price,
                        remaining_size,
                        is_bid,
                        trigger,
                        with_coid,
                    })
                }),
            2 => (0u128..12).prop_map(Op::DeleteSingle),
            2 => (0u8..3, levels(), levels())
                .prop_map(|(account, bids, asks)| Op::SetBulk { account, bids, asks }),
            1 => (0u8..3).prop_map(Op::DeleteBulk),
        ]
    }

    proptest! {
        /// The index rule: however the orders got there, the incrementally
        /// maintained book equals one rebuilt from the final orders alone.
        #[test]
        fn incremental_book_equals_rebuilt(ops in prop::collection::vec(op(), 0..60)) {
            let mut orders = MarketOrders::default();
            for op in &ops {
                match op {
                    Op::SetSingle(single) => set_single(&mut orders, single),
                    Op::DeleteSingle(order_id) => orders.apply(&single_key(*order_id), None),
                    Op::SetBulk { account, bids, asks } => {
                        orders.apply(&bulk_key(*account), Some(bulk(*account, bids, asks)))
                    },
                    Op::DeleteBulk(account) => orders.apply(&bulk_key(*account), None),
                }
            }
            prop_assert_eq!(&orders, &rebuilt(&orders));
        }
    }

    fn write_of(op: &Op) -> (OrderKey, Option<NativeOrder>) {
        match op {
            Op::SetSingle(single) => (single_key(single.order_id), Some(single.order())),
            Op::DeleteSingle(order_id) => (single_key(*order_id), None),
            Op::SetBulk {
                account,
                bids,
                asks,
            } => (bulk_key(*account), Some(bulk(*account, bids, asks))),
            Op::DeleteBulk(account) => (bulk_key(*account), None),
        }
    }

    fn market() -> MarketKey {
        MarketKey {
            exchange: addr(1),
            market: addr(7),
        }
    }

    /// Everything a view answers about one market.
    #[derive(Debug, PartialEq)]
    struct Answers {
        bids: Vec<(u64, BookEntry)>,
        asks: Vec<(u64, BookEntry)>,
        bid_depth: Vec<(u64, u128)>,
        ask_depth: Vec<(u64, u128)>,
        best: (Option<u64>, Option<u64>),
        singles: Vec<(u128, NativeSingleOrder)>,
        bulks: Vec<(AccountAddress, NativeBulkOrder)>,
    }

    fn answers(
        book: &BookView<'_>,
        singles: impl Iterator<Item = (u128, NativeSingleOrder)>,
        bulks: impl Iterator<Item = (AccountAddress, NativeBulkOrder)>,
    ) -> Answers {
        Answers {
            bids: book.bids().map(|(price, entry)| (price, *entry)).collect(),
            asks: book.asks().map(|(price, entry)| (price, *entry)).collect(),
            bid_depth: book.depth_levels(true).collect(),
            ask_depth: book.depth_levels(false).collect(),
            best: book.best_prices(),
            singles: singles.collect(),
            bulks: bulks.collect(),
        }
    }

    proptest! {
        /// A block's view, the base with the overlay's writes applied on
        /// read, answers exactly as the base does after folding the overlay.
        #[test]
        fn view_over_an_overlay_equals_the_folded_book(
            seed in prop::collection::vec(op(), 0..40),
            writes in prop::collection::vec(op(), 0..20),
        ) {
            let mut seeded = MarketOrders::default();
            for op in &seed {
                let (key, write) = write_of(op);
                seeded.apply(&key, write);
            }
            let rows: Vec<_> = seeded
                .singles()
                .iter()
                .map(|(order_id, order)| (single_key(*order_id), NativeOrder::Single(order.clone())))
                .chain(seeded.bulks().iter().map(|(account, order)| {
                    let key = OrderKey::Bulk {
                        exchange: addr(1),
                        market: addr(7),
                        account: *account,
                    };
                    (key, NativeOrder::Bulk(order.clone()))
                }))
                .collect();
            let base = OrderBase::new_at_version(Some(0), "test", rows.clone());
            let folded = OrderBase::new_at_version(Some(0), "test", rows);
            let writes: Vec<_> = writes.iter().map(write_of).collect();
            let overlay = base.new_overlay().extend(&base.layers(), 1, writes.clone());
            let folded_overlay = folded.new_overlay().extend(&folded.layers(), 1, writes);

            let from_view = base.with_view(&overlay, |view| {
                answers(
                    &view.book(&market()),
                    view.singles(&market()).map(|(id, order)| (id, order.clone())),
                    view.bulks(&market()).map(|(account, order)| (account, order.clone())),
                )
            });
            prop_assert!(folded.fold(&folded_overlay));
            let from_fold = folded.read(|view| {
                let orders = view.market_orders(&market());
                answers(
                    &BookView::new(orders.map(MarketOrders::book), BookDelta::empty()),
                    orders
                        .into_iter()
                        .flat_map(MarketOrders::singles)
                        .map(|(id, order)| (*id, order.clone())),
                    orders
                        .into_iter()
                        .flat_map(MarketOrders::bulks)
                        .map(|(account, order)| (*account, order.clone())),
                )
            });
            prop_assert_eq!(from_view, from_fold);
        }
    }
}
