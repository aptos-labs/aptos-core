// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    db::AptosDB,
    ledger_db::LedgerDb,
    schema::transaction_info::TransactionInfoSchema,
    utils::truncation_helper::{
        get_current_version_in_state_merkle_db, get_state_kv_commit_progress,
    },
};
use aptos_config::config::{HotStateConfig, RocksdbConfigs, StorageDirPaths};
use aptos_storage_interface::{db_ensure as ensure, db_other_bail, Result};
use aptos_types::transaction::{TransactionInfo, Version};
use claims::assert_le;
use clap::Parser;
use itertools::Itertools;
use std::{fs, path::PathBuf, sync::Arc};

#[derive(Parser)]
#[clap(about = "Delete all data after the provided version.")]
#[clap(group(clap::ArgGroup::new("backup")
        .required(true)
        .args(&["backup_checkpoint_dir", "opt_out_backup_checkpoint"]),
))]
pub struct Cmd {
    // TODO(grao): Support db_path_overrides here.
    #[clap(long, value_parser)]
    db_dir: PathBuf,

    /// Must be the last version of a block.
    #[clap(long)]
    target_version: u64,

    #[clap(long, default_value_t = 1000)]
    ledger_db_batch_size: usize,

    #[clap(long, value_parser, group = "backup")]
    backup_checkpoint_dir: Option<PathBuf>,

    #[clap(long, group = "backup")]
    opt_out_backup_checkpoint: bool,
}

impl Cmd {
    pub fn run(self) -> Result<()> {
        if !self.opt_out_backup_checkpoint {
            let backup_checkpoint_dir = self.backup_checkpoint_dir.unwrap();
            ensure!(
                !backup_checkpoint_dir.exists(),
                "Backup dir already exists."
            );
            println!("Creating backup at: {:?}", backup_checkpoint_dir);
            fs::create_dir_all(&backup_checkpoint_dir)?;
            AptosDB::create_checkpoint(&self.db_dir, backup_checkpoint_dir)?;
            println!("Done!");
        } else {
            println!("Opted out backup creation!.");
        }

        let db_dir = StorageDirPaths::from_path(&self.db_dir);

        // Open DBs to validate versions and resolve target_version.
        let mut target_version = self.target_version;
        {
            let (ledger_db, _hot_state_merkle_db, state_merkle_db, _hot_state_kv_db, state_kv_db) =
                AptosDB::open_dbs(
                    &db_dir,
                    RocksdbConfigs::default(),
                    None,
                    None,
                    /*readonly=*/ false,
                    /*max_num_nodes_per_lru_cache_shard=*/ 0,
                    HotStateConfig {
                        delete_on_restart: false,
                        ..HotStateConfig::default()
                    },
                )?;

            let ledger_db = Arc::new(ledger_db);
            let state_merkle_db = Arc::new(state_merkle_db);
            let state_kv_db = Arc::new(state_kv_db);

            let overall_version = ledger_db
                .metadata_db()
                .get_synced_version()
                .expect("DB read failed.")
                .expect("Overall commit progress must exist.");
            let ledger_db_version = ledger_db
                .metadata_db()
                .get_ledger_commit_progress()
                .expect("Current version of ledger db must exist.");
            let state_kv_db_version = get_state_kv_commit_progress(&state_kv_db)?
                .expect("Current version of state kv db must exist.");
            let state_merkle_db_version = get_current_version_in_state_merkle_db(&state_merkle_db)?
                .expect("Current version of state merkle db must exist.");

            assert_le!(overall_version, ledger_db_version);
            assert_le!(overall_version, state_kv_db_version);
            assert_le!(state_merkle_db_version, overall_version);
            assert_le!(target_version, overall_version);

            println!(
                "overall_version: {}, ledger_db_version: {}, state_kv_db_version: {}, \
                 state_merkle_db_version: {}, target_version: {}",
                overall_version,
                ledger_db_version,
                state_kv_db_version,
                state_merkle_db_version,
                target_version,
            );

            ensure_block_end(&ledger_db, target_version, overall_version)?;

            if ledger_db.metadata_db().get_usage(target_version).is_err() {
                println!(
                    "Unable to truncate to version {}, since there is no VersionData on that version.",
                    target_version
                );
                println!(
                    "Trying to fallback to the largest valid version before version {}.",
                    target_version,
                );
                target_version = ledger_db
                    .metadata_db()
                    .get_usage_before_or_at(target_version)?
                    .0;
                target_version = block_end_with_usage_at_or_before(&ledger_db, target_version)?;
            }
        }

        println!("Starting db truncation to version {}...", target_version);
        AptosDB::truncate_to_version(&db_dir, target_version)?;
        println!("Done!");

        Ok(())
    }
}

fn is_block_end(ledger_db: &LedgerDb, version: Version) -> Result<bool> {
    Ok(ledger_db
        .transaction_info_db()
        .get_transaction_info(version)?
        .has_state_checkpoint_hash())
}

/// Errors with the nearest block ends if `version` is in the middle of a block.
fn ensure_block_end(
    ledger_db: &LedgerDb,
    version: Version,
    overall_version: Version,
) -> Result<()> {
    if is_block_end(ledger_db, version)? {
        return Ok(());
    }

    let mut iter = ledger_db
        .transaction_info_db_raw()
        .rev_iter::<TransactionInfoSchema>()?;
    iter.seek_for_prev(&version)?;
    let prev = first_block_end(iter)?;
    let mut iter = ledger_db
        .transaction_info_db_raw()
        .iter::<TransactionInfoSchema>()?;
    iter.seek(&version)?;
    let next = first_block_end(iter)?.filter(|v| *v <= overall_version);

    let fmt = |v: Option<Version>| v.map_or("none".to_string(), |v| v.to_string());
    db_other_bail!(
        "Version {} is in the middle of a block. Nearest block ends: {} before it, {} after it.",
        version,
        fmt(prev),
        fmt(next),
    );
}

/// Returns the last block end with usage at or before `version`, which has usage.
fn block_end_with_usage_at_or_before(ledger_db: &LedgerDb, version: Version) -> Result<Version> {
    let mut version = version;
    // Usage used to be written at the end of each chunk, which can be mid-block.
    while !is_block_end(ledger_db, version)? {
        let prev_version = version.checked_sub(1).expect("Genesis is a block end.");
        version = ledger_db
            .metadata_db()
            .get_usage_before_or_at(prev_version)?
            .0;
    }
    Ok(version)
}

fn first_block_end(
    iter: impl Iterator<Item = Result<(Version, TransactionInfo)>>,
) -> Result<Option<Version>> {
    iter.filter_map_ok(|(version, txn_info)| {
        txn_info.has_state_checkpoint_hash().then_some(version)
    })
    .next()
    .transpose()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_helper::arb_blocks_to_commit_with_params;
    use aptos_config::config::{
        BUFFERED_STATE_TARGET_ITEMS_FOR_TEST, DEFAULT_MAX_NUM_NODES_PER_LRU_CACHE_SHARD,
        NO_OP_STORAGE_PRUNER_CONFIG,
    };
    use aptos_storage_interface::DbReader;
    use aptos_temppath::TempPath;
    use proptest::prelude::*;

    fn open_db_with_persistent_hot_state(db_dir: &TempPath) -> AptosDB {
        AptosDB::open(
            StorageDirPaths::from_path(db_dir.path()),
            /*readonly=*/ false,
            NO_OP_STORAGE_PRUNER_CONFIG,
            RocksdbConfigs::default(),
            BUFFERED_STATE_TARGET_ITEMS_FOR_TEST,
            DEFAULT_MAX_NUM_NODES_PER_LRU_CACHE_SHARD,
            None,
            HotStateConfig {
                delete_on_restart: false,
                ..HotStateConfig::default()
            },
        )
        .unwrap()
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(5))]

        #[test]
        fn test_truncate_keeps_hot_state(
            (input, num_blocks_to_keep) in arb_blocks_to_commit_with_params(
                50,   /* num_accounts */
                2,    /* max_user_txns_per_block */
                4,    /* min_blocks */
                10,   /* max_blocks */
                true, /* make_hot_in_epilogue */
            ).prop_flat_map(|blocks| {
                let len = blocks.len();
                (Just(blocks), 1..len)
            }),
        ) {
            let hot_root_hash = |db: &AptosDB| {
                let live = db.state_store.live().unwrap();
                let state = live.current_state_locked();
                let latest = state.latest();
                (latest.version(), latest.summary().hot_root_hash().unwrap())
            };

            let tmp_dir = TempPath::new();
            let db = open_db_with_persistent_hot_state(&tmp_dir);
            let mut version = 0;
            let mut expected = None;
            for (i, (txns_to_commit, ledger_info_with_sigs)) in input.iter().enumerate() {
                db.save_transactions_for_test(
                    txns_to_commit,
                    version,
                    Some(ledger_info_with_sigs),
                    true,
                )
                .unwrap();
                version += txns_to_commit.len() as Version;
                if i + 1 == num_blocks_to_keep {
                    expected = Some(hot_root_hash(&db));
                }
            }
            drop(db);

            let expected = expected.unwrap();
            Cmd {
                db_dir: tmp_dir.path().to_path_buf(),
                target_version: expected.0.unwrap(),
                ledger_db_batch_size: 1000,
                backup_checkpoint_dir: None,
                opt_out_backup_checkpoint: true,
            }
            .run()
            .unwrap();

            prop_assert_eq!(
                hot_root_hash(&open_db_with_persistent_hot_state(&tmp_dir)),
                expected
            );
        }

        #[test]
        fn test_truncate_refuses_mid_block(
            (input, block_idx) in arb_blocks_to_commit_with_params(
                10,    /* num_accounts */
                2,     /* max_user_txns_per_block */
                2,     /* min_blocks */
                5,     /* max_blocks */
                false, /* make_hot_in_epilogue */
            ).prop_flat_map(|blocks| {
                let len = blocks.len();
                (Just(blocks), 1..len)
            }),
        ) {
            let tmp_dir = TempPath::new();
            let db = open_db_with_persistent_hot_state(&tmp_dir);
            let mut block_ends = Vec::new();
            let mut version = 0;
            for (txns_to_commit, ledger_info_with_sigs) in &input {
                db.save_transactions_for_test(
                    txns_to_commit,
                    version,
                    Some(ledger_info_with_sigs),
                    true,
                )
                .unwrap();
                version += txns_to_commit.len() as Version;
                block_ends.push(version - 1);
            }
            drop(db);

            // Every block has a user txn before the block-ending one.
            let err = Cmd {
                db_dir: tmp_dir.path().to_path_buf(),
                target_version: block_ends[block_idx] - 1,
                ledger_db_batch_size: 1000,
                backup_checkpoint_dir: None,
                opt_out_backup_checkpoint: true,
            }
            .run()
            .unwrap_err();
            let expected = format!(
                "Nearest block ends: {} before it, {} after it.",
                block_ends[block_idx - 1],
                block_ends[block_idx],
            );
            prop_assert!(err.to_string().contains(&expected), "{}", err);

            prop_assert_eq!(
                open_db_with_persistent_hot_state(&tmp_dir).expect_synced_version(),
                *block_ends.last().unwrap()
            );
        }
    }
}
