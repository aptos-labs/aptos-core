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
use std::collections::{BTreeMap, HashMap, HashSet};

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

/// Per-epoch store that tracks secret share aggregation state for each round.
/// Remote shares can accumulate here while the self-share derivation is still
/// in flight. Once enough shares arrive and the self share is added,
/// aggregation produces a `SecretSharedKey` sent via `decision_tx`.
///
/// Note: there is no garbage collection of items after they're decided. They
/// are kept around until the epoch ends.
pub struct SecretShareStore {
    epoch: u64,
    self_author: Author,
    secret_share_config: SecretShareConfig,
    secret_share_map: BTreeMap<Round, SecretShareItem>,
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
            secret_share_map: BTreeMap::new(),
            highest_known_round: 0,
            decision_tx,
        }
    }

    pub fn update_highest_known_round(&mut self, round: u64) {
        self.highest_known_round = std::cmp::max(self.highest_known_round, round);
    }

    pub fn reset(&mut self, round: u64) {
        self.update_highest_known_round(round);
        // remove future rounds items in case they're already decided
        // otherwise if the block re-enters the queue, it'll be stuck
        let _ = self.secret_share_map.split_off(&round);
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
        let weight = self.secret_share_config.get_peer_weight(share.author())?;
        let metadata = share.metadata();
        ensure!(metadata.epoch == self.epoch, "Share from different epoch");
        ensure!(
            metadata.round <= self.highest_known_round + FUTURE_ROUNDS_TO_ACCEPT,
            "Share from future round"
        );

        // TODO(ibalajiarun): Make sure to garbage collect the items after they're decided.
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rand::secret_sharing::test_utils::{
        create_metadata, create_secret_share, TestContext,
    };
    use aptos_types::secret_sharing::SecretSharedKey;
    use futures_channel::mpsc::{unbounded, UnboundedReceiver};

    fn make_store(ctx: &TestContext) -> (SecretShareStore, UnboundedReceiver<SecretSharedKey>) {
        let (tx, rx) = unbounded();
        let store = SecretShareStore::new(
            ctx.epoch,
            ctx.authors[0],
            ctx.secret_share_config.clone(),
            tx,
        );
        (store, rx)
    }

    #[test]
    fn test_store_update_highest_known_round() {
        let ctx = TestContext::new(vec![1, 1, 1, 1]);
        let (mut store, _rx) = make_store(&ctx);

        assert_eq!(store.highest_known_round, 0);
        store.update_highest_known_round(5);
        assert_eq!(store.highest_known_round, 5);
        // Should take max
        store.update_highest_known_round(3);
        assert_eq!(store.highest_known_round, 5);
        store.update_highest_known_round(10);
        assert_eq!(store.highest_known_round, 10);
    }

    #[test]
    fn test_store_add_self_share_validation() {
        let ctx = TestContext::new(vec![1, 1, 1, 1]);
        let (mut store, _rx) = make_store(&ctx);
        store.update_highest_known_round(10);

        // Wrong epoch fails
        let bad_epoch_meta = create_metadata(99, 5);
        let share = create_secret_share(&ctx, 0, &bad_epoch_meta);
        assert!(store.add_self_share(share).is_err());

        // Round too far in future fails (> highest + 200)
        let far_future_meta = create_metadata(ctx.epoch, 10 + FUTURE_ROUNDS_TO_ACCEPT + 1);
        let share = create_secret_share(&ctx, 0, &far_future_meta);
        assert!(store.add_self_share(share).is_err());

        // Valid round succeeds
        let valid_meta = create_metadata(ctx.epoch, 5);
        let share = create_secret_share(&ctx, 0, &valid_meta);
        assert!(store.add_self_share(share).is_ok());
    }

    #[test]
    fn test_store_add_share_validation() {
        let ctx = TestContext::new(vec![1, 1, 1, 1]);
        let (mut store, _rx) = make_store(&ctx);
        store.update_highest_known_round(10);

        // Wrong epoch fails
        let bad_epoch_meta = create_metadata(99, 5);
        let share = create_secret_share(&ctx, 1, &bad_epoch_meta);
        assert!(store.add_share(share).is_err());

        // Future round fails
        let far_future_meta = create_metadata(ctx.epoch, 10 + FUTURE_ROUNDS_TO_ACCEPT + 1);
        let share = create_secret_share(&ctx, 1, &far_future_meta);
        assert!(store.add_share(share).is_err());

        // Valid share succeeds
        let valid_meta = create_metadata(ctx.epoch, 5);
        let share = create_secret_share(&ctx, 1, &valid_meta);
        assert!(store.add_share(share).is_ok());
    }

    #[tokio::test]
    async fn test_store_self_share_then_peer_shares() {
        let ctx = TestContext::new(vec![1, 1, 1, 1]);
        let (mut store, mut rx) = make_store(&ctx);
        let round = 5;
        store.update_highest_known_round(round);
        let metadata = create_metadata(ctx.epoch, round);

        // Add self share first -> PendingDecision
        let self_share = create_secret_share(&ctx, 0, &metadata);
        store.add_self_share(self_share).unwrap();

        // Add peer shares until aggregation triggers
        for i in 1..ctx.authors.len() {
            let share = create_secret_share(&ctx, i, &metadata);
            let decided = store.add_share(share).unwrap();
            if decided {
                break;
            }
        }

        // Verify decision arrives on channel
        use futures::StreamExt;
        let key = tokio::time::timeout(std::time::Duration::from_secs(5), rx.next())
            .await
            .expect("Timed out waiting for decision")
            .expect("Channel closed unexpectedly");
        assert_eq!(key.metadata, metadata);
    }

    #[tokio::test]
    async fn test_store_peer_shares_then_self_share() {
        let ctx = TestContext::new(vec![1, 1, 1, 1]);
        let (mut store, mut rx) = make_store(&ctx);
        let round = 5;
        store.update_highest_known_round(round);
        let metadata = create_metadata(ctx.epoch, round);

        // Add peer shares first (PendingMetadata accumulates)
        for i in 1..ctx.authors.len() {
            let share = create_secret_share(&ctx, i, &metadata);
            store.add_share(share).unwrap();
        }

        // Add self share with metadata -> triggers transition + aggregation
        let self_share = create_secret_share(&ctx, 0, &metadata);
        store.add_self_share(self_share).unwrap();

        // Verify decision arrives on channel
        use futures::StreamExt;
        let key = tokio::time::timeout(std::time::Duration::from_secs(5), rx.next())
            .await
            .expect("Timed out waiting for decision")
            .expect("Channel closed unexpectedly");
        assert_eq!(key.metadata, metadata);
    }

    #[test]
    fn test_store_get_all_shares_authors() {
        let ctx = TestContext::new(vec![1, 1, 1, 1]);
        let (mut store, _rx) = make_store(&ctx);
        let round = 5;
        store.update_highest_known_round(round);
        let metadata = create_metadata(ctx.epoch, round);

        // Add self share
        let self_share = create_secret_share(&ctx, 0, &metadata);
        store.add_self_share(self_share).unwrap();

        // Add one peer share
        let peer_share = create_secret_share(&ctx, 1, &metadata);
        store.add_share(peer_share).unwrap();

        // Should return authors who have contributed shares
        let authors = store.get_all_shares_authors(&metadata).unwrap();
        assert!(authors.contains(&ctx.authors[0]));
        assert!(authors.contains(&ctx.authors[1]));
        assert_eq!(authors.len(), 2);
    }

    #[test]
    fn test_store_get_self_share() {
        let ctx = TestContext::new(vec![1, 1, 1, 1]);
        let (mut store, _rx) = make_store(&ctx);
        let round = 5;
        store.update_highest_known_round(round);
        let metadata = create_metadata(ctx.epoch, round);

        // Future round errors
        let future_meta = create_metadata(ctx.epoch, round + 1);
        assert!(store.get_self_share(&future_meta).is_err());

        // No share yet -> None
        assert!(store.get_self_share(&metadata).unwrap().is_none());

        // Add self share
        let self_share = create_secret_share(&ctx, 0, &metadata);
        store.add_self_share(self_share).unwrap();

        // Matching metadata returns share
        let retrieved = store.get_self_share(&metadata).unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().author, ctx.authors[0]);

        // Mismatched metadata returns None
        let other_meta = create_metadata(ctx.epoch, round);
        assert!(store.get_self_share(&other_meta).unwrap().is_none());
    }
}
