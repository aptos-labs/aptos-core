// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! End-to-end proptest for the node restart flow.
//!
//! Commits a proptest-generated block sequence to an `AptosDB`, drops the DB, reopens it
//! from disk, and commits the remainder. Every commit is `sync_commit = true` so the
//! snapshot committer runs inline — a panic there (e.g. mishandling of `StateSlot`s
//! whose `state_key` is `None` after being loaded from the hot state KV DB) surfaces
//! as a test failure. Post-restart reads of pre-restart data additionally check that
//! proofs still verify across the boundary.
//!
//! Two knobs make the setup relevant to the hot-state path:
//!   - `delete_on_restart = false`, so the hot KV DB survives reopen and
//!     `load_hot_state_or_empty` actually populates `StateSlot`s whose `state_key` is
//!     `None` (the persisted rows only carry the key hash).
//!   - Small `max_items_per_shard`, so even modest workloads drive LRU insertion and
//!     eviction — which is what patches those neighbor slots (still without a
//!     `state_key`) into `pending` and sends them through `make_delta` into the
//!     committer.

use crate::db::{
    test_helper::{arb_blocks_to_commit_with_params, verify_committed_transactions},
    AptosDB,
};
use aptos_config::config::{
    HotStateConfig, RocksdbConfigs, StorageDirPaths, BUFFERED_STATE_TARGET_ITEMS_FOR_TEST,
    DEFAULT_MAX_NUM_NODES_PER_LRU_CACHE_SHARD, NO_OP_STORAGE_PRUNER_CONFIG,
};
use aptos_storage_interface::DbReader;
use aptos_temppath::TempPath;
use aptos_types::{
    ledger_info::LedgerInfoWithSignatures,
    transaction::{TransactionToCommit, Version},
};
use proptest::prelude::*;
use std::path::Path;

/// Small per-shard capacity forces LRU churn (insertions and evictions) under modest
/// workloads, so post-restart blocks reliably touch DB-loaded neighbor slots.
const MAX_HOT_ITEMS_PER_SHARD: usize = 4;

fn open_db<P: AsRef<Path>>(path: P) -> AptosDB {
    let hot_state_config = HotStateConfig {
        max_items_per_shard: MAX_HOT_ITEMS_PER_SHARD,
        refresh_interval_versions: 1_000,
        // The whole point of this test: persist hotness across restart so reopen loads
        // `StateSlot`s (with `state_key = None`) via `load_hot_state_kvs`.
        delete_on_restart: false,
        compute_root_hash: true,
        persist_hotness_in_write_set: true,
    };
    AptosDB::open(
        StorageDirPaths::from_path(path),
        false, /* readonly */
        NO_OP_STORAGE_PRUNER_CONFIG,
        RocksdbConfigs::default(),
        BUFFERED_STATE_TARGET_ITEMS_FOR_TEST,
        DEFAULT_MAX_NUM_NODES_PER_LRU_CACHE_SHARD,
        None, /* internal_indexer_db */
        hot_state_config,
    )
    .expect("Failed to open AptosDB")
}

fn commit_batches(
    db: &AptosDB,
    batches: &[(Vec<TransactionToCommit>, LedgerInfoWithSignatures)],
    start_version: Version,
) -> Version {
    let mut version = start_version;
    for (txns, li) in batches.iter() {
        db.save_transactions_for_test(txns, version, Some(li), true /* sync_commit */)
            .unwrap();
        // `is_latest` is currently unused inside `verify_committed_transactions`; passing
        // `false` keeps the call site uniform whether we're at the end or not.
        verify_committed_transactions(db, txns, version, li, false);
        version += txns.len() as Version;
    }
    version
}

fn test_restart_impl(
    input: Vec<(Vec<TransactionToCommit>, LedgerInfoWithSignatures)>,
    split_at: usize,
) {
    assert!(
        split_at >= 1 && split_at < input.len(),
        "split_at must leave at least one batch on each side"
    );
    let (first_half, second_half) = input.split_at(split_at);

    let tmp_dir = TempPath::new();
    tmp_dir.create_as_dir().unwrap();

    // Phase 1: commit, then drop. Drop flushes `BufferedState` and the snapshot committer
    // thread, leaving the hot KV DB and state merkle DB on disk.
    let version = {
        let db = open_db(tmp_dir.path());
        commit_batches(&db, first_half, 0)
    };

    // Phase 2: reopen. `load_hot_state_or_empty` runs here, populating the committed
    // DashMaps with `StateSlot`s whose `state_key` is `None`. Continuing to commit drives
    // LRU touches on those slots.
    let db = open_db(tmp_dir.path());
    assert_eq!(db.expect_synced_version(), version - 1);
    let final_version = commit_batches(&db, second_half, version);

    // Re-verify a pre-restart batch against the *current* (post-restart) ledger info. This
    // confirms that the accumulator / state proofs remain consistent across the boundary.
    let (first_batch_txns, _) = &input[0];
    let latest_ledger_info = &input.last().unwrap().1;
    verify_committed_transactions(
        &db,
        first_batch_txns,
        0, /* first_version */
        latest_ledger_info,
        false, /* is_latest */
    );

    let expected_total: u64 = input.iter().map(|(txns, _)| txns.len() as u64).sum();
    assert_eq!(final_version, expected_total);
    assert_eq!(db.expect_synced_version(), expected_total - 1);
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(8))]

    #[test]
    fn test_restart(
        (input, split_at) in arb_blocks_to_commit_with_params(
            10, /* num_accounts — comfortably above 16 shards × MAX_HOT_ITEMS_PER_SHARD */
            2,  /* max_user_txns_per_block */
            3,  /* min_blocks — ≥ 2 so any split_at in `1..len` keeps both sides non-empty */
            8,  /* max_blocks */
        ).prop_flat_map(|blocks| {
            let len = blocks.len();
            (Just(blocks), 1usize..len)
        }),
    ) {
        test_restart_impl(input, split_at);
    }
}
