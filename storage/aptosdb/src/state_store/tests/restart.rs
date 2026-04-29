// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! End-to-end proptest for the node restart flow.
//!
//! Commits a proptest-generated block sequence to an `AptosDB`, drops the DB, reopens
//! it from disk, commits the remainder, and verifies that data and proofs remain
//! consistent across the restart boundary.

use crate::db::{
    test_helper::{arb_blocks_to_commit_with_params, verify_committed_transactions},
    AptosDB,
};
use aptos_config::config::{
    HotStateConfig, RocksdbConfigs, StorageDirPaths, DEFAULT_MAX_NUM_NODES_PER_LRU_CACHE_SHARD,
    NO_OP_STORAGE_PRUNER_CONFIG,
};
use aptos_storage_interface::DbReader;
use aptos_temppath::TempPath;
use aptos_types::{
    ledger_info::LedgerInfoWithSignatures,
    transaction::{TransactionToCommit, Version},
};
use proptest::prelude::*;
use std::path::Path;

fn open_db<P: AsRef<Path>>(
    path: P,
    max_items_per_shard: usize,
    buffered_state_target_items: usize,
) -> AptosDB {
    let hot_state_config = HotStateConfig {
        max_items_per_shard,
        refresh_interval_versions: 1_000,
        delete_on_restart: false,
        compute_root_hash: true,
        persist_hotness_in_write_set: true,
        persist_hotness_in_epilogue: false,
    };
    AptosDB::open(
        StorageDirPaths::from_path(path),
        false, /* readonly */
        NO_OP_STORAGE_PRUNER_CONFIG,
        RocksdbConfigs::default(),
        buffered_state_target_items,
        DEFAULT_MAX_NUM_NODES_PER_LRU_CACHE_SHARD,
        None, /* internal_indexer_db */
        hot_state_config,
    )
    .expect("Failed to open AptosDB")
}

fn commit_batches(
    db: &AptosDB,
    batches: &[(Vec<TransactionToCommit>, LedgerInfoWithSignatures, bool)],
    start_version: Version,
) -> Version {
    let mut version = start_version;
    for (txns, li, sync_commit) in batches {
        db.save_transactions_for_test(txns, version, Some(li), *sync_commit)
            .unwrap();
        verify_committed_transactions(db, txns, version, li, false /* is_latest */);
        version += txns.len() as Version;
    }
    version
}

fn test_restart_impl(
    input: Vec<(Vec<TransactionToCommit>, LedgerInfoWithSignatures, bool)>,
    split_at: usize,
    max_items_per_shard: usize,
    buffered_state_target_items: usize,
) {
    let (first_half, second_half) = input.split_at(split_at);

    let tmp_dir = TempPath::new();
    tmp_dir.create_as_dir().unwrap();

    // Phase 1: commit, then drop. Drop flushes `BufferedState` and the snapshot committer.
    let version = {
        let db = open_db(
            tmp_dir.path(),
            max_items_per_shard,
            buffered_state_target_items,
        );
        commit_batches(&db, first_half, 0)
    };

    // Phase 2: reopen and commit the rest.
    let db = open_db(
        tmp_dir.path(),
        max_items_per_shard,
        buffered_state_target_items,
    );
    assert_eq!(db.expect_synced_version(), version - 1);
    let final_version = commit_batches(&db, second_half, version);

    // Re-verify every batch (pre- and post-restart) against the current (post-restart)
    // ledger info — the per-batch verification inside `commit_batches` only checks against
    // each batch's own LI.
    let latest_ledger_info = &input.last().unwrap().1;
    let last_idx = input.len() - 1;
    let mut v: Version = 0;
    for (i, (txns, _, _)) in input.iter().enumerate() {
        verify_committed_transactions(
            &db,
            txns,
            v, /* first_version */
            latest_ledger_info,
            i == last_idx, /* is_latest */
        );
        v += txns.len() as Version;
    }

    let expected_total: u64 = input.iter().map(|(txns, _, _)| txns.len() as u64).sum();
    assert_eq!(final_version, expected_total);
    assert_eq!(db.expect_synced_version(), expected_total - 1);
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(20))]

    #[test]
    fn test_restart(
        (input, split_at, max_items_per_shard, buffered_state_target_items)
            in arb_blocks_to_commit_with_params(
                10, /* num_accounts */
                2,  /* max_user_txns_per_block */
                3,  /* min_blocks */
                8,  /* max_blocks */
            ).prop_flat_map(|blocks| {
                let len = blocks.len();
                (
                    Just(blocks),
                    proptest::collection::vec(any::<bool>(), len),
                    1usize..len,
                    1usize..10,
                    1usize..1000,
                )
            }).prop_map(|(blocks, sync_flags, split_at, max_items_per_shard, buffered_state_target_items)| {
                let input = blocks
                    .into_iter()
                    .zip(sync_flags)
                    .map(|((txns, li), sync)| (txns, li, sync))
                    .collect::<Vec<_>>();
                (input, split_at, max_items_per_shard, buffered_state_target_items)
            }),
    ) {
        test_restart_impl(
            input,
            split_at,
            max_items_per_shard,
            buffered_state_target_items,
        );
    }
}
