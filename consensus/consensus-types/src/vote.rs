// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    common::Author, quorum_cert::QuorumCert, timeout_2chain::TwoChainTimeout, vote_data::VoteData,
};
use anyhow::{ensure, Context};
use velor_crypto::{bls12381, hash::CryptoHash, CryptoMaterialError};
use velor_short_hex_str::AsShortHexStr;
use velor_types::{
    ledger_info::{LedgerInfo, SignatureWithStatus},
    validator_signer::ValidatorSigner,
    validator_verifier::ValidatorVerifier,
};
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Display, Formatter};

/// Vote is the struct that is ultimately sent by the voter in response for
/// receiving a proposal.
/// Vote carries the `LedgerInfo` of a block that is going to be committed in case this vote
/// is gathers QuorumCertificate (see the detailed explanation in the comments of `LedgerInfo`).
#[derive(Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct Vote {
    /// The data of the vote.
    vote_data: VoteData,
    /// The identity of the voter.
    author: Author,
    /// LedgerInfo of a block that is going to be committed in case this vote gathers QC.
    ledger_info: LedgerInfo,
    /// Signature on the LedgerInfo along with a status on whether the signature is verified.
    signature: SignatureWithStatus,
    /// The 2-chain timeout and corresponding signature.
    two_chain_timeout: Option<(TwoChainTimeout, bls12381::Signature)>,
}

// this is required by structured log
impl Debug for Vote {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

impl Display for Vote {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(
            f,
            "Vote: [vote data: {}, author: {}, is_timeout: {}, {}]",
            self.vote_data,
            self.author.short_str(),
            self.is_timeout(),
            self.ledger_info
        )
    }
}

impl Vote {
    /// Generates a new Vote corresponding to the "fast-vote" path without the round signatures
    /// that can be aggregated into a timeout certificate
    pub fn new(
        vote_data: VoteData,
        author: Author,
        mut ledger_info_placeholder: LedgerInfo,
        validator_signer: &ValidatorSigner,
    ) -> Result<Self, CryptoMaterialError> {
        ledger_info_placeholder.set_consensus_data_hash(vote_data.hash());
        let signature = validator_signer.sign(&ledger_info_placeholder)?;
        Ok(Self::new_with_signature(
            vote_data,
            author,
            ledger_info_placeholder,
            signature,
        ))
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
            signature: SignatureWithStatus::from(signature),
            two_chain_timeout: None,
        }
    }

    /// Add the 2-chain timeout and signature in the vote.
    pub fn add_2chain_timeout(&mut self, timeout: TwoChainTimeout, signature: bls12381::Signature) {
        self.two_chain_timeout = Some((timeout, signature));
    }

    pub fn vote_data(&self) -> &VoteData {
        &self.vote_data
    }

    /// Return the author of the vote
    pub fn author(&self) -> Author {
        self.author
    }

    /// Return the LedgerInfo associated with this vote
    pub fn ledger_info(&self) -> &LedgerInfo {
        &self.ledger_info
    }

    /// Return the signature of the vote
    pub fn signature(&self) -> &bls12381::Signature {
        self.signature.signature()
    }

    pub fn signature_with_status(&self) -> &SignatureWithStatus {
        &self.signature
    }

    /// Returns whether the signature is verified
    pub fn is_verified(&self) -> bool {
        self.signature.is_verified()
    }

    /// Returns the 2-chain timeout.
    pub fn generate_2chain_timeout(&self, qc: QuorumCert) -> TwoChainTimeout {
        TwoChainTimeout::new(
            self.vote_data.proposed().epoch(),
            self.vote_data.proposed().round(),
            qc,
        )
    }

    /// Return the epoch of the vote
    pub fn epoch(&self) -> u64 {
        self.vote_data.proposed().epoch()
    }

    /// Return the two chain timeout vote and signature.
    pub fn two_chain_timeout(&self) -> Option<&(TwoChainTimeout, bls12381::Signature)> {
        self.two_chain_timeout.as_ref()
    }

    /// The vote message is considered a timeout vote message if it carries a signature on the
    /// round, which can then be used for aggregating it to the TimeoutCertificate.
    pub fn is_timeout(&self) -> bool {
        self.two_chain_timeout.is_some()
    }

    /// Verifies that the consensus data hash of LedgerInfo corresponds to the vote info,
    /// and then verifies the signature.
    pub fn verify(&self, validator: &ValidatorVerifier) -> anyhow::Result<()> {
        // TODO(ibalajiarun): Ensure timeout is None if RoundTimeoutMsg is enabled.

        ensure!(
            self.ledger_info.consensus_data_hash() == self.vote_data.hash(),
            "Vote's hash mismatch with LedgerInfo"
        );
        validator
            .optimistic_verify(self.author(), &self.ledger_info, &self.signature)
            .context("Failed to verify Vote")?;
        if let Some((timeout, signature)) = &self.two_chain_timeout {
            ensure!(
                (timeout.epoch(), timeout.round())
                    == (self.epoch(), self.vote_data.proposed().round()),
                "2-chain timeout has different (epoch, round) than Vote"
            );
            timeout.verify(validator)?;
            validator
                .verify(self.author(), &timeout.signing_format(), signature)
                .context("Failed to verify 2-chain timeout signature")?;
        }
        // Let us verify the vote data as well
        self.vote_data().verify()?;
        Ok(())
    }
}
