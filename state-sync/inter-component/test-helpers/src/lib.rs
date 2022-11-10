// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_infallible::RwLock;
use aptos_types::transaction::{Transaction, WriteSetPayload};
use aptos_vm::AptosVM;
use aptosdb::AptosDB;
use claims::assert_ok;
use executor_test_helpers::bootstrap_genesis;
use std::sync::Arc;
use storage_interface::DbReaderWriter;

pub fn create_database() -> Arc<RwLock<DbReaderWriter>> {
    // Generate a genesis change set
    let (genesis, _) = vm_genesis::test_genesis_change_set_and_validators(Some(1));

    // Create test aptos database
    let db_path = aptos_temppath::TempPath::new();
    assert_ok!(db_path.create_as_dir());
    let (_, db_rw) = DbReaderWriter::wrap(AptosDB::new_for_test(db_path.path()));

    // Bootstrap the genesis transaction
    let genesis_txn = Transaction::GenesisTransaction(WriteSetPayload::Direct(genesis));
    assert_ok!(bootstrap_genesis::<AptosVM>(&db_rw, &genesis_txn));

    Arc::new(RwLock::new(db_rw))
}
