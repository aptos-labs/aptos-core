// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

//! This crate defines `PtxBlockExecutor` and supporting type that executes purely P-Transactions which
//! have accurately predicable read/write sets.

mod analyzer;
mod common;
mod executor;
mod finalizer;
mod scheduler;
mod state_reader;
mod state_view;

use crate::{
    analyzer::PtxAnalyzer, executor::PtxExecutor, finalizer::PtxFinalizer, scheduler::PtxScheduler,
    state_reader::PtxStateReader,
};
use aptos_infallible::Mutex;
use aptos_state_view::StateView;
use aptos_types::{
    block_executor::partitioner::PartitionedTransactions,
    transaction::{Transaction, TransactionOutput},
};
use aptos_vm::{
    aptos_vm::RAYON_EXEC_POOL,
    sharded_block_executor::{executor_client::ExecutorClient, ShardedBlockExecutor},
    AptosVM, VMExecutor,
};
use move_core_types::vm_status::VMStatus;
use std::sync::{mpsc::channel, Arc};

struct PtxBlockExecutor;

impl VMExecutor for PtxBlockExecutor {
    fn execute_block(
        transactions: Vec<Transaction>,
        state_view: &(impl StateView + Sync),
        _maybe_block_gas_limit: Option<u64>,
    ) -> Result<Vec<TransactionOutput>, VMStatus> {
        // 1. Analyze: annotate read / write sets.
        // 2. Schedule: build dependency graph by remembering the latests writes for each key.
        // 3. Execute: start executing a transaction once its dependencies are met.
        //             and inform dependent transactions after execution.
        // 5. Finalize: materialize aggregators, etc.
        let concurrency_level = AptosVM::get_concurrency_level();
        assert!(
            concurrency_level > 5,
            "Each of the components needs its own main thread."
        );
        let num_executor_workers = concurrency_level - 5;

        let ret = Arc::new(Mutex::new(None));
        let ret_clone = ret.clone();
        RAYON_EXEC_POOL.scope(move |scope| {
            let num_txns = transactions.len();
            let (result_tx, result_rx) = channel();
            let finalizer = PtxFinalizer::spawn(scope, state_view, result_tx);
            let executor = PtxExecutor::spawn(scope, state_view, num_executor_workers, finalizer);
            let state_reader = PtxStateReader::spawn(scope, executor.clone(), state_view);
            let scheduler = PtxScheduler::spawn(scope, executor, state_reader);
            let analyzer = PtxAnalyzer::spawn(scope, scheduler);

            for txn in transactions {
                analyzer.analyze_transaction(txn);
            }
            analyzer.finish_block();

            let mut txn_outputs = vec![];
            while let Ok(txn_output) = result_rx.recv() {
                txn_outputs.push(txn_output);
            }
            assert_eq!(txn_outputs.len(), num_txns);
            ret_clone.lock().replace(txn_outputs);
        });
        let ret = ret.lock().take().unwrap();
        Ok(ret)
    }

    fn execute_block_sharded<S: StateView + Sync + Send + 'static, E: ExecutorClient<S>>(
        _sharded_block_executor: &ShardedBlockExecutor<S, E>,
        _transactions: PartitionedTransactions,
        _state_view: Arc<S>,
        _maybe_block_gas_limit: Option<u64>,
    ) -> Result<Vec<TransactionOutput>, VMStatus> {
        unimplemented!()
    }
}
