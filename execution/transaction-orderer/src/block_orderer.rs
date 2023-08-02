// Copyright © Aptos Foundation

use crate::batch_orderer::{BatchOrderer, BatchOrdererWithWindow};
use std::cell::RefCell;
use std::cmp::{max, min};

/// Orders transactions in a way to avoid close dependencies between transactions
/// as much as possible. I.e., if transaction A depends on transaction B (i.e., it reads or
/// writes what B writes), then A should not be ordered close to B.
/// The orderer implementation can be heuristic and may not guarantee optimality of the resulting
/// order.
pub trait BlockOrderer {
    type Txn;

    fn order_transactions<F, E>(
        &self,
        txns: Vec<Self::Txn>,
        send_transactions_for_execution: F,
    ) -> Result<(), E>
    where
        F: FnMut(Vec<Self::Txn>) -> Result<(), E>,
        E: std::error::Error;
}

/// Orders the transactions in a block in batches, using the underlying `BatchOrderer`,
/// which guarantees that there are no dependencies between transactions in the same batch.
/// Note, however, that there may be dependencies between transactions in neighbouring batches.
/// The underlying `BatchOrderer` must maintain the following invariant to ensure that all
/// transactions are eventually ordered: if not `batch_orderer.is_empty()`,
/// then `batch_orderer.count_selected() > 0`.
pub struct BatchedBlockOrdererWithoutWindow<O> {
    batch_orderer: RefCell<O>,
    insert_batch_size: usize,
}

impl<O> BatchedBlockOrdererWithoutWindow<O> {
    pub fn new(batch_orderer: O, insert_batch_size: usize) -> Self {
        Self {
            batch_orderer: RefCell::new(batch_orderer),
            insert_batch_size,
        }
    }
}

impl<O> BlockOrderer for BatchedBlockOrdererWithoutWindow<O>
where
    O: BatchOrderer,
{
    type Txn = O::Txn;

    fn order_transactions<F, E>(
        &self,
        txns: Vec<Self::Txn>,
        mut send_transactions_for_execution: F,
    ) -> Result<(), E>
    where
        F: FnMut(Vec<Self::Txn>) -> Result<(), E>,
        E: std::error::Error,
    {
        let mut batch_orderer = self.batch_orderer.borrow_mut();
        assert!(batch_orderer.is_empty());

        let mut txns = txns.into_iter();
        let mut n_ordered = 0;
        let mut n_added = 0;

        while txns.len() > 0 || !batch_orderer.is_empty() {
            // The second condition ensures that addition of transactions to the orderer
            // is amortized so that the time complexity until K transactions are ordered
            // is proportional to K and not to the size of the block.
            if txns.len() > 0 && n_added < max(5 * self.insert_batch_size, n_ordered * 10) {
                n_added += min(txns.len(), self.insert_batch_size);
                batch_orderer.add_transactions((&mut txns).take(self.insert_batch_size));
            }

            let n_selected = batch_orderer.count_selected();
            assert!(n_selected > 0);
            batch_orderer.commit_prefix_callback((n_selected + 1) / 2, |ordered_batch| {
                n_ordered += ordered_batch.len();
                send_transactions_for_execution(ordered_batch)
            })?;
        }
        Ok(())
    }
}

/// Orders the transactions in a block in batches, using the underlying `BatchOrdererWithWindow`,
/// avoiding close dependencies between transactions within and across batches.
/// The underlying `BatchOrdererWithWindow` must maintain the following invariant to ensure
/// that all transactions are eventually ordered: if not `batch_orderer.is_empty()` and
/// `batch_orderer.get_window_size() == 0`, then `batch_orderer.count_selected() > 0`.
pub struct BatchedBlockOrdererWithWindow<O> {
    batch_orderer: RefCell<O>,
    insert_batch_size: usize,
    max_window_size: usize,
}

impl<O> BatchedBlockOrdererWithWindow<O> {
    pub fn new(batch_orderer: O, insert_batch_size: usize, max_window_size: usize) -> Self {
        Self {
            batch_orderer: RefCell::new(batch_orderer),
            insert_batch_size,
            max_window_size,
        }
    }
}

impl<O> BlockOrderer for BatchedBlockOrdererWithWindow<O>
where
    O: BatchOrdererWithWindow,
{
    type Txn = O::Txn;

    fn order_transactions<F, E>(
        &self,
        txns: Vec<Self::Txn>,
        mut send_transactions_for_execution: F,
    ) -> Result<(), E>
    where
        F: FnMut(Vec<Self::Txn>) -> Result<(), E>,
        E: std::error::Error,
    {
        let mut batch_orderer = self.batch_orderer.borrow_mut();
        assert!(batch_orderer.is_empty());

        let mut txns = txns.into_iter();
        let mut n_ordered = 0;
        let mut n_added = 0;

        while txns.len() > 0 || !batch_orderer.is_empty() {
            // The second condition ensures that addition of transactions to the orderer
            // is amortized so that the time complexity until K transactions are ordered
            // is proportional to K and not to the size of the block.
            if txns.len() > 0 && n_added < max(5 * self.insert_batch_size, n_ordered * 10) {
                n_added += min(txns.len(), self.insert_batch_size);
                batch_orderer.add_transactions((&mut txns).take(self.insert_batch_size));
            }

            if batch_orderer.get_window_size() > self.max_window_size {
                let window_size = batch_orderer.get_window_size();
                batch_orderer.forget_prefix(window_size - self.max_window_size);
            }

            while 2 * batch_orderer.get_window_size() > batch_orderer.count_selected() {
                let window_size = batch_orderer.get_window_size();
                batch_orderer.forget_prefix((window_size + 2) / 3);
            }

            let n_selected = batch_orderer.count_selected();
            assert!(n_selected > 0);
            batch_orderer.commit_prefix_callback((n_selected + 1) / 2, |ordered_batch| {
                n_ordered += ordered_batch.len();
                send_transactions_for_execution(ordered_batch)
            })?;
        }
        Ok(())
    }
}
