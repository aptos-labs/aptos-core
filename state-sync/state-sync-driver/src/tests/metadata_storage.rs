// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    metadata_storage::{
        database_schema::{MetadataKey, MetadataSchema, MetadataValue},
        MetadataStorageInterface, PersistentMetadataStorage, StateSnapshotProgress,
    },
    snapshot_kind::SnapshotKind,
    tests::utils::{create_epoch_ending_ledger_info, create_ledger_info_at_version},
};
use aptos_schemadb::schema::fuzzing::assert_encode_decode;
use aptos_temppath::TempPath;
use claims::{assert_err, assert_matches, assert_none};

#[test]
fn test_create_then_open() {
    // Create a new metadata storage
    let tmp_dir = TempPath::new();
    let metadata_storage = PersistentMetadataStorage::new(tmp_dir.path());

    // Verify the storage is empty
    assert_none!(metadata_storage
        .previous_snapshot_sync_target(SnapshotKind::MainState)
        .unwrap());

    // Insert a new state value entry for the target
    let target_ledger_info = create_ledger_info_at_version(12345);
    let last_persisted_state_value = 100000;
    let snapshot_sync_completed = false;
    metadata_storage
        .update_last_persisted_index(
            &target_ledger_info,
            last_persisted_state_value,
            snapshot_sync_completed,
            SnapshotKind::MainState,
        )
        .unwrap();

    // Drop the handle to the storage (mimic a reboot)
    drop(metadata_storage);

    // Create another storage (it should reopen the existing file) and verify the state
    let metadata_storage = PersistentMetadataStorage::new(tmp_dir.path());
    assert_eq!(
        Some(target_ledger_info.clone()),
        metadata_storage
            .previous_snapshot_sync_target(SnapshotKind::MainState)
            .unwrap()
    );
    assert_eq!(
        last_persisted_state_value,
        metadata_storage
            .get_last_persisted_index(&target_ledger_info, SnapshotKind::MainState)
            .unwrap()
    );
    assert_eq!(
        snapshot_sync_completed,
        metadata_storage
            .is_snapshot_sync_complete(&target_ledger_info, SnapshotKind::MainState)
            .unwrap()
    );

    // Insert the next state value entry for the target
    let last_persisted_state_value = 200000;
    let snapshot_sync_completed = true;
    metadata_storage
        .update_last_persisted_index(
            &target_ledger_info,
            last_persisted_state_value,
            snapshot_sync_completed,
            SnapshotKind::MainState,
        )
        .unwrap();

    // Drop the handle to the storage (mimic a reboot)
    drop(metadata_storage);

    // Create another storage (it should reopen the existing file) and verify the state
    let metadata_storage = PersistentMetadataStorage::new(tmp_dir.path());
    assert_eq!(
        Some(target_ledger_info.clone()),
        metadata_storage
            .previous_snapshot_sync_target(SnapshotKind::MainState)
            .unwrap()
    );
    assert_eq!(
        last_persisted_state_value,
        metadata_storage
            .get_last_persisted_index(&target_ledger_info, SnapshotKind::MainState)
            .unwrap()
    );
    assert_eq!(
        snapshot_sync_completed,
        metadata_storage
            .is_snapshot_sync_complete(&target_ledger_info, SnapshotKind::MainState)
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
    assert_encode_decode::<MetadataSchema>(
        &MetadataKey::HotStateSnapshotSync,
        &MetadataValue::HotStateSnapshotSync(StateSnapshotProgress {
            target_ledger_info: create_epoch_ending_ledger_info(),
            last_persisted_state_value_index: 5678,
            snapshot_sync_completed: false,
        }),
    );
}

/// The key/value variants are BCS encoded by index, so appending the hot state
/// variants must not shift the existing ones (that would orphan stored rows).
#[test]
fn test_metadata_schema_variants_are_append_only() {
    assert_eq!(bcs::to_bytes(&MetadataKey::StateSnapshotSync).unwrap(), [0]);
    assert_eq!(
        bcs::to_bytes(&MetadataKey::PositionSnapshotSync).unwrap(),
        [1]
    );
    assert_eq!(
        bcs::to_bytes(&MetadataKey::HotStateSnapshotSync).unwrap(),
        [2]
    );

    // An old main-state row must still decode after the hot variants were added
    let progress = StateSnapshotProgress {
        target_ledger_info: create_epoch_ending_ledger_info(),
        last_persisted_state_value_index: 5678,
        snapshot_sync_completed: true,
    };
    let old_bytes = bcs::to_bytes(&MetadataValue::StateSnapshotSync(progress)).unwrap();
    assert_eq!(old_bytes[0], 0);
    assert_matches!(
        bcs::from_bytes::<MetadataValue>(&old_bytes).unwrap(),
        MetadataValue::StateSnapshotSync(_)
    );
}

/// Each stage owns an independent row: completing one never completes another,
/// and a stale target for one stage is rejected without disturbing the others.
#[test]
fn test_independent_snapshot_stage_progress() {
    // Create a new metadata storage
    let tmp_dir = TempPath::new();
    let metadata_storage = PersistentMetadataStorage::new(tmp_dir.path());

    // Verify every stage starts empty
    let target_ledger_info = create_ledger_info_at_version(12345);
    for kind in [
        SnapshotKind::MainState,
        SnapshotKind::HotState,
        SnapshotKind::Position,
    ] {
        assert_none!(metadata_storage
            .previous_snapshot_sync_target(kind)
            .unwrap());
        assert_err!(metadata_storage.is_snapshot_sync_complete(&target_ledger_info, kind));
    }

    // Complete the main state stage and start (but don't complete) the hot stage
    metadata_storage
        .update_last_persisted_index(&target_ledger_info, 999, true, SnapshotKind::MainState)
        .unwrap();
    metadata_storage
        .update_last_persisted_index(&target_ledger_info, 42, false, SnapshotKind::HotState)
        .unwrap();

    // Drop the handle to the storage (mimic a reboot)
    drop(metadata_storage);
    let metadata_storage = PersistentMetadataStorage::new(tmp_dir.path());

    // Main state is complete, hot state is not, and position never started
    assert!(metadata_storage
        .is_snapshot_sync_complete(&target_ledger_info, SnapshotKind::MainState)
        .unwrap());
    assert!(!metadata_storage
        .is_snapshot_sync_complete(&target_ledger_info, SnapshotKind::HotState)
        .unwrap());
    assert_eq!(
        42,
        metadata_storage
            .get_last_persisted_index(&target_ledger_info, SnapshotKind::HotState)
            .unwrap()
    );
    assert_none!(metadata_storage
        .previous_snapshot_sync_target(SnapshotKind::Position)
        .unwrap());

    // A hot stage update against a different target is rejected
    let other_target_ledger_info = create_ledger_info_at_version(54321);
    metadata_storage
        .update_last_persisted_index(&other_target_ledger_info, 43, false, SnapshotKind::HotState)
        .unwrap_err();

    // Completing the hot stage leaves the main stage untouched
    metadata_storage
        .update_last_persisted_index(&target_ledger_info, 43, true, SnapshotKind::HotState)
        .unwrap();
    assert!(metadata_storage
        .is_snapshot_sync_complete(&target_ledger_info, SnapshotKind::HotState)
        .unwrap());
    assert_eq!(
        999,
        metadata_storage
            .get_last_persisted_index(&target_ledger_info, SnapshotKind::MainState)
            .unwrap()
    );
}

#[test]
fn test_multiple_reads_and_writes() {
    // Create a new metadata storage
    let tmp_dir = TempPath::new();
    let metadata_storage = PersistentMetadataStorage::new(tmp_dir.path());

    // Verify the storage is empty
    let target_ledger_info = create_ledger_info_at_version(100000);
    assert_none!(metadata_storage
        .previous_snapshot_sync_target(SnapshotKind::MainState)
        .unwrap());
    assert_err!(
        metadata_storage.is_snapshot_sync_complete(&target_ledger_info, SnapshotKind::MainState)
    );
    assert_err!(
        metadata_storage.get_last_persisted_index(&target_ledger_info, SnapshotKind::MainState)
    );

    // Do multiple writes
    for index in 0..100 {
        // Insert a new state value entry for the target
        let last_persisted_state_value = 50000 + index;
        let snapshot_sync_completed = false;
        metadata_storage
            .update_last_persisted_index(
                &target_ledger_info,
                last_persisted_state_value,
                snapshot_sync_completed,
                SnapshotKind::MainState,
            )
            .unwrap();

        // Fetch and verify the last state value entry
        assert_eq!(
            Some(target_ledger_info.clone()),
            metadata_storage
                .previous_snapshot_sync_target(SnapshotKind::MainState)
                .unwrap()
        );
        assert_eq!(
            last_persisted_state_value,
            metadata_storage
                .get_last_persisted_index(&target_ledger_info, SnapshotKind::MainState)
                .unwrap()
        );
        assert_eq!(
            snapshot_sync_completed,
            metadata_storage
                .is_snapshot_sync_complete(&target_ledger_info, SnapshotKind::MainState)
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
    assert_none!(metadata_storage
        .previous_snapshot_sync_target(SnapshotKind::MainState)
        .unwrap());

    // Write a new progress entry into the storage
    let target_ledger_info = create_ledger_info_at_version(100);
    metadata_storage
        .update_last_persisted_index(&target_ledger_info, 10101, false, SnapshotKind::MainState)
        .unwrap();

    // Write another progress entry with a different target and verify that it fails
    let target_ledger_info = create_ledger_info_at_version(200);
    metadata_storage
        .update_last_persisted_index(&target_ledger_info, 10101, false, SnapshotKind::MainState)
        .unwrap_err();
}
