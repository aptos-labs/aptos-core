use aptos_db::{
    db::{
        gather_state_updates_until_last_checkpoint, test_helper,
        test_helper::{arb_blocks_to_commit, arb_blocks_to_commit_with_block_nums},
    },
    transaction_store,
    transaction_store::TransactionStore,
    AptosDB,
};
use aptos_storage_interface::{cached_state_view::ShardedStateCache, DbWriter};
use aptos_temppath::TempPath;
use aptos_types::transaction::{TransactionToCommit, Version};
use proptest::{prelude::*, strategy::ValueTree, test_runner::TestRunner};
use std::{collections::HashMap, time::Instant};

fn main() {
    // Initialize temporary directory and database
    let tmp_dir = TempPath::new();
    let db = AptosDB::new_for_test(&tmp_dir);

    // 打印 db 的路径
    println!("数据库路径: {:?}", tmp_dir.path());

    // Generate test data
    let (input, _) = arb_blocks_to_commit_with_block_nums(10000, 20000)
        .new_tree(&mut TestRunner::default())
        .unwrap()
        .current();

    // Initialize in-memory state
    let mut in_memory_state = db
        .state_store
        .buffered_state()
        .lock()
        .current_state()
        .clone();
    let _ancester = in_memory_state.current.clone();
    let mut cur_ver: Version = 0;

    // Start timing
    let start = Instant::now();

    // Save transactions
    for (txns_to_commit, ledger_info_with_sigs) in input.iter() {
        test_helper::update_in_memory_state(&mut in_memory_state, txns_to_commit.as_slice());
        let base_checkpoint = in_memory_state.clone();
        let base_state_version = cur_ver.checked_sub(1);

        db.save_transactions(
            txns_to_commit,
            cur_ver,
            base_state_version,
            Some(ledger_info_with_sigs),
            false, // sync commit
            in_memory_state.clone(),
            gather_state_updates_until_last_checkpoint(cur_ver, &in_memory_state, txns_to_commit),
            //Some(&ShardedStateCache::default()),
            None,
        )
        .unwrap();

        cur_ver += txns_to_commit.len() as u64;
    }

    // End timing
    let duration = start.elapsed();
    let num_txns = cur_ver;
    let tps = num_txns as f64 / duration.as_secs_f64();

    println!(
        "Processed {} transactions in {:?} seconds",
        num_txns, duration
    );
    println!("Transactions per second: {}", tps);
}
