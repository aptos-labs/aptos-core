// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use aptos_bitvec::BitVec;
use aptos_crypto::HashValue;
use aptos_executor_service::remote_executor_client::RemoteExecutorClient;
use aptos_language_e2e_tests::{
    account_universe::{AUTransactionGen, AccountPickStyle, AccountUniverse, AccountUniverseGen},
    data_store::FakeDataStore,
    executor::FakeExecutor,
    gas_costs::TXN_RESERVED,
};
use aptos_types::{
    block_metadata::BlockMetadata,
    on_chain_config::{OnChainConfig, ValidatorSet},
    transaction::Transaction,
};
use aptos_vm::{
    data_cache::AsMoveResolver,
    sharded_block_executor::{block_executor_client::LocalExecutorClient, ShardedBlockExecutor},
};
use criterion::{measurement::Measurement, BatchSize, Bencher};
use proptest::{
    collection::vec,
    strategy::{Strategy, ValueTree},
    test_runner::TestRunner,
};
use std::{net::SocketAddr, sync::Arc, time::Instant};

/// Benchmarking support for transactions.
#[derive(Clone)]
pub struct TransactionBencher<S> {
    num_accounts: usize,
    num_transactions: usize,
    strategy: S,
}

impl<S> TransactionBencher<S>
where
    S: Strategy,
    S::Value: AUTransactionGen,
{
    /// The number of accounts created by default.
    pub const DEFAULT_NUM_ACCOUNTS: usize = 100;
    /// The number of transactions created by default.
    pub const DEFAULT_NUM_TRANSACTIONS: usize = 1000;

    /// Creates a new transaction bencher with default settings.
    pub fn new(strategy: S) -> Self {
        Self {
            num_accounts: Self::DEFAULT_NUM_ACCOUNTS,
            num_transactions: Self::DEFAULT_NUM_TRANSACTIONS,
            strategy,
        }
    }

    /// Sets a custom number of accounts.
    pub fn num_accounts(&mut self, num_accounts: usize) -> &mut Self {
        self.num_accounts = num_accounts;
        self
    }

    /// Sets a custom number of transactions.
    pub fn num_transactions(&mut self, num_transactions: usize) -> &mut Self {
        self.num_transactions = num_transactions;
        self
    }

    /// Runs the bencher.
    pub fn bench<M: Measurement>(&self, b: &mut Bencher<M>) {
        b.iter_batched(
            || {
                TransactionBenchState::with_size(
                    &self.strategy,
                    self.num_accounts,
                    self.num_transactions,
                    1,
                    None,
                    AccountPickStyle::Unlimited,
                )
            },
            |state| state.execute_sequential(),
            // The input here is the entire list of signed transactions, so it's pretty large.
            BatchSize::LargeInput,
        )
    }

    /// Runs the bencher.
    pub fn bench_parallel<M: Measurement>(&self, b: &mut Bencher<M>) {
        b.iter_batched(
            || {
                TransactionBenchState::with_size(
                    &self.strategy,
                    self.num_accounts,
                    self.num_transactions,
                    1,
                    None,
                    AccountPickStyle::Unlimited,
                )
            },
            |state| state.execute_parallel(),
            // The input here is the entire list of signed transactions, so it's pretty large.
            BatchSize::LargeInput,
        )
    }

    /// Runs the bencher.
    pub fn blockstm_benchmark(
        &self,
        num_accounts: usize,
        num_txn: usize,
        run_par: bool,
        run_seq: bool,
        num_warmups: usize,
        num_runs: usize,
        num_executor_shards: usize,
        concurrency_level_per_shard: usize,
        remote_executor_addresses: Option<Vec<SocketAddr>>,
        no_conflict_txn: bool,
        maybe_block_gas_limit: Option<u64>,
    ) -> (Vec<usize>, Vec<usize>) {
        let mut par_tps = Vec::new();
        let mut seq_tps = Vec::new();

        let total_runs = num_warmups + num_runs;

        println!(
            "RUN benchmark for: num_shards {},  concurrency_level_per_shard = {}, \
                        num_account = {}, \
                        block_size = {}",
            num_executor_shards, concurrency_level_per_shard, num_accounts, num_txn,
        );
        let account_pick_style = if no_conflict_txn {
            AccountPickStyle::Limited(1)
        } else {
            AccountPickStyle::Unlimited
        };

        let mut state = TransactionBenchState::with_size(
            &self.strategy,
            num_accounts,
            num_txn,
            num_executor_shards,
            remote_executor_addresses,
            account_pick_style,
        );

        for i in 0..total_runs {
            if i < num_warmups {
                println!("WARMUP - ignore results");
                state.execute_blockstm_benchmark(
                    run_par,
                    run_seq,
                    no_conflict_txn,
                    concurrency_level_per_shard,
                    maybe_block_gas_limit,
                );
            } else {
                let tps = state.execute_blockstm_benchmark(
                    run_par,
                    run_seq,
                    no_conflict_txn,
                    concurrency_level_per_shard,
                    maybe_block_gas_limit,
                );
                par_tps.push(tps.0);
                seq_tps.push(tps.1);
            }
        }

        (par_tps, seq_tps)
    }
}

struct TransactionBenchState<S> {
    num_transactions: usize,
    strategy: S,
    account_universe: AccountUniverse,
    parallel_block_executor: Arc<ShardedBlockExecutor<FakeDataStore>>,
    sequential_block_executor: Arc<ShardedBlockExecutor<FakeDataStore>>,
    validator_set: ValidatorSet,
    state_view: Arc<FakeDataStore>,
}

impl<S> TransactionBenchState<S>
where
    S: Strategy,
    S::Value: AUTransactionGen,
{
    /// Creates a new benchmark state with the given number of accounts and transactions.
    fn with_size(
        strategy: S,
        num_accounts: usize,
        num_transactions: usize,
        num_executor_shards: usize,
        remote_executor_addresses: Option<Vec<SocketAddr>>,
        account_pick_style: AccountPickStyle,
    ) -> Self {
        Self::with_universe(
            strategy,
            universe_strategy(num_accounts, num_transactions, account_pick_style),
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
        remote_executor_addresses: Option<Vec<SocketAddr>>,
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
        let parallel_block_executor =
            if let Some(remote_executor_addresses) = remote_executor_addresses {
                let remote_executor_clients = remote_executor_addresses
                    .into_iter()
                    .map(|addr| RemoteExecutorClient::new(addr, 10000))
                    .collect::<Vec<RemoteExecutorClient>>();
                Arc::new(ShardedBlockExecutor::new(remote_executor_clients))
            } else {
                let local_executor_client =
                    LocalExecutorClient::create_local_clients(num_executor_shards, None);
                Arc::new(ShardedBlockExecutor::new(local_executor_client))
            };
        let sequential_executor_client = LocalExecutorClient::create_local_clients(1, Some(1));
        let sequential_block_executor =
            Arc::new(ShardedBlockExecutor::new(sequential_executor_client));

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
            parallel_block_executor,
            sequential_block_executor,
            validator_set,
            state_view,
        }
    }

    pub fn gen_transaction(&mut self, no_conflict_txns: bool) -> Vec<Transaction> {
        if no_conflict_txns {
            // resetting the picker here so that we can re-use the same accounts from last block
            // but still generate non conflicting transactions for this block.
            self.account_universe.reset_picker()
        }
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
        transactions
    }

    /// Executes this state in a single block.
    fn execute_sequential(mut self) {
        // The output is ignored here since we're just testing transaction performance, not trying
        // to assert correctness.
        let txns = self.gen_transaction(false);
        let executor = self.sequential_block_executor;
        executor
            .execute_block(self.state_view.clone(), txns, 1, None)
            .expect("VM should not fail to start");
    }

    /// Executes this state in a single block.
    fn execute_parallel(mut self) {
        // The output is ignored here since we're just testing transaction performance, not trying
        // to assert correctness.
        let txns = self.gen_transaction(false);
        let executor = self.parallel_block_executor.clone();
        executor
            .execute_block(self.state_view.clone(), txns, num_cpus::get(), None)
            .expect("VM should not fail to start");
    }

    fn execute_benchmark(
        &self,
        transactions: Vec<Transaction>,
        block_executor: Arc<ShardedBlockExecutor<FakeDataStore>>,
        concurrency_level_per_shard: usize,
        maybe_block_gas_limit: Option<u64>,
    ) -> usize {
        let block_size = transactions.len();
        let timer = Instant::now();
        block_executor
            .execute_block(
                self.state_view.clone(),
                transactions,
                concurrency_level_per_shard,
                maybe_block_gas_limit,
            )
            .expect("VM should not fail to start");
        let exec_time = timer.elapsed().as_millis();

        block_size * 1000 / exec_time as usize
    }

    fn execute_blockstm_benchmark(
        &mut self,
        run_par: bool,
        run_seq: bool,
        no_conflict_txns: bool,
        conurrency_level_per_shard: usize,
        maybe_block_gas_limit: Option<u64>,
    ) -> (usize, usize) {
        let transactions = self.gen_transaction(no_conflict_txns);
        let par_tps = if run_par {
            println!("Parallel execution starts...");
            let tps = self.execute_benchmark(
                transactions.clone(),
                self.parallel_block_executor.clone(),
                conurrency_level_per_shard,
                maybe_block_gas_limit,
            );
            println!("Parallel execution finishes, TPS = {}", tps);
            tps
        } else {
            0
        };
        let seq_tps = if run_seq {
            println!("Sequential execution starts...");
            let tps = self.execute_benchmark(
                transactions,
                self.sequential_block_executor.clone(),
                1,
                maybe_block_gas_limit,
            );
            println!("Sequential execution finishes, TPS = {}", tps);
            tps
        } else {
            0
        };
        (par_tps, seq_tps)
    }
}

/// Returns a strategy for the account universe customized for benchmarks, i.e. having
/// sufficiently large balance for gas.
fn universe_strategy(
    num_accounts: usize,
    num_transactions: usize,
    account_pick_style: AccountPickStyle,
) -> impl Strategy<Value = AccountUniverseGen> {
    let balance = TXN_RESERVED * num_transactions as u64 * 5;
    AccountUniverseGen::strategy(num_accounts, balance..(balance + 1), account_pick_style)
}
