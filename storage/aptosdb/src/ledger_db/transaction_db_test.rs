// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{AptosDB, ledger_db::transaction_db::TransactionDb};
use aptos_crypto::hash::CryptoHash;
use aptos_proptest_helpers::Index;
use aptos_schemadb::batch::SchemaBatch;
use aptos_storage_interface::Result;
use aptos_temppath::TempPath;
use aptos_types::{
    proptest_types::{AccountInfoUniverse, SignatureCheckedTransactionGen},
    transaction::{Transaction, Version},
};
use proptest::{collection::vec, prelude::*};

proptest! {
    #![proptest_config(ProptestConfig::with_cases(10))]

    #[test]
    fn test_get_transaction(
        universe in any_with::<AccountInfoUniverse>(3),
        gens in vec(
            (any::<Index>(), any::<SignatureCheckedTransactionGen>()),
            1..10
        ),
    ) {
        let tmp_dir = TempPath::new();
        let db = AptosDB::new_for_test(&tmp_dir);
        let transaction_db  = db.ledger_db.transaction_db();
        let txns = init_db(universe, gens, transaction_db);

        let num_txns = txns.len();
        for (version, txn) in txns.into_iter().enumerate() {
            let hash = txn.hash();
            prop_assert_eq!(transaction_db.get_transaction(version as Version).unwrap(), txn);
            prop_assert_eq!(transaction_db.get_transaction_version_by_hash(&hash, num_txns as Version).unwrap(), Some(version as Version));
            if version > 0 {
                prop_assert_eq!(transaction_db.get_transaction_version_by_hash(&hash, version as Version - 1).unwrap(), None);
            }
        }

        prop_assert!(transaction_db.get_transaction(num_txns as Version).is_err());
    }

    #[test]
    fn test_get_transaction_iter(
        universe in any_with::<AccountInfoUniverse>(3),
        gens in vec(
            (any::<Index>(), any::<SignatureCheckedTransactionGen>()),
            1..10
        ),
    ) {
        let tmp_dir = TempPath::new();
        let db = AptosDB::new_for_test(&tmp_dir);
        let transaction_db  = db.ledger_db.transaction_db();
        let txns = init_db(universe, gens, transaction_db);

        let total_num_txns = txns.len();

        let actual = transaction_db
            .get_transaction_iter(0, total_num_txns)
            .unwrap()
            .collect::<Result<Vec<_>>>()
            .unwrap();
        prop_assert_eq!(actual, txns.clone());

        let actual = transaction_db
            .get_transaction_iter(0, total_num_txns + 1)
            .unwrap()
            .collect::<Result<Vec<_>>>()
            .unwrap();
        prop_assert_eq!(actual, txns.clone());

        let actual = transaction_db
            .get_transaction_iter(0, 0)
            .unwrap()
            .collect::<Result<Vec<_>>>()
            .unwrap();
        prop_assert!(actual.is_empty());

        if total_num_txns > 0 {
            let actual = transaction_db
                .get_transaction_iter(0, total_num_txns - 1)
                .unwrap()
                .collect::<Result<Vec<_>>>()
                .unwrap();
            prop_assert_eq!(
                actual,
                txns
                .into_iter()
                .take(total_num_txns - 1)
                .collect::<Vec<_>>()
            );
        }

        prop_assert!(transaction_db.get_transaction_iter(10, usize::MAX).is_err());
    }

    #[test]
    fn test_prune(
        universe in any_with::<AccountInfoUniverse>(3),
        gens in vec(
            (any::<Index>(), any::<SignatureCheckedTransactionGen>()),
            2..10
        ),
    ) {
        let tmp_dir = TempPath::new();
        let db = AptosDB::new_for_test(&tmp_dir);
        let transaction_db  = db.ledger_db.transaction_db();
        let txns = init_db(universe, gens, transaction_db);
        let num_txns = txns.len();

        {
            prop_assert!(transaction_db.get_transaction(0).is_ok());
            let mut batch = SchemaBatch::new();
            transaction_db.prune_transactions(0, 1, &mut batch).unwrap();
            transaction_db.write_schemas(batch).unwrap();
            prop_assert!(transaction_db.get_transaction(0).is_err());
        }

        {
            prop_assert!(transaction_db.get_transaction(1).is_ok());
            prop_assert_eq!(transaction_db.get_transaction_version_by_hash(&txns[1].hash(), num_txns as Version).unwrap(), Some(1));
            let mut batch = SchemaBatch::new();
            transaction_db.prune_transaction_by_hash_indices(std::iter::once(txns[1].hash()), &mut batch).unwrap();
            transaction_db.write_schemas(batch).unwrap();
            prop_assert!(transaction_db.get_transaction(1).is_ok());
            prop_assert_eq!(transaction_db.get_transaction_version_by_hash(&txns[1].hash(), num_txns as Version).unwrap(), None);
        }
    }
}

pub(crate) fn init_db(
    mut universe: AccountInfoUniverse,
    gens: Vec<(Index, SignatureCheckedTransactionGen)>,
    transaction_db: &TransactionDb,
) -> Vec<Transaction> {
    let txns = gens
        .into_iter()
        .map(|(index, txn_gen)| {
            Transaction::UserTransaction(txn_gen.materialize(*index, &mut universe).into_inner())
        })
        .collect::<Vec<_>>();

    assert!(transaction_db.get_transaction(0).is_err());

    transaction_db
        .commit_transactions(0, &txns, /*skip_index=*/ false)
        .unwrap();

    txns
}
