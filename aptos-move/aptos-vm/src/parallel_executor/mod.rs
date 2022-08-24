// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

mod storage_wrapper;
mod vm_wrapper;

use aptos_crypto::hash::DefaultHasher;
use bcs::to_bytes;
use concurrent_lru::sharded::LruCache;
use rand::{thread_rng, Rng};

use crate::{
    adapter_common::{preprocess_transaction, PreprocessedTransaction},
    aptos_vm::AptosVM,
    parallel_executor::vm_wrapper::AptosVMWrapper,
};
use aptos_parallel_executor::{
    errors::Error,
    executor::ParallelTransactionExecutor,
    task::{Transaction as PTransaction, TransactionOutput as PTransactionOutput},
};
use aptos_state_view::StateView;
use aptos_types::{
    state_store::state_key::StateKey,
    transaction::{Transaction, TransactionOutput, TransactionStatus},
    write_set::{WriteOp, WriteSet},
};
use move_deps::move_core_types::vm_status::{StatusCode, VMStatus};
use once_cell::sync::Lazy;
use rayon::prelude::*;

impl PTransaction for PreprocessedTransaction {
    type Key = StateKey;
    type Value = WriteOp;
}

// Wrapper to avoid orphan rule
pub(crate) struct AptosTransactionOutput(TransactionOutput);

impl AptosTransactionOutput {
    pub fn new(output: TransactionOutput) -> Self {
        Self(output)
    }
    pub fn into(self) -> TransactionOutput {
        self.0
    }
}

impl PTransactionOutput for AptosTransactionOutput {
    type T = PreprocessedTransaction;

    fn get_writes(&self) -> Vec<(StateKey, WriteOp)> {
        self.0.write_set().iter().cloned().collect()
    }

    /// Execution output for transactions that comes after SkipRest signal.
    fn skip_output() -> Self {
        Self(TransactionOutput::new(
            WriteSet::default(),
            vec![],
            0,
            TransactionStatus::Retry,
        ))
    }
}

static RAYON_EXEC_POOL: Lazy<rayon::ThreadPool> = Lazy::new(|| {
    rayon::ThreadPoolBuilder::new()
        .num_threads(num_cpus::get())
        .thread_name(|index| format!("parallel_executor_{}", index))
        .build()
        .unwrap()
});

struct LruCache16ShardWrapper {
    state: LruCache<[u8; 32], u64>,
}

impl LruCache16ShardWrapper {
    fn new(cache_size_per_shard: usize) -> LruCache16ShardWrapper {
        LruCache16ShardWrapper {
            state: LruCache::new(cache_size_per_shard as u64),
        }
    }

    pub fn get_key(&self, item: &Transaction) -> [u8; 32] {
        let bytes = to_bytes(item).unwrap();
        let mut hasher = DefaultHasher::new(b"CacheTesting");
        hasher.update(&bytes);
        hasher.finish().get_bytes()
    }

    /// return whether the item exists (cache hit).
    /// NOTE: false cache miss is possible.
    pub fn insert(&self, item: &Transaction) -> bool {
        let nonce = thread_rng().gen::<u64>();
        let entry = self.state.get_or_init(self.get_key(item), 1, |_e| nonce);
        let hit = *entry.value() != nonce;
        hit
    }
}

const CACHE_SIZE_PER_SHARD: usize = 4096;

static CACHE: Lazy<LruCache16ShardWrapper> =
    Lazy::new(|| LruCache16ShardWrapper::new(CACHE_SIZE_PER_SHARD));

pub struct ParallelAptosVM();

impl ParallelAptosVM {
    pub fn execute_block<S: StateView>(
        transactions: Vec<Transaction>,
        state_view: &S,
        concurrency_level: usize,
    ) -> Result<(Vec<TransactionOutput>, Option<Error<VMStatus>>), VMStatus> {
        // Verify the signatures of all the transactions in parallel.
        // This is time consuming so don't wait and do the checking
        // sequentially while executing the transactions.
        let signature_verified_block: Vec<PreprocessedTransaction> = transactions
            .par_iter()
            .filter(|txn| !CACHE.insert(txn))
            .map(|txn| preprocess_transaction::<AptosVM>(txn.clone()))
            .collect();

        match ParallelTransactionExecutor::<PreprocessedTransaction, AptosVMWrapper<S>>::new(
            &RAYON_EXEC_POOL,
            concurrency_level,
        )
        .execute_transactions_parallel(state_view, signature_verified_block)
        {
            Ok(results) => Ok((
                results
                    .into_iter()
                    .map(AptosTransactionOutput::into)
                    .collect(),
                None,
            )),
            Err(err @ Error::ModulePathReadWrite) => {
                let output = AptosVM::execute_block_and_keep_vm_status(transactions, state_view)?;
                Ok((
                    output
                        .into_iter()
                        .map(|(_vm_status, txn_output)| txn_output)
                        .collect(),
                    Some(err),
                ))
            }
            Err(Error::InvariantViolation) => Err(VMStatus::Error(
                StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
            )),
            Err(Error::UserError(err)) => Err(err),
        }
    }
}
