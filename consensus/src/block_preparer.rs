// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    counters::{self, MAX_TXNS_FROM_BLOCK_TO_EXECUTE, TXN_SHUFFLE_SECONDS},
    monitor,
    payload_manager::TPayloadManager,
    transaction_deduper::TransactionDeduper,
    transaction_filter::TransactionFilter,
    transaction_shuffler::TransactionShuffler,
};
use aptos_consensus_types::{
    block::Block, pipelined_block::OrderedBlockWindow, quorum_cert::QuorumCert,
};
use aptos_executor_types::ExecutorResult;
use aptos_types::transaction::SignedTransaction;
use fail::fail_point;
use futures::{future::Shared, stream::FuturesOrdered, StreamExt};
use std::{future::Future, sync::Arc, time::Instant};

pub struct BlockPreparer {
    payload_manager: Arc<dyn TPayloadManager>,
    txn_filter: Arc<TransactionFilter>,
    txn_deduper: Arc<dyn TransactionDeduper>,
    txn_shuffler: Arc<dyn TransactionShuffler>,
    is_execution_pool_enabled: bool,
}

impl BlockPreparer {
    pub fn new(
        payload_manager: Arc<dyn TPayloadManager>,
        txn_filter: Arc<TransactionFilter>,
        txn_deduper: Arc<dyn TransactionDeduper>,
        txn_shuffler: Arc<dyn TransactionShuffler>,
        is_execution_pool_enabled: bool,
    ) -> Self {
        Self {
            payload_manager,
            txn_filter,
            txn_deduper,
            txn_shuffler,
            is_execution_pool_enabled,
        }
    }

    async fn get_transactions(
        &self,
        block: &Block,
        block_window: Option<&OrderedBlockWindow>,
        block_qc_fut: Shared<impl Future<Output = Option<Arc<QuorumCert>>>>,
    ) -> ExecutorResult<(Vec<SignedTransaction>, Option<u64>, Option<u64>)> {
        let mut all_txns = vec![];
        let pipelined_blocks = if self.is_execution_pool_enabled {
            block_window.map_or(vec![], |window| window.pipelined_blocks())
        } else {
            vec![]
        };
        let mut futures = FuturesOrdered::new();
        for block in pipelined_blocks.iter() {
            futures.push_back(async move {
                self.payload_manager
                    .get_transactions(
                        block.block(),
                        Some(
                            block
                                .quorum_cert()
                                .ledger_info()
                                .get_voters_bitvec()
                                .clone(),
                        ),
                    )
                    .await
            });
        }
        let (txns, max_txns_from_block_to_execute, block_gas_limit) = tokio::select! {
            // Poll the block qc future until a QC is received. Ignore None outcomes.
            Some(qc) = block_qc_fut => {
                let block_voters = Some(qc.ledger_info().get_voters_bitvec().clone());
                self.payload_manager.get_transactions(block, block_voters).await
            },
            result = self.payload_manager.get_transactions(block, None) => {
               result
            }
        }?;

        while let Some(result) = futures.next().await {
            let (block_txns, _max_txns, _gas_limit) = result?;
            all_txns.extend(block_txns);
        }
        all_txns.extend(txns);
        Ok((all_txns, max_txns_from_block_to_execute, block_gas_limit))
    }

    pub async fn prepare_block(
        &self,
        block: &Block,
        block_window: Option<&OrderedBlockWindow>,
        block_qc_fut: Shared<impl Future<Output = Option<Arc<QuorumCert>>>>,
    ) -> ExecutorResult<(Vec<SignedTransaction>, Option<u64>)> {
        fail_point!("consensus::prepare_block", |_| {
            use aptos_executor_types::ExecutorError;
            use std::{thread, time::Duration};
            thread::sleep(Duration::from_millis(10));
            Err(ExecutorError::CouldNotGetData)
        });
        let start_time = Instant::now();
        let (txns, max_txns_from_block_to_execute, block_gas_limit) =
            monitor!("get_transactions", {
                self.get_transactions(block, block_window, block_qc_fut)
                    .await?
            });
        let txn_filter = self.txn_filter.clone();
        let txn_deduper = self.txn_deduper.clone();
        let txn_shuffler = self.txn_shuffler.clone();
        let block_id = block.id();
        let block_timestamp_usecs = block.timestamp_usecs();
        // Transaction filtering, deduplication and shuffling are CPU intensive tasks, so we run them in a blocking task.
        let result = tokio::task::spawn_blocking(move || {
            let filtered_txns = txn_filter.filter(block_id, block_timestamp_usecs, txns);
            let deduped_txns = txn_deduper.dedup(filtered_txns);
            let mut shuffled_txns = {
                let _timer = TXN_SHUFFLE_SECONDS.start_timer();

                txn_shuffler.shuffle(deduped_txns)
            };

            if let Some(max_txns_from_block_to_execute) = max_txns_from_block_to_execute {
                shuffled_txns.truncate(max_txns_from_block_to_execute as usize);
            }
            MAX_TXNS_FROM_BLOCK_TO_EXECUTE.observe(shuffled_txns.len() as f64);
            Ok(shuffled_txns)
        })
        .await
        .expect("Failed to spawn blocking task for transaction generation");
        counters::BLOCK_PREPARER_LATENCY.observe_duration(start_time.elapsed());
        result.map(|result| (result, block_gas_limit))
    }
}
