// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::common::{Author, Round};
use anyhow::{ensure, Context};
use velor_crypto::{bls12381, CryptoMaterialError};
use velor_short_hex_str::AsShortHexStr;
use velor_types::{
    block_info::BlockInfo,
    ledger_info::{LedgerInfo, SignatureWithStatus},
    validator_signer::ValidatorSigner,
    validator_verifier::ValidatorVerifier,
};
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Display, Formatter};

#[derive(Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct CommitVote {
    author: Author,
    ledger_info: LedgerInfo,
    /// Signature on the LedgerInfo along with a status on whether the signature is verified.
    signature: SignatureWithStatus,
}

// this is required by structured log
impl Debug for CommitVote {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

impl Display for CommitVote {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(
            f,
            "CommitProposal: [author: {}, {}]",
            self.author.short_str(),
            self.ledger_info
        )
    }
}

impl CommitVote {
    /// Generates a new CommitProposal
    pub fn new(
        author: Author,
        ledger_info_placeholder: LedgerInfo,
        validator_signer: &ValidatorSigner,
    ) -> Result<Self, CryptoMaterialError> {
        let signature = validator_signer.sign(&ledger_info_placeholder)?;
        Ok(Self::new_with_signature(
            author,
            ledger_info_placeholder,
            signature,
        ))
    }

    /// Generates a new CommitProposal using a signature over the specified ledger_info
    pub fn new_with_signature(
        author: Author,
        ledger_info: LedgerInfo,
        signature: bls12381::Signature,
    ) -> Self {
        Self {
            author,
            ledger_info,
            signature: SignatureWithStatus::from(signature),
        }
    }

    /// Return the author of the commit proposal
    pub fn author(&self) -> Author {
        self.author
    }

    /// Return the LedgerInfo associated with this commit proposal
    pub fn ledger_info(&self) -> &LedgerInfo {
        &self.ledger_info
    }

    /// Return the signature of the vote
    pub fn signature(&self) -> &bls12381::Signature {
        self.signature.signature()
    }

    /// Returns the signature along with the verification status of the signature.
    // Note: SignatureWithStatus has interior mutability for verification status.
    // Need to make sure the verification status is set to true only the verification is successful.
    pub fn signature_with_status(&self) -> &SignatureWithStatus {
        &self.signature
    }

    pub fn round(&self) -> Round {
        self.ledger_info.round()
    }

    pub fn epoch(&self) -> u64 {
        self.ledger_info.epoch()
    }

    /// Verifies that the consensus data hash of LedgerInfo corresponds to the commit proposal,
    /// and then verifies the signature.
    pub fn verify(&self, sender: Author, validator: &ValidatorVerifier) -> anyhow::Result<()> {
        ensure!(
            self.author() == sender,
            "Commit vote author {:?} doesn't match with the sender {:?}",
            self.author(),
            sender
        );
        validator
            .optimistic_verify(self.author(), &self.ledger_info, &self.signature)
            .context("Failed to verify Commit Vote")
    }

    pub fn commit_info(&self) -> &BlockInfo {
        self.ledger_info().commit_info()
    }
}
