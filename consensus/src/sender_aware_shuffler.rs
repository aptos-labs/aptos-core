use crate::{
    counters::{NUM_SENDERS_IN_BLOCK, TXN_SHUFFLE_SECONDS},
    transaction_shuffler::TransactionShuffler,
};
use aptos_types::transaction::SignedTransaction;
use move_core_types::account_address::AccountAddress;
use std::{
    cmp::min,
    collections::{HashMap, VecDeque},
};

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
/// In order to make the look up faster, it creates an index of contiguous set of transactions by
/// senders. This makes searching for non-conflicting transactions much faster as compared to brute
/// force approach.
///
/// Another optimization introduced is the `look_ahead_sender_window` - this limits the number of senders
/// to check to find non-conflicting transactions before giving up. This is needed to ensure the shuffling
/// is not O(n) even in the worst case.

pub struct SenderAwareShuffler {
    conflict_window_size: usize,
    look_ahead_sender_window: usize,
}

impl TransactionShuffler for SenderAwareShuffler {
    fn shuffle(&self, txns: Vec<SignedTransaction>) -> Vec<SignedTransaction> {
        let _timer = TXN_SHUFFLE_SECONDS.start_timer();

        // maintains the intermediate state of the shuffled transactions
        let mut sliding_window = SlidingWindowState::new(self.conflict_window_size);
        let num_transactions = txns.len();
        let mut candidate_txn_chunks = self.prepare_txn_chunk_by_senders(txns);
        for _ in 0..num_transactions {
            let max_lookup = min(self.look_ahead_sender_window, candidate_txn_chunks.len());
            // These are the chunks, which are evaluated in each iteration. These needs to be pushed
            // back to preserve the original order of the transactions after each iteration.
            let mut to_be_pushed_back_chunks = VecDeque::new();
            let mut candidate_found = false;
            let mut current_sender_index = 0;
            while current_sender_index < max_lookup {
                let mut candidate_chunk = candidate_txn_chunks
                    .pop_front()
                    .expect("Expected transaction chunk in the candidate transaction chunk");
                if !sliding_window.has_conflict_in_window(&candidate_chunk.sender()) {
                    sliding_window.add_transaction(
                        candidate_chunk
                            .remove_transaction()
                            .expect("Expected transaction in candidate chunk"),
                    );
                    candidate_found = true;
                    if !candidate_chunk.is_empty() {
                        // Non-empty chunk needs to be pushed back again.
                        to_be_pushed_back_chunks.push_front(candidate_chunk)
                    }
                    break;
                }
                current_sender_index += 1;
                to_be_pushed_back_chunks.push_front(candidate_chunk);
            }

            if !candidate_found {
                // We didn't find any non-conflicting txn in the look up window. In this case, just
                // add the first candidate to the block. Which can be found by popping the back of the
                // `to_be_pushed_back_chunks`
                let mut chunk = to_be_pushed_back_chunks
                    .pop_back()
                    .expect("Expected non empty vector");
                sliding_window.add_transaction(
                    chunk
                        .remove_transaction()
                        .expect("Empty chunk not expected"),
                );
                if !chunk.is_empty() {
                    to_be_pushed_back_chunks.push_back(chunk)
                }
            }

            // Add the remaining txns to the candidate txns list in the original order.
            let mut chunk = to_be_pushed_back_chunks.pop_front();
            while chunk.is_some() {
                candidate_txn_chunks.push_front(chunk.unwrap());
                chunk = to_be_pushed_back_chunks.pop_front();
            }
        }
        sliding_window.finalize()
    }
}

impl SenderAwareShuffler {
    pub fn new(conflict_window_size: usize, look_ahead_sender_window: usize) -> Self {
        Self {
            conflict_window_size,
            look_ahead_sender_window,
        }
    }

    // Prepares the index of transaction chunks by sender
    fn prepare_txn_chunk_by_senders(
        &self,
        txns: Vec<SignedTransaction>,
    ) -> VecDeque<TransactionsChunkBySender> {
        let mut candidate_txn_chunks = VecDeque::new();
        let mut prev_sender = None;
        for txn in txns {
            let current_sender = txn.sender();
            if prev_sender != Some(current_sender) {
                let txn_chunk = TransactionsChunkBySender::new(current_sender);
                candidate_txn_chunks.push_back(txn_chunk);
            }
            candidate_txn_chunks
                .back_mut()
                .unwrap()
                .add_transaction(txn);
            prev_sender = Some(current_sender)
        }
        candidate_txn_chunks
    }
}

/// A state full data structure maintained by the transaction shuffer during shuffling. On a
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
    pub fn new(window_size: usize) -> Self {
        Self {
            start_index: -(window_size as i64),
            senders_in_window: HashMap::new(),
            txns: Vec::new(),
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

    pub fn has_conflict_in_window(&self, addr: &AccountAddress) -> bool {
        self.senders_in_window
            .get(addr)
            .map_or(false, |count| *count != 0)
    }

    pub fn finalize(self) -> Vec<SignedTransaction> {
        NUM_SENDERS_IN_BLOCK.set(self.senders_in_window.len() as f64);
        self.txns
    }
}

/// This represents a contiguous chunk of transactions in a block grouped by the same sender.
struct TransactionsChunkBySender {
    sender: AccountAddress,
    transactions: VecDeque<SignedTransaction>,
}

impl TransactionsChunkBySender {
    pub fn new(sender: AccountAddress) -> Self {
        Self {
            sender,
            transactions: VecDeque::new(),
        }
    }

    pub fn add_transaction(&mut self, txn: SignedTransaction) {
        self.transactions.push_back(txn);
    }

    pub fn remove_transaction(&mut self) -> Option<SignedTransaction> {
        self.transactions.pop_front()
    }

    pub fn is_empty(&self) -> bool {
        self.transactions.is_empty()
    }

    pub fn sender(&self) -> AccountAddress {
        self.sender
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        sender_aware_shuffler::SenderAwareShuffler, transaction_shuffler::TransactionShuffler,
    };
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
        for i in [1, 5, 50, 500] {
            let txns = create_signed_transaction(i);
            let txn_shuffer = SenderAwareShuffler::new(10, 10);
            let optimized_txns = txn_shuffer.shuffle(txns.clone());
            assert_eq!(txns.len(), optimized_txns.len());
            // Assert that ordering is unchanged in case of single sender block
            assert_eq!(txns, optimized_txns)
        }
    }

    #[test]
    fn test_unique_sender_txns() {
        let num_senders = 500;
        let mut txns = Vec::new();
        let mut senders = Vec::new();
        for _ in 0..num_senders {
            let mut sender_txns = create_signed_transaction(1);
            senders.push(sender_txns.get(0).unwrap().sender());
            txns.append(&mut sender_txns);
        }
        let txn_shuffer = SenderAwareShuffler::new(10, 10);
        let optimized_txns = txn_shuffer.shuffle(txns.clone());
        assert_eq!(txns.len(), optimized_txns.len());
        // Assert that the ordering is unchanged in case of unique senders txns.
        assert_eq!(txns, optimized_txns)
    }

    #[test]
    fn test_perfect_shuffling() {
        let num_senders = 50;
        let mut txns = Vec::new();
        let mut senders = Vec::new();
        for _ in 0..num_senders {
            let mut sender_txns = create_signed_transaction(10);
            senders.push(sender_txns.get(0).unwrap().sender());
            txns.append(&mut sender_txns);
        }

        let txn_shuffler = SenderAwareShuffler::new(num_senders - 1, 1000);
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
            senders.push(sender_txns.get(0).unwrap().sender());
            txns.append(&mut sender_txns);
        }

        let now = Instant::now();
        let txn_shuffler = SenderAwareShuffler::new(32, 256);
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
            orig_txns_by_sender.insert(sender_txns.get(0).unwrap().sender(), sender_txns.clone());
            orig_txns.append(&mut sender_txns);
        }
        let txn_shuffler = SenderAwareShuffler::new(num_senders - 1, 100);
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
        let txn_shuffler = SenderAwareShuffler::new(3, 100);
        let optimized_txns = txn_shuffler.shuffle(orig_txns);
        assert_eq!(optimized_txns.get(0).unwrap(), sender1_txns.get(0).unwrap());
        assert_eq!(optimized_txns.get(1).unwrap(), sender2_txns.get(0).unwrap());
        assert_eq!(optimized_txns.get(2).unwrap(), sender3_txns.get(0).unwrap());
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
        let txn_shuffler = SenderAwareShuffler::new(3, 100);
        let optimized_txns = txn_shuffler.shuffle(orig_txns);
        assert_eq!(optimized_txns.get(0).unwrap(), sender1_txns.get(0).unwrap());
        assert_eq!(optimized_txns.get(1).unwrap(), sender2_txns.get(0).unwrap());
        assert_eq!(optimized_txns.get(2).unwrap(), sender3_txns.get(0).unwrap());
        assert_eq!(optimized_txns.get(3).unwrap(), sender4_txns.get(0).unwrap());
        assert_eq!(optimized_txns.get(4).unwrap(), sender1_txns.get(1).unwrap());
        assert_eq!(optimized_txns.get(5).unwrap(), sender5_txns.get(0).unwrap());
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
        let txn_shuffler = SenderAwareShuffler::new(3, 100);
        let optimized_txns = txn_shuffler.shuffle(orig_txns);
        assert_eq!(optimized_txns.get(0).unwrap(), sender1_txns.get(0).unwrap());
        assert_eq!(optimized_txns.get(1).unwrap(), sender2_txns.get(0).unwrap());
        assert_eq!(optimized_txns.get(2).unwrap(), sender3_txns.get(0).unwrap());
        assert_eq!(optimized_txns.get(3).unwrap(), sender4_txns.get(0).unwrap());
        assert_eq!(optimized_txns.get(4).unwrap(), sender1_txns.get(1).unwrap());
        assert_eq!(optimized_txns.get(5).unwrap(), sender5_txns.get(0).unwrap());
        assert_eq!(optimized_txns.get(6).unwrap(), sender3_txns.get(1).unwrap());
        assert_eq!(optimized_txns.get(7).unwrap(), sender6_txns.get(0).unwrap());
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
            senders.push(sender_txns.get(0).unwrap().sender());
            orig_txns.append(&mut sender_txns);
        }
        for txn in orig_txns.clone() {
            orig_txn_set.insert(txn.into_raw_transaction());
        }

        let txn_shuffler = SenderAwareShuffler::new(num_senders - 1, 100);
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
}
