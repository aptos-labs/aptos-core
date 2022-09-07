// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    metadata_storage::{
        database_schema::{MetadataKey, MetadataSchema, MetadataValue},
        MetadataStorageInterface, PersistentMetadataStorage, StateSnapshotProgress,
    },
    tests::utils::{create_epoch_ending_ledger_info, create_ledger_info_at_version},
};
use aptos_temppath::TempPath;
use claims::{assert_err, assert_none};
use schemadb::schema::fuzzing::assert_encode_decode;

#[test]
fn test_create_then_open() {
    // Create a new metadata storage
    let tmp_dir = TempPath::new();
    let metadata_storage = PersistentMetadataStorage::new(tmp_dir.path());

    // Verify the storage is empty
    assert_none!(metadata_storage.previous_snapshot_sync_target().unwrap());

    // Insert a new state value entry for the target
    let target_ledger_info = create_ledger_info_at_version(12345);
    let last_persisted_state_value = 100000;
    let snapshot_sync_completed = false;
    metadata_storage
        .update_last_persisted_state_value_index(
            &target_ledger_info,
            last_persisted_state_value,
            snapshot_sync_completed,
        )
        .unwrap();

    // Drop the handle to the storage (mimic a reboot)
    drop(metadata_storage);

    // Create another storage (it should reopen the existing file) and verify the state
    let metadata_storage = PersistentMetadataStorage::new(tmp_dir.path());
    assert_eq!(
        Some(target_ledger_info.clone()),
        metadata_storage.previous_snapshot_sync_target().unwrap()
    );
    assert_eq!(
        last_persisted_state_value,
        metadata_storage
            .get_last_persisted_state_value_index(&target_ledger_info)
            .unwrap()
    );
    assert_eq!(
        snapshot_sync_completed,
        metadata_storage
            .is_snapshot_sync_complete(&target_ledger_info)
            .unwrap()
    );

    // Insert the next state value entry for the target
    let last_persisted_state_value = 200000;
    let snapshot_sync_completed = true;
    metadata_storage
        .update_last_persisted_state_value_index(
            &target_ledger_info,
            last_persisted_state_value,
            snapshot_sync_completed,
        )
        .unwrap();

    // Drop the handle to the storage (mimic a reboot)
    drop(metadata_storage);

    // Create another storage (it should reopen the existing file) and verify the state
    let metadata_storage = PersistentMetadataStorage::new(tmp_dir.path());
    assert_eq!(
        Some(target_ledger_info.clone()),
        metadata_storage.previous_snapshot_sync_target().unwrap()
    );
    assert_eq!(
        last_persisted_state_value,
        metadata_storage
            .get_last_persisted_state_value_index(&target_ledger_info)
            .unwrap()
    );
    assert_eq!(
        snapshot_sync_completed,
        metadata_storage
            .is_snapshot_sync_complete(&target_ledger_info)
            .unwrap()
    );
}

#[test]
fn test_metadata_schema_encode_decode() {
    assert_encode_decode::<MetadataSchema>(
        &MetadataKey::StateSnapshotSync,
        &MetadataValue::StateSnapshotSync(StateSnapshotProgress {
            target_ledger_info: create_epoch_ending_ledger_info(),
            last_persisted_state_value_index: 5678,
            snapshot_sync_completed: false,
        }),
    );
}

#[test]
fn test_multiple_reads_and_writes() {
    // Create a new metadata storage
    let tmp_dir = TempPath::new();
    let metadata_storage = PersistentMetadataStorage::new(tmp_dir.path());

    // Verify the storage is empty
    let target_ledger_info = create_ledger_info_at_version(100000);
    assert_none!(metadata_storage.previous_snapshot_sync_target().unwrap());
    assert_err!(metadata_storage.is_snapshot_sync_complete(&target_ledger_info));
    assert_err!(metadata_storage.get_last_persisted_state_value_index(&target_ledger_info));

    // Do multiple writes
    for index in 0..100 {
        // Insert a new state value entry for the target
        let last_persisted_state_value = 50000 + index;
        let snapshot_sync_completed = false;
        metadata_storage
            .update_last_persisted_state_value_index(
                &target_ledger_info,
                last_persisted_state_value,
                snapshot_sync_completed,
            )
            .unwrap();

        // Fetch and verify the last state value entry
        assert_eq!(
            Some(target_ledger_info.clone()),
            metadata_storage.previous_snapshot_sync_target().unwrap()
        );
        assert_eq!(
            last_persisted_state_value,
            metadata_storage
                .get_last_persisted_state_value_index(&target_ledger_info)
                .unwrap()
        );
        assert_eq!(
            snapshot_sync_completed,
            metadata_storage
                .is_snapshot_sync_complete(&target_ledger_info)
                .unwrap()
        );
    }
}

#[test]
fn test_writes_to_different_targets() {
    // Create a new metadata storage
    let tmp_dir = TempPath::new();
    let metadata_storage = PersistentMetadataStorage::new(tmp_dir.path());

    // Verify the storage is empty
    assert_none!(metadata_storage.previous_snapshot_sync_target().unwrap());

    // Write a new progress entry into the storage
    let target_ledger_info = create_ledger_info_at_version(100);
    metadata_storage
        .update_last_persisted_state_value_index(&target_ledger_info, 10101, false)
        .unwrap();

    // Write another progress entry with a different target and verify that it fails
    let target_ledger_info = create_ledger_info_at_version(200);
    metadata_storage
        .update_last_persisted_state_value_index(&target_ledger_info, 10101, false)
        .unwrap_err();
}
