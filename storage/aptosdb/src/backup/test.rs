// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    test_helper::{arb_blocks_to_commit, update_in_memory_state},
    AptosDB,
};
use anyhow::Result;
use aptos_temppath::TempPath;
use aptos_types::transaction::Version;
use proptest::prelude::*;
use storage_interface::DbWriter;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(10))]

    #[test]
    fn test_get_transaction_iter(input in arb_blocks_to_commit()) {
        let tmp_dir = TempPath::new();
        let db = AptosDB::new_for_test(&tmp_dir);
        let mut in_memory_state = db.state_store.buffered_state().lock().current_state().clone();
        let _ancestor = in_memory_state.base.clone();
        let mut cur_ver: Version = 0;
        for (txns_to_commit, ledger_info_with_sigs) in input.iter() {
            update_in_memory_state(&mut in_memory_state, txns_to_commit.as_slice());
            db.save_transactions(txns_to_commit, cur_ver, cur_ver.checked_sub(1), Some(ledger_info_with_sigs), true, in_memory_state.clone())
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
