// Copyright © Aptos Foundation
use crate::{
    dkg::{build_dkg_pvss_config, DKGPvssConfig},
    on_chain_config::{OnChainConfig, ValidatorSet},
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DKGSessionState {
    pub start_time_us: u64,
    pub dealer_epoch: u64,
    pub dealer_validator_set: ValidatorSet,
    pub target_epoch: u64,
    pub target_validator_set: ValidatorSet,
    pub result: Vec<u8>,
    pub deadline_microseconds: u64,
}

impl DKGSessionState {
    pub fn pvss_config(&self) -> DKGPvssConfig {
        build_dkg_pvss_config(self.dealer_epoch, &self.target_validator_set)
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct DKGState {
    pub last_complete: Option<DKGSessionState>,
    pub in_progress: Option<DKGSessionState>,
}

impl DKGState {
    pub fn maybe_last_complete(&self, epoch: u64) -> Option<&DKGSessionState> {
        match &self.last_complete {
            Some(session) if session.target_epoch == epoch => Some(session),
            _ => None,
        }
    }

    pub fn last_complete(&self) -> &DKGSessionState {
        self.last_complete.as_ref().unwrap()
    }
}

impl OnChainConfig for DKGState {
    const MODULE_IDENTIFIER: &'static str = "dkg";
    const TYPE_IDENTIFIER: &'static str = "DKGState";
}
