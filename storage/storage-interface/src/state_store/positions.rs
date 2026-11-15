// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Native position index: the account index keyed by `(exchange, account)`
//! and, within an account, by market.

#![forbid(unsafe_code)]

pub use crate::state_store::account_index::{shard_of, AccountKey};
use crate::state_store::{
    account_index::{
        AccountEntries, AccountIndex, AccountIndexBase, AccountIndexBaseView, AccountIndexKey,
        AccountIndexView,
    },
    native_overlay::{
        NativeOverlay, NativeParent, NativeWrites, ShardedKey, ShardedNativeLayers,
        NUM_NATIVE_INDEX_SHARDS,
    },
};
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
};

pub const NUM_POSITION_SHARDS: usize = NUM_NATIVE_INDEX_SHARDS;

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

impl ShardedKey for PositionKey {
    fn shard(&self) -> usize {
        shard_of(&self.account())
    }
}

impl AccountIndexKey for PositionKey {
    type Entry = AccountAddress;

    fn account(&self) -> AccountKey {
        PositionKey::account(self)
    }

    fn entry(&self) -> AccountAddress {
        self.market
    }
}

/// The position index: an account's positions, by market.
#[derive(Debug)]
pub enum PositionIndex {}

impl AccountIndex for PositionIndex {
    type Key = PositionKey;
    type Value = NativePosition;

    const NAME: &'static str = "position";

    fn decode_row(
        state_key: &StateKey,
        state_value: &StateValue,
    ) -> crate::Result<(PositionKey, NativePosition)> {
        let key = position_key_of(state_key)?;
        let position = NativePosition::deserialize(state_value.bytes()).map_err(|e| {
            crate::AptosDbError::Other(format!("native position at startup failed to decode: {e}"))
        })?;
        Ok((key, position))
    }
}

/// An account's positions, by market.
pub type AccountPositions = AccountEntries<PositionIndex>;

pub type ShardedPositionLayers = ShardedNativeLayers<PositionKey, NativePosition>;

/// A block's position writes. `None` deletes.
pub type PositionWrites = NativeWrites<PositionKey, NativePosition>;

/// Speculative overlay: one layer per shard per chunk, chained from the
/// parent's.
pub type PositionOverlay = NativeOverlay<PositionKey, NativePosition>;

pub type PositionParent<'a> = NativeParent<'a, PositionKey, NativePosition>;

pub type PositionBase = AccountIndexBase<PositionIndex>;

pub type PositionBaseView<'a> = AccountIndexBaseView<'a, PositionIndex>;

/// A [`PositionOverlay`] merged over the base it sits on.
pub type NativeStateView<'a> = AccountIndexView<'a, PositionIndex>;

impl AccountIndexBaseView<'_, PositionIndex> {
    pub fn account_positions(&self, account: &AccountKey) -> Option<&AccountPositions> {
        self.account_entries(account)
    }

    pub fn num_positions(&self) -> usize {
        self.num_entries()
    }
}

impl AccountIndexView<'_, PositionIndex> {
    pub fn iter_position_accounts_for_exchange(
        &self,
        exchange: AccountAddress,
    ) -> Vec<AccountAddress> {
        self.iter_accounts_for_exchange(exchange)
    }

    pub fn count_positions_for_exchange(&self, exchange: AccountAddress) -> usize {
        self.count_entries_for_exchange(exchange)
    }

    /// In market order.
    pub fn get_account_positions(
        &self,
        exchange: AccountAddress,
        account: AccountAddress,
    ) -> Vec<(AccountAddress, NativePosition)> {
        self.get_account_entries(exchange, account)
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

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

    #[test]
    fn layers_above_counts_unfolded_chunks() {
        let base = base_with(Vec::new());
        let floor = base.layers();
        let overlay = base
            .new_overlay()
            .extend(&floor, 1, vec![(key(2, 9), Some(position(1)))])
            .extend(&floor, 2, Vec::new())
            .extend(&floor, 3, vec![(key(5, 1), Some(position(3)))]);
        assert_eq!(overlay.layers_above(&floor), 3);

        let mid = base
            .new_overlay()
            .extend(&floor, 1, vec![(key(2, 9), Some(position(1)))]);
        let overlay = mid.extend(&floor, 2, Vec::new());
        assert!(base.fold(&mid));
        assert_eq!(overlay.layers_above(&base.layers()), 1);
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
