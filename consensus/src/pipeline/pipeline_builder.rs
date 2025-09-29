// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    block_preparer::BlockPreparer,
    block_storage::tracing::{observe_block, BlockStage},
    counters::{self, update_counters_for_block, update_counters_for_compute_result},
    monitor,
    network::NetworkSender,
    payload_manager::TPayloadManager,
    txn_notifier::TxnNotifier,
    IntGaugeGuard,
};
use anyhow::anyhow;
use aptos_consensus_notifications::ConsensusNotificationSender;
use aptos_consensus_types::{
    block::Block,
    common::Round,
    pipeline::commit_vote::CommitVote,
    pipelined_block::{
        CommitLedgerResult, CommitVoteResult, ExecuteResult, LedgerUpdateResult,
        NotifyStateSyncResult, PipelineFutures, PipelineInputRx, PipelineInputTx, PipelinedBlock,
        PostCommitResult, PostLedgerUpdateResult, PreCommitResult, PrepareResult, RandResult,
        TaskError, TaskFuture, TaskResult,
    },
    quorum_cert::QuorumCert,
    wrapped_ledger_info::WrappedLedgerInfo,
};
use aptos_crypto::HashValue;
use aptos_executor_types::{state_compute_result::StateComputeResult, BlockExecutorTrait};
use aptos_experimental_runtimes::thread_manager::optimal_min_len;
use aptos_infallible::Mutex;
use aptos_logger::{error, info, trace, warn};
use aptos_storage_interface::state_store::state_view::cached_state_view::CachedStateView;
use aptos_types::{
    account_config::randomness_event::RANDOMNESS_GENERATED_EVENT_MOVE_TYPE_TAG,
    block_executor::config::BlockExecutorConfigFromOnchain,
    ledger_info::{LedgerInfo, LedgerInfoWithSignatures},
    on_chain_config::OnChainConsensusConfig,
    randomness::Randomness,
    state_store::StateViewId,
    transaction::{
        signature_verified_transaction::{SignatureVerifiedTransaction, TransactionProvider},
        AuxiliaryInfo, EphemeralAuxiliaryInfo, PersistedAuxiliaryInfo, SignedTransaction,
        Transaction, TransactionExecutableRef,
    },
    validator_signer::ValidatorSigner,
    vm::module_metadata::get_randomness_annotation_for_entry_function,
};
use aptos_vm_validator::vm_validator::ValidationState;
use futures::FutureExt;
use move_core_types::account_address::AccountAddress;
use move_vm_runtime::ModuleStorage;
use once_cell::sync::Lazy;
use rayon::prelude::*;
use std::{
    future::Future,
    ops::Deref,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::{select, sync::oneshot, task::AbortHandle};

static SIG_VERIFY_POOL: Lazy<Arc<rayon::ThreadPool>> = Lazy::new(|| {
    Arc::new(
        rayon::ThreadPoolBuilder::new()
            .num_threads(16)
            .thread_name(|index| format!("signature-checker-{}", index))
            .build()
            .expect("Failed to create signature verification thread pool"),
    )
});

/// Status to help synchornize the pipeline and sync_manager
/// It is used to track the round of the block that could be pre-committed and sync manager decides
/// whether to enter state sync or not and pause pre-commit during state sync to avoid race condition
/// that state sync starts but pre-commit runs over the target.
pub struct PreCommitStatus {
    round: Round,
    paused: bool,
    is_enabled: bool,
}

impl PreCommitStatus {
    pub fn new(round: Round, is_enabled: bool) -> Self {
        Self {
            round,
            paused: false,
            is_enabled,
        }
    }

    pub fn is_active(&self) -> bool {
        self.is_enabled && !self.paused
    }

    pub fn pause(&mut self) {
        self.paused = true;
    }

    pub fn resume(&mut self) {
        self.paused = false;
    }

    pub fn round(&self) -> Round {
        self.round
    }

    pub fn update_round(&mut self, round: Round) {
        self.round = std::cmp::max(self.round, round);
    }
}

/// The pipeline builder is responsible for constructing the pipeline structure for a block.
/// Each phase is represented as a shared future, takes in other futures as pre-condition.
/// Future returns a TaskResult<T>, which error can be either a user error or task error (e.g. cancellation).
///
/// Currently, the critical path is the following, more details can be found in the comments of each phase.
/// prepare -> execute -> ledger update -> pre-commit -> commit ledger
///    rand ->
///                         order proof ->
///                                      commit proof ->
#[derive(Clone)]
pub struct PipelineBuilder {
    block_preparer: Arc<BlockPreparer>,
    executor: Arc<dyn BlockExecutorTrait>,
    validators: Arc<[AccountAddress]>,
    block_executor_onchain_config: BlockExecutorConfigFromOnchain,
    is_randomness_enabled: bool,
    signer: Arc<ValidatorSigner>,
    state_sync_notifier: Arc<dyn ConsensusNotificationSender>,
    payload_manager: Arc<dyn TPayloadManager>,
    txn_notifier: Arc<dyn TxnNotifier>,
    pre_commit_status: Arc<Mutex<PreCommitStatus>>,
    order_vote_enabled: bool,
    persisted_auxiliary_info_version: u8,
    rand_check_enabled: bool,
    module_cache: Arc<Mutex<Option<ValidationState<CachedStateView>>>>,
    network_sender: Arc<NetworkSender>,
}

fn spawn_shared_fut<
    T: Send + Clone + 'static,
    F: Future<Output = TaskResult<T>> + Send + 'static,
>(
    f: F,
    abort_handles: Option<&mut Vec<AbortHandle>>,
) -> TaskFuture<T> {
    let join_handle = tokio::spawn(f);
    if let Some(handles) = abort_handles {
        handles.push(join_handle.abort_handle());
    }
    async move {
        match join_handle.await {
            Ok(Ok(res)) => Ok(res),
            Ok(e @ Err(TaskError::PropagatedError(_))) => e,
            Ok(Err(e @ TaskError::InternalError(_) | e @ TaskError::JoinError(_))) => {
                Err(TaskError::PropagatedError(Box::new(e)))
            },
            Err(e) => Err(TaskError::JoinError(Arc::new(e))),
        }
    }
    .boxed()
    .shared()
}

fn spawn_ready_fut<T: Send + Clone + 'static>(f: T) -> TaskFuture<T> {
    async move { Ok(f) }.boxed().shared()
}

async fn wait_and_log_error<T, F: Future<Output = TaskResult<T>>>(f: F, msg: String) {
    if let Err(TaskError::InternalError(e)) = f.await {
        warn!("{} failed: {}", msg, e);
    }
}

struct Tracker {
    name: &'static str,
    block_id: HashValue,
    epoch: u64,
    round: Round,
    created_at: Instant,
    started_at: Option<Instant>,
    running_guard: Option<IntGaugeGuard>,
}

impl Tracker {
    fn start_waiting(name: &'static str, block: &Block) -> Self {
        Self {
            name,
            block_id: block.id(),
            epoch: block.epoch(),
            round: block.round(),
            created_at: Instant::now(),
            started_at: None,
            running_guard: None,
        }
    }

    fn start_working(&mut self) {
        self.started_at = Some(Instant::now());
        self.running_guard = Some(IntGaugeGuard::new(
            counters::OP_COUNTERS.gauge(&format!("{}_running", self.name)),
        ));
        self.log_start();
    }

    fn log_start(&self) {
        trace!(
            "[Pipeline] Block {} {} {} enters {}",
            self.block_id,
            self.epoch,
            self.round,
            self.name
        );
    }

    fn log_end(&self) {
        let Some(started_at) = self.started_at else {
            return;
        };
        let wait_time = started_at.duration_since(self.created_at);
        let work_time = Instant::now().duration_since(started_at);
        counters::PIPELINE_TRACING
            .with_label_values(&[self.name, "wait_time"])
            .observe(wait_time.as_secs_f64());
        counters::PIPELINE_TRACING
            .with_label_values(&[self.name, "work_time"])
            .observe(work_time.as_secs_f64());
        trace!(
            "[Pipeline] Block {} {} {} finishes {}, waits {}ms, takes {}ms",
            self.block_id,
            self.epoch,
            self.round,
            self.name,
            wait_time.as_millis(),
            work_time.as_millis()
        );
    }
}

impl Drop for Tracker {
    fn drop(&mut self) {
        self.log_end();
    }
}

impl PipelineBuilder {
    pub fn new(
        block_preparer: Arc<BlockPreparer>,
        executor: Arc<dyn BlockExecutorTrait>,
        validators: Arc<[AccountAddress]>,
        block_executor_onchain_config: BlockExecutorConfigFromOnchain,
        is_randomness_enabled: bool,
        signer: Arc<ValidatorSigner>,
        state_sync_notifier: Arc<dyn ConsensusNotificationSender>,
        payload_manager: Arc<dyn TPayloadManager>,
        txn_notifier: Arc<dyn TxnNotifier>,
        enable_pre_commit: bool,
        consensus_onchain_config: &OnChainConsensusConfig,
        persisted_auxiliary_info_version: u8,
        network_sender: Arc<NetworkSender>,
    ) -> Self {
        let module_cache = Arc::new(Mutex::new(None));
        Self {
            block_preparer,
            executor,
            validators,
            block_executor_onchain_config,
            is_randomness_enabled,
            signer,
            state_sync_notifier,
            payload_manager,
            txn_notifier,
            pre_commit_status: Arc::new(Mutex::new(PreCommitStatus::new(0, enable_pre_commit))),
            order_vote_enabled: consensus_onchain_config.order_vote_enabled(),
            persisted_auxiliary_info_version,
            rand_check_enabled: consensus_onchain_config.rand_check_enabled(),
            module_cache,
            network_sender,
        }
    }

    pub fn pre_commit_status(&self) -> Arc<Mutex<PreCommitStatus>> {
        self.pre_commit_status.clone()
    }

    fn channel(abort_handles: &mut Vec<AbortHandle>) -> (PipelineInputTx, PipelineInputRx) {
        let (qc_tx, qc_rx) = oneshot::channel();
        let (rand_tx, rand_rx) = oneshot::channel();
        let (order_vote_tx, order_vote_rx) = oneshot::channel();
        let (order_proof_tx, order_proof_fut) = oneshot::channel();
        let (commit_proof_tx, commit_proof_fut) = oneshot::channel();
        let order_proof_fut = spawn_shared_fut(
            async move {
                order_proof_fut
                    .await
                    .map_err(|_| TaskError::from(anyhow!("order proof tx cancelled")))
            },
            Some(abort_handles),
        );
        let commit_proof_fut = spawn_shared_fut(
            async move {
                commit_proof_fut
                    .await
                    .map_err(|_| TaskError::from(anyhow!("commit proof tx cancelled")))
            },
            Some(abort_handles),
        );
        (
            PipelineInputTx {
                qc_tx: Some(qc_tx),
                rand_tx: Some(rand_tx),
                order_vote_tx: Some(order_vote_tx),
                order_proof_tx: Some(order_proof_tx),
                commit_proof_tx: Some(commit_proof_tx),
            },
            PipelineInputRx {
                qc_rx,
                rand_rx,
                order_vote_rx,
                order_proof_fut,
                commit_proof_fut,
            },
        )
    }

    pub fn build_root(
        &self,
        compute_result: StateComputeResult,
        commit_proof: LedgerInfoWithSignatures,
    ) -> PipelineFutures {
        let prepare_fut = spawn_ready_fut((Arc::new(vec![]), None));
        let rand_check_fut = spawn_ready_fut((None, false));
        let execute_fut = spawn_ready_fut(Duration::from_millis(0));
        let ledger_update_fut =
            spawn_ready_fut((compute_result.clone(), Duration::from_millis(0), None));
        let commit_vote_fut = spawn_ready_fut(CommitVote::new_with_signature(
            self.signer.author(),
            commit_proof.ledger_info().clone(),
            self.signer
                .sign(commit_proof.ledger_info())
                .expect("Signing should succeed"),
        ));
        let pre_commit_fut = spawn_ready_fut(compute_result);
        let commit_ledger_fut = spawn_ready_fut(Some(commit_proof));
        let post_ledger_update_fut = spawn_ready_fut(());
        let notify_state_sync_fut = spawn_ready_fut(());
        let post_commit_fut = spawn_ready_fut(());
        PipelineFutures {
            prepare_fut,
            rand_check_fut,
            execute_fut,
            ledger_update_fut,
            post_ledger_update_fut,
            commit_vote_fut,
            pre_commit_fut,
            notify_state_sync_fut,
            commit_ledger_fut,
            post_commit_fut,
        }
    }

    pub fn build(
        &self,
        pipelined_block: &PipelinedBlock,
        parent_futs: PipelineFutures,
        block_store_callback: Box<
            dyn FnOnce(WrappedLedgerInfo, LedgerInfoWithSignatures) + Send + Sync,
        >,
    ) {
        let (futs, tx, abort_handles) = self.build_internal(
            parent_futs,
            Arc::new(pipelined_block.block().clone()),
            block_store_callback,
        );
        pipelined_block.set_pipeline_futs(futs);
        pipelined_block.set_pipeline_tx(tx);
        pipelined_block.set_pipeline_abort_handles(abort_handles);
    }

    fn build_internal(
        &self,
        parent: PipelineFutures,
        block: Arc<Block>,
        block_store_callback: Box<
            dyn FnOnce(WrappedLedgerInfo, LedgerInfoWithSignatures) + Send + Sync,
        >,
    ) -> (PipelineFutures, PipelineInputTx, Vec<AbortHandle>) {
        let mut abort_handles = vec![];
        let (tx, rx) = Self::channel(&mut abort_handles);
        let PipelineInputRx {
            qc_rx,
            rand_rx,
            order_vote_rx,
            order_proof_fut,
            commit_proof_fut,
        } = rx;

        let prepare_fut = spawn_shared_fut(
            Self::prepare(self.block_preparer.clone(), block.clone(), qc_rx),
            Some(&mut abort_handles),
        );
        let rand_check_fut = spawn_shared_fut(
            Self::rand_check(
                prepare_fut.clone(),
                parent.execute_fut.clone(),
                rand_rx,
                self.executor.clone(),
                block.clone(),
                self.is_randomness_enabled,
                self.rand_check_enabled,
                self.module_cache.clone(),
            ),
            Some(&mut abort_handles),
        );
        let execute_fut = spawn_shared_fut(
            Self::execute(
                prepare_fut.clone(),
                parent.execute_fut.clone(),
                rand_check_fut.clone(),
                self.executor.clone(),
                block.clone(),
                self.validators.clone(),
                self.block_executor_onchain_config.clone(),
                self.persisted_auxiliary_info_version,
            ),
            None,
        );
        let ledger_update_fut = spawn_shared_fut(
            Self::ledger_update(
                rand_check_fut.clone(),
                execute_fut.clone(),
                parent.ledger_update_fut.clone(),
                self.executor.clone(),
                block.clone(),
            ),
            None,
        );
        let commit_vote_fut = spawn_shared_fut(
            Self::sign_and_broadcast_commit_vote(
                ledger_update_fut.clone(),
                order_vote_rx,
                order_proof_fut.clone(),
                commit_proof_fut.clone(),
                self.signer.clone(),
                block.clone(),
                self.order_vote_enabled,
                self.network_sender.clone(),
            ),
            Some(&mut abort_handles),
        );
        let pre_commit_fut = spawn_shared_fut(
            Self::pre_commit(
                ledger_update_fut.clone(),
                parent.pre_commit_fut.clone(),
                order_proof_fut.clone(),
                commit_proof_fut.clone(),
                self.executor.clone(),
                block.clone(),
                self.pre_commit_status(),
            ),
            None,
        );
        let commit_ledger_fut = spawn_shared_fut(
            Self::commit_ledger(
                pre_commit_fut.clone(),
                commit_proof_fut,
                parent.commit_ledger_fut.clone(),
                self.executor.clone(),
                block.clone(),
            ),
            None,
        );

        let post_ledger_update_fut = spawn_shared_fut(
            Self::post_ledger_update(
                prepare_fut.clone(),
                ledger_update_fut.clone(),
                self.txn_notifier.clone(),
                block.clone(),
            ),
            Some(&mut abort_handles),
        );
        let notify_state_sync_fut = spawn_shared_fut(
            Self::notify_state_sync(
                pre_commit_fut.clone(),
                commit_ledger_fut.clone(),
                parent.notify_state_sync_fut.clone(),
                self.state_sync_notifier.clone(),
                block.clone(),
            ),
            None,
        );
        let post_commit_fut = spawn_shared_fut(
            Self::post_commit_ledger(
                pre_commit_fut.clone(),
                order_proof_fut,
                commit_ledger_fut.clone(),
                notify_state_sync_fut.clone(),
                parent.post_commit_fut.clone(),
                self.payload_manager.clone(),
                block_store_callback,
                block.clone(),
            ),
            None,
        );
        let all_fut = PipelineFutures {
            prepare_fut,
            rand_check_fut,
            execute_fut,
            ledger_update_fut,
            post_ledger_update_fut,
            commit_vote_fut,
            pre_commit_fut,
            notify_state_sync_fut,
            commit_ledger_fut,
            post_commit_fut,
        };
        tokio::spawn(Self::monitor(
            block.epoch(),
            block.round(),
            block.id(),
            all_fut.clone(),
        ));
        (all_fut, tx, abort_handles)
    }

    /// Precondition: Block is inserted into block tree (all ancestors are available)
    /// What it does: Wait for all data becomes available and verify transaction signatures
    async fn prepare(
        preparer: Arc<BlockPreparer>,
        block: Arc<Block>,
        qc_rx: oneshot::Receiver<Arc<QuorumCert>>,
    ) -> TaskResult<PrepareResult> {
        let mut tracker = Tracker::start_waiting("prepare", &block);
        tracker.start_working();

        let qc_rx = async {
            match qc_rx.await {
                Ok(qc) => Some(qc),
                Err(_) => {
                    warn!("[BlockPreparer] qc tx cancelled for block {}", block.id());
                    None
                },
            }
        }
        .shared();
        // the loop can only be abort by the caller
        let (input_txns, block_gas_limit) = loop {
            match preparer.prepare_block(&block, qc_rx.clone()).await {
                Ok(input_txns) => break input_txns,
                Err(e) => {
                    warn!(
                        "[BlockPreparer] failed to prepare block {}, retrying: {}",
                        block.id(),
                        e
                    );
                    tokio::time::sleep(Duration::from_millis(100)).await;
                },
            }
        };
        let sig_verification_start = Instant::now();
        let sig_verified_txns: Vec<SignatureVerifiedTransaction> = SIG_VERIFY_POOL.install(|| {
            let num_txns = input_txns.len();
            input_txns
                .into_par_iter()
                .with_min_len(optimal_min_len(num_txns, 32))
                .map(|t| Transaction::UserTransaction(t).into())
                .collect::<Vec<_>>()
        });
        counters::PREPARE_BLOCK_SIG_VERIFICATION_TIME
            .observe_duration(sig_verification_start.elapsed());
        Ok((Arc::new(sig_verified_txns), block_gas_limit))
    }

    /// Precondition: 1. prepare finishes, 2. parent block's execution phase finishes
    /// What it does: decides if the block requires a randomness seed and return the value
    async fn rand_check(
        prepare_fut: TaskFuture<PrepareResult>,
        parent_block_execute_fut: TaskFuture<ExecuteResult>,
        rand_rx: oneshot::Receiver<Option<Randomness>>,
        executor: Arc<dyn BlockExecutorTrait>,
        block: Arc<Block>,
        is_randomness_enabled: bool,
        rand_check_enabled: bool,
        module_cache: Arc<Mutex<Option<ValidationState<CachedStateView>>>>,
    ) -> TaskResult<RandResult> {
        let mut tracker = Tracker::start_waiting("rand_check", &block);
        parent_block_execute_fut.await?;
        let (user_txns, _) = prepare_fut.await?;

        tracker.start_working();
        if !is_randomness_enabled {
            return Ok((None, false));
        }
        let grand_parent_id = block.quorum_cert().parent_block().id();
        let parent_state_view = executor
            .state_view(block.parent_id())
            .map_err(anyhow::Error::from)?;

        let mut has_randomness = false;
        // scope to drop the lock, compiler seems not able to figure out manual drop with async point
        {
            let mut cache_guard = module_cache.lock();
            if let Some(cache_mut) = cache_guard.as_mut() {
                // flush the cache if the execution state view is not linear
                // in case of speculative executing a forked block
                let previous_state_view = cache_mut.state_view_id();
                let expected_state_view = StateViewId::BlockExecution {
                    block_id: grand_parent_id,
                };
                if previous_state_view == expected_state_view {
                    cache_mut.reset_state_view(parent_state_view);
                } else {
                    counters::RAND_BLOCK
                        .with_label_values(&["reset_cache"])
                        .inc();
                    cache_mut.reset_all(parent_state_view);
                }
            } else {
                *cache_guard = Some(ValidationState::new(parent_state_view));
            }
            let cache_ref = cache_guard.as_mut().expect("just set");

            for txn in user_txns.iter() {
                if let Some(txn) = txn.borrow_into_inner().try_as_signed_user_txn() {
                    if let Ok(TransactionExecutableRef::EntryFunction(entry_fn)) =
                        txn.executable_ref()
                    {
                        // use the deserialized API to avoid cloning the metadata
                        // should migrate once we move metadata into the extension and avoid cloning
                        if let Ok(Some(module)) = cache_ref.unmetered_get_deserialized_module(
                            entry_fn.module().address(),
                            entry_fn.module().name(),
                        ) {
                            if get_randomness_annotation_for_entry_function(
                                entry_fn,
                                &module.metadata,
                            )
                            .is_some()
                            {
                                has_randomness = true;
                                break;
                            }
                        }
                    }
                }
            }
        }
        let label = if has_randomness {
            "has_rand"
        } else {
            "no_rand"
        };
        counters::RAND_BLOCK.with_label_values(&[label]).inc();
        if has_randomness {
            info!(
                "[Pipeline] Block {} {} {} has randomness txn",
                block.id(),
                block.epoch(),
                block.round()
            );
        }
        drop(tracker);
        // if rand check is enabled and no txn requires randomness, we skip waiting for randomness
        let mut tracker = Tracker::start_waiting("rand_gen", &block);
        tracker.start_working();
        let maybe_rand = if rand_check_enabled && !has_randomness {
            None
        } else {
            rand_rx
                .await
                .map_err(|_| anyhow!("randomness tx cancelled"))?
        };
        Ok((Some(maybe_rand), has_randomness))
    }

    /// Precondition: 1. prepare finishes, 2. parent block's phase finishes 3. randomness is available
    /// What it does: Execute all transactions in block executor
    async fn execute(
        prepare_fut: TaskFuture<PrepareResult>,
        parent_block_execute_fut: TaskFuture<ExecuteResult>,
        rand_check: TaskFuture<RandResult>,
        executor: Arc<dyn BlockExecutorTrait>,
        block: Arc<Block>,
        validator: Arc<[AccountAddress]>,
        onchain_execution_config: BlockExecutorConfigFromOnchain,
        persisted_auxiliary_info_version: u8,
    ) -> TaskResult<ExecuteResult> {
        let mut tracker = Tracker::start_waiting("execute", &block);
        parent_block_execute_fut.await?;
        let (user_txns, block_gas_limit) = prepare_fut.await?;
        let onchain_execution_config =
            onchain_execution_config.with_block_gas_limit_override(block_gas_limit);

        let (rand_result, _has_randomness) = rand_check.await?;

        tracker.start_working();
        // if randomness is disabled, the metadata skips DKG and triggers immediate reconfiguration
        let metadata_txn = if let Some(maybe_rand) = rand_result {
            block.new_metadata_with_randomness(&validator, maybe_rand)
        } else {
            block.new_block_metadata(&validator).into()
        };
        let txns = [
            vec![SignatureVerifiedTransaction::from(Transaction::from(
                metadata_txn,
            ))],
            block
                .validator_txns()
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .map(Transaction::ValidatorTransaction)
                .map(SignatureVerifiedTransaction::from)
                .collect(),
            user_txns.as_ref().clone(),
        ]
        .concat();
        let proposer_index = block
            .author()
            .and_then(|proposer| validator.iter().position(|&v| v == proposer));

        let auxiliary_info: Vec<_> = txns
            .iter()
            .enumerate()
            .map(|(txn_index, txn)| {
                let persisted_auxiliary_info = match persisted_auxiliary_info_version {
                    0 => PersistedAuxiliaryInfo::None,
                    1 => PersistedAuxiliaryInfo::V1 {
                        transaction_index: txn_index as u32,
                    },
                    _ => unimplemented!("Unsupported persisted auxiliary info version"),
                };

                let ephemeral_auxiliary_info = txn
                    .borrow_into_inner()
                    .try_as_signed_user_txn()
                    .and_then(|_| {
                        proposer_index.map(|index| EphemeralAuxiliaryInfo {
                            proposer_index: index as u64,
                        })
                    });

                AuxiliaryInfo::new(persisted_auxiliary_info, ephemeral_auxiliary_info)
            })
            .collect();

        let start = Instant::now();
        tokio::task::spawn_blocking(move || {
            executor
                .execute_and_update_state(
                    (block.id(), txns, auxiliary_info).into(),
                    block.parent_id(),
                    onchain_execution_config,
                )
                .map_err(anyhow::Error::from)
        })
        .await
        .expect("spawn blocking failed")?;
        Ok(start.elapsed())
    }

    /// Precondition: 1. execute finishes, 2. parent block's phase finishes
    /// What it does: Generate state compute result from the execution, it's split from execution for more parallelism
    /// It carries block timestamp from epoch-ending block to all suffix block
    async fn ledger_update(
        rand_check: TaskFuture<RandResult>,
        execute_fut: TaskFuture<ExecuteResult>,
        parent_block_ledger_update_fut: TaskFuture<LedgerUpdateResult>,
        executor: Arc<dyn BlockExecutorTrait>,
        block: Arc<Block>,
    ) -> TaskResult<LedgerUpdateResult> {
        let mut tracker = Tracker::start_waiting("ledger_update", &block);
        let (_, _, prev_epoch_end_timestamp) = parent_block_ledger_update_fut.await?;
        let execution_time = execute_fut.await?;

        tracker.start_working();
        let block_clone = block.clone();
        let result = tokio::task::spawn_blocking(move || {
            executor
                .ledger_update(block_clone.id(), block_clone.parent_id())
                .map_err(anyhow::Error::from)
        })
        .await
        .expect("spawn blocking failed")?;
        let timestamp = block.timestamp_usecs();
        observe_block(timestamp, BlockStage::EXECUTED);
        let epoch_end_timestamp =
            if result.has_reconfiguration() && !result.compute_status_for_input_txns().is_empty() {
                Some(timestamp)
            } else {
                prev_epoch_end_timestamp
            };
        // check for randomness consistency
        let (_, has_randomness) = rand_check.await?;
        if !has_randomness {
            let mut label = "consistent";
            for event in result.execution_output.subscribable_events.get(None) {
                if event.type_tag() == RANDOMNESS_GENERATED_EVENT_MOVE_TYPE_TAG.deref() {
                    error!(
                            "[Pipeline] Block {} {} {} generated randomness event without has_randomness being true!",
                            block.id(),
                            block.epoch(),
                            block.round()
                        );
                    label = "inconsistent";
                    break;
                }
            }
            counters::RAND_BLOCK.with_label_values(&[label]).inc();
        }
        Ok((result, execution_time, epoch_end_timestamp))
    }

    /// Precondition: ledger update finishes
    /// What it does: For now this is mainly to notify mempool about failed transactions
    /// This is off critical path
    async fn post_ledger_update(
        prepare_fut: TaskFuture<PrepareResult>,
        ledger_update_fut: TaskFuture<LedgerUpdateResult>,
        mempool_notifier: Arc<dyn TxnNotifier>,
        block: Arc<Block>,
    ) -> TaskResult<PostLedgerUpdateResult> {
        let mut tracker = Tracker::start_waiting("post_ledger_update", &block);
        let (user_txns, _) = prepare_fut.await?;
        let (compute_result, _, _) = ledger_update_fut.await?;

        tracker.start_working();
        let compute_status = compute_result.compute_status_for_input_txns();
        // the length of compute_status is user_txns.len() + num_vtxns + 1 due to having blockmetadata
        if user_txns.len() >= compute_status.len() {
            // reconfiguration suffix blocks don't have any transactions
            // otherwise, this is an error
            if !compute_status.is_empty() {
                error!(
                        "Expected compute_status length and actual compute_status length mismatch! user_txns len: {}, compute_status len: {}, has_reconfiguration: {}",
                        user_txns.len(),
                        compute_status.len(),
                        compute_result.has_reconfiguration(),
                    );
            }
        } else {
            let user_txn_status = &compute_status[compute_status.len() - user_txns.len()..];
            // todo: avoid clone
            let txns: Vec<SignedTransaction> = user_txns
                .iter()
                .flat_map(|txn| txn.get_transaction().map(|t| t.try_as_signed_user_txn()))
                .flatten()
                .cloned()
                .collect();

            // notify mempool about failed transaction
            if let Err(e) = mempool_notifier
                .notify_failed_txn(&txns, user_txn_status)
                .await
            {
                error!(
                    error = ?e, "Failed to notify mempool of rejected txns",
                );
            }
        }
        Ok(())
    }

    /// Precondition: 1. ledger update finishes, 2. order vote or order proof or commit proof is received
    /// What it does: Sign the commit vote with execution result and broadcast, it needs to update the timestamp for reconfig suffix blocks
    async fn sign_and_broadcast_commit_vote(
        ledger_update_fut: TaskFuture<LedgerUpdateResult>,
        order_vote_rx: oneshot::Receiver<()>,
        order_proof_fut: TaskFuture<WrappedLedgerInfo>,
        commit_proof_fut: TaskFuture<LedgerInfoWithSignatures>,
        signer: Arc<ValidatorSigner>,
        block: Arc<Block>,
        order_vote_enabled: bool,
        network_sender: Arc<NetworkSender>,
    ) -> TaskResult<CommitVoteResult> {
        let mut tracker = Tracker::start_waiting("sign_commit_vote", &block);
        let (compute_result, _, epoch_end_timestamp) = ledger_update_fut.await?;
        let mut consensus_data_hash = select! {
            Ok(_) = order_vote_rx => {
                HashValue::zero()
            }
            Ok(li) = order_proof_fut => {
                li.ledger_info().ledger_info().consensus_data_hash()
            }
            Ok(li) = commit_proof_fut => {
                li.ledger_info().consensus_data_hash()
            }
            else => {
                return Err(anyhow!("all receivers dropped"))?;
            }
        };
        if order_vote_enabled {
            consensus_data_hash = HashValue::zero();
        }
        tracker.start_working();

        let mut block_info = block.gen_block_info(
            compute_result.root_hash(),
            compute_result.last_version_or_0(),
            compute_result.epoch_state().clone(),
        );
        if let Some(timestamp) = epoch_end_timestamp {
            info!(
                "[Pipeline] update block timestamp from {} to epoch end timestamp {}",
                block_info.timestamp_usecs(),
                timestamp
            );
            block_info.change_timestamp(timestamp);
        }
        let ledger_info = LedgerInfo::new(block_info, consensus_data_hash);
        info!("[Pipeline] Signed ledger info {ledger_info}");
        let signature = signer.sign(&ledger_info).expect("Signing should succeed");
        let commit_vote = CommitVote::new_with_signature(signer.author(), ledger_info, signature);
        network_sender
            .broadcast_commit_vote(commit_vote.clone())
            .await;
        Ok(commit_vote)
    }

    /// Precondition: 1. ledger update finishes, 2. parent block's phase finishes 2. order proof is received
    /// What it does: pre-write result to storage even commit proof is not yet available
    /// For epoch ending block, wait until commit proof is available
    async fn pre_commit(
        ledger_update_fut: TaskFuture<LedgerUpdateResult>,
        parent_block_pre_commit_fut: TaskFuture<PreCommitResult>,
        order_proof_fut: TaskFuture<WrappedLedgerInfo>,
        commit_proof_fut: TaskFuture<LedgerInfoWithSignatures>,
        executor: Arc<dyn BlockExecutorTrait>,
        block: Arc<Block>,
        pre_commit_status: Arc<Mutex<PreCommitStatus>>,
    ) -> TaskResult<PreCommitResult> {
        let mut tracker = Tracker::start_waiting("pre_commit", &block);
        let (compute_result, _, _) = ledger_update_fut.await?;
        parent_block_pre_commit_fut.await?;

        order_proof_fut.await?;

        let wait_for_proof = {
            let mut status_guard = pre_commit_status.lock();
            let wait_for_proof = compute_result.has_reconfiguration() || !status_guard.is_active();
            // it's a bit ugly here, but we want to make the check and update atomic in the pre_commit case
            // to avoid race that check returns active, sync manager pauses pre_commit and round gets updated
            if !wait_for_proof {
                status_guard.update_round(block.round());
            }
            wait_for_proof
        };

        if wait_for_proof {
            commit_proof_fut.await?;
            pre_commit_status.lock().update_round(block.round());
        }

        tracker.start_working();
        tokio::task::spawn_blocking(move || {
            executor
                .pre_commit_block(block.id())
                .map_err(anyhow::Error::from)
        })
        .await
        .expect("spawn blocking failed")?;
        Ok(compute_result)
    }

    /// Precondition: 1. pre-commit finishes, 2. parent block's phase finishes 3. commit proof is available
    /// What it does: Commit the ledger info to storage, this makes the data visible for clients
    async fn commit_ledger(
        pre_commit_fut: TaskFuture<PreCommitResult>,
        commit_proof_fut: TaskFuture<LedgerInfoWithSignatures>,
        parent_block_commit_fut: TaskFuture<CommitLedgerResult>,
        executor: Arc<dyn BlockExecutorTrait>,
        block: Arc<Block>,
    ) -> TaskResult<CommitLedgerResult> {
        let mut tracker = Tracker::start_waiting("commit_ledger", &block);
        parent_block_commit_fut.await?;
        pre_commit_fut.await?;
        let ledger_info_with_sigs = commit_proof_fut.await?;

        // it's committed as prefix
        if ledger_info_with_sigs.commit_info().id() != block.id() {
            return Ok(None);
        }

        tracker.start_working();
        let ledger_info_with_sigs_clone = ledger_info_with_sigs.clone();
        tokio::task::spawn_blocking(move || {
            executor
                .commit_ledger(ledger_info_with_sigs_clone)
                .map_err(anyhow::Error::from)
        })
        .await
        .expect("spawn blocking failed")?;
        Ok(Some(ledger_info_with_sigs))
    }

    /// Precondition: 1. commit ledger finishes, 2. parent block's phase finishes 3. post pre commit finishes
    /// What it does: Update counters for the block, and notify block tree about the commit
    async fn post_commit_ledger(
        pre_commit_fut: TaskFuture<PreCommitResult>,
        order_proof_fut: TaskFuture<WrappedLedgerInfo>,
        commit_ledger_fut: TaskFuture<CommitLedgerResult>,
        notify_state_sync_fut: TaskFuture<NotifyStateSyncResult>,
        parent_post_commit: TaskFuture<PostCommitResult>,
        payload_manager: Arc<dyn TPayloadManager>,
        block_store_callback: Box<
            dyn FnOnce(WrappedLedgerInfo, LedgerInfoWithSignatures) + Send + Sync,
        >,
        block: Arc<Block>,
    ) -> TaskResult<PostCommitResult> {
        let mut tracker = Tracker::start_waiting("post_commit_ledger", &block);
        parent_post_commit.await?;
        let maybe_ledger_info_with_sigs = commit_ledger_fut.await?;
        let compute_result = pre_commit_fut.await?;
        notify_state_sync_fut.await?;

        tracker.start_working();
        update_counters_for_block(&block);
        update_counters_for_compute_result(&compute_result);

        let payload = block.payload().cloned();
        let timestamp = block.timestamp_usecs();
        let payload_vec = payload.into_iter().collect();
        payload_manager.notify_commit(timestamp, payload_vec);

        if let Some(ledger_info_with_sigs) = maybe_ledger_info_with_sigs {
            let order_proof = order_proof_fut.await?;
            block_store_callback(order_proof, ledger_info_with_sigs);
        }
        Ok(())
    }

    /// Precondition: 1. commit ledger finishes or fallback to state sync happens, 2. parent block's phase finishes
    /// What it does: Notify state synchronizer and payload manager about committed transactions
    /// This is off critical path
    async fn notify_state_sync(
        pre_commit_fut: TaskFuture<PreCommitResult>,
        commit_ledger_fut: TaskFuture<CommitLedgerResult>,
        parent_notify_state_sync_fut: TaskFuture<PostCommitResult>,
        state_sync_notifier: Arc<dyn ConsensusNotificationSender>,
        block: Arc<Block>,
    ) -> TaskResult<NotifyStateSyncResult> {
        let mut tracker = Tracker::start_waiting("notify_state_sync", &block);
        let compute_result = pre_commit_fut.await?;
        parent_notify_state_sync_fut.await?;
        // if commit ledger is aborted, it's typically an abort caused by reset to fall back to state sync
        // we want to finish notifying already pre-committed txns before go into state sync
        // so only return if there's internal error from commit ledger
        if let Err(e @ TaskError::InternalError(_)) = commit_ledger_fut.await {
            return Err(TaskError::PropagatedError(Box::new(e)));
        }

        tracker.start_working();
        let txns = compute_result.transactions_to_commit().to_vec();
        let subscribable_events = compute_result.subscribable_events().to_vec();
        if let Err(e) = monitor!(
            "notify_state_sync",
            state_sync_notifier
                .notify_new_commit(txns, subscribable_events)
                .await
        ) {
            error!(error = ?e, "Failed to notify state synchronizer");
        }

        Ok(())
    }

    async fn monitor(epoch: u64, round: Round, block_id: HashValue, all_futs: PipelineFutures) {
        let PipelineFutures {
            prepare_fut,
            rand_check_fut: _,
            execute_fut,
            ledger_update_fut,
            post_ledger_update_fut: _,
            commit_vote_fut: _,
            pre_commit_fut,
            notify_state_sync_fut: _,
            commit_ledger_fut,
            post_commit_fut: _,
        } = all_futs;
        wait_and_log_error(prepare_fut, format!("{epoch} {round} {block_id} prepare")).await;
        wait_and_log_error(execute_fut, format!("{epoch} {round} {block_id} execute")).await;
        wait_and_log_error(
            ledger_update_fut,
            format!("{epoch} {round} {block_id} ledger update"),
        )
        .await;
        wait_and_log_error(
            pre_commit_fut,
            format!("{epoch} {round} {block_id} pre commit"),
        )
        .await;
        wait_and_log_error(
            commit_ledger_fut,
            format!("{epoch} {round} {block_id} commit ledger"),
        )
        .await;
    }
}
