// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    quorum_store::{
        quorum_store_db::{QuorumStoreDB, QuorumStoreStorage},
        types::{Batch, PersistedValue},
    },
    test_utils::create_vec_signed_transactions,
};
use aptos_temppath::TempPath;
use aptos_types::{account_address::AccountAddress, quorum_store::BatchId};
use claims::assert_ok;

#[test]
fn test_db_for_data() {
    let tmp_dir = TempPath::new();
    let db = QuorumStoreDB::new(&tmp_dir);

    let source = AccountAddress::random();
    let signed_txns = create_vec_signed_transactions(100);
    let persist_request_1: PersistedValue =
        Batch::new(BatchId::new_for_test(1), signed_txns, 1, 20, source, 0).into();
    let clone_1 = persist_request_1.clone();
    assert!(db.save_batch(clone_1).is_ok());

    assert_eq!(
        db.get_batch(persist_request_1.digest())
            .expect("could not read from db")
            .unwrap(),
        persist_request_1
    );

    let signed_txns = create_vec_signed_transactions(200);
    let persist_request_2: PersistedValue =
        Batch::new(BatchId::new_for_test(1), signed_txns, 1, 20, source, 0).into();
    let clone_2 = persist_request_2.clone();
    assert_ok!(db.save_batch(clone_2));

    let signed_txns = create_vec_signed_transactions(300);
    let persist_request_3: PersistedValue =
        Batch::new(BatchId::new_for_test(1), signed_txns, 1, 20, source, 0).into();
    let clone_3 = persist_request_3.clone();
    assert_ok!(db.save_batch(clone_3));

    let batches = vec![*persist_request_3.digest()];
    assert_ok!(db.delete_batches(batches));
    assert_eq!(
        db.get_batch(persist_request_3.digest())
            .expect("could not read from db"),
        None
    );

    let all_batches = db.get_all_batches().expect("could not read from db");
    assert_eq!(all_batches.len(), 2);
    assert!(all_batches.contains_key(persist_request_1.digest()));
    assert!(all_batches.contains_key(persist_request_2.digest()));
}

#[test]
fn test_db_for_batch_id() {
    let tmp_dir = TempPath::new();
    let db = QuorumStoreDB::new(&tmp_dir);

    assert!(db
        .clean_and_get_batch_id(0)
        .expect("could not read from db")
        .is_none());
    assert_ok!(db.save_batch_id(0, BatchId::new_for_test(0)));
    assert_ok!(db.save_batch_id(0, BatchId::new_for_test(4)));
    assert_eq!(
        db.clean_and_get_batch_id(0)
            .expect("could not read from db")
            .unwrap(),
        BatchId::new_for_test(4)
    );
    assert_ok!(db.save_batch_id(1, BatchId::new_for_test(1)));
    assert_ok!(db.save_batch_id(2, BatchId::new_for_test(2)));
    assert_eq!(
        db.clean_and_get_batch_id(2)
            .expect("could not read from db")
            .unwrap(),
        BatchId::new_for_test(2)
    );
}
