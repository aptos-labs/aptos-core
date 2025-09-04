// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{on_chain_config::OnChainConfig, validator_info::ValidatorInfo};
use move_core_types::account_address::AccountAddress;
#[cfg(any(test, feature = "fuzzing"))]
use proptest_derive::Arbitrary;
use serde::{Deserialize, Serialize};
use std::{
    fmt,
    iter::{Chain, IntoIterator},
    vec,
    vec::IntoIter,
};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(Arbitrary))]
#[repr(u8)]
pub enum ConsensusScheme {
    BLS12381 = 0,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(Arbitrary))]
pub struct ValidatorSet {
    pub scheme: ConsensusScheme,
    pub active_validators: Vec<ValidatorInfo>,
    pub pending_inactive: Vec<ValidatorInfo>,
    pub pending_active: Vec<ValidatorInfo>,
    pub total_voting_power: u128,
    pub total_joining_power: u128,
}

impl fmt::Display for ValidatorSet {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "[")?;
        for validator in self.payload() {
            write!(f, "{} ", validator)?;
        }
        write!(f, "]")
    }
}

impl ValidatorSet {
    /// Constructs a ValidatorSet resource.
    pub fn new(payload: Vec<ValidatorInfo>) -> Self {
        Self {
            scheme: ConsensusScheme::BLS12381,
            active_validators: payload,
            pending_inactive: vec![],
            pending_active: vec![],
            total_voting_power: 0,
            total_joining_power: 0,
        }
    }

    pub fn payload(&self) -> impl Iterator<Item = &ValidatorInfo> {
        self.active_validators
            .iter()
            .chain(self.pending_inactive.iter())
    }

    pub fn empty() -> Self {
        ValidatorSet::new(Vec::new())
    }

    pub fn num_validators(&self) -> usize {
        self.active_validators.len() + self.pending_inactive.len()
    }

    pub fn active_validators(&self) -> Vec<AccountAddress> {
        self.active_validators
            .iter()
            .cloned()
            .map(|v| v.account_address)
            .collect()
    }

    pub fn pending_active_validators(&self) -> Vec<AccountAddress> {
        self.pending_active
            .iter()
            .cloned()
            .map(|v| v.account_address)
            .collect()
    }

    pub fn pending_inactive_validators(&self) -> Vec<AccountAddress> {
        self.pending_inactive
            .iter()
            .cloned()
            .map(|v| v.account_address)
            .collect()
    }
}

impl OnChainConfig for ValidatorSet {
    // validator_set_address
    const MODULE_IDENTIFIER: &'static str = "stake";
    const TYPE_IDENTIFIER: &'static str = "ValidatorSet";
}

impl IntoIterator for ValidatorSet {
    type IntoIter = Chain<IntoIter<Self::Item>, IntoIter<Self::Item>>;
    type Item = ValidatorInfo;

    fn into_iter(self) -> Self::IntoIter {
        self.active_validators
            .into_iter()
            .chain(self.pending_inactive)
    }
}
