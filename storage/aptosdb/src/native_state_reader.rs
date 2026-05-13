// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Validator-side reader API for native-position data.
//!
//! At block boundaries off-Move consumers query the in-memory
//! [`NativeStateStore`] via [`NativeStateReader`] to inspect the
//! committed Position set without going through the VM session
//! cache.
//!
//! This module owns the trait and a default implementation that
//! decodes the durable byte payloads (compact-binary codec from
//! `aptos-position-natives`) into the in-memory `NativePosition`
//! struct. The decoder is infallible against valid payloads written
//! via the standard codec; corrupt entries are silently skipped.

#![forbid(unsafe_code)]

use crate::{
    native_state_store::{NativeStateStore, UserKey},
    position_metrics::POSITION_DECODE_ERRORS,
    position_pruner::PositionPruner,
};
use aptos_position_natives::{NativePosition, PositionKey};
use move_core_types::account_address::AccountAddress;
use std::sync::{Arc, Mutex};

/// Centralize the silent-skip decode pattern so every reader site
/// gets the same observability. Returns `Some(pos)` on success or
/// `None` after bumping `POSITION_DECODE_ERRORS` — the caller is
/// expected to filter `None` out of the result. Logging is left to
/// the caller's discretion (most reader paths run on every block,
/// per-row logging would be too noisy; the counter is the canonical
/// signal).
fn decode_or_count(bytes: &[u8]) -> Option<NativePosition> {
    match NativePosition::deserialize(bytes) {
        Ok(pos) => Some(pos),
        Err(_) => {
            POSITION_DECODE_ERRORS.inc();
            None
        },
    }
}

/// Process-global handle to the most-recently initialized
/// [`InMemoryNativeStateReader`]. Used by ad-hoc consumers (bench
/// harnesses, debug tools) that don't have a structured path to the
/// AptosDB handle. Replace-on-install semantics support the bench
/// harness opening multiple DB instances per run.
static LATEST_READER: Mutex<Option<Arc<InMemoryNativeStateReader>>> = Mutex::new(None);

pub fn install_global_reader(reader: Arc<InMemoryNativeStateReader>) {
    if let Ok(mut guard) = LATEST_READER.lock() {
        *guard = Some(reader);
    }
}

pub fn global_reader() -> Option<Arc<InMemoryNativeStateReader>> {
    LATEST_READER.lock().ok().and_then(|g| g.clone())
}

/// Process-global handle to the position pruner. Installed alongside
/// the reader by `AptosDB::init_native_position`. Production pruning
/// runs through `LedgerPrunerManager` — this raw handle is *not* the
/// production scheduler. It exists so ad-hoc consumers (bench
/// harnesses, debug tools) can synchronously drain stale rows at
/// well-defined boundaries without threading the AptosDB through.
static LATEST_PRUNERS: Mutex<Option<Arc<PositionPruner>>> = Mutex::new(None);

pub fn install_global_pruners(position: Arc<PositionPruner>) {
    if let Ok(mut guard) = LATEST_PRUNERS.lock() {
        *guard = Some(position);
    }
}

pub fn global_pruners() -> Option<Arc<PositionPruner>> {
    LATEST_PRUNERS.lock().ok().and_then(|g| g.clone())
}

/// Read-only validator-side accessor for committed native-position
/// state. The per-account variants (`get_account_positions`,
/// `iter_users_for_exchange`) are O(M_u) per call — they consult
/// the per-user store directly. The exchange-wide
/// `iter_positions_for_exchange` variant is O(N) and intended for
/// diagnostics, not the hot path.
pub trait NativeStateReader: Send + Sync {
    /// Every committed Position for `exchange`. Output order is
    /// unspecified. Diagnostic-only: prefer `iter_users_for_exchange`
    /// + `get_account_positions` for per-block walks.
    fn iter_positions_for_exchange(
        &self,
        exchange: AccountAddress,
    ) -> Vec<(PositionKey, NativePosition)>;

    /// Every account that has at least one position for
    /// `exchange`.
    fn iter_users_for_exchange(&self, exchange: AccountAddress) -> Vec<AccountAddress>;

    /// All Positions belonging to `(exchange, account)`. Each
    /// entry is `(market, position)`.
    fn get_account_positions(
        &self,
        exchange: AccountAddress,
        account: AccountAddress,
    ) -> Vec<(AccountAddress, NativePosition)>;

    /// Count of resident position entries for `exchange` without
    /// materializing a Vec or decoding the payloads. Default impl
    /// walks the full iterator (correct but expensive); production
    /// implementors override for O(users) cost.
    fn count_positions_for_exchange(&self, exchange: AccountAddress) -> usize {
        self.iter_positions_for_exchange(exchange).len()
    }
}

/// Default implementation backed by the [`NativeStateStore`]
/// attached to `AptosDB`.
pub struct InMemoryNativeStateReader {
    store: Arc<NativeStateStore>,
}

impl InMemoryNativeStateReader {
    pub fn new(store: Arc<NativeStateStore>) -> Self {
        Self { store }
    }
}

impl NativeStateReader for InMemoryNativeStateReader {
    fn iter_positions_for_exchange(
        &self,
        exchange: AccountAddress,
    ) -> Vec<(PositionKey, NativePosition)> {
        let mut out = Vec::new();
        for entry in self.store.users.iter() {
            let key = entry.key();
            if key.exchange != exchange {
                continue;
            }
            for (market, sv) in entry.value().positions.iter() {
                if let Some(pos) = decode_or_count(sv.bytes()) {
                    out.push((
                        PositionKey {
                            exchange: key.exchange,
                            account: key.account,
                            market: *market,
                        },
                        pos,
                    ));
                }
            }
        }
        out
    }

    fn iter_users_for_exchange(&self, exchange: AccountAddress) -> Vec<AccountAddress> {
        let mut out = Vec::new();
        for entry in self.store.users.iter() {
            let key = entry.key();
            if key.exchange == exchange {
                out.push(key.account);
            }
        }
        out
    }

    fn count_positions_for_exchange(&self, exchange: AccountAddress) -> usize {
        let mut total = 0usize;
        for entry in self.store.users.iter() {
            if entry.key().exchange == exchange {
                total += entry.value().positions.len();
            }
        }
        total
    }

    fn get_account_positions(
        &self,
        exchange: AccountAddress,
        account: AccountAddress,
    ) -> Vec<(AccountAddress, NativePosition)> {
        let key = UserKey { exchange, account };
        let Some(entry) = self.store.users.get(&key) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for (market, sv) in entry.value().positions.iter() {
            if let Some(pos) = decode_or_count(sv.bytes()) {
                out.push((*market, pos));
            }
        }
        out
    }
}
