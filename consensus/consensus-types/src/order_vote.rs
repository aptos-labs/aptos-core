// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{common::Author, vote_data::VoteData};
use anyhow::{ensure, Context};
use aptos_crypto::{bls12381, hash::CryptoHash, CryptoMaterialError};
use aptos_short_hex_str::AsShortHexStr;
use aptos_types::{
    ledger_info::LedgerInfo, validator_signer::ValidatorSigner,
    validator_verifier::ValidatorVerifier,
};
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Display, Formatter};

#[derive(Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct OrderVote {
    /// The data of the vote
    vote_data: VoteData,
    /// The identity of the voter.
    author: Author,
    /// LedgerInfo of a block that is going to be ordered in case this vote gathers QC.
    ledger_info: LedgerInfo,
    /// Signature of the LedgerInfo.
    signature: bls12381::Signature,
}

impl Display for OrderVote {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(
            f,
            "OrderVote: [vote data: {}, author: {}, ledger_info: {}]",
            self.vote_data,
            self.author.short_str(),
            self.ledger_info
        )
    }
}

// this is required by structured log
impl Debug for OrderVote {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

impl OrderVote {
    pub fn new(
        vote_data: VoteData,
        author: Author,
        mut ledger_info_placeholder: LedgerInfo,
        validator_signer: &ValidatorSigner,
    ) -> Result<Self, CryptoMaterialError> {
        ledger_info_placeholder.set_consensus_data_hash(vote_data.hash());
        let signature = validator_signer.sign(&ledger_info_placeholder)?;
        Ok(Self {
            vote_data,
            author,
            ledger_info: ledger_info_placeholder,
            signature,
        })
    }

    /// Generates a new Vote using a signature over the specified ledger_info
    pub fn new_with_signature(
        vote_data: VoteData,
        author: Author,
        ledger_info: LedgerInfo,
        signature: bls12381::Signature,
    ) -> Self {
        Self {
            vote_data,
            author,
            ledger_info,
            signature,
        }
    }

    pub fn author(&self) -> Author {
        self.author
    }

    pub fn vote_data(&self) -> &VoteData {
        &self.vote_data
    }

    pub fn ledger_info(&self) -> &LedgerInfo {
        &self.ledger_info
    }

    pub fn signature(&self) -> &bls12381::Signature {
        &self.signature
    }

    pub fn epoch(&self) -> u64 {
        self.ledger_info.epoch()
    }

    pub fn round_to_be_committed(&self) -> u64 {
        self.ledger_info.round()
    }

    /// Verifies that the consensus data hash of LedgerInfo corresponds to the vote info,
    /// and then verifies the signature.
    pub fn verify(&self, validator: &ValidatorVerifier) -> anyhow::Result<()> {
        ensure!(
            self.ledger_info.consensus_data_hash() == self.vote_data.hash(),
            "OrderVote's hash mismatch with LedgerInfo"
        );

        validator
            .verify(self.author(), &self.ledger_info, &self.signature)
            .context("Failed to verify OrderVote")?;

        self.vote_data().verify()?;

        Ok(())
    }
}
