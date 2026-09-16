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

/// BCS-encoded `NativePosition` whose `size` carries `tag`, so tests can
/// tell writes apart.
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
) -> crate::native_state_store::PositionWrites {
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
        .1
        .as_ref()
        .expect("upsert carries the decoded value");
    assert_eq!(pos.size(), 7, "decoded NativePosition's tag survives apply");

    let deletes = commit_one(&db, 1, key.clone(), delete());
    assert_eq!(deletes.len(), 1);
    assert!(deletes[0].1.is_none(), "delete carries no value");
}

/// One `PositionWrite` per input, in arrival order, correctly decoded.
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
        (StateKey::position(exch, acct_a, market_x), upsert(100)),
        (StateKey::position(exch, acct_b, market_y), upsert(200)),
        (StateKey::position(exch, acct_a, market_y), delete()),
    ];

    let writes = committer
        .apply(0, inputs, &mut sharded_kv_batches, &mut in_chunk_prior)
        .expect("apply");

    assert_eq!(writes.len(), 3, "one PositionWrite per input");

    assert_eq!(writes[0].0.exchange, exch);
    assert_eq!(writes[0].0.account, acct_a);
    assert_eq!(writes[0].0.market, market_x);
    assert_eq!(writes[0].1.as_ref().unwrap().size(), 100);

    assert_eq!(writes[1].0.account, acct_b);
    assert_eq!(writes[1].0.market, market_y);
    assert_eq!(writes[1].1.as_ref().unwrap().size(), 200);

    assert_eq!(writes[2].0.account, acct_a);
    assert_eq!(writes[2].0.market, market_y);
    assert!(writes[2].1.is_none(), "delete carries no value");
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
        native_state_reader::{InMemoryNativeStateReader, NativeStateReader},
        native_state_store::{PositionBase, PositionKey, PositionOverlay, PositionWrites},
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

    /// The writer's post-commit publish of one chunk.
    fn fold_chunk(handle: &Arc<Mutex<PositionOverlay>>, version: Version, writes: PositionWrites) {
        let mut up = handle.lock();
        *up = up.extend(version, writes);
    }

    /// Inserts, latest-wins overwrites, deletes, and cross-exchange
    /// isolation across chunks, against the overlay the reader holds.
    #[test]
    fn reader_sees_writes_folded_through_positions() {
        let base = Arc::new(PositionBase::new_empty("test"));
        let handle = Arc::new(Mutex::new(PositionOverlay::new_at_base(base)));
        let reader = InMemoryNativeStateReader::new(Arc::clone(&handle));

        let exch_a = exchange(1);
        let exch_b = exchange(2);
        let acct_x = account(10);
        let acct_y = account(11);
        let mkt_p = market(100);
        let mkt_q = market(101);

        // Chunk 0: account X on exchange A opens two markets.
        fold_chunk(&handle, 0, vec![
            (
                PositionKey {
                    exchange: exch_a,
                    account: acct_x,
                    market: mkt_p,
                },
                Some(position(100)),
            ),
            (
                PositionKey {
                    exchange: exch_a,
                    account: acct_x,
                    market: mkt_q,
                },
                Some(position(200)),
            ),
        ]);

        let xa = reader.get_account_positions(exch_a, acct_x);
        let xa_sizes: Vec<u64> = xa.iter().map(|(_, p)| p.size()).collect();
        assert_eq!(xa.len(), 2);
        assert!(xa_sizes.contains(&100));
        assert!(xa_sizes.contains(&200));

        assert_eq!(reader.iter_position_accounts_for_exchange(exch_a), vec![
            acct_x
        ]);
        assert_eq!(reader.count_positions_for_exchange(exch_a), 2);

        // Exchange B is empty at this point.
        assert!(reader
            .iter_position_accounts_for_exchange(exch_b)
            .is_empty());
        assert_eq!(reader.count_positions_for_exchange(exch_b), 0);

        // Chunk 1: overwrite mkt_p on X, delete mkt_q on X, open one
        // position for account Y on a different exchange.
        fold_chunk(&handle, 1, vec![
            (
                PositionKey {
                    exchange: exch_a,
                    account: acct_x,
                    market: mkt_p,
                },
                Some(position(300)),
            ),
            (
                PositionKey {
                    exchange: exch_a,
                    account: acct_x,
                    market: mkt_q,
                },
                None,
            ),
            (
                PositionKey {
                    exchange: exch_b,
                    account: acct_y,
                    market: mkt_p,
                },
                Some(position(50)),
            ),
        ]);

        // Account X: latest-wins on mkt_p, mkt_q gone.
        let xa = reader.get_account_positions(exch_a, acct_x);
        assert_eq!(xa.len(), 1);
        assert_eq!(xa[0].0, mkt_p);
        assert_eq!(xa[0].1.size(), 300);
        assert_eq!(reader.count_positions_for_exchange(exch_a), 1);

        // Account Y now visible on exchange B.
        assert_eq!(reader.iter_position_accounts_for_exchange(exch_b), vec![
            acct_y
        ]);
        let yb = reader.get_account_positions(exch_b, acct_y);
        assert_eq!(yb.len(), 1);
        assert_eq!(yb[0].1.size(), 50);

        // Chunk 2: delete X's last position; the tombstone drops the account.
        fold_chunk(&handle, 2, vec![(
            PositionKey {
                exchange: exch_a,
                account: acct_x,
                market: mkt_p,
            },
            None,
        )]);
        assert!(reader
            .iter_position_accounts_for_exchange(exch_a)
            .is_empty());
        assert!(reader.get_account_positions(exch_a, acct_x).is_empty());
        assert_eq!(reader.count_positions_for_exchange(exch_a), 0);
    }

    /// `cargo test -p aptos-db --release -- --ignored --nocapture position_index_cost`
    #[test]
    #[ignore]
    fn position_index_cost_at_200k_positions() {
        use crate::native_state_store::{AccountKey, PositionKey};
        use aptos_types::account_address::AccountAddress;
        use std::time::Instant;

        fn addr_n(n: u64) -> AccountAddress {
            let mut a = [0u8; AccountAddress::LENGTH];
            a[AccountAddress::LENGTH - 8..].copy_from_slice(&n.to_be_bytes());
            AccountAddress::new(a)
        }

        // Same 200k positions over 8 exchanges: a per-exchange scan should
        // cost roughly an eighth, since it ranges rather than filters.
        for (n_accounts, markets, n_exchanges) in
            [(200_000u64, 1u64, 1u64), (20_000, 10, 1), (200_000, 1, 8)]
        {
            let exch = addr_n(1);
            let seed: Vec<_> = (0..n_accounts)
                .flat_map(|a| {
                    (0..markets).map(move |m| {
                        (
                            PositionKey {
                                exchange: addr_n(1 + a % n_exchanges),
                                account: addr_n(a),
                                market: addr_n(1 << 40 | m),
                            },
                            position(a + m),
                        )
                    })
                })
                .collect();
            let total = seed.len();
            let base = Arc::new(PositionBase::new_at_version(Some(0), "bench", seed));

            for writes in [0u64, 1_000] {
                let handle = Arc::new(Mutex::new(PositionOverlay::new_at_base(Arc::clone(&base))));
                if writes > 0 {
                    let chunk: PositionWrites = (0..writes)
                        .map(|i| {
                            (
                                PositionKey {
                                    exchange: addr_n(1 + i % n_exchanges),
                                    account: addr_n(i),
                                    market: addr_n(1 << 40),
                                },
                                Some(position(i + 1)),
                            )
                        })
                        .collect();
                    let mut up = handle.lock();
                    *up = up.extend(1, chunk);
                }
                let reader = InMemoryNativeStateReader::new(Arc::clone(&handle));
                println!(
                    "\n=== {n_accounts} accounts x {markets} markets = {total} positions over \
                     {n_exchanges} exchange(s), {writes} writes in overlay ===",
                );

                let t = Instant::now();
                let accounts = reader.iter_position_accounts_for_exchange(exch);
                println!(
                    "iter accounts          {:>10.1?}  ({} accounts)",
                    t.elapsed(),
                    accounts.len()
                );

                let t = Instant::now();
                let counted = reader.count_positions_for_exchange(exch);
                println!("count positions        {:>10.1?}  ({counted})", t.elapsed());
                assert_eq!(counted as u64, total as u64 / n_exchanges);

                let t = Instant::now();
                let mut found = 0usize;
                reader.with_view(|view| {
                    for i in 0..1000u64 {
                        let a = i * (n_accounts / 1000);
                        found += view
                            .get_account_positions(addr_n(1 + a % n_exchanges), addr_n(a))
                            .len();
                    }
                });
                println!(
                    "1000 account reads     {:>10.1?}  ({found} positions)",
                    t.elapsed()
                );

                if writes > 0 {
                    let fresh = Arc::new(PositionBase::new_at_version(
                        Some(0),
                        "bench",
                        base.read(|v| {
                            (0..crate::native_state_store::NUM_POSITION_SHARDS)
                                .flat_map(|s| {
                                    v.shard(s).iter().flat_map(|(account, markets)| {
                                        markets.iter().map(move |(market, p)| {
                                            (
                                                PositionKey {
                                                    exchange: account.exchange,
                                                    account: account.account,
                                                    market: *market,
                                                },
                                                p.clone(),
                                            )
                                        })
                                    })
                                })
                                .collect::<Vec<_>>()
                        }),
                    ));
                    let up = PositionOverlay::new_at_base(Arc::clone(&fresh))
                        .extend(1, handle.lock().writes_since_base());
                    let t = Instant::now();
                    assert!(fresh.fold(&up));
                    println!("fold                   {:>10.1?}", t.elapsed());
                }

                let _ = AccountKey {
                    exchange: exch,
                    account: addr_n(0),
                };
            }
        }
    }

    /// The base read lock is taken while the tip mutex is still held, so a
    /// fold cannot land mid-view and every query sees the same moment.
    #[test]
    fn a_fold_cannot_land_while_a_view_is_open() {
        use std::{
            sync::atomic::{AtomicBool, Ordering},
            time::Duration,
        };

        let base = Arc::new(PositionBase::new_empty("test"));
        let handle = Arc::new(Mutex::new(PositionOverlay::new_at_base(Arc::clone(&base))));
        let reader = InMemoryNativeStateReader::new(Arc::clone(&handle));

        let exch_a = exchange(1);
        let mkt_p = market(100);
        fold_chunk(&handle, 0, vec![(
            PositionKey {
                exchange: exch_a,
                account: account(10),
                market: mkt_p,
            },
            Some(position(1)),
        )]);

        let folded = Arc::new(AtomicBool::new(false));

        let joiner = reader.with_view(|view| {
            assert_eq!(view.count_positions_for_exchange(exch_a), 1);

            let base_for_thread = Arc::clone(&base);
            let handle_for_thread = Arc::clone(&handle);
            let folded_for_thread = Arc::clone(&folded);
            let joiner = std::thread::spawn(move || {
                let tip = handle_for_thread.lock();
                base_for_thread.fold(&tip);
                folded_for_thread.store(true, Ordering::SeqCst);
            });

            // Deterministic: the fold needs the write lock this view holds.
            std::thread::sleep(Duration::from_millis(50));
            assert!(
                !folded.load(Ordering::SeqCst),
                "a fold landed while a view was open"
            );

            // Repeat queries still agree — same moment throughout.
            assert_eq!(view.count_positions_for_exchange(exch_a), 1);
            assert_eq!(
                view.get_account_positions(exch_a, account(10))[0].1.size(),
                1
            );
            joiner
        });

        joiner.join().unwrap();
        assert!(
            folded.load(Ordering::SeqCst),
            "fold should proceed once the view closes"
        );
        assert_eq!(base.version(), Some(0));
        assert_eq!(reader.count_positions_for_exchange(exch_a), 1);
    }

    /// After the base advances the accounts live only in the base map, and
    /// a freshly seeded overlay carries nothing.
    #[test]
    fn reader_sees_accounts_after_the_overlay_folds_into_the_base() {
        let base = Arc::new(PositionBase::new_empty("test"));
        let handle = Arc::new(Mutex::new(PositionOverlay::new_at_base(Arc::clone(&base))));
        let reader = InMemoryNativeStateReader::new(Arc::clone(&handle));

        let exch_a = exchange(1);
        let acct_x = account(10);
        let acct_y = account(11);
        let mkt_p = market(100);

        fold_chunk(&handle, 0, vec![
            (
                PositionKey {
                    exchange: exch_a,
                    account: acct_x,
                    market: mkt_p,
                },
                Some(position(7)),
            ),
            (
                PositionKey {
                    exchange: exch_a,
                    account: acct_y,
                    market: mkt_p,
                },
                Some(position(9)),
            ),
        ]);
        assert_eq!(reader.count_positions_for_exchange(exch_a), 2);

        // Advance the base and re-seat the published tip, which is what
        // `advance_position_base` does when the executor calls it at the
        // start of a block's execution.
        {
            let mut tip = handle.lock();
            base.fold(&tip);
            *tip = tip.rebased_on_current_base();
        }
        assert_eq!(base.version(), Some(0));
        assert!(handle.lock().writes_since_base().is_empty());

        // Same answers, now served entirely from the base.
        assert_eq!(reader.count_positions_for_exchange(exch_a), 2);
        let mut accounts = reader.iter_position_accounts_for_exchange(exch_a);
        accounts.sort();
        let mut expected = vec![acct_x, acct_y];
        expected.sort();
        assert_eq!(accounts, expected);
        assert_eq!(reader.get_account_positions(exch_a, acct_x)[0].1.size(), 7);

        // A write after the fold shadows the base rather than duplicating it.
        fold_chunk(&handle, 1, vec![(
            PositionKey {
                exchange: exch_a,
                account: acct_x,
                market: mkt_p,
            },
            Some(position(70)),
        )]);
        assert_eq!(reader.count_positions_for_exchange(exch_a), 2);
        assert_eq!(reader.get_account_positions(exch_a, acct_x)[0].1.size(), 70);

        // And a delete after the fold removes the base entry from the scan.
        fold_chunk(&handle, 2, vec![(
            PositionKey {
                exchange: exch_a,
                account: acct_y,
                market: mkt_p,
            },
            None,
        )]);
        assert_eq!(reader.iter_position_accounts_for_exchange(exch_a), vec![
            acct_x
        ]);
        assert_eq!(reader.count_positions_for_exchange(exch_a), 1);
    }
}
