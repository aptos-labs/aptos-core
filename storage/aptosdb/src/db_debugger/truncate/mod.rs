// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    jellyfish_merkle_node::JellyfishMerkleNodeSchema,
    schema::{
        epoch_by_version::EpochByVersionSchema, ledger_info::LedgerInfoSchema,
        state_value::StateValueSchema, transaction::TransactionSchema, write_set::WriteSetSchema,
    },
    stale_node_index::StaleNodeIndexSchema,
    stale_node_index_cross_epoch::StaleNodeIndexCrossEpochSchema,
    stale_state_value_index::StaleStateValueIndexSchema,
    transaction_accumulator::TransactionAccumulatorSchema,
    transaction_info::TransactionInfoSchema,
    version_data::VersionDataSchema,
    AptosDB, EventStore, StateStore, TransactionStore,
};
use anyhow::{ensure, Result};
use aptos_config::config::RocksdbConfigs;
use aptos_jellyfish_merkle::{node_type::NodeKey, StaleNodeIndex};
use aptos_schemadb::{
    schema::{Schema, SeekKeyCodec},
    ReadOptions, SchemaBatch, DB,
};
use aptos_types::{proof::position::Position, transaction::Version};
use claims::{assert_ge, assert_le, assert_lt};
use clap::Parser;
use status_line::StatusLine;
use std::{
    fmt::{Display, Formatter},
    fs,
    path::PathBuf,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};

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
}

struct Progress {
    current_version: AtomicU64,
    target_version: Version,
}

impl Progress {
    pub fn new(target_version: Version) -> Self {
        Self {
            current_version: 0.into(),
            target_version,
        }
    }

    pub fn set_current_version(&self, current_version: Version) {
        self.current_version
            .store(current_version, Ordering::Relaxed);
    }
}

impl Display for Progress {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "current: {}, target: {}",
            self.current_version.load(Ordering::Relaxed),
            self.target_version
        )
    }
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
            AptosDB::create_checkpoint(&self.db_dir, backup_checkpoint_dir)?;
            println!("Done!");
        } else {
            println!("Opted out backup creation!.");
        }

        let (ledger_db, state_merkle_db, _kv_db) = AptosDB::open_dbs(
            &self.db_dir,
            RocksdbConfigs::default(),
            /*readonly=*/ false,
        )?;

        // TODO(grao): Handle kv db once we enable it.

        let ledger_db_version = Self::get_current_version_in_ledger_db(&ledger_db)?
            .expect("Current version of ledger db must exist.");
        let state_merkle_db_version =
            Self::get_current_version_in_state_merkle_db(&state_merkle_db)?
                .expect("Current version of state merkle db must exist.");

        assert_le!(state_merkle_db_version, ledger_db_version);
        assert_le!(self.target_version, ledger_db_version);

        println!(
            "ledger_db_version: {}, state_merkle_db_version: {}, target_version: {}",
            ledger_db_version, state_merkle_db_version, self.target_version,
        );

        // TODO(grao): We are using a brute force implementation for now. We might be able to make
        // it faster, since our data is append only.
        if self.target_version < state_merkle_db_version {
            let state_merkle_target_version = Self::find_tree_root_at_or_before(
                &ledger_db,
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
            Self::truncate_state_merkle_db(&state_merkle_db, state_merkle_target_version)?;
            println!("Done!");
        }

        println!("Starting ledger db truncation...");
        let ledger_db = Arc::new(ledger_db);
        Self::truncate_ledger_db(
            Arc::clone(&ledger_db),
            ledger_db_version,
            self.target_version,
            self.ledger_db_batch_size,
        )?;
        println!("Done!");

        if let Some(state_merkle_db_version) =
            Self::get_current_version_in_state_merkle_db(&state_merkle_db)?
        {
            if state_merkle_db_version < self.target_version {
                println!(
                    "Trying to catch up state merkle db, by replaying write set in ledger db."
                );
                let version =
                    StateStore::catch_up_state_merkle_db(Arc::clone(&ledger_db), state_merkle_db)?;
                println!("Done! current_version: {:?}", version);
            }
        }

        Ok(())
    }

    fn get_current_version_in_ledger_db(ledger_db: &DB) -> Result<Option<Version>> {
        let mut iter = ledger_db.iter::<TransactionInfoSchema>(ReadOptions::default())?;
        iter.seek_to_last();
        Ok(iter.next().transpose()?.map(|item| item.0))
    }

    fn get_current_version_in_state_merkle_db(state_merkle_db: &DB) -> Result<Option<Version>> {
        Self::find_closest_node_version_at_or_before(state_merkle_db, u64::max_value())
    }

    fn find_tree_root_at_or_before(
        ledger_db: &DB,
        state_merkle_db: &DB,
        version: Version,
    ) -> Result<Option<Version>> {
        match Self::find_closest_node_version_at_or_before(state_merkle_db, version)? {
            Some(closest_version) => {
                if Self::root_exists_at_version(state_merkle_db, closest_version)? {
                    return Ok(Some(closest_version));
                }
                let mut iter = ledger_db.iter::<EpochByVersionSchema>(ReadOptions::default())?;
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

    fn root_exists_at_version(state_merkle_db: &DB, version: Version) -> Result<bool> {
        Ok(state_merkle_db
            .get::<JellyfishMerkleNodeSchema>(&NodeKey::new_empty_path(version))?
            .is_some())
    }

    fn find_closest_node_version_at_or_before(
        state_merkle_db: &DB,
        version: Version,
    ) -> Result<Option<Version>> {
        let mut iter = state_merkle_db.rev_iter::<JellyfishMerkleNodeSchema>(Default::default())?;
        iter.seek_for_prev(&NodeKey::new_empty_path(version))?;
        Ok(iter.next().transpose()?.map(|item| item.0.version()))
    }

    fn truncate_ledger_db(
        ledger_db: Arc<DB>,
        current_version: Version,
        target_version: Version,
        batch_size: usize,
    ) -> Result<()> {
        let status = StatusLine::new(Progress::new(target_version));

        let event_store = EventStore::new(Arc::clone(&ledger_db));
        let transaction_store = TransactionStore::new(Arc::clone(&ledger_db));

        let mut current_version = current_version;
        while current_version > target_version {
            let start_version =
                std::cmp::max(current_version - batch_size as u64 + 1, target_version + 1);
            let end_version = current_version + 1;
            Self::truncate_ledger_db_single_batch(
                &ledger_db,
                &event_store,
                &transaction_store,
                start_version,
                end_version,
            )?;
            current_version = start_version - 1;
            status.set_current_version(current_version);
        }
        assert_eq!(current_version, target_version);
        Ok(())
    }

    fn num_frozen_nodes_in_accumulator(num_leaves: u64) -> u64 {
        2 * num_leaves - num_leaves.count_ones() as u64
    }

    fn truncate_transaction_accumulator(
        ledger_db: &DB,
        start_version: Version,
        end_version: Version,
        batch: &SchemaBatch,
    ) -> Result<()> {
        let num_frozen_nodes = Self::num_frozen_nodes_in_accumulator(end_version);
        let mut iter = ledger_db.iter::<TransactionAccumulatorSchema>(ReadOptions::default())?;
        iter.seek_to_last();
        let (position, _) = iter.next().transpose()?.unwrap();
        assert_eq!(position.to_postorder_index() + 1, num_frozen_nodes);

        let num_frozen_nodes_after_this_batch =
            Self::num_frozen_nodes_in_accumulator(start_version);

        let mut num_nodes_to_delete = num_frozen_nodes - num_frozen_nodes_after_this_batch;

        let start_position = Position::from_postorder_index(num_frozen_nodes_after_this_batch)?;
        iter.seek(&start_position)?;

        for item in iter {
            let (position, _) = item?;
            batch.delete::<TransactionAccumulatorSchema>(&position)?;
            num_nodes_to_delete -= 1;
        }

        assert_eq!(num_nodes_to_delete, 0);

        Ok(())
    }

    fn truncate_ledger_db_single_batch(
        ledger_db: &DB,
        event_store: &EventStore,
        transaction_store: &TransactionStore,
        start_version: Version,
        end_version: Version,
    ) -> Result<()> {
        let batch = SchemaBatch::new();

        Self::delete_transaction_index_data(transaction_store, start_version, end_version, &batch)?;
        Self::delete_per_epoch_data(ledger_db, start_version, end_version, &batch)?;
        Self::delete_per_version_data(start_version, end_version, &batch)?;
        Self::delete_state_value_and_index(ledger_db, start_version, end_version, &batch)?;

        event_store.prune_events(start_version, end_version, &batch)?;

        Self::truncate_transaction_accumulator(ledger_db, start_version, end_version, &batch)?;

        ledger_db.write_schemas(batch)
    }

    fn delete_transaction_index_data(
        transaction_store: &TransactionStore,
        start_version: Version,
        end_version: Version,
        batch: &SchemaBatch,
    ) -> Result<()> {
        let transactions = transaction_store
            .get_transaction_iter(start_version, (end_version - start_version) as usize)?
            .collect::<Result<Vec<_>>>()?;
        transaction_store.prune_transaction_by_account(&transactions, batch)?;
        transaction_store.prune_transaction_by_hash(&transactions, batch)?;

        Ok(())
    }

    fn delete_per_epoch_data(
        ledger_db: &DB,
        start_version: Version,
        end_version: Version,
        batch: &SchemaBatch,
    ) -> Result<()> {
        let mut iter = ledger_db.iter::<LedgerInfoSchema>(ReadOptions::default())?;
        iter.seek_to_last();
        if let Some((epoch, ledger_info)) = iter.next().transpose()? {
            let version = ledger_info.commit_info().version();
            assert_lt!(version, end_version);
            if version >= start_version {
                batch.delete::<LedgerInfoSchema>(&epoch)?;
            }
        }

        let mut iter = ledger_db.iter::<EpochByVersionSchema>(ReadOptions::default())?;
        iter.seek(&start_version)?;

        for item in iter {
            let (version, epoch) = item?;
            assert_lt!(version, end_version);
            batch.delete::<EpochByVersionSchema>(&version)?;
            batch.delete::<LedgerInfoSchema>(&epoch)?;
        }

        Ok(())
    }

    fn delete_per_version_data(
        start_version: Version,
        end_version: Version,
        batch: &SchemaBatch,
    ) -> Result<()> {
        for version in start_version..end_version {
            batch.delete::<TransactionInfoSchema>(&version)?;
            batch.delete::<TransactionSchema>(&version)?;
            batch.delete::<VersionDataSchema>(&version)?;
            batch.delete::<WriteSetSchema>(&version)?;
        }

        Ok(())
    }

    fn delete_state_value_and_index(
        ledger_db: &DB,
        start_version: Version,
        end_version: Version,
        batch: &SchemaBatch,
    ) -> Result<()> {
        let mut iter = ledger_db.iter::<StaleStateValueIndexSchema>(ReadOptions::default())?;
        iter.seek(&start_version)?;

        for item in iter {
            let (index, _) = item?;
            assert_lt!(index.stale_since_version, end_version);
            batch.delete::<StaleStateValueIndexSchema>(&index)?;
            batch.delete::<StateValueSchema>(&(index.state_key, index.stale_since_version))?;
        }

        Ok(())
    }

    fn truncate_state_merkle_db(state_merkle_db: &DB, target_version: Version) -> Result<()> {
        let status = StatusLine::new(Progress::new(target_version));
        loop {
            let batch = SchemaBatch::new();
            let current_version = Self::get_current_version_in_state_merkle_db(state_merkle_db)?
                .expect("Current version of state merkle db must exist.");
            status.set_current_version(current_version);
            assert_ge!(current_version, target_version);
            if current_version == target_version {
                break;
            }

            let mut iter =
                state_merkle_db.iter::<JellyfishMerkleNodeSchema>(ReadOptions::default())?;
            iter.seek(&NodeKey::new_empty_path(current_version))?;
            for item in iter {
                let (key, _) = item?;
                batch.delete::<JellyfishMerkleNodeSchema>(&key)?;
            }

            Self::delete_stale_node_index_at_version::<StaleNodeIndexSchema>(
                state_merkle_db,
                current_version,
                &batch,
            )?;
            Self::delete_stale_node_index_at_version::<StaleNodeIndexCrossEpochSchema>(
                state_merkle_db,
                current_version,
                &batch,
            )?;

            state_merkle_db.write_schemas(batch)?;
        }

        Ok(())
    }

    fn delete_stale_node_index_at_version<S>(
        state_merkle_db: &DB,
        version: Version,
        batch: &SchemaBatch,
    ) -> Result<()>
    where
        S: Schema<Key = StaleNodeIndex>,
        Version: SeekKeyCodec<S>,
    {
        let mut iter = state_merkle_db.iter::<S>(ReadOptions::default())?;
        iter.seek(&version)?;
        for item in iter {
            let (index, _) = item?;
            assert_eq!(index.stale_since_version, version);
            batch.delete::<S>(&index)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        test_helper::{arb_blocks_to_commit_with_block_nums, update_in_memory_state},
        AptosDB,
    };
    use aptos_storage_interface::{DbReader, DbWriter};
    use aptos_temppath::TempPath;
    use proptest::prelude::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(10))]

        #[test]
        fn test_truncation(input in arb_blocks_to_commit_with_block_nums(80, 120)) {
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

            let (ledger_db, state_merkle_db, _) = AptosDB::open_dbs(
                tmp_dir.path().to_path_buf(),
                RocksdbConfigs::default(),
                /*readonly=*/ false,
            ).unwrap();

            let num_frozen_nodes = Cmd::num_frozen_nodes_in_accumulator(target_version + 1);
            let mut iter = ledger_db.iter::<TransactionAccumulatorSchema>(ReadOptions::default()).unwrap();
            iter.seek_to_last();
            let position = iter.next().transpose().unwrap().unwrap().0;
            prop_assert_eq!(position.to_postorder_index() + 1, num_frozen_nodes);

            let mut iter = ledger_db.iter::<TransactionInfoSchema>(ReadOptions::default()).unwrap();
            iter.seek_to_last();
            prop_assert_eq!(iter.next().transpose().unwrap().unwrap().0, target_version);

            let mut iter = ledger_db.iter::<TransactionSchema>(ReadOptions::default()).unwrap();
            iter.seek_to_last();
            prop_assert_eq!(iter.next().transpose().unwrap().unwrap().0, target_version);

            let mut iter = ledger_db.iter::<VersionDataSchema>(ReadOptions::default()).unwrap();
            iter.seek_to_last();
            prop_assert_eq!(iter.next().transpose().unwrap().unwrap().0, target_version);

            let mut iter = ledger_db.iter::<WriteSetSchema>(ReadOptions::default()).unwrap();
            iter.seek_to_last();
            prop_assert_eq!(iter.next().transpose().unwrap().unwrap().0, target_version);

            let mut iter = ledger_db.iter::<EpochByVersionSchema>(ReadOptions::default()).unwrap();
            iter.seek_to_last();
            let (version, epoch) = iter.next().transpose().unwrap().unwrap();
            prop_assert!(version <= target_version);

            let mut iter = ledger_db.iter::<LedgerInfoSchema>(ReadOptions::default()).unwrap();
            iter.seek_to_last();
            prop_assert_eq!(iter.next().transpose().unwrap().unwrap().0, epoch);

            let mut iter = ledger_db.iter::<StateValueSchema>(ReadOptions::default()).unwrap();
            iter.seek_to_first();
            for item in iter {
                let ((_, version), _) = item.unwrap();
                prop_assert!(version <= target_version);
            }

            let mut iter = ledger_db.iter::<StaleStateValueIndexSchema>(ReadOptions::default()).unwrap();
            iter.seek_to_first();
            for item in iter {
                let version = item.unwrap().0.stale_since_version;
                prop_assert!(version <= target_version);
            }

            let mut iter = state_merkle_db.iter::<StaleNodeIndexSchema>(ReadOptions::default()).unwrap();
            iter.seek_to_first();
            for item in iter {
                let version = item.unwrap().0.stale_since_version;
                prop_assert!(version <= target_version);
            }

            let mut iter = state_merkle_db.iter::<StaleNodeIndexCrossEpochSchema>(ReadOptions::default()).unwrap();
            iter.seek_to_first();
            for item in iter {
                let version = item.unwrap().0.stale_since_version;
                prop_assert!(version <= target_version);
            }

            let mut iter = state_merkle_db.iter::<JellyfishMerkleNodeSchema>(ReadOptions::default()).unwrap();
            iter.seek_to_first();
            for item in iter {
                let version = item.unwrap().0.version();
                prop_assert!(version <= target_version);
            }
        }
    }
}
