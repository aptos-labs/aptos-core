// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Validator-side reader API for native-position data.
//!
//! Off-Move consumers query the position index held at the
//! `PositionBundle` level via [`NativeStateReader`], without going
//! through the VM session cache. Reads resolve against the committed
//! overlay first and the account-nested base behind it.

#![forbid(unsafe_code)]

use crate::native_state_store::{
    shard_of, AccountKey, PositionBaseView, PositionOverlay, NUM_POSITION_SHARDS,
};
use aptos_infallible::Mutex;
use aptos_types::state_store::native_position::NativePosition;
use move_core_types::account_address::AccountAddress;
use rayon::prelude::*;
use std::sync::Arc;

static LATEST_READER: std::sync::Mutex<Option<Arc<InMemoryNativeStateReader>>> =
    std::sync::Mutex::new(None);

pub fn install_global_reader(reader: Arc<InMemoryNativeStateReader>) {
    if let Ok(mut guard) = LATEST_READER.lock() {
        *guard = Some(reader);
    }
}

pub fn global_reader() -> Option<Arc<InMemoryNativeStateReader>> {
    LATEST_READER.lock().ok().and_then(|g| g.clone())
}

pub trait NativeStateReader: Send + Sync {
    fn iter_position_accounts_for_exchange(&self, exchange: AccountAddress) -> Vec<AccountAddress>;

    fn get_account_positions(
        &self,
        exchange: AccountAddress,
        account: AccountAddress,
    ) -> Vec<(AccountAddress, NativePosition)>;

    fn count_positions_for_exchange(&self, exchange: AccountAddress) -> usize;
}

/// The overlay's writes grouped by account, bucketed per shard so each
/// shard task walks only its own. Built once per view, which is
/// affordable because the overlay holds only the writes since the last
/// fold — ordering it instead would cost a sort on a 96-byte key and buy
/// nothing.
type OverlayByAccount = ahash::AHashMap<AccountKey, Vec<(AccountAddress, Option<NativePosition>)>>;
type OverlayByShard = Vec<OverlayByAccount>;

pub struct InMemoryNativeStateReader {
    positions: Arc<Mutex<PositionOverlay>>,
}

impl InMemoryNativeStateReader {
    pub fn new(positions: Arc<Mutex<PositionOverlay>>) -> Self {
        Self { positions }
    }

    /// Runs `f` against a coherent view of the overlay and the base.
    ///
    /// The base lock is taken while the tip mutex is still held, so a
    /// fold can't land between freezing the overlay and reading the
    /// base and leave the two halves at different versions. Whatever
    /// `f` does delays the next fold, so keep it to the queries.
    pub fn with_view<R>(&self, f: impl FnOnce(&NativeStateView<'_>) -> R) -> R {
        let tip = self.positions.lock();
        let base = Arc::clone(tip.base());
        base.read(move |base_view| {
            let overlay = tip.clone();
            drop(tip);
            let mut by_shard: OverlayByShard = (0..NUM_POSITION_SHARDS)
                .map(|_| OverlayByAccount::default())
                .collect();
            for (key, write) in overlay.writes_since_base() {
                let account = key.account();
                by_shard[shard_of(&account)]
                    .entry(account)
                    .or_default()
                    .push((key.market, write));
            }
            f(&NativeStateView {
                base: base_view,
                by_shard,
            })
        })
    }
}

impl NativeStateReader for InMemoryNativeStateReader {
    fn iter_position_accounts_for_exchange(&self, exchange: AccountAddress) -> Vec<AccountAddress> {
        self.with_view(|view| view.iter_position_accounts_for_exchange(exchange))
    }

    fn count_positions_for_exchange(&self, exchange: AccountAddress) -> usize {
        self.with_view(|view| view.count_positions_for_exchange(exchange))
    }

    fn get_account_positions(
        &self,
        exchange: AccountAddress,
        account: AccountAddress,
    ) -> Vec<(AccountAddress, NativePosition)> {
        self.with_view(|view| view.get_account_positions(exchange, account))
    }
}

/// The base pinned beneath the committed overlay's writes, valid for the
/// scope of [`InMemoryNativeStateReader::with_view`].
pub struct NativeStateView<'a> {
    base: &'a PositionBaseView<'a>,
    by_shard: OverlayByShard,
}

impl NativeStateView<'_> {
    /// Visits each of the exchange's accounts exactly once, with its
    /// full position set.
    ///
    /// Scoped to the exchange by a range rather than a filter, so it
    /// costs what the exchange holds rather than what every exchange
    /// holds together. The base is nested by account, so the grouping
    /// is structural — no run detection. Only the overlay's accounts
    /// need merging in, and there are at most one block's worth.
    pub fn for_each_account_in_exchange<T, F>(&self, exchange: AccountAddress, f: F) -> Vec<T>
    where
        T: Send,
        F: Fn(&AccountKey, &[(AccountAddress, NativePosition)]) -> Option<T> + Sync + Send,
    {
        (0..NUM_POSITION_SHARDS)
            .into_par_iter()
            .flat_map_iter(|shard| {
                let mut out = Vec::new();
                let mut buf: Vec<(AccountAddress, NativePosition)> = Vec::new();

                for (account, markets) in self.base.accounts_in_exchange(shard, exchange) {
                    buf.clear();
                    for (market, position) in markets {
                        match self.overlay_write(account, market) {
                            // Overridden or deleted by a later write.
                            Some(Some(newer)) => buf.push((*market, newer)),
                            Some(None) => {},
                            None => buf.push((*market, position.clone())),
                        }
                    }
                    if let Some(writes) = self.by_shard[shard_of(account)].get(account) {
                        for (market, write) in writes {
                            if let Some(position) = write
                                && !markets.contains_key(market)
                            {
                                buf.push((*market, position.clone()));
                            }
                        }
                        buf.sort_by_key(|(market, _)| *market);
                    }
                    if !buf.is_empty()
                        && let Some(item) = f(account, &buf)
                    {
                        out.push(item);
                    }
                }

                // Accounts the overlay opened that the base has never
                // seen. Few enough to test each against the base.
                for (account, writes) in self.by_shard[shard].iter() {
                    if account.exchange != exchange || self.base.shard(shard).contains_key(account)
                    {
                        continue;
                    }
                    let mut positions: Vec<_> = writes
                        .iter()
                        .filter_map(|(market, write)| write.clone().map(|p| (*market, p)))
                        .collect();
                    positions.sort_by_key(|(market, _)| *market);
                    if !positions.is_empty()
                        && let Some(item) = f(account, &positions)
                    {
                        out.push(item);
                    }
                }
                out
            })
            .collect()
    }

    /// The account's overlay write for `market`, if any. `Some(None)` is
    /// a delete. An account holds few markets, so a linear walk beats
    /// building a second map.
    fn overlay_write(
        &self,
        account: &AccountKey,
        market: &AccountAddress,
    ) -> Option<Option<NativePosition>> {
        self.by_shard[shard_of(account)]
            .get(account)
            .and_then(|writes| {
                writes
                    .iter()
                    .find(|(m, _)| m == market)
                    .map(|(_, write)| write.clone())
            })
    }

    /// Order is shard-interleaved, not address-sorted. Callers needing a
    /// stable order must sort.
    pub fn iter_position_accounts_for_exchange(
        &self,
        exchange: AccountAddress,
    ) -> Vec<AccountAddress> {
        self.for_each_account_in_exchange(exchange, |account, positions| {
            (!positions.is_empty()).then_some(account.account)
        })
    }

    pub fn count_positions_for_exchange(&self, exchange: AccountAddress) -> usize {
        self.for_each_account_in_exchange(exchange, |_, positions| Some(positions.len()))
            .into_iter()
            .sum()
    }

    /// Range scan over the account's base positions, merged with its
    /// overlay writes. In market order.
    /// One lookup into the account's positions, merged with its overlay
    /// writes. In market order.
    pub fn get_account_positions(
        &self,
        exchange: AccountAddress,
        account: AccountAddress,
    ) -> Vec<(AccountAddress, NativePosition)> {
        let key = AccountKey { exchange, account };
        let base_markets = self.base.account_positions(&key);
        let mut positions: Vec<(AccountAddress, NativePosition)> = base_markets
            .into_iter()
            .flatten()
            .filter_map(
                |(market, position)| match self.overlay_write(&key, market) {
                    Some(Some(newer)) => Some((*market, newer)),
                    Some(None) => None,
                    None => Some((*market, position.clone())),
                },
            )
            .collect();
        if let Some(writes) = self.by_shard[shard_of(&key)].get(&key) {
            for (market, write) in writes {
                if let Some(position) = write
                    && !base_markets.is_some_and(|m| m.contains_key(market))
                {
                    positions.push((*market, position.clone()));
                }
            }
            positions.sort_by_key(|(market, _)| *market);
        }
        positions
    }
}
