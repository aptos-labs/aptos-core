// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Copyright © Aptos Foundation

use crate::block_orderer::BlockOrderer;

pub trait BlockPartitioner {
    type Txn;

    fn partition_transactions<F, E>(
        &self,
        txns: Vec<Self::Txn>,
        send_transactions_for_execution: F,
    ) -> Result<(), E>
    where
        F: FnMut(Vec<Vec<(usize, Self::Txn)>>) -> Result<(), E>;
}

pub struct OrderedRoundRobinPartitioner<O> {
    block_orderer: O,
    n_shards: usize,
    min_per_shard_batch_size: usize,
}

impl<O> OrderedRoundRobinPartitioner<O> {
    pub fn new(block_orderer: O, n_shards: usize, min_per_shard_batch_size: usize) -> Self {
        Self {
            block_orderer,
            n_shards,
            min_per_shard_batch_size,
        }
    }
}

impl<O> BlockPartitioner for OrderedRoundRobinPartitioner<O>
where
    O: BlockOrderer,
    O::Txn: Clone,
{
    type Txn = O::Txn;

    fn partition_transactions<F, E>(
        &self,
        txns: Vec<Self::Txn>,
        mut send_transactions_for_execution: F,
    ) -> Result<(), E>
    where
        F: FnMut(Vec<Vec<(usize, Self::Txn)>>) -> Result<(), E>,
    {
        let num_txns = txns.len();
        let extra_txns_threshold = (0.2 * (num_txns / self.n_shards) as f64) as usize;
        let exp_ordered_txns_non_conflicting_window = self.block_orderer.get_max_window_size();
        let mut ordered = 0;
        let mut batch = vec![vec![]; self.n_shards];

        println!("OrderedRoundRobinPartitioner: num_txns={}, extra_txns_threshold={}", num_txns, extra_txns_threshold);

        self.block_orderer
            .order_transactions(txns, |ordered_txns| {
                if ordered_txns.len() < exp_ordered_txns_non_conflicting_window
                    && (num_txns - ordered) <= extra_txns_threshold {
                    println!("Putting {} transactions in shard 0; remaining txns {}", ordered_txns.len(), num_txns - ordered);
                    for tx in ordered_txns {
                        let idx = ordered;
                        batch[0].push((idx, tx.clone()));
                        ordered += 1;
                    }
                } else {
                    for tx in ordered_txns {
                        let idx = ordered;
                        let shard_id = idx % self.n_shards;
                        batch[shard_id].push((idx, tx));
                        ordered += 1;
                    }
                }
                if batch[self.n_shards - 1].len() >= self.min_per_shard_batch_size {
                    send_transactions_for_execution(batch.clone())?;
                    for shard in &mut batch {
                        shard.clear();
                    }
                }
                Ok(())
            })?;

        if !batch[0].is_empty() {
            send_transactions_for_execution(batch)?;
        }

        Ok(())
    }
}
