// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::framework_usage::FrameworkUsageCollector;
use anyhow::{bail, Context, Error, Ok, Result};
use aptos_backup_cli::utils::{ReplayConcurrencyLevelOpt, RocksdbOpt};
use aptos_block_executor::txn_provider::default::DefaultTxnProvider;
use aptos_config::config::{
    HotStateConfig, StorageDirPaths, BUFFERED_STATE_TARGET_ITEMS,
    DEFAULT_MAX_NUM_NODES_PER_LRU_CACHE_SHARD, NO_OP_STORAGE_PRUNER_CONFIG,
};
use aptos_db::{backup::backup_handler::BackupHandler, AptosDB};
use aptos_logger::prelude::*;
use aptos_storage_interface::{
    state_store::state_view::db_state_view::DbStateViewAtVersion, AptosDbError, DbReader,
};
use aptos_types::{
    block_executor::{
        config::BlockExecutorConfigFromOnchain,
        transaction_slice_metadata::TransactionSliceMetadata,
    },
    contract_event::ContractEvent,
    state_store::NUM_STATE_SHARDS,
    transaction::{
        signature_verified_transaction::SignatureVerifiedTransaction, AuxiliaryInfo, BlockOutput,
        PersistedAuxiliaryInfo, Transaction, TransactionInfo, Version,
    },
    write_set::WriteSet,
};
use aptos_vm::{
    aptos_vm::AptosVMBlockExecutor, function_usage::install_function_usage_sink, AptosVM,
    VMBlockExecutor,
};
use aptos_vm_environment::prod_configs::{
    set_async_runtime_checks, set_layout_caches, set_paranoid_type_checks,
};
use clap::Parser;
use rayon::{iter::ParallelIterator, prelude::IntoParallelIterator};
use std::{
    panic,
    path::{Path, PathBuf},
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

    #[clap(
        long,
        default_value_t = false,
        help = "Enable paranoid type checks in the Move VM"
    )]
    pub paranoid_type_checks: bool,
}

impl Opt {
    pub async fn run(self) -> Result<()> {
        self.run_impl(None).await
    }

    pub(crate) async fn run_with_function_usage(
        self,
        output: PathBuf,
        html_output: Option<PathBuf>,
    ) -> Result<()> {
        anyhow::ensure!(
            self.replay_concurrency_level.get() == 1,
            "framework usage replay requires --replay-concurrency-level 1"
        );
        let collector = Arc::new(FrameworkUsageCollector::new(
            self.start_version,
            self.end_version,
        ));
        let _sink_guard = install_function_usage_sink(collector.clone())?;
        self.run_impl(Some((collector, output, html_output))).await
    }

    async fn run_impl(
        self,
        function_usage: Option<(Arc<FrameworkUsageCollector>, PathBuf, Option<PathBuf>)>,
    ) -> Result<()> {
        let verifier = Verifier::new(
            &self,
            function_usage.as_ref().map(|(sink, _, _)| sink.clone()),
        )?;
        if function_usage.is_some() {
            let expected_limit = self
                .end_version
                .checked_sub(self.start_version)
                .and_then(|limit| limit.checked_add(1))
                .ok_or_else(|| anyhow::anyhow!("invalid or overflowing framework usage range"))?;
            anyhow::ensure!(
                verifier.start == self.start_version && verifier.limit == expected_limit,
                "archive database does not contain the complete requested framework usage range"
            );
        }
        if let Some((collector, _, _)) = &function_usage {
            let end = verifier
                .start
                .checked_add(verifier.limit - 1)
                .context("framework usage end version overflow")?;
            collector.set_ledger_timestamps(
                verifier.arc_db.get_block_timestamp(verifier.start)?,
                verifier.arc_db.get_block_timestamp(end)?,
            )?;
        }
        let all_errors = verifier.run()?;
        if !all_errors.is_empty() {
            error!("{} failed transactions", all_errors.len());
            for e in all_errors {
                error!("Failed: {}", e);
            }
            process::exit(2);
        }
        if let Some((collector, output, html_output)) = function_usage {
            collector.write_report(&output, html_output.as_deref())?;
            info!(output = ?output, html_output = ?html_output, "Wrote framework usage report.");
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

    pub fn get_txn_cnt(&self) -> u64 {
        self.txn_cnt.load(std::sync::atomic::Ordering::Relaxed)
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
    function_usage: Option<Arc<FrameworkUsageCollector>>,
}

impl Verifier {
    pub fn new(config: &Opt, function_usage: Option<Arc<FrameworkUsageCollector>>) -> Result<Self> {
        validate_archive_db_dir(&config.db_dir)?;

        // Replay-on-archive historically opens in write mode once to create any DBs introduced by
        // a newer binary. Framework usage is an analysis of an existing archive and must remain
        // read-only: archive snapshots are commonly mounted without write permission, and this
        // initialization otherwise emits caught shard-opening panics before the read-only open.
        if function_usage.is_none() {
            if let Err(e) = panic::catch_unwind(|| {
                AptosDB::open(
                    StorageDirPaths::from_path(config.db_dir.as_path()),
                    false,
                    NO_OP_STORAGE_PRUNER_CONFIG,
                    config.rocksdb_opt.clone().into(),
                    BUFFERED_STATE_TARGET_ITEMS,
                    DEFAULT_MAX_NUM_NODES_PER_LRU_CACHE_SHARD,
                    None,
                    HotStateConfig::default(),
                )
            }) {
                warn!("Unable to open AptosDB in write mode: {:?}", e);
            };
        }

        let aptos_db = AptosDB::open(
            StorageDirPaths::from_path(config.db_dir.as_path()),
            true,
            NO_OP_STORAGE_PRUNER_CONFIG,
            config.rocksdb_opt.clone().into(),
            BUFFERED_STATE_TARGET_ITEMS,
            DEFAULT_MAX_NUM_NODES_PER_LRU_CACHE_SHARD,
            None,
            HotStateConfig {
                delete_on_restart: false,
                ..Default::default()
            },
        )?;

        let backup_handler = aptos_db.get_backup_handler();
        let arc_db = Arc::new(aptos_db) as Arc<dyn DbReader>;

        // calculate a valid start and limit
        let (start, limit) =
            Self::get_start_and_limit(&arc_db, config.start_version, config.end_version)?;
        set_layout_caches(true);
        set_paranoid_type_checks(config.paranoid_type_checks);
        // Paranoid checks are done async if enabled.
        set_async_runtime_checks(config.paranoid_type_checks);
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
            function_usage,
        })
    }

    // Split the replay to multiple reply tasks running in parallel
    pub fn run(self) -> Result<Vec<Error>> {
        if self.limit == 0 {
            info!("Nothing to verify.");
            return Ok(vec![]);
        }

        AptosVM::set_concurrency_level_once(self.replay_concurrency_level);
        let thread_pool = rayon::ThreadPoolBuilder::new()
            .num_threads(self.concurrent_replay)
            .thread_name(|i| format!("replay-verify-{}", i))
            .build()?;
        let chunk_size = self.chunk_size as u64;
        let total_chunks = self.limit.div_ceil(chunk_size);
        let res: Vec<_> = thread_pool.install(|| {
            (0..total_chunks)
                .into_par_iter()
                .map(|i| {
                    let start = self.start + i * chunk_size;
                    let end = std::cmp::min(start + chunk_size - 1, self.start + self.limit - 1);
                    self.verify(start, end - start + 1)
                })
                .collect()
        });
        let mut all_failed_txns = Vec::new();
        for iter in res.into_iter() {
            all_failed_txns.extend(iter?);
        }
        anyhow::ensure!(
            self.replay_stat.get_txn_cnt() == self.limit,
            "replayed {} transactions but expected {}",
            self.replay_stat.get_txn_cnt(),
            self.limit
        );
        Ok(all_failed_txns)
    }

    // Execute the verify one valid range
    pub fn verify(&self, start: Version, limit: u64) -> Result<Vec<Error>> {
        let mut total_failed_txns = Vec::with_capacity(limit as usize);
        let txn_iter = self
            .backup_handler
            .get_transaction_iter(start, limit as usize)?;
        let mut cur_txns = Vec::with_capacity(limit as usize);
        let mut cur_persisted_aux_info = Vec::with_capacity(limit as usize);
        let mut expected_events = Vec::with_capacity(limit as usize);
        let mut expected_writesets = Vec::with_capacity(limit as usize);
        let mut expected_txn_infos = Vec::with_capacity(limit as usize);
        let mut chunk_start_version = start;
        let executor = AptosVMBlockExecutor::new();
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
                        &executor,
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
        let cnt = cur_txns.len();
        let fail_txns = self.execute_and_verify(
            &executor,
            &mut chunk_start_version,
            &mut cur_txns,
            &mut cur_persisted_aux_info,
            &mut expected_txn_infos,
            &mut expected_events,
            &mut expected_writesets,
        )?;
        total_failed_txns.extend(fail_txns);
        self.replay_stat.update_cnt(cnt as u64);
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

        Ok((start, limit))
    }

    fn execute_and_verify(
        &self,
        executor: &AptosVMBlockExecutor,
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
        let executed_outputs = executor
            .execute_block(
                &txns_provider,
                &self
                    .arc_db
                    .state_view_at_version(current_version.checked_sub(1))?,
                BlockExecutorConfigFromOnchain::new_no_block_limit(), // TODO(HotState): will need to incorporate some features.
                TransactionSliceMetadata::Chunk {
                    begin: *current_version,
                    end: *current_version + cur_txns.len() as u64,
                },
            )
            .map(BlockOutput::into_transaction_outputs_forced)?;
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

            if let (Some(collector), Transaction::UserTransaction(txn)) =
                (&self.function_usage, &cur_txns[idx])
            {
                collector.assign_version(txn.committed_hash(), version)?;
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

fn validate_archive_db_dir(db_dir: &Path) -> Result<()> {
    anyhow::ensure!(
        db_dir.is_dir(),
        "archive database directory {:?} does not exist; --target-db-dir must point to an existing Aptos archive DB (`/mnt/archive/db` is the mount used inside replay CI pods)",
        db_dir
    );

    let state_kv_dir = db_dir.join("state_kv_db");
    let missing_shards = (0..NUM_STATE_SHARDS)
        .filter(|shard| !state_kv_dir.join(format!("shard_{shard}")).is_dir())
        .collect::<Vec<_>>();
    anyhow::ensure!(
        missing_shards.is_empty(),
        "archive database {:?} is missing state KV shard directories {:?}; check that --target-db-dir points to the DB root and that the archive snapshot is complete",
        db_dir,
        missing_shards
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn archive_db_layout_validation_reports_missing_paths() {
        let temp_dir = aptos_temppath::TempPath::new();
        let missing_root = temp_dir.path().join("missing");
        let error = validate_archive_db_dir(&missing_root).unwrap_err();
        assert!(error.to_string().contains("does not exist"));

        temp_dir.create_as_dir().unwrap();
        let error = validate_archive_db_dir(temp_dir.path()).unwrap_err();
        assert!(error.to_string().contains("missing state KV shard"));

        for shard in 0..NUM_STATE_SHARDS {
            std::fs::create_dir_all(
                temp_dir
                    .path()
                    .join("state_kv_db")
                    .join(format!("shard_{shard}")),
            )
            .unwrap();
        }
        validate_archive_db_dir(temp_dir.path()).unwrap();
    }
}
