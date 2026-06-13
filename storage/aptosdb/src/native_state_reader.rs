// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Validator-side reader API for native-position data.
//!
//! At block boundaries off-Move consumers query the layered
//! per-account [`UserPositions`] held at the `PositionBundle` level
//! via [`NativeStateReader`] to inspect committed Position state
//! without going through the VM session cache.
//!
//! Values are stored decoded (`NativePosition`) inside `UserPositions`,
//! so this module is decode-free — readers return typed values
//! directly. Decoding happens once at write time in
//! [`NativeStateCommitter`] / cold-load.

#![forbid(unsafe_code)]

use crate::native_state_store::{UserPositionKey, UserPositions};
use aptos_infallible::Mutex;
use aptos_types::state_store::native_position::NativePosition;
use move_core_types::account_address::AccountAddress;
use std::sync::Arc;

/// Process-global handle to the most-recently initialized
/// [`InMemoryNativeStateReader`]. Used by ad-hoc consumers (bench
/// harnesses, debug tools) that don't have a structured path to the
/// AptosDB handle. Replace-on-install semantics support the bench
/// harness opening multiple DB instances per run.
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

/// Read-only validator-side accessor for native-position state. The
/// per-account variant `get_account_positions` is O(L_acct + M_acct)
/// per call (L_acct = layer depth between the account's first write
/// and the top layer; M_acct = positions for that account). The
/// exchange-wide variants walk the layered view and are intended for
/// diagnostics, not the hot path.
///
/// **Freshness vs durability:** `UserPositions` is extended by
/// `commit_native_position` *after* `PositionDb::commit(...)` flushes
/// the chunk to disk, so the layered view always reflects the durable
/// state. A reader observing a write means that write has landed.
///
/// **Snapshot semantics:** each call clones the current top layer
/// once and reads through that view. Concurrent writes land on a new
/// layer above the snapshot and are invisible to the in-flight call.
pub trait NativeStateReader: Send + Sync {
    /// Every account that has at least one position for `exchange`.
    fn iter_position_accounts_for_exchange(&self, exchange: AccountAddress) -> Vec<AccountAddress>;

    /// All Positions belonging to `(exchange, account)`. Each
    /// entry is `(market, position)`.
    fn get_account_positions(
        &self,
        exchange: AccountAddress,
        account: AccountAddress,
    ) -> Vec<(AccountAddress, NativePosition)>;

    /// Count of resident position entries for `exchange`. Default
    /// impl walks `iter_position_accounts_for_exchange` and sums
    /// per-account position counts via `get_account_positions`;
    /// production implementors override for a one-pass walk.
    fn count_positions_for_exchange(&self, exchange: AccountAddress) -> usize {
        self.iter_position_accounts_for_exchange(exchange)
            .into_iter()
            .map(|account| self.get_account_positions(exchange, account).len())
            .sum()
    }
}

/// Default implementation backed by the `UserPositions` attached to
/// `PositionBundle`.
pub struct InMemoryNativeStateReader {
    user_positions: Arc<Mutex<UserPositions>>,
}

impl InMemoryNativeStateReader {
    pub fn new(user_positions: Arc<Mutex<UserPositions>>) -> Self {
        Self { user_positions }
    }

    fn snapshot(&self) -> UserPositions {
        self.user_positions.lock().clone()
    }
}

impl NativeStateReader for InMemoryNativeStateReader {
    fn iter_position_accounts_for_exchange(&self, exchange: AccountAddress) -> Vec<AccountAddress> {
        let snap = self.snapshot();
        let view = snap.top().view_layers_after(snap.family_root());
        view.iter()
            .filter_map(|(position_key, state)| {
                (position_key.exchange == exchange && !state.is_empty())
                    .then_some(position_key.account)
            })
            .collect()
    }

    fn count_positions_for_exchange(&self, exchange: AccountAddress) -> usize {
        let snap = self.snapshot();
        let view = snap.top().view_layers_after(snap.family_root());
        view.iter()
            .filter(|(position_key, _)| position_key.exchange == exchange)
            .map(|(_, state)| state.positions.len())
            .sum()
    }

    fn get_account_positions(
        &self,
        exchange: AccountAddress,
        account: AccountAddress,
    ) -> Vec<(AccountAddress, NativePosition)> {
        let snap = self.snapshot();
        let key = UserPositionKey { exchange, account };
        snap.get(&key)
            .map(|us| us.positions.into_iter().collect())
            .unwrap_or_default()
    }
}
