// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_backup_service::start_backup_service;
use aptos_config::{
    config::{
        HotStateConfig, RocksdbConfigs, StorageDirPaths, BUFFERED_STATE_TARGET_ITEMS_FOR_TEST,
        DEFAULT_MAX_NUM_NODES_PER_LRU_CACHE_SHARD, NO_OP_STORAGE_PRUNER_CONFIG,
    },
    utils::get_available_port,
};
use aptos_db::{
    db::test_helper::{arb_blocks_to_commit, arb_blocks_to_commit_with_positions},
    AptosDB,
};
use aptos_proptest_helpers::ValueGenerator;
use aptos_temppath::TempPath;
use aptos_types::{
    ledger_info::LedgerInfoWithSignatures,
    transaction::{TransactionToCommit, Version},
};
use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::Path,
    sync::Arc,
};
use tokio::runtime::Runtime;

pub fn tmp_db_empty() -> (TempPath, Arc<AptosDB>) {
    let tmpdir = TempPath::new();
    let db = Arc::new(AptosDB::new_for_test(&tmpdir));

    (tmpdir, db)
}

pub fn tmp_db_with_random_content() -> (
    TempPath,
    Arc<AptosDB>,
    Vec<(Vec<TransactionToCommit>, LedgerInfoWithSignatures)>,
) {
    let (tmpdir, db) = tmp_db_empty();
    let blocks = ValueGenerator::new().generate(arb_blocks_to_commit());
    commit_blocks(&db, &blocks);
    (tmpdir, db, blocks)
}

/// Like [`tmp_db_with_random_content`], on a DB with native-position storage
/// whose checkpoints commit position state roots.
pub fn tmp_db_with_random_positions() -> (
    TempPath,
    Arc<AptosDB>,
    Vec<(Vec<TransactionToCommit>, LedgerInfoWithSignatures)>,
) {
    let tmpdir = TempPath::new();
    let db = Arc::new(open_with_positions(tmpdir.path()));
    let blocks = ValueGenerator::new().generate(arb_blocks_to_commit_with_positions());
    commit_blocks(&db, &blocks);
    (tmpdir, db, blocks)
}

pub fn open_with_positions(path: &Path) -> AptosDB {
    AptosDB::open(
        StorageDirPaths::from_path(path),
        false, /* readonly */
        NO_OP_STORAGE_PRUNER_CONFIG,
        RocksdbConfigs {
            enable_trading_native: true,
            ..Default::default()
        },
        BUFFERED_STATE_TARGET_ITEMS_FOR_TEST,
        DEFAULT_MAX_NUM_NODES_PER_LRU_CACHE_SHARD,
        None,
        HotStateConfig::default(),
    )
    .expect("open with native-position storage")
}

fn commit_blocks(db: &AptosDB, blocks: &[(Vec<TransactionToCommit>, LedgerInfoWithSignatures)]) {
    let mut cur_ver: Version = 0;
    for (txns_to_commit, ledger_info_with_sigs) in blocks {
        db.save_transactions_for_test(
            txns_to_commit,
            cur_ver, /* first_version */
            Some(ledger_info_with_sigs),
            true, /* sync_commit */
        )
        .unwrap();
        cur_ver += txns_to_commit.len() as u64;
    }
}

pub fn start_local_backup_service(db: Arc<AptosDB>) -> (Runtime, u16) {
    let port = get_available_port();
    let rt = start_backup_service(
        SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port),
        db,
        None,
    );
    (rt, port)
}
