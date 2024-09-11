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
    // 初始化临时目录和数据库
    let tmp_dir = TempPath::new();
    let db = AptosDB::new_for_test(&tmp_dir);

    // 生成测试数据
    let (input, _) = arb_blocks_to_commit_with_block_nums(10000, 20000)
        .new_tree(&mut TestRunner::default())
        .unwrap()
        .current();

    // 初始化内存状态
    let mut in_memory_state = db
        .state_store
        .buffered_state()
        .lock()
        .current_state()
        .clone();
    let _ancester = in_memory_state.current.clone();
    let mut cur_ver: Version = 0;

    // 开始计时
    let start = Instant::now();

    // 保存交易
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

    // 结束计时
    let duration = start.elapsed();
    let num_txns = cur_ver;
    let tps = num_txns as f64 / duration.as_secs_f64();

    println!(
        "Processed {} transactions in {:?} seconds",
        num_txns, duration
    );
    println!("Transactions per second: {}", tps);
}
