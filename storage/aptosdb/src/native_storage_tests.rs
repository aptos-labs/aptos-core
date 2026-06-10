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
use aptos_types::{
    state_store::{native_position::NativePosition, state_key::StateKey},
    transaction::Version,
    write_set::WriteOp,
};
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

fn account(byte: u8) -> AccountAddress {
    exchange(byte)
}

fn market(byte: u8) -> AccountAddress {
    exchange(byte)
}

fn position_key(byte: u8) -> StateKey {
    StateKey::position(exchange(1), account(byte), market(1))
}

/// Build a valid BCS-encoded `NativePosition` whose `size` carries
/// `tag` — lets tests distinguish writes without needing the bytes
/// to look like a particular string.
fn position_bytes(tag: u64) -> Vec<u8> {
    NativePosition::PerpV1 {
        size: tag,
        is_long: true,
        entry_px_times_size_sum: 0,
        avg_acquire_entry_px: 0,
        user_leverage: 1,
        is_isolated: false,
        funding_index_at_last_update: 0,
        unrealized_funding_amount_before_last_update: 0,
        timestamp: 0,
    }
    .serialize()
    .expect("NativePosition serialize")
}

fn upsert(tag: u64) -> WriteOp {
    WriteOp::legacy_modification(position_bytes(tag).into())
}

fn delete() -> WriteOp {
    WriteOp::legacy_deletion()
}

/// Commit one position write at `version` using the committer's
/// per-chunk-batched path. Returns the decoded `PositionWrite`s the
/// committer emitted for this chunk.
fn commit_one(
    db: &Arc<PositionDb>,
    version: Version,
    key: StateKey,
    op: WriteOp,
) -> Vec<crate::native_state_committer::PositionWrite> {
    let committer = NativeStateCommitter::new(Arc::clone(db));
    let mut sharded_kv_batches = new_sharded_kv_batches();
    let mut in_chunk_prior = InChunkPriorVersions::new();
    let writes = committer
        .apply(
            version,
            std::iter::once((key, op)),
            &mut sharded_kv_batches,
            &mut in_chunk_prior,
        )
        .expect("apply");
    db.commit(version, None, sharded_kv_batches)
        .expect("position_db commit");
    writes
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

    commit_one(&db, 0, key.clone(), upsert(0));
    commit_one(&db, 5, key.clone(), upsert(5));
    commit_one(&db, 10, key.clone(), upsert(10));

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

    commit_one(&db, 7, key.clone(), upsert(7));

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

    commit_one(&db, 0, key.clone(), upsert(0));
    commit_one(&db, 5, key.clone(), upsert(5));

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
fn apply_emits_typed_position_for_upsert_none_for_delete() {
    let (_tmp, db) = open_position_db();
    let key = position_key(1);

    let upserts = commit_one(&db, 0, key.clone(), upsert(7));
    assert_eq!(upserts.len(), 1);
    let pos = upserts[0]
        .value
        .as_ref()
        .expect("upsert PositionWrite carries decoded value");
    assert_eq!(pos.size(), 7, "decoded NativePosition's tag survives apply");

    let deletes = commit_one(&db, 1, key.clone(), delete());
    assert_eq!(deletes.len(), 1);
    assert!(
        deletes[0].value.is_none(),
        "delete PositionWrite has no decoded value"
    );
}

/// Verifies that a single `apply` call over a batch of position
/// writes emits one `PositionWrite` per input in arrival order, each
/// with the correct `position_key`, `market`, and decoded value.
#[test]
fn apply_emits_one_position_write_per_input() {
    let (_tmp, db) = open_position_db();
    let committer = NativeStateCommitter::new(Arc::clone(&db));
    let mut sharded_kv_batches = new_sharded_kv_batches();
    let mut in_chunk_prior = InChunkPriorVersions::new();

    let exch = exchange(1);
    let acct_a = account(2);
    let acct_b = account(3);
    let market_x = market(7);
    let market_y = market(8);

    let inputs = vec![
        (
            StateKey::position(exch, acct_a, market_x),
            upsert(100),
        ),
        (
            StateKey::position(exch, acct_b, market_y),
            upsert(200),
        ),
        (StateKey::position(exch, acct_a, market_y), delete()),
    ];

    let writes = committer
        .apply(0, inputs, &mut sharded_kv_batches, &mut in_chunk_prior)
        .expect("apply");

    assert_eq!(writes.len(), 3, "one PositionWrite per input");

    assert_eq!(writes[0].position_key.exchange, exch);
    assert_eq!(writes[0].position_key.account, acct_a);
    assert_eq!(writes[0].market, market_x);
    assert_eq!(writes[0].value.as_ref().unwrap().size(), 100);

    assert_eq!(writes[1].position_key.account, acct_b);
    assert_eq!(writes[1].market, market_y);
    assert_eq!(writes[1].value.as_ref().unwrap().size(), 200);

    assert_eq!(writes[2].position_key.account, acct_a);
    assert_eq!(writes[2].market, market_y);
    assert!(writes[2].value.is_none(), "delete carries no value");
}

#[test]
fn truncate_advances_overall_position_commit_progress() {
    let (_tmp, db) = open_position_db();
    let key = position_key(1);
    let hash = key.hash();

    commit_one(&db, 3, key.clone(), upsert(3));
    commit_one(&db, 7, key.clone(), upsert(7));
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
    for (i, tag) in [11u64, 12, 13].iter().enumerate() {
        committer
            .apply(
                i as Version,
                std::iter::once((key.clone(), upsert(*tag))),
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

mod integration {
    use super::{account, exchange, market};
    use crate::{
        native_state_committer::PositionWrite,
        native_state_store::materialize_user_position_updates,
        native_state_reader::{InMemoryNativeStateReader, NativeStateReader},
        native_state_store::{UserPositionKey, UserPositions},
    };
    use aptos_infallible::Mutex;
    use aptos_types::{state_store::native_position::NativePosition, transaction::Version};
    use std::sync::Arc;

    fn position(size: u64) -> NativePosition {
        NativePosition::PerpV1 {
            size,
            is_long: true,
            entry_px_times_size_sum: 0,
            avg_acquire_entry_px: 0,
            user_leverage: 1,
            is_isolated: false,
            funding_index_at_last_update: 0,
            unrealized_funding_amount_before_last_update: 0,
            timestamp: 0,
        }
    }

    /// Simulates the writer's post-commit fold of one chunk: turns
    /// per-tx `PositionWrite`s into per-account `UserPositionState`
    /// deltas via `materialize_user_position_updates`, then extends
    /// the layered `UserPositions` once.
    fn fold_chunk(
        handle: &Arc<Mutex<UserPositions>>,
        version: Version,
        writes: Vec<PositionWrite>,
    ) {
        let mut up = handle.lock();
        let updates = materialize_user_position_updates(&up, writes);
        *up = up.extend(version, updates);
    }

    /// Writer-shaped chunk fold against the same `Arc<Mutex<UserPositions>>`
    /// the reader holds. Verifies inserts, latest-wins overwrites,
    /// deletes, and cross-exchange isolation across multiple chunks.
    #[test]
    fn reader_sees_writes_folded_through_user_positions() {
        let handle = Arc::new(Mutex::new(UserPositions::new_empty("test")));
        let reader = InMemoryNativeStateReader::new(Arc::clone(&handle));

        let exch_a = exchange(1);
        let exch_b = exchange(2);
        let acct_x = account(10);
        let acct_y = account(11);
        let mkt_p = market(100);
        let mkt_q = market(101);

        // Chunk 0: account X on exchange A opens two markets.
        fold_chunk(&handle, 0, vec![
            PositionWrite {
                position_key: UserPositionKey {
                    exchange: exch_a,
                    account: acct_x,
                },
                market: mkt_p,
                value: Some(position(100)),
            },
            PositionWrite {
                position_key: UserPositionKey {
                    exchange: exch_a,
                    account: acct_x,
                },
                market: mkt_q,
                value: Some(position(200)),
            },
        ]);

        let xa = reader.get_account_positions(exch_a, acct_x);
        let xa_sizes: Vec<u64> = xa.iter().map(|(_, p)| p.size()).collect();
        assert_eq!(xa.len(), 2);
        assert!(xa_sizes.contains(&100));
        assert!(xa_sizes.contains(&200));

        assert_eq!(
            reader.iter_position_accounts_for_exchange(exch_a),
            vec![acct_x]
        );
        assert_eq!(reader.count_positions_for_exchange(exch_a), 2);

        // Exchange B is empty at this point.
        assert!(reader.iter_position_accounts_for_exchange(exch_b).is_empty());
        assert_eq!(reader.count_positions_for_exchange(exch_b), 0);

        // Chunk 1: overwrite mkt_p on X, delete mkt_q on X, open one
        // position for account Y on a different exchange.
        fold_chunk(&handle, 1, vec![
            PositionWrite {
                position_key: UserPositionKey {
                    exchange: exch_a,
                    account: acct_x,
                },
                market: mkt_p,
                value: Some(position(300)),
            },
            PositionWrite {
                position_key: UserPositionKey {
                    exchange: exch_a,
                    account: acct_x,
                },
                market: mkt_q,
                value: None,
            },
            PositionWrite {
                position_key: UserPositionKey {
                    exchange: exch_b,
                    account: acct_y,
                },
                market: mkt_p,
                value: Some(position(50)),
            },
        ]);

        // Account X: latest-wins on mkt_p, mkt_q gone.
        let xa = reader.get_account_positions(exch_a, acct_x);
        assert_eq!(xa.len(), 1);
        assert_eq!(xa[0].0, mkt_p);
        assert_eq!(xa[0].1.size(), 300);
        assert_eq!(reader.count_positions_for_exchange(exch_a), 1);

        // Account Y now visible on exchange B.
        assert_eq!(
            reader.iter_position_accounts_for_exchange(exch_b),
            vec![acct_y]
        );
        let yb = reader.get_account_positions(exch_b, acct_y);
        assert_eq!(yb.len(), 1);
        assert_eq!(yb[0].1.size(), 50);

        // Chunk 2: delete account X's last position. The layer
        // stores an empty `UserPositionState`, which
        // `iter_position_accounts_for_exchange` filters out.
        fold_chunk(&handle, 2, vec![PositionWrite {
            position_key: UserPositionKey {
                exchange: exch_a,
                account: acct_x,
            },
            market: mkt_p,
            value: None,
        }]);
        assert!(reader.iter_position_accounts_for_exchange(exch_a).is_empty());
        assert!(reader.get_account_positions(exch_a, acct_x).is_empty());
        assert_eq!(reader.count_positions_for_exchange(exch_a), 0);
    }

    /// `InMemoryNativeStateReader::snapshot` returns a coherent view
    /// across many queries — writes that land after the snapshot
    /// is taken must not leak in.
    #[test]
    fn snapshot_view_is_coherent_across_writes() {
        let handle = Arc::new(Mutex::new(UserPositions::new_empty("test")));
        let reader = InMemoryNativeStateReader::new(Arc::clone(&handle));

        let exch_a = exchange(1);
        let exch_b = exchange(2);
        let acct_x = account(10);
        let acct_y = account(11);
        let mkt_p = market(100);

        // Chunk 0: account X on A, account Y on B.
        fold_chunk(&handle, 0, vec![
            PositionWrite {
                position_key: UserPositionKey {
                    exchange: exch_a,
                    account: acct_x,
                },
                market: mkt_p,
                value: Some(position(7)),
            },
            PositionWrite {
                position_key: UserPositionKey {
                    exchange: exch_b,
                    account: acct_y,
                },
                market: mkt_p,
                value: Some(position(9)),
            },
        ]);

        // Take a snapshot, then mutate via another chunk. Snapshot
        // must keep showing the version-0 state for ALL queries.
        let snap = reader.snapshot();

        fold_chunk(&handle, 1, vec![
            // After-snapshot delete on X and append on B's Y.
            PositionWrite {
                position_key: UserPositionKey {
                    exchange: exch_a,
                    account: acct_x,
                },
                market: mkt_p,
                value: None,
            },
            PositionWrite {
                position_key: UserPositionKey {
                    exchange: exch_b,
                    account: acct_y,
                },
                market: market(101),
                value: Some(position(11)),
            },
        ]);

        // Snapshot still reflects v0.
        assert_eq!(snap.iter_position_accounts_for_exchange(exch_a), vec![
            acct_x
        ]);
        assert_eq!(snap.count_positions_for_exchange(exch_a), 1);
        let xa = snap.get_account_positions(exch_a, acct_x);
        assert_eq!(xa.len(), 1);
        assert_eq!(xa[0].1.size(), 7);
        assert_eq!(snap.count_positions_for_exchange(exch_b), 1);

        // Live reader now reflects v1.
        assert!(reader.iter_position_accounts_for_exchange(exch_a).is_empty());
        assert_eq!(reader.count_positions_for_exchange(exch_b), 2);
    }
}
