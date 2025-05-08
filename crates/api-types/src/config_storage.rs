
use bytes::Bytes;


pub enum OnChainConfig {
    ConsensusConfig,
    ExecutionConfig,
    ValidatorInfo,
}


/// Trait to be implemented by a storage type from which to read on-chain configs
pub trait ConfigStorage {
    fn fetch_config_bytes(&self, config_name: OnChainConfig, block_number: u64) -> Option<Bytes>;
}