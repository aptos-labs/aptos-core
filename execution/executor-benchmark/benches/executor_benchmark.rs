// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use criterion::{criterion_group, criterion_main, measurement::Measurement, BatchSize, Criterion};
use executor_benchmark::{
    init_db_and_executor, transaction_executor::TransactionExecutor,
    transaction_generator::TransactionGenerator,
};
use executor_types::BlockExecutorTrait;
use std::sync::Arc;

pub const NUM_ACCOUNTS: usize = 1000;
pub const NUM_SEED_ACCOUNTS: usize = 5;
pub const SMALL_BLOCK_SIZE: usize = 500;
pub const MEDIUM_BLOCK_SIZE: usize = 1000;
pub const LARGE_BLOCK_SIZE: usize = 1000;
pub const INITIAL_BALANCE: u64 = 1000000;

//
// Transaction benchmarks
//

fn executor_benchmark<M: Measurement + 'static>(c: &mut Criterion<M>) {
    let (config, genesis_key) = aptos_genesis::test_utils::test_config();

    let (db, executor) = init_db_and_executor(&config);
    let parent_block_id = executor.committed_block_id();
    let executor = Arc::new(executor);

    let mut generator = TransactionGenerator::new(genesis_key);
    let (commit_tx, _commit_rx) = std::sync::mpsc::sync_channel(50 /* bound */);

    let mut executor = TransactionExecutor::new(executor, parent_block_id, 0, Some(commit_tx));

    let txns = generator.create_seed_accounts(
        db.reader,
        NUM_ACCOUNTS,
        SMALL_BLOCK_SIZE,
        INITIAL_BALANCE * 10_000,
    );
    for txn_block in txns {
        executor.execute_block(txn_block);
    }

    let txns = generator.create_and_fund_accounts(
        /*num_existing_accounts=*/ 0,
        NUM_ACCOUNTS,
        INITIAL_BALANCE,
        SMALL_BLOCK_SIZE,
    );
    for txn_block in txns {
        executor.execute_block(txn_block);
    }

    c.bench_function("bench_p2p_small", |bencher| {
        bencher.iter_batched(
            || generator.gen_transfer_transactions(SMALL_BLOCK_SIZE, 1),
            |mut txn_block| executor.execute_block(txn_block.pop().unwrap()),
            BatchSize::LargeInput,
        )
    });

    c.bench_function("bench_p2p_medium", |bencher| {
        bencher.iter_batched(
            || generator.gen_transfer_transactions(MEDIUM_BLOCK_SIZE, 1),
            |mut txn_block| executor.execute_block(txn_block.pop().unwrap()),
            BatchSize::LargeInput,
        )
    });

    c.bench_function("bench_p2p_large", |bencher| {
        bencher.iter_batched(
            || generator.gen_transfer_transactions(LARGE_BLOCK_SIZE, 1),
            |mut txn_block| executor.execute_block(txn_block.pop().unwrap()),
            BatchSize::LargeInput,
        )
    });
}

criterion_group!(
    name = txn_benches;
    config = Criterion::default().sample_size(10);
    targets = executor_benchmark
);

criterion_main!(txn_benches);
