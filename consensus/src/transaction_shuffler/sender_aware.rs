// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{counters::NUM_SENDERS_IN_BLOCK, transaction_shuffler::TransactionShuffler};
use aptos_types::transaction::SignedTransaction;
use move_core_types::account_address::AccountAddress;
use std::collections::{HashMap, VecDeque};

/// An implementation of transaction shuffler, which tries to spread transactions from same senders
/// in a block in order to reduce conflict. On a high level, it works as follows - It defines a
/// `conflict_window_size`, which maintains a set of senders added to the block in last `conflict_window_size`
/// transactions. When trying to select a new transaction to the block, the shuffler tries to find
/// a transaction which are not part of the conflicting senders in the window. If it does, it adds
/// the first non-conflicting transaction it finds to the block, if it doesn't then it preserves the
/// order and adds the first transaction in the remaining block. It always maintains the following
/// invariant in terms of ordering
/// 1. Relative ordering of all transactions from the same before and after shuffling is same
/// 2. Relative ordering of all transactions across different senders will also be maintained if they are
/// non-conflicting. In other words, if the input block has only one transaction per sender, the output
/// ordering will remain unchanged.
///
/// The shuffling algorithm is O(n) and following is the pseudo code for it.
/// loop:
///   if a sender fell out of the sliding window in previous iteration,
///      then: we add the first pending transaction from that sender to the block
///   else while we have transactions to process in the original transaction order
///         take a new one,
///         if it conflicts, add to the pending set
///         else we add it to the block
///   else
///       take the first transaction from the pending transactions and add it to the block

pub struct SenderAwareShuffler {
    conflict_window_size: usize,
}

impl TransactionShuffler for SenderAwareShuffler {
    fn shuffle(&self, txns: Vec<SignedTransaction>) -> Vec<SignedTransaction> {
        // Early return for performance reason if there are no transactions to shuffle
        if txns.is_empty() {
            return txns;
        }

        // handle the corner case of conflict window being 0, in which case we don't do any shuffling
        if self.conflict_window_size == 0 {
            return txns;
        }

        // maintains the intermediate state of the shuffled transactions
        let mut sliding_window = SlidingWindowState::new(self.conflict_window_size, txns.len());
        let mut pending_txns = PendingTransactions::new();
        let num_transactions = txns.len();
        let mut orig_txns = VecDeque::from(txns);
        let mut next_to_add = |sliding_window: &mut SlidingWindowState| -> SignedTransaction {
            // First check if we have a sender dropped off of conflict window in previous step, if so,
            // we try to find pending transaction from the corresponding sender and add it to the block.
            if let Some(sender) = sliding_window.last_dropped_sender() {
                if let Some(txn) = pending_txns.remove_pending_from_sender(sender) {
                    return txn;
                }
            }
            // If we can't find any transaction from a sender dropped off of conflict window, then
            // iterate through the original transactions and try to find the next candidate
            while let Some(txn) = orig_txns.pop_front() {
                if !sliding_window.has_conflict(&txn.sender()) {
                    return txn;
                }
                pending_txns.add_transaction(txn);
            }

            // If we can't find any candidate in above steps, then lastly
            // add pending transactions in the order if we can't find any other candidate
            pending_txns.remove_first_pending().unwrap()
        };
        while sliding_window.num_txns() < num_transactions {
            let txn = next_to_add(&mut sliding_window);
            sliding_window.add_transaction(txn)
        }
        sliding_window.finalize()
    }
}

impl SenderAwareShuffler {
    pub fn new(conflict_window_size: usize) -> Self {
        Self {
            conflict_window_size,
        }
    }
}

/// A structure to maintain a set of transactions that are pending to be added to the block indexed by
/// the sender. For a particular sender, relative ordering of transactions are maintained,
/// so that the final block preserves the ordering of transactions by sender. It also maintains a vector
/// to preserve the original order of the transactions. This is needed in case we can't find
/// any non-conflicting transactions and we need to add the first pending transaction to the block.
struct PendingTransactions {
    txns_by_senders: HashMap<AccountAddress, VecDeque<SignedTransaction>>,
    // Transactions are kept in the original order. This is not kept in sync with pending transactions,
    // so this can contain a bunch of transactions that are already added to the block.
    ordered_txns: VecDeque<SignedTransaction>,
}

impl PendingTransactions {
    pub fn new() -> Self {
        Self {
            txns_by_senders: HashMap::new(),
            ordered_txns: VecDeque::new(),
        }
    }

    pub fn add_transaction(&mut self, txn: SignedTransaction) {
        self.ordered_txns.push_back(txn.clone());
        self.txns_by_senders
            .entry(txn.sender())
            .or_default()
            .push_back(txn);
    }

    /// Removes the first pending transaction from the sender. Please note that the transaction is not
    /// removed from the `ordered_txns`, so the `ordered_txns` will contain a set of transactions that
    /// are removed from pending transactions already.
    pub fn remove_pending_from_sender(
        &mut self,
        sender: AccountAddress,
    ) -> Option<SignedTransaction> {
        self.txns_by_senders
            .get_mut(&sender)
            .and_then(|txns| txns.pop_front())
    }

    pub fn remove_first_pending(&mut self) -> Option<SignedTransaction> {
        while let Some(txn) = self.ordered_txns.pop_front() {
            let sender = txn.sender();
            // We don't remove the txns from ordered_txns when remove_pending_from_sender is called.
            // So it is possible that the ordered_txns has some transactions that are not pending
            // anymore.
            if Some(txn).as_ref() == self.txns_by_senders.get(&sender).unwrap().front() {
                return self.remove_pending_from_sender(sender);
            }
        }
        None
    }
}

/// A stateful data structure maintained by the transaction shuffler during shuffling. On a
/// high level, it maintains a sliding window of the conflicting transactions, which helps the payload
/// generator include a set of transactions which are non-conflicting with each other within a particular
/// window size.
struct SlidingWindowState {
    // Please note that the start index can be negative in case the window size is larger than the
    // end_index.
    start_index: i64,
    // Hashmap of senders to the number of transactions included in the window for the corresponding
    // sender.
    senders_in_window: HashMap<AccountAddress, usize>,
    // Partially ordered transactions, needs to be updated every time add_transactions is called.
    txns: Vec<SignedTransaction>,
}

impl SlidingWindowState {
    pub fn new(window_size: usize, num_txns: usize) -> Self {
        Self {
            start_index: -(window_size as i64),
            senders_in_window: HashMap::new(),
            txns: Vec::with_capacity(num_txns),
        }
    }

    /// Slides the current window. Essentially, it increments the start_index and
    /// updates the senders_in_window map if start_index is greater than 0
    pub fn add_transaction(&mut self, txn: SignedTransaction) {
        if self.start_index >= 0 {
            // if the start_index is negative, then no sender falls out of the window.
            let sender = self
                .txns
                .get(self.start_index as usize)
                .expect("Transaction expected")
                .sender();
            self.senders_in_window
                .entry(sender)
                .and_modify(|count| *count -= 1);
        }
        let count = self
            .senders_in_window
            .entry(txn.sender())
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
        let prev_start_index = self.start_index - 1;
        if prev_start_index >= 0 {
            let last_sender = self.txns.get(prev_start_index as usize).unwrap().sender();
            if *self.senders_in_window.get(&last_sender).unwrap() == 0 {
                return Some(last_sender);
            }
        }
        None
    }

    pub fn num_txns(&self) -> usize {
        self.txns.len()
    }

    pub fn finalize(self) -> Vec<SignedTransaction> {
        NUM_SENDERS_IN_BLOCK.set(self.senders_in_window.len() as f64);
        self.txns
    }
}

#[cfg(test)]
mod tests {
    use crate::transaction_shuffler::{sender_aware::SenderAwareShuffler, TransactionShuffler};
    use aptos_crypto::{ed25519::Ed25519PrivateKey, PrivateKey, SigningKey, Uniform};
    use aptos_types::{
        chain_id::ChainId,
        transaction::{RawTransaction, Script, SignedTransaction, TransactionPayload},
    };
    use move_core_types::account_address::AccountAddress;
    use rand::{rngs::OsRng, Rng};
    use std::{
        collections::{HashMap, HashSet},
        time::Instant,
    };

    fn create_signed_transaction(num_transactions: usize) -> Vec<SignedTransaction> {
        let private_key = Ed25519PrivateKey::generate_for_testing();
        let public_key = private_key.public_key();
        let sender = AccountAddress::random();

        let mut transactions = Vec::new();

        for i in 0..num_transactions {
            let transaction_payload =
                TransactionPayload::Script(Script::new(vec![], vec![], vec![]));
            let raw_transaction = RawTransaction::new(
                sender,
                i as u64,
                transaction_payload,
                0,
                0,
                0,
                ChainId::new(10),
            );
            let signed_transaction = SignedTransaction::new(
                raw_transaction.clone(),
                public_key.clone(),
                private_key.sign(&raw_transaction).unwrap(),
            );
            transactions.push(signed_transaction)
        }
        transactions
    }

    #[test]
    fn test_single_user_txns() {
        for num_txns in [1, 5, 50, 500] {
            let txns = create_signed_transaction(num_txns);
            let txn_shuffer = SenderAwareShuffler::new(10);
            let optimized_txns = txn_shuffer.shuffle(txns.clone());
            assert_eq!(txns.len(), optimized_txns.len());
            // Assert that ordering is unchanged in case of single sender block
            assert_eq!(txns, optimized_txns)
        }
    }

    #[test]
    fn test_unique_sender_txns() {
        for num_senders in [1, 5, 50, 500] {
            let mut txns = Vec::new();
            let mut senders = Vec::new();
            for _ in 0..num_senders {
                let mut sender_txns = create_signed_transaction(1);
                senders.push(sender_txns.first().unwrap().sender());
                txns.append(&mut sender_txns);
            }
            let txn_shuffer = SenderAwareShuffler::new(10);
            let optimized_txns = txn_shuffer.shuffle(txns.clone());
            assert_eq!(txns.len(), optimized_txns.len());
            // Assert that the ordering is unchanged in case of unique senders txns.
            assert_eq!(txns, optimized_txns)
        }
    }

    #[test]
    fn test_perfect_shuffling() {
        let num_senders = 50;
        let mut txns = Vec::new();
        let mut senders = Vec::new();
        for _ in 0..num_senders {
            let mut sender_txns = create_signed_transaction(10);
            senders.push(sender_txns.first().unwrap().sender());
            txns.append(&mut sender_txns);
        }

        let txn_shuffler = SenderAwareShuffler::new(num_senders - 1);
        let optimized_txns = txn_shuffler.shuffle(txns.clone());
        assert_eq!(txns.len(), optimized_txns.len());
        let mut sender_index = 0;
        for txn in optimized_txns {
            assert_eq!(&txn.sender(), senders.get(sender_index).unwrap());
            sender_index = (sender_index + 1) % senders.len()
        }
    }

    #[test]
    fn test_shuffling_benchmark() {
        let num_senders = 200;
        let mut txns = Vec::new();
        let mut senders = Vec::new();
        for _ in 0..num_senders {
            let mut sender_txns = create_signed_transaction(10);
            senders.push(sender_txns.first().unwrap().sender());
            txns.append(&mut sender_txns);
        }

        let now = Instant::now();
        let txn_shuffler = SenderAwareShuffler::new(32);
        let optimized_txns = txn_shuffler.shuffle(txns.clone());
        println!("elapsed time is {}", now.elapsed().as_millis());
        assert_eq!(txns.len(), optimized_txns.len());
    }

    #[test]
    fn test_same_sender_relative_order() {
        let mut rng = OsRng;
        let max_txn_per_sender = 100;
        let num_senders = 100;
        let mut orig_txns = Vec::new();
        let mut orig_txns_by_sender = HashMap::new();
        for _ in 0..num_senders {
            let mut sender_txns = create_signed_transaction(rng.gen_range(1, max_txn_per_sender));
            orig_txns_by_sender.insert(sender_txns.first().unwrap().sender(), sender_txns.clone());
            orig_txns.append(&mut sender_txns);
        }
        let txn_shuffler = SenderAwareShuffler::new(num_senders - 1);
        let optimized_txns = txn_shuffler.shuffle(orig_txns.clone());
        let mut optimized_txns_by_sender = HashMap::new();
        for txn in optimized_txns {
            optimized_txns_by_sender
                .entry(txn.sender())
                .or_insert_with(Vec::new)
                .push(txn);
        }

        for (sender, orig_txns) in orig_txns_by_sender {
            assert_eq!(optimized_txns_by_sender.get(&sender).unwrap(), &orig_txns)
        }
    }

    #[test]
    // S1_1, S2_1, S3_1, S3_2
    // with conflict_window_size=3, should return (keep the order, fairness to early transactions):
    // S1_1, S2_1, S3_1, S3_2
    fn test_3_sender_shuffling() {
        let mut orig_txns = Vec::new();
        let sender1_txns = create_signed_transaction(1);
        let sender2_txns = create_signed_transaction(1);
        let sender3_txns = create_signed_transaction(2);
        orig_txns.extend(sender1_txns.clone());
        orig_txns.extend(sender2_txns.clone());
        orig_txns.extend(sender3_txns.clone());
        let txn_shuffler = SenderAwareShuffler::new(3);
        let optimized_txns = txn_shuffler.shuffle(orig_txns);
        assert_eq!(
            optimized_txns.first().unwrap(),
            sender1_txns.first().unwrap()
        );
        assert_eq!(
            optimized_txns.get(1).unwrap(),
            sender2_txns.first().unwrap()
        );
        assert_eq!(
            optimized_txns.get(2).unwrap(),
            sender3_txns.first().unwrap()
        );
        assert_eq!(optimized_txns.get(3).unwrap(), sender3_txns.get(1).unwrap());
    }

    #[test]
    // S1_1, S2_1, S1_2, S3_1, S4_1, S5_1
    // with conflict_window_size=3, should return
    // (we separate transactions from same sender, even if they are not consecutive):
    // S1_1, S2_1, S3_1, S4_1, S1_2, S5_1
    fn test_5_sender_shuffling() {
        let mut orig_txns = Vec::new();
        let sender1_txns = create_signed_transaction(2);
        let sender2_txns = create_signed_transaction(1);
        let sender3_txns = create_signed_transaction(1);
        let sender4_txns = create_signed_transaction(1);
        let sender5_txns = create_signed_transaction(1);
        orig_txns.extend(sender1_txns.clone());
        orig_txns.extend(sender2_txns.clone());
        orig_txns.extend(sender3_txns.clone());
        orig_txns.extend(sender4_txns.clone());
        orig_txns.extend(sender5_txns.clone());
        let txn_shuffler = SenderAwareShuffler::new(3);
        let optimized_txns = txn_shuffler.shuffle(orig_txns);
        assert_eq!(
            optimized_txns.first().unwrap(),
            sender1_txns.first().unwrap()
        );
        assert_eq!(
            optimized_txns.get(1).unwrap(),
            sender2_txns.first().unwrap()
        );
        assert_eq!(
            optimized_txns.get(2).unwrap(),
            sender3_txns.first().unwrap()
        );
        assert_eq!(
            optimized_txns.get(3).unwrap(),
            sender4_txns.first().unwrap()
        );
        assert_eq!(optimized_txns.get(4).unwrap(), sender1_txns.get(1).unwrap());
        assert_eq!(
            optimized_txns.get(5).unwrap(),
            sender5_txns.first().unwrap()
        );
    }

    #[test]
    // S1_1, S1_2, S2_1, S3_1, S3_2, S4_1, S5_1, S6_1
    // with conflict_window_size=3, should return (each batches are separated from the point they appear on):
    // S1_1, S2_1, S3_1, S4_1, S1_2, S5_1, S3_2, S6_1
    fn test_6_sender_shuffling() {
        let mut orig_txns = Vec::new();
        let sender1_txns = create_signed_transaction(2);
        let sender2_txns = create_signed_transaction(1);
        let sender3_txns = create_signed_transaction(2);
        let sender4_txns = create_signed_transaction(1);
        let sender5_txns = create_signed_transaction(1);
        let sender6_txns = create_signed_transaction(1);
        orig_txns.extend(sender1_txns.clone());
        orig_txns.extend(sender2_txns.clone());
        orig_txns.extend(sender3_txns.clone());
        orig_txns.extend(sender4_txns.clone());
        orig_txns.extend(sender5_txns.clone());
        orig_txns.extend(sender6_txns.clone());
        let txn_shuffler = SenderAwareShuffler::new(3);
        let optimized_txns = txn_shuffler.shuffle(orig_txns);
        assert_eq!(
            optimized_txns.first().unwrap(),
            sender1_txns.first().unwrap()
        );
        assert_eq!(
            optimized_txns.get(1).unwrap(),
            sender2_txns.first().unwrap()
        );
        assert_eq!(
            optimized_txns.get(2).unwrap(),
            sender3_txns.first().unwrap()
        );
        assert_eq!(
            optimized_txns.get(3).unwrap(),
            sender4_txns.first().unwrap()
        );
        assert_eq!(optimized_txns.get(4).unwrap(), sender1_txns.get(1).unwrap());
        assert_eq!(
            optimized_txns.get(5).unwrap(),
            sender5_txns.first().unwrap()
        );
        assert_eq!(optimized_txns.get(6).unwrap(), sender3_txns.get(1).unwrap());
        assert_eq!(
            optimized_txns.get(7).unwrap(),
            sender6_txns.first().unwrap()
        );
    }

    #[test]
    fn test_random_shuffling() {
        let mut rng = OsRng;
        let max_senders = 50;
        let max_txn_per_sender = 100;
        let num_senders = rng.gen_range(1, max_senders);
        let mut orig_txns = Vec::new();
        let mut senders = Vec::new();
        let mut orig_txn_set = HashSet::new();
        for _ in 0..num_senders {
            let mut sender_txns = create_signed_transaction(rng.gen_range(1, max_txn_per_sender));
            senders.push(sender_txns.first().unwrap().sender());
            orig_txns.append(&mut sender_txns);
        }
        for txn in orig_txns.clone() {
            orig_txn_set.insert(txn.into_raw_transaction());
        }

        let txn_shuffler = SenderAwareShuffler::new(num_senders - 1);
        let optimized_txns = txn_shuffler.shuffle(orig_txns.clone());
        let mut optimized_txn_set = HashSet::new();
        assert_eq!(orig_txns.len(), optimized_txns.len());

        for optimized_txn in optimized_txns {
            assert!(orig_txn_set.contains(&optimized_txn.clone().into_raw_transaction()));
            optimized_txn_set.insert(optimized_txn.into_raw_transaction());
        }

        for orig_txn in orig_txns {
            assert!(optimized_txn_set.contains(&orig_txn.into_raw_transaction()));
        }
    }

    #[test]
    fn test_shuffling_zero_conflict_window() {
        let mut rng = OsRng;
        let max_senders = 50;
        let max_txn_per_sender = 100;
        let num_senders = rng.gen_range(1, max_senders);
        let mut orig_txns = Vec::new();
        let mut senders = Vec::new();
        for _ in 0..num_senders {
            let mut sender_txns = create_signed_transaction(rng.gen_range(1, max_txn_per_sender));
            senders.push(sender_txns.first().unwrap().sender());
            orig_txns.append(&mut sender_txns);
        }

        let txn_shuffler = SenderAwareShuffler::new(0);
        let optimized_txns = txn_shuffler.shuffle(orig_txns.clone());
        assert_eq!(orig_txns.len(), optimized_txns.len());
        // Assert that the ordering is unchanged in case of unique senders txns.
        assert_eq!(orig_txns, optimized_txns);
    }
}
