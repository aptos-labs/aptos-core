// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::transaction_shuffler::sender_aware::{
    config::Config, pending_transactions::PendingTransactions,
    sliding_window_state::SlidingWindowState,
};
use aptos_types::transaction::sender_aware::SenderAwareTransaction;
use std::{collections::VecDeque, fmt::Debug};

#[derive(Debug)]
pub struct ShuffledTransactionIterator<Txn> {
    input_num_transactions: usize,
    input_queue: VecDeque<Txn>,
    sliding_window_state: SlidingWindowState<Txn>,
    pending_txns: PendingTransactions<Txn>,
}

impl<Txn> ShuffledTransactionIterator<Txn>
where
    Txn: SenderAwareTransaction + Debug + Clone + PartialEq,
{
    pub fn new(config: Config, input_txns: Vec<Txn>) -> Self {
        let pending_txns = PendingTransactions::new();
        let input_num_transactions = input_txns.len();
        let orig_txns = VecDeque::from(input_txns);
        Self {
            input_num_transactions,
            input_queue: orig_txns,
            sliding_window_state: SlidingWindowState::new(
                config.conflict_window_size,
                input_num_transactions,
            ),
            pending_txns,
        }
    }

    pub(super) fn select_next_txn_inner(&mut self) -> Txn {
        // First check if we have a sender dropped off of conflict window in previous step, if so,
        // we try to find pending transaction from the corresponding sender and add it to the block.
        if let Some(sender) = self.sliding_window_state.last_dropped_sender() {
            if let Some(txn) = self.pending_txns.remove_pending_from_sender(sender) {
                self.sliding_window_state.add_transaction(txn.clone());
                return txn;
            }
        }
        // If we can't find any transaction from a sender dropped off of conflict window, then
        // iterate through the original transactions and try to find the next candidate
        while let Some(txn) = self.input_queue.pop_front() {
            if !self.sliding_window_state.has_conflict(&txn.parse_sender()) {
                self.sliding_window_state.add_transaction(txn.clone());
                return txn;
            }
            self.pending_txns.add_transaction(txn);
        }

        // If we can't find any candidate in above steps, then lastly
        // add pending transactions in the order if we can't find any other candidate
        let txn = self
            .pending_txns
            .remove_first_pending()
            .expect("Pending should return a transaction");
        self.sliding_window_state.add_transaction(txn.clone());
        txn
    }

    pub(super) fn select_next_txn(&mut self) -> Option<Txn> {
        if self.sliding_window_state.num_txns() < self.input_num_transactions {
            let txn = self.select_next_txn_inner();
            Some(txn)
        } else {
            None
        }
    }
}

impl<Txn> Iterator for ShuffledTransactionIterator<Txn>
where
    Txn: SenderAwareTransaction + Debug + Clone + PartialEq,
{
    type Item = Txn;

    fn next(&mut self) -> Option<Self::Item> {
        self.select_next_txn()
    }
}
