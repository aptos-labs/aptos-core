// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    block_storage::tracing::{observe_block, BlockStage},
    counters,
    rand::rand_gen::{
        rand_manager::Sender,
        types::{PathType, RandConfig, RandShare, TShare, FUTURE_ROUNDS_TO_ACCEPT},
    },
};
use anyhow::ensure;
use aptos_consensus_types::common::{Author, Round};
use aptos_logger::warn;
use aptos_types::randomness::{FullRandMetadata, RandMetadata, Randomness};
use itertools::Either;
use std::collections::{BTreeMap, HashMap, HashSet};

pub enum AggregationResult<S> {
    Success {
        randomness: Randomness,
        round: Round,
        path_type: PathType,
    },
    Failure {
        round: Round,
        path_type: PathType,
        metadata: FullRandMetadata,
        shares: HashMap<Author, RandShare<S>>,
        total_weight: u64,
    },
}

pub struct ShareAggregator<S> {
    author: Author,
    shares: HashMap<Author, RandShare<S>>,
    total_weight: u64,
    path_type: PathType,
}

impl<S: TShare> ShareAggregator<S> {
    pub fn new(author: Author, path_type: PathType) -> Self {
        Self {
            author,
            shares: HashMap::new(),
            total_weight: 0,
            path_type,
        }
    }

    pub fn add_share(&mut self, weight: u64, share: RandShare<S>) {
        if self.shares.insert(*share.author(), share).is_none() {
            self.total_weight += weight;
        }
    }

    /// Attempt to aggregate shares if threshold is met.
    ///
    /// NOTE: This method is called while holding the `Mutex<RandStore>` lock.
    /// `pre_aggregate_verify` below takes ~7ms on mainnet (150 validators), which blocks
    /// all other share additions for any round during that time. A future improvement is to
    /// move `pre_aggregate_verify` outside the lock into an async task with a failure recovery
    /// path (e.g., an `Aggregating` state that retries on verification failure).
    pub fn try_aggregate(
        mut self,
        rand_config: &RandConfig,
        rand_metadata: FullRandMetadata,
        result_tx: Sender<AggregationResult<S>>,
    ) -> Either<Self, (RandShare<S>, PathType)> {
        if self.total_weight < rand_config.threshold() {
            return Either::Left(self);
        }

        // Pre-verify shares synchronously before spawning the aggregation task.
        // This runs while the RandStore mutex is held (~7ms on mainnet).
        // TODO: move pre_aggregate_verify into spawn_blocking and add failure
        // recovery for the below-threshold case (requires sending Failure from
        // the async task when pre-verify drops weight below threshold).
        let _verify_timer = counters::RAND_PRE_AGGREGATE_VERIFY_DURATION.start_timer();
        let bad_authors =
            S::pre_aggregate_verify(self.shares.values(), rand_config, &rand_metadata.metadata);
        drop(_verify_timer);
        for author in &bad_authors {
            if self.shares.remove(author).is_some() {
                self.total_weight = self
                    .total_weight
                    .saturating_sub(rand_config.get_peer_weight(author));
            }
        }
        if self.total_weight < rand_config.threshold() {
            return Either::Left(self);
        }

        match self.path_type {
            PathType::Fast => {
                observe_block(
                    rand_metadata.timestamp,
                    BlockStage::RAND_ADD_ENOUGH_SHARE_FAST,
                );
            },
            PathType::Slow => {
                observe_block(
                    rand_metadata.timestamp,
                    BlockStage::RAND_ADD_ENOUGH_SHARE_SLOW,
                );
            },
        }

        let rand_config = rand_config.clone();
        let self_share = self
            .get_self_share()
            .expect("Aggregated item should have self share");
        let path_type = self.path_type;
        tokio::task::spawn_blocking(move || {
            let _agg_timer = counters::RAND_AGGREGATION_DURATION.start_timer();
            let maybe_randomness = S::aggregate(
                self.shares.values(),
                &rand_config,
                rand_metadata.metadata.clone(),
            );
            drop(_agg_timer);
            match maybe_randomness {
                Ok(randomness) => {
                    let _ = result_tx.unbounded_send(AggregationResult::Success {
                        randomness,
                        round: rand_metadata.metadata.round,
                        path_type,
                    });
                },
                Err(e) => {
                    warn!(
                        epoch = rand_metadata.metadata.epoch,
                        round = rand_metadata.metadata.round,
                        "Aggregation error: {e}"
                    );
                    if path_type == PathType::Fast {
                        return;
                    }
                    // Filter out authors added to the pessimistic set during fallback.
                    self.shares
                        .retain(|author, _| !rand_config.is_in_pessimistic_set(author));
                    let total_weight: u64 = self
                        .shares
                        .keys()
                        .map(|author| rand_config.get_peer_weight(author))
                        .sum();
                    let _ = result_tx.unbounded_send(AggregationResult::Failure {
                        round: rand_metadata.metadata.round,
                        path_type,
                        metadata: rand_metadata,
                        shares: self.shares,
                        total_weight,
                    });
                },
            }
        });
        Either::Right((self_share, path_type))
    }

    fn retain(&mut self, rand_config: &RandConfig, rand_metadata: &FullRandMetadata) {
        self.shares
            .retain(|_, share| share.metadata() == &rand_metadata.metadata);
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
        metadata: FullRandMetadata,
        share_aggregator: ShareAggregator<S>,
    },
    /// Aggregation is in progress asynchronously. New shares are buffered here.
    Aggregating {
        metadata: FullRandMetadata,
        self_share: RandShare<S>,
        buffered_shares: Vec<RandShare<S>>,
        path_type: PathType,
    },
    Decided {
        self_share: RandShare<S>,
    },
}

impl<S: TShare> RandItem<S> {
    fn new(author: Author, path_type: PathType) -> Self {
        Self::PendingMetadata(ShareAggregator::new(author, path_type))
    }

    fn total_weights(&self) -> Option<u64> {
        match self {
            RandItem::PendingMetadata(aggr) => Some(aggr.total_weights()),
            RandItem::PendingDecision {
                share_aggregator, ..
            } => Some(share_aggregator.total_weights()),
            RandItem::Aggregating { .. } | RandItem::Decided { .. } => None,
        }
    }

    fn has_decision(&self) -> bool {
        matches!(
            self,
            RandItem::Decided { .. } | RandItem::Aggregating { .. }
        )
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
                    &metadata.metadata == share.metadata(),
                    "[RandStore] RandShare metadata from {} mismatch with block metadata!",
                    share.author(),
                );
                share_aggregator.add_share(rand_config.get_peer_weight(share.author()), share);
                Ok(())
            },
            RandItem::Aggregating {
                metadata,
                buffered_shares,
                ..
            } => {
                ensure!(
                    &metadata.metadata == share.metadata(),
                    "[RandStore] RandShare metadata from {} mismatch with block metadata!",
                    share.author(),
                );
                buffered_shares.push(share);
                Ok(())
            },
            RandItem::Decided { .. } => Ok(()),
        }
    }

    fn try_aggregate(&mut self, rand_config: &RandConfig, result_tx: Sender<AggregationResult<S>>) {
        let item = std::mem::replace(self, Self::new(Author::ONE, PathType::Slow));
        let new_item = match item {
            RandItem::PendingDecision {
                share_aggregator,
                metadata,
            } => match share_aggregator.try_aggregate(rand_config, metadata.clone(), result_tx) {
                Either::Left(share_aggregator) => Self::PendingDecision {
                    metadata,
                    share_aggregator,
                },
                Either::Right((self_share, path_type)) => {
                    if path_type == PathType::Fast {
                        // Fast path is optimistic; no recovery needed on failure.
                        Self::Decided { self_share }
                    } else {
                        Self::Aggregating {
                            metadata,
                            self_share,
                            buffered_shares: Vec::new(),
                            path_type,
                        }
                    }
                },
            },
            item @ (RandItem::Decided { .. }
            | RandItem::PendingMetadata(_)
            | RandItem::Aggregating { .. }) => item,
        };
        let _ = std::mem::replace(self, new_item);
    }

    fn add_metadata(&mut self, rand_config: &RandConfig, rand_metadata: FullRandMetadata) {
        let item = std::mem::replace(self, Self::new(Author::ONE, PathType::Slow));
        let new_item = match item {
            RandItem::PendingMetadata(mut share_aggregator) => {
                share_aggregator.retain(rand_config, &rand_metadata);
                Self::PendingDecision {
                    metadata: rand_metadata,
                    share_aggregator,
                }
            },
            item @ (RandItem::PendingDecision { .. }
            | RandItem::Decided { .. }
            | RandItem::Aggregating { .. }) => item,
        };
        let _ = std::mem::replace(self, new_item);
    }

    fn get_all_shares_authors(&self) -> Option<HashSet<Author>> {
        match self {
            RandItem::PendingDecision {
                share_aggregator, ..
            } => Some(share_aggregator.shares.keys().cloned().collect()),
            RandItem::Decided { .. } | RandItem::Aggregating { .. } => None,
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
            RandItem::Aggregating { self_share, .. } => Some(self_share.clone()),
            RandItem::Decided { self_share, .. } => Some(self_share.clone()),
        }
    }
}

pub struct RandStore<S> {
    epoch: u64,
    author: Author,
    rand_config: RandConfig,
    rand_map: BTreeMap<Round, RandItem<S>>,
    fast_rand_config: Option<RandConfig>,
    fast_rand_map: Option<BTreeMap<Round, RandItem<S>>>,
    highest_known_round: u64,
    result_tx: Sender<AggregationResult<S>>,
}

impl<S: TShare> RandStore<S> {
    pub fn new(
        epoch: u64,
        author: Author,
        rand_config: RandConfig,
        fast_rand_config: Option<RandConfig>,
        result_tx: Sender<AggregationResult<S>>,
    ) -> Self {
        Self {
            epoch,
            author,
            rand_config,
            rand_map: BTreeMap::new(),
            fast_rand_config: fast_rand_config.clone(),
            fast_rand_map: fast_rand_config.map(|_| BTreeMap::new()),
            highest_known_round: 0,
            result_tx,
        }
    }

    pub fn update_highest_known_round(&mut self, round: u64) {
        self.highest_known_round = std::cmp::max(self.highest_known_round, round);
    }

    pub fn reset(&mut self, round: u64) {
        self.update_highest_known_round(round);
        // remove future rounds items in case they're already decided
        // otherwise if the block re-enters the queue, it'll be stuck
        let _ = self.rand_map.split_off(&round);
        let _ = self.fast_rand_map.as_mut().map(|map| map.split_off(&round));
    }

    pub fn add_rand_metadata(&mut self, rand_metadata: FullRandMetadata) {
        let rand_item = self
            .rand_map
            .entry(rand_metadata.round())
            .or_insert_with(|| RandItem::new(self.author, PathType::Slow));
        rand_item.add_metadata(&self.rand_config, rand_metadata.clone());
        rand_item.try_aggregate(&self.rand_config, self.result_tx.clone());
        // fast path
        if let (Some(fast_rand_map), Some(fast_rand_config)) =
            (self.fast_rand_map.as_mut(), self.fast_rand_config.as_ref())
        {
            let fast_rand_item = fast_rand_map
                .entry(rand_metadata.round())
                .or_insert_with(|| RandItem::new(self.author, PathType::Fast));
            fast_rand_item.add_metadata(fast_rand_config, rand_metadata.clone());
            fast_rand_item.try_aggregate(fast_rand_config, self.result_tx.clone());
        }
    }

    pub fn add_share(&mut self, share: RandShare<S>, path: PathType) -> anyhow::Result<bool> {
        ensure!(
            share.metadata().epoch == self.epoch,
            "Share from different epoch"
        );
        ensure!(
            share.metadata().round <= self.highest_known_round + FUTURE_ROUNDS_TO_ACCEPT,
            "Share from future round"
        );
        let rand_metadata = share.metadata().clone();

        let (rand_config, rand_item) = if path == PathType::Fast {
            match (self.fast_rand_config.as_ref(), self.fast_rand_map.as_mut()) {
                (Some(fast_rand_config), Some(fast_rand_map)) => (
                    fast_rand_config,
                    fast_rand_map
                        .entry(rand_metadata.round)
                        .or_insert_with(|| RandItem::new(self.author, path)),
                ),
                _ => anyhow::bail!("Fast path not enabled"),
            }
        } else {
            (
                &self.rand_config,
                self.rand_map
                    .entry(rand_metadata.round)
                    .or_insert_with(|| RandItem::new(self.author, PathType::Slow)),
            )
        };

        rand_item.add_share(share, rand_config)?;
        rand_item.try_aggregate(rand_config, self.result_tx.clone());
        Ok(rand_item.has_decision())
    }

    /// This should only be called after the block is added, returns None if already decided
    /// Otherwise returns existing shares' authors
    pub fn get_all_shares_authors(&self, round: Round) -> Option<HashSet<Author>> {
        self.rand_map
            .get(&round)
            .and_then(|item| item.get_all_shares_authors())
    }

    pub fn get_self_share(
        &mut self,
        metadata: &RandMetadata,
    ) -> anyhow::Result<Option<RandShare<S>>> {
        ensure!(
            metadata.round <= self.highest_known_round,
            "Request share from future round {}, highest known round {}",
            metadata.round,
            self.highest_known_round
        );
        Ok(self
            .rand_map
            .get(&metadata.round)
            .and_then(|item| item.get_self_share())
            .filter(|share| share.metadata() == metadata))
    }

    /// Called when an async aggregation task succeeds.
    /// Transitions the item from Aggregating to Decided.
    pub fn handle_aggregation_success(&mut self, round: Round, path_type: PathType) {
        let rand_map = if path_type == PathType::Fast {
            match self.fast_rand_map.as_mut() {
                Some(map) => map,
                None => return,
            }
        } else {
            &mut self.rand_map
        };

        if let Some(item) = rand_map.get_mut(&round) {
            let current = std::mem::replace(item, RandItem::new(self.author, path_type));
            *item = match current {
                RandItem::Aggregating { self_share, .. } => RandItem::Decided { self_share },
                other => other,
            };
        }
    }

    /// Called when an async aggregation task fails.
    /// Merges valid shares from the failed attempt with buffered shares,
    /// then retries aggregation or falls back to PendingDecision.
    /// Returns true if re-aggregation was triggered, false if insufficient shares remain
    /// (caller should re-initiate the slow path broadcast for this round).
    pub fn handle_aggregation_failure(
        &mut self,
        round: Round,
        path_type: PathType,
        metadata: FullRandMetadata,
        mut valid_shares: HashMap<Author, RandShare<S>>,
        mut valid_weight: u64,
    ) -> bool {
        let rand_config = if path_type == PathType::Fast {
            match self.fast_rand_config.as_ref() {
                Some(config) => config.clone(),
                None => return false,
            }
        } else {
            self.rand_config.clone()
        };
        let rand_map = if path_type == PathType::Fast {
            match self.fast_rand_map.as_mut() {
                Some(map) => map,
                None => return false,
            }
        } else {
            &mut self.rand_map
        };

        let Some(item) = rand_map.get_mut(&round) else {
            return false;
        };

        let current = std::mem::replace(item, RandItem::new(self.author, path_type));
        let mut re_aggregated = false;
        *item = match current {
            RandItem::Aggregating {
                metadata: agg_metadata,
                self_share,
                buffered_shares,
                path_type: agg_path,
            } => {
                // Guard against stale failure messages
                if agg_metadata != metadata {
                    RandItem::Aggregating {
                        metadata: agg_metadata,
                        self_share,
                        buffered_shares,
                        path_type: agg_path,
                    }
                } else {
                    // Merge buffered shares into valid_shares, filtering out
                    // authors added to the pessimistic set during the failed aggregation.
                    for share in buffered_shares {
                        let author = *share.author();
                        if share.metadata() == &metadata.metadata
                            && !rand_config.is_in_pessimistic_set(&author)
                        {
                            if !valid_shares.contains_key(&author) {
                                valid_weight += rand_config.get_peer_weight(&author);
                            }
                            valid_shares.insert(author, share);
                        }
                    }
                    // Rebuild ShareAggregator from merged shares
                    let share_aggregator = ShareAggregator {
                        author: self.author,
                        shares: valid_shares,
                        total_weight: valid_weight,
                        path_type,
                    };
                    let mut new_item = RandItem::PendingDecision {
                        metadata,
                        share_aggregator,
                    };
                    new_item.try_aggregate(&rand_config, self.result_tx.clone());
                    re_aggregated = new_item.has_decision();
                    new_item
                }
            },
            other => other,
        };
        re_aggregated
    }
}

#[cfg(test)]
mod tests {
    use crate::rand::rand_gen::{
        block_queue::QueueItem,
        rand_store::{AggregationResult, RandItem, RandStore, ShareAggregator},
        test_utils::{create_ordered_blocks, create_share, create_share_for_round},
        types::{MockShare, PathType, RandConfig, RandShare, TShare},
    };
    use aptos_consensus_types::common::Author;
    use aptos_crypto::{bls12381, HashValue, Uniform};
    use aptos_dkg::{
        pvss::{traits::TranscriptCore, Player, WeightedConfigBlstrs},
        weighted_vuf::traits::WeightedVUF,
    };
    use aptos_types::{
        dkg::{real_dkg::maybe_dk_from_bls_sk, DKGSessionMetadata, DKGTrait, DefaultDKG},
        on_chain_config::OnChainRandomnessConfig,
        randomness::{FullRandMetadata, RandKeys, RandMetadata, Randomness, WvufPP, WVUF},
        validator_verifier::{
            ValidatorConsensusInfo, ValidatorConsensusInfoMoveStruct, ValidatorVerifier,
        },
    };
    use futures::StreamExt;
    use futures_channel::mpsc::unbounded;
    use rand::thread_rng;
    use serde::{Deserialize, Serialize};
    use std::{collections::HashMap, str::FromStr};

    /// A mock share whose aggregate always fails. Used to test the failure path
    /// in ShareAggregator::try_aggregate.
    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    struct FailingMockShare;

    impl TShare for FailingMockShare {
        fn verify(
            &self,
            _rand_config: &RandConfig,
            _rand_metadata: &RandMetadata,
            _author: &Author,
        ) -> anyhow::Result<()> {
            Ok(())
        }

        fn generate(rand_config: &RandConfig, rand_metadata: RandMetadata) -> RandShare<Self> {
            RandShare::new(rand_config.author(), rand_metadata, Self)
        }

        fn aggregate<'a>(
            _shares: impl Iterator<Item = &'a RandShare<Self>>,
            _rand_config: &RandConfig,
            _rand_metadata: RandMetadata,
        ) -> anyhow::Result<Randomness> {
            Err(anyhow::anyhow!("Simulated aggregation failure"))
        }
    }

    fn create_failing_share(metadata: RandMetadata, author: Author) -> RandShare<FailingMockShare> {
        RandShare::new(author, metadata, FailingMockShare)
    }

    /// Captures important data items across the whole DKG-WVUF flow.
    struct TestContext {
        authors: Vec<Author>,
        dealer_epoch: u64,
        target_epoch: u64,
        rand_config: RandConfig,
    }

    impl TestContext {
        fn new(weights: Vec<u64>, my_index: usize) -> Self {
            let dealer_epoch = 0;
            let target_epoch = 1;
            let num_validators = weights.len();
            let mut rng = thread_rng();
            let authors: Vec<_> = (0..num_validators)
                .map(|i| Author::from_str(&format!("{:x}", i)).unwrap())
                .collect();
            let private_keys: Vec<bls12381::PrivateKey> = (0..num_validators)
                .map(|_| bls12381::PrivateKey::generate_for_testing())
                .collect();
            let public_keys: Vec<bls12381::PublicKey> =
                private_keys.iter().map(bls12381::PublicKey::from).collect();
            let dkg_decrypt_keys: Vec<<DefaultDKG as DKGTrait>::NewValidatorDecryptKey> =
                private_keys
                    .iter()
                    .map(|sk| maybe_dk_from_bls_sk(sk).unwrap())
                    .collect();
            let consensus_infos: Vec<ValidatorConsensusInfo> = (0..num_validators)
                .map(|idx| {
                    ValidatorConsensusInfo::new(
                        authors[idx],
                        public_keys[idx].clone(),
                        weights[idx],
                    )
                })
                .collect();
            let consensus_info_move_structs = consensus_infos
                .clone()
                .into_iter()
                .map(ValidatorConsensusInfoMoveStruct::from)
                .collect::<Vec<_>>();
            let verifier = ValidatorVerifier::new(consensus_infos.clone());
            let dkg_session_metadata = DKGSessionMetadata {
                dealer_epoch: 999,
                randomness_config: OnChainRandomnessConfig::default_enabled().into(),
                dealer_validator_set: consensus_info_move_structs.clone(),
                target_validator_set: consensus_info_move_structs.clone(),
            };
            let dkg_pub_params = DefaultDKG::new_public_params(&dkg_session_metadata);
            let input_secret = <DefaultDKG as DKGTrait>::InputSecret::generate_for_testing();
            let transcript = DefaultDKG::generate_transcript(
                &mut rng,
                &dkg_pub_params,
                &input_secret,
                0,
                &private_keys[0],
                &public_keys[0],
            );
            let (sk, pk) = DefaultDKG::decrypt_secret_share_from_transcript(
                &dkg_pub_params,
                &transcript,
                my_index as u64,
                &dkg_decrypt_keys[my_index],
            )
            .unwrap();

            let pk_shares = (0..num_validators)
                .map(|id| {
                    transcript
                        .main
                        .get_public_key_share(&dkg_pub_params.pvss_config.wconfig, &Player { id })
                })
                .collect::<Vec<_>>();
            let vuf_pub_params = WvufPP::from(&dkg_pub_params.pvss_config.pp);

            let aggregate_pk = transcript.main.get_dealt_public_key();
            let (ask, apk) = WVUF::augment_key_pair(&vuf_pub_params, sk.main, pk.main, &mut rng);

            let rand_keys = RandKeys::new(ask, apk, pk_shares, num_validators);
            let weights: Vec<usize> = weights.into_iter().map(|x| x as usize).collect();
            let half_total_weights = weights.clone().into_iter().sum::<usize>() / 2;
            let weighted_config = WeightedConfigBlstrs::new(half_total_weights, weights).unwrap();
            let rand_config = RandConfig::new(
                authors[my_index],
                target_epoch,
                verifier.into(),
                vuf_pub_params,
                rand_keys,
                weighted_config,
                aggregate_pk,
                false,
            );

            Self {
                authors,
                dealer_epoch,
                target_epoch,
                rand_config,
            }
        }
    }

    #[test]
    fn test_share_aggregator() {
        let ctxt = TestContext::new(vec![1, 2, 3], 0);
        let mut aggr = ShareAggregator::new(ctxt.authors[0], PathType::Slow);
        aggr.add_share(
            1,
            create_share_for_round(ctxt.target_epoch, 1, ctxt.authors[0]),
        );
        aggr.add_share(
            2,
            create_share_for_round(ctxt.target_epoch, 2, ctxt.authors[1]),
        );
        aggr.add_share(
            3,
            create_share_for_round(ctxt.target_epoch, 1, ctxt.authors[2]),
        );
        assert_eq!(aggr.shares.len(), 3);
        assert_eq!(aggr.total_weight, 6);
        // retain the shares with the same metadata
        aggr.retain(
            &ctxt.rand_config,
            &FullRandMetadata::new(ctxt.target_epoch, 1, HashValue::zero(), 1700000000),
        );
        assert_eq!(aggr.shares.len(), 2);
        assert_eq!(aggr.total_weight, 4);
    }

    #[tokio::test]
    async fn test_rand_item() {
        let ctxt = TestContext::new(vec![1, 2, 3], 1);
        let (tx, mut rx) = unbounded::<AggregationResult<MockShare>>();
        let shares = [
            create_share_for_round(ctxt.target_epoch, 2, ctxt.authors[0]),
            create_share_for_round(ctxt.target_epoch, 1, ctxt.authors[1]),
            create_share_for_round(ctxt.target_epoch, 1, ctxt.authors[2]),
        ];

        let mut item = RandItem::<MockShare>::new(ctxt.authors[1], PathType::Slow);
        for share in shares.iter() {
            item.add_share(share.clone(), &ctxt.rand_config).unwrap();
        }
        assert_eq!(item.total_weights().unwrap(), 6);
        item.add_metadata(
            &ctxt.rand_config,
            FullRandMetadata::new(ctxt.target_epoch, 1, HashValue::zero(), 1700000000),
        );
        assert_eq!(item.total_weights().unwrap(), 5);
        item.try_aggregate(&ctxt.rand_config, tx);
        // After try_aggregate, item is in Aggregating state (async task in flight)
        assert!(matches!(item, RandItem::Aggregating { .. }));
        assert!(item.has_decision());
        // The async task should send Success
        assert!(matches!(
            rx.next().await,
            Some(AggregationResult::Success { .. })
        ));

        let mut item = RandItem::<MockShare>::new(ctxt.authors[0], PathType::Slow);
        item.add_metadata(
            &ctxt.rand_config,
            FullRandMetadata::new(ctxt.target_epoch, 2, HashValue::zero(), 1700000000),
        );
        for share in shares[1..].iter() {
            item.add_share(share.clone(), &ctxt.rand_config)
                .unwrap_err();
        }
    }

    #[tokio::test]
    async fn test_rand_store() {
        let ctxt = TestContext::new(vec![100; 7], 0);
        let (result_tx, mut result_rx) = unbounded::<AggregationResult<MockShare>>();
        let mut rand_store = RandStore::new(
            ctxt.target_epoch,
            ctxt.authors[1],
            ctxt.rand_config.clone(),
            None,
            result_tx,
        );

        let rounds = [vec![1], vec![2, 3], vec![5, 8, 13]];
        let blocks_1 = QueueItem::new(create_ordered_blocks(rounds[0].clone()), None);
        let blocks_2 = QueueItem::new(create_ordered_blocks(rounds[1].clone()), None);
        let metadata_1 = blocks_1.all_rand_metadata();
        let metadata_2 = blocks_2.all_rand_metadata();

        // shares come before metadata
        for share in ctxt.authors[0..5]
            .iter()
            .map(|author| create_share(metadata_1[0].metadata.clone(), *author))
        {
            rand_store.add_share(share, PathType::Slow).unwrap();
        }
        assert!(result_rx.try_next().is_err());
        for metadata in blocks_1.all_rand_metadata() {
            rand_store.add_rand_metadata(metadata);
        }
        assert!(matches!(
            result_rx.next().await,
            Some(AggregationResult::Success { .. })
        ));

        // metadata come after shares
        for metadata in blocks_2.all_rand_metadata() {
            rand_store.add_rand_metadata(metadata);
        }
        assert!(result_rx.try_next().is_err());

        for share in ctxt.authors[1..6]
            .iter()
            .map(|author| create_share(metadata_2[0].metadata.clone(), *author))
        {
            rand_store.add_share(share, PathType::Slow).unwrap();
        }
        assert!(matches!(
            result_rx.next().await,
            Some(AggregationResult::Success { .. })
        ));
    }

    // ==================== FailingMockShare tests (A-C) ====================

    /// Test A: FailingMockShare::aggregate returns Err → AggregationResult::Failure
    /// sent on channel with correct fields and all shares returned.
    #[tokio::test]
    async fn test_aggregation_failure_sends_failure_on_channel() {
        let ctxt = TestContext::new(vec![1, 2, 3], 1);
        let (tx, mut rx) = unbounded::<AggregationResult<FailingMockShare>>();
        let metadata = FullRandMetadata::new(ctxt.target_epoch, 1, HashValue::zero(), 1700000000);

        let mut item = RandItem::<FailingMockShare>::new(ctxt.authors[1], PathType::Slow);
        item.add_metadata(&ctxt.rand_config, metadata.clone());

        // author[1] (w=2) + author[2] (w=3) = 5 >= threshold 3
        let share1 = create_failing_share(metadata.metadata.clone(), ctxt.authors[1]);
        let share2 = create_failing_share(metadata.metadata.clone(), ctxt.authors[2]);
        item.add_share(share1, &ctxt.rand_config).unwrap();
        item.add_share(share2, &ctxt.rand_config).unwrap();

        item.try_aggregate(&ctxt.rand_config, tx);
        assert!(matches!(item, RandItem::Aggregating { .. }));

        // FailingMockShare::aggregate returns Err → Failure on channel
        match rx.next().await {
            Some(AggregationResult::Failure {
                round,
                path_type,
                shares,
                total_weight,
                ..
            }) => {
                assert_eq!(round, 1);
                assert_eq!(path_type, PathType::Slow);
                // No pessimistic filtering (FailingMockShare doesn't add to set)
                assert_eq!(shares.len(), 2);
                assert!(shares.contains_key(&ctxt.authors[1]));
                assert!(shares.contains_key(&ctxt.authors[2]));
                assert_eq!(total_weight, 5); // 2 + 3
            },
            other => panic!("Expected Failure, got Success={}", other.is_some()),
        }
    }

    /// Test B: Authors in pessimistic set are filtered from valid_shares in Failure.
    #[tokio::test]
    async fn test_aggregation_failure_with_pessimistic_filtering() {
        let ctxt = TestContext::new(vec![1, 2, 3], 1);
        let (tx, mut rx) = unbounded::<AggregationResult<FailingMockShare>>();
        let metadata = FullRandMetadata::new(ctxt.target_epoch, 1, HashValue::zero(), 1700000000);

        // Pre-add author[2] to pessimistic set
        ctxt.rand_config.add_to_pessimistic_set(ctxt.authors[2]);

        let mut item = RandItem::<FailingMockShare>::new(ctxt.authors[1], PathType::Slow);
        item.add_metadata(&ctxt.rand_config, metadata.clone());

        let share0 = create_failing_share(metadata.metadata.clone(), ctxt.authors[0]);
        let share1 = create_failing_share(metadata.metadata.clone(), ctxt.authors[1]);
        let share2 = create_failing_share(metadata.metadata.clone(), ctxt.authors[2]);
        item.add_share(share0, &ctxt.rand_config).unwrap();
        item.add_share(share1, &ctxt.rand_config).unwrap();
        item.add_share(share2, &ctxt.rand_config).unwrap();

        item.try_aggregate(&ctxt.rand_config, tx);

        match rx.next().await {
            Some(AggregationResult::Failure {
                shares,
                total_weight,
                ..
            }) => {
                // author[2] filtered out by pessimistic set
                assert_eq!(shares.len(), 2);
                assert!(shares.contains_key(&ctxt.authors[0]));
                assert!(shares.contains_key(&ctxt.authors[1]));
                assert!(!shares.contains_key(&ctxt.authors[2]));
                assert_eq!(total_weight, 3); // 1 + 2
            },
            other => panic!("Expected Failure, got Success={}", other.is_some()),
        }
    }

    /// Test C: Full RandStore roundtrip with FailingMockShare: fail → buffer → recover → fail again.
    #[tokio::test]
    async fn test_aggregation_failure_full_store_roundtrip() {
        let ctxt = TestContext::new(vec![1, 2, 3], 1);
        let (result_tx, mut result_rx) = unbounded::<AggregationResult<FailingMockShare>>();
        let mut rand_store = RandStore::<FailingMockShare>::new(
            ctxt.target_epoch,
            ctxt.authors[1],
            ctxt.rand_config.clone(),
            None,
            result_tx,
        );

        let metadata = FullRandMetadata::new(ctxt.target_epoch, 1, HashValue::zero(), 1700000000);
        rand_store.update_highest_known_round(1);
        rand_store.add_rand_metadata(metadata.clone());

        // Trigger aggregation: author[1] (w=2) + author[2] (w=3) = 5 >= 3
        let share1 = create_failing_share(metadata.metadata.clone(), ctxt.authors[1]);
        let share2 = create_failing_share(metadata.metadata.clone(), ctxt.authors[2]);
        rand_store
            .add_share(share1.clone(), PathType::Slow)
            .unwrap();
        rand_store
            .add_share(share2.clone(), PathType::Slow)
            .unwrap();

        // Receive first Failure
        let result = result_rx.next().await;
        assert!(matches!(result, Some(AggregationResult::Failure { .. })));

        // Buffer a share while in Aggregating
        let share0 = create_failing_share(metadata.metadata.clone(), ctxt.authors[0]);
        rand_store
            .add_share(share0.clone(), PathType::Slow)
            .unwrap();

        // Extract failure data and feed into handle_aggregation_failure
        if let Some(AggregationResult::Failure {
            round,
            path_type,
            metadata: fail_meta,
            shares: valid_shares,
            total_weight,
        }) = result
        {
            rand_store.handle_aggregation_failure(
                round,
                path_type,
                fail_meta,
                valid_shares,
                total_weight,
            );
            // Merged shares (share1 w=2 + share2 w=3 + buffered share0 w=1 = 6) >= threshold
            // → retries aggregation → FailingMockShare fails again
            let result2 = result_rx.next().await;
            assert!(matches!(result2, Some(AggregationResult::Failure { .. })));
        }
    }

    // ==================== State machine & handler tests (D-J) ====================

    /// Test D: Shares arriving in Aggregating state are buffered correctly.
    #[tokio::test]
    async fn test_aggregating_state_buffers_shares() {
        let ctxt = TestContext::new(vec![1, 2, 3], 1);
        let (tx, mut rx) = unbounded::<AggregationResult<MockShare>>();
        let metadata = FullRandMetadata::new(ctxt.target_epoch, 1, HashValue::zero(), 1700000000);

        let mut item = RandItem::<MockShare>::new(ctxt.authors[1], PathType::Slow);
        item.add_metadata(&ctxt.rand_config, metadata.clone());

        // Reach threshold: author[1] (w=2) + author[2] (w=3) = 5 >= 3
        let share1 = create_share(metadata.metadata.clone(), ctxt.authors[1]);
        let share2 = create_share(metadata.metadata.clone(), ctxt.authors[2]);
        item.add_share(share1, &ctxt.rand_config).unwrap();
        item.add_share(share2, &ctxt.rand_config).unwrap();

        item.try_aggregate(&ctxt.rand_config, tx);
        assert!(matches!(item, RandItem::Aggregating { .. }));

        // Aggregating counts as decided (stops reliable broadcast)
        assert!(item.has_decision());
        assert!(item.get_all_shares_authors().is_none());
        assert!(item.total_weights().is_none());

        // Buffer a share
        let share0 = create_share(metadata.metadata.clone(), ctxt.authors[0]);
        item.add_share(share0.clone(), &ctxt.rand_config).unwrap();

        // Inspect buffered_shares
        match &item {
            RandItem::Aggregating {
                buffered_shares, ..
            } => {
                assert_eq!(buffered_shares.len(), 1);
                assert_eq!(buffered_shares[0].author(), &ctxt.authors[0]);
            },
            _ => panic!("Expected Aggregating state"),
        }

        // State invariants still hold
        assert!(item.has_decision());

        // Drain Success from MockShare
        assert!(matches!(
            rx.next().await,
            Some(AggregationResult::Success { .. })
        ));
    }

    /// Test E: handle_aggregation_success transitions Aggregating → Decided.
    #[tokio::test]
    async fn test_handle_aggregation_success() {
        let ctxt = TestContext::new(vec![1, 2, 3], 1);
        let (result_tx, mut result_rx) = unbounded::<AggregationResult<MockShare>>();
        let mut rand_store = RandStore::new(
            ctxt.target_epoch,
            ctxt.authors[1],
            ctxt.rand_config.clone(),
            None,
            result_tx,
        );

        let metadata = FullRandMetadata::new(ctxt.target_epoch, 1, HashValue::zero(), 1700000000);
        rand_store.update_highest_known_round(1);
        rand_store.add_rand_metadata(metadata.clone());

        let share1 = create_share(metadata.metadata.clone(), ctxt.authors[1]);
        let share2 = create_share(metadata.metadata.clone(), ctxt.authors[2]);
        rand_store
            .add_share(share1.clone(), PathType::Slow)
            .unwrap();
        rand_store
            .add_share(share2.clone(), PathType::Slow)
            .unwrap();

        // Drain Success from MockShare
        assert!(matches!(
            result_rx.next().await,
            Some(AggregationResult::Success { .. })
        ));

        // Transition Aggregating → Decided
        rand_store.handle_aggregation_success(1, PathType::Slow);

        // Verify Decided: add_share returns has_decision() = true
        let share0 = create_share(metadata.metadata.clone(), ctxt.authors[0]);
        let has_decision = rand_store.add_share(share0, PathType::Slow).unwrap();
        assert!(has_decision);

        // No new aggregation result
        assert!(result_rx.try_next().is_err());
        assert!(rand_store.get_all_shares_authors(1).is_none());
    }

    /// Test F: Failure with valid_shares + buffered shares >= threshold triggers retry.
    #[tokio::test]
    async fn test_handle_aggregation_failure_retry_succeeds() {
        let ctxt = TestContext::new(vec![1, 2, 3], 1);
        let (result_tx, mut result_rx) = unbounded::<AggregationResult<MockShare>>();
        let mut rand_store = RandStore::new(
            ctxt.target_epoch,
            ctxt.authors[1],
            ctxt.rand_config.clone(),
            None,
            result_tx,
        );

        let metadata = FullRandMetadata::new(ctxt.target_epoch, 1, HashValue::zero(), 1700000000);
        rand_store.update_highest_known_round(1);
        rand_store.add_rand_metadata(metadata.clone());

        let share1 = create_share(metadata.metadata.clone(), ctxt.authors[1]);
        let share2 = create_share(metadata.metadata.clone(), ctxt.authors[2]);
        rand_store
            .add_share(share1.clone(), PathType::Slow)
            .unwrap();
        rand_store
            .add_share(share2.clone(), PathType::Slow)
            .unwrap();

        // Drain Success
        let _ = result_rx.next().await;

        // Buffer a share while in Aggregating
        let share0 = create_share(metadata.metadata.clone(), ctxt.authors[0]);
        rand_store
            .add_share(share0.clone(), PathType::Slow)
            .unwrap();

        // Simulate failure: only share1 valid (w=2)
        // After merge with buffered share0 (w=1), total = 3 >= threshold
        let mut valid_shares = HashMap::new();
        valid_shares.insert(*share1.author(), share1.clone());

        rand_store.handle_aggregation_failure(1, PathType::Slow, metadata.clone(), valid_shares, 2);

        // Merged shares meet threshold → MockShare succeeds → Success on channel
        assert!(matches!(
            result_rx.next().await,
            Some(AggregationResult::Success { .. })
        ));
    }

    /// Test G: Failure with insufficient shares → PendingDecision. Add more shares later → retry.
    #[tokio::test]
    async fn test_handle_aggregation_failure_insufficient_shares() {
        let ctxt = TestContext::new(vec![1, 2, 3], 1);
        let (result_tx, mut result_rx) = unbounded::<AggregationResult<MockShare>>();
        let mut rand_store = RandStore::new(
            ctxt.target_epoch,
            ctxt.authors[1],
            ctxt.rand_config.clone(),
            None,
            result_tx,
        );

        let metadata = FullRandMetadata::new(ctxt.target_epoch, 1, HashValue::zero(), 1700000000);
        rand_store.update_highest_known_round(1);
        rand_store.add_rand_metadata(metadata.clone());

        let share1 = create_share(metadata.metadata.clone(), ctxt.authors[1]);
        let share2 = create_share(metadata.metadata.clone(), ctxt.authors[2]);
        rand_store
            .add_share(share1.clone(), PathType::Slow)
            .unwrap();
        rand_store
            .add_share(share2.clone(), PathType::Slow)
            .unwrap();

        // Drain Success
        let _ = result_rx.next().await;

        // No buffered shares. Simulate failure with only self share (w=2) valid.
        // In practice, self share always passes verification.
        let mut valid_shares = HashMap::new();
        valid_shares.insert(*share1.author(), share1.clone());

        rand_store.handle_aggregation_failure(
            1,
            PathType::Slow,
            metadata.clone(),
            valid_shares,
            2, // weight of share1
        );

        // Insufficient weight (2 < 3) → no new aggregation
        assert!(result_rx.try_next().is_err());

        // Item is in PendingDecision: add_share returns has_decision=false
        let share0 = create_share(metadata.metadata.clone(), ctxt.authors[0]);
        let has_decision = rand_store.add_share(share0, PathType::Slow).unwrap();
        assert!(has_decision); // 2 + 1 = 3 >= 3 → triggers aggregation, Aggregating counts as decided

        // Wait, weight is now 3 >= 3, so aggregation triggered immediately
        assert!(matches!(
            result_rx.next().await,
            Some(AggregationResult::Success { .. })
        ));
    }

    /// Test H: Failure with stale metadata (different block_id) → item stays Aggregating.
    #[tokio::test]
    async fn test_handle_aggregation_failure_stale_metadata() {
        let ctxt = TestContext::new(vec![1, 2, 3], 1);
        let (result_tx, mut result_rx) = unbounded::<AggregationResult<MockShare>>();
        let mut rand_store = RandStore::new(
            ctxt.target_epoch,
            ctxt.authors[1],
            ctxt.rand_config.clone(),
            None,
            result_tx,
        );

        let metadata = FullRandMetadata::new(ctxt.target_epoch, 1, HashValue::zero(), 1700000000);
        rand_store.update_highest_known_round(1);
        rand_store.add_rand_metadata(metadata.clone());

        let share1 = create_share(metadata.metadata.clone(), ctxt.authors[1]);
        let share2 = create_share(metadata.metadata.clone(), ctxt.authors[2]);
        rand_store
            .add_share(share1.clone(), PathType::Slow)
            .unwrap();
        rand_store
            .add_share(share2.clone(), PathType::Slow)
            .unwrap();

        // Drain Success
        let _ = result_rx.next().await;

        // Stale metadata: different block_id
        let stale_metadata = FullRandMetadata::new(
            ctxt.target_epoch,
            1,
            HashValue::from_slice([1u8; 32]).unwrap(),
            1700000000,
        );

        let mut valid_shares = HashMap::new();
        valid_shares.insert(*share1.author(), share1.clone());

        rand_store.handle_aggregation_failure(1, PathType::Slow, stale_metadata, valid_shares, 2);

        // Stale → no retry
        assert!(result_rx.try_next().is_err());
        // Still in Aggregating (not PendingDecision): get_all_shares_authors returns None
        assert!(rand_store.get_all_shares_authors(1).is_none());
    }

    /// Test I: reset() removes round → handle_aggregation_failure is no-op.
    #[tokio::test]
    async fn test_handle_aggregation_failure_after_reset() {
        let ctxt = TestContext::new(vec![1, 2, 3], 1);
        let (result_tx, mut result_rx) = unbounded::<AggregationResult<MockShare>>();
        let mut rand_store = RandStore::new(
            ctxt.target_epoch,
            ctxt.authors[1],
            ctxt.rand_config.clone(),
            None,
            result_tx,
        );

        let metadata = FullRandMetadata::new(ctxt.target_epoch, 1, HashValue::zero(), 1700000000);
        rand_store.update_highest_known_round(1);
        rand_store.add_rand_metadata(metadata.clone());

        let share1 = create_share(metadata.metadata.clone(), ctxt.authors[1]);
        let share2 = create_share(metadata.metadata.clone(), ctxt.authors[2]);
        rand_store
            .add_share(share1.clone(), PathType::Slow)
            .unwrap();
        rand_store
            .add_share(share2.clone(), PathType::Slow)
            .unwrap();

        // Drain Success
        let _ = result_rx.next().await;

        // Reset removes round >= 1
        rand_store.reset(1);

        // handle_aggregation_failure for removed round → no-op, no crash
        let mut valid_shares = HashMap::new();
        valid_shares.insert(*share1.author(), share1.clone());
        rand_store.handle_aggregation_failure(1, PathType::Slow, metadata.clone(), valid_shares, 2);

        assert!(result_rx.try_next().is_err());
        assert!(rand_store.get_all_shares_authors(1).is_none());
    }

    /// Test J: try_aggregate in Aggregating state is no-op — only one result.
    #[tokio::test]
    async fn test_try_aggregate_noop_in_aggregating() {
        let ctxt = TestContext::new(vec![1, 2, 3], 1);
        let (tx, mut rx) = unbounded::<AggregationResult<MockShare>>();
        let metadata = FullRandMetadata::new(ctxt.target_epoch, 1, HashValue::zero(), 1700000000);

        let mut item = RandItem::<MockShare>::new(ctxt.authors[1], PathType::Slow);
        item.add_metadata(&ctxt.rand_config, metadata.clone());

        let share1 = create_share(metadata.metadata.clone(), ctxt.authors[1]);
        let share2 = create_share(metadata.metadata.clone(), ctxt.authors[2]);
        item.add_share(share1, &ctxt.rand_config).unwrap();
        item.add_share(share2, &ctxt.rand_config).unwrap();

        // First try_aggregate → Aggregating
        item.try_aggregate(&ctxt.rand_config, tx.clone());
        assert!(matches!(item, RandItem::Aggregating { .. }));

        // Drain the first (and only) result
        assert!(matches!(
            rx.next().await,
            Some(AggregationResult::Success { .. })
        ));

        // Add a share and call try_aggregate again while in Aggregating
        let share0 = create_share(metadata.metadata.clone(), ctxt.authors[0]);
        item.add_share(share0, &ctxt.rand_config).unwrap();
        // Keep a sender alive so the channel stays open (try_next returns Err = no pending items)
        let _keep_alive = tx.clone();
        item.try_aggregate(&ctxt.rand_config, tx);

        // Still Aggregating, no second result
        assert!(matches!(item, RandItem::Aggregating { .. }));
        assert!(rx.try_next().is_err());
    }
}
