// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use anyhow::{bail, Error, Ok, Result};
use aptos_backup_cli::utils::{ReplayConcurrencyLevelOpt, RocksdbOpt};
use aptos_config::config::{
    StorageDirPaths, BUFFERED_STATE_TARGET_ITEMS, DEFAULT_MAX_NUM_NODES_PER_LRU_CACHE_SHARD,
    NO_OP_STORAGE_PRUNER_CONFIG,
};
use aptos_db::{backup::backup_handler::BackupHandler, AptosDB};
use aptos_logger::{error, info};
use aptos_storage_interface::{state_view::DbStateViewAtVersion, AptosDbError, DbReader};
use aptos_types::{
    contract_event::ContractEvent,
    transaction::{
        signature_verified_transaction::SignatureVerifiedTransaction, Transaction, TransactionInfo,
        Version,
    },
    write_set::WriteSet,
};
use aptos_vm::{aptos_vm::AptosVMBlockExecutor, AptosVM, VMBlockExecutor};
use clap::Parser;
use itertools::multizip;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use std::{
    path::PathBuf,
    process,
    sync::{atomic::AtomicU64, Arc},
    time::Instant,
};
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
        default_value = "500",
        help = "The number of transactions to be replayed in a chunk"
    )]
    pub chunk_size: usize,

    #[clap(long, default_value = "1", help = "The number of concurrent replays")]
    pub concurrent_replay: usize,

    #[clap(
        long,
        help = "The maximum time in seconds to wait for each transaction replay"
    )]
    pub timeout_secs: Option<u64>,
}

impl Opt {
    pub async fn run(self) -> Result<()> {
        let verifier = Verifier::new(&self)?;
        let all_errors = verifier.run()?;
        if !all_errors.is_empty() {
            error!("All failed transactions: {:?}", all_errors);
            process::exit(2);
        }
        Ok(())
    }
}
struct ReplayTps {
    timer: Instant,
    txn_cnt: AtomicU64,
}

impl ReplayTps {
    pub fn new() -> Self {
        Self {
            timer: Instant::now(),
            txn_cnt: AtomicU64::new(0),
        }
    }

    pub fn update_cnt(&self, cnt: u64) {
        self.txn_cnt
            .fetch_add(cnt, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn print_tps(&self) {
        let elapsed = self.timer.elapsed().as_secs_f64();
        let cnt = self.txn_cnt.load(std::sync::atomic::Ordering::Relaxed);
        let tps = (cnt as f64) / elapsed;
        info!(
            "Replayed {} transactions in {} seconds, TPS: {}",
            cnt, elapsed, tps
        );
    }

    pub fn get_elapsed_secs(&self) -> u64 {
        self.timer.elapsed().as_secs()
    }
}

struct Verifier {
    backup_handler: BackupHandler,
    arc_db: Arc<dyn DbReader>,
    start: Version,
    limit: u64,
    replay_concurrency_level: usize,
    chunk_size: usize,
    concurrent_replay: usize,
    replay_stat: ReplayTps,
    timeout_secs: Option<u64>,
}

impl Verifier {
    pub fn new(config: &Opt) -> Result<Self> {
        let aptos_db = AptosDB::open(
            StorageDirPaths::from_path(config.db_dir.as_path()),
            true,
            NO_OP_STORAGE_PRUNER_CONFIG,
            config.rocksdb_opt.clone().into(),
            false,
            BUFFERED_STATE_TARGET_ITEMS,
            DEFAULT_MAX_NUM_NODES_PER_LRU_CACHE_SHARD,
            None,
        )?;

        let backup_handler = aptos_db.get_backup_handler();
        let arc_db = Arc::new(aptos_db) as Arc<dyn DbReader>;

        // calculate a valid start and limit
        let (start, limit) =
            Self::get_start_and_limit(&arc_db, config.start_version, config.end_version)?;
        info!(
            start_version = start,
            end_version = start + limit,
            "Replaying transactions."
        );
        Ok(Self {
            backup_handler,
            arc_db,
            start,
            limit,
            replay_concurrency_level: config.replay_concurrency_level.get(),
            chunk_size: config.chunk_size,
            concurrent_replay: config.concurrent_replay,
            replay_stat: ReplayTps::new(),
            timeout_secs: config.timeout_secs,
        })
    }

    // Split the replay to multiple reply tasks running in parallel
    pub fn run(self) -> Result<Vec<Error>> {
        AptosVM::set_concurrency_level_once(self.replay_concurrency_level);
        let task_size = self.limit / self.concurrent_replay as u64;
        let ranges: Vec<(u64, u64)> = (0..self.concurrent_replay)
            .map(|i| {
                let chunk_start = self.start + (i as u64) * task_size;
                let chunk_limit = if i == self.concurrent_replay - 1 {
                    self.start + self.limit - chunk_start
                } else {
                    task_size
                };
                (chunk_start, chunk_limit)
            })
            .collect();

        // Process each range in parallel using `par_iter`
        let res = ranges
            .par_iter()
            .map(|(start, limit)| self.verify(*start, *limit))
            .collect::<Vec<Result<Vec<Error>>>>();
        let mut all_failed_txns = Vec::new();
        for iter in res.into_iter() {
            all_failed_txns.extend(iter?);
        }
        Ok(all_failed_txns)
    }

    // Execute the verify one valide range
    pub fn verify(&self, start: Version, limit: u64) -> Result<Vec<Error>> {
        let mut total_failed_txns = Vec::new();
        let txn_iter = self
            .backup_handler
            .get_transaction_iter(start, limit as usize)?;
        let mut cur_txns = Vec::new();
        let mut expected_events = Vec::new();
        let mut expected_writesets = Vec::new();
        let mut expected_txn_infos = Vec::new();
        let mut chunk_start_version = start;
        for (idx, item) in txn_iter.enumerate() {
            let (input_txn, expected_txn_info, expected_event, expected_writeset) = item?;
            let is_epoch_ending = expected_event.iter().any(ContractEvent::is_new_epoch_event);
            cur_txns.push(input_txn);
            expected_txn_infos.push(expected_txn_info);
            expected_events.push(expected_event);
            expected_writesets.push(expected_writeset);
            if is_epoch_ending || cur_txns.len() >= self.chunk_size {
                // verify results
                let fail_txns = self.execute_and_verify(
                    chunk_start_version,
                    &cur_txns,
                    &expected_txn_infos,
                    &expected_events,
                    &expected_writesets,
                )?;
                // collect failed transactions
                total_failed_txns.extend(fail_txns);
                self.replay_stat.update_cnt(cur_txns.len() as u64);
                self.replay_stat.print_tps();

                if let Some(duration) = self.timeout_secs {
                    if self.replay_stat.get_elapsed_secs() >= duration {
                        return Ok(total_failed_txns);
                    }
                }

                // empty for the new chunk
                chunk_start_version = start + (idx as u64) + 1;
                cur_txns.clear();
                expected_txn_infos.clear();
                expected_events.clear();
                expected_writesets.clear();
            }
        }
        // verify results
        let fail_txns = self.execute_and_verify(
            chunk_start_version,
            &cur_txns,
            &expected_txn_infos,
            &expected_events,
            &expected_writesets,
        )?;
        total_failed_txns.extend(fail_txns);
        Ok(total_failed_txns)
    }

    /// utility functions
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

    fn execute_and_verify(
        &self,
        start_version: Version,
        cur_txns: &[Transaction],
        expected_txn_infos: &Vec<TransactionInfo>,
        expected_epoch_events: &Vec<Vec<ContractEvent>>,
        expected_epoch_writesets: &Vec<WriteSet>,
    ) -> Result<Vec<Error>> {
        if cur_txns.is_empty() {
            return Ok(Vec::new());
        }
        let executed_outputs = AptosVMBlockExecutor::new().execute_block_no_limit(
            cur_txns
                .iter()
                .map(|txn| SignatureVerifiedTransaction::from(txn.clone()))
                .collect::<Vec<_>>()
                .as_slice(),
            &self
                .arc_db
                .state_view_at_version(start_version.checked_sub(1))?,
        )?;

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
}
