// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use anyhow::{bail, Error, Result};
use aptos_backup_cli::utils::{ReplayConcurrencyLevelOpt, RocksdbOpt};
use aptos_config::config::{
    StorageDirPaths, BUFFERED_STATE_TARGET_ITEMS, DEFAULT_MAX_NUM_NODES_PER_LRU_CACHE_SHARD,
    NO_OP_STORAGE_PRUNER_CONFIG,
};
use aptos_db::AptosDB;
use aptos_executor_types::ParsedTransactionOutput;
use aptos_logger::{debug, info};
use aptos_storage_interface::{state_view::DbStateViewAtVersion, AptosDbError, DbReader};
use aptos_types::{
    contract_event::ContractEvent,
    transaction::{
        signature_verified_transaction::SignatureVerifiedTransaction, TransactionInfo,
        TransactionOutput, Version,
    },
    write_set::WriteSet,
};
use aptos_vm::{AptosVM, VMExecutor};
use clap::Parser;
use itertools::multizip;
use std::{path::PathBuf, process, sync::Arc, time::Instant};

// Replay Verify controller is responsible for providing legit range with start and end versions.
#[derive(Parser)]
pub struct Opt {
    #[clap(
        long,
        help = "The first transaction version required to be replayed and verified"
    )]
    start_version: Version,
    #[clap(
        long,
        help = "The last transaction version required to be replayed and verified"
    )]
    end_version: Version,
    #[clap(flatten)]
    replay_concurrency_level: ReplayConcurrencyLevelOpt,
    #[clap(long = "target-db-dir", value_parser)]
    pub db_dir: PathBuf,
    #[clap(flatten)]
    pub rocksdb_opt: RocksdbOpt,
    #[clap(
        long,
        default_value = "2000",
        help = "The number of transactions to be replayed in a chunk"
    )]
    pub chunk_size: usize,
}

impl Opt {
    pub async fn run(self) -> Result<()> {
        AptosVM::set_concurrency_level_once(self.replay_concurrency_level.get());
        let replay_start: Instant = Instant::now();
        let aptos_db = AptosDB::open(
            StorageDirPaths::from_path(self.db_dir.as_path()),
            true,
            NO_OP_STORAGE_PRUNER_CONFIG,
            self.rocksdb_opt.into(),
            false,
            BUFFERED_STATE_TARGET_ITEMS,
            DEFAULT_MAX_NUM_NODES_PER_LRU_CACHE_SHARD,
            None,
        )?;

        let backup_handler = aptos_db.get_backup_handler();
        let arc_db = Arc::new(aptos_db) as Arc<dyn DbReader>;
        let (start, limit) =
            Self::get_start_and_limit(&arc_db, self.start_version, self.end_version)?;
        info!(
            start_version = start,
            end_version = start + limit,
            "Replaying transactions."
        );
        let mut failed_txns = Vec::new();
        let txn_iter = backup_handler.get_transaction_iter(start, limit as usize)?;
        let mut cur_txns = Vec::new();
        let mut expected_events = Vec::new();
        let mut expected_writesets = Vec::new();
        let mut expected_txn_infos = Vec::new();
        let mut chunk_start_version = start;
        for (idx, item) in txn_iter.enumerate() {
            let (input_txn, expected_txn_info, expected_event, expected_writeset) = item?;
            let is_epoch_ending = ParsedTransactionOutput::parse_reconfig_events(&expected_event)
                .next()
                .is_some();
            cur_txns.push(input_txn);
            expected_txn_infos.push(expected_txn_info);
            expected_events.push(expected_event);
            expected_writesets.push(expected_writeset);
            if is_epoch_ending || cur_txns.len() >= self.chunk_size {
                let executed_outputs = AptosVM::execute_block_no_limit(
                    cur_txns
                        .iter()
                        .map(|txn| SignatureVerifiedTransaction::from(txn.clone()))
                        .collect::<Vec<_>>()
                        .as_slice(),
                    &arc_db.state_view_at_version(chunk_start_version.checked_sub(1))?,
                )?;
                // verify results
                let fail_txns = Self::verify_execution_results(
                    chunk_start_version,
                    &expected_txn_infos,
                    &expected_events,
                    &expected_writesets,
                    &executed_outputs,
                )?;
                // collect failed transactions
                failed_txns.extend(fail_txns);

                // empty for the new chunk
                chunk_start_version = start + (idx as u64) + 1;
                cur_txns.clear();
                expected_txn_infos.clear();
                expected_events.clear();
                expected_writesets.clear();
                info!(
                    version = start + idx as u64,
                    accumulative_tps = ((idx as f64) / replay_start.elapsed().as_secs_f64()) as u64,
                    "Transactions verified."
                );
            }
        }
        // Replay the remaining txns
        let executed_outputs = AptosVM::execute_block_no_limit(
            cur_txns
                .iter()
                .map(|txn| SignatureVerifiedTransaction::from(txn.clone()))
                .collect::<Vec<_>>()
                .as_slice(),
            &arc_db.state_view_at_version(chunk_start_version.checked_sub(1))?,
        )?;
        let fail_txns = Self::verify_execution_results(
            chunk_start_version,
            &expected_txn_infos,
            &expected_events,
            &expected_writesets,
            &executed_outputs,
        )?;
        info!(
            version = start + limit,
            accumulative_tps = ((limit as f64) / replay_start.elapsed().as_secs_f64()) as u64,
            "Transactions verified."
        );

        failed_txns.extend(fail_txns);

        if !failed_txns.is_empty() {
            debug!("Failed transactions: {:?}", failed_txns);
            process::exit(2);
        }
        Ok(())
    }

    fn verify_execution_results(
        start_version: Version,
        expected_txn_infos: &Vec<TransactionInfo>,
        expected_epoch_events: &Vec<Vec<ContractEvent>>,
        expected_epoch_writesets: &Vec<WriteSet>,
        executed_outputs: &Vec<TransactionOutput>,
    ) -> Result<Vec<Error>> {
        let mut failed_txns = Vec::new();
        let mut version = start_version;
        for (idx, (expected_txn_info, expected_events, expected_writeset, executed_output)) in
            multizip((
                expected_txn_infos,
                expected_epoch_events,
                expected_epoch_writesets,
                executed_outputs,
            ))
            .enumerate()
        {
            version = start_version + idx as Version;
            if let Err(err) = executed_output.ensure_match_transaction_info(
                version,
                expected_txn_info,
                Some(expected_writeset),
                Some(expected_events),
            ) {
                failed_txns.push(err);
            }
        }

        if (version + 1 - start_version) as usize != expected_txn_infos.len() {
            bail!(
                "processed transaction count {} is not equal to expected transaction count {}",
                version + 1 - start_version,
                expected_txn_infos.len()
            );
        }
        Ok(failed_txns)
    }

    fn get_start_and_limit(
        aptos_db: &Arc<dyn DbReader>,
        start_version: Version,
        end_version: Version,
    ) -> Result<(Version, u64)> {
        let start_version = std::cmp::max(
            aptos_db
                .get_first_txn_version()?
                .ok_or(AptosDbError::NotFound(
                    "First txn version is None".to_string(),
                ))?,
            start_version,
        );

        let end_version = std::cmp::min(
            aptos_db
                .get_synced_version()?
                .ok_or(AptosDbError::NotFound("Synced version is None".to_string()))?,
            end_version,
        );
        assert!(
            start_version <= end_version,
            "start_version {} must be less than or equal to end_version{}",
            start_version,
            end_version
        );
        let limit = end_version - start_version;
        Ok((start_version, limit))
    }
}
