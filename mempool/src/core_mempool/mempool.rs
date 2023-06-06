// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Mempool is used to track transactions which have been submitted but not yet
//! agreed upon.
use crate::{
    core_mempool::{
        index::TxnPointer,
        transaction::{InsertionInfo, MempoolTransaction, TimelineState},
        transaction_store::TransactionStore,
    },
    counters,
    logging::{LogEntry, LogSchema, TxnsLog},
    shared_mempool::types::MultiBucketTimelineIndexIds,
};
use aptos_config::config::NodeConfig;
use aptos_consensus_types::common::TransactionInProgress;
use aptos_crypto::HashValue;
use aptos_logger::prelude::*;
use aptos_types::{
    account_address::AccountAddress,
    mempool_status::{MempoolStatus, MempoolStatusCode},
    transaction::SignedTransaction,
    vm_status::DiscardedVMStatus,
};
use std::{
    collections::{HashMap, HashSet},
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
    pub(crate) fn commit_transaction(&mut self, sender: &AccountAddress, sequence_number: u64) {
        trace!(
            LogSchema::new(LogEntry::RemoveTxn).txns(TxnsLog::new_txn(*sender, sequence_number)),
            is_rejected = false
        );
        self.log_latency(*sender, sequence_number, counters::COMMIT_ACCEPTED_LABEL);
        if let Some(ranking_score) = self.transactions.get_ranking_score(sender, sequence_number) {
            counters::core_mempool_txn_ranking_score(
                counters::REMOVE_LABEL,
                counters::COMMIT_ACCEPTED_LABEL,
                self.transactions.get_bucket(ranking_score),
                ranking_score,
            );
        }

        self.transactions
            .commit_transaction(sender, sequence_number);
    }

    fn log_reject_transaction(
        &self,
        sender: &AccountAddress,
        sequence_number: u64,
        reason_label: &'static str,
    ) {
        trace!(
            LogSchema::new(LogEntry::RemoveTxn).txns(TxnsLog::new_txn(*sender, sequence_number)),
            is_rejected = true,
            label = reason_label,
        );
        self.log_latency(*sender, sequence_number, reason_label);
        if let Some(ranking_score) = self.transactions.get_ranking_score(sender, sequence_number) {
            counters::core_mempool_txn_ranking_score(
                counters::REMOVE_LABEL,
                reason_label,
                self.transactions.get_bucket(ranking_score),
                ranking_score,
            );
        }
    }

    pub(crate) fn reject_transaction(
        &mut self,
        sender: &AccountAddress,
        sequence_number: u64,
        hash: &HashValue,
        reason: &DiscardedVMStatus,
    ) {
        if *reason == DiscardedVMStatus::SEQUENCE_NUMBER_TOO_NEW {
            self.log_reject_transaction(sender, sequence_number, counters::COMMIT_IGNORED_LABEL);
            // Do not remove the transaction from mempool
            return;
        }

        let label = if *reason == DiscardedVMStatus::SEQUENCE_NUMBER_TOO_OLD {
            counters::COMMIT_REJECTED_DUPLICATE_LABEL
        } else {
            counters::COMMIT_REJECTED_LABEL
        };
        self.log_reject_transaction(sender, sequence_number, label);
        self.transactions
            .reject_transaction(sender, sequence_number, hash);
    }

    pub(crate) fn log_txn_commit_latency(
        insertion_info: InsertionInfo,
        bucket: &str,
        stage: &'static str,
    ) {
        if let Ok(time_delta) = SystemTime::now().duration_since(insertion_info.insertion_time) {
            counters::core_mempool_txn_commit_latency(
                stage,
                insertion_info.submitted_by_label(),
                bucket,
                time_delta,
            );
        }
    }

    fn log_latency(&self, account: AccountAddress, sequence_number: u64, stage: &'static str) {
        if let Some((&insertion_info, bucket)) = self
            .transactions
            .get_insertion_info_and_bucket(&account, sequence_number)
        {
            Self::log_txn_commit_latency(insertion_info, bucket, stage);
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
        db_sequence_number: u64,
        timeline_state: TimelineState,
        client_submitted: bool,
    ) -> MempoolStatus {
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
            db_sequence_number,
            now,
            client_submitted,
        );

        let status = self.transactions.insert(txn_info);
        counters::core_mempool_txn_ranking_score(
            counters::INSERT_LABEL,
            status.code.to_string().as_str(),
            self.transactions.get_bucket(ranking_score),
            ranking_score,
        );
        status
    }

    /// Fetches next block of transactions for consensus.
    /// `return_non_full` - if false, only return transactions when max_txns or max_bytes is reached
    ///                     Should always be true for Quorum Store.
    /// `include_gas_upgraded` - Return transactions that had gas upgraded, even if they are in
    ///                          exclude_transactions. Should only be true for Quorum Store.
    /// `exclude_transactions` - transactions that were sent to Consensus but were not committed yet
    ///  mempool should filter out such transactions.
    #[allow(clippy::explicit_counter_loop)]
    pub(crate) fn get_batch(
        &self,
        max_txns: u64,
        max_bytes: u64,
        return_non_full: bool,
        include_gas_upgraded: bool,
        mut exclude_transactions: Vec<TransactionInProgress>,
    ) -> Vec<SignedTransaction> {
        // Sort, so per TxnPointer the highest gas will be in the map
        if include_gas_upgraded {
            exclude_transactions.sort();
        }
        let mut seen: HashMap<TxnPointer, u64> = exclude_transactions
            .iter()
            .map(|txn| (txn.summary, txn.gas_unit_price))
            .collect();
        // Do not exclude transactions that had a gas upgrade
        if include_gas_upgraded {
            let mut seen_and_upgraded = vec![];
            for (txn_pointer, new_gas) in self.transactions.get_gas_upgraded_txns() {
                if let Some(gas) = seen.get(txn_pointer) {
                    if *new_gas > *gas {
                        seen_and_upgraded.push(txn_pointer);
                    }
                }
            }
            for txn_pointer in seen_and_upgraded {
                seen.remove(txn_pointer);
            }
        }

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
            if seen.contains_key(&TxnPointer::from(txn)) {
                continue;
            }
            let tx_seq = txn.sequence_number.transaction_sequence_number;
            let account_sequence_number = self.transactions.get_sequence_number(&txn.address);
            let seen_previous =
                tx_seq > 0 && seen.contains_key(&TxnPointer::new(txn.address, tx_seq - 1));
            // include transaction if it's "next" for given account or
            // we've already sent its ancestor to Consensus.
            if seen_previous || account_sequence_number == Some(&tx_seq) {
                let ptr = TxnPointer::from(txn);
                seen.insert(ptr, txn.gas_ranking_score);
                result.push(ptr);
                if (result.len() as u64) == max_txns {
                    break;
                }

                // check if we can now include some transactions
                // that were skipped before for given account
                let mut skipped_txn = TxnPointer::new(txn.address, tx_seq + 1);
                while skipped.contains(&skipped_txn) {
                    seen.insert(skipped_txn, txn.gas_ranking_score);
                    result.push(skipped_txn);
                    if (result.len() as u64) == max_txns {
                        break 'main;
                    }
                    skipped_txn = TxnPointer::new(txn.address, skipped_txn.sequence_number + 1);
                }
            } else {
                skipped.insert(TxnPointer::from(txn));
            }
        }
        let result_size = result.len();
        let mut block = Vec::with_capacity(result_size);
        let mut full_bytes = false;
        for txn_pointer in result {
            if let Some((txn, ranking_score)) = self
                .transactions
                .get_with_ranking_score(&txn_pointer.sender, txn_pointer.sequence_number)
            {
                let txn_size = txn.raw_txn_bytes_len();
                if total_bytes + txn_size > max_bytes as usize {
                    full_bytes = true;
                    break;
                }
                total_bytes += txn_size;
                block.push(txn);
                counters::core_mempool_txn_ranking_score(
                    counters::CONSENSUS_PULLED_LABEL,
                    counters::CONSENSUS_PULLED_LABEL,
                    self.transactions.get_bucket(ranking_score),
                    ranking_score,
                );
            }
        }

        if result_size > 0 {
            debug!(
                LogSchema::new(LogEntry::GetBlock),
                seen_consensus = seen_size,
                walked = txn_walked,
                seen_after = seen.len(),
                // before size and non full check
                result_size = result_size,
                // before non full check
                byte_size = total_bytes,
                block_size = block.len(),
                return_non_full = return_non_full,
            );
        } else {
            trace!(
                LogSchema::new(LogEntry::GetBlock),
                seen_consensus = seen_size,
                walked = txn_walked,
                seen_after = seen.len(),
                // before size and non full check
                result_size = result_size,
                // before non full check
                byte_size = total_bytes,
                block_size = block.len(),
                return_non_full = return_non_full,
            );
        }

        if !return_non_full && !full_bytes && (block.len() as u64) < max_txns {
            block.clear();
        }

        counters::mempool_service_transactions(counters::GET_BLOCK_LABEL, block.len());
        counters::MEMPOOL_SERVICE_BYTES_GET_BLOCK.observe(total_bytes as f64);
        for transaction in &block {
            self.log_latency(
                transaction.sender(),
                transaction.sequence_number(),
                counters::CONSENSUS_PULLED_LABEL,
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
        timeline_id: &MultiBucketTimelineIndexIds,
        count: usize,
    ) -> (Vec<SignedTransaction>, MultiBucketTimelineIndexIds) {
        self.transactions.read_timeline(timeline_id, count)
    }

    /// Read transactions from timeline from `start_id` (exclusive) to `end_id` (inclusive).
    pub(crate) fn timeline_range(
        &self,
        start_end_pairs: &Vec<(u64, u64)>,
    ) -> Vec<SignedTransaction> {
        self.transactions.timeline_range(start_end_pairs)
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
