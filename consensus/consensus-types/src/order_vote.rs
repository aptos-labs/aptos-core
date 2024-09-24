// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::common::Author;
use anyhow::{ensure, Context};
use aptos_crypto::{bls12381, HashValue};
use aptos_short_hex_str::AsShortHexStr;
use aptos_types::{
    ledger_info::{LedgerInfo, SignatureWithStatus},
    validator_verifier::ValidatorVerifier,
};
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Display, Formatter};

#[derive(Deserialize, Serialize, Clone)]
pub struct OrderVote {
    /// The identity of the voter.
    author: Author,
    /// LedgerInfo of a block that is going to be ordered in case this vote gathers QC.
    ledger_info: LedgerInfo,
    /// Signature on the LedgerInfo along with a status on whether the signature is verified.
    signature: SignatureWithStatus,
}

impl Display for OrderVote {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(
            f,
            "OrderVote: [author: {}, ledger_info: {}]",
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
    /// Generates a new Vote using a signature over the specified ledger_info
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

    pub fn author(&self) -> Author {
        self.author
    }

    pub fn ledger_info(&self) -> &LedgerInfo {
        &self.ledger_info
    }

    pub fn signature(&self) -> &SignatureWithStatus {
        &self.signature
    }

    pub fn epoch(&self) -> u64 {
        self.ledger_info.epoch()
    }

    /// Performs basic checks, excluding the signature verification.
    pub fn verify_metadata(&self) -> anyhow::Result<()> {
        ensure!(
            self.ledger_info.consensus_data_hash() == HashValue::zero(),
            "Failed to verify OrderVote. Consensus data hash is not Zero"
        );
        Ok(())
    }

    /// Verifies the signature on the LedgerInfo.
    pub fn verify_signature(&self, validator: &ValidatorVerifier) -> anyhow::Result<()> {
        validator
            .verify(self.author(), &self.ledger_info, self.signature.signature())
            .context("Failed to verify OrderVote")?;
        self.signature.set_verified();
        Ok(())
    }

    /// Performs full verification including the signature verification.
    pub fn verify(&self, validator: &ValidatorVerifier) -> anyhow::Result<()> {
        self.verify_metadata()?;
        self.verify_signature(validator)?;
        Ok(())
    }
}
