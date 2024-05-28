// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    counters::{MAX_TXNS_FROM_BLOCK_TO_EXECUTE, TXN_SHUFFLE_SECONDS},
    payload_manager::PayloadManager,
    transaction_deduper::TransactionDeduper,
    transaction_filter::TransactionFilter,
    transaction_shuffler::TransactionShuffler,
};
use aptos_consensus_types::{block::Block, pipelined_block::OrderedBlockWindow};
use aptos_executor_types::ExecutorResult;
use aptos_logger::info;
use aptos_types::transaction::SignedTransaction;
use std::{cmp::Ordering, sync::Arc};

pub struct BlockPreparer {
    payload_manager: Arc<PayloadManager>,
    txn_filter: Arc<TransactionFilter>,
    txn_deduper: Arc<dyn TransactionDeduper>,
    txn_shuffler: Arc<dyn TransactionShuffler>,
}

impl BlockPreparer {
    pub fn new(
        payload_manager: Arc<PayloadManager>,
        txn_filter: Arc<TransactionFilter>,
        txn_deduper: Arc<dyn TransactionDeduper>,
        txn_shuffler: Arc<dyn TransactionShuffler>,
    ) -> Self {
        Self {
            payload_manager,
            txn_filter,
            txn_deduper,
            txn_shuffler,
        }
    }

    pub async fn prepare_block(
        &self,
        block: &Block,
        block_window: &OrderedBlockWindow,
    ) -> ExecutorResult<Vec<SignedTransaction>> {
        info!(
            "BlockPreparer: Preparing for block {} and window {:?}",
            block.id(),
            block_window
                .blocks()
                .iter()
                .map(|b| b.id())
                .collect::<Vec<_>>()
        );

        let mut txns = vec![];
        for block in block_window.blocks() {
            let (block_txns, _) = self.payload_manager.get_transactions(block).await?;
            txns.extend(block_txns);
        }
        // We take the ordered block's max_txns
        let (current_block_txns, max_txns_from_block_to_execute) =
            self.payload_manager.get_transactions(block).await?;
        txns.extend(current_block_txns);

        info!(
            "BlockPreparer: Prepared {} transactions for block {} and window {:?}",
            txns.len(),
            block.id(),
            block_window
                .blocks()
                .iter()
                .map(|b| b.id())
                .collect::<Vec<_>>()
        );

        // TODO: Copy-pasta from Mempool OrderedQueueKey. Make them share code?
        txns.sort_by(|a, b| {
            match a.gas_unit_price().cmp(&b.gas_unit_price()) {
                Ordering::Equal => {},
                ordering => return ordering,
            }
            match a
                .expiration_timestamp_secs()
                .cmp(&b.expiration_timestamp_secs())
                .reverse()
            {
                Ordering::Equal => {},
                ordering => return ordering,
            }
            match a.sender().cmp(&b.sender()) {
                Ordering::Equal => {},
                ordering => return ordering,
            }
            match a.sequence_number().cmp(&b.sequence_number()).reverse() {
                Ordering::Equal => {},
                ordering => return ordering,
            }
            a.committed_hash().cmp(&b.committed_hash())
        });

        let txn_filter = self.txn_filter.clone();
        let txn_deduper = self.txn_deduper.clone();
        let txn_shuffler = self.txn_shuffler.clone();
        let block_id = block.id();
        let block_timestamp_usecs = block.timestamp_usecs();
        // Transaction filtering, deduplication and shuffling are CPU intensive tasks, so we run them in a blocking task.
        tokio::task::spawn_blocking(move || {
            let filtered_txns = txn_filter.filter(block_id, block_timestamp_usecs, txns);
            let deduped_txns = txn_deduper.dedup(filtered_txns);
            let mut shuffled_txns = {
                let _timer = TXN_SHUFFLE_SECONDS.start_timer();

                txn_shuffler.shuffle(deduped_txns)
            };

            if let Some(max_txns_from_block_to_execute) = max_txns_from_block_to_execute {
                shuffled_txns.truncate(max_txns_from_block_to_execute);
            }
            MAX_TXNS_FROM_BLOCK_TO_EXECUTE.observe(shuffled_txns.len() as f64);
            Ok(shuffled_txns)
        })
        .await
        .expect("Failed to spawn blocking task for transaction generation")
    }
}
