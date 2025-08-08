// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    block_storage::tracing::{observe_block, BlockStage},
    rand::rand_gen::{
        rand_manager::Sender,
        types::{PathType, FUTURE_ROUNDS_TO_ACCEPT},
    },
};
use anyhow::ensure;
use aptos_consensus_types::common::{Author, Round};
use aptos_logger::{info, warn};
use aptos_types::decryption::{DecConfig, DecKey, DecMetadata, DecShare, Digest, DECRYPTION_POOL};
use itertools::Either;
use std::collections::{BTreeMap, HashMap, HashSet};

pub struct DecShareAggregator {
    author: Author,
    shares: HashMap<Author, DecShare>,
    total_weight: u64,
    path_type: PathType,
}

impl DecShareAggregator {
    pub fn new(author: Author, path_type: PathType) -> Self {
        Self {
            author,
            shares: HashMap::new(),
            total_weight: 0,
            path_type,
        }
    }

    pub fn add_share(&mut self, weight: u64, share: DecShare) {
        if self.shares.insert(share.author, share).is_none() {
            self.total_weight += weight;
        }
    }

    pub fn try_aggregate(
        self,
        dec_config: &DecConfig,
        metadata: DecMetadata,
        decision_tx: Sender<DecKey>,
    ) -> Either<Self, DecShare> {
        if self.total_weight < dec_config.threshold() {
            return Either::Left(self);
        }
        match self.path_type {
            PathType::Fast => {
                observe_block(
                    metadata.timestamp,
                    BlockStage::DEC_ADD_ENOUGH_SHARE_FAST,
                );
            },
            PathType::Slow => {
                observe_block(
                    metadata.timestamp,
                    BlockStage::DEC_ADD_ENOUGH_SHARE_SLOW,
                );
            },
        }
        let dec_config = dec_config.clone();
        let self_share = self
            .get_self_share()
            .expect("Aggregated item should have self share");
        tokio::task::spawn_blocking(move || {
            let maybe_key = DecShare::aggregate(
                self.shares.values(),
                &dec_config,
                &DECRYPTION_POOL,
            );
            match maybe_key {
                Ok(key) => {
                    let dec_key = DecKey::new(metadata, key);
                    let _ = decision_tx.unbounded_send(dec_key);
                },
                Err(e) => {
                    warn!(
                        epoch = metadata.epoch,
                        round = metadata.round,
                        "Aggregation error: {e}"
                    );
                },
            }
        });
        Either::Right(self_share)
    }

    fn retain(&mut self, dec_config: &DecConfig, metadata: &DecMetadata) {
        self.shares
            .retain(|_, share| share.metadata == *metadata);
        self.total_weight = self
            .shares
            .keys()
            .map(|author| dec_config.get_peer_weight(author))
            .sum();
    }

    fn get_self_share(&self) -> Option<DecShare> {
        self.shares.get(&self.author).cloned()
    }

    fn total_weights(&self) -> u64 {
        self.total_weight
    }
}

enum DecItem {
    PendingMetadata(DecShareAggregator),
    PendingDecision {
        metadata: DecMetadata,
        share_aggregator: DecShareAggregator,
    },
    Decided {
        self_share: DecShare,
    },
}

impl DecItem {
    fn new(author: Author, path_type: PathType) -> Self {
        Self::PendingMetadata(DecShareAggregator::new(author, path_type))
    }

    fn total_weights(&self) -> Option<u64> {
        match self {
            DecItem::PendingMetadata(aggr) => Some(aggr.total_weights()),
            DecItem::PendingDecision {
                share_aggregator, ..
            } => Some(share_aggregator.total_weights()),
            DecItem::Decided { .. } => None,
        }
    }

    fn has_decision(&self) -> bool {
        matches!(self, DecItem::Decided { .. })
    }

    fn add_share(&mut self, share: DecShare, dec_config: &DecConfig) -> anyhow::Result<()> {
        match self {
            DecItem::PendingMetadata(aggr) => {
                aggr.add_share(dec_config.get_peer_weight(&share.author), share);
                Ok(())
            },
            DecItem::PendingDecision {
                metadata,
                share_aggregator,
            } => {
                ensure!(
                    metadata == &share.metadata,
                    "[DecStore] DecShare metadata from {} mismatch with block metadata!",
                    share.author,
                );
                share_aggregator.add_share(dec_config.get_peer_weight(&share.author), share);
                Ok(())
            },
            DecItem::Decided { .. } => Ok(()),
        }
    }

    fn try_aggregate(&mut self, dec_config: &DecConfig, decision_tx: Sender<DecKey>) {
        let item = std::mem::replace(self, Self::new(Author::ONE, PathType::Slow));
        let new_item = match item {
            DecItem::PendingDecision {
                share_aggregator,
                metadata,
            } => match share_aggregator.try_aggregate(dec_config, metadata.clone(), decision_tx) {
                Either::Left(share_aggregator) => Self::PendingDecision {
                    metadata,
                    share_aggregator,
                },
                Either::Right(self_share) => Self::Decided { self_share },
            },
            item @ (DecItem::Decided { .. } | DecItem::PendingMetadata(_)) => item,
        };
        let _ = std::mem::replace(self, new_item);
    }

    fn add_metadata(&mut self, dec_config: &DecConfig, metadata: DecMetadata) {
        let item = std::mem::replace(self, Self::new(Author::ONE, PathType::Slow));
        let new_item = match item {
            DecItem::PendingMetadata(mut share_aggregator) => {
                share_aggregator.retain(dec_config, &metadata);
                Self::PendingDecision {
                    metadata,
                    share_aggregator,
                }
            },
            item @ (DecItem::PendingDecision { .. } | DecItem::Decided { .. }) => item,
        };
        let _ = std::mem::replace(self, new_item);
    }

    fn get_all_shares_authors(&self) -> Option<HashSet<Author>> {
        match self {
            DecItem::PendingDecision {
                share_aggregator, ..
            } => Some(share_aggregator.shares.keys().cloned().collect()),
            DecItem::Decided { .. } => None,
            DecItem::PendingMetadata(_) => {
                unreachable!("Should only be called after block is added")
            },
        }
    }

    fn get_self_share(&self) -> Option<DecShare> {
        match self {
            DecItem::PendingMetadata(aggr) => aggr.get_self_share(),
            DecItem::PendingDecision {
                share_aggregator, ..
            } => share_aggregator.get_self_share(),
            DecItem::Decided { self_share, .. } => Some(self_share.clone()),
        }
    }
}

pub struct DecStore {
    epoch: u64,
    author: Author,
    dec_config: DecConfig,
    dec_map: HashMap<Round, DecItem>,
    fast_dec_config: DecConfig,
    fast_dec_map: HashMap<Round, DecItem>,
    highest_known_round: u64,
    decision_tx: Sender<DecKey>,
}

impl DecStore {
    pub fn new(
        epoch: u64,
        author: Author,
        dec_config: DecConfig,
        fast_dec_config: DecConfig,
        decision_tx: Sender<DecKey>,
    ) -> Self {
        Self {
            epoch,
            author,
            dec_config,
            dec_map: HashMap::new(),
            fast_dec_config,
            fast_dec_map: HashMap::new(),
            highest_known_round: 0,
            decision_tx,
        }
    }

    pub fn update_highest_known_round(&mut self, round: u64) {
        self.highest_known_round = std::cmp::max(self.highest_known_round, round);
    }

    pub fn add_dec_metadata(&mut self, metadata: DecMetadata) {
        let dec_item = self
            .dec_map
            .entry(metadata.round)
            .or_insert_with(|| DecItem::new(self.author, PathType::Slow));
        dec_item.add_metadata(&self.dec_config, metadata.clone());
        dec_item.try_aggregate(&self.dec_config, self.decision_tx.clone());
        // fast path
        let fast_dec_item = self
            .fast_dec_map
            .entry(metadata.round)
            .or_insert_with(|| DecItem::new(self.author, PathType::Fast));
        fast_dec_item.add_metadata(&self.fast_dec_config, metadata.clone());
        fast_dec_item.try_aggregate(&self.fast_dec_config, self.decision_tx.clone());
    }

    pub fn add_share(&mut self, share: DecShare, path: PathType) -> anyhow::Result<bool> {
        ensure!(
            share.metadata.epoch == self.epoch,
            "Share from different epoch"
        );
        ensure!(
            share.metadata.round <= self.highest_known_round + FUTURE_ROUNDS_TO_ACCEPT,
            "Share from future round"
        );
        let metadata = share.metadata.clone();

        let (dec_config, dec_item) = if path == PathType::Fast {
            (&self.fast_dec_config, self.fast_dec_map.entry(metadata.round).or_insert_with(|| DecItem::new(self.author, path)))
        } else {
            (
                &self.dec_config,
                self.dec_map
                    .entry(metadata.round)
                    .or_insert_with(|| DecItem::new(self.author, PathType::Slow)),
            )
        };

        dec_item.add_share(share, dec_config)?;
        dec_item.try_aggregate(dec_config, self.decision_tx.clone());
        Ok(dec_item.has_decision())
    }

    /// This should only be called after the block is added, returns None if already decided
    /// Otherwise returns existing shares' authors
    pub fn get_all_shares_authors(&self, metadata: &DecMetadata) -> Option<HashSet<Author>> {
        self.dec_map
            .get(&metadata.round)
            .and_then(|item| item.get_all_shares_authors())
    }

    pub fn get_self_share(
        &mut self,
        metadata: &DecMetadata,
    ) -> anyhow::Result<Option<DecShare>> {
        ensure!(
            metadata.round <= self.highest_known_round,
            "Request share from future round {}, highest known round {}",
            metadata.round,
            self.highest_known_round
        );
        Ok(self
            .dec_map
            .get(&metadata.round)
            .and_then(|item| item.get_self_share())
            .filter(|share| &share.metadata == metadata))
    }
}
