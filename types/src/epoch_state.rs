// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    epoch_change::Verifier,
    ledger_info::{LedgerInfo, LedgerInfoWithSignatures},
    validator_verifier::ValidatorVerifier,
};
use anyhow::ensure;
#[cfg(any(test, feature = "fuzzing"))]
use proptest_derive::Arbitrary;
use serde::{Deserialize, Serialize};
use std::{fmt, sync::Arc};

/// EpochState represents a trusted validator set to validate messages from the specific epoch,
/// it could be updated with EpochChangeProof.
#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(Arbitrary))]
pub struct EpochState {
    pub epoch: u64,
    pub verifier: Arc<ValidatorVerifier>,
}

impl EpochState {
    pub fn new(epoch: u64, verifier: ValidatorVerifier) -> Self {
        Self {
            epoch,
            verifier: verifier.into(),
        }
    }

    pub fn empty() -> Self {
        Self {
            epoch: 0,
            verifier: Arc::new(ValidatorVerifier::new(vec![])),
        }
    }
}

impl Verifier for EpochState {
    fn verify(&self, ledger_info: &LedgerInfoWithSignatures) -> anyhow::Result<()> {
        ensure!(
            self.epoch == ledger_info.ledger_info().epoch(),
            "LedgerInfo has unexpected epoch {}, expected {}",
            ledger_info.ledger_info().epoch(),
            self.epoch
        );
        ledger_info.verify_signatures(&self.verifier)?;
        Ok(())
    }

    fn epoch_change_verification_required(&self, epoch: u64) -> bool {
        self.epoch < epoch
    }

    fn is_ledger_info_stale(&self, ledger_info: &LedgerInfo) -> bool {
        ledger_info.epoch() < self.epoch
    }
}

// this is required by structured log
impl fmt::Debug for EpochState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl fmt::Display for EpochState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "EpochState [epoch: {}, validator: {}]",
            self.epoch, self.verifier
        )
    }
}
