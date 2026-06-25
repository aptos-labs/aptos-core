// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Per-account layered position index.
//!
//! `UserPositions` is the in-memory speculative view keyed by
//! `(exchange, account)` and carries the *decoded* `UserPositionState`
//! per layer.
//!
//! **Values are stored as `Arc<UserPositionState>` in the layered
//! map.** Layer construction and rebase only refcount-bump untouched
//! accounts; deep-cloning the inner `BTreeMap` happens once per
//! actually-written account on the write path.
//!
//! **Chain growth bounded by periodic rebase.** `family_root` is
//! pinned so reads can view-from-root regardless of chain depth (the
//! scanner's hot path stays decode-free with no durable fallback).
//! Unbounded growth is avoided via [`UserPositions::rebase`] —
//! triggered by the merkle batch committer when a snapshot lands.
//! Rebase walks the current top, collects the full live state (one
//! `Arc` clone per account, no value copy), and seeds a fresh family
//! with one layer holding it; the old family drops as soon as
//! outstanding speculative references release.

#![forbid(unsafe_code)]

use aptos_experimental_layered_map::MapLayer;
use aptos_types::{
    account_address::AccountAddress,
    state_store::{
        native_position::NativePosition,
        state_key::{
            inner::{StateKeyInner, TradingNativeKey},
            StateKey,
        },
        state_value::StateValue,
    },
    transaction::Version,
};
use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
};

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd, Debug)]
pub struct UserPositionKey {
    pub exchange: AccountAddress,
    pub account: AccountAddress,
}

/// `BTreeMap` so iteration is sorted for cross-validator determinism.
///
/// Values are stored decoded (`NativePosition`), not as `StateValue`
/// bytes. Validator-side scanners walk the layered view frequently
/// for risk / ADL — we pay one BCS decode at commit / cold-load time
/// and zero decodes per read.
#[derive(Default, Debug, Clone)]
pub struct UserPositionState {
    pub positions: BTreeMap<AccountAddress, NativePosition>,
}

impl UserPositionState {
    pub fn is_empty(&self) -> bool {
        self.positions.is_empty()
    }
}

/// Multi-version layered per-account position index. Each chunk
/// pushes one new layer; speculative branches Arc-drop the layer
/// without disturbing ancestors. Layer values are
/// `Arc<UserPositionState>` so untouched accounts cost a refcount
/// bump per layer build, not a deep copy.
///
/// `family_root` keeps the family-root layer alive so `get` /
/// full-walk views can `view_layers_after(family_root)` and see the
/// whole chain — without that pinned reference the parent `Weak`
/// links could be dropped and `LayeredMap::get` would lose visibility
/// into older layers.
#[derive(Clone, Debug)]
pub struct UserPositions {
    next_version: Version,
    family_root: Arc<MapLayer<UserPositionKey, Arc<UserPositionState>>>,
    top: MapLayer<UserPositionKey, Arc<UserPositionState>>,
}

impl UserPositions {
    pub fn new_empty(family: &'static str) -> Self {
        let root = MapLayer::new_family(family);
        Self {
            next_version: 0,
            family_root: Arc::new(root.clone()),
            top: root,
        }
    }

    pub fn new_at_version(version: Option<Version>, family: &'static str) -> Self {
        let root = MapLayer::new_family(family);
        Self {
            next_version: version.map_or(0, |v| v + 1),
            family_root: Arc::new(root.clone()),
            top: root,
        }
    }

    pub fn next_version(&self) -> Version {
        self.next_version
    }

    pub fn version(&self) -> Option<Version> {
        self.next_version.checked_sub(1)
    }

    pub fn family_root(&self) -> &MapLayer<UserPositionKey, Arc<UserPositionState>> {
        &self.family_root
    }

    pub fn top(&self) -> &MapLayer<UserPositionKey, Arc<UserPositionState>> {
        &self.top
    }

    pub fn is_descendant_of(&self, rhs: &Self) -> bool {
        self.top.is_descendant_of(&rhs.top)
    }

    /// Layered read: returns the top-most `UserPositionState` for
    /// `key` reachable from the family root. The returned `Arc` is
    /// shared with the layer that owns the value — callers wanting
    /// to mutate must `Arc::make_mut` (or build a fresh state).
    pub fn get(&self, key: &UserPositionKey) -> Option<Arc<UserPositionState>> {
        self.top.view_layers_after(&self.family_root).get(key)
    }

    /// Seed a single base layer above the family root with `seed`.
    /// Used by cold-load to hydrate at startup. `next_version` is
    /// unchanged (set by the caller via `new_at_version`).
    pub fn with_seeded_base(
        self,
        seed: Vec<(UserPositionKey, Arc<UserPositionState>)>,
    ) -> Self {
        let top = self
            .top
            .view_layers_after(&self.family_root)
            .new_layer(&seed);
        Self {
            next_version: self.next_version,
            family_root: self.family_root,
            top,
        }
    }

    /// Collapse the chain into a fresh `MapLayer` family holding
    /// the current full state in a single layer. `next_version` is
    /// preserved. The old family becomes unreachable from the
    /// returned value; it drops as soon as outstanding speculative
    /// references (e.g. per-block `UserPositions` in
    /// `PartialStateComputeResult`) release.
    ///
    /// Untouched accounts cost only an `Arc::clone` here — the inner
    /// `BTreeMap<MarketAddress, NativePosition>` is not duplicated.
    /// Touched accounts already paid their inner-map cost on the
    /// write path; this walk shares those same `Arc`s.
    ///
    /// Called by the merkle batch committer when a snapshot lands —
    /// keeps in-memory chain depth bounded by snapshot cadence while
    /// preserving the "in-memory holds all data" invariant the
    /// scanner reads rely on (no durable fallback needed).
    pub fn rebase(&self) -> Self {
        let entries: Vec<(UserPositionKey, Arc<UserPositionState>)> =
            self.top.view_layers_after(&self.family_root).iter().collect();
        Self::new_at_version(self.version(), "position").with_seeded_base(entries)
    }

    /// Push a new layer atop `self`. The new layer's base is
    /// anchored at `family_root` so reader paths viewing from
    /// `family_root` see the whole chain across any number of
    /// extends.
    pub fn extend(
        &self,
        new_version: Version,
        updates: Vec<(UserPositionKey, Arc<UserPositionState>)>,
    ) -> Self {
        let top = self
            .top
            .view_layers_after(&self.family_root)
            .new_layer(&updates);
        Self {
            next_version: new_version + 1,
            family_root: Arc::clone(&self.family_root),
            top,
        }
    }
}

/// Streams JMT rows into the `(UserPositionKey, Arc<UserPositionState>)`
/// seed used to initialize the base layer of a [`UserPositions`] at
/// startup. Memory is bounded by the live position set — one
/// decoded row at a time, no double-buffering of the durable
/// snapshot.
pub fn decode_rows_to_user_position_states<I>(
    rows: I,
) -> crate::Result<Vec<(UserPositionKey, Arc<UserPositionState>)>>
where
    I: IntoIterator<Item = crate::Result<(StateKey, StateValue)>>,
{
    let mut by_account: HashMap<UserPositionKey, UserPositionState> = HashMap::new();
    for row in rows {
        let (state_key, state_value) = row?;
        let (exchange, account, market) = match state_key.inner() {
            StateKeyInner::TradingNative(TradingNativeKey::Position {
                exchange,
                account,
                market,
            }) => (*exchange, *account, *market),
            other => {
                return Err(crate::AptosDbError::Other(format!(
                    "non-Position native StateKey in position snapshot: {other:?}"
                )));
            },
        };
        let position = NativePosition::deserialize(state_value.bytes()).map_err(|e| {
            crate::AptosDbError::Other(format!(
                "native position value at startup failed to decode: {e}"
            ))
        })?;
        by_account
            .entry(UserPositionKey { exchange, account })
            .or_default()
            .positions
            .insert(market, position);
    }
    Ok(by_account.into_iter().map(|(k, v)| (k, Arc::new(v))).collect())
}

/// Decoded per-tx Position write captured by the commit applier.
/// The caller groups by `position_key`, reads the previous
/// `UserPositionState` from the layered view, applies the writes,
/// and pushes the resulting per-account updates as the next layer.
#[derive(Clone, Debug)]
pub struct PositionWrite {
    pub position_key: UserPositionKey,
    pub market: AccountAddress,
    pub value: Option<NativePosition>,
}

/// Collapse a chunk's `PositionWrite` stream into one
/// `Arc<UserPositionState>` per touched `UserPositionKey`, reading
/// the base from `current` (the previous committed layer). Writes
/// apply in arrival order, latest-wins per `(account, market)`.
///
/// We deep-clone the inner `BTreeMap` once per touched account (when
/// the prior state is shared with the layered map). Untouched
/// accounts pay nothing — they stay in their existing layer as
/// shared `Arc`s.
pub fn materialize_user_position_updates(
    current: &UserPositions,
    writes: Vec<PositionWrite>,
) -> Vec<(UserPositionKey, Arc<UserPositionState>)> {
    use std::collections::hash_map::Entry;
    let mut by_account: HashMap<UserPositionKey, UserPositionState> = HashMap::new();
    for w in writes {
        let entry = match by_account.entry(w.position_key) {
            Entry::Vacant(v) => v.insert(
                current
                    .get(&w.position_key)
                    .map(|arc| (*arc).clone())
                    .unwrap_or_default(),
            ),
            Entry::Occupied(o) => o.into_mut(),
        };
        match w.value {
            Some(pos) => {
                entry.positions.insert(w.market, pos);
            },
            None => {
                entry.positions.remove(&w.market);
            },
        }
    }
    by_account
        .into_iter()
        .map(|(k, v)| (k, Arc::new(v)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn addr(byte: u8) -> AccountAddress {
        let mut a = [0u8; AccountAddress::LENGTH];
        a[AccountAddress::LENGTH - 1] = byte;
        AccountAddress::new(a)
    }

    fn position(size: u64) -> NativePosition {
        NativePosition::PerpV1 {
            size,
            is_long: true,
            entry_px_times_size_sum: 0,
            avg_acquire_entry_px: 0,
            user_leverage: 1,
            is_isolated: false,
            funding_index_at_last_update: 0,
            unrealized_funding_amount_before_last_update: 0,
            timestamp: 0,
        }
    }

    fn user_state(market: AccountAddress, size: u64) -> Arc<UserPositionState> {
        let mut positions = BTreeMap::new();
        positions.insert(market, position(size));
        Arc::new(UserPositionState { positions })
    }

    #[test]
    fn reads_through_family_root_after_multiple_extends() {
        let key_a = UserPositionKey {
            exchange: addr(1),
            account: addr(2),
        };
        let key_b = UserPositionKey {
            exchange: addr(1),
            account: addr(3),
        };
        let market = addr(9);

        let v0 = UserPositions::new_empty("test").extend(0, vec![(key_a, user_state(market, 100))]);
        let v1 = v0.extend(1, vec![(key_b, user_state(market, 200))]);
        let v2 = v1.extend(2, vec![(key_a, user_state(market, 300))]);

        assert_eq!(
            v2.get(&key_a).map(|s| s.positions[&market].size()),
            Some(300)
        );
        assert_eq!(
            v2.get(&key_b).map(|s| s.positions[&market].size()),
            Some(200)
        );
    }

    #[test]
    fn cold_load_seed_then_extend() {
        let key_a = UserPositionKey {
            exchange: addr(1),
            account: addr(2),
        };
        let key_b = UserPositionKey {
            exchange: addr(1),
            account: addr(3),
        };
        let market = addr(9);

        let seeded = UserPositions::new_at_version(Some(10), "test")
            .with_seeded_base(vec![(key_a, user_state(market, 50))]);
        let v11 = seeded.extend(11, vec![(key_b, user_state(market, 60))]);

        assert_eq!(
            v11.get(&key_a).map(|s| s.positions[&market].size()),
            Some(50)
        );
        assert_eq!(
            v11.get(&key_b).map(|s| s.positions[&market].size()),
            Some(60)
        );
    }

    #[test]
    fn rebase_collapses_chain_preserving_content() {
        let key_a = UserPositionKey {
            exchange: addr(1),
            account: addr(2),
        };
        let key_b = UserPositionKey {
            exchange: addr(1),
            account: addr(3),
        };
        let market = addr(9);

        // Build a chain of three layers.
        let v0 = UserPositions::new_empty("test").extend(0, vec![(key_a, user_state(market, 100))]);
        let v1 = v0.extend(1, vec![(key_b, user_state(market, 200))]);
        let v2 = v1.extend(2, vec![(key_a, user_state(market, 300))]);

        let rebased = v2.rebase();

        // Same per-key content visible from the rebased view.
        assert_eq!(
            rebased.get(&key_a).map(|s| s.positions[&market].size()),
            Some(300)
        );
        assert_eq!(
            rebased.get(&key_b).map(|s| s.positions[&market].size()),
            Some(200)
        );

        // Fresh family — not a descendant of v2.
        assert!(!rebased.is_descendant_of(&v2));
        // next_version preserved.
        assert_eq!(rebased.next_version(), v2.next_version());
    }

    #[test]
    fn materialize_collapses_chunk_writes_per_account() {
        let key = UserPositionKey {
            exchange: addr(1),
            account: addr(2),
        };
        let market_x = addr(7);
        let market_y = addr(8);

        let current = UserPositions::new_empty("test")
            .extend(0, vec![(key, user_state(market_x, 100))]);

        let writes = vec![
            PositionWrite {
                position_key: key,
                market: market_y,
                value: Some(position(50)),
            },
            PositionWrite {
                position_key: key,
                market: market_x,
                value: Some(position(150)),
            },
        ];
        let updates = materialize_user_position_updates(&current, writes);
        assert_eq!(updates.len(), 1);
        let (k, state) = &updates[0];
        assert_eq!(*k, key);
        assert_eq!(state.positions[&market_x].size(), 150);
        assert_eq!(state.positions[&market_y].size(), 50);
    }

    #[test]
    fn rebase_shares_value_arcs_for_untouched_accounts() {
        let key_touched = UserPositionKey {
            exchange: addr(1),
            account: addr(2),
        };
        let key_quiet = UserPositionKey {
            exchange: addr(1),
            account: addr(3),
        };
        let market = addr(9);

        let v0 = UserPositions::new_empty("test").extend(
            0,
            vec![
                (key_touched, user_state(market, 100)),
                (key_quiet, user_state(market, 200)),
            ],
        );
        let v1 = v0.extend(1, vec![(key_touched, user_state(market, 300))]);

        let before = v1.get(&key_quiet).expect("quiet must be present");
        let rebased = v1.rebase();
        let after = rebased.get(&key_quiet).expect("quiet must survive rebase");

        // Untouched account's Arc is the *same* allocation as before
        // the rebase — rebase did not deep-clone its inner BTreeMap.
        assert!(Arc::ptr_eq(&before, &after));
    }
}
