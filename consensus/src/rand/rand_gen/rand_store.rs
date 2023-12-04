// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    experimental::buffer_manager::OrderedBlocks,
    rand::rand_gen::{
        block_queue::{BlockQueue, QueueItem},
        types::{Proof, RandConfig, RandDecision, RandShare, Share},
    },
};
use anyhow::ensure;
use aptos_consensus_types::{
    common::{Author, Round},
    randomness::RandMetadata,
};
use aptos_logger::error;
use std::collections::HashMap;

struct ShareAggregator<S> {
    shares: HashMap<Author, RandShare<S>>,
    total_weight: u64,
}

impl<S: Share> ShareAggregator<S> {
    fn new() -> Self {
        Self {
            shares: HashMap::new(),
            total_weight: 0,
        }
    }

    fn add_share(&mut self, weight: u64, share: RandShare<S>) {
        self.shares.insert(*share.author(), share);
        self.total_weight += weight;
    }

    fn try_aggregate<P: Proof<Share = S>>(
        &self,
        rand_config: &RandConfig,
        rand_metadata: RandMetadata,
    ) -> Option<RandDecision<P>> {
        if self.total_weight < rand_config.threshold_weight() {
            return None;
        }
        Some(P::aggregate(
            self.shares.values(),
            rand_config,
            rand_metadata,
        ))
    }

    fn retain(&mut self, rand_config: &RandConfig, rand_metadata: &RandMetadata) {
        self.shares
            .retain(|_, share| share.metadata() == rand_metadata);
        self.total_weight = self
            .shares
            .keys()
            .map(|author| rand_config.get_peer_weight(author))
            .sum();
    }
}

enum RandItem<S, P> {
    PendingBlock(ShareAggregator<S>),
    PendingDecision {
        metadata: RandMetadata,
        share_aggregator: ShareAggregator<S>,
    },
    Decided(RandDecision<P>),
}

impl<S: Share, P: Proof<Share = S>> Default for RandItem<S, P> {
    fn default() -> Self {
        Self::PendingBlock(ShareAggregator::new())
    }
}

impl<S: Share, P: Proof<Share = S>> RandItem<S, P> {
    fn add_share(&mut self, share: RandShare<S>, rand_config: &RandConfig) -> anyhow::Result<()> {
        match self {
            RandItem::PendingBlock(aggr) => {
                aggr.add_share(rand_config.get_peer_weight(share.author()), share);
                Ok(())
            },
            RandItem::PendingDecision {
                metadata,
                share_aggregator,
            } => {
                ensure!(
                    metadata == share.metadata(),
                    "[RandStore] RandShare metadata from {} mismatch with block metadata!",
                    share.author(),
                );
                share_aggregator.add_share(rand_config.get_peer_weight(share.author()), share);
                Ok(())
            },
            RandItem::Decided(_) => Ok(()),
        }
    }

    fn decision(&self) -> Option<&RandDecision<P>> {
        match self {
            RandItem::PendingBlock(_) | RandItem::PendingDecision { .. } => None,
            RandItem::Decided(decision) => Some(decision),
        }
    }

    fn try_aggregate(&mut self, rand_config: &RandConfig) {
        let item = std::mem::take(self);
        let new_item = match item {
            RandItem::PendingDecision {
                share_aggregator,
                metadata,
            } => {
                if let Some(decision) =
                    share_aggregator.try_aggregate(rand_config, metadata.clone())
                {
                    Self::Decided(decision)
                } else {
                    Self::PendingDecision {
                        metadata,
                        share_aggregator,
                    }
                }
            },
            item @ (RandItem::Decided(_) | RandItem::PendingBlock(_)) => item,
        };
        let _ = std::mem::replace(self, new_item);
    }

    fn add_block(&mut self, rand_config: &RandConfig, rand_metadata: RandMetadata) {
        let item = std::mem::take(self);
        let new_item = match item {
            RandItem::PendingBlock(mut share_aggregator) => {
                share_aggregator.retain(rand_config, &rand_metadata);
                Self::PendingDecision {
                    metadata: rand_metadata,
                    share_aggregator,
                }
            },
            item @ (RandItem::PendingDecision { .. } | RandItem::Decided(_)) => item,
        };
        let _ = std::mem::replace(self, new_item);
    }
}

pub struct RandStore<S, P> {
    author: Author,
    rand_config: RandConfig,
    rand_map: HashMap<Round, RandItem<S, P>>,
    block_queue: BlockQueue,
}

impl<S: Share, P: Proof<Share = S>> RandStore<S, P> {
    pub fn new(author: Author, rand_config: RandConfig) -> Self {
        Self {
            author,
            rand_config,
            rand_map: HashMap::new(),
            block_queue: BlockQueue::new(),
        }
    }

    fn try_dequeue_rand_ready_prefix(&mut self) -> Option<Vec<OrderedBlocks>> {
        let prefix = self.block_queue.dequeue_rand_ready_prefix();
        if prefix.is_empty() {
            None
        } else {
            Some(prefix)
        }
    }

    fn add_share_impl(&mut self, share: RandShare<S>) -> anyhow::Result<()> {
        let rand_metadata = share.metadata().clone();
        let rand_item = self
            .rand_map
            .entry(rand_metadata.round())
            .or_insert_with(Default::default);
        rand_item.add_share(share, &self.rand_config)?;
        Self::try_aggregate(&self.rand_config, rand_item, &mut self.block_queue);
        Ok(())
    }

    fn try_aggregate(
        rand_config: &RandConfig,
        rand_item: &mut RandItem<S, P>,
        block_queue: &mut BlockQueue,
    ) {
        rand_item.try_aggregate(rand_config);
        if let Some(decision) = rand_item.decision() {
            block_queue.set_randomness(
                decision.rand_metadata().round(),
                decision.randomness().clone(),
            );
        }
    }

    pub fn add_block(&mut self, block: QueueItem) -> Option<Vec<OrderedBlocks>> {
        let all_rand_metadata = block.all_rand_metadata();
        self.block_queue.push_back(block);
        for rand_metadata in all_rand_metadata {
            let rand_item = self
                .rand_map
                .entry(rand_metadata.round())
                .or_insert_with(Default::default);
            rand_item.add_block(&self.rand_config, rand_metadata);
            Self::try_aggregate(&self.rand_config, rand_item, &mut self.block_queue);
        }
        self.try_dequeue_rand_ready_prefix()
    }

    pub fn add_share(&mut self, share: RandShare<S>) -> Option<Vec<OrderedBlocks>> {
        if let Err(e) = self.add_share_impl(share) {
            error!("[RandStore] error adding share {}", e);
        }
        self.try_dequeue_rand_ready_prefix()
    }

    pub fn add_decision(&mut self, decision: RandDecision<P>) -> Option<Vec<OrderedBlocks>> {
        let rand_metadata = decision.rand_metadata();
        self.block_queue
            .set_randomness(rand_metadata.round(), decision.randomness().clone());
        self.rand_map
            .insert(rand_metadata.round(), RandItem::Decided(decision));
        self.try_dequeue_rand_ready_prefix()
    }
}
