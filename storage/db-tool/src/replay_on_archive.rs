// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use anyhow::{bail, Error, Ok, Result};
use aptos_backup_cli::utils::{ReplayConcurrencyLevelOpt, RocksdbOpt};
use aptos_block_executor::txn_provider::default::DefaultTxnProvider;
use aptos_config::config::{
    StorageDirPaths, BUFFERED_STATE_TARGET_ITEMS, DEFAULT_MAX_NUM_NODES_PER_LRU_CACHE_SHARD,
    NO_OP_STORAGE_PRUNER_CONFIG,
};
use aptos_db::{backup::backup_handler::BackupHandler, AptosDB};
use aptos_logger::prelude::*;
use aptos_storage_interface::{
    state_store::state_view::db_state_view::DbStateViewAtVersion, AptosDbError, DbReader,
};
use aptos_types::{
    contract_event::ContractEvent,
    transaction::{
        signature_verified_transaction::SignatureVerifiedTransaction, AuxiliaryInfo,
        PersistedAuxiliaryInfo, Transaction, TransactionInfo, Version,
    },
    write_set::WriteSet,
};
use aptos_vm::{aptos_vm::AptosVMBlockExecutor, AptosVM, VMBlockExecutor};
use clap::Parser;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use std::{
    panic,
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
            error!("{} failed transactions", all_errors.len());
            for e in all_errors {
                error!("Failed: {}", e);
            }
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
        // Open in write mode to create any new DBs necessary.
        {
            if let Err(e) = panic::catch_unwind(|| {
                AptosDB::open(
                    StorageDirPaths::from_path(config.db_dir.as_path()),
                    false,
                    NO_OP_STORAGE_PRUNER_CONFIG,
                    config.rocksdb_opt.clone().into(),
                    false,
                    BUFFERED_STATE_TARGET_ITEMS,
                    DEFAULT_MAX_NUM_NODES_PER_LRU_CACHE_SHARD,
                    None,
                )
            }) {
                warn!("Unable to open AptosDB in write mode: {:?}", e);
            };
        }

        let aptos_db = AptosDB::open(
            StorageDirPaths::from_path(config.db_dir.as_path()),
            false,
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
            limit = limit,
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
        if self.limit == 0 {
            info!("Nothing to verify.");
            return Ok(vec![]);
        }

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
            .collect::<Vec<_>>();
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
        let mut cur_persisted_aux_info = Vec::new();
        let mut expected_events = Vec::new();
        let mut expected_writesets = Vec::new();
        let mut expected_txn_infos = Vec::new();
        let mut chunk_start_version = start;
        for item in txn_iter {
            // timeout check
            if let Some(duration) = self.timeout_secs {
                if self.replay_stat.get_elapsed_secs() >= duration {
                    bail!(
                        "Verify timeout: {}s elapsed. Deadline: {}s. Failed txns count: {}",
                        self.replay_stat.get_elapsed_secs(),
                        duration,
                        total_failed_txns.len(),
                    );
                }
            }

            let (
                input_txn,
                persisted_aux_info,
                expected_txn_info,
                expected_event,
                expected_writeset,
            ) = item?;
            let is_epoch_ending = expected_event.iter().any(ContractEvent::is_new_epoch_event);
            cur_txns.push(input_txn);
            cur_persisted_aux_info.push(persisted_aux_info);
            expected_txn_infos.push(expected_txn_info);
            expected_events.push(expected_event);
            expected_writesets.push(expected_writeset);
            if is_epoch_ending || cur_txns.len() >= self.chunk_size {
                let cnt = cur_txns.len();
                while !cur_txns.is_empty() {
                    // verify results
                    let failed_txn_opt = self.execute_and_verify(
                        &mut chunk_start_version,
                        &mut cur_txns,
                        &mut cur_persisted_aux_info,
                        &mut expected_txn_infos,
                        &mut expected_events,
                        &mut expected_writesets,
                    )?;
                    // collect failed transactions
                    total_failed_txns.extend(failed_txn_opt);
                }
                self.replay_stat.update_cnt(cnt as u64);
                self.replay_stat.print_tps();
            }
        }
        // verify results
        let fail_txns = self.execute_and_verify(
            &mut chunk_start_version,
            &mut cur_txns,
            &mut cur_persisted_aux_info,
            &mut expected_txn_infos,
            &mut expected_events,
            &mut expected_writesets,
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
        let db_start = aptos_db
            .get_first_txn_version()?
            .ok_or(AptosDbError::NotFound(
                "First txn version is None".to_string(),
            ))?;
        let start = std::cmp::max(db_start, start_version);

        let db_end = aptos_db
            .get_synced_version()?
            .ok_or(AptosDbError::NotFound("Synced version is None".to_string()))?;
        let end = std::cmp::min(end_version, db_end);

        let limit = if start <= end {
            end - start + 1
        } else {
            warn!(
                start = start_version,
                db_start = db_start,
                end = end_version,
                db_end = db_end,
                "No transactions to verify in requested range."
            );
            0
        };

        Ok((start_version, limit))
    }

    fn execute_and_verify(
        &self,
        current_version: &mut Version,
        cur_txns: &mut Vec<Transaction>,
        cur_persisted_aux_info: &mut Vec<PersistedAuxiliaryInfo>,
        expected_txn_infos: &mut Vec<TransactionInfo>,
        expected_events: &mut Vec<Vec<ContractEvent>>,
        expected_writesets: &mut Vec<WriteSet>,
    ) -> Result<Option<Error>> {
        if cur_txns.is_empty() {
            return Ok(None);
        }
        let txns = cur_txns
            .iter()
            .map(|txn| SignatureVerifiedTransaction::from(txn.clone()))
            .collect::<Vec<_>>();
        let txns_provider = DefaultTxnProvider::new(
            txns,
            cur_persisted_aux_info
                .iter()
                .map(|info| AuxiliaryInfo::new(*info, None))
                .collect(),
        );
        let executed_outputs = AptosVMBlockExecutor::new().execute_block_no_limit(
            &txns_provider,
            &self
                .arc_db
                .state_view_at_version(current_version.checked_sub(1))?,
        )?;
        assert_eq!(executed_outputs.len(), cur_txns.len());

        for idx in 0..cur_txns.len() {
            let version = *current_version;
            *current_version += 1;

            if let Err(err) = executed_outputs[idx].ensure_match_transaction_info(
                version,
                &expected_txn_infos[idx],
                Some(&expected_writesets[idx]),
                Some(&expected_events[idx]),
            ) {
                cur_txns.drain(0..idx + 1);
                cur_persisted_aux_info.drain(0..idx + 1);
                expected_txn_infos.drain(0..idx + 1);
                expected_events.drain(0..idx + 1);
                expected_writesets.drain(0..idx + 1);

                return Ok(Some(err));
            }
        }

        cur_txns.clear();
        cur_persisted_aux_info.clear();
        expected_txn_infos.clear();
        expected_events.clear();
        expected_writesets.clear();

        Ok(None)
    }
}
