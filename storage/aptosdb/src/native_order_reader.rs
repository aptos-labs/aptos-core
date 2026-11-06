// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Reader over the committed order tip, the order counterpart of
//! [`crate::native_state_reader`].
//!
//! The query engine is [`OrderBase::with_view`]; this binds it to the
//! bundle's published tip. Consumers that must be deterministic per block,
//! like a price or liquidation pipeline, read that block's overlay from the
//! executor through [`InMemoryNativeOrderReader::with_view_of`] instead: the
//! tip trails execution by however far the commit pipeline lags.

#![forbid(unsafe_code)]

use aptos_infallible::Mutex;
use aptos_storage_interface::state_store::orders::{
    MarketKey, NativeOrder, OrderBase, OrderKey, OrderOverlay, OrderView,
};
use aptos_types::transaction::Version;
use move_core_types::account_address::AccountAddress;
use std::sync::Arc;

pub trait NativeOrderReader: Send + Sync {
    fn get_order(&self, key: &OrderKey) -> Option<NativeOrder>;

    fn best_prices(&self, market: &MarketKey) -> (Option<u64>, Option<u64>);

    fn impact_prices(
        &self,
        market: &MarketKey,
        notional_raw: u128,
        max_levels: usize,
    ) -> (Option<u64>, Option<u64>);

    fn depth(&self, market: &MarketKey, is_bid: bool, max_levels: usize) -> Vec<(u64, u128)>;

    fn depth_within(&self, market: &MarketKey, is_bid: bool, limit_px: u64) -> u128;

    fn account_orders(&self, market: &MarketKey, account: AccountAddress) -> Vec<OrderKey>;

    fn markets_in_exchange(&self, exchange: AccountAddress) -> Vec<AccountAddress>;
}

pub struct InMemoryNativeOrderReader {
    base: Arc<OrderBase>,
    orders: Arc<Mutex<OrderOverlay>>,
}

impl InMemoryNativeOrderReader {
    pub fn new(base: Arc<OrderBase>, orders: Arc<Mutex<OrderOverlay>>) -> Self {
        Self { base, orders }
    }

    /// Holds the tip mutex for the scope, so the overlay cannot be
    /// republished underneath the view. Tip then base, everywhere.
    pub fn with_view<R>(&self, f: impl FnOnce(&OrderView<'_>) -> R) -> R {
        let tip = self.orders.lock();
        self.base.with_view(&tip, f)
    }

    /// A specific overlay — a block's, say — over the same base.
    pub fn with_view_of<R>(
        &self,
        overlay: &OrderOverlay,
        f: impl FnOnce(&OrderView<'_>) -> R,
    ) -> R {
        self.base.with_view(overlay, f)
    }

    /// Version of the published tip.
    pub fn tip_version(&self) -> Option<Version> {
        self.orders.lock().version()
    }
}

impl NativeOrderReader for InMemoryNativeOrderReader {
    fn get_order(&self, key: &OrderKey) -> Option<NativeOrder> {
        self.with_view(|view| view.get(key))
    }

    fn best_prices(&self, market: &MarketKey) -> (Option<u64>, Option<u64>) {
        self.with_view(|view| view.best_prices(market))
    }

    fn impact_prices(
        &self,
        market: &MarketKey,
        notional_raw: u128,
        max_levels: usize,
    ) -> (Option<u64>, Option<u64>) {
        self.with_view(|view| view.impact_prices(market, notional_raw, max_levels))
    }

    fn depth(&self, market: &MarketKey, is_bid: bool, max_levels: usize) -> Vec<(u64, u128)> {
        self.with_view(|view| view.depth(market, is_bid, max_levels))
    }

    fn depth_within(&self, market: &MarketKey, is_bid: bool, limit_px: u64) -> u128 {
        self.with_view(|view| view.depth_within(market, is_bid, limit_px))
    }

    fn account_orders(&self, market: &MarketKey, account: AccountAddress) -> Vec<OrderKey> {
        self.with_view(|view| view.account_orders(market, account))
    }

    fn markets_in_exchange(&self, exchange: AccountAddress) -> Vec<AccountAddress> {
        self.with_view(|view| view.markets_in_exchange(exchange))
    }
}
