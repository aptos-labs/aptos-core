// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{AptosDB, db::test_helper::put_transaction_auxiliary_data};
use aptos_temppath::TempPath;
use aptos_types::transaction::TransactionAuxiliaryData;
use proptest::{collection::vec, prelude::*};

proptest! {
    #![proptest_config(ProptestConfig::with_cases(10))]

    #[test]
    fn test_transaction_auxiliary_data_get_iterator(
        (txns, start_version, num_transactions) in
            vec(any::<TransactionAuxiliaryData>(), 1..100)
                .prop_flat_map(|txns| {
                    let num_txns = txns.len() as u64;
                    (Just(txns), 0..num_txns)
                })
                .prop_flat_map(|(txns, start_version)| {
                    let num_txns = txns.len() as u64;
                    (Just(txns), Just(start_version), 0..num_txns as usize * 2)
                })
    ) {
        let tmp_dir = TempPath::new();
        let db = AptosDB::new_for_test(&tmp_dir);
        put_transaction_auxiliary_data(&db, 0, &txns);

        let iter = db.ledger_db.transaction_auxiliary_data_db()
            .get_transaction_auxiliary_data_iter(start_version, num_transactions)
            .unwrap();
        prop_assert_eq!(
            txns
                .into_iter()
                .skip(start_version as usize)
                .take(num_transactions)
                .collect::<Vec<_>>(),
            iter.collect::<Result<Vec<_>, _>>().unwrap()
        );
    }
}
