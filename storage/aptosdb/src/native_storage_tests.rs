// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Storage-core unit tests for the native-position commit applier
//! and durable layer: `find_prior_version`, stale-index emission,
//! and truncation progress.

use crate::{
    native_state_committer::{new_sharded_kv_batches, InChunkPriorVersions, NativeStateCommitter},
    position_db::PositionDb,
    schema::stale_position_value_index::{StalePositionValueIndex, StalePositionValueIndexSchema},
    utils::truncation_helper::{get_position_commit_progress, truncate_position_db_shards},
};
use aptos_crypto::hash::CryptoHash;
use aptos_temppath::TempPath;
use aptos_types::{state_store::state_key::StateKey, transaction::Version, write_set::WriteOp};
use move_core_types::account_address::AccountAddress;
use std::sync::Arc;

fn open_position_db() -> (TempPath, Arc<PositionDb>) {
    let tmpdir = TempPath::new();
    std::fs::create_dir_all(tmpdir.path()).unwrap();
    let db_paths = aptos_config::config::StorageDirPaths::from_path(tmpdir.path());
    let db = Arc::new(
        PositionDb::new(
            &db_paths,
            aptos_config::config::RocksdbConfig::default(),
            None,
            None,
            /* readonly = */ false,
        )
        .expect("PositionDb::new"),
    );
    (tmpdir, db)
}

fn exchange(byte: u8) -> AccountAddress {
    let mut a = [0u8; AccountAddress::LENGTH];
    a[AccountAddress::LENGTH - 1] = byte;
    AccountAddress::new(a)
}

fn user(byte: u8) -> AccountAddress {
    exchange(byte)
}

fn market(byte: u8) -> AccountAddress {
    exchange(byte)
}

fn position_key(byte: u8) -> StateKey {
    StateKey::position(exchange(1), user(byte), market(1))
}

fn upsert(bytes: &[u8]) -> WriteOp {
    WriteOp::legacy_modification(bytes.to_vec().into())
}

fn delete() -> WriteOp {
    WriteOp::legacy_deletion()
}

/// Commit one position write at `version` using the committer's
/// per-chunk-batched path. Returns the merkle leaf updates.
fn commit_one(
    db: &Arc<PositionDb>,
    version: Version,
    key: StateKey,
    op: WriteOp,
) -> Vec<crate::native_state_committer::MerkleLeafUpdate> {
    let committer = NativeStateCommitter::new(Arc::clone(db));
    let mut sharded_kv_batches = new_sharded_kv_batches();
    let mut in_chunk_prior = InChunkPriorVersions::new();
    let updates = committer
        .apply(
            version,
            std::iter::once((key, op)),
            &mut sharded_kv_batches,
            &mut in_chunk_prior,
        )
        .expect("apply")
        .position;
    db.commit(version, None, sharded_kv_batches)
        .expect("position_db commit");
    updates
}

#[test]
fn find_prior_version_returns_none_for_unwritten_key() {
    let (_tmp, db) = open_position_db();
    let hash = position_key(1).hash();
    assert_eq!(db.find_prior_version(hash, 10).unwrap(), None);
}

#[test]
fn find_prior_version_returns_latest_below_target() {
    let (_tmp, db) = open_position_db();
    let key = position_key(1);
    let hash = key.hash();

    commit_one(&db, 0, key.clone(), upsert(b"v0"));
    commit_one(&db, 5, key.clone(), upsert(b"v5"));
    commit_one(&db, 10, key.clone(), upsert(b"v10"));

    assert_eq!(db.find_prior_version(hash, 11).unwrap(), Some(10));
    assert_eq!(db.find_prior_version(hash, 10).unwrap(), Some(5));
    assert_eq!(db.find_prior_version(hash, 8).unwrap(), Some(5));
    assert_eq!(db.find_prior_version(hash, 5).unwrap(), Some(0));
    assert_eq!(db.find_prior_version(hash, 1).unwrap(), Some(0));
    assert_eq!(db.find_prior_version(hash, 0).unwrap(), None);
}

#[test]
fn apply_emits_no_prev_version_sentinel_for_first_write() {
    let (_tmp, db) = open_position_db();
    let key = position_key(1);
    let hash = key.hash();

    commit_one(&db, 7, key.clone(), upsert(b"v7"));

    let shard = crate::sharded_kv_db::ShardedKvDb::shard_of_hash(hash);
    let mut iter = db
        .shard(shard)
        .iter::<StalePositionValueIndexSchema>()
        .unwrap();
    iter.seek_to_first();
    let mut entries: Vec<StalePositionValueIndex> = Vec::new();
    for row in iter {
        let (idx, _) = row.unwrap();
        if idx.state_key_hash == hash {
            entries.push(idx);
        }
    }
    assert_eq!(entries.len(), 1, "first write emits one stale-index row");
    assert!(
        entries[0].is_first_write(),
        "first write uses NO_PREV_VERSION sentinel"
    );
    assert_eq!(entries[0].stale_since_version, 7);
}

#[test]
fn apply_emits_prior_version_on_overwrite() {
    let (_tmp, db) = open_position_db();
    let key = position_key(1);
    let hash = key.hash();

    commit_one(&db, 0, key.clone(), upsert(b"v0"));
    commit_one(&db, 5, key.clone(), upsert(b"v5"));

    let shard = crate::sharded_kv_db::ShardedKvDb::shard_of_hash(hash);
    let mut iter = db
        .shard(shard)
        .iter::<StalePositionValueIndexSchema>()
        .unwrap();
    iter.seek_to_first();
    let mut entries: Vec<StalePositionValueIndex> = Vec::new();
    for row in iter {
        let (idx, _) = row.unwrap();
        if idx.state_key_hash == hash {
            entries.push(idx);
        }
    }
    assert_eq!(entries.len(), 2);

    // v0 row is the first write (NO_PREV_VERSION sentinel).
    let v0 = entries
        .iter()
        .find(|i| i.stale_since_version == 0)
        .expect("v0 stale-index");
    assert!(v0.is_first_write());

    // v5 row supersedes v0.
    let v5 = entries
        .iter()
        .find(|i| i.stale_since_version == 5)
        .expect("v5 stale-index");
    assert!(!v5.is_first_write());
    assert_eq!(v5.version, 0);
}

#[test]
fn apply_tombstone_carries_no_value_hash() {
    let (_tmp, db) = open_position_db();
    let key = position_key(1);

    let upserts = commit_one(&db, 0, key.clone(), upsert(b"v0"));
    assert!(upserts[0].value_hash.is_some());

    let deletes = commit_one(&db, 1, key.clone(), delete());
    assert_eq!(deletes[0].value_hash, None);
}

#[test]
fn truncate_advances_overall_position_commit_progress() {
    let (_tmp, db) = open_position_db();
    let key = position_key(1);
    let hash = key.hash();

    commit_one(&db, 3, key.clone(), upsert(b"v3"));
    commit_one(&db, 7, key.clone(), upsert(b"v7"));
    assert_eq!(get_position_commit_progress(&db).unwrap(), Some(7));

    truncate_position_db_shards(&db, 3).unwrap();
    assert_eq!(
        get_position_commit_progress(&db).unwrap(),
        Some(3),
        "overall progress marker must reflect the truncated version"
    );

    // The v7 stale-index row is gone; v3 remains.
    let shard = crate::sharded_kv_db::ShardedKvDb::shard_of_hash(hash);
    let mut iter = db
        .shard(shard)
        .iter::<StalePositionValueIndexSchema>()
        .unwrap();
    iter.seek_to_first();
    let stale_since_versions: Vec<Version> = iter
        .filter_map(|r| r.ok())
        .map(|(idx, _)| idx)
        .filter(|i| i.state_key_hash == hash)
        .map(|i| i.stale_since_version)
        .collect();
    assert_eq!(stale_since_versions, vec![3]);
}

#[test]
fn in_chunk_writes_chain_stale_index_versions() {
    let (_tmp, db) = open_position_db();
    let key = position_key(1);
    let hash = key.hash();
    let committer = NativeStateCommitter::new(Arc::clone(&db));
    let mut sharded_kv_batches = new_sharded_kv_batches();
    let mut in_chunk_prior = InChunkPriorVersions::new();

    // Three writes to the same key inside a single chunk.
    for (i, payload) in [b"a".as_slice(), b"b".as_slice(), b"c".as_slice()]
        .iter()
        .enumerate()
    {
        committer
            .apply(
                i as Version,
                std::iter::once((key.clone(), upsert(payload))),
                &mut sharded_kv_batches,
                &mut in_chunk_prior,
            )
            .expect("apply");
    }
    db.commit(2, None, sharded_kv_batches)
        .expect("position_db commit");

    let shard = crate::sharded_kv_db::ShardedKvDb::shard_of_hash(hash);
    let mut iter = db
        .shard(shard)
        .iter::<StalePositionValueIndexSchema>()
        .unwrap();
    iter.seek_to_first();
    let mut entries: Vec<StalePositionValueIndex> = iter
        .filter_map(|r| r.ok())
        .map(|(idx, _)| idx)
        .filter(|i| i.state_key_hash == hash)
        .collect();
    entries.sort_by_key(|i| i.stale_since_version);

    assert_eq!(entries.len(), 3);
    assert!(entries[0].is_first_write(), "v0 is first write");
    assert_eq!(entries[1].version, 0, "v1's stale-index points at v0");
    assert_eq!(entries[2].version, 1, "v2's stale-index points at v1");
}
