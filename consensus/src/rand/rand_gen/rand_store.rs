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
        if self.shares.insert(*share.author(), share).is_none() {
            self.total_weight += weight;
        }
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

    fn total_weights(&self) -> u64 {
        self.total_weight
    }
}

enum RandItem<S, P> {
    PendingMetadata(ShareAggregator<S>),
    PendingDecision {
        metadata: RandMetadata,
        share_aggregator: ShareAggregator<S>,
    },
    Decided(RandDecision<P>),
}

impl<S: Share, P: Proof<Share = S>> Default for RandItem<S, P> {
    fn default() -> Self {
        Self::PendingMetadata(ShareAggregator::new())
    }
}

impl<S: Share, P: Proof<Share = S>> RandItem<S, P> {
    fn decision(&self) -> Option<&RandDecision<P>> {
        match self {
            RandItem::PendingMetadata(_) | RandItem::PendingDecision { .. } => None,
            RandItem::Decided(decision) => Some(decision),
        }
    }

    fn total_weights(&self) -> Option<u64> {
        match self {
            RandItem::PendingMetadata(aggr) => Some(aggr.total_weights()),
            RandItem::PendingDecision {
                share_aggregator, ..
            } => Some(share_aggregator.total_weights()),
            RandItem::Decided(_) => None,
        }
    }

    fn add_share(&mut self, share: RandShare<S>, rand_config: &RandConfig) -> anyhow::Result<()> {
        match self {
            RandItem::PendingMetadata(aggr) => {
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
            item @ (RandItem::Decided(_) | RandItem::PendingMetadata(_)) => item,
        };
        let _ = std::mem::replace(self, new_item);
    }

    fn add_metadata(&mut self, rand_config: &RandConfig, rand_metadata: RandMetadata) {
        let item = std::mem::take(self);
        let new_item = match item {
            RandItem::PendingMetadata(mut share_aggregator) => {
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
            rand_item.add_metadata(&self.rand_config, rand_metadata);
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

#[cfg(test)]
mod tests {
    use crate::rand::rand_gen::{
        block_queue::QueueItem,
        rand_store::{RandItem, RandStore, ShareAggregator},
        test_utils::{
            create_decision, create_ordered_blocks, create_share, create_share_for_round,
        },
        types::{MockProof, MockShare, RandConfig},
    };
    use aptos_consensus_types::{common::Author, randomness::RandMetadata};
    use std::{collections::HashMap, str::FromStr};

    #[test]
    fn test_share_aggregator() {
        let mut aggr = ShareAggregator::new();
        let weights = HashMap::from([(Author::ONE, 1), (Author::TWO, 2), (Author::ZERO, 3)]);
        let shares = vec![
            create_share_for_round(1, Author::ONE),
            create_share_for_round(2, Author::TWO),
            create_share_for_round(1, Author::ZERO),
        ];
        for share in shares.iter() {
            aggr.add_share(*weights.get(share.author()).unwrap(), share.clone());
            // double add should be no op to the total weight
            aggr.add_share(*weights.get(share.author()).unwrap(), share.clone());
        }
        assert_eq!(aggr.shares.len(), 3);
        assert_eq!(aggr.total_weight, 6);
        // retain the shares with the same metadata
        aggr.retain(
            &RandConfig::new(Author::ZERO, weights),
            &RandMetadata::new_for_testing(1),
        );
        assert_eq!(aggr.shares.len(), 2);
        assert_eq!(aggr.total_weight, 4);
    }

    #[test]
    fn test_rand_item() {
        let weights = HashMap::from([(Author::ONE, 1), (Author::TWO, 2), (Author::ZERO, 3)]);
        let config = RandConfig::new(Author::ZERO, weights);
        let shares = vec![
            create_share_for_round(2, Author::ONE),
            create_share_for_round(1, Author::TWO),
            create_share_for_round(1, Author::ZERO),
        ];

        let mut item = RandItem::<MockShare, MockProof>::default();
        for share in shares.iter() {
            item.add_share(share.clone(), &config).unwrap();
        }
        assert_eq!(item.total_weights().unwrap(), 6);
        item.add_metadata(&config, RandMetadata::new_for_testing(1));
        assert_eq!(item.total_weights().unwrap(), 5);
        item.try_aggregate(&config);
        assert!(item.decision().is_some());

        let mut item = RandItem::<MockShare, MockProof>::default();
        item.add_metadata(&config, RandMetadata::new_for_testing(2));
        for share in shares[1..].iter() {
            item.add_share(share.clone(), &config).unwrap_err();
        }
    }

    #[test]
    fn test_rand_store() {
        let weights: HashMap<Author, u64> = (0..7)
            .map(|i| (Author::from_str(&format!("{:x}", i)).unwrap(), 1))
            .collect();
        let authors: Vec<Author> = weights.keys().cloned().collect();
        let config = RandConfig::new(Author::ZERO, weights);
        let mut rand_store = RandStore::new(Author::ZERO, config);

        let rounds = vec![vec![1], vec![2, 3], vec![5, 8, 13]];
        let blocks_1 = QueueItem::new(create_ordered_blocks(rounds[0].clone()));
        let blocks_2 = QueueItem::new(create_ordered_blocks(rounds[1].clone()));
        let blocks_3 = QueueItem::new(create_ordered_blocks(rounds[2].clone()));
        let metadata_1 = blocks_1.all_rand_metadata();
        let metadata_2 = blocks_2.all_rand_metadata();
        let metadata_3 = blocks_3.all_rand_metadata();

        // shares come before blocks
        for share in authors[0..5]
            .iter()
            .map(|author| create_share(metadata_1[0].clone(), *author))
        {
            assert!(rand_store.add_share(share).is_none());
        }
        assert_eq!(rand_store.add_block(blocks_1).unwrap().len(), 1);
        // blocks come after shares
        assert!(rand_store.add_block(blocks_2).is_none());

        for share in authors[1..6]
            .iter()
            .map(|author| create_share(metadata_2[0].clone(), *author))
        {
            assert!(rand_store.add_share(share).is_none());
        }
        // decisions comes before blocks
        assert!(rand_store
            .add_decision(create_decision(metadata_3[0].clone()))
            .is_none());
        assert!(rand_store
            .add_decision(create_decision(metadata_3[2].clone()))
            .is_none());
        assert!(rand_store.add_block(blocks_3).is_none());
        // decisions comes after blocks
        assert!(rand_store
            .add_decision(create_decision(metadata_3[1].clone()))
            .is_none());

        // last aggregated decision dequeue all
        for share in authors[2..6]
            .iter()
            .map(|author| create_share(metadata_2[1].clone(), *author))
        {
            assert!(rand_store.add_share(share).is_none());
        }
        assert_eq!(
            rand_store
                .add_share(create_share(metadata_2[1].clone(), authors[6]))
                .unwrap()
                .len(),
            2
        );
    }
}
