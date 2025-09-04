// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{ledger_db::WriteSetDb, VelorDB};
use velor_schemadb::batch::SchemaBatch;
use velor_storage_interface::Result;
use velor_temppath::TempPath;
use velor_types::{
    transaction::{ExecutionStatus, TransactionAuxiliaryData, TransactionOutput, Version},
    write_set::WriteSet,
};
use proptest::{collection::vec, prelude::*};

proptest! {
    #![proptest_config(ProptestConfig::with_cases(10))]

    #[test]
    fn test_get_write_set(
        write_sets in vec(
            any::<WriteSet>(),
            1..10
        ),
    ) {
        let tmp_dir = TempPath::new();
        let db = VelorDB::new_for_test(&tmp_dir);
        let write_set_db  = db.ledger_db.write_set_db();
        init_db(&write_sets, write_set_db);

        let num_write_sets = write_sets.len();
        for (version, write_set) in write_sets.into_iter().enumerate() {
            prop_assert_eq!(write_set_db.get_write_set(version as Version).unwrap(), write_set);
        }

        prop_assert!(write_set_db.get_write_set(num_write_sets as Version).is_err());
    }

    #[test]
    fn test_get_write_set_iter(
        write_sets in vec(
            any::<WriteSet>(),
            1..10
        ),
    ) {
        let tmp_dir = TempPath::new();
        let db = VelorDB::new_for_test(&tmp_dir);
        let write_set_db  = db.ledger_db.write_set_db();
        init_db(&write_sets, write_set_db);

        let num_write_sets = write_sets.len();

        let actual = write_set_db
            .get_write_set_iter(0, num_write_sets)
            .unwrap()
            .collect::<Result<Vec<_>>>()
            .unwrap();
        prop_assert_eq!(actual, write_sets.clone());

        let actual = write_set_db
            .get_write_set_iter(0, num_write_sets + 1)
            .unwrap()
            .collect::<Result<Vec<_>>>()
            .unwrap();
        prop_assert_eq!(actual, write_sets.clone());

        let actual = write_set_db
            .get_write_set_iter(0, 0)
            .unwrap()
            .collect::<Result<Vec<_>>>()
            .unwrap();
        prop_assert!(actual.is_empty());

        if num_write_sets > 0 {
            let actual = write_set_db
                .get_write_set_iter(0, num_write_sets - 1)
                .unwrap()
                .collect::<Result<Vec<_>>>()
                .unwrap();
            prop_assert_eq!(
                actual,
                write_sets
                .into_iter()
                .take(num_write_sets - 1)
                .collect::<Vec<_>>()
            );
        }

        prop_assert!(write_set_db.get_write_set_iter(10, usize::MAX).is_err());
    }

    #[test]
    fn test_prune(
        write_sets in vec(
            any::<WriteSet>(),
            2..10
        ),
    ) {
        let tmp_dir = TempPath::new();
        let db = VelorDB::new_for_test(&tmp_dir);
        let write_set_db  = db.ledger_db.write_set_db();
        init_db(&write_sets, write_set_db);

        {
            prop_assert!(write_set_db.get_write_set(0).is_ok());
            let mut batch = SchemaBatch::new();
            WriteSetDb::prune(0, 1, &mut batch).unwrap();
            write_set_db.write_schemas(batch).unwrap();
            prop_assert!(write_set_db.get_write_set(0).is_err());
        }
    }
}

fn init_db(write_sets: &[WriteSet], write_set_db: &WriteSetDb) {
    assert!(write_set_db.get_write_set(0).is_err());

    let dummy_txn_outs = write_sets
        .iter()
        .map(|write_set| {
            TransactionOutput::new(
                write_set.clone(),
                vec![],
                0,
                ExecutionStatus::Success.into(),
                TransactionAuxiliaryData::default(),
            )
        })
        .collect::<Vec<_>>();

    write_set_db.commit_write_sets(0, &dummy_txn_outs).unwrap();
}
