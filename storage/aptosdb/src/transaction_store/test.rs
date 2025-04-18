// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

#![allow(clippy::redundant_clone)] // Required to work around prop_assert_eq! limitations

use super::*;
use crate::{ledger_db::transaction_db_test::init_db, AptosDB};
use aptos_proptest_helpers::Index;
use aptos_temppath::TempPath;
use aptos_types::proptest_types::{AccountInfoUniverse, SignatureCheckedTransactionGen};
use proptest::{collection::vec, prelude::*};
use std::collections::BTreeMap;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(10))]

    #[test]
    fn test_put_get(
        universe in any_with::<AccountInfoUniverse>(3),
        gens in vec(
            (any::<Index>(), any::<SignatureCheckedTransactionGen>()),
            1..10
        ),
    ) {
        let tmp_dir = TempPath::new();
        let db = AptosDB::new_for_test(&tmp_dir);
        let store = &db.transaction_store;
        let txns = init_db(universe, gens, db.ledger_db.transaction_db());

        let ledger_version = txns.len() as Version - 1;
        for (ver, txn) in txns.iter().enumerate() {
            let user_txn = txn
                .try_as_signed_user_txn()
                .expect("All should be user transactions here.");
            prop_assert_eq!(
                store
                    .get_account_ordered_transaction_version(
                        user_txn.sender(),
                        user_txn.sequence_number(),
                        ledger_version
                    )
                    .unwrap(),
                Some(ver as Version)
            );
        }
    }

    #[test]
    fn test_get_account_transaction_version_iter(
        universe in any_with::<AccountInfoUniverse>(5),
        gens in vec(
            (any::<Index>(), any::<SignatureCheckedTransactionGen>()),
            1..=50,
        ),
        seq_num_offset in 0_u64..=10,
        ledger_version in 0_u64..50,
        num_versions in 0_u64..=50,
    ) {
        let tmp_dir = TempPath::new();
        let db = AptosDB::new_for_test(&tmp_dir);
        let store = &db.transaction_store;
        let txns = init_db(universe, gens, db.ledger_db.transaction_db());

        let txns = txns
            .iter()
            .enumerate()
            .map(|(version, txn)| (version as u64, txn.try_as_signed_user_txn().unwrap()))
            .collect::<Vec<_>>();

        // can we just get all the account transaction versions individually

        for (version, txn) in &txns {
            let mut iter = store.get_account_ordered_transactions_iter(
                txn.sender(),
                txn.sequence_number(),
                1, /* num_versions */
                ledger_version,
            ).unwrap();

            if *version <= ledger_version {
                let (actual_seq_num, actual_version) = iter.next().unwrap().unwrap();
                prop_assert!(iter.next().is_none());

                prop_assert_eq!(*version, actual_version);
                prop_assert_eq!(txn.sequence_number(), actual_seq_num);
            } else {
                prop_assert!(iter.next().is_none());
            }
        }

        // now do a full scan of each account

        // what does the expected view look like
        let mut expected_scan = BTreeMap::<AccountAddress, Vec<(u64, Version)>>::new();
        for (version, txn) in &txns {
            let seq_num = txn.sequence_number();
            if *version <= ledger_version && seq_num >= seq_num_offset {
                let txn_metadatas = expected_scan.entry(txn.sender()).or_default();
                if (txn_metadatas.len() as u64) < num_versions {
                    txn_metadatas.push((seq_num, *version));
                }
            }
        }

        // throw in some non-existent accounts; make sure we don't return anything for them
        expected_scan.entry(AccountAddress::from_hex_literal("0x1234").unwrap()).or_default();
        expected_scan.entry(AccountAddress::from_hex_literal("0x77777777").unwrap()).or_default();
        expected_scan.entry(AccountAddress::from_hex_literal("0x42").unwrap()).or_default();

        // scan the db
        let actual_scan = expected_scan
            .keys()
            .map(|address| {
                let txn_metadatas = store
                    .get_account_ordered_transactions_iter(
                        *address,
                        seq_num_offset,
                        num_versions,
                        ledger_version,
                    )
                    .unwrap()
                    .collect::<Result<Vec<_>>>()
                    .unwrap();
                (*address, txn_metadatas)
            })
            .collect::<BTreeMap<_, _>>();

        prop_assert_eq!(&actual_scan, &expected_scan);
    }
}
