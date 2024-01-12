// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    block_storage::tracing::{observe_block, BlockStage},
    rand::rand_gen::{
        rand_manager::Sender,
        types::{RandConfig, RandShare, Share},
    },
};
use anyhow::ensure;
use aptos_consensus_types::common::{Author, Round};
use aptos_types::randomness::{RandMetadata, Randomness};
use itertools::Either;
use std::collections::{BTreeMap, HashMap, HashSet};

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
        let timestamp = share.metadata().timestamp;
        if self.shares.insert(*share.author(), share).is_none() {
            observe_block(timestamp, BlockStage::RAND_ADD_SHARE);
            self.total_weight += weight;
        }
    }

    pub fn try_aggregate(
        self,
        rand_config: &RandConfig,
        rand_metadata: RandMetadata,
        decision_tx: Sender<Randomness>,
    ) -> Either<Self, RandShare<S>> {
        if self.total_weight < rand_config.threshold_weight() {
            return Either::Left(self);
        }
        let rand_config = rand_config.clone();
        let self_share = self
            .get_self_share()
            .expect("Aggregated item should have self share");
        tokio::task::spawn_blocking(move || {
            decision_tx.send(S::aggregate(
                self.shares.values(),
                &rand_config,
                rand_metadata,
            ))
        });
        Either::Right(self_share)
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
        self_share: RandShare<S>,
    },
}

impl<S: Share> RandItem<S> {
    fn new(author: Author) -> Self {
        Self::PendingMetadata(ShareAggregator::new(author))
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

    fn has_decision(&self) -> bool {
        matches!(self, RandItem::Decided { .. })
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

    fn try_aggregate(&mut self, rand_config: &RandConfig, decision_tx: Sender<Randomness>) {
        let item = std::mem::replace(self, Self::new(Author::ONE));
        let new_item = match item {
            RandItem::PendingDecision {
                share_aggregator,
                metadata,
            } => match share_aggregator.try_aggregate(rand_config, metadata.clone(), decision_tx) {
                Either::Left(share_aggregator) => Self::PendingDecision {
                    metadata,
                    share_aggregator,
                },
                Either::Right(self_share) => Self::Decided { self_share },
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

    fn get_all_shares_authors(&self) -> Option<HashSet<Author>> {
        match self {
            RandItem::PendingDecision {
                share_aggregator, ..
            } => Some(share_aggregator.shares.keys().cloned().collect()),
            RandItem::Decided { .. } => None,
            RandItem::PendingMetadata(_) => {
                unreachable!("Should only be called after block is added")
            },
        }
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
    highest_known_round: u64,
    decision_tx: Sender<Randomness>,
}

impl<S: Share> RandStore<S> {
    pub fn new(
        epoch: u64,
        author: Author,
        rand_config: RandConfig,
        decision_tx: Sender<Randomness>,
    ) -> Self {
        Self {
            epoch,
            author,
            rand_config,
            rand_map: BTreeMap::new(),
            highest_known_round: 0,
            decision_tx,
        }
    }

    pub fn reset(&mut self, target_round: u64) {
        self.highest_known_round = std::cmp::max(self.highest_known_round, target_round);
    }

    pub fn add_rand_metadata(&mut self, rand_metadata: RandMetadata) {
        self.highest_known_round = std::cmp::max(self.highest_known_round, rand_metadata.round());
        let rand_item = self
            .rand_map
            .entry(rand_metadata.round())
            .or_insert_with(|| RandItem::new(self.author));
        rand_item.add_metadata(&self.rand_config, rand_metadata.clone());
        rand_item.try_aggregate(&self.rand_config, self.decision_tx.clone());
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
        rand_item.try_aggregate(&self.rand_config, self.decision_tx.clone());
        Ok(rand_item.has_decision())
    }

    /// This should only be called after the block is added, returns None if already decided
    /// Otherwise returns existing shares' authors
    pub fn get_all_shares_authors(&self, metadata: &RandMetadata) -> Option<HashSet<Author>> {
        self.rand_map
            .get(&metadata.round())
            .and_then(|item| item.get_all_shares_authors())
    }

    pub fn get_self_share(
        &mut self,
        metadata: &RandMetadata,
    ) -> anyhow::Result<Option<RandShare<S>>> {
        ensure!(
            metadata.round() <= self.highest_known_round,
            "Request share from future round {}, highest known round {}",
            metadata.round(),
            self.highest_known_round
        );
        Ok(self
            .rand_map
            .get(&metadata.round())
            .and_then(|item| item.get_self_share())
            .filter(|share| share.metadata() == metadata))
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
    use aptos_consensus_types::common::Author;
    use aptos_types::randomness::RandMetadata;
    use std::{collections::HashMap, str::FromStr};
    use tokio::sync::mpsc::unbounded_channel;

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

    #[tokio::test]
    async fn test_rand_item() {
        let weights = HashMap::from([(Author::ONE, 1), (Author::TWO, 2), (Author::ZERO, 3)]);
        let config = RandConfig::new(1, Author::ZERO, weights);
        let (tx, _rx) = unbounded_channel();
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
        item.try_aggregate(&config, tx);
        assert!(item.has_decision());

        let mut item = RandItem::<MockShare>::new(Author::ONE);
        item.add_metadata(&config, RandMetadata::new_for_testing(2));
        for share in shares[1..].iter() {
            item.add_share(share.clone(), &config).unwrap_err();
        }
    }

    #[tokio::test]
    async fn test_rand_store() {
        let authors: Vec<_> = (0..7)
            .map(|i| Author::from_str(&format!("{:x}", i)).unwrap())
            .collect();
        let weights: HashMap<Author, u64> = authors.iter().map(|addr| (*addr, 1)).collect();
        let authors: Vec<Author> = weights.keys().cloned().collect();
        let config = RandConfig::new(1, Author::ZERO, weights);
        let (decision_tx, mut decision_rx) = unbounded_channel();
        let mut rand_store = RandStore::new(1, authors[1], config, decision_tx);

        let rounds = vec![vec![1], vec![2, 3], vec![5, 8, 13]];
        let blocks_1 = QueueItem::new(create_ordered_blocks(rounds[0].clone()), None);
        let blocks_2 = QueueItem::new(create_ordered_blocks(rounds[1].clone()), None);
        let metadata_1 = blocks_1.all_rand_metadata();
        let metadata_2 = blocks_2.all_rand_metadata();

        // shares come before metadata
        for share in authors[0..5]
            .iter()
            .map(|author| create_share(metadata_1[0].clone(), *author))
        {
            rand_store.add_share(share).unwrap();
        }
        assert!(decision_rx.try_recv().is_err());
        for metadata in blocks_1.all_rand_metadata() {
            rand_store.add_rand_metadata(metadata);
        }
        assert!(decision_rx.recv().await.is_some());
        // metadata come after shares
        for metadata in blocks_2.all_rand_metadata() {
            rand_store.add_rand_metadata(metadata);
        }
        assert!(decision_rx.try_recv().is_err());

        for share in authors[1..6]
            .iter()
            .map(|author| create_share(metadata_2[0].clone(), *author))
        {
            rand_store.add_share(share).unwrap();
        }
        assert!(decision_rx.recv().await.is_some());
    }
}
