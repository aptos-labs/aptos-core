// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    block_storage::tracing::{observe_block, BlockStage},
    rand::rand_gen::{
        rand_manager::Sender,
        types::{RandConfig, RandShare, TShare},
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

impl<S: TShare> ShareAggregator<S> {
    pub fn new(author: Author) -> Self {
        Self {
            author,
            shares: HashMap::new(),
            total_weight: 0,
        }
    }

    pub fn add_share(&mut self, weight: u64, share: RandShare<S>) {
        if self.shares.insert(*share.author(), share).is_none() {
            self.total_weight += weight;
        }
    }

    pub fn try_aggregate(
        self,
        rand_config: &RandConfig,
        rand_metadata: RandMetadata,
        decision_tx: Sender<Randomness>,
    ) -> Either<Self, RandShare<S>> {
        if self.total_weight < rand_config.threshold() {
            return Either::Left(self);
        }
        // timestamp records the time when the block is created
        observe_block(rand_metadata.timestamp, BlockStage::RAND_ADD_ENOUGH_SHARE);
        let rand_config = rand_config.clone();
        let self_share = self
            .get_self_share()
            .expect("Aggregated item should have self share");
        tokio::task::spawn_blocking(move || {
            decision_tx.unbounded_send(S::aggregate(
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

impl<S: TShare> RandItem<S> {
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

impl<S: TShare> RandStore<S> {
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

    pub fn update_highest_known_round(&mut self, round: u64) {
        self.highest_known_round = std::cmp::max(self.highest_known_round, round);
    }

    pub fn add_rand_metadata(&mut self, rand_metadata: RandMetadata) {
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
    use aptos_crypto::{bls12381, HashValue, Uniform};
    use aptos_dkg::{
        pvss::{traits::Transcript, Player, WeightedConfig},
        weighted_vuf::traits::WeightedVUF,
    };
    use aptos_types::{
        dkg::{real_dkg::maybe_dk_from_bls_sk, DKGSessionMetadata, DKGTrait, DefaultDKG},
        on_chain_config::OnChainRandomnessConfig,
        randomness::{RandKeys, RandMetadata, WvufPP, WVUF},
        validator_verifier::{
            ValidatorConsensusInfo, ValidatorConsensusInfoMoveStruct, ValidatorVerifier,
        },
    };
    use futures::StreamExt;
    use futures_channel::mpsc::unbounded;
    use rand::thread_rng;
    use std::str::FromStr;

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
                        .get_public_key_share(&dkg_pub_params.pvss_config.wconfig, &Player { id })
                })
                .collect::<Vec<_>>();
            let vuf_pub_params = WvufPP::from(&dkg_pub_params.pvss_config.pp);

            let (ask, apk) = WVUF::augment_key_pair(&vuf_pub_params, sk, pk, &mut rng);

            let rand_keys = RandKeys::new(ask, apk, pk_shares, num_validators);
            let weights: Vec<usize> = weights.into_iter().map(|x| x as usize).collect();
            let half_total_weights = weights.clone().into_iter().sum::<usize>() / 2;
            let weighted_config = WeightedConfig::new(half_total_weights, weights).unwrap();
            let rand_config = RandConfig::new(
                authors[my_index],
                target_epoch,
                verifier,
                vuf_pub_params,
                rand_keys,
                weighted_config,
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
        let mut aggr = ShareAggregator::new(ctxt.authors[0]);
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
            &RandMetadata::new(ctxt.target_epoch, 1, HashValue::zero(), 1700000000),
        );
        assert_eq!(aggr.shares.len(), 2);
        assert_eq!(aggr.total_weight, 4);
    }

    #[tokio::test]
    async fn test_rand_item() {
        let ctxt = TestContext::new(vec![1, 2, 3], 1);
        let (tx, _rx) = unbounded();
        let shares = vec![
            create_share_for_round(ctxt.target_epoch, 2, ctxt.authors[0]),
            create_share_for_round(ctxt.target_epoch, 1, ctxt.authors[1]),
            create_share_for_round(ctxt.target_epoch, 1, ctxt.authors[2]),
        ];

        let mut item = RandItem::<MockShare>::new(ctxt.authors[1]);
        for share in shares.iter() {
            item.add_share(share.clone(), &ctxt.rand_config).unwrap();
        }
        assert_eq!(item.total_weights().unwrap(), 6);
        item.add_metadata(
            &ctxt.rand_config,
            RandMetadata::new(ctxt.target_epoch, 1, HashValue::zero(), 1700000000),
        );
        assert_eq!(item.total_weights().unwrap(), 5);
        item.try_aggregate(&ctxt.rand_config, tx);
        assert!(item.has_decision());

        let mut item = RandItem::<MockShare>::new(ctxt.authors[0]);
        item.add_metadata(
            &ctxt.rand_config,
            RandMetadata::new(ctxt.target_epoch, 2, HashValue::zero(), 1700000000),
        );
        for share in shares[1..].iter() {
            item.add_share(share.clone(), &ctxt.rand_config)
                .unwrap_err();
        }
    }

    #[tokio::test]
    async fn test_rand_store() {
        let ctxt = TestContext::new(vec![100; 7], 0);
        let (decision_tx, mut decision_rx) = unbounded();
        let mut rand_store = RandStore::new(
            ctxt.target_epoch,
            ctxt.authors[1],
            ctxt.rand_config.clone(),
            decision_tx,
        );

        let rounds = vec![vec![1], vec![2, 3], vec![5, 8, 13]];
        let blocks_1 = QueueItem::new(create_ordered_blocks(rounds[0].clone()), None);
        let blocks_2 = QueueItem::new(create_ordered_blocks(rounds[1].clone()), None);
        let metadata_1 = blocks_1.all_rand_metadata();
        let metadata_2 = blocks_2.all_rand_metadata();

        // shares come before metadata
        for share in ctxt.authors[0..5]
            .iter()
            .map(|author| create_share(metadata_1[0].clone(), *author))
        {
            rand_store.add_share(share).unwrap();
        }
        assert!(decision_rx.try_next().is_err());
        for metadata in blocks_1.all_rand_metadata() {
            rand_store.add_rand_metadata(metadata);
        }
        assert!(decision_rx.next().await.is_some());

        // metadata come after shares
        for metadata in blocks_2.all_rand_metadata() {
            rand_store.add_rand_metadata(metadata);
        }
        assert!(decision_rx.try_next().is_err());

        for share in ctxt.authors[1..6]
            .iter()
            .map(|author| create_share(metadata_2[0].clone(), *author))
        {
            rand_store.add_share(share).unwrap();
        }
        assert!(decision_rx.next().await.is_some());
    }
}
