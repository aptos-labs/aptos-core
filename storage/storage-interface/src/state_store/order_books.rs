// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! A market's derived book: every index the Move order book keeps beside
//! its orders, as a pure function of the orders. Writes retract the old
//! value's entries and insert the new value's, so the book cannot drift
//! from the orders it is derived from.

#![forbid(unsafe_code)]

use aptos_types::{
    account_address::AccountAddress,
    state_store::native_order::{NativeBulkOrder, NativeSingleOrder, TriggerCondition},
};
use itertools::{EitherOrBoth, Itertools};
use std::{
    cmp::Reverse,
    collections::{BTreeMap, BTreeSet},
    sync::LazyLock,
};

/// Which order a book entry belongs to.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum BookOrder {
    Single {
        order_id: u128,
    },
    /// A bulk order, which its account holds at most one of per market.
    Bulk {
        account: AccountAddress,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BookEntry {
    pub order: BookOrder,
    pub size: u64,
}

/// Bids, best first: price descending, then priority ascending, as Move's
/// `PriceDescTime` orders them.
pub type BidKey = (Reverse<u64>, u128);

/// Asks, best first: price ascending, then priority ascending.
pub type AskKey = (u64, u128);

/// `PriceMoveAbove` triggers, next to fire first: lowest trigger price, then
/// priority ascending, as Move takes `PendingUpOrderKey` from the front.
pub type PendingUpKey = (u64, u128);

/// `PriceMoveBelow` triggers, next to fire first: highest trigger price, then
/// priority ascending, as Move takes `PendingDownOrderKey` from the back.
pub type PendingDownKey = (Reverse<u64>, u128);

/// `TimeBased` triggers, next to fire first: earliest time, then priority
/// ascending, as Move takes `PendingTimeKey` from the front.
pub type PendingTimeKey = (u64, u128);

/// The book of one `(exchange, market)`. Index rule, per order:
///
/// - single order, no trigger, `remaining_size > 0` → a price-time entry at
///   `(price, unique_priority_idx)` on its side, and its size at its price
///   in the depth index;
/// - single order with a trigger → one pending entry: up, down or time;
/// - single order with a client order id → `(account, coid) → order_id`;
/// - every single order → its id under its account;
/// - bulk order → a price-time entry at each side's first level, every
///   level in the depth index, and `order_id → account`.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct MarketBook {
    pub bids: BTreeMap<BidKey, BookEntry>,
    pub asks: BTreeMap<AskKey, BookEntry>,
    /// Aggregate resting size per price, best first. Unlike `bids`, holds
    /// every bulk level, not just the first.
    pub bid_depth: BTreeMap<Reverse<u64>, u128>,
    pub ask_depth: BTreeMap<u64, u128>,
    /// `PriceMoveAbove` triggers.
    pub pending_up: BTreeMap<PendingUpKey, u128>,
    /// `PriceMoveBelow` triggers.
    pub pending_down: BTreeMap<PendingDownKey, u128>,
    /// `TimeBased` triggers.
    pub pending_time: BTreeMap<PendingTimeKey, u128>,
    pub client_order_ids: BTreeMap<(AccountAddress, String), u128>,
    pub bulk_order_ids: BTreeMap<u128, AccountAddress>,
    /// Each account's single order ids. Its bulk order, if any, is keyed by
    /// account already.
    pub account_orders: BTreeMap<AccountAddress, BTreeSet<u128>>,
}

impl MarketBook {
    pub fn insert_single(&mut self, order: &NativeSingleOrder) {
        let NativeSingleOrder::V1(order) = order;
        let entry = BookEntry {
            order: BookOrder::Single {
                order_id: order.order_id,
            },
            size: order.remaining_size,
        };
        let idx = order.unique_priority_idx;
        match order.trigger_condition {
            None if order.remaining_size > 0 => {
                if order.is_bid {
                    insert_unique(&mut self.bids, (Reverse(order.price), idx), entry);
                    add_depth(
                        &mut self.bid_depth,
                        Reverse(order.price),
                        order.remaining_size,
                    );
                } else {
                    insert_unique(&mut self.asks, (order.price, idx), entry);
                    add_depth(&mut self.ask_depth, order.price, order.remaining_size);
                }
            },
            None => {},
            Some(TriggerCondition::PriceMoveAbove(price)) => {
                insert_unique(&mut self.pending_up, (price, idx), order.order_id);
            },
            Some(TriggerCondition::PriceMoveBelow(price)) => {
                insert_unique(
                    &mut self.pending_down,
                    (Reverse(price), idx),
                    order.order_id,
                );
            },
            Some(TriggerCondition::TimeBased(time)) => {
                insert_unique(&mut self.pending_time, (time, idx), order.order_id);
            },
        }
        if let Some(coid) = &order.client_order_id {
            insert_unique(
                &mut self.client_order_ids,
                (order.account, coid.clone()),
                order.order_id,
            );
        }
        self.account_orders
            .entry(order.account)
            .or_default()
            .insert(order.order_id);
    }

    pub fn remove_single(&mut self, order: &NativeSingleOrder) {
        let NativeSingleOrder::V1(order) = order;
        let idx = order.unique_priority_idx;
        match order.trigger_condition {
            None if order.remaining_size > 0 => {
                if order.is_bid {
                    self.bids.remove(&(Reverse(order.price), idx));
                    sub_depth(
                        &mut self.bid_depth,
                        Reverse(order.price),
                        order.remaining_size,
                    );
                } else {
                    self.asks.remove(&(order.price, idx));
                    sub_depth(&mut self.ask_depth, order.price, order.remaining_size);
                }
            },
            None => {},
            Some(TriggerCondition::PriceMoveAbove(price)) => {
                self.pending_up.remove(&(price, idx));
            },
            Some(TriggerCondition::PriceMoveBelow(price)) => {
                self.pending_down.remove(&(Reverse(price), idx));
            },
            Some(TriggerCondition::TimeBased(time)) => {
                self.pending_time.remove(&(time, idx));
            },
        }
        if let Some(coid) = &order.client_order_id {
            self.client_order_ids.remove(&(order.account, coid.clone()));
        }
        if let Some(ids) = self.account_orders.get_mut(&order.account) {
            ids.remove(&order.order_id);
            if ids.is_empty() {
                self.account_orders.remove(&order.account);
            }
        }
    }

    pub fn insert_bulk(&mut self, order: &NativeBulkOrder) {
        let NativeBulkOrder::V1(order) = order;
        let book_order = BookOrder::Bulk {
            account: order.account,
        };
        let idx = order.unique_priority_idx;
        if let (Some(price), Some(size)) = (order.bid_prices.first(), order.bid_sizes.first()) {
            insert_unique(&mut self.bids, (Reverse(*price), idx), BookEntry {
                order: book_order,
                size: *size,
            });
        }
        if let (Some(price), Some(size)) = (order.ask_prices.first(), order.ask_sizes.first()) {
            insert_unique(&mut self.asks, (*price, idx), BookEntry {
                order: book_order,
                size: *size,
            });
        }
        for (price, size) in order.bid_prices.iter().zip(&order.bid_sizes) {
            add_depth(&mut self.bid_depth, Reverse(*price), *size);
        }
        for (price, size) in order.ask_prices.iter().zip(&order.ask_sizes) {
            add_depth(&mut self.ask_depth, *price, *size);
        }
        insert_unique(&mut self.bulk_order_ids, order.order_id, order.account);
    }

    pub fn remove_bulk(&mut self, order: &NativeBulkOrder) {
        let NativeBulkOrder::V1(order) = order;
        let idx = order.unique_priority_idx;
        if let Some(price) = order.bid_prices.first() {
            self.bids.remove(&(Reverse(*price), idx));
        }
        if let Some(price) = order.ask_prices.first() {
            self.asks.remove(&(*price, idx));
        }
        for (price, size) in order.bid_prices.iter().zip(&order.bid_sizes) {
            sub_depth(&mut self.bid_depth, Reverse(*price), *size);
        }
        for (price, size) in order.ask_prices.iter().zip(&order.ask_sizes) {
            sub_depth(&mut self.ask_depth, *price, *size);
        }
        self.bulk_order_ids.remove(&order.order_id);
    }

    pub fn best_bid(&self) -> Option<(u64, &BookEntry)> {
        self.bids
            .first_key_value()
            .map(|((Reverse(price), _), entry)| (*price, entry))
    }

    pub fn best_ask(&self) -> Option<(u64, &BookEntry)> {
        self.asks
            .first_key_value()
            .map(|((price, _), entry)| (*price, entry))
    }
}

/// Keys the dual-write keeps unique (priority per side, client order id per
/// account, bulk order id); a collision means the writer broke that.
fn insert_unique<K: Ord, V>(map: &mut BTreeMap<K, V>, key: K, value: V) {
    let prior = map.insert(key, value);
    debug_assert!(prior.is_none(), "duplicate order book key");
}

fn add_depth<K: Ord>(depth: &mut BTreeMap<K, u128>, price: K, size: u64) {
    if size > 0 {
        *depth.entry(price).or_default() += u128::from(size);
    }
}

fn sub_depth<K: Ord>(depth: &mut BTreeMap<K, u128>, price: K, size: u64) {
    if size == 0 {
        return;
    }
    if let Some(total) = depth.get_mut(&price) {
        *total = total.saturating_sub(u128::from(size));
        if *total == 0 {
            depth.remove(&price);
        }
    }
}

/// What writes change in a base book: the written orders' old values derived
/// into `retracted`, their new values into `inserted`, by the same index rule.
#[derive(Debug, Default)]
pub struct BookDelta {
    pub retracted: MarketBook,
    pub inserted: MarketBook,
}

static EMPTY_DELTA: LazyLock<BookDelta> = LazyLock::new(BookDelta::default);

impl BookDelta {
    pub fn empty() -> &'static BookDelta {
        &EMPTY_DELTA
    }
}

/// A base book with a delta applied on read, without copying it: every query
/// merges `base - retracted + inserted`. Costs what the delta holds plus what
/// the query walks, never the size of the book.
pub struct BookView<'a> {
    base: Option<&'a MarketBook>,
    delta: &'a BookDelta,
}

impl<'a> BookView<'a> {
    pub fn new(base: Option<&'a MarketBook>, delta: &'a BookDelta) -> Self {
        Self { base, delta }
    }

    /// Bids, best first.
    pub fn bids(&self) -> impl Iterator<Item = (u64, &BookEntry)> {
        merge_entries(
            self.base.map(|book| &book.bids),
            &self.delta.retracted.bids,
            &self.delta.inserted.bids,
        )
        .map(|((Reverse(price), _), entry)| (*price, entry))
    }

    /// Asks, best first.
    pub fn asks(&self) -> impl Iterator<Item = (u64, &BookEntry)> {
        merge_entries(
            self.base.map(|book| &book.asks),
            &self.delta.retracted.asks,
            &self.delta.inserted.asks,
        )
        .map(|((price, _), entry)| (*price, entry))
    }

    /// `(best bid, best ask)`; `None` for an empty side.
    pub fn best_prices(&self) -> (Option<u64>, Option<u64>) {
        (
            self.bids().next().map(|(price, _)| price),
            self.asks().next().map(|(price, _)| price),
        )
    }

    /// Aggregate size per price on a side, best first.
    pub fn depth_levels(&self, is_bid: bool) -> Box<dyn Iterator<Item = (u64, u128)> + '_> {
        if is_bid {
            Box::new(
                merge_depth(
                    self.base.map(|book| &book.bid_depth),
                    &self.delta.retracted.bid_depth,
                    &self.delta.inserted.bid_depth,
                )
                .map(|(Reverse(price), size)| (price, size)),
            )
        } else {
            Box::new(merge_depth(
                self.base.map(|book| &book.ask_depth),
                &self.delta.retracted.ask_depth,
                &self.delta.inserted.ask_depth,
            ))
        }
    }

    /// The best `max_levels` depth levels on a side.
    pub fn depth(&self, is_bid: bool, max_levels: usize) -> Vec<(u64, u128)> {
        self.depth_levels(is_bid).take(max_levels).collect()
    }

    /// Size resting at `limit_px` or better: bids at or above it, asks at or
    /// below it.
    pub fn depth_within(&self, is_bid: bool, limit_px: u64) -> u128 {
        self.depth_levels(is_bid)
            .take_while(|(price, _)| {
                if is_bid {
                    *price >= limit_px
                } else {
                    *price <= limit_px
                }
            })
            .map(|(_, size)| size)
            .sum()
    }

    /// `(impact bid, impact ask)`: the average price of selling / buying
    /// `notional_raw` (price × size units) into the book, over at most
    /// `max_levels` levels per side. `None` for an empty side.
    pub fn impact_prices(
        &self,
        notional_raw: u128,
        max_levels: usize,
    ) -> (Option<u64>, Option<u64>) {
        (
            impact_price(self.depth_levels(true), notional_raw, max_levels, false),
            impact_price(self.depth_levels(false), notional_raw, max_levels, true),
        )
    }

    /// The account's single order ids, ascending.
    pub fn account_single_orders(&self, account: &AccountAddress) -> Vec<u128> {
        let retracted = self.delta.retracted.account_orders.get(account);
        let mut ids: BTreeSet<u128> = self
            .base
            .and_then(|book| book.account_orders.get(account))
            .into_iter()
            .flatten()
            .filter(|id| !retracted.is_some_and(|ids| ids.contains(*id)))
            .copied()
            .collect();
        if let Some(inserted) = self.delta.inserted.account_orders.get(account) {
            ids.extend(inserted);
        }
        ids.into_iter().collect()
    }
}

/// Base entries not retracted, merged in key order with the inserted ones.
/// A key both retracted and inserted (a partial fill keeps its key) reads as
/// the inserted entry.
fn merge_entries<'a, K: Ord, V>(
    base: Option<&'a BTreeMap<K, V>>,
    retracted: &'a BTreeMap<K, V>,
    inserted: &'a BTreeMap<K, V>,
) -> impl Iterator<Item = (&'a K, &'a V)> {
    let base = base
        .into_iter()
        .flatten()
        .filter(move |(key, _)| !retracted.contains_key(key));
    base.merge_join_by(inserted, |(a, _), (b, _)| a.cmp(b))
        .map(|either| match either {
            EitherOrBoth::Left(entry) | EitherOrBoth::Right(entry) => entry,
            EitherOrBoth::Both(_, inserted) => inserted,
        })
}

/// `base - retracted + inserted` per price, in key order, dropping levels
/// that net to zero. Retracted sizes only ever come out of base levels.
fn merge_depth<'a, K: Ord + Copy>(
    base: Option<&'a BTreeMap<K, u128>>,
    retracted: &'a BTreeMap<K, u128>,
    inserted: &'a BTreeMap<K, u128>,
) -> impl Iterator<Item = (K, u128)> + 'a {
    base.into_iter()
        .flatten()
        .merge_join_by(inserted, |(a, _), (b, _)| a.cmp(b))
        .filter_map(move |either| {
            let (key, base_size, inserted_size) = match either {
                EitherOrBoth::Left((key, size)) => (key, *size, 0),
                EitherOrBoth::Right((key, size)) => (key, 0, *size),
                EitherOrBoth::Both((key, base_size), (_, size)) => (key, *base_size, *size),
            };
            let size =
                base_size.saturating_sub(retracted.get(key).copied().unwrap_or(0)) + inserted_size;
            (size > 0).then_some((*key, size))
        })
}

/// The average price of filling `notional_raw` against `levels`, best first.
/// Only whole size units fill at the level that completes the target, as
/// loracle's `DepthBook::impact_{bid,ask}_price`; a book exhausted within
/// `max_levels` prices the remainder at the worst level reached. Buying
/// (asks) rounds up, selling (bids) rounds down.
pub fn impact_price(
    levels: impl Iterator<Item = (u64, u128)>,
    notional_raw: u128,
    max_levels: usize,
    round_up: bool,
) -> Option<u64> {
    let average = |notional: u128, filled: u128| {
        (filled > 0).then(|| {
            let price = if round_up {
                notional.div_ceil(filled)
            } else {
                notional / filled
            };
            u64::try_from(price).expect("an average of u64 prices fits in u64")
        })
    };
    let mut notional = 0u128;
    let mut filled = 0u128;
    let mut worst = None;
    // A zero price can't fill any notional.
    for (price, size) in levels.filter(|(price, _)| *price > 0).take(max_levels) {
        worst = Some(price);
        let price = u128::from(price);
        let level_notional = price.saturating_mul(size);
        let remaining = notional_raw - notional;
        if level_notional >= remaining {
            let units = remaining / price;
            return average(notional + units * price, filled + units);
        }
        notional += level_notional;
        filled += size;
    }
    let worst = u128::from(worst?);
    let units = (notional_raw - notional) / worst;
    average(notional + units * worst, filled + units)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `(impact bid, impact ask)` over raw depth levels, best first.
    fn impact(
        bids: &[(u64, u128)],
        asks: &[(u64, u128)],
        notional: u128,
        max_levels: usize,
    ) -> (Option<u64>, Option<u64>) {
        (
            impact_price(bids.iter().copied(), notional, max_levels, false),
            impact_price(asks.iter().copied(), notional, max_levels, true),
        )
    }

    /// loracle's `DepthBook::impact_{bid,ask}_price` test vectors.
    #[test]
    fn impact_prices_match_loracle() {
        let all = usize::MAX;
        // Partial fill on the second level; asks round up, bids down.
        assert_eq!(impact(&[], &[(100, 10), (110, 20)], 1500, all).1, Some(103));
        assert_eq!(impact(&[(200, 10), (190, 20)], &[], 3000, all).0, Some(196));
        // Partial fill of a single level.
        assert_eq!(impact(&[], &[(1000, 100)], 50_000, all).1, Some(1000));
        assert_eq!(impact(&[(500, 200)], &[], 25_000, all).0, Some(500));
        // Empty sides.
        assert_eq!(impact(&[], &[], 10_000, all), (None, None));
        // Exhausted: the remainder prices at the worst level.
        assert_eq!(impact(&[], &[(100, 10)], 5000, all).1, Some(100));
        assert_eq!(impact(&[(200, 5)], &[], 5000, all).0, Some(200));
        assert_eq!(impact(&[], &[(100, 10), (200, 5)], 5000, all).1, Some(167));
        assert_eq!(impact(&[(200, 10), (100, 5)], &[], 5000, all).0, Some(125));
        // Exact fills.
        assert_eq!(impact(&[], &[(100, 100)], 10_000, all).1, Some(100));
        assert_eq!(impact(&[(50, 200)], &[], 10_000, all).0, Some(50));
        // Sides are independent, not averaged.
        assert_eq!(
            impact(&[(1_000, 100)], &[(2_000, 100)], 100_000, all),
            (Some(1_000), Some(2_000))
        );
        // Realistic decimals: BTC at 60k, 6 price and 4 size decimals. Only
        // whole units fill, and the notional is what those units cost, so
        // one level prices at exactly its price.
        assert_eq!(
            impact(&[], &[(60_000_000_000, 10_000)], 100_000_000_000_000, all).1,
            Some(60_000_000_000)
        );
    }

    /// The walk stops at `max_levels` and treats the book as exhausted there.
    #[test]
    fn impact_walk_stops_at_max_levels() {
        let asks = [(100, 10), (200, 5)];
        assert_eq!(impact(&[], &asks, 5000, 2).1, Some(167));
        assert_eq!(impact(&[], &asks, 5000, 1).1, Some(100));
        assert_eq!(impact(&[], &asks, 5000, 0).1, None);
    }

    /// Nothing fills a zero notional, as in loracle.
    #[test]
    fn zero_notional_has_no_impact_price() {
        assert_eq!(impact(&[(100, 1)], &[(101, 1)], 0, 10), (None, None));
    }
}
