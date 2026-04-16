// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    block_storage::tracing::{observe_block, BlockStage},
    monitor,
    rand::{
        rand_gen::{rand_manager::Sender, types::FUTURE_ROUNDS_TO_ACCEPT},
        secret_sharing::verifier::SecretShareVerifier,
    },
};
use anyhow::{bail, ensure};
use aptos_batch_encryption::{
    schemes::fptx_weighted::FPTXWeighted, traits::BatchThresholdEncryption,
};
use aptos_consensus_types::common::{Author, Round};
use aptos_logger::warn;
use aptos_types::secret_sharing::{
    DecryptionKey, SecretShare, SecretShareMetadata, SecretSharedKey,
};
use itertools::Either;
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    sync::Arc,
};

pub enum SecretShareAggregationResult {
    Success(SecretSharedKey),
    Failure {
        round: Round,
        epoch: u64,
        metadata: SecretShareMetadata,
        surviving_shares: HashMap<Author, SecretShare>,
    },
}

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
        mut self,
        verifier: &Arc<SecretShareVerifier>,
        metadata: SecretShareMetadata,
        decision_tx: Sender<SecretShareAggregationResult>,
    ) -> Either<Self, SecretShare> {
        if self.total_weight < verifier.config().threshold() {
            return Either::Left(self);
        }
        observe_block(
            metadata.timestamp,
            BlockStage::SECRET_SHARING_ADD_ENOUGH_SHARE,
        );
        let self_share = match self.get_self_share() {
            Some(share) => share,
            None => {
                warn!("Aggregation threshold met but self share missing");
                return Either::Left(self);
            },
        };

        let verifier = verifier.clone();
        tokio::task::spawn_blocking(move || {
            let round = metadata.round;
            let epoch = metadata.epoch;

            match Self::aggregate_and_verify(&verifier, &mut self.shares, &metadata) {
                Ok(verified_key) => {
                    let dec_key = SecretSharedKey::new(metadata, verified_key);
                    let _ =
                        decision_tx.unbounded_send(SecretShareAggregationResult::Success(dec_key));
                },
                Err(e) => {
                    warn!(
                        epoch = epoch,
                        round = round,
                        "Aggregate-and-verify failed, evicting bad shares: {e}"
                    );

                    verifier.evict_bad_shares(&mut self.shares);
                    let remaining_weight: u64 = self
                        .shares
                        .keys()
                        .filter_map(|a| verifier.config().get_peer_weight(a).ok())
                        .sum();
                    if remaining_weight < verifier.config().threshold() {
                        warn!(
                            epoch = epoch,
                            round = round,
                            "Remaining weight {} below threshold {} after eviction",
                            remaining_weight,
                            verifier.config().threshold()
                        );
                        let _ = decision_tx.unbounded_send(SecretShareAggregationResult::Failure {
                            round,
                            epoch,
                            metadata: metadata.clone(),
                            surviving_shares: self.shares,
                        });
                        return;
                    }

                    match Self::aggregate_and_verify(&verifier, &mut self.shares, &metadata) {
                        Ok(verified_key) => {
                            let dec_key = SecretSharedKey::new(metadata, verified_key);
                            let _ = decision_tx
                                .unbounded_send(SecretShareAggregationResult::Success(dec_key));
                        },
                        Err(e) => {
                            warn!(
                                epoch = epoch,
                                round = round,
                                "Retry after eviction also failed: {e}"
                            );
                            let _ =
                                decision_tx.unbounded_send(SecretShareAggregationResult::Failure {
                                    round,
                                    epoch,
                                    metadata,
                                    surviving_shares: self.shares,
                                });
                        },
                    }
                },
            }
        });
        Either::Right(self_share)
    }

    fn aggregate_and_verify(
        verifier: &SecretShareVerifier,
        shares: &mut HashMap<Author, SecretShare>,
        metadata: &SecretShareMetadata,
    ) -> anyhow::Result<DecryptionKey> {
        let key = monitor!(
            "secret_share_aggregate",
            SecretShare::aggregate(shares.values(), verifier.config())
        )?;
        monitor!(
            "secret_share_post_aggregate_verify",
            FPTXWeighted::verify_decryption_key(
                verifier.config().encryption_key(),
                &metadata.digest,
                &key,
            )
        )?;
        Ok(key)
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
    Aggregating {
        metadata: SecretShareMetadata,
        self_share: SecretShare,
        pending_shares: HashMap<Author, (SecretShare, u64)>,
    },
    Decided {
        self_share: SecretShare,
    },
    /// Round had no encrypted txns; key derivation skipped, further shares rejected.
    Skipped,
}

impl SecretShareItem {
    fn new(author: Author) -> Self {
        Self::PendingMetadata(SecretShareAggregator::new(author))
    }

    fn has_decision(&self) -> bool {
        matches!(
            self,
            SecretShareItem::Aggregating { .. } | SecretShareItem::Decided { .. }
        )
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
            SecretShareItem::Aggregating {
                metadata,
                pending_shares,
                ..
            } => {
                ensure!(
                    metadata == &share.metadata,
                    "[SecretShareItem] SecretShare metadata from {} mismatch with block metadata!",
                    share.author,
                );
                pending_shares.insert(*share.author(), (share, share_weight));
                Ok(())
            },
            SecretShareItem::Decided { .. } => Ok(()),
            SecretShareItem::Skipped => {
                bail!("Received share for skipped round")
            },
        }
    }

    fn try_aggregate(
        &mut self,
        verifier: &Arc<SecretShareVerifier>,
        decision_tx: Sender<SecretShareAggregationResult>,
    ) {
        let item = std::mem::replace(self, Self::new(Author::ONE));
        let new_item = match item {
            SecretShareItem::PendingDecision {
                share_aggregator,
                metadata,
            } => match share_aggregator.try_aggregate(verifier, metadata.clone(), decision_tx) {
                Either::Left(share_aggregator) => Self::PendingDecision {
                    metadata,
                    share_aggregator,
                },
                Either::Right(self_share) => Self::Aggregating {
                    metadata,
                    self_share,
                    pending_shares: HashMap::new(),
                },
            },
            item @ (SecretShareItem::Decided { .. }
            | SecretShareItem::PendingMetadata(_)
            | SecretShareItem::Aggregating { .. }) => item,
            SecretShareItem::Skipped => {
                warn!("try_aggregate called on skipped round — logic bug");
                SecretShareItem::Skipped
            },
        };
        let _ = std::mem::replace(self, new_item);
    }

    fn aggregation_succeeded(&mut self) {
        let item = std::mem::replace(self, Self::new(Author::ONE));
        let new_item = match item {
            SecretShareItem::Aggregating { self_share, .. } => Self::Decided { self_share },
            other @ (SecretShareItem::PendingMetadata(_)
            | SecretShareItem::PendingDecision { .. }
            | SecretShareItem::Decided { .. }) => other,
            SecretShareItem::Skipped => {
                warn!("aggregation_succeeded called on skipped round — logic bug");
                SecretShareItem::Skipped
            },
        };
        let _ = std::mem::replace(self, new_item);
    }

    fn aggregation_failed(
        &mut self,
        verifier: &Arc<SecretShareVerifier>,
        surviving_shares: HashMap<Author, SecretShare>,
    ) {
        let item = std::mem::replace(self, Self::new(Author::ONE));
        let new_item = match item {
            SecretShareItem::Aggregating {
                metadata,
                self_share,
                pending_shares,
            } => {
                let mut aggregator = SecretShareAggregator::new(*self_share.author());
                // Add pending (unverified) first, then surviving (verified).
                // HashMap::insert overwrites, so verified shares take priority.
                for (_author, (share, weight)) in pending_shares {
                    aggregator.add_share(share, weight);
                }
                for (_, share) in surviving_shares {
                    let weight = verifier
                        .config()
                        .get_peer_weight(share.author())
                        .unwrap_or(0);
                    aggregator.add_share(share, weight);
                }
                Self::PendingDecision {
                    metadata,
                    share_aggregator: aggregator,
                }
            },
            other @ (SecretShareItem::PendingMetadata(_)
            | SecretShareItem::PendingDecision { .. }
            | SecretShareItem::Decided { .. }) => other,
            SecretShareItem::Skipped => {
                warn!("aggregation_failed called on skipped round — logic bug");
                SecretShareItem::Skipped
            },
        };
        let _ = std::mem::replace(self, new_item);
    }

    fn add_share_with_metadata(
        &mut self,
        share: SecretShare,
        share_weights: &HashMap<Author, u64>,
    ) -> anyhow::Result<()> {
        match self {
            SecretShareItem::PendingMetadata(_) => {
                let share_weight = *share_weights.get(share.author()).ok_or_else(|| {
                    anyhow::anyhow!("Author {} not found in weights", share.author())
                })?;
                let SecretShareItem::PendingMetadata(mut share_aggregator) =
                    std::mem::replace(self, Self::new(Author::ONE))
                else {
                    unreachable!("variant gated above")
                };
                let metadata = share.metadata.clone();
                share_aggregator.retain(share.metadata(), share_weights);
                share_aggregator.add_share(share, share_weight);
                *self = SecretShareItem::PendingDecision {
                    metadata,
                    share_aggregator,
                };
                Ok(())
            },
            SecretShareItem::PendingDecision { .. } => {
                bail!("Cannot add self share in PendingDecision state")
            },
            SecretShareItem::Aggregating { .. } | SecretShareItem::Decided { .. } => Ok(()),
            SecretShareItem::Skipped => {
                bail!("Cannot add self share for skipped round")
            },
        }
    }

    fn get_all_shares_authors(&self) -> Option<HashSet<Author>> {
        match self {
            SecretShareItem::PendingDecision {
                share_aggregator, ..
            } => Some(share_aggregator.shares.keys().cloned().collect()),
            SecretShareItem::Aggregating { .. }
            | SecretShareItem::Decided { .. }
            | SecretShareItem::PendingMetadata(_)
            | SecretShareItem::Skipped => None,
        }
    }

    fn get_self_share(&self) -> Option<SecretShare> {
        match self {
            SecretShareItem::PendingMetadata(aggr) => aggr.get_self_share(),
            SecretShareItem::PendingDecision {
                share_aggregator, ..
            } => share_aggregator.get_self_share(),
            SecretShareItem::Aggregating { self_share, .. }
            | SecretShareItem::Decided { self_share, .. } => Some(self_share.clone()),
            SecretShareItem::Skipped => None,
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
    verifier: Arc<SecretShareVerifier>,
    secret_share_map: BTreeMap<Round, SecretShareItem>,
    highest_known_round: u64,
    decision_tx: Sender<SecretShareAggregationResult>,
}

impl SecretShareStore {
    pub fn new(
        epoch: u64,
        author: Author,
        verifier: Arc<SecretShareVerifier>,
        decision_tx: Sender<SecretShareAggregationResult>,
    ) -> Self {
        Self {
            epoch,
            self_author: author,
            verifier,
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
        let peer_weights = self.verifier.config().get_peer_weights();
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
        item.try_aggregate(&self.verifier, self.decision_tx.clone());
        Ok(())
    }

    pub fn add_share(&mut self, share: SecretShare) -> anyhow::Result<bool> {
        let weight = self.verifier.config().get_peer_weight(share.author())?;
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
        item.try_aggregate(&self.verifier, self.decision_tx.clone());
        Ok(item.has_decision())
    }

    pub fn mark_round_skipped(&mut self, round: Round) {
        if let Some(existing) = self.secret_share_map.get(&round) {
            match existing {
                SecretShareItem::PendingMetadata(_) | SecretShareItem::Skipped => {},
                SecretShareItem::PendingDecision { .. }
                | SecretShareItem::Aggregating { .. }
                | SecretShareItem::Decided { .. } => {
                    warn!(
                        round = round,
                        "mark_round_skipped overwriting active state — logic bug"
                    );
                },
            }
        }
        self.secret_share_map
            .insert(round, SecretShareItem::Skipped);
    }

    pub fn handle_aggregation_success(&mut self, round: Round) {
        if let Some(item) = self.secret_share_map.get_mut(&round) {
            item.aggregation_succeeded();
        }
    }

    pub fn handle_aggregation_failure(
        &mut self,
        round: Round,
        surviving_shares: HashMap<Author, SecretShare>,
    ) -> Option<HashSet<Author>> {
        if let Some(item) = self.secret_share_map.get_mut(&round) {
            item.aggregation_failed(&self.verifier, surviving_shares);
            item.try_aggregate(&self.verifier, self.decision_tx.clone());
            if item.has_decision() {
                None
            } else {
                item.get_all_shares_authors()
            }
        } else {
            None
        }
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
    use crate::rand::secret_sharing::{
        test_utils::{create_bad_secret_share, create_metadata, create_secret_share, TestContext},
        verifier::SecretShareVerifier,
    };
    use futures_channel::mpsc::{unbounded, UnboundedReceiver};
    use std::sync::Arc;

    fn make_store(
        ctx: &TestContext,
    ) -> (
        SecretShareStore,
        UnboundedReceiver<SecretShareAggregationResult>,
    ) {
        let (tx, rx) = unbounded();
        let verifier = Arc::new(SecretShareVerifier::new(
            ctx.secret_share_config.clone(),
            true,
        ));
        let store = SecretShareStore::new(ctx.epoch, ctx.authors[0], verifier, tx);
        (store, rx)
    }

    /// Helper to extract SecretSharedKey from aggregation result, panics on Failure.
    fn unwrap_success(result: SecretShareAggregationResult) -> SecretSharedKey {
        match result {
            SecretShareAggregationResult::Success(key) => key,
            SecretShareAggregationResult::Failure { round, epoch, .. } => {
                panic!("Expected Success but got Failure for epoch={epoch}, round={round}")
            },
        }
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
        let result = tokio::time::timeout(std::time::Duration::from_secs(5), rx.next())
            .await
            .expect("Timed out waiting for decision")
            .expect("Channel closed unexpectedly");
        assert_eq!(unwrap_success(result).metadata, metadata);
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
        let result = tokio::time::timeout(std::time::Duration::from_secs(5), rx.next())
            .await
            .expect("Timed out waiting for decision")
            .expect("Channel closed unexpectedly");
        assert_eq!(unwrap_success(result).metadata, metadata);
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

    #[tokio::test]
    async fn test_store_aggregation_with_bad_share_evicted() {
        // 5 validators, weights [1,1,1,1,1], threshold = 4
        // Add self share (0) + 3 good peer shares (1,2,3) + 1 bad peer share (4)
        // Pre-aggregate eviction should remove validator 4, leaving weight 4 >= threshold
        // Aggregation should succeed with the 4 good shares
        let ctx = TestContext::new(vec![1, 1, 1, 1, 1]);
        let (mut store, mut rx) = make_store(&ctx);
        let round = 5;
        store.update_highest_known_round(round);
        let metadata = create_metadata(ctx.epoch, round);

        // Add self share
        let self_share = create_secret_share(&ctx, 0, &metadata);
        store.add_self_share(self_share).unwrap();

        // Add 3 good peer shares
        for i in 1..=3 {
            let share = create_secret_share(&ctx, i, &metadata);
            store.add_share(share).unwrap();
        }

        // Add 1 bad peer share — this should trigger aggregation (total weight 5 >= 4)
        // Pre-aggregate eviction removes the bad share, leaving weight 4 >= 4
        let bad_share = create_bad_secret_share(&ctx, 4, &metadata);
        store.add_share(bad_share).unwrap();

        // Verify decision arrives on channel (aggregation succeeded without the bad share)
        use futures::StreamExt;
        let result = tokio::time::timeout(std::time::Duration::from_secs(5), rx.next())
            .await
            .expect("Timed out waiting for decision")
            .expect("Channel closed unexpectedly");
        assert_eq!(unwrap_success(result).metadata, metadata);
    }

    #[test]
    fn test_aggregation_failed_merges_surviving_and_pending_shares() {
        let ctx = TestContext::new(vec![1, 1, 1, 1]);
        let verifier = Arc::new(SecretShareVerifier::new(
            ctx.secret_share_config.clone(),
            true,
        ));
        let metadata = create_metadata(ctx.epoch, 5);

        let self_share = create_secret_share(&ctx, 0, &metadata);
        let surviving_share_1 = create_secret_share(&ctx, 1, &metadata);
        let pending_share_2 = create_secret_share(&ctx, 2, &metadata);

        let mut surviving = HashMap::new();
        surviving.insert(ctx.authors[0], self_share.clone());
        surviving.insert(ctx.authors[1], surviving_share_1);

        let mut pending = HashMap::new();
        let w2 = verifier.config().get_peer_weight(&ctx.authors[2]).unwrap();
        pending.insert(ctx.authors[2], (pending_share_2, w2));

        let mut item = SecretShareItem::Aggregating {
            metadata: metadata.clone(),
            self_share,
            pending_shares: pending,
        };

        item.aggregation_failed(&verifier, surviving);

        match &item {
            SecretShareItem::PendingDecision {
                share_aggregator, ..
            } => {
                assert_eq!(share_aggregator.shares.len(), 3);
                assert!(share_aggregator.shares.contains_key(&ctx.authors[0]));
                assert!(share_aggregator.shares.contains_key(&ctx.authors[1]));
                assert!(share_aggregator.shares.contains_key(&ctx.authors[2]));
            },
            other => panic!("Expected PendingDecision, got {}", match other {
                SecretShareItem::PendingMetadata(_) => "PendingMetadata",
                SecretShareItem::Aggregating { .. } => "Aggregating",
                SecretShareItem::Decided { .. } => "Decided",
                SecretShareItem::Skipped => "Skipped",
                SecretShareItem::PendingDecision { .. } => unreachable!(),
            }),
        }
    }

    #[tokio::test]
    async fn test_store_failure_recovery_with_new_share() {
        // 3 validators, weights [1,1,1], threshold = 3
        // self(0) + good(1) + bad(2) → aggregation triggers
        // eviction removes bad(2), weight 2 < 3 → Failure with surviving {0, 1}
        // handle_aggregation_failure merges surviving into PendingDecision
        // add good(2) → threshold met → aggregation succeeds
        let ctx = TestContext::new(vec![1, 1, 1]);
        let (mut store, mut rx) = make_store(&ctx);
        let round = 5;
        store.update_highest_known_round(round);
        let metadata = create_metadata(ctx.epoch, round);

        let self_share = create_secret_share(&ctx, 0, &metadata);
        store.add_self_share(self_share).unwrap();

        let good_share = create_secret_share(&ctx, 1, &metadata);
        store.add_share(good_share).unwrap();

        let bad_share = create_bad_secret_share(&ctx, 2, &metadata);
        store.add_share(bad_share).unwrap();

        use futures::StreamExt;
        let result = tokio::time::timeout(std::time::Duration::from_secs(5), rx.next())
            .await
            .expect("Timed out waiting for result")
            .expect("Channel closed unexpectedly");

        match result {
            SecretShareAggregationResult::Failure {
                surviving_shares, ..
            } => {
                assert_eq!(surviving_shares.len(), 2);
                store.handle_aggregation_failure(round, surviving_shares);
            },
            SecretShareAggregationResult::Success(_) => {
                panic!("Expected Failure but got Success")
            },
        }

        let good_share_2 = create_secret_share(&ctx, 2, &metadata);
        store.add_share(good_share_2).unwrap();

        let result = tokio::time::timeout(std::time::Duration::from_secs(5), rx.next())
            .await
            .expect("Timed out waiting for decision")
            .expect("Channel closed unexpectedly");
        assert_eq!(unwrap_success(result).metadata, metadata);
    }

    fn variant_name(item: &SecretShareItem) -> &'static str {
        match item {
            SecretShareItem::PendingMetadata(_) => "PendingMetadata",
            SecretShareItem::PendingDecision { .. } => "PendingDecision",
            SecretShareItem::Aggregating { .. } => "Aggregating",
            SecretShareItem::Decided { .. } => "Decided",
            SecretShareItem::Skipped => "Skipped",
        }
    }

    #[test]
    fn test_add_share_with_metadata_on_pending_decision_preserves_state() {
        let ctx = TestContext::new(vec![1, 1, 1, 1]);
        let metadata = create_metadata(ctx.epoch, 5);
        let peer_weights = ctx.secret_share_config.get_peer_weights().clone();

        let mut aggregator = SecretShareAggregator::new(ctx.authors[0]);
        let peer_share = create_secret_share(&ctx, 1, &metadata);
        let w1 = ctx
            .secret_share_config
            .get_peer_weight(&ctx.authors[1])
            .unwrap();
        aggregator.add_share(peer_share, w1);

        let shares_before = aggregator.shares.clone();
        let total_weight_before = aggregator.total_weight;

        let mut item = SecretShareItem::PendingDecision {
            metadata: metadata.clone(),
            share_aggregator: aggregator,
        };

        let self_share = create_secret_share(&ctx, 0, &metadata);
        let result = item.add_share_with_metadata(self_share, &peer_weights);
        assert!(result.is_err());

        match &item {
            SecretShareItem::PendingDecision {
                metadata: m,
                share_aggregator,
            } => {
                assert_eq!(m, &metadata);
                assert_eq!(share_aggregator.shares.len(), shares_before.len());
                for (author, share) in &shares_before {
                    let got = share_aggregator
                        .shares
                        .get(author)
                        .expect("share should be preserved");
                    assert_eq!(got.author, share.author);
                    assert_eq!(got.metadata, share.metadata);
                }
                assert_eq!(share_aggregator.total_weight, total_weight_before);
            },
            other => panic!("Expected PendingDecision, got {}", variant_name(other)),
        }
    }

    #[test]
    fn test_add_share_with_metadata_unknown_author_preserves_state() {
        let ctx = TestContext::new(vec![1, 1, 1, 1]);
        let metadata = create_metadata(ctx.epoch, 5);
        let peer_weights = ctx.secret_share_config.get_peer_weights().clone();

        let mut aggregator = SecretShareAggregator::new(ctx.authors[0]);
        let peer_share = create_secret_share(&ctx, 1, &metadata);
        let w1 = ctx
            .secret_share_config
            .get_peer_weight(&ctx.authors[1])
            .unwrap();
        aggregator.add_share(peer_share, w1);

        let shares_before = aggregator.shares.clone();
        let total_weight_before = aggregator.total_weight;

        let mut item = SecretShareItem::PendingMetadata(aggregator);

        let unknown_author = Author::random();
        let self_share_template = create_secret_share(&ctx, 0, &metadata);
        let unknown_share = SecretShare::new(
            unknown_author,
            metadata.clone(),
            self_share_template.share.clone(),
        );
        let result = item.add_share_with_metadata(unknown_share, &peer_weights);
        assert!(result.is_err());

        match &item {
            SecretShareItem::PendingMetadata(aggr) => {
                assert_eq!(aggr.shares.len(), shares_before.len());
                for (author, share) in &shares_before {
                    let got = aggr.shares.get(author).expect("share should be preserved");
                    assert_eq!(got.author, share.author);
                    assert_eq!(got.metadata, share.metadata);
                }
                assert_eq!(aggr.total_weight, total_weight_before);
            },
            other => panic!("Expected PendingMetadata, got {}", variant_name(other)),
        }
    }

    #[test]
    fn test_get_all_shares_authors_returns_none_for_terminal_and_preaggregation_states() {
        // The share-requester task relies on `get_all_shares_authors` returning
        // `None` for any state in which requesting more shares is useless:
        //   - Aggregating / Decided: we already have (or are finalizing) enough
        //     shares; retry is driven by `process_aggregation_result` instead.
        //   - Skipped: round has no encrypted txns.
        //   - PendingMetadata: self-share not yet derived, we have no metadata
        //     to request against.
        // Only `PendingDecision` returns `Some(known_authors)`.
        let ctx = TestContext::new(vec![1, 1, 1, 1]);
        let metadata = create_metadata(ctx.epoch, 5);

        // Aggregating
        let self_share = create_secret_share(&ctx, 0, &metadata);
        let item = SecretShareItem::Aggregating {
            metadata: metadata.clone(),
            self_share: self_share.clone(),
            pending_shares: HashMap::new(),
        };
        assert!(item.get_all_shares_authors().is_none());

        // Decided
        let item = SecretShareItem::Decided { self_share };
        assert!(item.get_all_shares_authors().is_none());

        // Skipped
        let item = SecretShareItem::Skipped;
        assert!(item.get_all_shares_authors().is_none());

        // PendingMetadata
        let item = SecretShareItem::PendingMetadata(SecretShareAggregator::new(ctx.authors[0]));
        assert!(item.get_all_shares_authors().is_none());
    }

    #[test]
    fn test_mark_round_skipped_rejects_future_shares() {
        let ctx = TestContext::new(vec![1, 1, 1, 1]);
        let (mut store, _rx) = make_store(&ctx);
        let round = 5;
        store.update_highest_known_round(round);
        let metadata = create_metadata(ctx.epoch, round);

        store.mark_round_skipped(round);
        match store
            .secret_share_map
            .get(&round)
            .expect("entry should exist after mark_round_skipped")
        {
            SecretShareItem::Skipped => {},
            other => panic!("Expected Skipped, got {}", variant_name(other)),
        }

        let peer_share = create_secret_share(&ctx, 1, &metadata);
        assert!(store.add_share(peer_share).is_err());

        match store
            .secret_share_map
            .get(&round)
            .expect("entry should still exist")
        {
            SecretShareItem::Skipped => {},
            other => panic!(
                "Expected Skipped after add_share, got {}",
                variant_name(other)
            ),
        }
    }

    #[test]
    fn test_mark_round_skipped_after_pending_metadata() {
        let ctx = TestContext::new(vec![1, 1, 1, 1]);
        let (mut store, _rx) = make_store(&ctx);
        let round = 5;
        store.update_highest_known_round(round);
        let metadata = create_metadata(ctx.epoch, round);

        let peer_share = create_secret_share(&ctx, 1, &metadata);
        store
            .add_share(peer_share)
            .expect("add_share should succeed for fresh round");

        match store
            .secret_share_map
            .get(&round)
            .expect("entry should exist after add_share")
        {
            SecretShareItem::PendingMetadata(aggr) => {
                assert_eq!(aggr.shares.len(), 1);
            },
            other => panic!("Expected PendingMetadata, got {}", variant_name(other)),
        }

        store.mark_round_skipped(round);
        match store
            .secret_share_map
            .get(&round)
            .expect("entry should exist after mark_round_skipped")
        {
            SecretShareItem::Skipped => {},
            other => panic!("Expected Skipped, got {}", variant_name(other)),
        }

        let peer_share_2 = create_secret_share(&ctx, 2, &metadata);
        assert!(store.add_share(peer_share_2).is_err());

        match store
            .secret_share_map
            .get(&round)
            .expect("entry should still exist")
        {
            SecretShareItem::Skipped => {},
            other => panic!(
                "Expected Skipped after subsequent add_share, got {}",
                variant_name(other)
            ),
        }
    }
}
