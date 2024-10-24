// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    counters::{self, MAX_TXNS_FROM_BLOCK_TO_EXECUTE},
    monitor,
    payload_manager::TPayloadManager,
    transaction_deduper::TransactionDeduper,
    transaction_filter::TransactionFilter,
    transaction_shuffler::TransactionShuffler,
};
use aptos_consensus_types::{block::Block, pipelined_block::OrderedBlockWindow};
use aptos_executor_types::ExecutorResult;
use aptos_logger::info;
use aptos_types::transaction::SignedTransaction;
use fail::fail_point;
use futures::{stream::FuturesOrdered, StreamExt};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use std::{cmp::Reverse, collections::HashSet, sync::Arc, time::Instant};

pub struct BlockPreparer {
    payload_manager: Arc<dyn TPayloadManager>,
    txn_filter: Arc<TransactionFilter>,
    txn_deduper: Arc<dyn TransactionDeduper>,
    txn_shuffler: Arc<dyn TransactionShuffler>,
    max_block_txns: u64,
}

impl BlockPreparer {
    pub fn new(
        payload_manager: Arc<dyn TPayloadManager>,
        txn_filter: Arc<TransactionFilter>,
        txn_deduper: Arc<dyn TransactionDeduper>,
        txn_shuffler: Arc<dyn TransactionShuffler>,
        max_block_txns: u64,
    ) -> Self {
        Self {
            payload_manager,
            txn_filter,
            txn_deduper,
            txn_shuffler,
            max_block_txns,
        }
    }

    async fn get_transactions(
        &self,
        block: &Block,
        block_window: &OrderedBlockWindow,
    ) -> ExecutorResult<(Vec<(Arc<Vec<SignedTransaction>>, u64)>, Option<u64>)> {
        let mut txns = vec![];
        let mut futures = FuturesOrdered::new();
        info!(
            "get_transactions started in block window for ({}, {})",
            block.epoch(),
            block.round()
        );
        for block in block_window
            .pipelined_blocks()
            .iter()
            .map(|b| b.block())
            .chain(std::iter::once(block))
        {
            futures.push_back(self.payload_manager.get_transactions(block));
        }
        info!("get_transactions added all futures");

        let mut idx = 0;
        let mut max_txns_from_block_to_execute = None;
        loop {
            info!("get_transactions waiting for next: {}", idx);
            match futures.next().await {
                // TODO: we are turning off the max txns from block to execute feature for now
                Some(Ok((block_txns, _max_txns))) => {
                    txns.extend(block_txns);
                    max_txns_from_block_to_execute = None;
                },
                Some(Err(e)) => {
                    return Err(e);
                },
                None => break,
            }
            idx += 1;
        }
        info!(
            "get_transactions finished in block window for ({}, {})",
            block.epoch(),
            block.round()
        );
        Ok((txns, max_txns_from_block_to_execute))
    }

    pub async fn prepare_block(
        &self,
        block: &Block,
        block_window: &OrderedBlockWindow,
    ) -> ExecutorResult<Vec<SignedTransaction>> {
        fail_point!("consensus::prepare_block", |_| {
            use aptos_executor_types::ExecutorError;
            use std::{thread, time::Duration};
            thread::sleep(Duration::from_millis(10));
            Err(ExecutorError::CouldNotGetData)
        });
        let start_time = Instant::now();
        info!(
            "BlockPreparer: Preparing for block ({}, {}) {} and window {:?}",
            block.epoch(),
            block.round(),
            block.id(),
            block_window
                .blocks()
                .iter()
                .map(|b| b.id())
                .collect::<Vec<_>>()
        );

        let now = std::time::Instant::now();
        // TODO: we could do this incrementally, but for now just do it every time
        let mut committed_transactions = HashSet::new();

        // TODO: don't materialize these?
        let (mut batched_txns, max_txns_from_block_to_execute) = monitor!("get_transactions", {
            self.get_transactions(block, block_window).await?
        });

        // TODO: lots of repeated code here
        monitor!("wait_for_committed_transactions", {
            for b in block_window
                .pipelined_blocks()
                .iter()
                .filter(|window_block| window_block.round() < block.round() - 1)
            {
                info!(
                    "BlockPreparer: Waiting for committed transactions at block {} for block {}",
                    b.round(),
                    block.round()
                );
                for txn_hash in b.wait_for_committed_transactions()?.iter() {
                    committed_transactions.insert(*txn_hash);
                }
                info!(
                    "BlockPreparer: Waiting for committed transactions at block {} for block {}: Done",
                    b.round(),
                    block.round()
                );
            }
        });
        info!(
            "BlockPreparer: Waiting for part of committed transactions for round {} took {} ms",
            block.round(),
            now.elapsed().as_millis()
        );

        let txn_filter = self.txn_filter.clone();
        let txn_deduper = self.txn_deduper.clone();
        let block_id = block.id();
        let block_timestamp_usecs = block.timestamp_usecs();
        let max_prepared_block_txns = self.max_block_txns as usize * 2;
        // Transaction filtering, deduplication and shuffling are CPU intensive tasks, so we run them in a blocking task.
        let result = tokio::task::spawn_blocking(move || {
            // stable sort to ensure batches with same gas are in the same order
            batched_txns.sort_by_key(|(_, gas)| Reverse(*gas));

            let batched_txns: Vec<Vec<_>> = monitor!(
                "filter_committed_transactions",
                batched_txns
                    .into_par_iter()
                    .map(|(txns, _)| {
                        txns.iter()
                            .filter(|txn| !committed_transactions.contains(&txn.committed_hash()))
                            .cloned()
                            .collect()
                    })
                    .collect()
            );
            let txns: Vec<_> = monitor!(
                "flatten_transactions",
                batched_txns
                    .into_iter()
                    .flatten()
                    .take(max_prepared_block_txns)
                    .collect()
            );
            let filtered_txns = monitor!("filter_transactions", {
                txn_filter.filter(block_id, block_timestamp_usecs, txns)
            });
            let mut deduped_txns = monitor!("dedup_transactions", txn_deduper.dedup(filtered_txns));
            if let Some(max_txns_from_block_to_execute) = max_txns_from_block_to_execute {
                deduped_txns.truncate(max_txns_from_block_to_execute as usize);
            }
            MAX_TXNS_FROM_BLOCK_TO_EXECUTE.observe(deduped_txns.len() as f64);
            Ok(deduped_txns)
        })
        .await
        .expect("Failed to spawn blocking task for transaction generation");
        counters::BLOCK_PREPARER_LATENCY.observe_duration(start_time.elapsed());
        result
    }
}
