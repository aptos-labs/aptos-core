// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    framework::NodeId,
    raptr::{
        protocol,
        types::{
            common::{Prefix, Round},
            Block, N_SUB_BLOCKS,
        },
    },
};
use anyhow::ensure;
use aptos_consensus_types::proof_of_store::ProofCache;
pub use aptos_consensus_types::proof_of_store::{BatchId, BatchInfo};
pub use aptos_crypto::hash::HashValue;
use aptos_crypto::hash::{CryptoHash, CryptoHasher};
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use aptos_types::validator_verifier::ValidatorVerifier;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::{fmt::Debug, ops::Range};

pub type PoA = aptos_consensus_types::proof_of_store::ProofOfStore;

#[derive(Clone, CryptoHasher, BCSCryptoHash, Serialize, Deserialize)]
pub struct Payload {
    round: Round,
    author: NodeId,
    pub inner: aptos_consensus_types::common::Payload,
}

impl Debug for Payload {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Payload")
            .field("round", &self.round)
            .field("author", &self.author)
            .finish()
    }
}

impl Payload {
    /// Creates a new block payload.
    pub fn new(
        round: Round,
        leader: NodeId,
        inner: aptos_consensus_types::common::Payload,
    ) -> Self {
        Self {
            round,
            author: leader,
            inner,
        }
    }

    pub fn author(&self) -> NodeId {
        self.author
    }

    /// Return a truncated payload that contains only `prefix` of the sub-blocks.
    pub fn with_prefix(&self, prefix: Prefix) -> Self {
        Self {
            round: self.round,
            author: self.author,
            inner: self.inner.as_raptr_payload().with_prefix(prefix).into(),
        }
    }

    /// Returns a new payload that does not include any of the PoAs and only includes sub-blocks
    /// from `range`.
    pub fn take_sub_blocks(&self, range: Range<Prefix>) -> Self {
        Self {
            round: self.round,
            author: self.author,
            inner: self.inner.as_raptr_payload().take_sub_blocks(range).into(),
        }
    }

    pub fn empty(round: Round, leader: NodeId) -> Self {
        Self::new(
            round,
            leader,
            aptos_consensus_types::common::Payload::Raptr(
                aptos_consensus_types::payload::RaptrPayload::new_empty(),
            ),
        )
    }

    pub fn round(&self) -> Round {
        self.round
    }

    pub fn leader(&self) -> NodeId {
        self.author
    }

    pub fn poas(&self) -> &Vec<PoA> {
        self.inner.as_raptr_payload().proofs()
    }

    pub fn sub_blocks(&self) -> impl ExactSizeIterator<Item = &Vec<BatchInfo>> {
        self.inner
            .as_raptr_payload()
            .sub_blocks()
            .iter()
            .map(|inner| &inner.batch_summary)
    }

    pub fn all(&self) -> impl Iterator<Item = &BatchInfo> {
        self.poas()
            .iter()
            .map(|poa| poa.info())
            .chain(self.sub_blocks().flatten())
    }

    pub fn verify(&self, verifier: &protocol::Verifier, block: &Block) -> anyhow::Result<()> {
        ensure!(self.round() == block.round(), "Invalid round");
        ensure!(self.author() == block.author(), "Invalid author");
        ensure!(
            self.sub_blocks().len() == N_SUB_BLOCKS,
            "Received a partial payload: Sub-blocks excluded"
        );

        self.inner.verify(
            verifier.sig_verifier.aptos_verifier(),
            &verifier.proof_cache,
            true,
        )
    }
}
