// Copyright © Aptos Foundation

use crate::transaction_bench_state::TransactionBenchState;
use aptos_language_e2e_tests::account_universe::{AUTransactionGen, AccountPickStyle};
use aptos_types::transaction::Transaction;
use proptest::strategy::Strategy;
use std::net::SocketAddr;

pub(crate) trait BenchmarkRunner {
    fn run_benchmark(
        &mut self,
        run_par: bool,
        run_seq: bool,
        conurrency_level_per_shard: usize,
        maybe_block_gas_limit: Option<u64>,
    ) -> (usize, usize);
}

pub struct TransactionBenchmarkRunner<S> {
    strategy: S,
    num_accounts: usize,
    num_txn: usize,
    num_executor_shards: usize,
    remote_executor_addresses: Option<Vec<SocketAddr>>,
    account_pick_style: AccountPickStyle,
}

impl<S> TransactionBenchmarkRunner<S>
where
    S: Strategy,
{
    pub fn new(
        strategy: S,
        num_accounts: usize,
        num_txn: usize,
        num_executor_shards: usize,
        remote_executor_addresses: Option<Vec<SocketAddr>>,
        account_pick_style: AccountPickStyle,
    ) -> Self {
        Self {
            strategy,
            num_accounts,
            num_txn,
            num_executor_shards,
            remote_executor_addresses,
            account_pick_style,
        }
    }
}

impl<S> BenchmarkRunner for TransactionBenchmarkRunner<S>
where
    S: Strategy,
    S::Value: AUTransactionGen,
{
    fn run_benchmark(
        &mut self,
        run_par: bool,
        run_seq: bool,
        concurrency_level_per_shard: usize,
        maybe_block_gas_limit: Option<u64>,
    ) -> (usize, usize) {
        let mut state = TransactionBenchState::with_size(
            &self.strategy,
            self.num_accounts,
            self.num_txn,
            self.num_executor_shards,
            self.remote_executor_addresses.clone(),
            self.account_pick_style.clone(),
        );
        let transactions = state.gen_transaction();
        state.execute_blockstm_benchmark(
            transactions,
            run_par,
            run_seq,
            concurrency_level_per_shard,
            maybe_block_gas_limit,
        )
    }
}

pub struct PreGeneratedTxnsBenchmarkRunner<'a, S> {
    states: Vec<TransactionBenchState<&'a S>>,
    // pre-generated transactions
    transactions: Vec<Vec<Transaction>>,
}

impl<'a, S> PreGeneratedTxnsBenchmarkRunner<'a, S>
where
    S: Strategy,
    S::Value: AUTransactionGen,
{
    pub fn new(
        strategy: &'a S,
        num_accounts: usize,
        num_txn: usize,
        num_executor_shards: usize,
        remote_executor_addresses: Option<Vec<SocketAddr>>,
        account_pick_style: AccountPickStyle,
        num_runs: usize,
    ) -> Self {
        println!("Generating transactions for {} runs", num_runs);
        let mut states: Vec<_> = (0..num_runs)
            .map(|_| {
                TransactionBenchState::with_size(
                    strategy,
                    num_accounts,
                    num_txn,
                    num_executor_shards,
                    remote_executor_addresses.clone(),
                    account_pick_style.clone(),
                )
            })
            .collect();
        let transactions = states
            .iter_mut()
            .map(|state| state.gen_transaction())
            .collect();
        println!("Done generating transactions for {} runs", num_runs);
        Self {
            states,
            transactions,
        }
    }
}

impl<S> BenchmarkRunner for PreGeneratedTxnsBenchmarkRunner<'_, S>
where
    S: Strategy,
    S::Value: AUTransactionGen,
{
    fn run_benchmark(
        &mut self,
        run_par: bool,
        run_seq: bool,
        concurrency_level_per_shard: usize,
        maybe_block_gas_limit: Option<u64>,
    ) -> (usize, usize) {
        let mut state = self.states.pop().unwrap();
        let transactions = self.transactions.pop().unwrap();
        state.execute_blockstm_benchmark(
            transactions,
            run_par,
            run_seq,
            concurrency_level_per_shard,
            maybe_block_gas_limit,
        )
    }
}
