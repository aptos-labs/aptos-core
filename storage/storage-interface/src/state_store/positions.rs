// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Native position index: a complete [`PositionBase`] with a speculative
//! [`PositionOverlay`] over it, both sharded on `(exchange, account)`.
//!
//! Keyed as the write set is, so writes apply blind — nothing reads prior
//! state in order to write.
//!
//! The base nests by account and sorts exchange-major, so risk's grouping
//! is structural and a per-exchange scan is a range. The overlay is a
//! `MapLayer`, hash-ordered and neither nested nor sorted; it holds only
//! writes since the base, few enough to group on the fly.

#![forbid(unsafe_code)]

use aptos_experimental_layered_map::MapLayer;
use aptos_infallible::RwLock;
use aptos_types::{
    account_address::AccountAddress,
    state_store::{
        native_position::NativePosition,
        state_key::{
            inner::{StateKeyInner, TradingNativeKey},
            StateKey,
        },
        state_value::StateValue,
        NUM_STATE_SHARDS,
    },
    transaction::Version,
};
use arr_macro::arr;
use rayon::prelude::*;
use std::{collections::BTreeMap, sync::Arc};

pub const NUM_POSITION_SHARDS: usize = NUM_STATE_SHARDS;

/// The `(exchange, account)` an account's positions group under.
///
/// **Field order is load-bearing.** The derived `Ord` is exchange-major,
/// which is what lets [`PositionBaseView::accounts_in_exchange`] serve a
/// per-exchange scan as a range rather than a filter over every account
/// on every exchange. Reordering these fields silently turns that range
/// into the wrong set. Guarded by `account_key_orders_by_exchange_first`.
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd, Debug)]
pub struct AccountKey {
    pub exchange: AccountAddress,
    pub account: AccountAddress,
}

/// One position, keyed as the write set keys it.
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd, Debug)]
pub struct PositionKey {
    pub exchange: AccountAddress,
    pub account: AccountAddress,
    pub market: AccountAddress,
}

impl PositionKey {
    pub fn account(&self) -> AccountKey {
        AccountKey {
            exchange: self.exchange,
            account: self.account,
        }
    }
}

/// An account's positions, by market.
pub type AccountPositions = BTreeMap<AccountAddress, NativePosition>;

/// Shard on the account, which is the base's outer key, so an account's
/// positions never split across shards and grouping needs no
/// coordination between them.
pub fn shard_of(account: &AccountKey) -> usize {
    let last = AccountAddress::LENGTH - 1;
    let byte = account.account.as_ref()[last] ^ account.exchange.as_ref()[last];
    usize::from(byte) % NUM_POSITION_SHARDS
}

pub type PositionLayer = MapLayer<PositionKey, Option<NativePosition>>;
pub type ShardedPositionLayers = [PositionLayer; NUM_POSITION_SHARDS];

/// A block's position writes. `None` deletes.
pub type PositionWrites = Vec<(PositionKey, Option<NativePosition>)>;

struct PositionBaseInner {
    version: Option<Version>,
    /// Nested rather than keyed by the flat `PositionKey`: the grouping
    /// risk reads by is then structural, not a consequence of field
    /// declaration order deciding `Ord`. Nesting costs nothing on the
    /// write side because the base is exclusively owned and mutated in
    /// place — unlike the overlay, where a grouped value would have to
    /// be cloned per write because it is shared across versions.
    shards: [BTreeMap<AccountKey, AccountPositions>; NUM_POSITION_SHARDS],
    /// Layer handles at `version` — the view base handed to new blocks.
    layers: ShardedPositionLayers,
}

impl std::fmt::Debug for PositionBaseInner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let len: usize = self.shards.iter().map(|s| s.len()).sum();
        write!(f, "PositionBaseInner(version={:?}, {len})", self.version)
    }
}

/// Resolves overlay misses, as the state KV DB does for main state. Has
/// to be resident and complete because the durable position store is
/// hash-keyed and can enumerate neither an exchange's accounts nor an
/// account's markets.
///
/// Single-version, so it must not move while a block executes: the
/// executor advances it under its execution lock, only to a version the
/// block about to run descends from.
#[derive(Debug)]
pub struct PositionBase {
    inner: RwLock<PositionBaseInner>,
}

/// Guarded view of a [`PositionBase`], scope-bound to a closure so no
/// caller can park a lock and stall folds.
pub struct PositionBaseView<'a> {
    inner: &'a PositionBaseInner,
}

impl PositionBaseView<'_> {
    pub fn version(&self) -> Option<Version> {
        self.inner.version
    }

    pub fn get(&self, key: &PositionKey) -> Option<&NativePosition> {
        self.inner.shards[shard_of(&key.account())]
            .get(&key.account())
            .and_then(|markets| markets.get(&key.market))
    }

    pub fn shard(&self, shard: usize) -> &BTreeMap<AccountKey, AccountPositions> {
        &self.inner.shards[shard]
    }

    /// The account's positions, in market order. `None` when it holds
    /// none — the fold prunes accounts whose last position is deleted,
    /// so an empty entry never lingers.
    pub fn account_positions(&self, account: &AccountKey) -> Option<&AccountPositions> {
        self.inner.shards[shard_of(account)].get(account)
    }

    /// The exchange's accounts within one shard, in account order.
    ///
    /// A range, not a filter: accounts of one exchange are contiguous
    /// because `AccountKey` sorts exchange-major, so this costs what the
    /// exchange holds rather than what every exchange holds together.
    pub fn accounts_in_exchange(
        &self,
        shard: usize,
        exchange: AccountAddress,
    ) -> impl Iterator<Item = (&AccountKey, &AccountPositions)> {
        self.inner.shards[shard]
            .range(
                AccountKey {
                    exchange,
                    account: AccountAddress::ZERO,
                }..,
            )
            .take_while(move |(key, _)| key.exchange == exchange)
    }

    pub fn num_accounts(&self) -> usize {
        self.inner.shards.iter().map(|s| s.len()).sum()
    }

    pub fn num_positions(&self) -> usize {
        self.inner
            .shards
            .iter()
            .map(|s| s.values().map(|markets| markets.len()).sum::<usize>())
            .sum()
    }
}

impl PositionBase {
    pub fn new_empty(family: &'static str) -> Self {
        Self::new_at_version(None, family, Vec::new())
    }

    /// Cold-load entry point: `seed` is the decoded durable snapshot.
    pub fn new_at_version(
        version: Option<Version>,
        family: &'static str,
        seed: Vec<(PositionKey, NativePosition)>,
    ) -> Self {
        let mut shards: [BTreeMap<AccountKey, AccountPositions>; NUM_POSITION_SHARDS] =
            arr![BTreeMap::new(); 16];
        for (key, position) in seed {
            let account = key.account();
            shards[shard_of(&account)]
                .entry(account)
                .or_default()
                .insert(key.market, position);
        }
        Self {
            inner: RwLock::new(PositionBaseInner {
                version,
                shards,
                layers: arr![MapLayer::new_family(family); 16],
            }),
        }
    }

    pub fn version(&self) -> Option<Version> {
        self.inner.read().version
    }

    /// Layer handles at the current base version. New overlays extend
    /// from these, which is what lets layers below them be freed once
    /// the blocks holding older handles are pruned.
    pub fn layers(&self) -> ShardedPositionLayers {
        self.inner.read().layers.clone()
    }

    pub fn read<R>(&self, f: impl FnOnce(&PositionBaseView<'_>) -> R) -> R {
        let guard = self.inner.read();
        f(&PositionBaseView { inner: &guard })
    }

    /// Applies everything written since the base layers, in O(writes
    /// since the last fold). Returns whether the base moved.
    ///
    /// Declines an overlay at or behind the base, or one not descended
    /// from it — checked here under the write lock, so a mis-ordered
    /// caller fails to advance rather than splicing in a discarded
    /// fork's writes.
    pub fn fold(&self, overlay: &PositionOverlay) -> bool {
        let mut inner = self.inner.write();
        if inner.version.is_some() && overlay.next_version() <= inner.version.map_or(0, |v| v + 1) {
            return false;
        }
        if !(0..NUM_POSITION_SHARDS)
            .all(|shard| overlay.tops[shard].is_descendant_of(&inner.layers[shard]))
        {
            return false;
        }
        for shard in 0..NUM_POSITION_SHARDS {
            let view = overlay.tops[shard].view_layers_after(&inner.layers[shard]);
            for (key, write) in view.iter() {
                let account = key.account();
                match write {
                    Some(position) => {
                        inner.shards[shard]
                            .entry(account)
                            .or_default()
                            .insert(key.market, position);
                    },
                    None => {
                        // Drop the account with its last position, or it
                        // lingers empty and reads as a live account.
                        if let Some(markets) = inner.shards[shard].get_mut(&account) {
                            markets.remove(&key.market);
                            if markets.is_empty() {
                                inner.shards[shard].remove(&account);
                            }
                        }
                    },
                }
            }
        }
        inner.layers = (*overlay.tops).clone();
        inner.version = overlay.version();
        true
    }
}

/// Speculative overlay: the position writes since [`PositionBase`]'s
/// version. Each chunk pushes one layer per shard; forks Arc-drop their
/// layers without disturbing ancestors.
#[derive(Clone, Debug)]
pub struct PositionOverlay {
    next_version: Version,
    base: Arc<PositionBase>,
    base_layers: Arc<ShardedPositionLayers>,
    tops: Arc<ShardedPositionLayers>,
}

impl PositionOverlay {
    /// Empty overlay sitting directly on `base`.
    pub fn new_at_base(base: Arc<PositionBase>) -> Self {
        let layers = base.layers();
        Self {
            next_version: base.version().map_or(0, |v| v + 1),
            base,
            base_layers: Arc::new(layers.clone()),
            tops: Arc::new(layers),
        }
    }

    pub fn next_version(&self) -> Version {
        self.next_version
    }

    pub fn version(&self) -> Option<Version> {
        self.next_version.checked_sub(1)
    }

    pub fn base(&self) -> &Arc<PositionBase> {
        &self.base
    }

    pub fn base_layers(&self) -> &ShardedPositionLayers {
        &self.base_layers
    }

    pub fn shards(&self) -> &ShardedPositionLayers {
        &self.tops
    }

    pub fn is_descendant_of(&self, rhs: &Self) -> bool {
        self.tops_descend_from(&rhs.tops)
    }

    /// Whether every shard's top descends from `layers`.
    ///
    /// The single spelling of this check. Shards move in lockstep — every
    /// `extend` builds all of them and `fold` assigns all of them — so
    /// testing one would do, but having one predicate is what keeps a
    /// caller from quietly omitting the check altogether.
    fn tops_descend_from(&self, layers: &ShardedPositionLayers) -> bool {
        (0..NUM_POSITION_SHARDS).all(|shard| self.tops[shard].is_descendant_of(&layers[shard]))
    }

    /// Whether every shard's top descends from the base's layers, the
    /// guard [`PositionBase::fold`] requires.
    ///
    /// Walks the ancestry rather than using `can_view_after`, which
    /// tests only family membership and a layer-number range — an
    /// abandoned sibling fork satisfies both while sharing no ancestry,
    /// and folding one would mix its writes into the certified base.
    pub fn descends_from_base(&self) -> bool {
        self.tops_descend_from(&self.base.layers())
    }

    /// Overlay only — no base fallback and so no base lock. `Some(None)`
    /// is a tombstone.
    pub fn get_in_overlay(&self, key: &PositionKey) -> Option<Option<NativePosition>> {
        let shard = shard_of(&key.account());
        self.tops[shard]
            .view_layers_after(&self.base_layers[shard])
            .get(key)
    }

    /// Overlay first, then the base.
    ///
    /// **Takes the base read lock.** A caller already holding a
    /// [`PositionBaseView`] — anything inside `PositionBase::read`, which
    /// includes the whole of a scoped reader view — must not call this:
    /// re-entering a `std::sync::RwLock` for read on one thread can
    /// deadlock against a queued writer. Use [`Self::get_in_overlay`] and
    /// resolve the miss against the view already in hand.
    pub fn get(&self, key: &PositionKey) -> Option<NativePosition> {
        match self.get_in_overlay(key) {
            Some(write) => write,
            None => self.base.read(|base| base.get(key).cloned()),
        }
    }

    /// Writes since the base — what the overlay shadows.
    ///
    /// Walks layers only and never touches the base, which is what makes
    /// it safe to call from inside `PositionBase::read`. Giving it a base
    /// fallback would deadlock those callers.
    pub fn writes_since_base(&self) -> PositionWrites {
        (0..NUM_POSITION_SHARDS)
            .into_par_iter()
            .flat_map_iter(|shard| {
                self.tops[shard]
                    .view_layers_after(&self.base_layers[shard])
                    .iter()
                    .collect::<Vec<_>>()
            })
            .collect()
    }

    fn split_by_shard(writes: PositionWrites) -> Vec<PositionWrites> {
        let mut per_shard = vec![PositionWrites::new(); NUM_POSITION_SHARDS];
        for (key, write) in writes {
            per_shard[shard_of(&key.account())].push((key, write));
        }
        per_shard
    }

    /// Push one layer per shard atop `self`. Writes apply blind — the
    /// key matches the write set, so there is no prior state to read.
    pub fn extend(&self, new_version: Version, writes: PositionWrites) -> Self {
        // The new layer covers `[self.next_version(), new_version]`, so a
        // lower version means the caller is extending from the wrong
        // parent. `fold` later makes skip/advance decisions off this
        // stamp, so a bad one propagates. Main state guards the same
        // thing in `State::update`.
        assert!(
            new_version >= self.next_version(),
            "extend at version {new_version} but overlay is already at {}",
            self.next_version(),
        );
        // Re-seat onto the base rather than inheriting the parent's
        // floor, or every block keeps the layers from process start alive
        // and nothing is ever freed. Skip it for a top that isn't
        // descended — viewing a fork from a floor it never passed through
        // mixes in the certified branch.
        let base_layers = {
            let current = self.base.layers();
            if self.tops_descend_from(&current) {
                Arc::new(current)
            } else {
                Arc::clone(&self.base_layers)
            }
        };
        let per_shard = Self::split_by_shard(writes);
        let tops: Vec<PositionLayer> = (0..NUM_POSITION_SHARDS)
            .into_par_iter()
            .map(|shard| {
                self.tops[shard]
                    .view_layers_after(&base_layers[shard])
                    .new_layer(&per_shard[shard])
            })
            .collect();
        Self {
            next_version: new_version + 1,
            base: Arc::clone(&self.base),
            base_layers,
            tops: Arc::new(
                tops.try_into()
                    .expect("Known to be NUM_POSITION_SHARDS shards."),
            ),
        }
    }

    /// Same writes, re-seated onto the base's current layers so the
    /// layers below can be freed.
    ///
    /// A no-op when the base is not an ancestor. Re-seating regardless
    /// would pair tops with a floor they never passed through, which
    /// panics in `view_layers_after` when the base is the deeper of the
    /// two and silently mixes branches when they are level.
    pub fn rebased_on_current_base(&self) -> Self {
        let current = self.base.layers();
        if !self.tops_descend_from(&current) {
            return self.clone();
        }
        Self {
            next_version: self.next_version,
            base: Arc::clone(&self.base),
            base_layers: Arc::new(current),
            tops: Arc::clone(&self.tops),
        }
    }
}

/// Decodes a block's native-position write set into the form both the
/// overlay and the durable commit take.
pub fn decode_position_writes<'a, I>(writes: I) -> crate::Result<PositionWrites>
where
    I: IntoIterator<Item = (&'a StateKey, Option<&'a StateValue>)>,
{
    writes
        .into_iter()
        .map(|(state_key, value)| {
            let key = position_key_of(state_key)?;
            let position = value
                .map(|sv| NativePosition::deserialize(sv.bytes()))
                .transpose()
                .map_err(|e| {
                    crate::AptosDbError::Other(format!("native position failed to decode: {e}"))
                })?;
            Ok((key, position))
        })
        .collect()
}

pub fn position_key_of(state_key: &StateKey) -> crate::Result<PositionKey> {
    match state_key.inner() {
        StateKeyInner::TradingNative(TradingNativeKey::Position {
            exchange,
            account,
            market,
        }) => Ok(PositionKey {
            exchange: *exchange,
            account: *account,
            market: *market,
        }),
        other => Err(crate::AptosDbError::Other(format!(
            "non-Position native StateKey: {other:?}"
        ))),
    }
}

/// Streams durable JMT rows into the seed for a [`PositionBase`].
pub fn decode_rows_to_positions<I>(rows: I) -> crate::Result<Vec<(PositionKey, NativePosition)>>
where
    I: IntoIterator<Item = crate::Result<(StateKey, StateValue)>>,
{
    rows.into_iter()
        .map(|row| {
            let (state_key, state_value) = row?;
            let key = position_key_of(&state_key)?;
            let position = NativePosition::deserialize(state_value.bytes()).map_err(|e| {
                crate::AptosDbError::Other(format!(
                    "native position at startup failed to decode: {e}"
                ))
            })?;
            Ok((key, position))
        })
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

    fn addr_n(n: u64) -> AccountAddress {
        let mut a = [0u8; AccountAddress::LENGTH];
        a[AccountAddress::LENGTH - 8..].copy_from_slice(&n.to_be_bytes());
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

    fn key(account: u8, market: u8) -> PositionKey {
        PositionKey {
            exchange: addr(1),
            account: addr(account),
            market: addr(market),
        }
    }

    fn base_with(seed: Vec<(PositionKey, NativePosition)>) -> Arc<PositionBase> {
        Arc::new(PositionBase::new_at_version(Some(0), "test", seed))
    }

    #[test]
    fn reads_fall_through_to_base() {
        let base = base_with(vec![(key(2, 9), position(100))]);
        let overlay = PositionOverlay::new_at_base(base);

        assert_eq!(overlay.get(&key(2, 9)).map(|p| p.size()), Some(100));
        assert!(overlay.get(&key(3, 9)).is_none());
        assert_eq!(overlay.next_version(), 1);
    }

    #[test]
    fn overlay_shadows_base_and_tombstones_delete() {
        let base = base_with(vec![(key(2, 9), position(100)), (key(2, 8), position(50))]);
        let overlay = PositionOverlay::new_at_base(base)
            .extend(1, vec![(key(2, 9), Some(position(300))), (key(2, 8), None)]);

        assert_eq!(overlay.get(&key(2, 9)).map(|p| p.size()), Some(300));
        assert!(
            overlay.get(&key(2, 8)).is_none(),
            "tombstone must shadow the base"
        );
    }

    /// Writes apply blind — nothing reads prior state — so an account's
    /// other markets are untouched by a write to one of them.
    #[test]
    fn writing_one_market_leaves_the_others_alone() {
        let base = base_with(vec![(key(2, 7), position(10)), (key(2, 8), position(20))]);
        let overlay =
            PositionOverlay::new_at_base(base).extend(1, vec![(key(2, 9), Some(position(30)))]);

        assert_eq!(overlay.get(&key(2, 7)).map(|p| p.size()), Some(10));
        assert_eq!(overlay.get(&key(2, 8)).map(|p| p.size()), Some(20));
        assert_eq!(overlay.get(&key(2, 9)).map(|p| p.size()), Some(30));
        assert_eq!(
            overlay.writes_since_base().len(),
            1,
            "only the written position belongs in the overlay"
        );
    }

    /// Grouping is structural now, so what needs guarding is that an
    /// account never splits across shards and that the fold does not
    /// leave empty accounts behind.
    #[test]
    fn accounts_never_split_across_shards() {
        let seed: Vec<_> = (0..200u64)
            .flat_map(|account| {
                (0..5u64).map(move |market| {
                    (
                        PositionKey {
                            exchange: addr(1),
                            account: addr_n(account),
                            market: addr_n(1 << 40 | market),
                        },
                        position(account + market),
                    )
                })
            })
            .collect();
        let base = base_with(seed);

        base.read(|view| {
            for shard in 0..NUM_POSITION_SHARDS {
                for (account, markets) in view.shard(shard).iter() {
                    assert_eq!(shard_of(account), shard);
                    assert_eq!(markets.len(), 5, "an account's markets must stay together");
                }
            }
            assert_eq!(view.num_accounts(), 200);
            assert_eq!(view.num_positions(), 1000);
        });
    }

    /// An account whose last position is deleted must disappear, or it
    /// reads as live with an empty position set.
    #[test]
    fn fold_prunes_an_account_that_loses_its_last_position() {
        let base = base_with(vec![(key(2, 7), position(10)), (key(2, 8), position(20))]);
        let overlay = PositionOverlay::new_at_base(Arc::clone(&base))
            .extend(1, vec![(key(2, 7), None), (key(2, 8), None)]);

        assert!(base.fold(&overlay));
        base.read(|view| {
            assert_eq!(view.num_accounts(), 0, "empty account must be pruned");
            assert!(view
                .account_positions(&AccountKey {
                    exchange: addr(1),
                    account: addr(2),
                })
                .is_none());
        });
    }

    /// Deleting one of several markets keeps the account.
    #[test]
    fn fold_keeps_an_account_that_still_holds_a_position() {
        let base = base_with(vec![(key(2, 7), position(10)), (key(2, 8), position(20))]);
        let overlay =
            PositionOverlay::new_at_base(Arc::clone(&base)).extend(1, vec![(key(2, 7), None)]);

        assert!(base.fold(&overlay));
        base.read(|view| {
            assert_eq!(view.num_accounts(), 1);
            assert_eq!(view.num_positions(), 1);
        });
    }

    #[test]
    fn account_positions_returns_only_that_accounts_markets_in_order() {
        let base = base_with(vec![
            (key(2, 9), position(1)),
            (key(2, 7), position(2)),
            (key(3, 8), position(3)),
        ]);
        base.read(|view| {
            let markets: Vec<_> = view
                .account_positions(&AccountKey {
                    exchange: addr(1),
                    account: addr(2),
                })
                .expect("account present")
                .keys()
                .copied()
                .collect();
            assert_eq!(markets, vec![addr(7), addr(9)]);
        });
    }

    #[test]
    fn fold_advances_base_applying_writes_and_deletes() {
        let base = base_with(vec![(key(2, 7), position(10))]);
        let overlay = PositionOverlay::new_at_base(Arc::clone(&base))
            .extend(1, vec![(key(2, 8), Some(position(20))), (key(2, 7), None)]);

        assert!(base.fold(&overlay));
        assert_eq!(base.version(), Some(1));
        base.read(|view| {
            assert!(view.get(&key(2, 7)).is_none(), "delete must remove");
            assert_eq!(view.get(&key(2, 8)).map(|p| p.size()), Some(20));
            assert_eq!(view.num_positions(), 1);
        });

        let fresh = PositionOverlay::new_at_base(Arc::clone(&base));
        assert!(fresh.writes_since_base().is_empty());
        assert_eq!(fresh.get(&key(2, 8)).map(|p| p.size()), Some(20));
    }

    #[test]
    fn overlay_built_before_fold_still_reads_correctly() {
        let base = base_with(Vec::new());
        let v0 = PositionOverlay::new_at_base(Arc::clone(&base))
            .extend(1, vec![(key(2, 9), Some(position(100)))]);
        let v1 = v0.extend(2, vec![(key(3, 9), Some(position(200)))]);

        assert!(base.fold(&v0));

        assert_eq!(v1.get(&key(2, 9)).map(|p| p.size()), Some(100));
        assert_eq!(v1.get(&key(3, 9)).map(|p| p.size()), Some(200));
        assert!(v1.descends_from_base());
    }

    /// Inheriting the parent's view base would keep the layers from
    /// process start alive and reclaim nothing.
    #[test]
    fn extend_reseats_onto_the_advanced_base() {
        let base = base_with(Vec::new());
        let v0 = PositionOverlay::new_at_base(Arc::clone(&base))
            .extend(1, vec![(key(2, 9), Some(position(100)))]);
        assert!(base.fold(&v0));

        let v1 = v0.extend(2, vec![(key(3, 9), Some(position(200)))]);

        let writes = v1.writes_since_base();
        assert_eq!(writes.len(), 1, "got {writes:?}");
        assert_eq!(writes[0].0, key(3, 9));
        assert_eq!(v1.get(&key(2, 9)).map(|p| p.size()), Some(100));
    }

    /// A sibling shares the family and the layer-number range, so
    /// `can_view_after` accepts it. Only an ancestry walk rejects it.
    #[test]
    fn sibling_fork_does_not_descend_from_a_folded_base() {
        let base = base_with(Vec::new());
        let parent = PositionOverlay::new_at_base(Arc::clone(&base))
            .extend(1, vec![(key(2, 9), Some(position(1)))]);
        let certified = parent.extend(2, vec![(key(3, 9), Some(position(2)))]);
        let abandoned = parent.extend(2, vec![(key(4, 9), Some(position(3)))]);

        assert!(base.fold(&certified));

        assert!(certified.descends_from_base());
        assert!(!abandoned.descends_from_base());
        assert!(!base.fold(&abandoned), "fold must decline a sibling");
        let base_layers = base.layers();
        assert!(abandoned.shards()[0].can_view_after(&base_layers[0]));
    }

    /// The per-exchange range scan reads this ordering. If the field
    /// order ever changes, the range silently returns the wrong set.
    /// The layered view resolves latest-wins when a layer is built, so
    /// a key written in several layers still surfaces once. Readers
    /// group these by account and look markets up directly; a duplicate
    /// would make that lookup order-dependent.
    #[test]
    fn writes_since_base_yields_each_key_once() {
        let base = base_with(Vec::new());
        let overlay = PositionOverlay::new_at_base(base)
            .extend(1, vec![(key(2, 9), Some(position(1)))])
            .extend(2, vec![(key(2, 9), Some(position(2)))])
            .extend(3, vec![
                (key(2, 9), Some(position(3))),
                (key(2, 8), Some(position(4))),
            ]);

        let writes = overlay.writes_since_base();
        assert_eq!(writes.len(), 2, "one entry per key, got {writes:?}");
        let latest = writes
            .iter()
            .find(|(k, _)| *k == key(2, 9))
            .expect("key present");
        assert_eq!(
            latest.1.as_ref().map(|p| p.size()),
            Some(3),
            "the newest write must be the surviving one"
        );
    }

    #[test]
    fn account_key_orders_by_exchange_first() {
        let low_exchange_high_account = AccountKey {
            exchange: addr(1),
            account: addr_n(u64::MAX),
        };
        let high_exchange_low_account = AccountKey {
            exchange: addr(2),
            account: addr_n(0),
        };
        assert!(low_exchange_high_account < high_exchange_low_account);
    }

    #[test]
    fn accounts_in_exchange_returns_only_that_exchange() {
        let base = base_with(vec![
            (key(2, 7), position(1)),
            (key(3, 7), position(2)),
            (
                PositionKey {
                    exchange: addr(9),
                    account: addr(2),
                    market: addr(7),
                },
                position(3),
            ),
        ]);
        base.read(|view| {
            let found: Vec<_> = (0..NUM_POSITION_SHARDS)
                .flat_map(|shard| {
                    view.accounts_in_exchange(shard, addr(1))
                        .map(|(account, _)| account.account)
                        .collect::<Vec<_>>()
                })
                .collect();
            assert_eq!(found.len(), 2, "exchange 1 holds two accounts: {found:?}");
            assert!(found.contains(&addr(2)) && found.contains(&addr(3)));

            let other: Vec<_> = (0..NUM_POSITION_SHARDS)
                .flat_map(|shard| {
                    view.accounts_in_exchange(shard, addr(9))
                        .map(|(account, _)| account.account)
                        .collect::<Vec<_>>()
                })
                .collect();
            assert_eq!(other, vec![addr(2)]);
        });
    }

    /// Re-seating a fork the base has moved past must leave its floor
    /// alone. Re-seating regardless would pair its tops with a floor at
    /// the same depth on another branch, filtering its own writes out of
    /// view and resolving them against the certified branch instead.
    #[test]
    fn rebasing_a_fork_that_does_not_descend_is_a_noop() {
        let base = base_with(Vec::new());
        let parent = PositionOverlay::new_at_base(Arc::clone(&base))
            .extend(1, vec![(key(2, 9), Some(position(1)))]);
        let certified = parent.extend(2, vec![(key(3, 9), Some(position(2)))]);
        let abandoned = parent.extend(2, vec![(key(4, 9), Some(position(3)))]);

        assert!(base.fold(&certified));
        assert!(!abandoned.descends_from_base());

        let reseated = abandoned.rebased_on_current_base();

        assert_eq!(
            reseated.get(&key(4, 9)).map(|p| p.size()),
            abandoned.get(&key(4, 9)).map(|p| p.size()),
            "the fork's own write must stay visible"
        );
        assert_eq!(
            reseated.writes_since_base().len(),
            abandoned.writes_since_base().len(),
            "the floor must not have moved"
        );
    }

    /// Composition of the two: an overlay whose floor lags the base gets
    /// extended again. The new layer must re-seat, drop what the base
    /// absorbed, and keep everything the base does not have.
    #[test]
    fn extending_a_lagging_overlay_after_a_fold_keeps_every_write_visible() {
        let base = base_with(Vec::new());
        let v0 = PositionOverlay::new_at_base(Arc::clone(&base))
            .extend(1, vec![(key(2, 9), Some(position(1)))]);
        let v1 = v0.extend(2, vec![(key(3, 9), Some(position(2)))]);

        assert!(base.fold(&v0)); // base absorbs key(2,9); v1's floor now lags

        let v2 = v1.extend(3, vec![(key(4, 9), Some(position(3)))]);

        assert_eq!(
            v2.get(&key(2, 9)).map(|p| p.size()),
            Some(1),
            "from the base"
        );
        assert_eq!(
            v2.get(&key(3, 9)).map(|p| p.size()),
            Some(2),
            "from v1's layer"
        );
        assert_eq!(
            v2.get(&key(4, 9)).map(|p| p.size()),
            Some(3),
            "from v2's layer"
        );
        assert_eq!(
            v2.writes_since_base().len(),
            2,
            "the folded write should have dropped out of the overlay"
        );
    }

    /// The same key written either side of a fold: the base holds the
    /// older value, the overlay the newer, and the overlay must win.
    #[test]
    fn overwrite_across_a_fold_resolves_to_the_newer_value() {
        let base = base_with(Vec::new());
        let v0 = PositionOverlay::new_at_base(Arc::clone(&base))
            .extend(1, vec![(key(2, 9), Some(position(1)))]);
        assert!(base.fold(&v0));

        let v1 = v0.extend(2, vec![(key(2, 9), Some(position(2)))]);
        assert_eq!(v1.get(&key(2, 9)).map(|p| p.size()), Some(2));

        // And a delete of a key that only exists in the base.
        let v2 = v1.extend(3, vec![(key(2, 9), None)]);
        assert!(
            v2.get(&key(2, 9)).is_none(),
            "tombstone must shadow the base"
        );
    }

    #[test]
    fn shard_of_ignores_the_market_and_spreads_accounts() {
        let account = AccountKey {
            exchange: addr(1),
            account: addr_n(12345),
        };
        let a = shard_of(
            &PositionKey {
                exchange: account.exchange,
                account: account.account,
                market: addr_n(7),
            }
            .account(),
        );
        let b = shard_of(
            &PositionKey {
                exchange: account.exchange,
                account: account.account,
                market: addr_n(999),
            }
            .account(),
        );
        assert_eq!(a, b);

        let mut seen = vec![0usize; NUM_POSITION_SHARDS];
        for i in 0..10_000u64 {
            seen[shard_of(&AccountKey {
                exchange: addr(1),
                account: addr_n(i),
            })] += 1;
        }
        let min = *seen.iter().min().unwrap();
        let max = *seen.iter().max().unwrap();
        assert!(min > 0 && max <= min * 2, "skewed: {seen:?}");
    }
}
