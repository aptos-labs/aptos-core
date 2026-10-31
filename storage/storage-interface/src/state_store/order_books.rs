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
use std::{
    cmp::Reverse,
    collections::{BTreeMap, BTreeSet},
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

/// Pending triggers, keyed by `(trigger value, priority)` ascending.
pub type PendingKey = (u64, u128);

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
    pub pending_up: BTreeMap<PendingKey, u128>,
    /// `PriceMoveBelow` triggers.
    pub pending_down: BTreeMap<PendingKey, u128>,
    /// `TimeBased` triggers.
    pub pending_time: BTreeMap<PendingKey, u128>,
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
                insert_unique(&mut self.pending_down, (price, idx), order.order_id);
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
                self.pending_down.remove(&(price, idx));
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
