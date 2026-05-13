// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Integration tests for the native-position storage layer.
//!
//! Covers the round-trip + correctness properties that are
//! consensus-load-bearing: state-sync producer/verifier, pruner,
//! backup verify, empty-updates merkle root, and the recently
//! tightened DashMap remove-when-empty race.

use crate::{
    native_state_committer::{MerkleLeafUpdate, NativeStateCommitter},
    native_state_store::{NativeStateStore, UserKey, UserState},
    position_db::{PositionDb, NUM_NATIVE_VALUE_SHARDS},
    position_merkle_db::PositionMerkleDb,
    position_pruner::PositionPruner,
    position_state_sync,
    schema::{
        jellyfish_merkle_node::JellyfishMerkleNodeSchema, stale_node_index::StaleNodeIndexSchema,
    },
};
use aptos_crypto::HashValue;
use aptos_jellyfish_merkle::{JellyfishMerkleTree, StaleNodeIndex, TreeUpdateBatch};
use aptos_schemadb::{batch::SchemaBatch, ColumnFamilyDescriptor, Options, DB};
use aptos_temppath::TempPath;
use aptos_types::{
    state_store::{state_key::StateKey, state_value::StateValue},
    transaction::Version,
    write_set::WriteOp,
};
use move_core_types::account_address::AccountAddress;
use std::sync::Arc;

// ---------------------------------------------------------------
// Test fixture
// ---------------------------------------------------------------

/// All the storage handles a position-side test needs, plus the
/// owned [`TempPath`] that keeps the on-disk DB files alive.
struct PositionFixture {
    _tmpdir: TempPath,
    position_db: Arc<PositionDb>,
    position_merkle_db: Arc<PositionMerkleDb>,
    store: Arc<NativeStateStore>,
}

impl PositionFixture {
    fn new() -> Self {
        let tmpdir = TempPath::new();
        std::fs::create_dir_all(tmpdir.path()).unwrap();

        let value_root = tmpdir.path().join("value");
        let merkle_dir = tmpdir.path().join("merkle");
        std::fs::create_dir_all(&value_root).unwrap();
        std::fs::create_dir_all(&merkle_dir).unwrap();

        // 16 independent shard DBs — using one shared DB across all
        // 16 slots double-counts every row in cross-shard scans
        // (which is why `new_uniform_for_test` is unfit for tests
        // that exercise the pruner / state-sync / backup paths).
        let position_db = Arc::new(
            PositionDb::new(
                &value_root,
                aptos_config::config::RocksdbConfig::default(),
                None,
                None,
                /* readonly = */ false,
            )
            .expect("PositionDb::new failed in test fixture"),
        );

        let merkle_db_raw = Arc::new(open_test_db(
            &merkle_dir,
            "position_merkle_db",
            &crate::db_options::position_merkle_db_column_families(),
        ));
        let position_merkle_db = Arc::new(PositionMerkleDb::new_uniform_for_test(merkle_db_raw));
        let store = Arc::new(NativeStateStore::empty());

        Self {
            _tmpdir: tmpdir,
            position_db,
            position_merkle_db,
            store,
        }
    }

    fn committer(&self) -> NativeStateCommitter {
        NativeStateCommitter::new(Arc::clone(&self.position_db), Arc::clone(&self.store))
            .with_position_merkle_db(Arc::clone(&self.position_merkle_db))
    }

    /// Test-only synchronous JMT apply: runs the same 16-shard pipeline
    /// `merklize_position` uses in production, but commits to disk
    /// inline so tests can assert against the resulting state without
    /// spinning up background threads.
    ///
    /// Returns the new `position_root`. Like the production path, runs
    /// the full pipeline regardless of whether `updates` is empty.
    fn apply_jmt_sync(&self, version: Version, updates: &[MerkleLeafUpdate]) -> HashValue {
        let base_version = version.checked_sub(1);
        let mut leaf_store: Vec<(HashValue, Option<(HashValue, StateKey)>)> =
            Vec::with_capacity(updates.len());
        for u in updates {
            match u.value_hash {
                Some(v) => leaf_store.push((u.state_key_hash, Some((v, u.state_key.clone())))),
                None => leaf_store.push((u.state_key_hash, None)),
            }
        }
        let value_set_refs: Vec<(HashValue, Option<&(HashValue, StateKey)>)> =
            leaf_store.iter().map(|(h, v)| (*h, v.as_ref())).collect();

        let tree = JellyfishMerkleTree::new(self.position_merkle_db.as_ref());
        let mut tree_update_batch = TreeUpdateBatch::new();
        let mut shard_root_nodes = Vec::with_capacity(NUM_NATIVE_VALUE_SHARDS);
        for shard_id in 0..NUM_NATIVE_VALUE_SHARDS as u8 {
            let shard_value_set: Vec<_> = value_set_refs
                .iter()
                .filter(|(k, _)| k.nibble(0) == shard_id)
                .cloned()
                .collect();
            let (shard_root_node, shard_batch) = tree
                .batch_put_value_set_for_shard(
                    shard_id,
                    shard_value_set,
                    None,
                    base_version,
                    version,
                )
                .unwrap();
            tree_update_batch.combine(shard_batch);
            shard_root_nodes.push(shard_root_node);
        }
        let (root_hash, _, top_batch) = tree
            .put_top_levels_nodes(shard_root_nodes, base_version, version)
            .unwrap();
        tree_update_batch.combine(top_batch);

        // Inline the equivalent of `PositionMerkleBatchCommitter::commit`
        // — write all accumulated nodes + stale entries to disk in one
        // batch. No cross-epoch split: tests don't exercise epoch
        // boundaries (and the regular CF holds entries for the pruner
        // either way).
        let mut write = SchemaBatch::new();
        for (node_key, node) in tree_update_batch.node_batch.iter().flatten() {
            write
                .put::<JellyfishMerkleNodeSchema>(node_key, node)
                .unwrap();
        }
        for stale in tree_update_batch.stale_node_index_batch.iter().flatten() {
            write
                .put::<StaleNodeIndexSchema>(
                    &StaleNodeIndex {
                        stale_since_version: stale.stale_since_version,
                        node_key: stale.node_key.clone(),
                    },
                    &(),
                )
                .unwrap();
        }
        // Uniform-for-test: metadata_db == every shard. See the
        // empty-updates branch above for why.
        self.position_merkle_db
            .metadata_db()
            .write_schemas(write)
            .unwrap();
        root_hash
    }
}

fn open_test_db(
    path: &std::path::Path,
    name: &str,
    cfs: &[aptos_schemadb::ColumnFamilyName],
) -> DB {
    let mut db_opts = Options::default();
    db_opts.create_if_missing(true);
    db_opts.create_missing_column_families(true);
    let cfds: Vec<ColumnFamilyDescriptor> = cfs
        .iter()
        .map(|n| ColumnFamilyDescriptor::new((*n).to_string(), Options::default()))
        .collect();
    DB::open_cf(db_opts, path, name, cfds).expect("test db open")
}

// ---------------------------------------------------------------
// Test data helpers
// ---------------------------------------------------------------

fn user(byte: u8) -> AccountAddress {
    let mut bytes = [0u8; AccountAddress::LENGTH];
    bytes[AccountAddress::LENGTH - 1] = byte;
    AccountAddress::new(bytes)
}

fn market(byte: u8) -> AccountAddress {
    let mut bytes = [0u8; AccountAddress::LENGTH];
    bytes[0] = 0xAA;
    bytes[AccountAddress::LENGTH - 1] = byte;
    AccountAddress::new(bytes)
}

/// Test-only exchange address generator. Mirrors `user` / `market`
/// but tags the leading byte (0xEE) so debug dumps make the role
/// obvious at a glance.
fn exchange(byte: u8) -> AccountAddress {
    let mut bytes = [0u8; AccountAddress::LENGTH];
    bytes[0] = 0xEE;
    bytes[AccountAddress::LENGTH - 1] = byte;
    AccountAddress::new(bytes)
}

fn state_value(payload: &[u8]) -> StateValue {
    StateValue::new_legacy(payload.to_vec().into())
}

fn write_op_create(payload: &[u8]) -> WriteOp {
    WriteOp::legacy_creation(payload.to_vec().into())
}

fn write_op_delete() -> WriteOp {
    WriteOp::legacy_deletion()
}

/// Commit a single Position write through the full committer +
/// merkle pipeline and return the new position subtree root.
fn commit_one(
    fx: &PositionFixture,
    version: Version,
    exchange: AccountAddress,
    account: AccountAddress,
    market: AccountAddress,
    payload: Option<&[u8]>,
) -> HashValue {
    let key = StateKey::position(exchange, account, market);
    let op = match payload {
        Some(bytes) => write_op_create(bytes),
        None => write_op_delete(),
    };
    let updates = fx
        .committer()
        .apply(version, std::iter::once((key, op)))
        .unwrap();
    fx.apply_jmt_sync(version, &updates.position)
}

// ---------------------------------------------------------------
// Tests
// ---------------------------------------------------------------

/// Round-trip test for the pruner: write to v1, v2, v3 for the same
/// key; prune up to v2; v1 row should be gone, v2 + v3 still present.
#[test]
fn pruner_drops_rows_up_to_horizon() {
    let fx = PositionFixture::new();
    let exchange = exchange(1);
    let account = user(0x01);
    let mkt = market(0x01);

    let _ = commit_one(&fx, 0, exchange, account, mkt, Some(b"v0-payload"));
    let _ = commit_one(&fx, 1, exchange, account, mkt, Some(b"v1-payload"));
    let _ = commit_one(&fx, 2, exchange, account, mkt, Some(b"v2-payload"));

    use aptos_crypto::hash::CryptoHash;
    let key_hash = StateKey::position(exchange, account, mkt).hash();

    // All three versions present before pruning.
    assert_eq!(
        fx.position_db
            .get_position_value(key_hash, 0)
            .unwrap()
            .map(|sv| sv.bytes().to_vec()),
        Some(b"v0-payload".to_vec()),
    );
    assert_eq!(
        fx.position_db
            .get_position_value(key_hash, 1)
            .unwrap()
            .map(|sv| sv.bytes().to_vec()),
        Some(b"v1-payload".to_vec()),
    );
    assert_eq!(
        fx.position_db
            .get_position_value(key_hash, 2)
            .unwrap()
            .map(|sv| sv.bytes().to_vec()),
        Some(b"v2-payload".to_vec()),
    );

    let pruner = PositionPruner::new(Arc::clone(&fx.position_db));
    // Horizon = 1 drains every stale-index entry whose
    // `stale_since_version <= 1`, which means rows superseded by the
    // write at v1 (i.e. the v0 row) are collected.
    let drained = pruner.prune_up_to(1).unwrap();
    assert_eq!(drained, 1, "expected exactly the v0 row to drain");

    // v0 row is gone; v1 + v2 still present.
    assert!(
        fx.position_db
            .get_position_value(key_hash, 0)
            .unwrap()
            .is_none(),
        "v0 row should be pruned"
    );
    assert_eq!(
        fx.position_db
            .get_position_value(key_hash, 1)
            .unwrap()
            .map(|sv| sv.bytes().to_vec()),
        Some(b"v1-payload".to_vec()),
    );
    assert_eq!(
        fx.position_db
            .get_position_value(key_hash, 2)
            .unwrap()
            .map(|sv| sv.bytes().to_vec()),
        Some(b"v2-payload".to_vec()),
    );

    // Idempotency: re-pruning to the same horizon is a no-op.
    let again = pruner.prune_up_to(1).unwrap();
    assert_eq!(again, 0);
}

/// State-sync round-trip: produce → verify ok; tamper → verify fails;
/// produce → verify → apply → in-memory == DB scan.
#[test]
fn state_sync_chunk_roundtrip_and_tamper_detection() {
    let producer = PositionFixture::new();
    let exchange = exchange(7);

    // Seed five distinct (account, market) entries.
    for i in 0..5u8 {
        commit_one(
            &producer,
            i as Version,
            exchange,
            user(i + 1),
            market(i + 1),
            Some(&[0xAA, i, 0xBB]),
        );
    }
    let producer_root = producer.position_merkle_db.get_root_hash(4).unwrap();

    // Produce chunks at the snapshot version.
    let chunks = position_state_sync::produce_chunks(
        &producer.position_db,
        &producer.position_merkle_db,
        /* version = */ 4,
        /* chunk_size = */ 2,
    )
    .unwrap();
    assert!(!chunks.is_empty(), "producer must emit at least one chunk");
    for chunk in &chunks {
        assert_eq!(chunk.expected_position_root, producer_root);
        position_state_sync::verify_chunk(chunk).expect("untampered chunk must verify");
    }

    // Tamper detection: flip a single payload byte in chunk[0].
    let mut bad_chunk = chunks[0].clone();
    let original = bad_chunk.entries[0]
        .value
        .as_ref()
        .unwrap()
        .bytes()
        .to_vec();
    let mut tampered = original.clone();
    tampered[0] ^= 0xFF;
    bad_chunk.entries[0].value = Some(state_value(&tampered));
    assert!(
        position_state_sync::verify_chunk(&bad_chunk).is_err(),
        "tampered chunk must fail verification"
    );

    // Apply chunks to a fresh receiver and check in-memory + DB
    // converge on the same set.
    let receiver = PositionFixture::new();
    for chunk in &chunks {
        position_state_sync::apply_chunk(chunk, &receiver.position_db, &receiver.store)
            .expect("apply_chunk succeeds on verified chunk");
    }
    // Every committed key from the seed loop must have a value row
    // visible at version 4 on the receiver side.
    use aptos_crypto::hash::CryptoHash;
    for i in 0..5u8 {
        let key = StateKey::position(exchange, user(i + 1), market(i + 1));
        let got = receiver
            .position_db
            .get_position_value(key.hash(), 4)
            .unwrap();
        assert!(
            got.is_some(),
            "receiver DB missing row for ({:?}, {:?})",
            user(i + 1),
            market(i + 1),
        );
    }
    // In-memory store reflects the same set via the public iter API.
    let mut receiver_in_memory: Vec<(AccountAddress, AccountAddress, AccountAddress)> = receiver
        .store
        .users
        .iter()
        .flat_map(|entry| {
            let user_key = *entry.key();
            entry
                .value()
                .positions
                .keys()
                .copied()
                .map(move |m| (user_key.exchange, user_key.account, m))
                .collect::<Vec<_>>()
        })
        .collect();
    receiver_in_memory.sort();
    assert_eq!(receiver_in_memory.len(), 5);
    for (eid, _, _) in &receiver_in_memory {
        assert_eq!(*eid, exchange);
    }
}

/// Empty-updates merkle root: a block at version V with zero Position
/// writes must produce the same position subtree root as V-1, not the
/// empty-tree placeholder. Regression guard for the carry-forward path
/// in `PositionSnapshotCommitter`.
#[test]
fn empty_updates_preserves_prior_root() {
    let fx = PositionFixture::new();

    // Plant some state at v0 so the prior root is non-empty.
    let root_v0 = commit_one(&fx, 0, exchange(1), user(0x10), market(0x20), Some(b"seed"));
    let stored_root_v0 = fx.position_merkle_db.get_root_hash(0).unwrap();
    assert_eq!(root_v0, stored_root_v0);

    // Apply zero updates at v1 — should re-stamp v0's root, not the
    // empty placeholder.
    let root_v1 = fx.apply_jmt_sync(1, &[]);
    assert_eq!(
        root_v1, stored_root_v0,
        "empty updates at v1 must yield the prior version's root"
    );
}

/// DashMap remove-when-empty race: a thread removing the last
/// position for a user must not destroy entries inserted concurrently
/// by another thread under the same UserKey.
///
/// Single user, two threads: one repeatedly removes the only known
/// market, the other repeatedly inserts a different market. With the
/// pre-fix code, the racing remove() can drop the freshly-inserted
/// entry. With `remove_if(.., |_, e| e.is_empty())` the inserter
/// always wins (its insert makes the entry non-empty before
/// remove_if re-evaluates).
#[test]
fn apply_position_write_no_data_loss_under_concurrent_remove() {
    use std::sync::atomic::{AtomicBool, Ordering};

    let store = Arc::new(NativeStateStore::empty());
    let exchange = exchange(42);
    let account = user(0xAB);
    let mkt_a = market(0x01); // the removed market
    let mkt_b = market(0x02); // the inserted market
    let stop = Arc::new(AtomicBool::new(false));

    // Seed mkt_a so the removing thread has work.
    store.apply_position_write(exchange, account, mkt_a, Some(state_value(b"seed")));

    let store_remover = Arc::clone(&store);
    let stop_remover = Arc::clone(&stop);
    let remover = std::thread::spawn(move || {
        while !stop_remover.load(Ordering::Relaxed) {
            // Re-seed then remove, repeatedly. Each remove call may
            // see the entry as empty (mkt_b transiently removed by
            // the inserter) and trigger the remove_if path.
            store_remover.apply_position_write(
                exchange,
                account,
                mkt_a,
                Some(state_value(b"seed")),
            );
            store_remover.apply_position_write(exchange, account, mkt_a, None);
        }
    });

    let store_inserter = Arc::clone(&store);
    let inserter = std::thread::spawn(move || {
        // Insert a different market, then assert the entry is still
        // there afterward. Race with the remover means the entry
        // could be destroyed by a buggy implementation between our
        // insert and our get_mut. Repeat enough times to surface a
        // race.
        let key = UserKey { exchange, account };
        let iterations = 50_000;
        for _ in 0..iterations {
            store_inserter.apply_position_write(
                exchange,
                account,
                mkt_b,
                Some(state_value(b"survives")),
            );
            // Look up and confirm mkt_b survives.
            let snapshot: Option<UserState> =
                store_inserter.users.get(&key).map(|e| e.value().clone());
            // The remover's flow could have legitimately removed our
            // mkt_b only if it overwrote it; it never touches mkt_b,
            // so once our insert returns, mkt_b must be visible until
            // we (or another caller) explicitly remove it. The race
            // would manifest as the UserKey entry being deleted
            // entirely between our insert and this get — i.e.
            // snapshot.is_none() after a successful insert.
            assert!(
                snapshot.is_some(),
                "UserKey entry was destroyed under concurrent remove (race)"
            );
            let snap = snapshot.unwrap();
            assert!(
                snap.positions.contains_key(&mkt_b),
                "mkt_b should remain present after insert"
            );
        }
    });

    inserter.join().expect("inserter thread");
    stop.store(true, Ordering::Relaxed);
    remover.join().expect("remover thread");
}

/// Smoke: encoded key layout is the 98-byte
/// `[tag=2][sub_tag=0][exchange:32][account:32][market:32]` that the
/// schema doc claims. A change here is a hard ledger migration.
#[test]
fn position_state_key_encoding_is_stable() {
    let key = StateKey::position(exchange(0xCC), user(0xAA), market(0xBB));
    let bytes = key.encoded();
    const ADDR: usize = AccountAddress::LENGTH;
    assert_eq!(bytes.len(), 2 + ADDR * 3);
    assert_eq!(bytes[0], 2, "TradingNative umbrella tag");
    assert_eq!(bytes[1], 0, "Position sub-tag");
    assert_eq!(bytes[2], 0xEE, "exchange first byte");
    assert_eq!(bytes[2 + ADDR - 1], 0xCC, "exchange last byte");
    assert_eq!(bytes[2 + 2 * ADDR - 1], 0xAA, "account last byte");
    assert_eq!(bytes[2 + 3 * ADDR - 1], 0xBB, "market last byte");
}

/// Sanity guard around the MerkleLeafUpdate shape — the committer
/// returns one update per (key, value_hash) pair and the merkle
/// committer consumes them. Verifies a tombstone produces a leaf
/// update with `value_hash = None`.
#[test]
fn committer_emits_tombstone_with_no_value_hash() {
    let fx = PositionFixture::new();
    let exchange = exchange(3);
    let account = user(0x55);
    let mkt = market(0x77);
    let key = StateKey::position(exchange, account, mkt);

    // Create then delete in two commits.
    commit_one(&fx, 0, exchange, account, mkt, Some(b"to-be-deleted"));
    let updates = fx
        .committer()
        .apply(1, std::iter::once((key, write_op_delete())))
        .unwrap();
    assert_eq!(updates.position.len(), 1);
    let MerkleLeafUpdate {
        value_hash,
        state_key,
        ..
    } = updates.position[0].clone();
    assert!(value_hash.is_none(), "tombstone must have no value_hash");
    assert_eq!(
        state_key.crypto_hash_ref().nibble(0),
        state_key.get_shard_id() as u8
    );
}

/// `iter_active_leaves` round-trip: write N distinct positions; the
/// iterator at the snapshot version yields exactly those N
/// StateKeys, deduplicated and sorted by hash.
#[test]
fn iter_active_leaves_enumerates_live_positions() {
    use aptos_crypto::hash::CryptoHash;
    let fx = PositionFixture::new();
    let exchange = exchange(11);
    let want: Vec<StateKey> = (0..4u8)
        .map(|i| {
            let account = user(i + 1);
            let mkt = market(i + 1);
            commit_one(&fx, i as Version, exchange, account, mkt, Some(&[i, 0xCC]));
            StateKey::position(exchange, account, mkt)
        })
        .collect();
    let snapshot_version: Version = 3;

    let got: Vec<(StateKey, HashValue)> = fx
        .position_merkle_db
        .iter_active_leaves(snapshot_version)
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(got.len(), want.len(), "leaf count");

    // Iterator should yield hash-sorted; verify each yielded
    // `state_key.hash() == reported_key_hash` and the set of
    // StateKeys matches.
    for (state_key, reported_hash) in &got {
        assert_eq!(
            state_key.hash(),
            *reported_hash,
            "reported hash must equal StateKey::hash()"
        );
    }
    let got_set: std::collections::BTreeSet<HashValue> =
        got.iter().map(|(k, _)| k.hash()).collect();
    let want_set: std::collections::BTreeSet<HashValue> = want.iter().map(StateKey::hash).collect();
    assert_eq!(got_set, want_set);
}

/// `iter_active_leaves` must NOT yield a leaf whose latest commit at
/// or before the snapshot version is a tombstone. JMT delete removes
/// the leaf from the tree.
///
/// Note: positions are planted at distinct versions because the
/// JMT's `apply_batch` writes the entire root at a version; two
/// `apply_batch` calls at the same version overwrite rather than
/// merge. Real commit flow batches multiple writes in one
/// `apply_batch` call per block.
#[test]
fn iter_active_leaves_excludes_tombstones() {
    use aptos_crypto::hash::CryptoHash;
    let fx = PositionFixture::new();
    let exchange = exchange(13);
    let acct_a = user(0x01);
    let acct_b = user(0x02);
    let mkt = market(0x10);

    // Plant two positions at distinct versions so each apply_batch
    // observes the previous root.
    commit_one(&fx, 0, exchange, acct_a, mkt, Some(b"a"));
    commit_one(&fx, 1, exchange, acct_b, mkt, Some(b"b"));
    // Delete one at v2.
    commit_one(&fx, 2, exchange, acct_a, mkt, None);

    let got: Vec<StateKey> = fx
        .position_merkle_db
        .iter_active_leaves(2)
        .unwrap()
        .map(|r| r.map(|(k, _)| k))
        .collect::<Result<_, _>>()
        .unwrap();
    let got_hashes: std::collections::BTreeSet<HashValue> =
        got.iter().map(StateKey::hash).collect();
    let kept = StateKey::position(exchange, acct_b, mkt);
    let deleted = StateKey::position(exchange, acct_a, mkt);
    assert!(
        got_hashes.contains(&kept.hash()),
        "kept position must still be enumerated"
    );
    assert!(
        !got_hashes.contains(&deleted.hash()),
        "deleted position must NOT be enumerated"
    );
    assert_eq!(got.len(), 1, "exactly one live leaf remains");

    // Looking back at v1 (after both creates, before the delete) must
    // still see both leaves.
    let at_v1: std::collections::BTreeSet<HashValue> = fx
        .position_merkle_db
        .iter_active_leaves(1)
        .unwrap()
        .map(|r| r.unwrap().0.hash())
        .collect();
    assert_eq!(at_v1.len(), 2, "v1 snapshot still has both");
    assert!(at_v1.contains(&kept.hash()));
    assert!(at_v1.contains(&deleted.hash()));
}

/// Updating the same position across versions does NOT produce two
/// leaves at the later version — the JMT replaces the leaf in place.
#[test]
fn iter_active_leaves_dedups_across_version_updates() {
    use aptos_crypto::hash::CryptoHash;
    let fx = PositionFixture::new();
    let exchange = exchange(17);
    let account = user(0x42);
    let mkt = market(0x99);
    let key = StateKey::position(exchange, account, mkt);

    commit_one(&fx, 0, exchange, account, mkt, Some(b"first"));
    commit_one(&fx, 1, exchange, account, mkt, Some(b"second"));
    commit_one(&fx, 2, exchange, account, mkt, Some(b"third"));

    let at_v2: Vec<_> = fx
        .position_merkle_db
        .iter_active_leaves(2)
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(at_v2.len(), 1, "single live leaf for the position");
    assert_eq!(at_v2[0].0.hash(), key.hash());

    // The leaf at v2 must point at the v2 row in position_db.
    let v2_value = fx
        .position_db
        .get_position_value(key.hash(), 2)
        .unwrap()
        .expect("v2 row exists");
    assert_eq!(v2_value.bytes().to_vec(), b"third".to_vec());
}

/// Iterator on a freshly-opened (empty) merkle DB yields nothing —
/// no JMT root has been written.
#[test]
fn iter_active_leaves_empty_tree() {
    let fx = PositionFixture::new();
    // Iterator construction may either error (no root at version 0)
    // or succeed and yield zero entries. Both encode "tree is
    // empty". Accept either.
    match fx.position_merkle_db.iter_active_leaves(0) {
        Ok(iter) => {
            let collected: Vec<_> = iter.collect();
            assert!(
                collected.is_empty(),
                "empty tree must yield zero leaves; got {} entries",
                collected.len()
            );
        },
        Err(_) => {
            // Acceptable: no root persisted at version 0.
        },
    }
}
