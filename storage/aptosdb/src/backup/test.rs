// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::db::{AptosDB, test_helper::arb_blocks_to_commit};
use anyhow::Result;
use aptos_temppath::TempPath;
use aptos_types::transaction::Version;
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(10))]

    #[test]
    fn test_get_transaction_iter(input in arb_blocks_to_commit()) {
        let tmp_dir = TempPath::new();
        let db = AptosDB::new_for_test(&tmp_dir);
        let mut cur_ver: Version = 0;
        for (txns_to_commit, ledger_info_with_sigs) in input.iter() {
            db.save_transactions_for_test(
                txns_to_commit,
                cur_ver,
                Some(ledger_info_with_sigs),
                true, // sync commit
            )
            .unwrap();
            cur_ver += txns_to_commit.len() as u64;
        }

        let expected: Vec<_> = input
            .iter()
            .flat_map(|(txns_to_commit, _ledger_info_with_sigs)| {
                txns_to_commit
                    .iter()
                    .map(|txn_to_commit| txn_to_commit.transaction().clone())
            })
            .collect();
        prop_assert_eq!(expected.len() as u64, cur_ver);

        let bh = db.get_backup_handler();

        let actual = bh
            .get_transaction_iter(0, cur_ver as usize)
            .unwrap()
            .map(|res| Ok(res?.0))
            .collect::<Result<Vec<_>>>()
            .unwrap();
        prop_assert_eq!(&actual, &expected);

        // try to overfetch, still expect the same result.
        let overfetched = bh
            .get_transaction_iter(0, cur_ver as usize + 10)
            .unwrap()
            .map(|res| Ok(res?.0))
            .collect::<Result<Vec<_>>>()
            .unwrap();
        prop_assert_eq!(&overfetched, &expected);

        let non_existent = bh
            .get_transaction_iter(100000, 100)
            .unwrap()
            .map(|res| Ok(res?.0))
            .collect::<Result<Vec<_>>>()
            .unwrap();
        prop_assert_eq!(&non_existent, &[]);
    }
}
