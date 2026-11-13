// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Native collateral index: the account index keyed by `(exchange, account)`
//! and, within an account, by scope. Shards with the position index, so an
//! account's collateral and positions share a shard.

#![forbid(unsafe_code)]

use crate::state_store::{
    account_index::{
        shard_of, AccountEntries, AccountIndex, AccountIndexBase, AccountIndexBaseView,
        AccountIndexKey, AccountIndexView, AccountKey,
    },
    native_overlay::{NativeOverlay, NativeParent, NativeWrites, ShardedKey, ShardedNativeLayers},
};
use aptos_types::{
    account_address::AccountAddress,
    state_store::{
        native_collateral::NativeCollateral,
        state_key::{
            inner::{CollateralScope, StateKeyInner, TradingNativeKey},
            StateKey,
        },
        state_value::StateValue,
    },
};

/// One collateral entry, keyed as the write set keys it.
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd, Debug)]
pub struct CollateralKey {
    pub exchange: AccountAddress,
    pub account: AccountAddress,
    pub scope: CollateralScope,
}

impl ShardedKey for CollateralKey {
    fn shard(&self) -> usize {
        shard_of(&self.account())
    }
}

impl AccountIndexKey for CollateralKey {
    type Entry = CollateralScope;

    fn account(&self) -> AccountKey {
        AccountKey {
            exchange: self.exchange,
            account: self.account,
        }
    }

    fn entry(&self) -> CollateralScope {
        self.scope
    }
}

/// The collateral index: an account's collateral, by scope.
#[derive(Debug)]
pub enum CollateralIndex {}

impl AccountIndex for CollateralIndex {
    type Key = CollateralKey;
    type Value = NativeCollateral;

    const NAME: &'static str = "collateral";

    fn decode_row(
        state_key: &StateKey,
        state_value: &StateValue,
    ) -> crate::Result<(CollateralKey, NativeCollateral)> {
        let key = collateral_key_of(state_key)?;
        let collateral = NativeCollateral::deserialize(state_value.bytes()).map_err(|e| {
            crate::AptosDbError::Other(format!(
                "native collateral at startup failed to decode: {e}"
            ))
        })?;
        Ok((key, collateral))
    }
}

/// An account's collateral, by scope.
pub type AccountCollateral = AccountEntries<CollateralIndex>;

pub type ShardedCollateralLayers = ShardedNativeLayers<CollateralKey, NativeCollateral>;

/// A block's collateral writes. `None` deletes.
pub type CollateralWrites = NativeWrites<CollateralKey, NativeCollateral>;

pub type CollateralOverlay = NativeOverlay<CollateralKey, NativeCollateral>;

pub type CollateralParent<'a> = NativeParent<'a, CollateralKey, NativeCollateral>;

pub type CollateralBase = AccountIndexBase<CollateralIndex>;

pub type CollateralBaseView<'a> = AccountIndexBaseView<'a, CollateralIndex>;

/// A [`CollateralOverlay`] merged over the base it sits on.
pub type CollateralView<'a> = AccountIndexView<'a, CollateralIndex>;

/// Decodes a block's native-collateral write set into the form both the
/// overlay and the durable commit take.
pub fn decode_collateral_writes<'a, I>(writes: I) -> crate::Result<CollateralWrites>
where
    I: IntoIterator<Item = (&'a StateKey, Option<&'a StateValue>)>,
{
    writes
        .into_iter()
        .map(|(state_key, value)| {
            let key = collateral_key_of(state_key)?;
            let collateral = value
                .map(|sv| NativeCollateral::deserialize(sv.bytes()))
                .transpose()
                .map_err(|e| {
                    crate::AptosDbError::Other(format!("native collateral failed to decode: {e}"))
                })?;
            Ok((key, collateral))
        })
        .collect()
}

pub fn collateral_key_of(state_key: &StateKey) -> crate::Result<CollateralKey> {
    match state_key.inner() {
        StateKeyInner::TradingNative(TradingNativeKey::Collateral {
            exchange,
            account,
            scope,
        }) => Ok(CollateralKey {
            exchange: *exchange,
            account: *account,
            scope: *scope,
        }),
        other => Err(crate::AptosDbError::Other(format!(
            "non-Collateral native StateKey: {other:?}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state_store::positions::PositionKey;

    fn addr(byte: u8) -> AccountAddress {
        let mut a = [0u8; AccountAddress::LENGTH];
        a[AccountAddress::LENGTH - 1] = byte;
        AccountAddress::new(a)
    }

    fn collateral(primary: i64) -> NativeCollateral {
        NativeCollateral::V1 {
            primary_offset: (primary as u64) ^ (1 << 63),
            secondaries: vec![],
            reserved: 0,
        }
    }

    fn cross(account: u8) -> CollateralKey {
        CollateralKey {
            exchange: addr(1),
            account: addr(account),
            scope: CollateralScope::Cross,
        }
    }

    fn isolated(account: u8, market: u8) -> CollateralKey {
        CollateralKey {
            scope: CollateralScope::Isolated {
                market: addr(market),
            },
            ..cross(account)
        }
    }

    fn base_with(seed: Vec<(CollateralKey, NativeCollateral)>) -> CollateralBase {
        CollateralBase::new_at_version(Some(0), "test", seed)
    }

    fn row(key: CollateralKey, primary: i64) -> crate::Result<(StateKey, StateValue)> {
        Ok((
            StateKey::collateral(key.exchange, key.account, key.scope),
            StateValue::new_legacy(collateral(primary).serialize().unwrap().into()),
        ))
    }

    fn read(
        base: &CollateralBase,
        overlay: &CollateralOverlay,
        key: &CollateralKey,
    ) -> Option<i64> {
        base.with_view(overlay, |view| view.get(key).map(|c| c.primary()))
    }

    #[test]
    fn primary_round_trips_through_the_helper() {
        assert_eq!(collateral(-5).primary(), -5);
        assert_eq!(collateral(7).primary(), 7);
    }

    #[test]
    fn cold_load_from_rows() {
        let base = CollateralBase::new_from_rows(Some(3), "test", vec![
            row(cross(2), 100),
            row(isolated(2, 9), -4),
        ])
        .unwrap();
        let overlay = base.new_overlay();
        assert_eq!(read(&base, &overlay, &cross(2)), Some(100));
        assert_eq!(read(&base, &overlay, &isolated(2, 9)), Some(-4));
        assert_eq!(overlay.next_version(), 4);
        assert_eq!(base.read(|v| v.num_entries()), 2);
    }

    #[test]
    fn rows_of_another_kind_fail_to_load() {
        let position = Ok((
            StateKey::position(addr(1), addr(2), addr(3)),
            StateValue::new_legacy(collateral(1).serialize().unwrap().into()),
        ));
        assert!(CollateralBase::new_from_rows(Some(0), "test", vec![position]).is_err());
    }

    #[test]
    fn overlay_shadows_base_and_tombstones_delete() {
        let base = base_with(vec![
            (cross(2), collateral(100)),
            (isolated(2, 9), collateral(50)),
        ]);
        let overlay = base.new_overlay().extend(&base.layers(), 1, vec![
            (cross(2), Some(collateral(300))),
            (isolated(2, 9), None),
        ]);
        assert_eq!(read(&base, &overlay, &cross(2)), Some(300));
        assert!(read(&base, &overlay, &isolated(2, 9)).is_none());
    }

    #[test]
    fn fold_prunes_an_account_that_loses_its_last_scope() {
        let base = base_with(vec![
            (cross(2), collateral(1)),
            (isolated(2, 9), collateral(2)),
        ]);
        let overlay = base.new_overlay().extend(&base.layers(), 1, vec![
            (cross(2), None),
            (isolated(2, 9), None),
        ]);
        assert!(base.fold(&overlay));
        assert_eq!(base.read(|v| v.num_accounts()), 0);
        assert_eq!(base.version(), Some(1));
    }

    #[test]
    fn fold_keeps_an_account_that_still_holds_a_scope() {
        let base = base_with(vec![
            (cross(2), collateral(1)),
            (isolated(2, 9), collateral(2)),
        ]);
        let overlay = base
            .new_overlay()
            .extend(&base.layers(), 1, vec![(isolated(2, 9), None)]);
        assert!(base.fold(&overlay));
        let account = cross(2).account();
        assert_eq!(
            base.read(|v| v.account_entries(&account).map(|e| e.len())),
            Some(1)
        );
    }

    #[test]
    fn account_entries_merge_overlay_over_base_in_scope_order() {
        let base = base_with(vec![
            (isolated(2, 9), collateral(2)),
            (cross(3), collateral(7)),
        ]);
        let overlay = base.new_overlay().extend(&base.layers(), 1, vec![
            (cross(2), Some(collateral(1))),
            (isolated(2, 8), Some(collateral(3))),
        ]);
        let entries = base.with_view(&overlay, |view| {
            view.get_account_entries(addr(1), addr(2))
                .into_iter()
                .map(|(scope, c)| (scope, c.primary()))
                .collect::<Vec<_>>()
        });
        assert_eq!(entries, vec![
            (CollateralScope::Cross, 1),
            (CollateralScope::Isolated { market: addr(8) }, 3),
            (CollateralScope::Isolated { market: addr(9) }, 2),
        ]);
    }

    #[test]
    fn exchange_scan_visits_base_and_overlay_accounts() {
        let base = base_with(vec![(cross(2), collateral(1)), (cross(3), collateral(2))]);
        let overlay = base.new_overlay().extend(&base.layers(), 1, vec![
            (cross(3), None),
            (cross(4), Some(collateral(5))),
        ]);
        let mut accounts =
            base.with_view(&overlay, |view| view.iter_accounts_for_exchange(addr(1)));
        accounts.sort();
        assert_eq!(accounts, vec![addr(2), addr(4)]);
        assert_eq!(
            base.with_view(&overlay, |view| view.count_entries_for_exchange(addr(1))),
            2
        );
    }

    #[test]
    fn sibling_fork_does_not_descend_from_a_folded_base() {
        let base = base_with(Vec::new());
        let parent = base.new_overlay();
        let kept = parent.extend(&base.layers(), 1, vec![(cross(2), Some(collateral(1)))]);
        let sibling = parent.extend(&base.layers(), 1, vec![(cross(2), Some(collateral(9)))]);
        assert!(base.fold(&kept));
        assert!(!base.is_ancestor_of(&sibling));
        assert!(!base.fold(&sibling));
    }

    #[test]
    fn collateral_shards_with_the_accounts_positions() {
        for account in 0..=255u8 {
            let position = PositionKey {
                exchange: addr(1),
                account: addr(account),
                market: addr(7),
            };
            assert_eq!(cross(account).shard(), position.shard());
            assert_eq!(isolated(account, 7).shard(), position.shard());
        }
    }

    #[test]
    fn decode_collateral_writes_reads_sets_and_deletes() {
        let set = StateValue::new_legacy(collateral(4).serialize().unwrap().into());
        let set_key = StateKey::collateral(addr(1), addr(2), CollateralScope::Cross);
        let delete_key = StateKey::collateral(addr(1), addr(2), CollateralScope::Isolated {
            market: addr(9),
        });
        let writes =
            decode_collateral_writes(vec![(&set_key, Some(&set)), (&delete_key, None)]).unwrap();
        assert_eq!(writes.len(), 2);
        assert_eq!(writes[0].0, cross(2));
        assert_eq!(writes[0].1.as_ref().map(|c| c.primary()), Some(4));
        assert_eq!(writes[1], (isolated(2, 9), None));
    }
}
