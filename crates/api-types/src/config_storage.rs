use bytes::Bytes;
use std::{str::FromStr, sync::{Arc, OnceLock}};

#[derive(Debug)]
pub enum OnChainConfig {
    ConsensusConfig,
    ExecutionConfig,
    ChainId,
    Configuration,
    ApprovedExecutionHashes,
    Version,
    GasSchedule,
    JWKConsensusConfig,
    RandomnessConfigSeqNum,
    RandomnessConfig,
    CurrentTimeMicroseconds,
    PerBlockRandomness,
    ValidatorSet,
    Epoch,
    ObservedJWKs,
    Features,
}

impl FromStr for OnChainConfig {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "ConsensusConfig" => Ok(OnChainConfig::ConsensusConfig),
            "ExecutionConfig" => Ok(OnChainConfig::ExecutionConfig),
            "ChainId" => Ok(OnChainConfig::ChainId),
            "Configuration" => Ok(OnChainConfig::Configuration),
            "ApprovedExecutionHashes" => Ok(OnChainConfig::ApprovedExecutionHashes),
            "Version" => Ok(OnChainConfig::Version),
            "GasSchedule" => Ok(OnChainConfig::GasSchedule),
            "JWKConsensusConfig" => Ok(OnChainConfig::JWKConsensusConfig),
            "ValidatorSet" => Ok(OnChainConfig::ValidatorSet),
            "Epoch" => Ok(OnChainConfig::Epoch),
            "PerBlockRandomness" => Ok(OnChainConfig::PerBlockRandomness),
            "RandomnessConfigSeqNum" => Ok(OnChainConfig::RandomnessConfigSeqNum),
            "RandomnessConfig" => Ok(OnChainConfig::RandomnessConfig),
            "CurrentTimeMicroseconds" => Ok(OnChainConfig::CurrentTimeMicroseconds),
            "ObservedJWKs" => Ok(OnChainConfig::ObservedJWKs),
            "Features" => Ok(OnChainConfig::Features),
            _ => Err(format!("Unknown OnChainConfig variant: {}", s)),
        }
    }
}

impl TryFrom<String> for OnChainConfig {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}

#[derive(Debug)]
pub struct OnChainConfigResType {
    bytes: Bytes,
    // TODO(Gravity_alex): add a type to indicate the type of the on-chain config
}

impl From<u64> for OnChainConfigResType {
    fn from(value: u64) -> Self {
        let serialized_bytes =
            bcs::to_bytes(&value).expect("BCS serialization of u64 should not fail");

        OnChainConfigResType {
            bytes: Bytes::from(serialized_bytes),
        }
    }
}

impl From<Bytes> for OnChainConfigResType {
    fn from(value: Bytes) -> Self {
        OnChainConfigResType { bytes: value }
    }
}

impl TryInto<u64> for OnChainConfigResType {
    type Error = String;

    fn try_into(self) -> Result<u64, Self::Error> {
        let bytes = self.bytes.as_ref();
        let value = bcs::from_bytes::<u64>(bytes)
            .map_err(|e| format!("Failed to deserialize u64: {}", e))?;
        Ok(value)
    }
}

impl TryInto<Bytes> for OnChainConfigResType {
    type Error = String;

    fn try_into(self) -> Result<Bytes, Self::Error> {
        Ok(self.bytes)
    }
}

/// Trait to be implemented by a storage type from which to read on-chain configs
pub trait ConfigStorage: Send + Sync + 'static {
    fn fetch_config_bytes(
        &self,
        config_name: OnChainConfig,
        block_number: u64,
    ) -> Option<OnChainConfigResType>;
}

pub static GLOBAL_CONFIG_STORAGE: OnceLock<Arc<dyn ConfigStorage>> = OnceLock::new();