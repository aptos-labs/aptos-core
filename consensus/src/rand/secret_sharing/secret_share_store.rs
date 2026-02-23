// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    block_storage::tracing::{observe_block, BlockStage},
    rand::rand_gen::{rand_manager::Sender, types::FUTURE_ROUNDS_TO_ACCEPT},
};
use anyhow::{bail, ensure};
use aptos_consensus_types::common::{Author, Round};
use aptos_logger::warn;
use aptos_types::secret_sharing::{
    SecretShare, SecretShareConfig, SecretShareMetadata, SecretSharedKey,
};
use itertools::Either;
use std::collections::{HashMap, HashSet};

pub struct SecretShareAggregator {
    self_author: Author,
    shares: HashMap<Author, SecretShare>,
    total_weight: u64,
}

impl SecretShareAggregator {
    pub fn new(self_author: Author) -> Self {
        Self {
            self_author,
            shares: HashMap::new(),
            total_weight: 0,
        }
    }

    pub fn add_share(&mut self, share: SecretShare, weight: u64) {
        if self.shares.insert(share.author, share).is_none() {
            self.total_weight += weight;
        }
    }

    pub fn try_aggregate(
        self,
        secret_share_config: &SecretShareConfig,
        metadata: SecretShareMetadata,
        decision_tx: Sender<SecretSharedKey>,
    ) -> Either<Self, SecretShare> {
        if self.total_weight < secret_share_config.threshold() {
            return Either::Left(self);
        }
        observe_block(
            metadata.timestamp,
            BlockStage::SECRET_SHARING_ADD_ENOUGH_SHARE,
        );
        let dec_config = secret_share_config.clone();
        let self_share = match self.get_self_share() {
            Some(share) => share,
            None => {
                warn!("Aggregation threshold met but self share missing");
                return Either::Left(self);
            },
        };
        tokio::task::spawn_blocking(move || {
            let maybe_key = SecretShare::aggregate(self.shares.values(), &dec_config);
            match maybe_key {
                Ok(key) => {
                    let dec_key = SecretSharedKey::new(metadata, key);
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

    fn retain(&mut self, metadata: &SecretShareMetadata, weights: &HashMap<Author, u64>) {
        self.shares.retain(|_, share| share.metadata == *metadata);
        self.total_weight = self
            .shares
            .keys()
            .filter_map(|author| weights.get(author))
            .sum();
    }

    fn get_self_share(&self) -> Option<SecretShare> {
        self.shares.get(&self.self_author).cloned()
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

    fn has_decision(&self) -> bool {
        matches!(self, SecretShareItem::Decided { .. })
    }

    fn add_share(&mut self, share: SecretShare, share_weight: u64) -> anyhow::Result<()> {
        match self {
            SecretShareItem::PendingMetadata(aggr) => {
                aggr.add_share(share, share_weight);
                Ok(())
            },
            SecretShareItem::PendingDecision {
                metadata,
                share_aggregator,
            } => {
                ensure!(
                    metadata == &share.metadata,
                    "[SecretShareItem] SecretShare metadata from {} mismatch with block metadata!",
                    share.author,
                );
                share_aggregator.add_share(share, share_weight);
                Ok(())
            },
            SecretShareItem::Decided { .. } => Ok(()),
        }
    }

    fn try_aggregate(
        &mut self,
        secret_share_config: &SecretShareConfig,
        decision_tx: Sender<SecretSharedKey>,
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

    fn add_share_with_metadata(
        &mut self,
        share: SecretShare,
        share_weights: &HashMap<Author, u64>,
    ) -> anyhow::Result<()> {
        let item = std::mem::replace(self, Self::new(Author::ONE));
        let share_weight = *share_weights
            .get(share.author())
            .ok_or_else(|| anyhow::anyhow!("Author {} not found in weights", share.author()))?;
        let new_item = match item {
            SecretShareItem::PendingMetadata(mut share_aggregator) => {
                let metadata = share.metadata.clone();
                share_aggregator.retain(share.metadata(), share_weights);
                share_aggregator.add_share(share, share_weight);
                SecretShareItem::PendingDecision {
                    metadata,
                    share_aggregator,
                }
            },
            SecretShareItem::PendingDecision { .. } => {
                bail!("Cannot add self share in PendingDecision state");
            },
            SecretShareItem::Decided { .. } => return Ok(()),
        };
        let _ = std::mem::replace(self, new_item);
        Ok(())
    }

    fn get_all_shares_authors(&self) -> Option<HashSet<Author>> {
        match self {
            SecretShareItem::PendingDecision {
                share_aggregator, ..
            } => Some(share_aggregator.shares.keys().cloned().collect()),
            SecretShareItem::Decided { .. } => None,
            SecretShareItem::PendingMetadata(_) => None,
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
    self_author: Author,
    secret_share_config: SecretShareConfig,
    secret_share_map: HashMap<Round, SecretShareItem>,
    highest_known_round: u64,
    decision_tx: Sender<SecretSharedKey>,
}

impl SecretShareStore {
    pub fn new(
        epoch: u64,
        author: Author,
        dec_config: SecretShareConfig,
        decision_tx: Sender<SecretSharedKey>,
    ) -> Self {
        Self {
            epoch,
            self_author: author,
            secret_share_config: dec_config,
            secret_share_map: HashMap::new(),
            highest_known_round: 0,
            decision_tx,
        }
    }

    pub fn update_highest_known_round(&mut self, round: u64) {
        self.highest_known_round = std::cmp::max(self.highest_known_round, round);
    }

    pub fn add_self_share(&mut self, share: SecretShare) -> anyhow::Result<()> {
        ensure!(
            self.self_author == share.author,
            "Only self shares can be added with metadata"
        );
        let peer_weights = self.secret_share_config.get_peer_weights();
        let metadata = share.metadata();
        ensure!(metadata.epoch == self.epoch, "Share from different epoch");
        ensure!(
            metadata.round <= self.highest_known_round + FUTURE_ROUNDS_TO_ACCEPT,
            "Share from future round"
        );

        let item = self
            .secret_share_map
            .entry(metadata.round)
            .or_insert_with(|| SecretShareItem::new(self.self_author));
        item.add_share_with_metadata(share, peer_weights)?;
        item.try_aggregate(&self.secret_share_config, self.decision_tx.clone());
        Ok(())
    }

    pub fn add_share(&mut self, share: SecretShare) -> anyhow::Result<bool> {
        let weight = self.secret_share_config.get_peer_weight(share.author());
        let metadata = share.metadata();
        ensure!(metadata.epoch == self.epoch, "Share from different epoch");
        ensure!(
            metadata.round <= self.highest_known_round + FUTURE_ROUNDS_TO_ACCEPT,
            "Share from future round"
        );

        let item = self
            .secret_share_map
            .entry(metadata.round)
            .or_insert_with(|| SecretShareItem::new(self.self_author));
        item.add_share(share, weight)?;
        item.try_aggregate(&self.secret_share_config, self.decision_tx.clone());
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
