// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{AptosDB, db::test_helper::put_persisted_auxiliary_info};
use aptos_temppath::TempPath;
use aptos_types::transaction::PersistedAuxiliaryInfo;
use proptest::{collection::vec, prelude::*};

fn get_persisted_auxiliary_info(
    db: &AptosDB,
    start_version: u64,
    count: usize,
) -> Vec<PersistedAuxiliaryInfo> {
    db.ledger_db
        .persisted_auxiliary_info_db()
        .get_persisted_auxiliary_info_iter(start_version, count)
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap()
}

#[test]
pub fn test_iterator() {
    let tmp_dir = TempPath::new();
    let db = AptosDB::new_for_test(&tmp_dir);
    let v1_info = PersistedAuxiliaryInfo::V1 {
        transaction_index: 0,
    };

    {
        assert_eq!(
            get_persisted_auxiliary_info(&db, 90, 20),
            vec![PersistedAuxiliaryInfo::None; 20]
        );
    }

    let persisted_info = vec![v1_info; 100];
    put_persisted_auxiliary_info(&db, 100, &persisted_info);

    {
        let mut expected = vec![];
        for _ in 0..10 {
            expected.push(PersistedAuxiliaryInfo::None);
        }
        for _ in 0..10 {
            expected.push(v1_info);
        }
        assert_eq!(get_persisted_auxiliary_info(&db, 90, 20), expected);
    }

    assert_eq!(
        get_persisted_auxiliary_info(&db, 0, 20),
        vec![PersistedAuxiliaryInfo::None; 20]
    );

    assert_eq!(get_persisted_auxiliary_info(&db, 100, 20), vec![
        v1_info;
        20
    ]);

    assert_eq!(get_persisted_auxiliary_info(&db, 190, 20), vec![
        v1_info;
        10
    ]);

    assert_eq!(get_persisted_auxiliary_info(&db, 200, 20), vec![]);
}

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
