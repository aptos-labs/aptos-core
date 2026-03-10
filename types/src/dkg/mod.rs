// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use self::real_dkg::RealDKG;
use crate::{
    dkg::real_dkg::rounding::DKGRoundingProfile,
    on_chain_config::{OnChainConfig, OnChainRandomnessConfig, RandomnessConfigMoveStruct},
    validator_verifier::{ValidatorConsensusInfo, ValidatorConsensusInfoMoveStruct},
};
use anyhow::{format_err, Result};
use aptos_crypto::Uniform;
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use move_core_types::{
    account_address::AccountAddress, ident_str, identifier::IdentStr, language_storage::TypeTag,
    move_resource::MoveStructType,
};
use once_cell::sync::Lazy;
use rand::{CryptoRng, RngCore};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    fmt::{Debug, Formatter},
    time::Duration,
};

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq, CryptoHasher, BCSCryptoHash)]
pub struct DKGTranscriptMetadata {
    pub epoch: u64,
    pub author: AccountAddress,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DKGStartEvent {
    pub session_metadata: DKGSessionMetadata,
    pub start_time_us: u64,
}

impl MoveStructType for DKGStartEvent {
    const MODULE_NAME: &'static IdentStr = ident_str!("dkg");
    const STRUCT_NAME: &'static IdentStr = ident_str!("DKGStartEvent");
}

pub static DKG_START_EVENT_MOVE_TYPE_TAG: Lazy<TypeTag> =
    Lazy::new(|| TypeTag::Struct(Box::new(DKGStartEvent::struct_tag())));

/// DKG transcript and its metadata.
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DKGTranscript {
    pub metadata: DKGTranscriptMetadata,
    #[serde(with = "serde_bytes")]
    pub transcript_bytes: Vec<u8>,
}

impl Debug for DKGTranscript {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DKGTranscript")
            .field("metadata", &self.metadata)
            .field("transcript_bytes_len", &self.transcript_bytes.len())
            .finish()
    }
}

impl DKGTranscript {
    pub fn new(epoch: u64, author: AccountAddress, transcript_bytes: Vec<u8>) -> Self {
        Self {
            metadata: DKGTranscriptMetadata { epoch, author },
            transcript_bytes,
        }
    }

    pub fn dummy() -> Self {
        Self {
            metadata: DKGTranscriptMetadata {
                epoch: 0,
                author: AccountAddress::ZERO,
            },
            transcript_bytes: vec![],
        }
    }
}

/// Reflection of `0x1::dkg::DKGSessionMetadata` in rust.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DKGSessionMetadata {
    pub dealer_epoch: u64,
    pub randomness_config: RandomnessConfigMoveStruct,
    pub dealer_validator_set: Vec<ValidatorConsensusInfoMoveStruct>,
    pub target_validator_set: Vec<ValidatorConsensusInfoMoveStruct>,
}

impl DKGSessionMetadata {
    pub fn target_validator_consensus_infos_cloned(&self) -> Vec<ValidatorConsensusInfo> {
        self.target_validator_set
            .clone()
            .into_iter()
            .filter_map(|obj| match obj.try_into() {
                Ok(info) => Some(info),
                Err(e) => {
                    tracing::warn!("Failed to convert target validator consensus info: {}", e);
                    None
                },
            })
            .collect()
    }

    pub fn dealer_consensus_infos_cloned(&self) -> Vec<ValidatorConsensusInfo> {
        self.dealer_validator_set
            .clone()
            .into_iter()
            .filter_map(|obj| match obj.try_into() {
                Ok(info) => Some(info),
                Err(e) => {
                    tracing::warn!("Failed to convert dealer validator consensus info: {}", e);
                    None
                },
            })
            .collect()
    }

    pub fn randomness_config_derived(&self) -> Option<OnChainRandomnessConfig> {
        OnChainRandomnessConfig::try_from(self.randomness_config.clone()).ok()
    }

    /// Convert from api_types DKGSessionMetadata to types DKGSessionMetadata
    pub fn from_api_types(
        api_metadata: api_types::on_chain_config::dkg::DKGSessionMetadata,
    ) -> Result<Self> {
        // Convert validator sets
        let dealer_validator_set = api_metadata
            .dealer_validator_set
            .into_iter()
            .map(|v| DKGSessionMetadata::convert_validator_consensus_info_from_api(v))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format_err!("Failed to convert dealer validator set: {}", e))?;

        let target_validator_set = api_metadata
            .target_validator_set
            .into_iter()
            .map(|v| DKGSessionMetadata::convert_validator_consensus_info_from_api(v))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format_err!("Failed to convert target validator set: {}", e))?;

        // Convert randomness config from api_types format
        let randomness_config =
            Self::convert_randomness_config_from_api(&api_metadata.randomness_config)?;

        Ok(DKGSessionMetadata {
            dealer_epoch: api_metadata.dealer_epoch,
            randomness_config,
            dealer_validator_set,
            target_validator_set,
        })
    }

    /// Convert from types DKGSessionMetadata to api_types DKGSessionMetadata
    pub fn to_api_types(&self) -> Result<api_types::on_chain_config::dkg::DKGSessionMetadata> {
        // Convert validator sets
        let dealer_validator_set = self
            .dealer_validator_set
            .iter()
            .map(|v| DKGSessionMetadata::convert_validator_consensus_info_to_api(v))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format_err!("Failed to convert dealer validator set: {}", e))?;

        let target_validator_set = self
            .target_validator_set
            .iter()
            .map(|v| DKGSessionMetadata::convert_validator_consensus_info_to_api(v))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format_err!("Failed to convert target validator set: {}", e))?;

        // Convert randomness config to api_types format
        let randomness_config = Self::convert_randomness_config_to_api(&self.randomness_config)?;

        Ok(api_types::on_chain_config::dkg::DKGSessionMetadata {
            dealer_epoch: self.dealer_epoch,
            randomness_config,
            dealer_validator_set,
            target_validator_set,
        })
    }

    /// Helper function to convert api_types ValidatorConsensusInfo to types ValidatorConsensusInfoMoveStruct
    fn convert_validator_consensus_info_from_api(
        api_validator: api_types::on_chain_config::dkg::ValidatorConsensusInfo,
    ) -> Result<ValidatorConsensusInfoMoveStruct> {
        // Convert ExternalAccountAddress to AccountAddress
        let addr = AccountAddress::from_bytes(&api_validator.addr.bytes())
            .map_err(|e| format_err!("Failed to convert address: {}", e))?;

        Ok(ValidatorConsensusInfoMoveStruct {
            addr,
            pk_bytes: api_validator.pk_bytes,
            voting_power: api_validator.voting_power,
        })
    }

    /// Helper function to convert types ValidatorConsensusInfoMoveStruct to api_types ValidatorConsensusInfo
    fn convert_validator_consensus_info_to_api(
        validator: &ValidatorConsensusInfoMoveStruct,
    ) -> Result<api_types::on_chain_config::dkg::ValidatorConsensusInfo> {
        // Convert AccountAddress to ExternalAccountAddress
        let addr = api_types::account::ExternalAccountAddress::new(validator.addr.into_bytes());

        Ok(api_types::on_chain_config::dkg::ValidatorConsensusInfo {
            addr,
            pk_bytes: validator.pk_bytes.clone(),
            voting_power: validator.voting_power,
        })
    }

    /// Helper function to convert api_types RandomnessConfigData to types RandomnessConfigMoveStruct
    ///
    /// NOTE: We directly construct ConfigV1/ConfigV2 with the original FixedPoint64 values
    /// instead of going through percentage conversion (which is lossy and causes DKG crashes).
    /// The old code did: FixedPoint64 → percentage (u64) → FixedPoint64, losing precision
    /// (e.g., 2/3 = 0.6666... → 66% → 0.66), causing DKG rounding mismatch.
    fn convert_randomness_config_from_api(
        api_config: &api_types::on_chain_config::dkg::RandomnessConfigData,
    ) -> Result<RandomnessConfigMoveStruct> {
        use crate::on_chain_config::randomness_config::{ConfigV1, ConfigV2};

        let on_chain_config = match api_config.variant {
            api_types::on_chain_config::dkg::ConfigVariant::V1 => {
                let config_v1 = &api_config.configV1;
                let secrecy_threshold =
                    DKGSessionMetadata::convert_fixed_point_from_api(&config_v1.secrecyThreshold)?;
                let reconstruction_threshold = DKGSessionMetadata::convert_fixed_point_from_api(
                    &config_v1.reconstructionThreshold,
                )?;

                OnChainRandomnessConfig::V1(ConfigV1 {
                    secrecy_threshold,
                    reconstruction_threshold,
                })
            },
            api_types::on_chain_config::dkg::ConfigVariant::V2 => {
                let config_v2 = &api_config.configV2;
                let secrecy_threshold =
                    DKGSessionMetadata::convert_fixed_point_from_api(&config_v2.secrecyThreshold)?;
                let reconstruction_threshold = DKGSessionMetadata::convert_fixed_point_from_api(
                    &config_v2.reconstructionThreshold,
                )?;
                let fast_path_secrecy_threshold = DKGSessionMetadata::convert_fixed_point_from_api(
                    &config_v2.fastPathSecrecyThreshold,
                )?;

                OnChainRandomnessConfig::V2(ConfigV2 {
                    secrecy_threshold,
                    reconstruction_threshold,
                    fast_path_secrecy_threshold,
                })
            },
        };

        Ok(RandomnessConfigMoveStruct::from(on_chain_config))
    }

    /// Helper function to convert types RandomnessConfigMoveStruct to api_types RandomnessConfigData
    fn convert_randomness_config_to_api(
        config: &RandomnessConfigMoveStruct,
    ) -> Result<api_types::on_chain_config::dkg::RandomnessConfigData> {
        use api_types::on_chain_config::dkg::{ConfigV1, ConfigV2, ConfigVariant, FixedPoint64};

        // Convert RandomnessConfigMoveStruct to OnChainRandomnessConfig
        let on_chain_config = OnChainRandomnessConfig::try_from(config.clone())
            .map_err(|e| format_err!("Failed to convert RandomnessConfigMoveStruct: {}", e))?;

        match on_chain_config {
            OnChainRandomnessConfig::Off => {
                // For Off config, return a default V1 config with zero values
                let config_v1 = ConfigV1 {
                    secrecyThreshold: FixedPoint64 { value: 0 },
                    reconstructionThreshold: FixedPoint64 { value: 0 },
                };

                let config_v2 = ConfigV2 {
                    secrecyThreshold: FixedPoint64 { value: 0 },
                    reconstructionThreshold: FixedPoint64 { value: 0 },
                    fastPathSecrecyThreshold: FixedPoint64 { value: 0 },
                };

                Ok(api_types::on_chain_config::dkg::RandomnessConfigData {
                    variant: ConfigVariant::V1,
                    configV1: config_v1,
                    configV2: config_v2,
                })
            },
            OnChainRandomnessConfig::V1(config_v1) => {
                let secrecy_threshold =
                    DKGSessionMetadata::convert_fixed_point_to_api(&config_v1.secrecy_threshold)?;
                let reconstruction_threshold = DKGSessionMetadata::convert_fixed_point_to_api(
                    &config_v1.reconstruction_threshold,
                )?;

                let api_config_v1 = ConfigV1 {
                    secrecyThreshold: secrecy_threshold,
                    reconstructionThreshold: reconstruction_threshold,
                };

                let config_v2 = ConfigV2 {
                    secrecyThreshold: FixedPoint64 { value: 0 },
                    reconstructionThreshold: FixedPoint64 { value: 0 },
                    fastPathSecrecyThreshold: FixedPoint64 { value: 0 },
                };

                Ok(api_types::on_chain_config::dkg::RandomnessConfigData {
                    variant: ConfigVariant::V1,
                    configV1: api_config_v1,
                    configV2: config_v2,
                })
            },
            OnChainRandomnessConfig::V2(config_v2) => {
                let secrecy_threshold =
                    DKGSessionMetadata::convert_fixed_point_to_api(&config_v2.secrecy_threshold)?;
                let reconstruction_threshold = DKGSessionMetadata::convert_fixed_point_to_api(
                    &config_v2.reconstruction_threshold,
                )?;
                let fast_path_secrecy_threshold = DKGSessionMetadata::convert_fixed_point_to_api(
                    &config_v2.fast_path_secrecy_threshold,
                )?;

                let config_v1 = ConfigV1 {
                    secrecyThreshold: FixedPoint64 { value: 0 },
                    reconstructionThreshold: FixedPoint64 { value: 0 },
                };

                let api_config_v2 = ConfigV2 {
                    secrecyThreshold: secrecy_threshold,
                    reconstructionThreshold: reconstruction_threshold,
                    fastPathSecrecyThreshold: fast_path_secrecy_threshold,
                };

                Ok(api_types::on_chain_config::dkg::RandomnessConfigData {
                    variant: ConfigVariant::V2,
                    configV1: config_v1,
                    configV2: api_config_v2,
                })
            },
        }
    }

    /// Helper function to convert api_types FixedPoint64 to types FixedPoint64MoveStruct
    fn convert_fixed_point_from_api(
        api_fixed_point: &api_types::on_chain_config::dkg::FixedPoint64,
    ) -> Result<crate::move_fixed_point::FixedPoint64MoveStruct> {
        use crate::move_fixed_point::FixedPoint64MoveStruct;
        use fixed::types::U64F64;

        // Convert u128 value to U64F64, then to FixedPoint64MoveStruct
        let u64f64 = U64F64::from_bits(api_fixed_point.value);
        Ok(FixedPoint64MoveStruct::from_u64f64(u64f64))
    }

    /// Helper function to convert types FixedPoint64MoveStruct to api_types FixedPoint64
    fn convert_fixed_point_to_api(
        fixed_point: &crate::move_fixed_point::FixedPoint64MoveStruct,
    ) -> Result<api_types::on_chain_config::dkg::FixedPoint64> {
        use fixed::types::U64F64;

        // Convert FixedPoint64MoveStruct to U64F64, then to u128
        let u64f64 = fixed_point.as_u64f64();
        Ok(api_types::on_chain_config::dkg::FixedPoint64 {
            value: u64f64.to_bits(),
        })
    }

    /// Helper function to convert FixedPoint64MoveStruct to percentage (u64)
    fn fixed_point_to_percentage(
        fixed_point: &crate::move_fixed_point::FixedPoint64MoveStruct,
    ) -> Result<u64> {
        use fixed::types::U64F64;

        // Convert FixedPoint64MoveStruct to U64F64, then multiply by 100 to get percentage
        let u64f64 = fixed_point.as_u64f64();
        let percentage = u64f64 * U64F64::from_num(100);

        // Convert to u64, rounding to nearest integer
        Ok(percentage.to_num::<u64>())
    }
}

impl MayHaveRoundingSummary for DKGSessionMetadata {
    fn rounding_summary(&self) -> Option<&RoundingSummary> {
        None
    }
}

/// Reflection of Move type `0x1::dkg::DKGSessionState`.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DKGSessionState {
    pub metadata: DKGSessionMetadata,
    pub start_time_us: u64,
    pub transcript: Vec<u8>,
}

impl DKGSessionState {
    pub fn target_epoch(&self) -> u64 {
        self.metadata.dealer_epoch + 1
    }

    /// Convert from api_types DKGSessionState to types DKGSessionState
    pub fn from_api_types(
        api_session: api_types::on_chain_config::dkg::DKGSessionState,
    ) -> Result<Self> {
        let metadata = DKGSessionMetadata::from_api_types(api_session.metadata)?;

        Ok(DKGSessionState {
            metadata,
            start_time_us: api_session.start_time_us,
            transcript: api_session.transcript,
        })
    }

    /// Convert from types DKGSessionState to api_types DKGSessionState
    pub fn to_api_types(&self) -> Result<api_types::on_chain_config::dkg::DKGSessionState> {
        let metadata = self.metadata.to_api_types()?;

        Ok(api_types::on_chain_config::dkg::DKGSessionState {
            metadata,
            start_time_us: self.start_time_us,
            transcript: self.transcript.clone(),
        })
    }
}
/// Reflection of Move type `0x1::dkg::DKGState`.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct DKGState {
    pub last_completed: Option<DKGSessionState>,
    pub in_progress: Option<DKGSessionState>,
}

impl DKGState {
    pub fn maybe_last_complete(&self, epoch: u64) -> Option<&DKGSessionState> {
        match &self.last_completed {
            Some(session) if session.target_epoch() == epoch => Some(session),
            _ => None,
        }
    }

    pub fn last_complete(&self) -> &DKGSessionState {
        self.last_completed.as_ref().unwrap()
    }

    /// Convert from api_types DKGState to types DKGState
    pub fn from_api_types(api_state: api_types::on_chain_config::dkg::DKGState) -> Result<Self> {
        let last_completed = if let Some(api_session) = api_state.last_completed {
            Some(DKGSessionState::from_api_types(api_session)?)
        } else {
            None
        };

        let in_progress = if let Some(api_session) = api_state.in_progress {
            Some(DKGSessionState::from_api_types(api_session)?)
        } else {
            None
        };

        Ok(DKGState {
            last_completed,
            in_progress,
        })
    }

    /// Convert from types DKGState to api_types DKGState
    pub fn to_api_types(&self) -> Result<api_types::on_chain_config::dkg::DKGState> {
        let last_completed = if let Some(session) = &self.last_completed {
            Some(session.to_api_types()?)
        } else {
            None
        };

        let in_progress = if let Some(session) = &self.in_progress {
            Some(session.to_api_types()?)
        } else {
            None
        };

        Ok(api_types::on_chain_config::dkg::DKGState {
            last_completed,
            in_progress,
        })
    }
}

impl OnChainConfig for DKGState {
    const MODULE_IDENTIFIER: &'static str = "dkg";
    const TYPE_IDENTIFIER: &'static str = "DKGState";

    fn deserialize_into_config(bytes: &[u8]) -> Result<Self> {
        // Deserialize from api_types DKGState format
        let api_dkg_state = bcs::from_bytes::<api_types::on_chain_config::dkg::DKGState>(bytes)
            .map_err(|e| {
                format_err!(
                    "[dkg state config] Failed to deserialize api_types DKGState: {}",
                    e
                )
            })?;

        // Convert api_types DKGState to types DKGState
        let dkg_state = Self::from_api_types(api_dkg_state)?;

        Ok(dkg_state)
    }
}

#[derive(Clone, Debug, Default)]
pub struct RoundingSummary {
    pub method: String,
    pub output: DKGRoundingProfile,
    pub error: Option<String>,
    pub exec_time: Duration,
}

pub trait MayHaveRoundingSummary {
    fn rounding_summary(&self) -> Option<&RoundingSummary>;
}

/// NOTE: this is a subset of the full scheme. Some data items/algorithms are not used in DKG and are omitted.
pub trait DKGTrait: Debug {
    type DealerPrivateKey;
    type PublicParams: Clone + Debug + Send + Sync + MayHaveRoundingSummary;
    type Transcript: Clone + Send + Sync + Serialize + for<'a> Deserialize<'a>;
    type InputSecret: Uniform;
    type DealtSecret;
    type DealtSecretShare;
    type DealtPubKeyShare;
    type NewValidatorDecryptKey: Uniform;

    fn new_public_params(dkg_session_metadata: &DKGSessionMetadata) -> Self::PublicParams;
    fn aggregate_input_secret(secrets: Vec<Self::InputSecret>) -> Self::InputSecret;
    fn dealt_secret_from_input(
        pub_params: &Self::PublicParams,
        input: &Self::InputSecret,
    ) -> Self::DealtSecret;
    fn generate_transcript<R: CryptoRng + RngCore>(
        rng: &mut R,
        params: &Self::PublicParams,
        input_secret: &Self::InputSecret,
        my_index: u64,
        sk: &Self::DealerPrivateKey,
    ) -> Self::Transcript;

    fn verify_transcript(params: &Self::PublicParams, trx: &Self::Transcript) -> Result<()>;

    fn aggregate_transcripts(
        params: &Self::PublicParams,
        accumulator: &mut Self::Transcript,
        element: Self::Transcript,
    );

    fn decrypt_secret_share_from_transcript(
        pub_params: &Self::PublicParams,
        trx: &Self::Transcript,
        player_idx: u64,
        dk: &Self::NewValidatorDecryptKey,
    ) -> Result<(Self::DealtSecretShare, Self::DealtPubKeyShare)>;

    fn reconstruct_secret_from_shares(
        pub_params: &Self::PublicParams,
        player_share_pairs: Vec<(u64, Self::DealtSecretShare)>,
    ) -> Result<Self::DealtSecret>;
    fn get_dealers(transcript: &Self::Transcript) -> BTreeSet<u64>;
}

pub mod dummy_dkg;
pub mod real_dkg;

pub type DefaultDKG = RealDKG;
