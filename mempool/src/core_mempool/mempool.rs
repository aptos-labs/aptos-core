// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! Mempool is used to track transactions which have been submitted but not yet
//! agreed upon.
use crate::{
    core_mempool::{
        index::TxnPointer,
        transaction::{MempoolTransaction, TimelineState},
        transaction_store::TransactionStore,
    },
    counters,
    logging::{LogEntry, LogSchema, TxnsLog},
};
use aptos_config::config::NodeConfig;
use aptos_crypto::HashValue;
use aptos_logger::prelude::*;
use aptos_types::{
    account_address::AccountAddress,
    account_config::AccountSequenceInfo,
    mempool_status::{MempoolStatus, MempoolStatusCode},
    transaction::SignedTransaction,
};
use std::{
    collections::HashSet,
    time::{Duration, SystemTime},
};

pub struct Mempool {
    // Stores the metadata of all transactions in mempool (of all states).
    transactions: TransactionStore,

    pub system_transaction_timeout: Duration,
}

impl Mempool {
    pub fn new(config: &NodeConfig) -> Self {
        Mempool {
            transactions: TransactionStore::new(&config.mempool),
            system_transaction_timeout: Duration::from_secs(
                config.mempool.system_transaction_timeout_secs,
            ),
        }
    }

    /// This function will be called once the transaction has been stored.
    pub(crate) fn remove_transaction(
        &mut self,
        sender: &AccountAddress,
        sequence_number: u64,
        is_rejected: bool,
    ) {
        trace!(
            LogSchema::new(LogEntry::RemoveTxn).txns(TxnsLog::new_txn(*sender, sequence_number)),
            is_rejected = is_rejected
        );
        let metric_label = if is_rejected {
            counters::COMMIT_REJECTED_LABEL
        } else {
            counters::COMMIT_ACCEPTED_LABEL
        };
        self.log_latency(*sender, sequence_number, metric_label);

        self.transactions
            .remove(sender, sequence_number, is_rejected);
    }

    fn log_latency(&self, account: AccountAddress, sequence_number: u64, metric: &str) {
        if let Some(&insertion_time) = self
            .transactions
            .get_insertion_time(&account, sequence_number)
        {
            if let Ok(time_delta) = SystemTime::now().duration_since(insertion_time) {
                counters::CORE_MEMPOOL_TXN_COMMIT_LATENCY
                    .with_label_values(&[metric])
                    .observe(time_delta.as_secs_f64());
            }
        }
    }

    pub(crate) fn get_by_hash(&self, hash: HashValue) -> Option<SignedTransaction> {
        self.transactions.get_by_hash(hash)
    }

    /// Used to add a transaction to the Mempool.
    /// Performs basic validation: checks account's sequence number.
    pub(crate) fn add_txn(
        &mut self,
        txn: SignedTransaction,
        ranking_score: u64,
        sequence_info: AccountSequenceInfo,
        timeline_state: TimelineState,
    ) -> MempoolStatus {
        let db_sequence_number = sequence_info.min_seq();
        trace!(
            LogSchema::new(LogEntry::AddTxn)
                .txns(TxnsLog::new_txn(txn.sender(), txn.sequence_number())),
            committed_seq_number = db_sequence_number
        );

        // don't accept old transactions (e.g. seq is less than account's current seq_number)
        if txn.sequence_number() < db_sequence_number {
            return MempoolStatus::new(MempoolStatusCode::InvalidSeqNumber).with_message(format!(
                "transaction sequence number is {}, current sequence number is  {}",
                txn.sequence_number(),
                db_sequence_number,
            ));
        }

        let now = SystemTime::now();
        let expiration_time =
            aptos_infallible::duration_since_epoch_at(&now) + self.system_transaction_timeout;

        let txn_info = MempoolTransaction::new(
            txn,
            expiration_time,
            ranking_score,
            timeline_state,
            AccountSequenceInfo::Sequential(db_sequence_number),
            now,
        );

        self.transactions.insert(txn_info)
    }

    /// Fetches next block of transactions for consensus.
    /// `batch_size` - size of requested block.
    /// `seen_txns` - transactions that were sent to Consensus but were not committed yet,
    ///  mempool should filter out such transactions.
    #[allow(clippy::explicit_counter_loop)]
    pub(crate) fn get_batch(
        &self,
        max_txns: u64,
        max_bytes: u64,
        mut seen: HashSet<TxnPointer>,
    ) -> Vec<SignedTransaction> {
        let mut result = vec![];
        // Helper DS. Helps to mitigate scenarios where account submits several transactions
        // with increasing gas price (e.g. user submits transactions with sequence number 1, 2
        // and gas_price 1, 10 respectively)
        // Later txn has higher gas price and will be observed first in priority index iterator,
        // but can't be executed before first txn. Once observed, such txn will be saved in
        // `skipped` DS and rechecked once it's ancestor becomes available
        let mut skipped = HashSet::new();
        let mut total_bytes = 0;
        let seen_size = seen.len();
        let mut txn_walked = 0usize;
        // iterate over the queue of transactions based on gas price
        'main: for txn in self.transactions.iter_queue() {
            txn_walked += 1;
            if seen.contains(&TxnPointer::from(txn)) {
                continue;
            }
            let tx_seq = txn.sequence_number.transaction_sequence_number;
            let account_sequence_number = self.transactions.get_sequence_number(&txn.address);
            let seen_previous = tx_seq > 0 && seen.contains(&(txn.address, tx_seq - 1));
            // include transaction if it's "next" for given account or
            // we've already sent its ancestor to Consensus.
            if seen_previous || account_sequence_number == Some(&tx_seq) {
                let ptr = TxnPointer::from(txn);
                seen.insert(ptr);
                result.push(ptr);
                if (result.len() as u64) == max_txns {
                    break;
                }

                // check if we can now include some transactions
                // that were skipped before for given account
                let mut skipped_txn = (txn.address, tx_seq + 1);
                while skipped.contains(&skipped_txn) {
                    seen.insert(skipped_txn);
                    result.push(skipped_txn);
                    if (result.len() as u64) == max_txns {
                        break 'main;
                    }
                    skipped_txn = (txn.address, skipped_txn.1 + 1);
                }
            } else {
                skipped.insert(TxnPointer::from(txn));
            }
        }
        let result_size = result.len();
        let mut block = Vec::with_capacity(result_size);
        for (address, seq) in result {
            if let Some(txn) = self.transactions.get(&address, seq) {
                let txn_size = txn.raw_txn_bytes_len();
                if total_bytes + txn_size > max_bytes as usize {
                    break;
                }
                total_bytes += txn_size;
                block.push(txn);
            }
        }

        debug!(
            LogSchema::new(LogEntry::GetBlock),
            seen_consensus = seen_size,
            walked = txn_walked,
            seen_after = seen.len(),
            result_size = result_size,
            block_size = block.len(),
            byte_size = total_bytes,
        );

        counters::mempool_service_transactions(counters::GET_BLOCK_LABEL, block.len());
        counters::MEMPOOL_SERVICE_BYTES_GET_BLOCK.observe(total_bytes as f64);
        for transaction in &block {
            self.log_latency(
                transaction.sender(),
                transaction.sequence_number(),
                counters::GET_BLOCK_STAGE_LABEL,
            );
        }
        block
    }

    /// Periodic core mempool garbage collection.
    /// Removes all expired transactions and clears expired entries in metrics
    /// cache and sequence number cache.
    pub(crate) fn gc(&mut self) {
        let now = aptos_infallible::duration_since_epoch();
        self.transactions.gc_by_system_ttl(now);
    }

    /// Garbage collection based on client-specified expiration time.
    pub(crate) fn gc_by_expiration_time(&mut self, block_time: Duration) {
        self.transactions.gc_by_expiration_time(block_time);
    }

    /// Returns block of transactions and new last_timeline_id.
    pub(crate) fn read_timeline(
        &self,
        timeline_id: u64,
        count: usize,
    ) -> (Vec<SignedTransaction>, u64) {
        self.transactions.read_timeline(timeline_id, count)
    }

    /// Read transactions from timeline from `start_id` (exclusive) to `end_id` (inclusive).
    pub(crate) fn timeline_range(&self, start_id: u64, end_id: u64) -> Vec<SignedTransaction> {
        self.transactions.timeline_range(start_id, end_id)
    }

    pub fn gen_snapshot(&self) -> TxnsLog {
        self.transactions.gen_snapshot()
    }

    #[cfg(test)]
    pub fn get_parking_lot_size(&self) -> usize {
        self.transactions.get_parking_lot_size()
    }

    #[cfg(test)]
    pub fn get_transaction_store(&self) -> &TransactionStore {
        &self.transactions
    }
}
