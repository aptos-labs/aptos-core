// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{db::test_helper::put_transaction_infos, VelorDB};
use velor_crypto::{hash::CryptoHash, HashValue};
use velor_temppath::TempPath;
use velor_types::transaction::{TransactionInfo, Version};
use proptest::{collection::vec, prelude::*};

fn verify(
    db: &VelorDB,
    txn_infos: &[TransactionInfo],
    first_version: Version,
    ledger_version: Version,
    root_hash: HashValue,
) {
    txn_infos
        .iter()
        .enumerate()
        .for_each(|(idx, expected_txn_info)| {
            let version = first_version + idx as u64;

            let txn_info_with_proof = db
                .ledger_db
                .transaction_info_db()
                .get_transaction_info_with_proof(
                    version,
                    ledger_version,
                    db.ledger_db.transaction_accumulator_db(),
                )
                .unwrap();

            assert_eq!(txn_info_with_proof.transaction_info(), expected_txn_info);
            txn_info_with_proof
                .ledger_info_to_transaction_info_proof()
                .verify(
                    root_hash,
                    txn_info_with_proof.transaction_info().hash(),
                    version,
                )
                .unwrap();
        })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(10))]

    #[test]
    fn test_transaction_info_put_get_verify(
        batch1 in vec(any::<TransactionInfo>(), 1..100),
        batch2 in vec(any::<TransactionInfo>(), 1..100),
    ) {
        let tmp_dir = TempPath::new();
        let db = VelorDB::new_for_test(&tmp_dir);

        // insert two batches of transaction infos
        let root_hash1 = put_transaction_infos(&db, 0, &batch1);
        let ledger_version1 = batch1.len() as u64 - 1;
        let root_hash2 = put_transaction_infos(&db, batch1.len() as u64, &batch2);
        let ledger_version2 = batch1.len() as u64 + batch2.len() as u64 - 1;

        // retrieve all leaves and verify against latest root hash
        verify(&db, &batch1, 0, ledger_version2, root_hash2);
        verify(&db, &batch2, batch1.len() as u64, ledger_version2, root_hash2);

        // retrieve batch1 and verify against root_hash after batch1 was inserted
        verify(&db, &batch1, 0, ledger_version1, root_hash1);
    }

    #[test]
    fn test_transaction_info_get_iterator(
        (infos, start_version, num_transaction_infos) in
            vec(any::<TransactionInfo>(), 1..100)
                .prop_flat_map(|infos| {
                    let num_infos = infos.len() as u64;
                    (Just(infos), 0..num_infos)
                })
                .prop_flat_map(|(infos, start_version)| {
                    let num_infos = infos.len() as u64;
                    (Just(infos), Just(start_version), 0..num_infos as usize * 2)
                })
    ) {
        let tmp_dir = TempPath::new();
        let db = VelorDB::new_for_test(&tmp_dir);
        put_transaction_infos(&db, 0, &infos);

        let iter = db.ledger_db.transaction_info_db()
            .get_transaction_info_iter(start_version, num_transaction_infos)
            .unwrap();
        prop_assert_eq!(
            infos
                .into_iter()
                .skip(start_version as usize)
                .take(num_transaction_infos)
                .collect::<Vec<_>>(),
            iter.collect::<Result<Vec<_>, _>>().unwrap()
        );
    }
}
