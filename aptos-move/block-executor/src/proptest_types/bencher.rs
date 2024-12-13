// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    code_cache_global_manager::AptosModuleCacheManagerGuard,
    executor::BlockExecutor,
    proptest_types::{
        baseline::BaselineOutput,
        types::{
            KeyType, MockOutput, MockTask, MockTransaction, TransactionGen, TransactionGenParams,
        },
    },
    txn_commit_hook::NoOpTransactionCommitHook,
    txn_provider::default::DefaultTxnProvider,
};
use aptos_types::{
    block_executor::config::BlockExecutorConfig, contract_event::TransactionEvent,
    executable::ExecutableTestType, state_store::MockStateView,
};
use criterion::{BatchSize, Bencher as CBencher};
use num_cpus;
use proptest::{
    arbitrary::Arbitrary,
    collection::vec,
    prelude::*,
    strategy::{Strategy, ValueTree},
    test_runner::TestRunner,
};
use std::{fmt::Debug, hash::Hash, marker::PhantomData, sync::Arc};

pub struct Bencher<K, V, E> {
    transaction_size: usize,
    transaction_gen_param: TransactionGenParams,
    universe_size: usize,
    phantom: PhantomData<(K, V, E)>,
}

pub(crate) struct BencherState<
    K: Hash + Clone + Debug + Eq + PartialOrd + Ord + Send + Sync + 'static,
    E: Send + Sync + Debug + Clone + TransactionEvent + 'static,
> {
    txns_provider: DefaultTxnProvider<MockTransaction<KeyType<K>, E>>,
    baseline_output: BaselineOutput<KeyType<K>>,
}

impl<K, V, E> Bencher<K, V, E>
where
    K: Hash + Clone + Debug + Eq + Send + Sync + PartialOrd + Ord + Arbitrary + 'static,
    V: Clone + Eq + Send + Sync + Arbitrary + 'static,
    E: Send + Sync + Debug + Clone + TransactionEvent + 'static,
    Vec<u8>: From<V>,
{
    pub fn new(transaction_size: usize, universe_size: usize) -> Self {
        Self {
            transaction_size,
            transaction_gen_param: TransactionGenParams::default(),
            universe_size,
            phantom: PhantomData,
        }
    }

    pub fn bench(&self, key_strategy: &impl Strategy<Value = K>, bencher: &mut CBencher) {
        bencher.iter_batched(
            || {
                BencherState::<K, E>::with_universe::<V>(
                    vec(key_strategy, self.universe_size),
                    self.transaction_size,
                    self.transaction_gen_param,
                )
            },
            |state| state.run(),
            // The input here is the entire list of signed transactions, so it's pretty large.
            BatchSize::LargeInput,
        )
    }
}

impl<K, E> BencherState<K, E>
where
    K: Hash + Clone + Debug + Eq + Send + Sync + PartialOrd + Ord + 'static,
    E: Send + Sync + Debug + Clone + TransactionEvent + 'static,
{
    /// Creates a new benchmark state with the given account universe strategy and number of
    /// transactions.
    pub(crate) fn with_universe<
        V: Into<Vec<u8>> + Clone + Eq + Send + Sync + Arbitrary + 'static,
    >(
        universe_strategy: impl Strategy<Value = Vec<K>>,
        num_transactions: usize,
        transaction_params: TransactionGenParams,
    ) -> Self {
        let mut runner = TestRunner::default();
        let key_universe = universe_strategy
            .new_tree(&mut runner)
            .expect("creating a new value should succeed")
            .current();

        let transaction_gens = vec(
            any_with::<TransactionGen<V>>(transaction_params),
            num_transactions,
        )
        .new_tree(&mut runner)
        .expect("creating a new value should succeed")
        .current();

        let transactions: Vec<_> = transaction_gens
            .into_iter()
            .map(|txn_gen| txn_gen.materialize(&key_universe, (false, false)))
            .collect();
        let txns_provider = DefaultTxnProvider::new(transactions.clone());

        let baseline_output = BaselineOutput::generate(txns_provider.get_txns(), None);

        Self {
            txns_provider: DefaultTxnProvider::new(transactions),
            baseline_output,
        }
    }

    pub(crate) fn run(self) {
        let state_view = MockStateView::empty();

        let executor_thread_pool = Arc::new(
            rayon::ThreadPoolBuilder::new()
                .num_threads(num_cpus::get())
                .build()
                .unwrap(),
        );

        let config = BlockExecutorConfig::new_no_block_limit(num_cpus::get());
        let mut guard = AptosModuleCacheManagerGuard::none();

        let output = BlockExecutor::<
            MockTransaction<KeyType<K>, E>,
            MockTask<KeyType<K>, E>,
            MockStateView<KeyType<K>>,
            NoOpTransactionCommitHook<MockOutput<KeyType<K>, E>, usize>,
            ExecutableTestType,
            DefaultTxnProvider<MockTransaction<KeyType<K>, E>>,
        >::new(config, executor_thread_pool, None)
        .execute_transactions_parallel(&self.txns_provider, &state_view, &mut guard);

        self.baseline_output.assert_parallel_output(&output);
    }
}
