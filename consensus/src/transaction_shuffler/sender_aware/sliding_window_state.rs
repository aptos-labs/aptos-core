// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::counters::NUM_SENDERS_IN_BLOCK;
use aptos_types::transaction::sender_aware::SenderAwareTransaction;
use move_core_types::account_address::AccountAddress;
use std::{collections::HashMap, fmt::Debug};

/// A stateful data structure maintained by the transaction shuffler during shuffling. On a
/// high level, it maintains a sliding window of the conflicting transactions, which helps the payload
/// generator include a set of transactions which are non-conflicting with each other within a particular
/// window size.
#[derive(Debug)]
pub(crate) struct SlidingWindowState<Txn> {
    // Please note that the start index can be negative in case the window size is larger than the
    // end_index.
    start_index: i64,
    // Hashmap of senders to the number of transactions included in the window for the corresponding
    // sender.
    senders_in_window: HashMap<AccountAddress, usize>,
    // Partially ordered transactions, needs to be updated every time add_transactions is called.
    txns: Vec<Txn>,
}

impl<Txn> SlidingWindowState<Txn>
where
    Txn: SenderAwareTransaction + Debug + Clone + PartialEq,
{
    pub fn new(window_size: usize, num_txns: usize) -> Self {
        Self {
            start_index: -(window_size as i64),
            senders_in_window: HashMap::new(),
            txns: Vec::with_capacity(num_txns),
        }
    }

    /// Slides the current window. Essentially, it increments the start_index and
    /// updates the senders_in_window map if start_index is greater than 0
    pub fn add_transaction(&mut self, txn: Txn) {
        if self.start_index >= 0 {
            // if the start_index is negative, then no sender falls out of the window.
            let sender = self
                .txns
                .get(self.start_index as usize)
                .expect("Transaction expected")
                .parse_sender();
            self.senders_in_window
                .entry(sender)
                .and_modify(|count| *count -= 1);
        }
        let count = self
            .senders_in_window
            .entry(txn.parse_sender())
            .or_insert_with(|| 0);
        *count += 1;
        self.txns.push(txn);
        self.start_index += 1;
    }

    pub fn has_conflict(&self, addr: &AccountAddress) -> bool {
        self.senders_in_window
            .get(addr)
            .map_or(false, |count| *count != 0)
    }

    /// Returns the sender which was dropped off of the conflict window in previous iteration.
    pub fn last_dropped_sender(&self) -> Option<AccountAddress> {
        if self.start_index > 0 {
            let prev_start_index = self.start_index - 1;
            if let Some(last_sender) = self
                .txns
                .get(prev_start_index as usize)
                .map(|txn| txn.parse_sender())
            {
                if let Some(&count) = self.senders_in_window.get(&last_sender) {
                    if count == 0 {
                        return Some(last_sender);
                    }
                }
            }
        }
        None
    }

    pub fn num_txns(&self) -> usize {
        self.txns.len()
    }

    pub fn finalize(self) -> Vec<Txn> {
        NUM_SENDERS_IN_BLOCK.set(self.senders_in_window.len() as f64);
        self.txns
    }
}
