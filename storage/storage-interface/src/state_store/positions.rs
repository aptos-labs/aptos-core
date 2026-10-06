// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Native position index: a complete [`PositionBase`] with a speculative
//! [`PositionOverlay`] over it, both sharded on `(exchange, account)`.
//!
//! Keyed as the write set is, so writes apply blind. The base nests by
//! account and sorts exchange-major, making a per-exchange scan a range;
//! the overlay is a hash-ordered `MapLayer` holding only writes since the
//! base, few enough to group on the fly.

#![forbid(unsafe_code)]

use ahash::AHashMap;
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
/// which is what lets [`PositionBaseView::accounts_in_exchange`] be a
/// range rather than a filter. Guarded by
/// `account_key_orders_by_exchange_first`.
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
    /// Nested rather than keyed by the flat `PositionKey` so the grouping
    /// risk reads by is structural. Free on the write side: the base is
    /// owned and mutated in place, unlike the overlay.
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

/// Resolves overlay misses, as the state KV DB does for main state.
/// Resident and complete, because the durable store is hash-keyed and can
/// enumerate neither an exchange's accounts nor an account's markets.
///
/// Single-version, so it must not move while a block executes: the
/// executor advances it under its execution lock.
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
    /// A range, not a filter — `AccountKey` sorts exchange-major — so this
    /// costs what the exchange holds, not what every exchange holds.
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

    /// Borrowed from the guarded view, so a descent check can run without
    /// re-entering the lock.
    pub fn layers(&self) -> &ShardedPositionLayers {
        &self.inner.layers
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
        seed: impl IntoIterator<Item = (PositionKey, NativePosition)>,
    ) -> Self {
        Self {
            inner: RwLock::new(
                Self::build(
                    version,
                    family,
                    seed.into_iter().map(Ok::<_, crate::AptosDbError>),
                )
                .expect("Infallible rows."),
            ),
        }
    }

    /// Cold-load from undecoded durable rows, so the decoded set is never
    /// materialized alongside the base it feeds.
    pub fn new_from_rows<I>(
        version: Option<Version>,
        family: &'static str,
        rows: I,
    ) -> crate::Result<Self>
    where
        I: IntoIterator<Item = crate::Result<(StateKey, StateValue)>>,
    {
        Ok(Self {
            inner: RwLock::new(Self::build(
                version,
                family,
                rows.into_iter().map(decode_position_row),
            )?),
        })
    }

    /// Rebuild in place from durable rows, for fast sync. Starts a new
    /// layer family, so overlays on the old base stop descending from it
    /// and must be re-seated.
    pub fn reset_from_rows<I>(
        &self,
        version: Option<Version>,
        family: &'static str,
        rows: I,
    ) -> crate::Result<()>
    where
        I: IntoIterator<Item = crate::Result<(StateKey, StateValue)>>,
    {
        let rebuilt = Self::build(version, family, rows.into_iter().map(decode_position_row))?;
        *self.inner.write() = rebuilt;
        Ok(())
    }

    fn build<E>(
        version: Option<Version>,
        family: &'static str,
        rows: impl Iterator<Item = std::result::Result<(PositionKey, NativePosition), E>>,
    ) -> std::result::Result<PositionBaseInner, E> {
        let mut shards: [BTreeMap<AccountKey, AccountPositions>; NUM_POSITION_SHARDS] =
            arr![BTreeMap::new(); 16];
        for row in rows {
            let (key, position) = row?;
            let account = key.account();
            shards[shard_of(&account)]
                .entry(account)
                .or_default()
                .insert(key.market, position);
        }
        Ok(PositionBaseInner {
            version,
            shards,
            layers: arr![MapLayer::new_family(family); 16],
        })
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

    /// Empty overlay sitting directly on the base's current layers.
    pub fn new_overlay(&self) -> PositionOverlay {
        let inner = self.inner.read();
        PositionOverlay {
            next_version: inner.version.map_or(0, |v| v + 1),
            tops: Arc::new(inner.layers.clone()),
        }
    }

    /// The guard [`Self::fold`] and [`Self::with_view`] require. Walks the
    /// ancestry rather than using `can_view_after`, which an abandoned
    /// sibling fork also satisfies while sharing no ancestry.
    pub fn is_ancestor_of(&self, overlay: &PositionOverlay) -> bool {
        overlay.descends_from(&self.inner.read().layers)
    }

    /// Runs `f` against `overlay` merged over this base.
    ///
    /// Holds the read lock for the scope, so a fold cannot land mid-view
    /// and leave the two halves at different versions; whatever `f` does
    /// delays the next fold, so keep it to the queries. The base must not
    /// have moved past `overlay` — never true of the committed tip, and
    /// true of a block's overlay until the base folds past that block.
    pub fn with_view<R>(
        &self,
        overlay: &PositionOverlay,
        f: impl FnOnce(&NativeStateView<'_>) -> R,
    ) -> R {
        self.read(|base_view| {
            // Under the lock, so the layers checked are the ones merged
            // against; checking before locking lets a fold land in between.
            assert!(
                overlay.descends_from(base_view.layers()),
                "position overlay does not descend from the base it would be merged over",
            );
            let mut by_shard: OverlayByShard = (0..NUM_POSITION_SHARDS)
                .map(|_| OverlayByAccount::default())
                .collect();
            for (key, write) in overlay.writes_since(base_view.layers()) {
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

    /// Applies everything written since the base layers. Returns whether
    /// the base moved; declines an overlay at or behind the base, or one
    /// not descended from it, re-checked here under the write lock.
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

/// Speculative overlay: one layer per shard per chunk, chained from the
/// parent's. Like main state's `State`, it holds the layers only; the base
/// they are read over is supplied at read time, so an overlay cannot name
/// a base other than the one it is merged with.
#[derive(Clone, Debug)]
pub struct PositionOverlay {
    next_version: Version,
    tops: Arc<ShardedPositionLayers>,
}

/// What a chunk's overlay extends from: the parent overlay and the base
/// layers it was checked against, captured together at the top of the
/// execution stage so the new layer is viewable from that base.
#[derive(Clone, Copy)]
pub struct PositionParent<'a> {
    pub overlay: &'a PositionOverlay,
    pub floor: &'a ShardedPositionLayers,
}

impl PositionOverlay {
    pub fn next_version(&self) -> Version {
        self.next_version
    }

    pub fn version(&self) -> Option<Version> {
        self.next_version.checked_sub(1)
    }

    pub fn shards(&self) -> &ShardedPositionLayers {
        &self.tops
    }

    pub fn is_descendant_of(&self, rhs: &Self) -> bool {
        self.descends_from(&rhs.tops)
    }

    /// Whether every shard's top descends from `layers`. Shards move in
    /// lockstep, so one predicate covers all of them.
    pub fn descends_from(&self, layers: &ShardedPositionLayers) -> bool {
        (0..NUM_POSITION_SHARDS).all(|shard| self.tops[shard].is_descendant_of(&layers[shard]))
    }

    /// Writes above `floor` — what the overlay shadows when read over a
    /// base at `floor`. Walks layers only, so it is safe inside
    /// `PositionBase::read`.
    pub fn writes_since(&self, floor: &ShardedPositionLayers) -> PositionWrites {
        (0..NUM_POSITION_SHARDS)
            .into_par_iter()
            .flat_map_iter(|shard| {
                self.tops[shard]
                    .view_layers_after(&floor[shard])
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

    /// Push one layer per shard atop `self`, built over `floor` — the
    /// base's layers at the time. A layer links to nothing at or below
    /// its floor, so this is the lowest base it can ever be read from;
    /// the base's current layers are the right floor because the base
    /// only moves forward.
    pub fn extend(
        &self,
        floor: &ShardedPositionLayers,
        new_version: Version,
        writes: PositionWrites,
    ) -> Self {
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
        // A sibling of the floor satisfies `can_view_after` yet shares no
        // ancestry; building over it would mix branches.
        assert!(
            self.descends_from(floor),
            "extending an overlay over a floor it does not descend from",
        );
        let per_shard = Self::split_by_shard(writes);
        let tops: Vec<PositionLayer> = (0..NUM_POSITION_SHARDS)
            .into_par_iter()
            .map(|shard| {
                self.tops[shard]
                    .view_layers_after(&floor[shard])
                    .new_layer(&per_shard[shard])
            })
            .collect();
        Self {
            next_version: new_version + 1,
            tops: Arc::new(
                tops.try_into()
                    .expect("Known to be NUM_POSITION_SHARDS shards."),
            ),
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

/// The overlay's writes grouped by account, bucketed per shard. Built once
/// per view, affordable because the overlay holds only writes since the
/// last fold; sorting it instead would cost a 96-byte-key sort for nothing.
type OverlayByAccount = AHashMap<AccountKey, Vec<(AccountAddress, Option<NativePosition>)>>;
type OverlayByShard = Vec<OverlayByAccount>;

/// A [`PositionOverlay`] merged over the base it sits on, valid for the
/// scope of [`PositionOverlay::with_view`].
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
    /// is structural. Only the overlay's accounts need merging in, and
    /// there are at most one chunk's worth.
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

    /// Overlay first, then the base. `None` for a tombstone or a miss.
    pub fn get(&self, key: &PositionKey) -> Option<NativePosition> {
        match self.overlay_write(&key.account(), &key.market) {
            Some(write) => write,
            None => self.base.get(key).cloned(),
        }
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

    /// One lookup into the account's base positions, merged with its
    /// overlay writes. In market order.
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

/// Decodes one durable JMT row into a [`PositionBase`] entry.
fn decode_position_row(
    row: crate::Result<(StateKey, StateValue)>,
) -> crate::Result<(PositionKey, NativePosition)> {
    let (state_key, state_value) = row?;
    let key = position_key_of(&state_key)?;
    let position = NativePosition::deserialize(state_value.bytes()).map_err(|e| {
        crate::AptosDbError::Other(format!("native position at startup failed to decode: {e}"))
    })?;
    Ok((key, position))
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

    fn row(key: PositionKey, size: u64) -> crate::Result<(StateKey, StateValue)> {
        Ok((
            StateKey::position(key.exchange, key.account, key.market),
            StateValue::new_legacy(position(size).serialize().unwrap().into()),
        ))
    }

    /// `overlay` read over `base`, as the size of the position at `key`.
    fn read(base: &PositionBase, overlay: &PositionOverlay, key: &PositionKey) -> Option<u64> {
        base.with_view(overlay, |view| view.get(key).map(|p| p.size()))
    }

    #[test]
    fn reset_from_rows_replaces_the_base_and_strands_the_old_overlay() {
        let base = base_with(Vec::new());
        let stale = base
            .new_overlay()
            .extend(&base.layers(), 1, vec![(key(2, 9), Some(position(1)))]);
        assert_eq!(read(&base, &stale, &key(2, 9)), Some(1));

        base.reset_from_rows(Some(7), "test", vec![row(key(3, 4), 500)])
            .unwrap();

        assert_eq!(base.version(), Some(7));
        let fresh = base.new_overlay();
        assert_eq!(read(&base, &fresh, &key(3, 4)), Some(500));
        assert!(read(&base, &fresh, &key(2, 9)).is_none());
        assert_eq!(fresh.next_version(), 8);
        // On the old family now, so it must not fold into the base.
        assert!(!base.is_ancestor_of(&stale));
    }

    #[test]
    fn reads_fall_through_to_base() {
        let base = base_with(vec![(key(2, 9), position(100))]);
        let overlay = base.new_overlay();

        assert_eq!(read(&base, &overlay, &key(2, 9)), Some(100));
        assert!(read(&base, &overlay, &key(3, 9)).is_none());
        assert_eq!(overlay.next_version(), 1);
    }

    #[test]
    fn overlay_shadows_base_and_tombstones_delete() {
        let base = base_with(vec![(key(2, 9), position(100)), (key(2, 8), position(50))]);
        let overlay = base.new_overlay().extend(&base.layers(), 1, vec![
            (key(2, 9), Some(position(300))),
            (key(2, 8), None),
        ]);

        assert_eq!(read(&base, &overlay, &key(2, 9)), Some(300));
        assert!(
            read(&base, &overlay, &key(2, 8)).is_none(),
            "tombstone must shadow the base"
        );
    }

    /// Writes apply blind, so an account's other markets are untouched.
    #[test]
    fn writing_one_market_leaves_the_others_alone() {
        let base = base_with(vec![(key(2, 7), position(10)), (key(2, 8), position(20))]);
        let overlay = base
            .new_overlay()
            .extend(&base.layers(), 1, vec![(key(2, 9), Some(position(30)))]);

        assert_eq!(read(&base, &overlay, &key(2, 7)), Some(10));
        assert_eq!(read(&base, &overlay, &key(2, 8)), Some(20));
        assert_eq!(read(&base, &overlay, &key(2, 9)), Some(30));
        assert_eq!(
            overlay.writes_since(&base.layers()).len(),
            1,
            "only the written position belongs in the overlay"
        );
    }

    /// Grouping is structural, so what needs guarding is that an account
    /// never splits across shards.
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

    /// Or it reads as live with an empty position set.
    #[test]
    fn fold_prunes_an_account_that_loses_its_last_position() {
        let base = base_with(vec![(key(2, 7), position(10)), (key(2, 8), position(20))]);
        let overlay = base.new_overlay().extend(&base.layers(), 1, vec![
            (key(2, 7), None),
            (key(2, 8), None),
        ]);

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
        let overlay = base
            .new_overlay()
            .extend(&base.layers(), 1, vec![(key(2, 7), None)]);

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
        let overlay = base.new_overlay().extend(&base.layers(), 1, vec![
            (key(2, 8), Some(position(20))),
            (key(2, 7), None),
        ]);

        assert!(base.fold(&overlay));
        assert_eq!(base.version(), Some(1));
        base.read(|view| {
            assert!(view.get(&key(2, 7)).is_none(), "delete must remove");
            assert_eq!(view.get(&key(2, 8)).map(|p| p.size()), Some(20));
            assert_eq!(view.num_positions(), 1);
        });

        let fresh = base.new_overlay();
        assert!(fresh.writes_since(&base.layers()).is_empty());
        assert_eq!(read(&base, &fresh, &key(2, 8)), Some(20));
    }

    #[test]
    fn overlay_built_before_fold_still_reads_correctly() {
        let base = base_with(Vec::new());
        let v0 = base
            .new_overlay()
            .extend(&base.layers(), 1, vec![(key(2, 9), Some(position(100)))]);
        let v1 = v0.extend(&base.layers(), 2, vec![(key(3, 9), Some(position(200)))]);

        assert!(base.fold(&v0));

        assert_eq!(read(&base, &v1, &key(2, 9)), Some(100));
        assert_eq!(read(&base, &v1, &key(3, 9)), Some(200));
        assert!(base.is_ancestor_of(&v1));
    }

    /// Reading over the advanced base excludes what it absorbed, with no
    /// re-seat needed.
    #[test]
    fn view_over_an_advanced_base_sheds_absorbed_writes() {
        let base = base_with(Vec::new());
        let v0 = base
            .new_overlay()
            .extend(&base.layers(), 1, vec![(key(2, 9), Some(position(100)))]);
        assert!(base.fold(&v0));

        let v1 = v0.extend(&base.layers(), 2, vec![(key(3, 9), Some(position(200)))]);

        let writes = v1.writes_since(&base.layers());
        assert_eq!(writes.len(), 1, "got {writes:?}");
        assert_eq!(writes[0].0, key(3, 9));
        assert_eq!(read(&base, &v1, &key(2, 9)), Some(100));
    }

    /// A sibling satisfies `can_view_after`; only an ancestry walk rejects it.
    #[test]
    fn sibling_fork_does_not_descend_from_a_folded_base() {
        let base = base_with(Vec::new());
        let floor = base.layers();
        let parent = base
            .new_overlay()
            .extend(&floor, 1, vec![(key(2, 9), Some(position(1)))]);
        let certified = parent.extend(&floor, 2, vec![(key(3, 9), Some(position(2)))]);
        let abandoned = parent.extend(&floor, 2, vec![(key(4, 9), Some(position(3)))]);

        assert!(base.fold(&certified));

        assert!(base.is_ancestor_of(&certified));
        assert!(!base.is_ancestor_of(&abandoned));
        assert!(!base.fold(&abandoned), "fold must decline a sibling");
        let base_layers = base.layers();
        assert!(abandoned.shards()[0].can_view_after(&base_layers[0]));
    }

    /// A key written in several layers must still surface once, or the
    /// per-account market lookup becomes order-dependent.
    #[test]
    fn writes_since_yields_each_key_once() {
        let base = base_with(Vec::new());
        let floor = base.layers();
        let overlay = base
            .new_overlay()
            .extend(&floor, 1, vec![(key(2, 9), Some(position(1)))])
            .extend(&floor, 2, vec![(key(2, 9), Some(position(2)))])
            .extend(&floor, 3, vec![
                (key(2, 9), Some(position(3))),
                (key(2, 8), Some(position(4))),
            ]);

        let writes = overlay.writes_since(&floor);
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

    /// The per-exchange range scan depends on this ordering.
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

    /// There is no floor a fork the base branched away from could be
    /// read over, so the view refuses rather than mixing branches.
    #[test]
    #[should_panic(expected = "does not descend")]
    fn viewing_a_fork_the_base_branched_away_from_panics() {
        let base = base_with(Vec::new());
        let floor = base.layers();
        let parent = base
            .new_overlay()
            .extend(&floor, 1, vec![(key(2, 9), Some(position(1)))]);
        let certified = parent.extend(&floor, 2, vec![(key(3, 9), Some(position(2)))]);
        let abandoned = parent.extend(&floor, 2, vec![(key(4, 9), Some(position(3)))]);

        assert!(base.fold(&certified));
        assert!(!base.is_ancestor_of(&abandoned));

        base.with_view(&abandoned, |_| ());
    }

    /// Extending over the advanced base drops what it absorbed and keeps
    /// the rest.
    #[test]
    fn extending_a_lagging_overlay_after_a_fold_keeps_every_write_visible() {
        let base = base_with(Vec::new());
        let v0 = base
            .new_overlay()
            .extend(&base.layers(), 1, vec![(key(2, 9), Some(position(1)))]);
        let v1 = v0.extend(&base.layers(), 2, vec![(key(3, 9), Some(position(2)))]);

        assert!(base.fold(&v0)); // base absorbs key(2,9); v1 was built over the old floor

        let v2 = v1.extend(&base.layers(), 3, vec![(key(4, 9), Some(position(3)))]);

        assert_eq!(read(&base, &v2, &key(2, 9)), Some(1), "from the base");
        assert_eq!(read(&base, &v2, &key(3, 9)), Some(2), "from v1's layer");
        assert_eq!(read(&base, &v2, &key(4, 9)), Some(3), "from v2's layer");
        assert_eq!(
            v2.writes_since(&base.layers()).len(),
            2,
            "the folded write should have dropped out of the overlay"
        );
    }

    /// Base holds the older value, overlay the newer; the overlay wins.
    #[test]
    fn overwrite_across_a_fold_resolves_to_the_newer_value() {
        let base = base_with(Vec::new());
        let v0 = base
            .new_overlay()
            .extend(&base.layers(), 1, vec![(key(2, 9), Some(position(1)))]);
        assert!(base.fold(&v0));

        let v1 = v0.extend(&base.layers(), 2, vec![(key(2, 9), Some(position(2)))]);
        assert_eq!(read(&base, &v1, &key(2, 9)), Some(2));

        // And a delete of a base-only key.
        let v2 = v1.extend(&base.layers(), 3, vec![(key(2, 9), None)]);
        assert!(
            read(&base, &v2, &key(2, 9)).is_none(),
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
