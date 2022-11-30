// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_bitvec::BitVec;
use aptos_crypto::HashValue;
use aptos_types::{
    block_metadata::BlockMetadata,
    on_chain_config::{OnChainConfig, ValidatorSet},
    transaction::Transaction,
};
use aptos_vm::{
    data_cache::AsMoveResolver, parallel_executor::ParallelAptosVM, AptosVM, VMExecutor,
};
use criterion::{measurement::Measurement, BatchSize, Bencher};
use language_e2e_tests::{
    account_universe::{log_balance_strategy, AUTransactionGen, AccountUniverseGen},
    executor::FakeExecutor,
    gas_costs::TXN_RESERVED,
};
use proptest::{
    collection::vec,
    strategy::{Strategy, ValueTree},
    test_runner::TestRunner,
};

/// Benchmarking support for transactions.
#[derive(Clone, Debug)]
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
                )
            },
            |state| state.execute(),
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
                )
            },
            |state| state.execute_parallel(),
            // The input here is the entire list of signed transactions, so it's pretty large.
            BatchSize::LargeInput,
        )
    }
}

struct TransactionBenchState {
    // Use the fake executor for now.
    // TODO: Hook up the real executor in the future. Here's what needs to be done:
    // 1. Provide a way to construct a write set from the genesis write set + initial balances.
    // 2. Provide a trait for an executor with the functionality required for account_universe.
    // 3. Implement the trait for the fake executor.
    // 4. Implement the trait for the real executor, using the genesis write set implemented in 1
    //    and the helpers in the execution_tests crate.
    // 5. Add a type parameter that implements the trait here and switch "executor" to use it.
    // 6. Add an enum to TransactionBencher that lets callers choose between the fake and real
    //    executors.
    executor: FakeExecutor,
    transactions: Vec<Transaction>,
}

impl TransactionBenchState {
    /// Creates a new benchmark state with the given number of accounts and transactions.
    fn with_size<S>(strategy: S, num_accounts: usize, num_transactions: usize) -> Self
    where
        S: Strategy,
        S::Value: AUTransactionGen,
    {
        let mut state = Self::with_universe(
            strategy,
            universe_strategy(num_accounts, num_transactions),
            num_transactions,
        );

        // Insert a blockmetadata transaction at the beginning to better simulate the real life traffic.
        let validator_set =
            ValidatorSet::fetch_config(&state.executor.get_state_view().as_move_resolver())
                .expect("Unable to retrieve the validator set from storage");

        let new_block = BlockMetadata::new(
            HashValue::zero(),
            0,
            0,
            *validator_set.payload().next().unwrap().account_address(),
            BitVec::with_num_bits(validator_set.num_validators() as u16).into(),
            vec![],
            1,
        );

        state
            .transactions
            .insert(0, Transaction::BlockMetadata(new_block));

        state
    }

    /// Creates a new benchmark state with the given account universe strategy and number of
    /// transactions.
    fn with_universe<S>(
        strategy: S,
        universe_strategy: impl Strategy<Value = AccountUniverseGen>,
        num_transactions: usize,
    ) -> Self
    where
        S: Strategy,
        S::Value: AUTransactionGen,
    {
        let mut runner = TestRunner::default();
        let universe = universe_strategy
            .new_tree(&mut runner)
            .expect("creating a new value should succeed")
            .current();

        let mut executor = FakeExecutor::from_head_genesis();
        // Run in gas-cost-stability mode for now -- this ensures that new accounts are ignored.
        // XXX We may want to include new accounts in case they have interesting performance
        // characteristics.
        let mut universe = universe.setup_gas_cost_stability(&mut executor);

        let transaction_gens = vec(strategy, num_transactions)
            .new_tree(&mut runner)
            .expect("creating a new value should succeed")
            .current();
        let transactions = transaction_gens
            .into_iter()
            .map(|txn_gen| Transaction::UserTransaction(txn_gen.apply(&mut universe).0))
            .collect();

        Self {
            executor,
            transactions,
        }
    }

    /// Executes this state in a single block.
    fn execute(self) {
        // The output is ignored here since we're just testing transaction performance, not trying
        // to assert correctness.
        AptosVM::execute_block(self.transactions, self.executor.get_state_view())
            .expect("VM should not fail to start");
    }

    /// Executes this state in a single block via parallel execution.
    fn execute_parallel(self) {
        // The output is ignored here since we're just testing transaction performance, not trying
        // to assert correctness.
        ParallelAptosVM::execute_block(
            self.transactions,
            self.executor.get_state_view(),
            num_cpus::get(),
        )
        .expect("VM should not fail to start");
    }
}

/// Returns a strategy for the account universe customized for benchmarks.
fn universe_strategy(
    num_accounts: usize,
    num_transactions: usize,
) -> impl Strategy<Value = AccountUniverseGen> {
    // Multiply by 5 past the number of  to provide
    let max_balance = TXN_RESERVED * num_transactions as u64 * 5;
    let balance_strategy = log_balance_strategy(max_balance);
    AccountUniverseGen::strategy(num_accounts, balance_strategy)
}
