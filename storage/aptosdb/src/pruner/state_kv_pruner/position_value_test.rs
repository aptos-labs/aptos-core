// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Tests for the `PositionValue` instantiation of the KV pruner.

use super::{
    generics::{PositionValue, PositionValuePrunerManager},
    StateKvPruner,
};
use crate::{
    native_state_committer::{new_sharded_kv_batches, InChunkPriorVersions, NativeStateCommitter},
    position_db::PositionDb,
    pruner::{db_pruner::DBPruner, pruner_manager::PrunerManager},
    schema::{
        position_value::PositionValueSchema,
        stale_position_value_index::StalePositionValueIndexSchema,
    },
    sharded_kv_db::ShardedKvDb,
};
use aptos_config::config::{LedgerPrunerConfig, RocksdbConfig, StorageDirPaths};
use aptos_crypto::{hash::CryptoHash, HashValue};
use aptos_temppath::TempPath;
use aptos_types::{state_store::state_key::StateKey, transaction::Version, write_set::WriteOp};
use move_core_types::account_address::AccountAddress;
use std::{collections::BTreeSet, sync::Arc};

fn open_position_db() -> (TempPath, Arc<PositionDb>) {
    let tmpdir = TempPath::new();
    std::fs::create_dir_all(tmpdir.path()).unwrap();
    let db_paths = StorageDirPaths::from_path(tmpdir.path());
    let db = Arc::new(
        PositionDb::new(&db_paths, RocksdbConfig::default(), None, None, false)
            .expect("PositionDb::new"),
    );
    (tmpdir, db)
}

fn addr(byte: u8) -> AccountAddress {
    let mut a = [0u8; AccountAddress::LENGTH];
    a[AccountAddress::LENGTH - 1] = byte;
    AccountAddress::new(a)
}

fn position_key(user_byte: u8) -> StateKey {
    StateKey::position(addr(1), addr(user_byte), addr(1))
}

fn upsert(bytes: &[u8]) -> WriteOp {
    WriteOp::legacy_modification(bytes.to_vec().into())
}

fn commit_at(db: &Arc<PositionDb>, version: Version, writes: Vec<(StateKey, WriteOp)>) {
    let committer = NativeStateCommitter::new(Arc::clone(db));
    let mut batches = new_sharded_kv_batches();
    let mut in_chunk_prior = InChunkPriorVersions::new();
    committer
        .apply(version, writes, &mut batches, &mut in_chunk_prior)
        .expect("apply");
    db.commit(version, None, batches).expect("commit");
}

fn value_exists(db: &PositionDb, hash: HashValue, version: Version) -> bool {
    let shard = ShardedKvDb::shard_of_hash(hash);
    db.shard(shard)
        .get::<PositionValueSchema>(&(hash, version))
        .unwrap()
        .is_some()
}

fn stale_versions(db: &PositionDb, hash: HashValue) -> Vec<Version> {
    let shard = ShardedKvDb::shard_of_hash(hash);
    let mut iter = db
        .shard(shard)
        .iter::<StalePositionValueIndexSchema>()
        .unwrap();
    iter.seek_to_first();
    let mut v: Vec<Version> = iter
        .filter_map(|r| r.ok())
        .map(|(idx, _)| idx)
        .filter(|i| i.state_key_hash == hash)
        .map(|i| i.stale_since_version)
        .collect();
    v.sort_unstable();
    v
}

#[test]
fn position_pruner_removes_superseded_values_keeps_live() {
    let (_tmp, db) = open_position_db();
    let key = position_key(1);
    let hash = key.hash();

    commit_at(&db, 0, vec![(key.clone(), upsert(b"v0"))]);
    commit_at(&db, 5, vec![(key.clone(), upsert(b"v5"))]);
    commit_at(&db, 10, vec![(key.clone(), upsert(b"v10"))]);

    let pruner = StateKvPruner::<PositionValue>::new(Arc::clone(&db)).unwrap();
    pruner.set_target_version(7);
    pruner.prune(100).unwrap();

    assert_eq!(pruner.progress(), 7);
    assert!(!value_exists(&db, hash, 0), "v0 value pruned");
    assert!(value_exists(&db, hash, 5), "v5 value retained");
    assert!(value_exists(&db, hash, 10), "v10 value retained");
    assert_eq!(
        stale_versions(&db, hash),
        vec![10],
        "only past-target row left"
    );
}

#[test]
fn position_pruner_first_write_removes_index_but_not_value() {
    let (_tmp, db) = open_position_db();
    let key = position_key(1);
    let hash = key.hash();

    commit_at(&db, 3, vec![(key.clone(), upsert(b"v3"))]);

    let pruner = StateKvPruner::<PositionValue>::new(Arc::clone(&db)).unwrap();
    pruner.set_target_version(100);
    pruner.prune(100).unwrap();

    assert!(
        stale_versions(&db, hash).is_empty(),
        "sentinel index row drained"
    );
    assert!(value_exists(&db, hash, 3), "first-write value must survive");
}

#[test]
fn position_pruner_fans_out_across_shards() {
    let (_tmp, db) = open_position_db();
    let keys: Vec<StateKey> = (1..=40u8).map(position_key).collect();
    let hashes: Vec<HashValue> = keys.iter().map(CryptoHash::hash).collect();

    let shards: BTreeSet<usize> = hashes
        .iter()
        .map(|h| ShardedKvDb::shard_of_hash(*h))
        .collect();
    assert!(shards.len() > 1, "test keys must span multiple shards");

    commit_at(
        &db,
        0,
        keys.iter().cloned().map(|k| (k, upsert(b"a"))).collect(),
    );
    commit_at(
        &db,
        1,
        keys.iter().cloned().map(|k| (k, upsert(b"b"))).collect(),
    );

    let pruner = StateKvPruner::<PositionValue>::new(Arc::clone(&db)).unwrap();
    pruner.set_target_version(1);
    pruner.prune(100).unwrap();

    for hash in &hashes {
        assert!(!value_exists(&db, *hash, 0), "v0 pruned in every shard");
        assert!(value_exists(&db, *hash, 1), "live v1 retained");
        assert!(stale_versions(&db, *hash).is_empty(), "stale index drained");
    }
}

/// The manager drives pruning through its background worker.
#[test]
fn position_manager_drives_pruning() {
    let (_tmp, db) = open_position_db();
    let key = position_key(1);
    let hash = key.hash();
    commit_at(&db, 0, vec![(key.clone(), upsert(b"v0"))]);
    commit_at(&db, 5, vec![(key.clone(), upsert(b"v5"))]);

    let manager = PositionValuePrunerManager::new(
        Arc::clone(&db),
        LedgerPrunerConfig {
            enable: true,
            prune_window: 0,
            batch_size: 1,
            user_pruning_window_offset: 0,
        },
    );
    manager.wake_and_wait_pruner(5).unwrap();

    assert!(!value_exists(&db, hash, 0), "v0 collected by the worker");
    assert!(value_exists(&db, hash, 5), "v5 retained");
    assert_eq!(manager.get_min_readable_version(), 5);
}
