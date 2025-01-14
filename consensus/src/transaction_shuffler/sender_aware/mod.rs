// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub(crate) mod config;
pub(crate) mod iterator;
pub(crate) mod pending_transactions;
pub(crate) mod sliding_window_state;

use crate::transaction_shuffler::{
    sender_aware::{
        config::Config, iterator::ShuffledTransactionIterator,
        pending_transactions::PendingTransactions, sliding_window_state::SlidingWindowState,
    },
    TransactionShuffler,
};
use aptos_types::transaction::{
    sender_aware::SenderAwareTransaction,
    signature_verified_transaction::SignatureVerifiedTransaction, SignedTransaction,
};
use std::{collections::VecDeque, fmt::Debug};

pub struct SenderAwareShuffler {
    pub config: Config,
}

impl SenderAwareShuffler {
    fn next_to_add<Txn: SenderAwareTransaction + Debug + Clone + PartialEq>(
        &self,
        sliding_window_state: &mut SlidingWindowState<Txn>,
        pending_txns: &mut PendingTransactions<Txn>,
        orig_txns: &mut VecDeque<Txn>,
    ) -> Txn {
        // First check if we have a sender dropped off of conflict window in previous step, if so,
        // we try to find pending transaction from the corresponding sender and add it to the block.
        if let Some(sender) = sliding_window_state.last_dropped_sender() {
            if let Some(txn) = pending_txns.remove_pending_from_sender(sender) {
                sliding_window_state.add_transaction(txn.clone());
                return txn;
            }
        }
        // If we can't find any transaction from a sender dropped off of conflict window, then
        // iterate through the original transactions and try to find the next candidate
        while let Some(txn) = orig_txns.pop_front() {
            if !sliding_window_state.has_conflict(&txn.parse_sender()) {
                sliding_window_state.add_transaction(txn.clone());
                return txn;
            }
            pending_txns.add_transaction(txn);
        }

        // If we can't find any candidate in above steps, then lastly
        // add pending transactions in the order if we can't find any other candidate
        let txn = pending_txns
            .remove_first_pending()
            .expect("Pending should return a transaction");
        sliding_window_state.add_transaction(txn.clone());
        txn
    }
}

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
impl TransactionShuffler for SenderAwareShuffler {
    fn shuffle(&self, txns: Vec<SignedTransaction>) -> Vec<SignedTransaction> {
        // Early return for performance reason if there are no transactions to shuffle
        if txns.is_empty() {
            return txns;
        }

        // handle the corner case of conflict window being 0, in which case we don't do any shuffling
        if self.config.conflict_window_size == 0 {
            return txns;
        }

        ShuffledTransactionIterator::new(self.config.clone(), txns).collect()
    }

    fn signed_transaction_iterator(
        &self,
        txns: Vec<SignedTransaction>,
    ) -> Box<dyn Iterator<Item = SignedTransaction> + 'static> {
        // Early return for performance reason if there are no transactions to shuffle
        if txns.is_empty() {
            return Box::new(txns.into_iter());
        }

        // handle the corner case of conflict window being 0, in which case we don't do any shuffling
        if self.config.conflict_window_size == 0 {
            return Box::new(txns.into_iter());
        }

        let iterator = ShuffledTransactionIterator::new(self.config.clone(), txns);
        Box::new(iterator)
    }

    fn signature_verified_transaction_iterator(
        &self,
        txns: Vec<SignatureVerifiedTransaction>,
    ) -> Box<dyn Iterator<Item = SignatureVerifiedTransaction> + 'static> {
        // Early return for performance reason if there are no transactions to shuffle
        if txns.is_empty() {
            return Box::new(txns.into_iter());
        }

        // handle the corner case of conflict window being 0, in which case we don't do any shuffling
        if self.config.conflict_window_size == 0 {
            return Box::new(txns.into_iter());
        }

        let iterator = ShuffledTransactionIterator::new(self.config.clone(), txns);
        Box::new(iterator)
    }
}

impl SenderAwareShuffler {
    pub fn new(conflict_window_size: usize) -> Self {
        Self {
            config: Config {
                conflict_window_size,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::transaction_shuffler::{sender_aware::SenderAwareShuffler, TransactionShuffler};
    use aptos_crypto::{ed25519::Ed25519PrivateKey, PrivateKey, SigningKey, Uniform};
    use aptos_types::{
        chain_id::ChainId,
        transaction::{
            sender_aware::SenderAwareTransaction, RawTransaction, Script, SignedTransaction,
            TransactionPayload,
        },
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
                senders.push(sender_txns.first().unwrap().parse_sender());
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
            senders.push(sender_txns.first().unwrap().parse_sender());
            txns.append(&mut sender_txns);
        }

        let txn_shuffler = SenderAwareShuffler::new(num_senders - 1);
        let optimized_txns = txn_shuffler.shuffle(txns.clone());
        assert_eq!(txns.len(), optimized_txns.len());
        let mut sender_index = 0;
        for txn in optimized_txns {
            assert_eq!(&txn.parse_sender(), senders.get(sender_index).unwrap());
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
            senders.push(sender_txns.first().unwrap().parse_sender());
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
            orig_txns_by_sender.insert(
                sender_txns.first().unwrap().parse_sender(),
                sender_txns.clone(),
            );
            orig_txns.append(&mut sender_txns);
        }
        let txn_shuffler = SenderAwareShuffler::new(num_senders - 1);
        let optimized_txns = txn_shuffler.shuffle(orig_txns.clone());
        let mut optimized_txns_by_sender = HashMap::new();
        for txn in optimized_txns {
            optimized_txns_by_sender
                .entry(txn.parse_sender())
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
            senders.push(sender_txns.first().unwrap().parse_sender());
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
            senders.push(sender_txns.first().unwrap().parse_sender());
            orig_txns.append(&mut sender_txns);
        }

        let txn_shuffler = SenderAwareShuffler::new(0);
        let optimized_txns = txn_shuffler.shuffle(orig_txns.clone());
        assert_eq!(orig_txns.len(), optimized_txns.len());
        // Assert that the ordering is unchanged in case of unique senders txns.
        assert_eq!(orig_txns, optimized_txns);
    }

    #[test]
    fn test_shuffling_iterator_zero_conflict_window() {
        let conflict_window_size = 0usize;
        let mut rng = OsRng;
        let max_senders = 50;
        let max_txn_per_sender = 100;
        let num_senders = rng.gen_range(1, max_senders);
        let mut orig_txns = Vec::new();
        let mut senders = Vec::new();
        for _ in 0..num_senders {
            let mut sender_txns = create_signed_transaction(rng.gen_range(1, max_txn_per_sender));
            senders.push(sender_txns.first().unwrap().parse_sender());
            orig_txns.append(&mut sender_txns);
        }

        let txn_shuffler = SenderAwareShuffler::new(conflict_window_size);

        // Shuffled Transaction checks
        let shuffled_txns = txn_shuffler.shuffle(orig_txns.clone());
        assert_eq!(orig_txns.len(), shuffled_txns.len());
        assert_eq!(orig_txns, shuffled_txns);

        // Shuffled Transaction Iterator checks
        let txn_shuffler = SenderAwareShuffler::new(conflict_window_size);
        let mut shuffled_iterator_txns: Vec<SignedTransaction> = Vec::new();
        for transaction in txn_shuffler.signed_transaction_iterator(orig_txns.clone()) {
            shuffled_iterator_txns.push(transaction)
        }

        assert_eq!(orig_txns.len(), shuffled_iterator_txns.len());
        assert_eq!(orig_txns, shuffled_iterator_txns)
    }

    /// Confirming that shuffle() and shuffle iterator return the same result for an empty
    /// vector of transactions
    #[test]
    fn test_shuffling_iterator_empty_vec() {
        let conflict_window_size = 10usize;
        let orig_txns = Vec::new();
        let txn_shuffler = SenderAwareShuffler::new(conflict_window_size);

        // Shuffled Transaction checks
        let shuffled_txns = txn_shuffler.shuffle(orig_txns.clone());
        assert_eq!(orig_txns.len(), shuffled_txns.len());
        assert_eq!(orig_txns, shuffled_txns);

        // Shuffled Transaction Iterator checks
        let txn_shuffler = SenderAwareShuffler::new(conflict_window_size);
        let mut shuffled_iterator_txns: Vec<SignedTransaction> = Vec::new();
        for transaction in txn_shuffler.signed_transaction_iterator(orig_txns.clone()) {
            shuffled_iterator_txns.push(transaction)
        }

        assert_eq!(orig_txns.len(), shuffled_iterator_txns.len());
        assert_eq!(orig_txns, shuffled_iterator_txns)
    }
}
