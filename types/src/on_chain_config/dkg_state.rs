// Copyright © Aptos Foundation

use crate::event::EventHandle;
use crate::on_chain_config::OnChainConfig;
use serde::{Serialize, Deserialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DKGState {
    pub target_epoch: u64,
    pub state_id: u64,
    pub countdown: u64,
    pub serialized_transcript: Vec<u8>,
    pub events: EventHandle,
}

impl OnChainConfig for DKGState {
    const MODULE_IDENTIFIER: &'static str = "dkg";
    const TYPE_IDENTIFIER: &'static str = "DKGState";
}
