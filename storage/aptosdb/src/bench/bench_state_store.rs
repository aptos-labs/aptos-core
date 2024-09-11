use aptos_db::{db::test_helper::arb_state_kv_sets, utils::new_sharded_kv_schema_batch, AptosDB};
use aptos_schemadb::SchemaBatch;
use aptos_storage_interface::{jmt_update_refs, jmt_updates};
use aptos_temppath::TempPath;
use aptos_types::state_store::state_storage_usage::StateStorageUsage;
use arr_macro::arr;
use proptest::{prelude::*, strategy::ValueTree, test_runner::TestRunner};
use std::{collections::HashMap, time::Instant};

fn main() {
    // 初始化临时目录和数据库
    let tmp_dir = TempPath::new();
    let db = AptosDB::new_for_test(&tmp_dir);
    let store = &db.state_store;

    // 生成测试数据
    let input = arb_state_kv_sets(10000, 5, 5000)
        .new_tree(&mut TestRunner::default())
        .unwrap()
        .current();

    // 开始计时
    let start = Instant::now();
    let mut version: u64 = 0;

    // 写入状态数据
    for (i, kv_set) in input.iter().enumerate() {
        for (key, value) in kv_set {
            let value_state_set = vec![(key, value.as_ref())].into_iter().collect();
            let mut sharded_value_state_set = arr![HashMap::new(); 16];
            sharded_value_state_set[key.get_shard_id() as usize].insert(key.clone(), value.clone());
            let jmt_updates = jmt_updates(&value_state_set);

            let cur_version = version;
            version += 1;

            let root = store
                .merklize_value_set(
                    jmt_update_refs(&jmt_updates),
                    cur_version,
                    cur_version.checked_sub(1),
                )
                .unwrap();
            let ledger_batch = SchemaBatch::new();
            let sharded_state_kv_batches = new_sharded_kv_schema_batch();
            let state_kv_metadata_batch = SchemaBatch::new();
            store
                .put_value_sets(
                    vec![&sharded_value_state_set],
                    cur_version,
                    StateStorageUsage::new_untracked(),
                    None,
                    &ledger_batch,
                    &sharded_state_kv_batches,
                    false,
                    false,
                    None,
                )
                .unwrap();
            store
                .ledger_db
                .metadata_db()
                .write_schemas(ledger_batch)
                .unwrap();
            store
                .state_kv_db
                .commit(version, state_kv_metadata_batch, sharded_state_kv_batches)
                .unwrap();
        }
    }

    // 结束计时
    let duration = start.elapsed();
    let num_versions = input.len() as u64;
    let tps = num_versions as f64 / duration.as_secs_f64();

    println!(
        "Processed {} versions in {:?} seconds",
        num_versions, duration
    );
    println!("Versions per second: {}", tps);
}
