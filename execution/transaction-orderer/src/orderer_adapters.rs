// Copyright © Aptos Foundation

use std::hash::Hash;
use std::marker::PhantomData;
use aptos_block_executor::transaction_hints::TransactionHints;
use crate::block_orderer::BlockOrderer;
use crate::parallel::key_compressor::compress_transactions_in_parallel;
use crate::transaction_compressor::CompressedHintsTransaction;

/// Returns a `BlockOrderer` that compresses all transactions using
/// [`ParallelTransactionCompressor`] before passing them to the given `orderer`.
/// The `orderer` then returns the original transactions, so the compression
/// is used purely as a performance optimization for the orderer.
pub fn parallel_compress_then_order<T, O>(orderer: O) -> ParallelCompressThenOrder<T, O>
where
    T: TransactionHints + Send + Sync,
    T::Key: Hash + Clone + Eq + Send + Sync,
    O: BlockOrderer<Txn = CompressedHintsTransaction<T>>,
{
    ParallelCompressThenOrder {
        orderer,
        _phantom: PhantomData,
    }
}

/// Returns a `BlockOrderer` that orders all transactions except the last one
/// using the given `orderer` and then sends the last transaction in its own batch.
pub fn keep_last<O>(orderer: O) -> KeepLast<O> {
    KeepLast { orderer }
}

pub struct ParallelCompressThenOrder<T, O> {
    orderer: O,
    _phantom: PhantomData<T>,
}

impl<T, O> BlockOrderer for ParallelCompressThenOrder<T, O>
where
    T: TransactionHints + Send + Sync,
    T::Key: Hash + Clone + Eq + Send + Sync,
    O: BlockOrderer<Txn = CompressedHintsTransaction<T>>,
{
    type Txn = T;

    fn order_transactions<F, E>(
        &self,
        txns: Vec<Self::Txn>,
        mut send_transactions_for_execution: F) -> Result<(), E>
    where
        F: FnMut(Vec<Self::Txn>) -> Result<(), E>
    {
        let compressed_txns = compress_transactions_in_parallel(txns);
        self.orderer.order_transactions(compressed_txns, |ordered_compressed_txns| {
            send_transactions_for_execution(ordered_compressed_txns.into_iter().map(|tx| tx.original).collect())
        })?;
        Ok(())
    }
}

pub struct KeepLast<O> {
    orderer: O,
}

impl<O: BlockOrderer> BlockOrderer for KeepLast<O> {
    type Txn = O::Txn;

    fn order_transactions<F, E>(
        &self,
        mut txns: Vec<Self::Txn>,
        mut send_transactions_for_execution: F
    ) -> Result<(), E>
    where
        F: FnMut(Vec<Self::Txn>) -> Result<(), E>,
    {
        let last_txn = txns.pop();
        self.orderer.order_transactions(txns, |txns| send_transactions_for_execution(txns))?;
        if let Some(tx) = last_txn {
            send_transactions_for_execution(vec![tx])?;
        }
        Ok(())
    }
}
