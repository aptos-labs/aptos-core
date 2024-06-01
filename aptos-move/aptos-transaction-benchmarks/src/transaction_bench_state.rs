// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{transactions, transactions::RAYON_EXEC_POOL};
use aptos_bitvec::BitVec;
use aptos_block_executor::txn_commit_hook::NoOpTransactionCommitHook;
use aptos_block_partitioner::{
    v2::config::PartitionerV2Config, BlockPartitioner, PartitionerConfig,
};
use aptos_crypto::HashValue;
use aptos_language_e2e_tests::{
    account_universe::{AUTransactionGen, AccountPickStyle, AccountUniverse, AccountUniverseGen},
    data_store::FakeDataStore,
    executor::FakeExecutor,
};
use aptos_types::{
    block_executor::{
        config::{BlockExecutorConfig, BlockExecutorConfigFromOnchain},
        partitioner::PartitionedTransactions,
    },
    block_metadata::BlockMetadata,
    on_chain_config::{OnChainConfig, ValidatorSet},
    transaction::{
        analyzed_transaction::AnalyzedTransaction,
        signature_verified_transaction::{
            into_signature_verified_block, SignatureVerifiedTransaction,
        },
        ExecutionStatus, Transaction, TransactionOutput, TransactionStatus,
    },
    vm_status::VMStatus,
};
use aptos_vm::{
    block_executor::{AptosTransactionOutput, BlockAptosVM},
    data_cache::AsMoveResolver,
    sharded_block_executor::{
        local_executor_shard::{LocalExecutorClient, LocalExecutorService},
        ShardedBlockExecutor,
    },
};
use proptest::{collection::vec, prelude::Strategy, strategy::ValueTree, test_runner::TestRunner};
use std::{net::SocketAddr, sync::Arc, time::Instant};

pub struct TransactionBenchState<S> {
    num_transactions: usize,
    strategy: S,
    account_universe: AccountUniverse,
    sharded_block_executor:
        Option<Arc<ShardedBlockExecutor<FakeDataStore, LocalExecutorClient<FakeDataStore>>>>,
    block_partitioner: Option<Box<dyn BlockPartitioner>>,
    validator_set: ValidatorSet,
    state_view: Arc<FakeDataStore>,
}

impl<S> TransactionBenchState<S>
where
    S: Strategy,
    S::Value: AUTransactionGen,
{
    /// Creates a new benchmark state with the given number of accounts and transactions.
    pub(crate) fn with_size(
        strategy: S,
        num_accounts: usize,
        num_transactions: usize,
        num_executor_shards: usize,
        remote_executor_addresses: Option<Vec<SocketAddr>>,
        account_pick_style: AccountPickStyle,
    ) -> Self {
        Self::with_universe(
            strategy,
            transactions::universe_strategy(num_accounts, num_transactions, account_pick_style),
            num_transactions,
            num_executor_shards,
            remote_executor_addresses,
        )
    }

    /// Creates a new benchmark state with the given account universe strategy and number of
    /// transactions.
    fn with_universe(
        strategy: S,
        universe_strategy: impl Strategy<Value = AccountUniverseGen>,
        num_transactions: usize,
        num_executor_shards: usize,
        // TODO(skedia): add support for remote executor addresses.
        _remote_executor_addresses: Option<Vec<SocketAddr>>,
    ) -> Self {
        let mut runner = TestRunner::default();
        let universe_gen = universe_strategy
            .new_tree(&mut runner)
            .expect("creating a new value should succeed")
            .current();

        let mut executor = FakeExecutor::from_head_genesis();
        // Run in gas-cost-stability mode for now -- this ensures that new accounts are ignored.
        // XXX We may want to include new accounts in case they have interesting performance
        // characteristics.
        let universe = universe_gen.setup_gas_cost_stability(&mut executor);

        let state_view = Arc::new(executor.get_state_view().clone());
        let (parallel_block_executor, block_partitioner) = if num_executor_shards == 1 {
            (None, None)
        } else {
            let client =
                LocalExecutorService::setup_local_executor_shards(num_executor_shards, None);
            let parallel_block_executor = Arc::new(ShardedBlockExecutor::new(client));
            (
                Some(parallel_block_executor),
                Some(
                    PartitionerV2Config::default()
                        .max_partitioning_rounds(4)
                        .cross_shard_dep_avoid_threshold(0.9)
                        .partition_last_round(true)
                        .build(),
                ),
            )
        };

        let validator_set = ValidatorSet::fetch_config(
            &FakeExecutor::from_head_genesis()
                .get_state_view()
                .as_move_resolver(),
        )
        .expect("Unable to retrieve the validator set from storage");

        Self {
            num_transactions,
            strategy,
            account_universe: universe,
            sharded_block_executor: parallel_block_executor,
            block_partitioner,
            validator_set,
            state_view,
        }
    }

    pub fn gen_transaction(&mut self) -> Vec<SignatureVerifiedTransaction> {
        let mut runner = TestRunner::default();
        let transaction_gens = vec(&self.strategy, self.num_transactions)
            .new_tree(&mut runner)
            .expect("creating a new value should succeed")
            .current();
        let mut transactions: Vec<Transaction> = transaction_gens
            .into_iter()
            .map(|txn_gen| {
                Transaction::UserTransaction(txn_gen.apply(&mut self.account_universe).0)
            })
            .collect();

        // Insert a blockmetadata transaction at the beginning to better simulate the real life traffic.
        let new_block = BlockMetadata::new(
            HashValue::zero(),
            0,
            0,
            *self
                .validator_set
                .payload()
                .next()
                .unwrap()
                .account_address(),
            BitVec::with_num_bits(self.validator_set.num_validators() as u16).into(),
            vec![],
            1,
        );

        transactions.insert(0, Transaction::BlockMetadata(new_block));
        into_signature_verified_block(transactions)
    }

    pub fn partition_txns_if_needed(
        &mut self,
        txns: &[SignatureVerifiedTransaction],
    ) -> Option<PartitionedTransactions> {
        if self.is_shareded() {
            Some(
                self.block_partitioner.as_ref().unwrap().partition(
                    txns.iter()
                        .skip(1)
                        .map(|txn| txn.expect_valid().clone().into())
                        .collect::<Vec<AnalyzedTransaction>>(),
                    self.sharded_block_executor.as_ref().unwrap().num_shards(),
                ),
            )
        } else {
            None
        }
    }

    /// Executes this state in a single block.
    pub(crate) fn execute_sequential(mut self) {
        // The output is ignored here since we're just testing transaction performance, not trying
        // to assert correctness.
        let txns = self.gen_transaction();
        self.execute_benchmark_sequential(&txns, None);
    }

    /// Executes this state in a single block.
    pub(crate) fn execute_parallel(mut self) {
        // The output is ignored here since we're just testing transaction performance, not trying
        // to assert correctness.
        let txns = self.gen_transaction();
        self.execute_benchmark_parallel(&txns, num_cpus::get(), None);
    }

    fn is_shareded(&self) -> bool {
        self.sharded_block_executor.is_some()
    }

    fn execute_benchmark_sequential(
        &self,
        transactions: &[SignatureVerifiedTransaction],
        maybe_block_gas_limit: Option<u64>,
    ) -> (Vec<TransactionOutput>, usize) {
        let block_size = transactions.len();
        let timer = Instant::now();
        let output = BlockAptosVM::execute_block::<
            _,
            NoOpTransactionCommitHook<AptosTransactionOutput, VMStatus>,
        >(
            Arc::clone(&RAYON_EXEC_POOL),
            transactions,
            self.state_view.as_ref(),
            BlockExecutorConfig::new_maybe_block_limit(1, maybe_block_gas_limit),
            None,
        )
        .expect("VM should not fail to start")
        .into_transaction_outputs_forced();
        let exec_time = timer.elapsed().as_millis();

        (output, block_size * 1000 / exec_time as usize)
    }

    fn execute_benchmark_sharded(
        &self,
        transactions: PartitionedTransactions,
        concurrency_level_per_shard: usize,
        maybe_block_gas_limit: Option<u64>,
    ) -> (Vec<TransactionOutput>, usize) {
        let block_size = transactions.num_txns();
        let timer = Instant::now();
        let output = self
            .sharded_block_executor
            .as_ref()
            .unwrap()
            .execute_block(
                self.state_view.clone(),
                transactions,
                concurrency_level_per_shard,
                BlockExecutorConfigFromOnchain::new_maybe_block_limit(maybe_block_gas_limit),
            )
            .expect("VM should not fail to start");
        let exec_time = timer.elapsed().as_millis();

        (output, block_size * 1000 / exec_time as usize)
    }

    fn execute_benchmark_parallel(
        &self,
        transactions: &[SignatureVerifiedTransaction],
        concurrency_level_per_shard: usize,
        maybe_block_gas_limit: Option<u64>,
    ) -> (Vec<TransactionOutput>, usize) {
        let block_size = transactions.len();
        let timer = Instant::now();
        let output = BlockAptosVM::execute_block::<
            _,
            NoOpTransactionCommitHook<AptosTransactionOutput, VMStatus>,
        >(
            Arc::clone(&RAYON_EXEC_POOL),
            transactions,
            self.state_view.as_ref(),
            BlockExecutorConfig::new_maybe_block_limit(
                concurrency_level_per_shard,
                maybe_block_gas_limit,
            ),
            None,
        )
        .expect("VM should not fail to start")
        .into_transaction_outputs_forced();
        let exec_time = timer.elapsed().as_millis();

        (output, block_size * 1000 / exec_time as usize)
    }

    pub(crate) fn execute_blockstm_benchmark(
        &mut self,
        transactions: Vec<SignatureVerifiedTransaction>,
        partitioned_txns: Option<PartitionedTransactions>,
        run_par: bool,
        run_seq: bool,
        conurrency_level_per_shard: usize,
        maybe_block_gas_limit: Option<u64>,
    ) -> (usize, usize) {
        let (output, par_tps) = if run_par {
            println!("Parallel execution starts...");
            let (output, tps) = if self.is_shareded() {
                self.execute_benchmark_sharded(
                    partitioned_txns.unwrap(),
                    conurrency_level_per_shard,
                    maybe_block_gas_limit,
                )
            } else {
                self.execute_benchmark_parallel(
                    &transactions,
                    conurrency_level_per_shard,
                    maybe_block_gas_limit,
                )
            };
            println!("Parallel execution finishes, TPS = {}", tps);
            (output, tps)
        } else {
            (vec![], 0)
        };
        output.iter().for_each(|txn_output| {
            assert_eq!(
                txn_output.status(),
                &TransactionStatus::Keep(ExecutionStatus::Success)
            );
        });
        let (output, seq_tps) = if run_seq {
            println!("Sequential execution starts...");
            let (output, tps) =
                self.execute_benchmark_sequential(&transactions, maybe_block_gas_limit);
            println!("Sequential execution finishes, TPS = {}", tps);
            (output, tps)
        } else {
            (vec![], 0)
        };
        output.iter().for_each(|txn_output| {
            assert_eq!(
                txn_output.status(),
                &TransactionStatus::Keep(ExecutionStatus::Success)
            );
        });
        (par_tps, seq_tps)
    }
}
