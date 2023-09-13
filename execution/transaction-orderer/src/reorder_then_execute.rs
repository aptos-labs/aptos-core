// Copyright © Aptos Foundation

use rayon::prelude::*;
use aptos_block_executor::executor_traits::{BlockExecutor, BlockExecutorBase, HintedBlockExecutor};
use aptos_block_executor::task::{ExecutorTask, IntoTransaction, Transaction};
use aptos_block_executor::transaction_hints::TransactionHints;
use aptos_state_view::TStateView;
use crate::block_orderer::BlockOrderer;


/// A [`HintedBlockExecutor`] that first reorders transactions using a [`BlockOrderer`]
/// and then executes them using a [`BlockExecutor`].
pub struct ReorderThenExecute<BO, BE> {
    orderer: BO,
    block_executor: BE,
}

impl<BO, BE> ReorderThenExecute<BO, BE>
where
    BE: BlockExecutor,
    BO: BlockOrderer,
    BO::Txn: TransactionHints<Key = <BE::Txn as Transaction>::Key> + IntoTransaction<Txn = BE::Txn>,
{
    pub fn new(orderer: BO, block_executor: BE) -> Self {
        Self {
            orderer,
            block_executor,
        }
    }
}

impl<BO, BE> BlockExecutorBase for ReorderThenExecute<BO, BE>
where
    BE: BlockExecutor,
    BO: BlockOrderer,
    BO::Txn: TransactionHints<Key = <BE::Txn as Transaction>::Key> + IntoTransaction<Txn = BE::Txn>,
{
    type Txn = BE::Txn;
    type ExecutorTask = BE::ExecutorTask;
    type Error = BE::Error;
}

impl<BO, BE> HintedBlockExecutor for ReorderThenExecute<BO, BE>
where
    BE: BlockExecutor,
    BO: BlockOrderer,
    BO::Txn: TransactionHints<Key = <BE::Txn as Transaction>::Key> + IntoTransaction<Txn = BE::Txn> + Send,
{
    type HintedTxn = BO::Txn;

    fn execute_block_hinted<S: TStateView<Key = <Self::Txn as Transaction>::Key> + Sync>(
        &self,
        executor_arguments: <Self::ExecutorTask as ExecutorTask>::Argument,
        hinted_transactions: Vec<BO::Txn>,
        base_view: &S,
    ) -> Result<Vec<<Self::ExecutorTask as ExecutorTask>::Output>, Self::Error> {
        let mut ordered_txns = vec![];
        self.orderer.order_transactions(hinted_transactions, |hinted_txns| {
            ordered_txns.par_extend(hinted_txns.into_par_iter().map(|tx| tx.into_transaction()));
            Ok(())
        })?;
        self.block_executor.execute_block(executor_arguments, ordered_txns, base_view)
    }
}
