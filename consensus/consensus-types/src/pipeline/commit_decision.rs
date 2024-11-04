// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::common::Round;
use anyhow::{ensure, Context};
use aptos_types::{ledger_info::LedgerInfoWithSignatures, validator_verifier::ValidatorVerifier};
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Display, Formatter};

#[derive(Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct CommitDecision {
    ledger_info: LedgerInfoWithSignatures,
}

// this is required by structured log
impl Debug for CommitDecision {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

impl Display for CommitDecision {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "CommitDecision: [{}]", self.ledger_info)
    }
}

impl CommitDecision {
    /// Generates a new CommitDecision
    pub fn new(ledger_info: LedgerInfoWithSignatures) -> Self {
        Self { ledger_info }
    }

    pub fn round(&self) -> Round {
        self.ledger_info.ledger_info().round()
    }

    pub fn epoch(&self) -> u64 {
        self.ledger_info.ledger_info().epoch()
    }

    /// Return the LedgerInfo associated with this commit proposal
    pub fn ledger_info(&self) -> &LedgerInfoWithSignatures {
        &self.ledger_info
    }

    /// Verifies that the signatures carried in the message forms a valid quorum,
    /// and then verifies the signature.
    pub fn verify(&self, validator: &ValidatorVerifier) -> anyhow::Result<()> {
        ensure!(
            !self.ledger_info.commit_info().is_ordered_only(),
            "Unexpected ordered only commit info"
        );
        // We do not need to check the author because as long as the signature tree
        // is valid, the message should be valid.
        self.ledger_info
            .verify_signatures(validator)
            .context("Failed to verify Commit Decision")
    }

    pub fn into_inner(self) -> LedgerInfoWithSignatures {
        self.ledger_info
    }
}
