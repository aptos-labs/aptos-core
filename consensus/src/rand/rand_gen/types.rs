// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_consensus_types::{
    common::{Author, Round},
    randomness::{RandMetadata, Randomness},
};
use aptos_types::epoch_state::EpochState;
use std::sync::Arc;

pub(super) struct MockShare;

pub(super) struct MockProof;

impl Share for MockShare {
    fn verify(
        &self,
        _rand_config: &RandConfig,
        _rand_metadata: &RandMetadata,
    ) -> anyhow::Result<()> {
        Ok(())
    }
}

impl Proof for MockProof {
    type Share = MockShare;

    fn verify(
        &self,
        _rand_config: &RandConfig,
        _rand_metadata: &RandMetadata,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    fn aggregate<'a>(
        _shares: impl Iterator<Item = &'a RandShare<Self::Share>>,
        _rand_config: &RandConfig,
        rand_metadata: RandMetadata,
    ) -> RandDecision<Self> {
        RandDecision::new(Randomness::new(rand_metadata, vec![0u8; 32]), Self)
    }
}

pub trait Share {
    fn verify(&self, rand_config: &RandConfig, rand_metadata: &RandMetadata) -> anyhow::Result<()>;
}

pub trait Proof {
    type Share: 'static;
    fn verify(&self, rand_config: &RandConfig, rand_metadata: &RandMetadata) -> anyhow::Result<()>;

    fn aggregate<'a>(
        shares: impl Iterator<Item = &'a RandShare<Self::Share>>,
        rand_config: &RandConfig,
        rand_metadata: RandMetadata,
    ) -> RandDecision<Self>
    where
        Self: Sized;
}

pub struct RandShare<S> {
    author: Author,
    metadata: RandMetadata,
    share: S,
}

impl<S: Share> RandShare<S> {
    pub fn author(&self) -> &Author {
        &self.author
    }

    pub fn metadata(&self) -> &RandMetadata {
        &self.metadata
    }

    pub fn round(&self) -> Round {
        self.metadata.round()
    }

    pub fn epoch(&self) -> u64 {
        self.metadata.epoch()
    }

    pub fn verify(&self, rand_config: &RandConfig) -> anyhow::Result<()> {
        self.share.verify(rand_config, &self.metadata)
    }
}

pub struct RandDecision<P> {
    randomness: Randomness,
    proof: P,
}

impl<P: Proof> RandDecision<P> {
    pub fn new(randomness: Randomness, proof: P) -> Self {
        Self { randomness, proof }
    }

    pub fn verify(&self, rand_config: &RandConfig) -> anyhow::Result<()> {
        self.proof.verify(rand_config, self.randomness.metadata())
    }

    pub fn randomness(&self) -> &Randomness {
        &self.randomness
    }

    pub fn rand_metadata(&self) -> &RandMetadata {
        self.randomness.metadata()
    }
}

pub struct RandConfig {
    author: Author,
    epoch_state: Arc<EpochState>,
}

impl RandConfig {
    pub fn new(author: Author, epoch_state: Arc<EpochState>) -> Self {
        Self {
            author,
            epoch_state,
        }
    }

    pub fn get_peer_weight(&self, author: &Author) -> u64 {
        self.epoch_state
            .verifier
            .get_voting_power(author)
            .expect("Author should exist after verify")
    }

    pub fn threshold_weight(&self) -> u64 {
        self.epoch_state.verifier.quorum_voting_power() as u64
    }
}
