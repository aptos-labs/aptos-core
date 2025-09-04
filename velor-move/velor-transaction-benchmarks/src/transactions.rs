// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    benchmark_runner::{
        BenchmarkRunner, PreGeneratedTxnsBenchmarkRunner, TransactionBenchmarkRunner,
    },
    transaction_bench_state::TransactionBenchState,
};
use velor_language_e2e_tests::{
    account_universe::{AUTransactionGen, AccountPickStyle, AccountUniverseGen},
    gas_costs::TXN_RESERVED,
};
use criterion::{measurement::Measurement, BatchSize, Bencher};
use proptest::strategy::Strategy;
use std::net::SocketAddr;

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
        generate_then_execute: bool,
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
        let mut runner: Box<dyn BenchmarkRunner> = if generate_then_execute {
            Box::new(PreGeneratedTxnsBenchmarkRunner::new(
                &self.strategy,
                num_accounts,
                num_txn,
                num_executor_shards,
                remote_executor_addresses,
                account_pick_style,
                total_runs,
            ))
        } else {
            Box::new(TransactionBenchmarkRunner::new(
                &self.strategy,
                num_accounts,
                num_txn,
                num_executor_shards,
                remote_executor_addresses,
                account_pick_style,
            ))
        };
        for i in 0..total_runs {
            if i < num_warmups {
                println!("WARMUP - ignore results");
                runner.run_benchmark(
                    run_par,
                    run_seq,
                    concurrency_level_per_shard,
                    maybe_block_gas_limit,
                );
            } else {
                let tps = runner.run_benchmark(
                    run_par,
                    run_seq,
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

/// Returns a strategy for the account universe customized for benchmarks, i.e. having
/// sufficiently large balance for gas.
pub(crate) fn universe_strategy(
    num_accounts: usize,
    num_transactions: usize,
    account_pick_style: AccountPickStyle,
) -> impl Strategy<Value = AccountUniverseGen> {
    let balance = TXN_RESERVED * num_transactions as u64 * 5;
    AccountUniverseGen::strategy(num_accounts, balance..(balance + 1), account_pick_style)
}
