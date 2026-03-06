use serde::{Deserialize, Serialize};

/// Represents the latest state for an oracle source
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OracleSourceState {
    /// Source type (e.g., 0 = BLOCKCHAIN)
    pub source_type: u32,
    /// Source ID (e.g., chain ID)
    pub source_id: u64,
    /// Latest nonce for this source
    pub latest_nonce: u128,
    /// The latest DataRecord (None if nonce is 0)
    pub latest_record: Option<LatestDataRecord>,
}

/// Rust representation of the on-chain DataRecord
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatestDataRecord {
    /// Timestamp when this was recorded
    pub recorded_at: u64,
    /// Source block number
    pub block_number: u64,
    /// Payload data
    pub data: Vec<u8>,
}