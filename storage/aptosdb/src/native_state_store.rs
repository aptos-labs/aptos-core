// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

#![forbid(unsafe_code)]

use aptos_experimental_layered_map::MapLayer;
use aptos_types::{
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
use move_core_types::account_address::AccountAddress;
use std::{collections::BTreeMap, sync::Arc};

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd, Debug)]
pub struct UserPositionKey {
    pub exchange: AccountAddress,
    pub account: AccountAddress,
}

/// `BTreeMap` so iteration is sorted for cross-validator determinism.
///
/// Values are stored decoded (`NativePosition`), not as `StateValue`
/// bytes. The reader is the hot path — validator-side scanners walk
/// the layered view frequently for risk / ADL — so we pay one BCS
/// decode at commit / cold-load time and zero decodes per read.
#[derive(Default, Debug, Clone)]
pub struct UserPositionState {
    pub positions: BTreeMap<AccountAddress, NativePosition>,
}

impl UserPositionState {
    pub fn is_empty(&self) -> bool {
        self.positions.is_empty()
    }
}

/// Multi-version layered per-account position index. A single
/// `MapLayer<UserPositionKey, UserPositionState>` chain: each chunk pushes one new
/// layer; speculative branches Arc-drop the layer without disturbing
/// ancestors.
///
/// `family_root` keeps the family-root layer alive so `get` /
/// full-walk views can `view_layers_after(root)` and see the whole
/// chain — without that anchor, the parent `Weak` references could
/// be dropped and `LayeredMap::get` would lose visibility into older
/// layers.
#[derive(Clone, Debug)]
pub struct UserPositions {
    next_version: Version,
    family_root: Arc<MapLayer<UserPositionKey, UserPositionState>>,
    top: MapLayer<UserPositionKey, UserPositionState>,
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

    pub fn family_root(&self) -> &MapLayer<UserPositionKey, UserPositionState> {
        &self.family_root
    }

    pub fn top(&self) -> &MapLayer<UserPositionKey, UserPositionState> {
        &self.top
    }

    pub fn is_descendant_of(&self, rhs: &Self) -> bool {
        self.top.is_descendant_of(&rhs.top)
    }

    /// Layered read: returns the `UserPositionState` value at the
    /// current top layer, or `None` if the account has no positions
    /// in this chain.
    pub fn get(&self, key: &UserPositionKey) -> Option<UserPositionState> {
        self.top.view_layers_after(&self.family_root).get(key)
    }

    /// Push a single base layer over the family root with `seed` —
    /// used by cold-load to hydrate `UserPositions` at startup. The
    /// resulting state's `next_version` is unchanged.
    ///
    /// The new layer's `base_layer` field is anchored at
    /// `family_root` (not at the previous top), so subsequent
    /// `top.view_layers_after(&family_root)` calls in readers and
    /// `get` always pass `can_view_after`.
    pub fn with_seeded_base(
        self,
        _seed_version: Version,
        seed: Vec<(UserPositionKey, UserPositionState)>,
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

    /// Push a new layer atop the current top. The new layer is
    /// anchored at `family_root` (its `base_layer` field equals the
    /// family root's layer = 0), so reader paths viewing from
    /// `family_root` see the whole chain across any number of
    /// extends.
    pub fn extend(
        &self,
        new_version: Version,
        updates: Vec<(UserPositionKey, UserPositionState)>,
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

/// Streams JMT rows into the `(UserPositionKey, UserPositionState)` seed used to
/// initialize the base layer of a [`UserPositions`] at startup. Memory
/// usage is bounded by the live position set — one decoded row at a
/// time, no double-buffering of the durable snapshot.
pub fn decode_rows_to_user_position_states<I>(rows: I) -> Result<Vec<(UserPositionKey, UserPositionState)>, PopulateError>
where
    I: IntoIterator<Item = aptos_storage_interface::Result<(StateKey, StateValue)>>,
{
    let mut by_account: BTreeMap<UserPositionKey, UserPositionState> = BTreeMap::new();
    for row in rows {
        let (state_key, state_value) = row.map_err(|e| PopulateError::Iter(e.to_string()))?;
        let (exchange, account, market) = match state_key.inner() {
            StateKeyInner::TradingNative(TradingNativeKey::Position {
                exchange,
                account,
                market,
            }) => (*exchange, *account, *market),
            _ => return Err(PopulateError::WrongTag(0)),
        };
        let position = NativePosition::deserialize(state_value.bytes())
            .map_err(|e| PopulateError::BadValue(e.to_string()))?;
        by_account
            .entry(UserPositionKey { exchange, account })
            .or_default()
            .positions
            .insert(market, position);
    }
    Ok(by_account.into_iter().collect())
}

#[derive(Debug, thiserror::Error)]
pub enum PopulateError {
    #[error("encoded native StateKey has unexpected length {0}")]
    BadLength(usize),
    #[error("encoded native StateKey has wrong tag 0x{0:02x}")]
    WrongTag(u8),
    #[error("encoded native StateKey has invalid AccountAddress field")]
    BadAccountAddress,
    #[error("native position value failed to decode: {0}")]
    BadValue(String),
    #[error("native position iterator error: {0}")]
    Iter(String),
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

    fn user_state(market: AccountAddress, size: u64) -> UserPositionState {
        let mut positions = BTreeMap::new();
        positions.insert(market, position(size));
        UserPositionState { positions }
    }

    /// After multiple `extend` calls the top layer must still be
    /// view-able from `family_root` — otherwise the reader paths
    /// (`get`, `iter`-based exchange walks) would panic on
    /// `can_view_after`.
    #[test]
    fn reads_through_family_root_after_multiple_extends() {
        let mirror = UserPositions::new_empty("test");
        let key_a = UserPositionKey {
            exchange: addr(1),
            account: addr(2),
        };
        let key_b = UserPositionKey {
            exchange: addr(1),
            account: addr(3),
        };
        let market = addr(9);

        let v0 = mirror.extend(0, vec![(key_a, user_state(market, 100))]);
        let v1 = v0.extend(1, vec![(key_b, user_state(market, 200))]);
        let v2 = v1.extend(2, vec![(key_a, user_state(market, 300))]);

        // Per-account read at the latest layer.
        assert_eq!(
            v2.get(&key_a).map(|s| s.positions[&market].size()),
            Some(300)
        );
        assert_eq!(
            v2.get(&key_b).map(|s| s.positions[&market].size()),
            Some(200)
        );

        // Full-walk view from the family root: layer-0 anchor must
        // still see all three layers' writes (latest-wins per key).
        let view = v2.top().view_layers_after(v2.family_root());
        let mut seen: Vec<(UserPositionKey, u64)> = view
            .iter()
            .map(|(k, s)| (k, s.positions[&market].size()))
            .collect();
        seen.sort_by_key(|(k, _)| *k);
        assert_eq!(seen, vec![(key_a, 300), (key_b, 200)]);
    }

    /// `with_seeded_base` followed by extends must also stay
    /// view-able from `family_root`.
    #[test]
    fn cold_load_seed_then_extend_views_from_family_root() {
        let market = addr(9);
        let key_a = UserPositionKey {
            exchange: addr(1),
            account: addr(2),
        };
        let key_b = UserPositionKey {
            exchange: addr(1),
            account: addr(3),
        };

        let seeded = UserPositions::new_at_version(Some(10), "test")
            .with_seeded_base(10, vec![(key_a, user_state(market, 50))]);
        let v11 = seeded.extend(11, vec![(key_b, user_state(market, 60))]);

        assert_eq!(
            v11.get(&key_a).map(|s| s.positions[&market].size()),
            Some(50)
        );
        assert_eq!(
            v11.get(&key_b).map(|s| s.positions[&market].size()),
            Some(60)
        );

        let view = v11.top().view_layers_after(v11.family_root());
        let n = view.iter().count();
        assert_eq!(n, 2);
    }
}
