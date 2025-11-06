use serde::{Deserialize, Serialize};
use crate::account::ExternalAccountAddress;

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
/// DKG transcript and its metadata.
pub struct DKGTranscript {
    pub metadata: DKGTranscriptMetadata,
    #[serde(with = "serde_bytes")]
    pub transcript_bytes: Vec<u8>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct DKGTranscriptMetadata {
    pub epoch: u64,
    pub author: ExternalAccountAddress,
}

#[derive(Hash, Clone, Eq, PartialEq, Serialize, Deserialize, Debug)]
pub struct DKGStartEvent {
    pub session_metadata: DKGSessionMetadata,
    pub start_time_us: u64,
}


#[derive(Hash, Clone, Eq, PartialEq, Serialize, Deserialize, Debug)]
pub struct RandomnessConfigData {
    pub variant: ConfigVariant,
    pub configV1: ConfigV1,
    pub configV2: ConfigV2,
}

#[derive(Hash, Clone, Eq, PartialEq, Serialize, Deserialize, Debug)]
pub enum ConfigVariant {
    V1,
    V2,
}


#[derive(Hash, Clone, Eq, PartialEq, Serialize, Deserialize, Debug)]
pub struct ConfigV1 {
    pub secrecyThreshold: FixedPoint64,
    pub reconstructionThreshold: FixedPoint64,
}


#[derive(Hash, Clone, Eq, PartialEq, Serialize, Deserialize, Debug)]
pub struct ConfigV2 {
    pub secrecyThreshold: FixedPoint64,
    pub reconstructionThreshold: FixedPoint64,
    pub fastPathSecrecyThreshold: FixedPoint64,
}


#[derive(Hash, Clone, Eq, PartialEq, Serialize, Deserialize, Debug)]
pub struct FixedPoint64 {
    pub value: u128,
}



/// Reflection of `0x1::dkg::DKGSessionMetadata` in rust.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Hash)]
pub struct DKGSessionMetadata {
    pub dealer_epoch: u64,
    pub randomness_config: RandomnessConfigData,
    pub dealer_validator_set: Vec<ValidatorConsensusInfo>,
    pub target_validator_set: Vec<ValidatorConsensusInfo>,
}
/// Reflection of `0x1::types::ValidatorConsensusInfo` in rust.
#[derive(Hash, Clone, Eq, PartialEq, Serialize, Deserialize, Debug)]
pub struct ValidatorConsensusInfo {
    pub addr: ExternalAccountAddress,
    pub pk_bytes: Vec<u8>,
    pub voting_power: u64,
}

#[derive(Hash, Clone, Eq, PartialEq, Serialize, Deserialize, Debug)]
pub struct DKGSessionState {
    pub metadata: DKGSessionMetadata,
    pub start_time_us: u64,
    pub transcript: Vec<u8>,
}

/// Reflection of Move type `0x1::dkg::DKGState`.
#[derive(Hash, Clone, Eq, PartialEq, Serialize, Deserialize, Debug, Default)]
pub struct DKGState {
    pub last_completed: Option<DKGSessionState>,
    pub in_progress: Option<DKGSessionState>,
}
