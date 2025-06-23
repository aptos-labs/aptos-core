// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{db::test_helper::put_persisted_auxiliary_info, AptosDB};
use aptos_temppath::TempPath;
use aptos_types::transaction::PersistedAuxiliaryInfo;
use proptest::{collection::vec, prelude::*};

proptest! {
    #![proptest_config(ProptestConfig::with_cases(10))]

    #[test]
    fn test_persisted_auxiliary_info_get_iterator(
        (persisted_info, start_version, num_transactions) in
            vec(any::<PersistedAuxiliaryInfo>(), 1..100)
                .prop_flat_map(|info| {
                    let num_txns = info.len() as u64;
                    (Just(info), 0..num_txns)
                })
                .prop_flat_map(|(info, start_version)| {
                    let num_txns = info.len() as u64;
                    (Just(info), Just(start_version), 0..num_txns as usize * 2)
                })
    ) {
        let tmp_dir = TempPath::new();
        let db = AptosDB::new_for_test(&tmp_dir);
        put_persisted_auxiliary_info(&db, 0, &persisted_info);

        let iter = db.ledger_db.persisted_auxiliary_info_db()
            .get_persisted_auxiliary_info_iter(start_version, num_transactions)
            .unwrap();
        prop_assert_eq!(
           persisted_info
               .into_iter()
               .skip(start_version as usize)
               .take(num_transactions)
               .collect::<Vec<_>>(),
           iter.collect::<Result<Vec<_>, _>>().unwrap()
        );
    }
}
