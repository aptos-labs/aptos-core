// Copyright © Aptos Foundation

use crate::event::EventHandle;
use crate::on_chain_config::{OnChainConfig, ValidatorSet};
use serde::{Serialize, Deserialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DKGSessionState {
    pub dealer_epoch: u64,
    pub dealer_validator_set: ValidatorSet,
    pub target_epoch: u64,
    pub target_validator_set: ValidatorSet,
    pub result: Vec<u8>,
    pub deadline_microseconds: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DKGState {
    pub last_complete: Option<DKGSessionState>,
    pub in_progress: Option<DKGSessionState>,
    pub events: EventHandle,
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
