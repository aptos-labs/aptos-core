// Copyright © Aptos Foundation

use std::collections::HashMap;
use crate::{
    block_executor::BlockAptosVM,
    sharded_block_executor::{
        coordinator_client::CoordinatorClient,
        counters::{SHARDED_BLOCK_EXECUTION_BY_ROUNDS_SECONDS, SHARDED_BLOCK_EXECUTOR_TXN_COUNT},
        cross_shard_client::{CrossShardClient, CrossShardCommitReceiver, CrossShardCommitSender},
        cross_shard_state_view::CrossShardStateView,
        messages::CrossShardMsg,
        ExecutorShardCommand,
    },
};
use aptos_logger::{info, trace};
use aptos_state_view::StateView;
use aptos_types::{
    block_executor::partitioner::{
        ShardId, SubBlock, SubBlocksForShard, TransactionWithDependencies,
    },
    transaction::{analyzed_transaction::AnalyzedTransaction, TransactionOutput},
};
use aptos_vm_logging::disable_speculative_logging;
use futures::{channel::oneshot, executor::block_on};
use move_core_types::vm_status::VMStatus;
use std::sync::{Arc, Condvar, Mutex};
use aptos_block_executor::txn_provider::sharded::ShardedTxnProvider;
use aptos_types::state_store::state_key::StateKey;
use crate::counters::BLOCK_EXECUTOR_SIGNATURE_VERIFICATION_SECONDS;
use crate::sharded_block_executor::TxnProviderArgs;

pub struct ShardedExecutorService<S: StateView + Sync + Send + 'static> {
    shard_id: ShardId,
    num_shards: usize,
    executor_thread_pool: Arc<rayon::ThreadPool>,
    coordinator_client: Arc<dyn CoordinatorClient<S>>,
    cross_shard_client: Arc<dyn CrossShardClient>,
}

impl<S: StateView + Sync + Send + 'static> ShardedExecutorService<S> {
    pub fn new(
        shard_id: ShardId,
        num_shards: usize,
        num_threads: usize,
        coordinator_client: Arc<dyn CoordinatorClient<S>>,
        cross_shard_client: Arc<dyn CrossShardClient>,
    ) -> Self {
        let executor_thread_pool = Arc::new(
            rayon::ThreadPoolBuilder::new()
                .thread_name(move |index|format!("shard-{}-worker-{}", shard_id, index))
                // We need two extra threads for the cross-shard commit receiver and the thread
                // that is blocked on waiting for execute block to finish.
                .num_threads(num_threads)
                .build()
                .unwrap(),
        );
        Self {
            shard_id,
            num_shards,
            executor_thread_pool,
            coordinator_client,
            cross_shard_client,
        }
    }

    pub fn execute_transactions_with_dependencies(
        shard_id: Option<ShardId>, // None means execution on global shard
        executor_thread_pool: Arc<rayon::ThreadPool>,
        txn_provider_args: TxnProviderArgs,
        cross_shard_client: Arc<dyn CrossShardClient>,
        cross_shard_commit_sender: Option<CrossShardCommitSender>,
        round: usize,
        state_view: &S,
        concurrency_level: usize,
        maybe_block_gas_limit: Option<u64>,
    ) -> Result<Vec<TransactionOutput>, VMStatus> {
        let TxnProviderArgs {
            block_id,
            num_shards,
            rx,
            senders,
            txns,
            local_idxs_by_global,
            global_idxs,
            remote_dependencies,
            following_shard_sets,
        } = txn_provider_args;

        let txns = txns
            .into_iter()
            .map(|txn| txn.into_txn())
            .collect();

        let signature_verification_timer =
            BLOCK_EXECUTOR_SIGNATURE_VERIFICATION_SECONDS.start_timer();
        let pre_processed_txns = executor_thread_pool.install(||{
            BlockAptosVM::verify_transactions(txns)
        });
        drop(signature_verification_timer);

        let txn_provider = ShardedTxnProvider::new(
            block_id,
            num_shards,
            shard_id.unwrap(),
            rx,
            senders,
            pre_processed_txns,
            local_idxs_by_global,
            global_idxs,
            remote_dependencies,
            following_shard_sets,
        );

        let ret = BlockAptosVM::execute_block(
            executor_thread_pool,
            txn_provider,
            state_view,
            concurrency_level,
            maybe_block_gas_limit,
            cross_shard_commit_sender,
        );
        ret
    }

    fn execute_block(
        &self,
        txn_provider_args: TxnProviderArgs,
        state_view: &S,
        concurrency_level: usize,
        maybe_block_gas_limit: Option<u64>,
    ) -> Result<Vec<TransactionOutput>, VMStatus> {
        let round = 0;
        let _timer = SHARDED_BLOCK_EXECUTION_BY_ROUNDS_SECONDS
            .with_label_values(&[&self.shard_id.to_string(), &round.to_string()])
            .start_timer();
        SHARDED_BLOCK_EXECUTOR_TXN_COUNT
            .with_label_values(&[&self.shard_id.to_string(), &round.to_string()])
            .observe(txn_provider_args.txns.len() as f64);
        info!(
                "executing sub block for shard {} and round {}, number of txns {}",
                self.shard_id,
                round,
                txn_provider_args.txns.len()
            );
        disable_speculative_logging();
        trace!(
            "executing sub block for shard {} and round {}",
            self.shard_id,
            round
        );
        let result =
            Self::execute_transactions_with_dependencies(
                Some(self.shard_id),
                self.executor_thread_pool.clone(),
                txn_provider_args,
                self.cross_shard_client.clone(),
                None,
                round,
                state_view,
                concurrency_level,
                maybe_block_gas_limit,
            )?;
        trace!(
                "Finished executing sub block for shard {} and round {}",
                self.shard_id,
                round
            );
        Ok(result)
    }

    pub fn start(&self) {
        trace!(
            "Shard starting, shard_id={}, num_shards={}.",
            self.shard_id,
            self.num_shards
        );
        loop {
            let command = self.coordinator_client.receive_execute_command();
            match command {
                ExecutorShardCommand::ExecuteSubBlocks(
                    state_view,
                    txn_provider_args,
                    concurrency_level_per_shard,
                    maybe_block_gas_limit,
                ) => {
                    trace!(
                        "Shard {} received ExecuteBlock command of block size {} ",
                        self.shard_id,
                        txn_provider_args.txns.len(),
                    );
                    let ret = self.execute_block(
                        txn_provider_args,
                        state_view.as_ref(),
                        concurrency_level_per_shard,
                        maybe_block_gas_limit,
                    );
                    drop(state_view);
                    self.coordinator_client.send_execution_result(ret);
                },
                ExecutorShardCommand::Stop => {
                    break;
                },
            }
        }
        trace!("Shard {} is shutting down", self.shard_id);
    }
}
