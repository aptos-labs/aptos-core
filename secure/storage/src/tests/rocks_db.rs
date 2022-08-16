// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::rocks_db::database_schema::{SecureStorageKey, SecureStorageSchema, SecureStorageValue};
use crate::rocks_db::RocksDbStorage;
use crate::tests::suite;
use crate::{GetResponse, Storage};
use aptos_temppath::TempPath;
use schemadb::schema::fuzzing::assert_encode_decode;

#[test]
fn rocks_db() {
    // Run the test suite
    let path_buf = TempPath::new().path().to_path_buf();
    let mut storage = Storage::from(RocksDbStorage::new(path_buf));
    suite::execute_all_storage_tests(&mut storage);

    // Test concurrent storage creation
    test_concurrent_storage_creations();
}

#[test]
fn test_metadata_schema_encode_decode() {
    let serialized_key = serde_json::to_vec("Test key").unwrap();
    let serialized_value = serde_json::to_vec(&GetResponse::new(4567, 10)).unwrap();
    assert_encode_decode::<SecureStorageSchema>(
        &SecureStorageKey::SerializedKey(serialized_key),
        &SecureStorageValue::SerializedValue(serialized_value),
    );
}

fn test_concurrent_storage_creations() {
    // Spawn a number of concurrent threads, all trying to create the same storage file
    let temp_path = TempPath::new().path().to_path_buf();
    let mut thread_handles = Vec::new();
    for _ in 0..10 {
        let temp_path = temp_path.clone();
        thread_handles.push(std::thread::spawn(move || {
            let _storage = RocksDbStorage::new(temp_path);
        }));
    }

    // Verify that no threads failed
    for thread_handle in thread_handles {
        thread_handle.join().unwrap();
    }
}
