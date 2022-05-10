// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use super::*;
use crate::{change_set::ChangeSet, AptosDB};
use aptos_temppath::TempPath;
use ledger_info_test_utils::*;
use proptest::{collection::vec, prelude::*};

proptest! {
    #![proptest_config(ProptestConfig::with_cases(20))]

    #[test]
    fn test_epoch_ending_ledger_infos_iter(
        (ledger_infos_with_sigs, start_epoch, end_epoch) in arb_ledger_infos_with_sigs()
            .prop_flat_map(|ledger_infos_with_sigs| {
                let first_epoch = get_first_epoch(&ledger_infos_with_sigs);
                let last_epoch = get_last_epoch(&ledger_infos_with_sigs);
                (
                    Just(ledger_infos_with_sigs),
                    first_epoch..=last_epoch,
                )
            })
            .prop_flat_map(|(ledger_infos_with_sigs, start_epoch)| {
                let last_epoch = get_last_epoch(&ledger_infos_with_sigs);
                (
                    Just(ledger_infos_with_sigs),
                    Just(start_epoch),
                    (start_epoch..=last_epoch),
                )
            })
    ) {
        let tmp_dir = TempPath::new();
        let db = set_up(&tmp_dir, &ledger_infos_with_sigs);

        let actual = db
            .ledger_store
            .get_epoch_ending_ledger_info_iter(start_epoch, end_epoch)
            .unwrap()
            .collect::<Result<Vec<_>>>()
            .unwrap();

        let expected: Vec<_> = ledger_infos_with_sigs
            .into_iter()
            .filter(|ledger_info_with_sigs| {
                let li = ledger_info_with_sigs.ledger_info();
                start_epoch <= li.epoch()
                    && li.epoch() < end_epoch
                    && li.next_epoch_state().is_some()
            }).collect();
        prop_assert_eq!(actual, expected);
    }

    #[test]
    fn test_get_epoch(
        (ledger_infos_with_sigs, version) in arb_ledger_infos_with_sigs()
            .prop_flat_map(|ledger_infos_with_sigs| {
                let last_version = get_last_version(&ledger_infos_with_sigs);
                (
                    Just(ledger_infos_with_sigs),
                    0..=last_version,
                )
            })
    ) {
        let tmp_dir = TempPath::new();
        let db = set_up(&tmp_dir, &ledger_infos_with_sigs);

        let actual = db.ledger_store.get_epoch(version).unwrap();
        // Find the first LI that is at or after version.
        let index = ledger_infos_with_sigs
            .iter()
            .position(|x| x.ledger_info().version() >= version)
            .unwrap();
        let expected = ledger_infos_with_sigs[index].ledger_info().epoch();
        prop_assert_eq!(actual, expected);
    }

    #[test]
    fn test_get_epoch_state(ledger_infos_with_sigs in arb_ledger_infos_with_sigs()) {
        let tmp_dir = TempPath::new();
        let db = set_up(&tmp_dir, &ledger_infos_with_sigs);

        assert!(db.ledger_store.get_epoch_state(0).is_err());

        for li_with_sigs in ledger_infos_with_sigs {
            let li = li_with_sigs.ledger_info();
            if li.next_epoch_state().is_some() {
                assert_eq!(
                    db.ledger_store.get_epoch_state(li.epoch()+1).unwrap(),
                    *li.next_epoch_state().unwrap(),
                );
            }

        }
    }

    #[test]
    fn test_get_startup_info(
        (ledger_infos_with_sigs, txn_infos) in arb_ledger_infos_with_sigs()
            .prop_flat_map(|lis| {
                let num_committed_txns = get_last_version(&lis) as usize + 1;
                (
                    Just(lis),
                    vec(any::<TransactionInfo>(), num_committed_txns..num_committed_txns + 10),
                )
            })
    ) {
        let tmp_dir = TempPath::new();
        let db = set_up(&tmp_dir, &ledger_infos_with_sigs);
        put_transaction_infos(&db, &txn_infos);

        let startup_info = db.ledger_store.get_startup_info().unwrap().unwrap();
        let latest_li = ledger_infos_with_sigs.last().unwrap().ledger_info();
        assert_eq!(startup_info.latest_ledger_info, *ledger_infos_with_sigs.last().unwrap());
        let expected_epoch_state = if latest_li.next_epoch_state().is_none() {
            Some(db.ledger_store.get_epoch_state(latest_li.epoch()).unwrap())
        } else {
            None
        };
        assert_eq!(startup_info.latest_epoch_state, expected_epoch_state);
        let committed_version = get_last_version(&ledger_infos_with_sigs);
        assert_eq!(
            startup_info.committed_tree_state.state_root_hash,
            txn_infos[committed_version as usize].state_change_hash(),
        );
        let synced_version = (txn_infos.len() - 1) as u64;
        if synced_version > committed_version {
            assert_eq!(
                startup_info.synced_tree_state.unwrap().state_root_hash,
                txn_infos.last().unwrap().state_change_hash(),
            );
        } else {
            assert!(startup_info.synced_tree_state.is_none());
        }
    }
}

fn put_transaction_infos(db: &AptosDB, txn_infos: &[TransactionInfo]) {
    let mut cs = ChangeSet::new();
    db.ledger_store
        .put_transaction_infos(0, txn_infos, &mut cs)
        .unwrap();
    db.db.write_schemas(cs.batch).unwrap()
}
