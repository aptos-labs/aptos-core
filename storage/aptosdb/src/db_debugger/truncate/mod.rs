// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    db::AptosDB,
    db_debugger::ShardingConfig,
    schema::db_metadata::{DbMetadataKey, DbMetadataSchema, DbMetadataValue},
    state_store::StateStore,
    utils::truncation_helper::{
        get_current_version_in_state_merkle_db, get_state_kv_commit_progress,
    },
};
use aptos_config::config::{RocksdbConfigs, StorageDirPaths};
use aptos_schemadb::batch::SchemaBatch;
use aptos_storage_interface::{db_ensure as ensure, AptosDbError, Result};
use claims::assert_le;
use clap::Parser;
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

    #[clap(long)]
    target_version: u64,

    #[clap(long, default_value_t = 1000)]
    ledger_db_batch_size: usize,

    #[clap(long, value_parser, group = "backup")]
    backup_checkpoint_dir: Option<PathBuf>,

    #[clap(long, group = "backup")]
    opt_out_backup_checkpoint: bool,

    #[clap(flatten)]
    sharding_config: ShardingConfig,
}

impl Cmd {
    pub fn run(self) -> Result<()> {
        if !self.opt_out_backup_checkpoint {
            let backup_checkpoint_dir = self.backup_checkpoint_dir.unwrap();
            ensure!(
                !backup_checkpoint_dir.exists(),
                "Backup dir already exists."
            );
            println!("Creating backup at: {:?}", &backup_checkpoint_dir);
            fs::create_dir_all(&backup_checkpoint_dir)?;
            AptosDB::create_checkpoint(
                &self.db_dir,
                backup_checkpoint_dir,
                self.sharding_config.enable_storage_sharding,
            )?;
            println!("Done!");
        } else {
            println!("Opted out backup creation!.");
        }

        let rocksdb_config = RocksdbConfigs {
            enable_storage_sharding: self.sharding_config.enable_storage_sharding,
            ..Default::default()
        };
        let (ledger_db, state_merkle_db, state_kv_db) = AptosDB::open_dbs(
            &StorageDirPaths::from_path(&self.db_dir),
            rocksdb_config,
            /*readonly=*/ false,
            /*max_num_nodes_per_lru_cache_shard=*/ 0,
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

        let mut target_version = self.target_version;

        assert_le!(overall_version, ledger_db_version);
        assert_le!(overall_version, state_kv_db_version);
        assert_le!(state_merkle_db_version, overall_version);
        assert_le!(target_version, overall_version);

        println!(
            "overall_version: {}, ledger_db_version: {}, state_kv_db_version: {}, state_merkle_db_version: {}, target_version: {}",
            overall_version, ledger_db_version, state_kv_db_version, state_merkle_db_version, target_version,
        );

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
        }

        println!("Starting db truncation...");
        let mut batch = SchemaBatch::new();
        batch.put::<DbMetadataSchema>(
            &DbMetadataKey::OverallCommitProgress,
            &DbMetadataValue::Version(target_version),
        )?;
        ledger_db.metadata_db().write_schemas(batch)?;

        StateStore::sync_commit_progress(
            Arc::clone(&ledger_db),
            Arc::clone(&state_kv_db),
            Arc::clone(&state_merkle_db),
            /*crash_if_difference_is_too_large=*/ false,
        );
        println!("Done!");

        if let Some(state_merkle_db_version) =
            get_current_version_in_state_merkle_db(&state_merkle_db)?
        {
            if state_merkle_db_version < target_version {
                println!(
                    "Trying to catch up state merkle db, by replaying write set in ledger db."
                );
                let version = StateStore::catch_up_state_merkle_db(
                    Arc::clone(&ledger_db),
                    Arc::clone(&state_merkle_db),
                    Arc::clone(&state_kv_db),
                )?;
                println!("Done! current_version: {:?}", version);
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        db::{test_helper::arb_blocks_to_commit_with_block_nums, AptosDB},
        schema::{
            epoch_by_version::EpochByVersionSchema,
            jellyfish_merkle_node::JellyfishMerkleNodeSchema, ledger_info::LedgerInfoSchema,
            stale_node_index::StaleNodeIndexSchema,
            stale_node_index_cross_epoch::StaleNodeIndexCrossEpochSchema,
            stale_state_value_index::StaleStateValueIndexSchema,
            stale_state_value_index_by_key_hash::StaleStateValueIndexByKeyHashSchema,
            state_value::StateValueSchema, state_value_by_key_hash::StateValueByKeyHashSchema,
            transaction::TransactionSchema, transaction_accumulator::TransactionAccumulatorSchema,
            transaction_info::TransactionInfoSchema, version_data::VersionDataSchema,
            write_set::WriteSetSchema,
        },
        utils::truncation_helper::num_frozen_nodes_in_accumulator,
    };
    use aptos_storage_interface::DbReader;
    use aptos_temppath::TempPath;
    use aptos_types::state_store::NUM_STATE_SHARDS;
    use proptest::prelude::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(1))]

        #[test]
        fn test_truncation(input in arb_blocks_to_commit_with_block_nums(80, 120)) {
            use aptos_config::config::DEFAULT_MAX_NUM_NODES_PER_LRU_CACHE_SHARD;
            aptos_logger::Logger::new().init();
            let sharding_config = ShardingConfig {
                enable_storage_sharding: input.1,
            };
            let tmp_dir = TempPath::new();

            let db = if input.1 { AptosDB::new_for_test_with_sharding(&tmp_dir, DEFAULT_MAX_NUM_NODES_PER_LRU_CACHE_SHARD) } else { AptosDB::new_for_test(&tmp_dir) };
            let mut version = 0;
            for (txns_to_commit, ledger_info_with_sigs) in input.0.iter() {
                db.save_transactions_for_test(
                    txns_to_commit,
                    version,
                    Some(ledger_info_with_sigs),
                    true,
                )
                    .unwrap();
                version += txns_to_commit.len() as u64;
            }

            let db_version = db.expect_synced_version();
            prop_assert_eq!(db_version, version - 1);

            drop(db);

            let mut target_version = db_version - 70;

            let cmd = Cmd {
                db_dir: tmp_dir.path().to_path_buf(),
                target_version,
                ledger_db_batch_size: 15,
                opt_out_backup_checkpoint: true,
                backup_checkpoint_dir: None,
                sharding_config: sharding_config.clone(),
            };

            cmd.run().unwrap();

            let db = if input.1 { AptosDB::new_for_test_with_sharding(&tmp_dir, DEFAULT_MAX_NUM_NODES_PER_LRU_CACHE_SHARD) } else { AptosDB::new_for_test(&tmp_dir) };
            let db_version = db.expect_synced_version();
            prop_assert!(db_version <= target_version);
            target_version = db_version;

            let txn_list_with_proof = db.get_transactions(0, db_version + 1, db_version, true).unwrap();
            prop_assert_eq!(txn_list_with_proof.transactions.len() as u64, db_version + 1);
            prop_assert_eq!(txn_list_with_proof.events.unwrap().len() as u64, db_version + 1);
            prop_assert_eq!(txn_list_with_proof.first_transaction_version, Some(0));

            let state_checkpoint_version = db.get_latest_state_checkpoint_version().unwrap().unwrap();
            let state_leaf_count = db.get_state_item_count(state_checkpoint_version).unwrap();
            let state_value_chunk_with_proof = db.get_state_value_chunk_with_proof(state_checkpoint_version, 0, state_leaf_count).unwrap();
            prop_assert_eq!(state_value_chunk_with_proof.first_index, 0);
            prop_assert_eq!(state_value_chunk_with_proof.last_index as usize, state_leaf_count - 1);
            prop_assert_eq!(state_value_chunk_with_proof.raw_values.len(), state_leaf_count);
            prop_assert!(state_value_chunk_with_proof.is_last_chunk());

            drop(db);

            let (ledger_db, state_merkle_db, state_kv_db) = AptosDB::open_dbs(
                &StorageDirPaths::from_path(tmp_dir.path()),
                RocksdbConfigs {
                    enable_storage_sharding: input.1,
                    ..Default::default()
                },
                /*readonly=*/ false,
                /*max_num_nodes_per_lru_cache_shard=*/ 0,
            ).unwrap();

            let ledger_metadata_db = ledger_db.metadata_db_arc();

            let num_frozen_nodes = num_frozen_nodes_in_accumulator(target_version + 1);
            let mut iter = ledger_db.transaction_accumulator_db_raw().iter::<TransactionAccumulatorSchema>().unwrap();
            iter.seek_to_last();
            let position = iter.next().transpose().unwrap().unwrap().0;
            prop_assert_eq!(position.to_postorder_index() + 1, num_frozen_nodes);

            let mut iter = ledger_db.transaction_info_db_raw().iter::<TransactionInfoSchema>().unwrap();
            iter.seek_to_last();
            prop_assert_eq!(iter.next().transpose().unwrap().unwrap().0, target_version);

            let mut iter = ledger_db.transaction_db_raw().iter::<TransactionSchema>().unwrap();
            iter.seek_to_last();
            prop_assert_eq!(iter.next().transpose().unwrap().unwrap().0, target_version);

            let mut iter = ledger_metadata_db.iter::<VersionDataSchema>().unwrap();
            iter.seek_to_last();
            prop_assert!(iter.next().transpose().unwrap().unwrap().0 <= target_version);

            let mut iter = ledger_db.write_set_db_raw().iter::<WriteSetSchema>().unwrap();
            iter.seek_to_last();
            prop_assert_eq!(iter.next().transpose().unwrap().unwrap().0, target_version);

            let mut iter = ledger_metadata_db.iter::<EpochByVersionSchema>().unwrap();
            iter.seek_to_last();
            let (version, epoch) = iter.next().transpose().unwrap().unwrap();
            prop_assert!(version <= target_version);

            let mut iter = ledger_metadata_db.iter::<LedgerInfoSchema>().unwrap();
            iter.seek_to_last();
            prop_assert_eq!(iter.next().transpose().unwrap().unwrap().0, epoch);

            if sharding_config.enable_storage_sharding {
                let mut iter = state_kv_db.metadata_db().iter::<StateValueByKeyHashSchema>().unwrap();
                iter.seek_to_first();
                for item in iter {
                    let ((_, version), _) = item.unwrap();
                    prop_assert!(version <= target_version);
                }

                let mut iter = state_kv_db.metadata_db().iter::<StaleStateValueIndexByKeyHashSchema>().unwrap();
                iter.seek_to_first();
                for item in iter {
                    let version = item.unwrap().0.stale_since_version;
                    prop_assert!(version <= target_version);
                }

            } else {
                let mut iter = state_kv_db.metadata_db().iter::<StateValueSchema>().unwrap();
                iter.seek_to_first();
                for item in iter {
                    let ((_, version), _) = item.unwrap();
                    prop_assert!(version <= target_version);
                }

                let mut iter = state_kv_db.metadata_db().iter::<StaleStateValueIndexSchema>().unwrap();
                iter.seek_to_first();
                for item in iter {
                    let version = item.unwrap().0.stale_since_version;
                    prop_assert!(version <= target_version);
                }
            }

            let mut iter = state_merkle_db.metadata_db().iter::<StaleNodeIndexSchema>().unwrap();
            iter.seek_to_first();
            for item in iter {
                let version = item.unwrap().0.stale_since_version;
                prop_assert!(version <= target_version);
            }

            let mut iter = state_merkle_db.metadata_db().iter::<StaleNodeIndexCrossEpochSchema>().unwrap();
            iter.seek_to_first();
            for item in iter {
                let version = item.unwrap().0.stale_since_version;
                prop_assert!(version <= target_version);
            }

            let mut iter = state_merkle_db.metadata_db().iter::<JellyfishMerkleNodeSchema>().unwrap();
            iter.seek_to_first();
            for item in iter {
                let version = item.unwrap().0.version();
                prop_assert!(version <= target_version);
            }

            if sharding_config.enable_storage_sharding {
                let state_merkle_db = Arc::new(state_merkle_db);
                for i in 0..NUM_STATE_SHARDS {
                    let mut kv_shard_iter = state_kv_db.db_shard(i).iter::<StateValueByKeyHashSchema>().unwrap();
                    kv_shard_iter.seek_to_first();
                    for item in kv_shard_iter {
                        let ((_, version), _) = item.unwrap();
                        prop_assert!(version <= target_version);
                    }

                    let value_index_shard_iter = state_kv_db.db_shard(i).iter::<StaleStateValueIndexByKeyHashSchema>().unwrap();
                    for item in value_index_shard_iter {
                        let version = item.unwrap().0.stale_since_version;
                        prop_assert!(version <= target_version);
                    }

                    let mut stale_node_ind_iter = state_merkle_db.db_shard(i).iter::<StaleNodeIndexSchema>().unwrap();
                    stale_node_ind_iter.seek_to_first();
                    for item in stale_node_ind_iter {
                        let version = item.unwrap().0.stale_since_version;
                        prop_assert!(version <= target_version);
                    }

                    let mut jelly_iter = state_merkle_db.db_shard(i).iter::<JellyfishMerkleNodeSchema>().unwrap();
                    jelly_iter.seek_to_first();
                    for item in jelly_iter {
                        let version = item.unwrap().0.version();
                        prop_assert!(version <= target_version);
                    }

                    let mut cross_iter = state_merkle_db.db_shard(i).iter::<StaleNodeIndexCrossEpochSchema>().unwrap();
                    cross_iter.seek_to_first();
                    for item in cross_iter {
                        let version = item.unwrap().0.stale_since_version;
                        prop_assert!(version <= target_version);
                    }
                }
            }
        }
    }
}
