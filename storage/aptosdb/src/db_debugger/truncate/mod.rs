// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    jellyfish_merkle_node::JellyfishMerkleNodeSchema,
    schema::{
        db_metadata::{DbMetadataKey, DbMetadataSchema, DbMetadataValue},
        epoch_by_version::EpochByVersionSchema,
    },
    state_merkle_db::StateMerkleDb,
    utils::truncation_helper::{
        find_closest_node_version_at_or_before, get_current_version_in_state_merkle_db,
        get_ledger_commit_progress, get_overall_commit_progress, get_state_kv_commit_progress,
        truncate_state_merkle_db,
    },
    AptosDB, StateStore,
};
use anyhow::{ensure, Result};
use aptos_config::config::RocksdbConfigs;
use aptos_jellyfish_merkle::node_type::NodeKey;
use aptos_schemadb::{ReadOptions, DB};
use aptos_types::transaction::Version;
use claims::assert_le;
use clap::Parser;
use std::{fs, path::PathBuf, sync::Arc};

#[derive(Parser)]
#[clap(about = "Delete all data after the provided version.")]
#[clap(group(clap::ArgGroup::new("backup")
        .required(true)
        .args(&["backup-checkpoint-dir", "opt-out-backup-checkpoint"]),
))]
pub struct Cmd {
    #[clap(long, parse(from_os_str))]
    db_dir: PathBuf,

    #[clap(long)]
    target_version: u64,

    #[clap(long, default_value = "1000")]
    ledger_db_batch_size: usize,

    #[clap(long, parse(from_os_str), group = "backup")]
    backup_checkpoint_dir: Option<PathBuf>,

    #[clap(long, group = "backup")]
    opt_out_backup_checkpoint: bool,

    #[clap(long)]
    split_ledger_db: bool,
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
            // TODO(grao): Support sharded state merkle db here.
            AptosDB::create_checkpoint(
                &self.db_dir,
                backup_checkpoint_dir,
                self.split_ledger_db,
                false,
            )?;
            println!("Done!");
        } else {
            println!("Opted out backup creation!.");
        }

        let rocksdb_config = RocksdbConfigs {
            split_ledger_db: self.split_ledger_db,
            ..Default::default()
        };
        let (ledger_db, state_merkle_db, state_kv_db) = AptosDB::open_dbs(
            &self.db_dir,
            rocksdb_config,
            /*readonly=*/ false,
            /*max_num_nodes_per_lru_cache_shard=*/ 0,
        )?;

        let ledger_db = Arc::new(ledger_db);
        let state_merkle_db = Arc::new(state_merkle_db);
        let state_kv_db = Arc::new(state_kv_db);
        let overall_version = get_overall_commit_progress(ledger_db.metadata_db())?
            .expect("Overall commit progress must exist.");
        let ledger_db_version = get_ledger_commit_progress(ledger_db.metadata_db())?
            .expect("Current version of ledger db must exist.");
        let state_kv_db_version = get_state_kv_commit_progress(&state_kv_db)?
            .expect("Current version of state kv db must exist.");
        let state_merkle_db_version = get_current_version_in_state_merkle_db(&state_merkle_db)?
            .expect("Current version of state merkle db must exist.");

        assert_le!(overall_version, ledger_db_version);
        assert_le!(overall_version, state_kv_db_version);
        assert_le!(state_merkle_db_version, overall_version);
        assert_le!(self.target_version, overall_version);

        println!(
            "overall_version: {}, ledger_db_version: {}, state_kv_db_version: {}, state_merkle_db_version: {}, target_version: {}",
            overall_version, ledger_db_version, state_kv_db_version, state_merkle_db_version, self.target_version,
        );

        // TODO(grao): We are using a brute force implementation for now. We might be able to make
        // it faster, since our data is append only.
        if self.target_version < state_merkle_db_version {
            let state_merkle_target_version = Self::find_tree_root_at_or_before(
                ledger_db.metadata_db(),
                &state_merkle_db,
                self.target_version,
            )?
            .unwrap_or_else(|| {
                panic!(
                    "Could not find a valid root before or at version {}, maybe it was pruned?",
                    self.target_version
                )
            });

            println!(
                "Starting state merkle db truncation... target_version: {}",
                state_merkle_target_version
            );
            truncate_state_merkle_db(&state_merkle_db, state_merkle_target_version)?;
            println!("Done!");
        }

        println!("Starting ledger db and state kv db truncation...");
        ledger_db.metadata_db().put::<DbMetadataSchema>(
            &DbMetadataKey::OverallCommitProgress,
            &DbMetadataValue::Version(self.target_version),
        )?;
        StateStore::sync_commit_progress(
            Arc::clone(&ledger_db),
            Arc::clone(&state_kv_db),
            /*crash_if_difference_is_too_large=*/ false,
        );
        println!("Done!");

        if let Some(state_merkle_db_version) =
            get_current_version_in_state_merkle_db(&state_merkle_db)?
        {
            if state_merkle_db_version < self.target_version {
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

    fn find_tree_root_at_or_before(
        ledger_metadata_db: &DB,
        state_merkle_db: &StateMerkleDb,
        version: Version,
    ) -> Result<Option<Version>> {
        match find_closest_node_version_at_or_before(state_merkle_db, version)? {
            Some(closest_version) => {
                if Self::root_exists_at_version(state_merkle_db, closest_version)? {
                    return Ok(Some(closest_version));
                }
                let mut iter =
                    ledger_metadata_db.iter::<EpochByVersionSchema>(ReadOptions::default())?;
                iter.seek_for_prev(&version)?;
                match iter.next().transpose()? {
                    Some((closest_epoch_version, _)) => {
                        if Self::root_exists_at_version(state_merkle_db, closest_epoch_version)? {
                            Ok(Some(closest_epoch_version))
                        } else {
                            Ok(None)
                        }
                    },
                    None => Ok(None),
                }
            },
            None => Ok(None),
        }
    }

    fn root_exists_at_version(state_merkle_db: &StateMerkleDb, version: Version) -> Result<bool> {
        Ok(state_merkle_db
            .metadata_db()
            .get::<JellyfishMerkleNodeSchema>(&NodeKey::new_empty_path(version))?
            .is_some())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        schema::{
            epoch_by_version::EpochByVersionSchema, ledger_info::LedgerInfoSchema,
            stale_node_index::StaleNodeIndexSchema,
            stale_node_index_cross_epoch::StaleNodeIndexCrossEpochSchema,
            stale_state_value_index::StaleStateValueIndexSchema, state_value::StateValueSchema,
            transaction::TransactionSchema, transaction_accumulator::TransactionAccumulatorSchema,
            transaction_info::TransactionInfoSchema, version_data::VersionDataSchema,
            write_set::WriteSetSchema,
        },
        test_helper::{arb_blocks_to_commit_with_block_nums, update_in_memory_state},
        utils::truncation_helper::num_frozen_nodes_in_accumulator,
        AptosDB,
    };
    use aptos_storage_interface::{DbReader, DbWriter};
    use aptos_temppath::TempPath;
    use proptest::prelude::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(10))]

        #[test]
        fn test_truncation(input in arb_blocks_to_commit_with_block_nums(80, 120)) {
            aptos_logger::Logger::new().init();
            let tmp_dir = TempPath::new();
            let db = AptosDB::new_for_test(&tmp_dir);
            let mut in_memory_state = db.state_store.buffered_state().lock().current_state().clone();
            let _ancestor = in_memory_state.base.clone();
            let mut version = 0;
            for (txns_to_commit, ledger_info_with_sigs) in input.iter() {
                update_in_memory_state(&mut in_memory_state, txns_to_commit.as_slice());
                db.save_transactions(txns_to_commit, version, version.checked_sub(1), Some(ledger_info_with_sigs), true, in_memory_state.clone())
                    .unwrap();
                version += txns_to_commit.len() as u64;
            }

            let db_version = db.get_latest_transaction_info_option().unwrap().unwrap().0;
            prop_assert_eq!(db_version, version - 1);

            drop(db);

            let target_version = db_version - 70;

            let cmd = Cmd {
                db_dir: tmp_dir.path().to_path_buf(),
                target_version,
                ledger_db_batch_size: 15,
                opt_out_backup_checkpoint: true,
                backup_checkpoint_dir: None,
                split_ledger_db: false,
            };

            cmd.run().unwrap();

            let db = AptosDB::new_for_test(&tmp_dir);
            let db_version = db.get_latest_transaction_info_option().unwrap().unwrap().0;
            prop_assert_eq!(db_version, target_version);

            let txn_list_with_proof = db.get_transactions(0, db_version + 1, db_version, true).unwrap();
            prop_assert_eq!(txn_list_with_proof.transactions.len() as u64, db_version + 1);
            prop_assert_eq!(txn_list_with_proof.events.unwrap().len() as u64, db_version + 1);
            prop_assert_eq!(txn_list_with_proof.first_transaction_version, Some(0));

            let state_checkpoint_version = db.get_latest_state_checkpoint_version().unwrap().unwrap();
            let state_leaf_count = db.get_state_leaf_count(state_checkpoint_version).unwrap();
            let state_value_chunk_with_proof = db.get_state_value_chunk_with_proof(state_checkpoint_version, 0, state_leaf_count).unwrap();
            prop_assert_eq!(state_value_chunk_with_proof.first_index, 0);
            prop_assert_eq!(state_value_chunk_with_proof.last_index as usize, state_leaf_count - 1);
            prop_assert_eq!(state_value_chunk_with_proof.raw_values.len(), state_leaf_count);
            prop_assert!(state_value_chunk_with_proof.is_last_chunk());

            drop(db);

            let (ledger_db, state_merkle_db, state_kv_db) = AptosDB::open_dbs(
                tmp_dir.path().to_path_buf(),
                RocksdbConfigs::default(),
                /*readonly=*/ false,
                /*max_num_nodes_per_lru_cache_shard=*/ 0,
            ).unwrap();

            let num_frozen_nodes = num_frozen_nodes_in_accumulator(target_version + 1);
            let mut iter = ledger_db.transaction_accumulator_db().iter::<TransactionAccumulatorSchema>(ReadOptions::default()).unwrap();
            iter.seek_to_last();
            let position = iter.next().transpose().unwrap().unwrap().0;
            prop_assert_eq!(position.to_postorder_index() + 1, num_frozen_nodes);

            let mut iter = ledger_db.transaction_info_db().iter::<TransactionInfoSchema>(ReadOptions::default()).unwrap();
            iter.seek_to_last();
            prop_assert_eq!(iter.next().transpose().unwrap().unwrap().0, target_version);

            let mut iter = ledger_db.transaction_db().iter::<TransactionSchema>(ReadOptions::default()).unwrap();
            iter.seek_to_last();
            prop_assert_eq!(iter.next().transpose().unwrap().unwrap().0, target_version);

            let mut iter = ledger_db.metadata_db().iter::<VersionDataSchema>(ReadOptions::default()).unwrap();
            iter.seek_to_last();
            prop_assert_eq!(iter.next().transpose().unwrap().unwrap().0, target_version);

            let mut iter = ledger_db.write_set_db().iter::<WriteSetSchema>(ReadOptions::default()).unwrap();
            iter.seek_to_last();
            prop_assert_eq!(iter.next().transpose().unwrap().unwrap().0, target_version);

            let mut iter = ledger_db.metadata_db().iter::<EpochByVersionSchema>(ReadOptions::default()).unwrap();
            iter.seek_to_last();
            let (version, epoch) = iter.next().transpose().unwrap().unwrap();
            prop_assert!(version <= target_version);

            let mut iter = ledger_db.metadata_db().iter::<LedgerInfoSchema>(ReadOptions::default()).unwrap();
            iter.seek_to_last();
            prop_assert_eq!(iter.next().transpose().unwrap().unwrap().0, epoch);

            // TODO(grao): Support sharding here.
            let mut iter = state_kv_db.metadata_db().iter::<StateValueSchema>(ReadOptions::default()).unwrap();
            iter.seek_to_first();
            for item in iter {
                let ((_, version), _) = item.unwrap();
                prop_assert!(version <= target_version);
            }

            // TODO(grao): Support sharding here.
            let mut iter = state_kv_db.metadata_db().iter::<StaleStateValueIndexSchema>(ReadOptions::default()).unwrap();
            iter.seek_to_first();
            for item in iter {
                let version = item.unwrap().0.stale_since_version;
                prop_assert!(version <= target_version);
            }

            // TODO(grao): Support sharding here.
            let mut iter = state_merkle_db.metadata_db().iter::<StaleNodeIndexSchema>(ReadOptions::default()).unwrap();
            iter.seek_to_first();
            for item in iter {
                let version = item.unwrap().0.stale_since_version;
                prop_assert!(version <= target_version);
            }

            // TODO(grao): Support sharding here.
            let mut iter = state_merkle_db.metadata_db().iter::<StaleNodeIndexCrossEpochSchema>(ReadOptions::default()).unwrap();
            iter.seek_to_first();
            for item in iter {
                let version = item.unwrap().0.stale_since_version;
                prop_assert!(version <= target_version);
            }

            // TODO(grao): Support sharding here.
            let mut iter = state_merkle_db.metadata_db().iter::<JellyfishMerkleNodeSchema>(ReadOptions::default()).unwrap();
            iter.seek_to_first();
            for item in iter {
                let version = item.unwrap().0.version();
                prop_assert!(version <= target_version);
            }
        }
    }
}
