// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

//! This crate defines `PtxBlockExecutor` and supporting type that executes purely P-Transactions which
//! have accurately predicable read/write sets.

mod analyzer;
mod common;
mod finalizer;
mod metrics;
mod runner;
mod sorter;
mod state_reader;
mod state_view;

mod scheduler;

use crate::{
    analyzer::PtxAnalyzer, finalizer::PtxFinalizer, metrics::TIMER, runner::PtxRunner,
    scheduler::PtxScheduler, sorter::PtxSorter, state_reader::PtxStateReader,
};
use aptos_block_executor::txn_provider::{default::DefaultTxnProvider, TxnProvider};
use aptos_experimental_runtimes::thread_manager::THREAD_MANAGER;
use aptos_infallible::Mutex;
use aptos_metrics_core::TimerHelper;
use aptos_types::{
    block_executor::{
        config::BlockExecutorConfigFromOnchain, partitioner::PartitionedTransactions,
        transaction_slice_metadata::TransactionSliceMetadata,
    },
    state_store::StateView,
    transaction::{
        signature_verified_transaction::SignatureVerifiedTransaction, BlockOutput,
        TransactionOutput,
    },
};
use aptos_vm::{
    sharded_block_executor::{executor_client::ExecutorClient, ShardedBlockExecutor},
    AptosVM, VMBlockExecutor,
};
use move_core_types::vm_status::VMStatus;
use std::sync::{mpsc::channel, Arc};

pub struct PtxBlockExecutor;

impl VMBlockExecutor for PtxBlockExecutor {
    fn new() -> Self {
        Self
    }

    fn execute_block(
        &self,
        txn_provider: &DefaultTxnProvider<SignatureVerifiedTransaction>,
        state_view: &(impl StateView + Sync),
        _onchain_config: BlockExecutorConfigFromOnchain,
        _transaction_slice_metadata: TransactionSliceMetadata,
    ) -> Result<BlockOutput<TransactionOutput>, VMStatus> {
        let _timer = TIMER.timer_with(&["block_total"]);

        let concurrency_level = AptosVM::get_concurrency_level();
        // 1. Analyze: annotate read / write sets.
        // 2. Sort: build dependency graph by remembering the latest writes for each key.
        // 3. Schedule: send readily runnable transactions to the runner.
        // 4. Run: in a pool of workers, inform txn outputs to the scheduler to unblock others.
        // 5. Finalize: materialize aggregators.
        // And, there is the state reader that asynchronously does the DB reads for the scheduler.
        // -- in total we need 6 threads other than the runner worker.
        assert!(
            concurrency_level > 6,
            "Each of the components needs its own main thread."
        );
        let num_executor_workers = concurrency_level - 6;

        let ret = Arc::new(Mutex::new(None));
        let ret_clone = ret.clone();
        THREAD_MANAGER.get_exe_cpu_pool().scope(move |scope| {
            let num_txns = txn_provider.num_txns();
            let (result_tx, result_rx) = channel();

            // Spawn all the components.
            let finalizer = PtxFinalizer::spawn(scope, state_view, result_tx);
            let runner = PtxRunner::spawn(scope, state_view, finalizer);
            let scheduler = PtxScheduler::spawn(scope, runner.clone());
            runner.spawn_workers(scheduler.clone(), num_executor_workers);
            let state_reader = PtxStateReader::spawn(scope, scheduler.clone(), state_view);
            let sorter = PtxSorter::spawn(scope, scheduler, state_reader);
            let analyzer = PtxAnalyzer::spawn(scope, sorter);

            // Feed the transactions down the pipeline.
            //for txn in transactions {
            for idx in 0..num_txns {
                let txn = txn_provider.get_txn(idx as u32);
                analyzer.analyze_transaction((*txn).clone());
            }
            analyzer.finish_block();

            // Collect results from the other side of the pipeline and hand over to outside of the
            // scope.
            let mut txn_outputs = vec![];
            while let Ok(txn_output) = result_rx.recv() {
                txn_outputs.push(txn_output);
            }
            assert_eq!(txn_outputs.len(), num_txns);
            ret_clone.lock().replace(txn_outputs);
        });
        let ret = ret.lock().take().unwrap();
        Ok(BlockOutput::new(ret, None))
    }

    fn execute_block_sharded<S: StateView + Sync + Send + 'static, E: ExecutorClient<S>>(
        _sharded_block_executor: &ShardedBlockExecutor<S, E>,
        _transactions: PartitionedTransactions,
        _state_view: Arc<S>,
        _onchain_config: BlockExecutorConfigFromOnchain,
    ) -> Result<Vec<TransactionOutput>, VMStatus> {
        unimplemented!()
    }
}
