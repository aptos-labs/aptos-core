// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_types::transaction::sender_aware::SenderAwareTransaction;
use move_core_types::account_address::AccountAddress;
use std::{
    collections::{HashMap, VecDeque},
    fmt::Debug,
};

/// A structure to maintain a set of transactions that are pending to be added to the block indexed by
/// the sender. For a particular sender, relative ordering of transactions are maintained,
/// so that the final block preserves the ordering of transactions by sender. It also maintains a vector
/// to preserve the original order of the transactions. This is needed in case we can't find
/// any non-conflicting transactions and we need to add the first pending transaction to the block.
#[derive(Debug)]
pub(crate) struct PendingTransactions<Txn> {
    txns_by_senders: HashMap<AccountAddress, VecDeque<Txn>>,
    // Transactions are kept in the original order. This is not kept in sync with pending transactions,
    // so this can contain a bunch of transactions that are already added to the block.
    ordered_txns: VecDeque<Txn>,
}

impl<Txn> PendingTransactions<Txn>
where
    Txn: SenderAwareTransaction + Clone + Debug + PartialEq,
{
    pub fn new() -> Self {
        Self {
            txns_by_senders: HashMap::new(),
            ordered_txns: VecDeque::new(),
        }
    }

    pub fn add_transaction(&mut self, txn: Txn) {
        self.ordered_txns.push_back(txn.clone());
        self.txns_by_senders
            .entry(txn.parse_sender())
            .or_default()
            .push_back(txn);
    }

    /// Removes the first pending transaction from the sender. Please note that the transaction is not
    /// removed from the `ordered_txns`, so the `ordered_txns` will contain a set of transactions that
    /// are removed from pending transactions already.
    pub fn remove_pending_from_sender(&mut self, sender: AccountAddress) -> Option<Txn> {
        self.txns_by_senders
            .get_mut(&sender)
            .and_then(|txns| txns.pop_front())
    }

    pub fn remove_first_pending(&mut self) -> Option<Txn> {
        while let Some(txn) = self.ordered_txns.pop_front() {
            let sender = txn.parse_sender();
            if let Some(sender_queue) = self.txns_by_senders.get(&sender) {
                if Some(txn).as_ref() == sender_queue.front() {
                    return self.remove_pending_from_sender(sender);
                }
            }
        }
        None
    }
}
