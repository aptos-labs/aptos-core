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
use aptos_logger::warn;
use aptos_types::secret_sharing::{
    SecretShare, SecretShareConfig, SecretShareKey, SecretShareMetadata,
};
use itertools::Either;
use std::collections::{HashMap, HashSet};

pub struct SecretShareAggregator {
    author: Author,
    shares: HashMap<Author, SecretShare>,
    total_weight: u64,
}

impl SecretShareAggregator {
    pub fn new(author: Author) -> Self {
        Self {
            author,
            shares: HashMap::new(),
            total_weight: 0,
        }
    }

    pub fn add_share(&mut self, weight: u64, share: SecretShare) {
        if self.shares.insert(share.author, share).is_none() {
            self.total_weight += weight;
        }
    }

    pub fn try_aggregate(
        self,
        secret_share_config: &SecretShareConfig,
        metadata: SecretShareMetadata,
        decision_tx: Sender<SecretShareKey>,
    ) -> Either<Self, SecretShare> {
        if self.total_weight < secret_share_config.threshold() {
            return Either::Left(self);
        }
        observe_block(
            metadata.timestamp,
            BlockStage::SECRET_SHARING_ADD_ENOUGH_SHARE,
        );
        let dec_config = secret_share_config.clone();
        let self_share = self
            .get_self_share()
            .expect("Aggregated item should have self share");
        tokio::task::spawn_blocking(move || {
            let maybe_key = SecretShare::aggregate(self.shares.values(), &dec_config);
            match maybe_key {
                Ok(key) => {
                    let dec_key = SecretShareKey::new(metadata, key);
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

    fn retain(&mut self, dec_config: &SecretShareConfig, metadata: &SecretShareMetadata) {
        self.shares.retain(|_, share| share.metadata == *metadata);
        self.total_weight = self
            .shares
            .keys()
            .map(|author| dec_config.get_peer_weight(author))
            .sum();
    }

    fn get_self_share(&self) -> Option<SecretShare> {
        self.shares.get(&self.author).cloned()
    }

    fn total_weights(&self) -> u64 {
        self.total_weight
    }
}

enum SecretShareItem {
    PendingMetadata(SecretShareAggregator),
    PendingDecision {
        metadata: SecretShareMetadata,
        share_aggregator: SecretShareAggregator,
    },
    Decided {
        self_share: SecretShare,
    },
}

impl SecretShareItem {
    fn new(author: Author) -> Self {
        Self::PendingMetadata(SecretShareAggregator::new(author))
    }

    fn total_weights(&self) -> Option<u64> {
        match self {
            SecretShareItem::PendingMetadata(aggr) => Some(aggr.total_weights()),
            SecretShareItem::PendingDecision {
                share_aggregator, ..
            } => Some(share_aggregator.total_weights()),
            SecretShareItem::Decided { .. } => None,
        }
    }

    fn has_decision(&self) -> bool {
        matches!(self, SecretShareItem::Decided { .. })
    }

    fn add_share(
        &mut self,
        share: SecretShare,
        dec_config: &SecretShareConfig,
    ) -> anyhow::Result<()> {
        match self {
            SecretShareItem::PendingMetadata(aggr) => {
                aggr.add_share(dec_config.get_peer_weight(&share.author), share);
                Ok(())
            },
            SecretShareItem::PendingDecision {
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
            SecretShareItem::Decided { .. } => Ok(()),
        }
    }

    fn try_aggregate(
        &mut self,
        secret_share_config: &SecretShareConfig,
        decision_tx: Sender<SecretShareKey>,
    ) {
        let item = std::mem::replace(self, Self::new(Author::ONE));
        let new_item = match item {
            SecretShareItem::PendingDecision {
                share_aggregator,
                metadata,
            } => match share_aggregator.try_aggregate(
                secret_share_config,
                metadata.clone(),
                decision_tx,
            ) {
                Either::Left(share_aggregator) => Self::PendingDecision {
                    metadata,
                    share_aggregator,
                },
                Either::Right(self_share) => Self::Decided { self_share },
            },
            item @ (SecretShareItem::Decided { .. } | SecretShareItem::PendingMetadata(_)) => item,
        };
        let _ = std::mem::replace(self, new_item);
    }

    fn add_metadata(&mut self, config: &SecretShareConfig, metadata: SecretShareMetadata) {
        let item = std::mem::replace(self, Self::new(Author::ONE));
        let new_item = match item {
            SecretShareItem::PendingMetadata(mut share_aggregator) => {
                share_aggregator.retain(config, &metadata);
                Self::PendingDecision {
                    metadata,
                    share_aggregator,
                }
            },
            item @ (SecretShareItem::PendingDecision { .. } | SecretShareItem::Decided { .. }) => {
                item
            },
        };
        let _ = std::mem::replace(self, new_item);
    }

    fn get_all_shares_authors(&self) -> Option<HashSet<Author>> {
        match self {
            SecretShareItem::PendingDecision {
                share_aggregator, ..
            } => Some(share_aggregator.shares.keys().cloned().collect()),
            SecretShareItem::Decided { .. } => None,
            SecretShareItem::PendingMetadata(_) => {
                unreachable!("Should only be called after block is added")
            },
        }
    }

    fn get_self_share(&self) -> Option<SecretShare> {
        match self {
            SecretShareItem::PendingMetadata(aggr) => aggr.get_self_share(),
            SecretShareItem::PendingDecision {
                share_aggregator, ..
            } => share_aggregator.get_self_share(),
            SecretShareItem::Decided { self_share, .. } => Some(self_share.clone()),
        }
    }
}

pub struct SecretShareStore {
    epoch: u64,
    author: Author,
    secret_share_config: SecretShareConfig,
    secret_share_map: HashMap<Round, SecretShareItem>,
    highest_known_round: u64,
    decision_tx: Sender<SecretShareKey>,
}

impl SecretShareStore {
    pub fn new(
        epoch: u64,
        author: Author,
        dec_config: SecretShareConfig,
        decision_tx: Sender<SecretShareKey>,
    ) -> Self {
        Self {
            epoch,
            author,
            secret_share_config: dec_config,
            secret_share_map: HashMap::new(),
            highest_known_round: 0,
            decision_tx,
        }
    }

    pub fn update_highest_known_round(&mut self, round: u64) {
        self.highest_known_round = std::cmp::max(self.highest_known_round, round);
    }

    pub fn add_secret_share_metadata(&mut self, metadata: SecretShareMetadata) {
        let item = self
            .secret_share_map
            .entry(metadata.round)
            .or_insert_with(|| SecretShareItem::new(self.author));
        item.add_metadata(&self.secret_share_config, metadata.clone());
        item.try_aggregate(&self.secret_share_config, self.decision_tx.clone());
    }

    pub fn add_share(&mut self, share: SecretShare) -> anyhow::Result<bool> {
        ensure!(
            share.metadata.epoch == self.epoch,
            "Share from different epoch"
        );
        ensure!(
            share.metadata.round <= self.highest_known_round + FUTURE_ROUNDS_TO_ACCEPT,
            "Share from future round"
        );
        let metadata = share.metadata.clone();

        let (config, item) = (
            &self.secret_share_config,
            self.secret_share_map
                .entry(metadata.round)
                .or_insert_with(|| SecretShareItem::new(self.author)),
        );

        item.add_share(share, config)?;
        item.try_aggregate(config, self.decision_tx.clone());
        Ok(item.has_decision())
    }

    /// This should only be called after the block is added, returns None if already decided
    /// Otherwise returns existing shares' authors
    pub fn get_all_shares_authors(
        &self,
        metadata: &SecretShareMetadata,
    ) -> Option<HashSet<Author>> {
        self.secret_share_map
            .get(&metadata.round)
            .and_then(|item| item.get_all_shares_authors())
    }

    pub fn get_self_share(
        &mut self,
        metadata: &SecretShareMetadata,
    ) -> anyhow::Result<Option<SecretShare>> {
        ensure!(
            metadata.round <= self.highest_known_round,
            "Request share from future round {}, highest known round {}",
            metadata.round,
            self.highest_known_round
        );
        Ok(self
            .secret_share_map
            .get(&metadata.round)
            .and_then(|item| item.get_self_share())
            .filter(|share| &share.metadata == metadata))
    }
}
