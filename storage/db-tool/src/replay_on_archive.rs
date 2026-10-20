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
    MAX_REQUEST_LIMIT,
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
    ops::Range,
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
        help = "The first transaction version required to be replayed and verified, extended to the start of its block"
    )]
    start_version: Version,

    #[clap(
        long,
        help = "The last transaction version required to be replayed and verified, extended to the end of its block"
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
        help = "The number of transactions to be replayed in a chunk, extended to the next block end"
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
        if let Some((collector, _, _)) = &function_usage {
            anyhow::ensure!(
                self.start_version <= self.end_version,
                "invalid framework usage range"
            );
            // The replay is extended to block boundaries, while the report keeps the requested
            // range, so that adjacent shards still line up.
            let replayed = verifier.start..verifier.start + verifier.limit;
            anyhow::ensure!(
                replayed.contains(&self.start_version) && replayed.contains(&self.end_version),
                "archive database does not contain the complete requested framework usage range"
            );
            collector.set_ledger_timestamps(
                verifier.arc_db.get_block_timestamp(self.start_version)?,
                verifier.arc_db.get_block_timestamp(self.end_version)?,
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
            Self::get_start_and_limit(arc_db.as_ref(), config.start_version, config.end_version)?;
        set_layout_caches(true);
        set_paranoid_type_checks(config.paranoid_type_checks);
        // Paranoid checks are done async if enabled.
        set_async_runtime_checks(config.paranoid_type_checks);
        info!(
            requested_start_version = config.start_version,
            requested_end_version = config.end_version,
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
        let chunks = split_into_chunks(
            self.arc_db.as_ref(),
            self.start..self.start + self.limit,
            self.chunk_size as u64,
        )?;
        info!(num_chunks = chunks.len(), "Split the replay into chunks.");
        let res: Vec<_> = thread_pool.install(|| {
            chunks
                .into_par_iter()
                .map(|chunk| self.verify(chunk.start, chunk.end - chunk.start))
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
        let mut txn_iter = self
            .backup_handler
            .get_transaction_iter(start, limit as usize)?
            .peekable();
        let mut cur_txns = Vec::with_capacity(limit as usize);
        let mut cur_persisted_aux_info = Vec::with_capacity(limit as usize);
        let mut expected_events = Vec::with_capacity(limit as usize);
        let mut expected_writesets = Vec::with_capacity(limit as usize);
        let mut expected_txn_infos = Vec::with_capacity(limit as usize);
        let mut chunk_start_version = start;
        let executor = AptosVMBlockExecutor::new();
        while let Some(item) = txn_iter.next() {
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
            // The chunk is already sized to end at a block end, and cutting it by size could split
            // a block.
            let is_chunk_end = txn_iter.peek().is_none();
            if is_epoch_ending || is_chunk_end {
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
        Ok(total_failed_txns)
    }

    /// utility functions
    fn get_start_and_limit(
        aptos_db: &dyn DbReader,
        start_version: Version,
        end_version: Version,
    ) -> Result<(Version, u64)> {
        let db_start = aptos_db
            .get_first_txn_version()?
            .ok_or(AptosDbError::NotFound(
                "First txn version is None".to_string(),
            ))?;
        let db_end = aptos_db
            .get_synced_version()?
            .ok_or(AptosDbError::NotFound("Synced version is None".to_string()))?;

        match align_to_blocks(aptos_db, start_version, end_version, db_start..db_end + 1)? {
            Some((start, end)) => Ok((start, end - start + 1)),
            None => {
                warn!(
                    start = start_version,
                    db_start = db_start,
                    end = end_version,
                    db_end = db_end,
                    "No whole blocks to verify in requested range."
                );
                Ok((start_version, 0))
            },
        }
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
        anyhow::ensure!(
            expected_txn_infos
                .last()
                .is_some_and(TransactionInfo::has_state_checkpoint_hash),
            "Version {} is not a block end.",
            *current_version + cur_txns.len() as Version - 1,
        );
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
                // The rest of the block was executed on top of the mismatch, so resume from the
                // next block.
                let num_skipped = expected_txn_infos[idx..]
                    .iter()
                    .position(TransactionInfo::has_state_checkpoint_hash)
                    .expect("Txns to execute end at a block end.");
                if num_skipped > 0 {
                    warn!(
                        version = version,
                        num_skipped = num_skipped,
                        "Skipping the rest of the block after a mismatch."
                    );
                }
                *current_version += num_skipped as Version;
                let block_end_idx = idx + num_skipped;
                cur_txns.drain(..=block_end_idx);
                cur_persisted_aux_info.drain(..=block_end_idx);
                expected_txn_infos.drain(..=block_end_idx);
                expected_events.drain(..=block_end_idx);
                expected_writesets.drain(..=block_end_idx);

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

/// Clamps `[start, end]` to the whole blocks in `db_range`, then extends it to block boundaries.
/// Returns `None` if no whole block is left.
fn align_to_blocks(
    db: &dyn DbReader,
    start: Version,
    end: Version,
    db_range: Range<Version>,
) -> Result<Option<(Version, Version)>> {
    if db_range.is_empty() {
        return Ok(None);
    }
    // The DB can start and end mid-block.
    let first_block_start = if starts_block(db, db_range.start)? {
        db_range.start
    } else if let Some(partial_block_end) = first_block_end_in(db, db_range.clone())? {
        partial_block_end + 1
    } else {
        return Ok(None);
    };
    let Some(last_block_end) = last_block_end_in(db, db_range)? else {
        return Ok(None);
    };

    let start = start.max(first_block_start);
    let end = end.min(last_block_end);
    if start > end {
        return Ok(None);
    }
    let block_start = last_block_end_in(db, first_block_start..start)?
        .map_or(first_block_start, |prev_block_end| prev_block_end + 1);
    let block_end = first_block_end_in(db, end..last_block_end)?.unwrap_or(last_block_end);
    Ok(Some((block_start, block_end)))
}

/// Splits `range` of whole blocks into chunks of whole blocks, each extended from `chunk_size` txns
/// to the next block end.
fn split_into_chunks(
    db: &dyn DbReader,
    range: Range<Version>,
    chunk_size: u64,
) -> Result<Vec<Range<Version>>> {
    anyhow::ensure!(chunk_size > 0, "Chunk size must be positive.");
    let mut chunks = Vec::new();
    let mut start = range.start;
    while start < range.end {
        let last = start.saturating_add(chunk_size).min(range.end) - 1;
        let Some(block_end) = first_block_end_in(db, last..range.end)? else {
            bail!("Version {} is not a block end.", range.end - 1);
        };
        chunks.push(start..block_end + 1);
        start = block_end + 1;
    }
    Ok(chunks)
}

/// Whether the txn at `version` starts a block, judging by the txn itself, for when the previous
/// `TransactionInfo` is not available.
fn starts_block(db: &dyn DbReader, version: Version) -> Result<bool> {
    let txn = db
        .get_transaction_iterator(version, 1)?
        .next()
        .transpose()?
        .with_context(|| format!("Transaction missing at version {version}."))?;
    Ok(txn.is_block_start() || matches!(txn, Transaction::GenesisTransaction(_)))
}

fn first_block_end_in(db: &dyn DbReader, range: Range<Version>) -> Result<Option<Version>> {
    for window_start in range.clone().step_by(MAX_REQUEST_LIMIT as usize) {
        let window = window_start
            ..window_start
                .saturating_add(MAX_REQUEST_LIMIT)
                .min(range.end);
        if let Some(block_end) = block_ends_in(db, window)?.next() {
            return Ok(Some(block_end?));
        }
    }
    Ok(None)
}

fn last_block_end_in(db: &dyn DbReader, range: Range<Version>) -> Result<Option<Version>> {
    let mut window_end = range.end;
    while window_end > range.start {
        let window_start = window_end
            .saturating_sub(MAX_REQUEST_LIMIT)
            .max(range.start);
        let block_ends =
            block_ends_in(db, window_start..window_end)?.collect::<Result<Vec<_>>>()?;
        if let Some(block_end) = block_ends.last() {
            return Ok(Some(*block_end));
        }
        window_end = window_start;
    }
    Ok(None)
}

/// Lazily yields the versions in `range` that end a block, i.e. whose `TransactionInfo` has a state
/// checkpoint hash. `range` can't be longer than `MAX_REQUEST_LIMIT`.
fn block_ends_in(
    db: &dyn DbReader,
    range: Range<Version>,
) -> Result<impl Iterator<Item = Result<Version>> + '_> {
    let mut txn_infos = db.get_transaction_info_iterator(range.start, range.end - range.start)?;
    Ok(range
        .map(move |version| {
            // The DB iterator ends silently where txn infos are missing.
            let txn_info = txn_infos
                .next()
                .with_context(|| format!("TransactionInfo missing at version {version}."))??;
            Ok(txn_info.has_state_checkpoint_hash().then_some(version))
        })
        .filter_map(Result::transpose))
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
    use aptos_crypto::HashValue;
    use aptos_executor_test_helpers::integration_test_impl::test_execution_with_storage_impl_inner;
    use aptos_storage_interface::Result as DbResult;
    use aptos_types::{
        block_metadata::BlockMetadata,
        transaction::{ChangeSet, ExecutionStatus, WriteSetPayload},
    };
    use move_core_types::account_address::AccountAddress;

    /// Txns from `first_version` on, ending a block wherever `block_ends` says so.
    struct MockDb {
        first_version: Version,
        block_ends: Vec<bool>,
        has_partial_first_block: bool,
    }

    impl MockDb {
        fn new(first_version: Version, block_sizes: &[usize]) -> Self {
            let block_ends = block_sizes
                .iter()
                .flat_map(|&size| (1..=size).map(move |i| i == size))
                .collect();
            Self {
                first_version,
                block_ends,
                has_partial_first_block: false,
            }
        }

        /// Starts the DB at the last `num_txns` txns of a block.
        fn with_partial_first_block(mut self, num_txns: usize) -> Self {
            self.first_version -= num_txns as Version;
            self.block_ends
                .splice(0..0, (1..=num_txns).map(|i| i == num_txns));
            self.has_partial_first_block = true;
            self
        }

        fn without_last_block_end(mut self) -> Self {
            *self.block_ends.last_mut().unwrap() = false;
            self
        }

        fn db_range(&self) -> Range<Version> {
            self.first_version..self.first_version + self.block_ends.len() as Version
        }

        fn slice(&self, start_version: Version, limit: u64) -> &[bool] {
            let start = (start_version - self.first_version) as usize;
            let end = (start + limit as usize).min(self.block_ends.len());
            &self.block_ends[start..end]
        }
    }

    impl DbReader for MockDb {
        fn get_transaction_iterator(
            &self,
            start_version: Version,
            limit: u64,
        ) -> DbResult<Box<dyn Iterator<Item = DbResult<Transaction>> + '_>> {
            let first_version = self.first_version;
            let starts_with_block = !self.has_partial_first_block;
            let block_ends = self.block_ends.clone();
            DbResult::Ok(Box::new((start_version..start_version + limit).map(
                move |version| {
                    let idx = (version - first_version) as usize;
                    let is_block_start = match idx {
                        0 => starts_with_block,
                        _ => block_ends[idx - 1],
                    };
                    DbResult::Ok(
                        if version == 0 {
                            Transaction::GenesisTransaction(WriteSetPayload::Direct(
                                ChangeSet::new(WriteSet::default(), vec![]),
                            ))
                        } else if is_block_start {
                            Transaction::BlockMetadata(BlockMetadata::new(
                                HashValue::zero(),
                                0,
                                0,
                                AccountAddress::ZERO,
                                vec![],
                                vec![],
                                0,
                            ))
                        } else {
                            Transaction::StateCheckpoint(HashValue::zero())
                        },
                    )
                },
            )))
        }

        fn get_transaction_info_iterator(
            &self,
            start_version: Version,
            limit: u64,
        ) -> DbResult<Box<dyn Iterator<Item = DbResult<TransactionInfo>> + '_>> {
            assert!(limit <= MAX_REQUEST_LIMIT);
            DbResult::Ok(Box::new(self.slice(start_version, limit).iter().map(
                |&is_block_end| {
                    DbResult::Ok(TransactionInfo::new_placeholder(
                        0,
                        is_block_end.then(HashValue::zero),
                        ExecutionStatus::Success,
                    ))
                },
            )))
        }
    }

    fn align(db: &MockDb, start: Version, end: Version) -> Option<(Version, Version)> {
        align_to_blocks(db, start, end, db.db_range()).unwrap()
    }

    #[test]
    fn align_extends_to_block_boundaries() {
        // Blocks: [100, 102], [103, 106], [107, 107], [108, 111].
        let db = MockDb::new(100, &[3, 4, 1, 4]);
        assert_eq!(align(&db, 103, 106), Some((103, 106)));
        assert_eq!(align(&db, 104, 105), Some((103, 106)));
        assert_eq!(align(&db, 104, 108), Some((103, 111)));
        assert_eq!(align(&db, 107, 107), Some((107, 107)));
        assert_eq!(align(&db, 101, 101), Some((100, 102)));
        assert_eq!(align(&db, 100, 111), Some((100, 111)));
    }

    #[test]
    fn align_shrinks_to_blocks_in_db() {
        // The DB starts and ends in the middle of blocks: [100, 101], [102, 104], [105, 108],
        // [109, 110].
        let db = MockDb::new(102, &[3, 4, 2])
            .with_partial_first_block(2)
            .without_last_block_end();
        assert_eq!(db.db_range(), 100..111);
        assert_eq!(align(&db, 100, 110), Some((102, 108)));
        assert_eq!(align(&db, 101, 103), Some((102, 104)));
        assert_eq!(align(&db, 106, 110), Some((105, 108)));
        // Only partial blocks.
        assert_eq!(align(&db, 100, 101), None);
        assert_eq!(align(&db, 109, 110), None);
    }

    #[test]
    fn align_trusts_block_start_at_db_start() {
        // Blocks: [100, 101], [102, 104].
        let db = MockDb::new(100, &[2, 3]);
        assert_eq!(align(&db, 100, 100), Some((100, 101)));
        // Genesis.
        let db = MockDb::new(0, &[1, 3]);
        assert_eq!(align(&db, 0, 2), Some((0, 3)));
    }

    #[test]
    fn align_scans_across_request_limits() {
        let n = MAX_REQUEST_LIMIT as usize;
        // Blocks: [10, 10], [11, 11 + 2n], [12 + 2n, 12 + 2n].
        let db = MockDb::new(10, &[1, 2 * n + 1, 1]);
        let big_block = (11, 11 + 2 * n as Version);
        assert_eq!(
            align(&db, 12 + n as Version, 13 + n as Version),
            Some(big_block)
        );
        assert_eq!(align(&db, 11, 11), Some(big_block));
        assert_eq!(align(&db, big_block.1, big_block.1), Some(big_block));
    }

    #[test]
    fn chunks_end_at_block_ends() {
        // Blocks: [100, 102], [103, 106], [107, 107], [108, 111].
        let db = MockDb::new(100, &[3, 4, 1, 4]);
        let split = |chunk_size| split_into_chunks(&db, db.db_range(), chunk_size).unwrap();
        assert_eq!(split(1), vec![100..103, 103..107, 107..108, 108..112]);
        assert_eq!(split(4), vec![100..107, 107..112]);
        assert_eq!(split(5), vec![100..107, 107..112]);
        assert_eq!(split(8), vec![100..108, 108..112]);
        assert_eq!(split(100), vec![100..112]);
        assert_eq!(split_into_chunks(&db, 103..108, 2).unwrap(), vec![
            103..107,
            107..108
        ]);
    }

    #[test]
    fn chunks_cover_big_blocks() {
        let n = MAX_REQUEST_LIMIT;
        // Blocks: [0, 0], [1, 2n + 1], [2n + 2, 2n + 2].
        let db = MockDb::new(0, &[1, 2 * n as usize + 1, 1]);
        assert_eq!(split_into_chunks(&db, db.db_range(), 1).unwrap(), vec![
            0..1,
            1..2 * n + 2,
            2 * n + 2..2 * n + 3
        ]);
        assert_eq!(split_into_chunks(&db, db.db_range(), 3).unwrap(), vec![
            0..2 * n + 2,
            2 * n + 2..2 * n + 3
        ]);
    }

    #[test]
    fn chunks_require_range_ending_at_block_end() {
        let db = MockDb::new(100, &[3, 4]).without_last_block_end();
        assert!(split_into_chunks(&db, db.db_range(), 2).is_err());
    }

    #[test]
    fn replay_extends_to_blocks() {
        let db_dir = aptos_temppath::TempPath::new();
        // Blocks: [0, 0], [1, 11], [12, 13], [14, 29], where 11 and 13 end epochs. Dropped to
        // release the DB lock.
        drop(test_execution_with_storage_impl_inner(false, db_dir.path()));

        for (start_version, end_version, chunk_size, expected_start, expected_limit) in [
            (5, 20, 4, 1, 29),
            (12, 12, 1, 12, 2),
            (14, 29, 1, 14, 16),
            (0, 100, 500, 0, 30),
        ] {
            let opt = Opt::try_parse_from([
                "replay-on-archive",
                "--start-version",
                &start_version.to_string(),
                "--end-version",
                &end_version.to_string(),
                "--target-db-dir",
                db_dir.path().to_str().unwrap(),
                "--chunk-size",
                &chunk_size.to_string(),
                "--concurrent-replay",
                "2",
                "--replay-concurrency-level",
                "2",
            ])
            .unwrap();
            let verifier = Verifier::new(&opt, None).unwrap();
            assert_eq!(
                (verifier.start, verifier.limit),
                (expected_start, expected_limit)
            );
            let errors = verifier.run().unwrap();
            assert!(errors.is_empty(), "{errors:?}");
        }
    }

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
