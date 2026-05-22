// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Per-user in-memory residency for native-position data.
//!
//! Single map keyed by `(exchange, account)`. Each user's
//! positions (per-market) live together in one [`UserState`]. The
//! market-set index is intrinsic: it's just `state.positions.keys()`.
//!
//! During block execution this map serves the validator-side native-
//! position resolver entirely from memory; RocksDB is touched only at
//! startup (cold-load) and at commit (persist). On node open the
//! `PositionDb` is scanned, each latest-non-tombstone `position_value`
//! row is grouped by `(exchange, account)` and inserted into the
//! corresponding `UserState.positions` inner map.

#![forbid(unsafe_code)]

use crate::position_metrics::POSITION_IN_MEMORY_COUNT;
use aptos_types::state_store::{
    state_key::{
        inner::{StateKeyInner, TradingNativeKey},
        StateKey,
    },
    state_value::StateValue,
};
use dashmap::DashMap;
use move_core_types::account_address::AccountAddress;
use std::collections::BTreeMap;

/// Per-user identifier within an exchange. A single
/// `(exchange, account)` pair owns one [`UserState`].
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd, Debug)]
pub struct UserKey {
    pub exchange: AccountAddress,
    pub account: AccountAddress,
}

/// All native-mirror data attached to one user. Inner map uses
/// `BTreeMap` so iteration is sorted (required for cross-validator
/// determinism).
#[derive(Default, Debug, Clone)]
pub struct UserState {
    /// `market -> StateValue` (compact-binary `NativePosition` payload).
    pub positions: BTreeMap<AccountAddress, StateValue>,
}

impl UserState {
    pub fn is_empty(&self) -> bool {
        self.positions.is_empty()
    }
}

/// In-memory store maintained on every validator and RPC-serving
/// fullnode that participates in native-position queries.
pub struct NativeStateStore {
    /// `(exchange, account) -> UserState`. Per-user shard locking
    /// via DashMap; inner BTreeMap is sorted for deterministic
    /// iteration.
    pub users: DashMap<UserKey, UserState>,
}

impl Default for NativeStateStore {
    fn default() -> Self {
        Self::empty()
    }
}

impl NativeStateStore {
    pub fn empty() -> Self {
        Self {
            users: DashMap::new(),
        }
    }

    /// Apply a committed Position write under
    /// `(exchange, account, market)`. Inserts/replaces or removes,
    /// and drops the user entry entirely when the inner map goes
    /// empty.
    pub fn apply_position_write(
        &self,
        exchange: AccountAddress,
        account: AccountAddress,
        market: AccountAddress,
        value: Option<StateValue>,
    ) {
        let key = UserKey { exchange, account };
        match value {
            Some(sv) => {
                let mut entry = self.users.entry(key).or_default();
                let was_new = entry.positions.insert(market, sv).is_none();
                if was_new {
                    POSITION_IN_MEMORY_COUNT.inc();
                }
            },
            None => {
                // Single critical section: the Entry holds the shard
                // write lock for the whole remove + empty-check +
                // remove-entry sequence, so a concurrent insert under
                // the same key cannot interleave.
                use dashmap::mapref::entry::Entry;
                if let Entry::Occupied(mut entry) = self.users.entry(key) {
                    if entry.get_mut().positions.remove(&market).is_some() {
                        POSITION_IN_MEMORY_COUNT.dec();
                    }
                    if entry.get().is_empty() {
                        entry.remove();
                    }
                }
            },
        }
    }

    /// Return the count of resident position entries grouped by
    /// `exchange`. Used by the startup consistency check to
    /// compare against per-exchange `AggregatorV2` values.
    pub fn positions_per_exchange(&self) -> std::collections::HashMap<AccountAddress, u64> {
        let mut out = std::collections::HashMap::new();
        for entry in self.users.iter() {
            let exchange = entry.key().exchange;
            *out.entry(exchange).or_insert(0) += entry.value().positions.len() as u64;
        }
        out
    }

    /// Populate `users` from an iterator of `(StateKey, StateValue)`
    /// pairs at the snapshot version. Caller is expected to source
    /// the pairs by walking the position JMT (see
    /// [`crate::position_merkle_db::PositionMerkleDb::iter_active_leaves`])
    /// and looking up values by hash in `position_db`. Non-Position
    /// state keys are rejected.
    pub fn populate_from_rows<I>(&self, rows: I) -> Result<usize, PopulateError>
    where
        I: IntoIterator<Item = (StateKey, StateValue)>,
    {
        let mut count = 0usize;
        for (state_key, state_value) in rows {
            let (exchange, account, market) = match state_key.inner() {
                StateKeyInner::TradingNative(TradingNativeKey::Position {
                    exchange,
                    account,
                    market,
                }) => (*exchange, *account, *market),
                _ => return Err(PopulateError::WrongTag(0)),
            };
            self.users
                .entry(UserKey { exchange, account })
                .or_default()
                .positions
                .insert(market, state_value);
            count += 1;
        }
        Ok(count)
    }
}

/// Result of `verify_startup_consistency`: per-exchange
/// (in_memory_count, expected_count_from_aggregator) for each
/// exchange where the two diverge. Empty on a clean load.
pub type ConsistencyMismatches = Vec<(AccountAddress, u64, u64)>;

/// Compare per-exchange position counts in the in-memory store
/// against the per-exchange `position_count` aggregators in main
/// state.
pub fn verify_startup_consistency(
    store: &NativeStateStore,
    expected_counts: &std::collections::HashMap<AccountAddress, u64>,
) -> ConsistencyMismatches {
    let in_memory = store.positions_per_exchange();
    let mut mismatches = Vec::new();
    for (exchange, in_mem_count) in &in_memory {
        let expected = expected_counts.get(exchange).copied().unwrap_or(0);
        if *in_mem_count != expected {
            mismatches.push((*exchange, *in_mem_count, expected));
        }
    }
    for (exchange, expected) in expected_counts {
        if !in_memory.contains_key(exchange) && *expected != 0 {
            mismatches.push((*exchange, 0, *expected));
        }
    }
    mismatches
}

/// Decoded form of an encoded `TradingNativeKey::Position` byte string.
/// Used by `populate_from_rows` and surfaced via
/// `decode_position_state_key_pub` for state-sync apply paths.
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd, Debug)]
pub struct DecodedPositionKey {
    pub exchange: AccountAddress,
    pub account: AccountAddress,
    pub market: AccountAddress,
}

/// Failure modes for the cold-load + state-sync apply paths.
#[derive(Debug, thiserror::Error)]
pub enum PopulateError {
    #[error("encoded native StateKey has unexpected length {0}")]
    BadLength(usize),
    #[error("encoded native StateKey has wrong tag 0x{0:02x}")]
    WrongTag(u8),
    #[error("encoded native StateKey has invalid AccountAddress field")]
    BadAccountAddress,
}

/// Decode the 98-byte
/// `[tag=2][sub_tag=0][exchange:32][account:32][market:32]` key
/// produced by encoding a
/// `StateKeyInner::TradingNative(TradingNativeKey::Position { .. })`.
/// Strict: rejects wrong-length input, wrong umbrella tag, or wrong
/// sub-tag.
fn decode_position_state_key(bytes: &[u8]) -> Result<DecodedPositionKey, PopulateError> {
    const ADDR: usize = AccountAddress::LENGTH;
    const EXPECTED_LEN: usize = 2 + ADDR * 3;
    if bytes.len() != EXPECTED_LEN {
        return Err(PopulateError::BadLength(bytes.len()));
    }
    if bytes[0] != 2 {
        return Err(PopulateError::WrongTag(bytes[0]));
    }
    if bytes[1] != 0 {
        return Err(PopulateError::WrongTag(bytes[1]));
    }
    let exchange = AccountAddress::from_bytes(&bytes[2..2 + ADDR])
        .map_err(|_| PopulateError::BadAccountAddress)?;
    let account = AccountAddress::from_bytes(&bytes[2 + ADDR..2 + ADDR * 2])
        .map_err(|_| PopulateError::BadAccountAddress)?;
    let market = AccountAddress::from_bytes(&bytes[2 + ADDR * 2..EXPECTED_LEN])
        .map_err(|_| PopulateError::BadAccountAddress)?;
    Ok(DecodedPositionKey {
        exchange,
        account,
        market,
    })
}

/// Public re-export of the encoded-key decoder for state-sync apply
/// paths and other downstream modules that handle position chunks.
pub fn decode_position_state_key_pub(bytes: &[u8]) -> Result<DecodedPositionKey, PopulateError> {
    decode_position_state_key(bytes)
}
