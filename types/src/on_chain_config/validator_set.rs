// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{on_chain_config::OnChainConfig, validator_info::ValidatorInfo};

use crate::on_chain_config::{ConfigID, CONFIG_ADDRESS_STR};
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
    Ed25519 = 0,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(Arbitrary))]
pub struct ValidatorSet {
    scheme: ConsensusScheme,
    minimum_stake: u64,
    maximum_stake: u64,
    active_validators: Vec<ValidatorInfo>,
    pending_inactive: Vec<ValidatorInfo>,
    pending_active: Vec<ValidatorInfo>,
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
            scheme: ConsensusScheme::Ed25519,
            minimum_stake: 0,
            maximum_stake: 0,
            active_validators: payload,
            pending_inactive: vec![],
            pending_active: vec![],
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
}

impl OnChainConfig for ValidatorSet {
    // validator_set_address
    const IDENTIFIER: &'static str = "ValidatorSet";
    const CONFIG_ID: ConfigID = ConfigID(CONFIG_ADDRESS_STR, "Stake", Self::IDENTIFIER);
}

impl IntoIterator for ValidatorSet {
    type Item = ValidatorInfo;
    type IntoIter = Chain<IntoIter<Self::Item>, IntoIter<Self::Item>>;

    fn into_iter(self) -> Self::IntoIter {
        self.active_validators
            .into_iter()
            .chain(self.pending_inactive.into_iter())
    }
}
