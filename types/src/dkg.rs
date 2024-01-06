// Copyright © Aptos Foundation

use crate::on_chain_config::{OnChainConfig, ValidatorSet};
use aptos_crypto_derive::CryptoHasher;
use move_core_types::{ident_str, identifier::IdentStr, move_resource::MoveStructType};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

/// Reflection of Move type `0x1::dkg::DKGStartEvent`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DKGStartEvent {
    pub target_epoch: u64,
    pub start_time_us: u64,
    pub target_validator_set: ValidatorSet,
}

impl MoveStructType for DKGStartEvent {
    const MODULE_NAME: &'static IdentStr = ident_str!("dkg");
    const STRUCT_NAME: &'static IdentStr = ident_str!("DKGStartEvent");
}

/// DKG parameters.
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct DKGConfig {
    //TODO
}

/// Aggregated DKG transcript.
#[derive(Clone, Serialize, Deserialize, CryptoHasher, Debug, PartialEq, Eq)]
pub struct DKGAggNode {
    //TODO
}

/// Reflection of Move type `0x1::dkg::DKGSessionState`.
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

/// Reflection of Move type `0x1::dkg::DKGState`.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct DKGState {
    pub last_complete: Option<DKGSessionState>,
    pub in_progress: Option<DKGSessionState>,
}

impl OnChainConfig for DKGState {
    const MODULE_IDENTIFIER: &'static str = "dkg";
    const TYPE_IDENTIFIER: &'static str = "DKGState";
}
