// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    block_storage::tracing::{observe_block, BlockStage},
    pipeline::buffer_manager::OrderedBlocks,
    rand::rand_gen::{
        block_queue::{BlockQueue, QueueItem},
        types::{RandConfig, RandShare, Share},
    },
};
use anyhow::ensure;
use aptos_consensus_types::{
    common::{Author, Round},
    randomness::{RandMetadata, Randomness},
};
use std::collections::{BTreeMap, HashMap};

const FUTURE_ROUNDS_TO_ACCEPT: u64 = 200;

pub struct ShareAggregator<S> {
    author: Author,
    shares: HashMap<Author, RandShare<S>>,
    total_weight: u64,
}

impl<S: Share> ShareAggregator<S> {
    pub fn new(author: Author) -> Self {
        Self {
            author,
            shares: HashMap::new(),
            total_weight: 0,
        }
    }

    pub fn add_share(&mut self, weight: u64, share: RandShare<S>) {
        let timestamp = share.metadata().timestamp();
        if self.shares.insert(*share.author(), share).is_none() {
            observe_block(timestamp, BlockStage::RAND_ADD_SHARE);
            self.total_weight += weight;
        }
    }

    pub fn try_aggregate(
        &self,
        rand_config: &RandConfig,
        rand_metadata: RandMetadata,
    ) -> Option<Randomness> {
        if self.total_weight < rand_config.threshold_weight() {
            return None;
        }
        Some(S::aggregate(
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

    fn get_self_share(&self) -> Option<RandShare<S>> {
        self.shares.get(&self.author).cloned()
    }

    fn total_weights(&self) -> u64 {
        self.total_weight
    }
}

enum RandItem<S> {
    PendingMetadata(ShareAggregator<S>),
    PendingDecision {
        metadata: RandMetadata,
        share_aggregator: ShareAggregator<S>,
    },
    Decided {
        decision: Randomness,
        self_share: RandShare<S>,
    },
}

impl<S: Share> RandItem<S> {
    fn new(author: Author) -> Self {
        Self::PendingMetadata(ShareAggregator::new(author))
    }

    fn decision(&self) -> Option<&Randomness> {
        match self {
            RandItem::PendingMetadata(_) | RandItem::PendingDecision { .. } => None,
            RandItem::Decided { decision, .. } => Some(decision),
        }
    }

    fn total_weights(&self) -> Option<u64> {
        match self {
            RandItem::PendingMetadata(aggr) => Some(aggr.total_weights()),
            RandItem::PendingDecision {
                share_aggregator, ..
            } => Some(share_aggregator.total_weights()),
            RandItem::Decided { .. } => None,
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
            RandItem::Decided { .. } => Ok(()),
        }
    }

    fn try_aggregate(&mut self, rand_config: &RandConfig) {
        let item = std::mem::replace(self, Self::new(Author::ONE));
        let new_item = match item {
            RandItem::PendingDecision {
                share_aggregator,
                metadata,
            } => {
                if let Some(decision) =
                    share_aggregator.try_aggregate(rand_config, metadata.clone())
                {
                    observe_block(
                        decision.metadata().timestamp(),
                        BlockStage::RAND_AGG_DECISION,
                    );
                    Self::Decided {
                        decision,
                        self_share: share_aggregator
                            .get_self_share()
                            .expect("Aggregated item should have self share"),
                    }
                } else {
                    Self::PendingDecision {
                        metadata,
                        share_aggregator,
                    }
                }
            },
            item @ (RandItem::Decided { .. } | RandItem::PendingMetadata(_)) => item,
        };
        let _ = std::mem::replace(self, new_item);
    }

    fn add_metadata(&mut self, rand_config: &RandConfig, rand_metadata: RandMetadata) {
        let item = std::mem::replace(self, Self::new(Author::ONE));
        let new_item = match item {
            RandItem::PendingMetadata(mut share_aggregator) => {
                share_aggregator.retain(rand_config, &rand_metadata);
                Self::PendingDecision {
                    metadata: rand_metadata,
                    share_aggregator,
                }
            },
            item @ (RandItem::PendingDecision { .. } | RandItem::Decided { .. }) => item,
        };
        let _ = std::mem::replace(self, new_item);
    }

    fn get_self_share(&self) -> Option<RandShare<S>> {
        match self {
            RandItem::PendingMetadata(aggr) => aggr.get_self_share(),
            RandItem::PendingDecision {
                share_aggregator, ..
            } => share_aggregator.get_self_share(),
            RandItem::Decided { self_share, .. } => Some(self_share.clone()),
        }
    }
}

pub struct RandStore<S> {
    epoch: u64,
    author: Author,
    rand_config: RandConfig,
    rand_map: BTreeMap<Round, RandItem<S>>,
    block_queue: BlockQueue,
    highest_known_round: u64,
}

impl<S: Share> RandStore<S> {
    pub fn new(epoch: u64, author: Author, rand_config: RandConfig) -> Self {
        Self {
            epoch,
            author,
            rand_config,
            rand_map: BTreeMap::new(),
            block_queue: BlockQueue::new(),
            highest_known_round: 0,
        }
    }

    pub fn reset(&mut self, target_round: u64) {
        self.block_queue = BlockQueue::new();
        self.highest_known_round = std::cmp::max(self.highest_known_round, target_round);
    }

    pub fn try_dequeue_rand_ready_prefix(&mut self) -> Option<Vec<OrderedBlocks>> {
        let prefix = self.block_queue.dequeue_rand_ready_prefix();
        if prefix.is_empty() {
            None
        } else {
            Some(prefix)
        }
    }

    fn try_aggregate(
        rand_config: &RandConfig,
        rand_item: &mut RandItem<S>,
        block_queue: &mut BlockQueue,
    ) {
        rand_item.try_aggregate(rand_config);
        if let Some(decision) = rand_item.decision() {
            block_queue.set_randomness(decision.metadata().round(), decision.clone());
        }
    }

    pub fn add_blocks(&mut self, block: QueueItem) {
        let all_rand_metadata = block.all_rand_metadata();
        self.block_queue.push_back(block);
        for rand_metadata in all_rand_metadata {
            self.highest_known_round =
                std::cmp::max(self.highest_known_round, rand_metadata.round());
            let rand_item = self
                .rand_map
                .entry(rand_metadata.round())
                .or_insert_with(|| RandItem::new(self.author));
            rand_item.add_metadata(&self.rand_config, rand_metadata.clone());
            Self::try_aggregate(&self.rand_config, rand_item, &mut self.block_queue);
        }
    }

    pub fn add_share(&mut self, share: RandShare<S>) -> anyhow::Result<bool> {
        ensure!(
            share.metadata().epoch() == self.epoch,
            "Share from different epoch"
        );
        ensure!(
            share.metadata().round() <= self.highest_known_round + FUTURE_ROUNDS_TO_ACCEPT,
            "Share from future round"
        );
        let rand_metadata = share.metadata().clone();
        let rand_item = self
            .rand_map
            .entry(rand_metadata.round())
            .or_insert_with(|| RandItem::new(self.author));
        rand_item.add_share(share, &self.rand_config)?;
        Self::try_aggregate(&self.rand_config, rand_item, &mut self.block_queue);
        Ok(rand_item.decision().is_some())
    }

    pub fn get_self_share(&mut self, metadata: &RandMetadata) -> Option<RandShare<S>> {
        self.rand_map
            .get(&metadata.round())
            .and_then(|item| item.get_self_share())
            .filter(|share| share.metadata() == metadata)
    }
}

#[cfg(test)]
mod tests {
    use crate::rand::rand_gen::{
        block_queue::QueueItem,
        rand_store::{RandItem, RandStore, ShareAggregator},
        test_utils::{create_ordered_blocks, create_share, create_share_for_round},
        types::{MockShare, RandConfig},
    };
    use aptos_consensus_types::{common::Author, randomness::RandMetadata};
    use std::{collections::HashMap, str::FromStr};

    #[test]
    fn test_share_aggregator() {
        let mut aggr = ShareAggregator::new(Author::ONE);
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
            &RandConfig::new(1, Author::ZERO, weights),
            &RandMetadata::new_for_testing(1),
        );
        assert_eq!(aggr.shares.len(), 2);
        assert_eq!(aggr.total_weight, 4);
    }

    #[test]
    fn test_rand_item() {
        let weights = HashMap::from([(Author::ONE, 1), (Author::TWO, 2), (Author::ZERO, 3)]);
        let config = RandConfig::new(1, Author::ZERO, weights);
        let shares = vec![
            create_share_for_round(2, Author::ONE),
            create_share_for_round(1, Author::TWO),
            create_share_for_round(1, Author::ZERO),
        ];

        let mut item = RandItem::<MockShare>::new(Author::TWO);
        for share in shares.iter() {
            item.add_share(share.clone(), &config).unwrap();
        }
        assert_eq!(item.total_weights().unwrap(), 6);
        item.add_metadata(&config, RandMetadata::new_for_testing(1));
        assert_eq!(item.total_weights().unwrap(), 5);
        item.try_aggregate(&config);
        assert!(item.decision().is_some());

        let mut item = RandItem::<MockShare>::new(Author::ONE);
        item.add_metadata(&config, RandMetadata::new_for_testing(2));
        for share in shares[1..].iter() {
            item.add_share(share.clone(), &config).unwrap_err();
        }
    }

    #[test]
    fn test_rand_store() {
        let authors: Vec<_> = (0..7)
            .map(|i| Author::from_str(&format!("{:x}", i)).unwrap())
            .collect();
        let weights: HashMap<Author, u64> = authors.iter().map(|addr| (*addr, 1)).collect();
        let authors: Vec<Author> = weights.keys().cloned().collect();
        let config = RandConfig::new(1, Author::ZERO, weights);
        let mut rand_store = RandStore::new(1, authors[1], config);

        let rounds = vec![vec![1], vec![2, 3], vec![5, 8, 13]];
        let blocks_1 = QueueItem::new(create_ordered_blocks(rounds[0].clone()), None);
        let blocks_2 = QueueItem::new(create_ordered_blocks(rounds[1].clone()), None);
        let metadata_1 = blocks_1.all_rand_metadata();
        let metadata_2 = blocks_2.all_rand_metadata();

        // shares come before blocks
        for share in authors[0..5]
            .iter()
            .map(|author| create_share(metadata_1[0].clone(), *author))
        {
            rand_store.add_share(share).unwrap();
        }
        assert!(rand_store.try_dequeue_rand_ready_prefix().is_none());
        rand_store.add_blocks(blocks_1);
        assert_eq!(rand_store.try_dequeue_rand_ready_prefix().unwrap().len(), 1);
        // blocks come after shares
        rand_store.add_blocks(blocks_2);
        assert!(rand_store.try_dequeue_rand_ready_prefix().is_none());

        for share in authors[1..6]
            .iter()
            .map(|author| create_share(metadata_2[0].clone(), *author))
        {
            rand_store.add_share(share).unwrap();
        }
        assert!(rand_store.try_dequeue_rand_ready_prefix().is_none());
    }
}
