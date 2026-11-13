// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! A native index grouped by account: a complete [`AccountIndexBase`] with a
//! speculative [`NativeOverlay`] over it, both sharded on `(exchange, account)`.
//!
//! Keyed as the write set is, so writes apply blind. The base nests by
//! account and sorts exchange-major, making a per-exchange scan a range; the
//! overlay holds only writes since the base, few enough to group on the fly.
//! Generic over what an account holds: positions key it by market, collateral
//! by scope.

#![forbid(unsafe_code)]

use crate::state_store::native_overlay::{
    NativeOverlay, NativeValue, ShardedKey, ShardedNativeLayers, NUM_NATIVE_INDEX_SHARDS,
};
use ahash::AHashMap;
use aptos_experimental_layered_map::MapLayer;
use aptos_infallible::RwLock;
use aptos_types::{
    account_address::AccountAddress,
    state_store::{state_key::StateKey, state_value::StateValue},
    transaction::Version,
};
use arr_macro::arr;
use rayon::prelude::*;
use std::{collections::BTreeMap, fmt::Debug, hash::Hash};

/// The `(exchange, account)` an account's entries group under.
///
/// **Field order is load-bearing.** The derived `Ord` is exchange-major,
/// which is what lets [`AccountIndexBaseView::accounts_in_exchange`] be a
/// range rather than a filter. Guarded by
/// `account_key_orders_by_exchange_first`.
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd, Debug)]
pub struct AccountKey {
    pub exchange: AccountAddress,
    pub account: AccountAddress,
}

/// Shard on the account, which is the base's outer key, so an account's
/// entries never split across shards and grouping needs no coordination
/// between them. Every account index shards this way, so one account's
/// positions and collateral share a shard.
pub fn shard_of(account: &AccountKey) -> usize {
    let last = AccountAddress::LENGTH - 1;
    let byte = account.account.as_ref()[last] ^ account.exchange.as_ref()[last];
    usize::from(byte) % NUM_NATIVE_INDEX_SHARDS
}

/// A key of an account index: the account it groups under and the entry it
/// names within that account. Its shard must be `shard_of(&self.account())`.
pub trait AccountIndexKey: ShardedKey + Copy {
    type Entry: Copy + Ord + Hash + Debug + Send + Sync + 'static;

    fn account(&self) -> AccountKey;

    fn entry(&self) -> Self::Entry;
}

/// What an account index holds and how its durable rows decode.
pub trait AccountIndex: 'static {
    type Key: AccountIndexKey;
    type Value: NativeValue;

    /// Names the index in diagnostics.
    const NAME: &'static str;

    fn decode_row(
        state_key: &StateKey,
        state_value: &StateValue,
    ) -> crate::Result<(Self::Key, Self::Value)>;
}

pub type Entry<I> = <<I as AccountIndex>::Key as AccountIndexKey>::Entry;

/// An account's entries, in entry order.
pub type AccountEntries<I> = BTreeMap<Entry<I>, <I as AccountIndex>::Value>;

pub type ShardedIndexLayers<I> =
    ShardedNativeLayers<<I as AccountIndex>::Key, <I as AccountIndex>::Value>;

pub type IndexOverlay<I> = NativeOverlay<<I as AccountIndex>::Key, <I as AccountIndex>::Value>;

struct BaseInner<I: AccountIndex> {
    version: Option<Version>,
    /// Nested rather than keyed by the flat key so the grouping risk reads
    /// by is structural. Free on the write side: the base is owned and
    /// mutated in place, unlike the overlay.
    shards: [BTreeMap<AccountKey, AccountEntries<I>>; NUM_NATIVE_INDEX_SHARDS],
    /// Layer handles at `version` — the view base handed to new blocks.
    layers: ShardedIndexLayers<I>,
}

impl<I: AccountIndex> Debug for BaseInner<I> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let len: usize = self.shards.iter().map(|s| s.len()).sum();
        write!(f, "{}BaseInner(version={:?}, {len})", I::NAME, self.version)
    }
}

/// Resolves overlay misses, as the state KV DB does for main state.
/// Resident and complete, because the durable store is hash-keyed and can
/// enumerate neither an exchange's accounts nor an account's entries.
///
/// Single-version, so it must not move while a block executes: the
/// executor advances it under its execution lock.
#[derive(Debug)]
pub struct AccountIndexBase<I: AccountIndex> {
    inner: RwLock<BaseInner<I>>,
}

/// Guarded view of an [`AccountIndexBase`], scope-bound to a closure so no
/// caller can park a lock and stall folds.
pub struct AccountIndexBaseView<'a, I: AccountIndex> {
    inner: &'a BaseInner<I>,
}

impl<I: AccountIndex> AccountIndexBaseView<'_, I> {
    pub fn version(&self) -> Option<Version> {
        self.inner.version
    }

    pub fn get(&self, key: &I::Key) -> Option<&I::Value> {
        self.inner.shards[shard_of(&key.account())]
            .get(&key.account())
            .and_then(|entries| entries.get(&key.entry()))
    }

    pub fn shard(&self, shard: usize) -> &BTreeMap<AccountKey, AccountEntries<I>> {
        &self.inner.shards[shard]
    }

    /// The account's entries, in entry order. `None` when it holds none —
    /// the fold prunes accounts whose last entry is deleted, so an empty
    /// entry never lingers.
    pub fn account_entries(&self, account: &AccountKey) -> Option<&AccountEntries<I>> {
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
    ) -> impl Iterator<Item = (&AccountKey, &AccountEntries<I>)> {
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
    pub fn layers(&self) -> &ShardedIndexLayers<I> {
        &self.inner.layers
    }

    pub fn num_accounts(&self) -> usize {
        self.inner.shards.iter().map(|s| s.len()).sum()
    }

    pub fn num_entries(&self) -> usize {
        self.inner
            .shards
            .iter()
            .map(|s| s.values().map(|entries| entries.len()).sum::<usize>())
            .sum()
    }
}

impl<I: AccountIndex> AccountIndexBase<I> {
    pub fn new_empty(family: &'static str) -> Self {
        Self::new_at_version(None, family, Vec::new())
    }

    /// Cold-load entry point: `seed` is the decoded durable snapshot.
    pub fn new_at_version(
        version: Option<Version>,
        family: &'static str,
        seed: impl IntoIterator<Item = (I::Key, I::Value)>,
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
    pub fn new_from_rows<R>(
        version: Option<Version>,
        family: &'static str,
        rows: R,
    ) -> crate::Result<Self>
    where
        R: IntoIterator<Item = crate::Result<(StateKey, StateValue)>>,
    {
        Ok(Self {
            inner: RwLock::new(Self::build(
                version,
                family,
                rows.into_iter().map(decode_row::<I>),
            )?),
        })
    }

    /// Rebuild in place from durable rows, for fast sync. Starts a new
    /// layer family, so overlays on the old base stop descending from it
    /// and must be re-seated.
    pub fn reset_from_rows<R>(
        &self,
        version: Option<Version>,
        family: &'static str,
        rows: R,
    ) -> crate::Result<()>
    where
        R: IntoIterator<Item = crate::Result<(StateKey, StateValue)>>,
    {
        let rebuilt = Self::build(version, family, rows.into_iter().map(decode_row::<I>))?;
        *self.inner.write() = rebuilt;
        Ok(())
    }

    fn build<E>(
        version: Option<Version>,
        family: &'static str,
        rows: impl Iterator<Item = std::result::Result<(I::Key, I::Value), E>>,
    ) -> std::result::Result<BaseInner<I>, E> {
        let mut shards: [BTreeMap<AccountKey, AccountEntries<I>>; NUM_NATIVE_INDEX_SHARDS] =
            arr![BTreeMap::new(); 16];
        for row in rows {
            let (key, value) = row?;
            let account = key.account();
            shards[shard_of(&account)]
                .entry(account)
                .or_default()
                .insert(key.entry(), value);
        }
        Ok(BaseInner {
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
    pub fn layers(&self) -> ShardedIndexLayers<I> {
        self.inner.read().layers.clone()
    }

    pub fn read<R>(&self, f: impl FnOnce(&AccountIndexBaseView<'_, I>) -> R) -> R {
        let guard = self.inner.read();
        f(&AccountIndexBaseView { inner: &guard })
    }

    /// Empty overlay sitting directly on the base's current layers.
    pub fn new_overlay(&self) -> IndexOverlay<I> {
        let inner = self.inner.read();
        IndexOverlay::<I>::new(inner.version.map_or(0, |v| v + 1), inner.layers.clone())
    }

    /// The guard [`Self::fold`] and [`Self::with_view`] require. Walks the
    /// ancestry rather than using `can_view_after`, which an abandoned
    /// sibling fork also satisfies while sharing no ancestry.
    pub fn is_ancestor_of(&self, overlay: &IndexOverlay<I>) -> bool {
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
        overlay: &IndexOverlay<I>,
        f: impl FnOnce(&AccountIndexView<'_, I>) -> R,
    ) -> R {
        self.read(|base_view| {
            // Under the lock, so the layers checked are the ones merged
            // against; checking before locking lets a fold land in between.
            assert!(
                overlay.descends_from(base_view.layers()),
                "{} overlay does not descend from the base it would be merged over",
                I::NAME,
            );
            let mut by_shard: Vec<OverlayByAccount<I>> = (0..NUM_NATIVE_INDEX_SHARDS)
                .map(|_| OverlayByAccount::<I>::default())
                .collect();
            for (key, write) in overlay.writes_since(base_view.layers()) {
                let account = key.account();
                by_shard[shard_of(&account)]
                    .entry(account)
                    .or_default()
                    .push((key.entry(), write));
            }
            f(&AccountIndexView {
                base: base_view,
                by_shard,
            })
        })
    }

    /// Applies everything written since the base layers. Returns whether
    /// the base moved; declines an overlay at or behind the base, or one
    /// not descended from it, re-checked here under the write lock.
    pub fn fold(&self, overlay: &IndexOverlay<I>) -> bool {
        let mut inner = self.inner.write();
        if inner.version.is_some() && overlay.next_version() <= inner.version.map_or(0, |v| v + 1) {
            return false;
        }
        if !(0..NUM_NATIVE_INDEX_SHARDS)
            .all(|shard| overlay.shards()[shard].is_descendant_of(&inner.layers[shard]))
        {
            return false;
        }
        for shard in 0..NUM_NATIVE_INDEX_SHARDS {
            let view = overlay.shards()[shard].view_layers_after(&inner.layers[shard]);
            for (key, write) in view.iter() {
                let account = key.account();
                match write {
                    Some(value) => {
                        inner.shards[shard]
                            .entry(account)
                            .or_default()
                            .insert(key.entry(), value);
                    },
                    None => {
                        // Drop the account with its last entry, or it
                        // lingers empty and reads as a live account.
                        if let Some(entries) = inner.shards[shard].get_mut(&account) {
                            entries.remove(&key.entry());
                            if entries.is_empty() {
                                inner.shards[shard].remove(&account);
                            }
                        }
                    },
                }
            }
        }
        inner.layers = overlay.shards().clone();
        inner.version = overlay.version();
        true
    }
}

/// The overlay's writes grouped by account, bucketed per shard. Built once
/// per view, affordable because the overlay holds only writes since the
/// last fold; sorting it instead would cost a full-key sort for nothing.
type OverlayByAccount<I> =
    AHashMap<AccountKey, Vec<(Entry<I>, Option<<I as AccountIndex>::Value>)>>;

/// An overlay merged over the base it sits on, valid for the scope of
/// [`AccountIndexBase::with_view`].
pub struct AccountIndexView<'a, I: AccountIndex> {
    base: &'a AccountIndexBaseView<'a, I>,
    by_shard: Vec<OverlayByAccount<I>>,
}

impl<I: AccountIndex> AccountIndexView<'_, I> {
    /// Visits each of the exchange's accounts exactly once, with its
    /// full entry set.
    ///
    /// Scoped to the exchange by a range rather than a filter, so it
    /// costs what the exchange holds rather than what every exchange
    /// holds together. The base is nested by account, so the grouping
    /// is structural. Only the overlay's accounts need merging in, and
    /// there are at most one chunk's worth.
    pub fn for_each_account_in_exchange<T, F>(&self, exchange: AccountAddress, f: F) -> Vec<T>
    where
        T: Send,
        F: Fn(&AccountKey, &[(Entry<I>, I::Value)]) -> Option<T> + Sync + Send,
    {
        (0..NUM_NATIVE_INDEX_SHARDS)
            .into_par_iter()
            .flat_map_iter(|shard| {
                let mut out = Vec::new();
                let mut buf: Vec<(Entry<I>, I::Value)> = Vec::new();

                for (account, entries) in self.base.accounts_in_exchange(shard, exchange) {
                    buf.clear();
                    for (entry, value) in entries {
                        match self.overlay_write(account, entry) {
                            // Overridden or deleted by a later write.
                            Some(Some(newer)) => buf.push((*entry, newer)),
                            Some(None) => {},
                            None => buf.push((*entry, value.clone())),
                        }
                    }
                    if let Some(writes) = self.by_shard[shard_of(account)].get(account) {
                        for (entry, write) in writes {
                            if let Some(value) = write
                                && !entries.contains_key(entry)
                            {
                                buf.push((*entry, value.clone()));
                            }
                        }
                        buf.sort_by_key(|(entry, _)| *entry);
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
                    let mut entries: Vec<_> = writes
                        .iter()
                        .filter_map(|(entry, write)| write.clone().map(|v| (*entry, v)))
                        .collect();
                    entries.sort_by_key(|(entry, _)| *entry);
                    if !entries.is_empty()
                        && let Some(item) = f(account, &entries)
                    {
                        out.push(item);
                    }
                }
                out
            })
            .collect()
    }

    /// The account's overlay write for `entry`, if any. `Some(None)` is
    /// a delete. An account holds few entries, so a linear walk beats
    /// building a second map.
    fn overlay_write(&self, account: &AccountKey, entry: &Entry<I>) -> Option<Option<I::Value>> {
        self.by_shard[shard_of(account)]
            .get(account)
            .and_then(|writes| {
                writes
                    .iter()
                    .find(|(e, _)| e == entry)
                    .map(|(_, write)| write.clone())
            })
    }

    /// The version the base is folded to.
    pub fn base_version(&self) -> Option<Version> {
        self.base.version()
    }

    /// Overlay first, then the base. `None` for a tombstone or a miss.
    pub fn get(&self, key: &I::Key) -> Option<I::Value> {
        match self.overlay_write(&key.account(), &key.entry()) {
            Some(write) => write,
            None => self.base.get(key).cloned(),
        }
    }

    /// Order is shard-interleaved, not address-sorted. Callers needing a
    /// stable order must sort.
    pub fn iter_accounts_for_exchange(&self, exchange: AccountAddress) -> Vec<AccountAddress> {
        self.for_each_account_in_exchange(exchange, |account, entries| {
            (!entries.is_empty()).then_some(account.account)
        })
    }

    pub fn count_entries_for_exchange(&self, exchange: AccountAddress) -> usize {
        self.for_each_account_in_exchange(exchange, |_, entries| Some(entries.len()))
            .into_iter()
            .sum()
    }

    /// One lookup into the account's base entries, merged with its
    /// overlay writes. In entry order.
    pub fn get_account_entries(
        &self,
        exchange: AccountAddress,
        account: AccountAddress,
    ) -> Vec<(Entry<I>, I::Value)> {
        let key = AccountKey { exchange, account };
        let base_entries = self.base.account_entries(&key);
        let mut entries: Vec<(Entry<I>, I::Value)> = base_entries
            .into_iter()
            .flatten()
            .filter_map(|(entry, value)| match self.overlay_write(&key, entry) {
                Some(Some(newer)) => Some((*entry, newer)),
                Some(None) => None,
                None => Some((*entry, value.clone())),
            })
            .collect();
        if let Some(writes) = self.by_shard[shard_of(&key)].get(&key) {
            for (entry, write) in writes {
                if let Some(value) = write
                    && !base_entries.is_some_and(|e| e.contains_key(entry))
                {
                    entries.push((*entry, value.clone()));
                }
            }
            entries.sort_by_key(|(entry, _)| *entry);
        }
        entries
    }
}

/// Decodes one durable JMT row into a base entry.
fn decode_row<I: AccountIndex>(
    row: crate::Result<(StateKey, StateValue)>,
) -> crate::Result<(I::Key, I::Value)> {
    let (state_key, state_value) = row?;
    I::decode_row(&state_key, &state_value)
}
