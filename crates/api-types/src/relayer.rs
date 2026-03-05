use crate::{on_chain_config::jwks::JWKStruct, ExecError};
use async_trait::async_trait;
use std::sync::{Arc, OnceLock};

/// Result of polling a URI, containing JWK structures and the maximum block number fetched
/// It's polled directly from RPC provider, so it might be malicious.
/// Security relies on BFT quorum, not individual relayer trust.
#[derive(Debug, Clone)]
pub struct PollResult {
    /// JWK structures from the observed state
    pub jwk_structs: Vec<JWKStruct>,
    /// Maximum block number that was fetched in this poll (used for polling progress)
    pub max_block_number: u64,
    /// Nonce from the observed source (used for version comparison in JWKManager)
    /// For blockchain events: GravityPortal.MessageSent nonce
    pub nonce: Option<u64>,
    /// Whether the state was updated in this poll
    pub updated: bool,
}

#[async_trait]
pub trait Relayer: Send + Sync + 'static {
    async fn add_uri(&self, uri: &str, rpc_url: &str) -> Result<(), ExecError>;

    async fn get_last_state(&self, uri: &str) -> Result<PollResult, ExecError>;
}

pub static GLOBAL_RELAYER: OnceLock<Arc<dyn Relayer>> = OnceLock::new();
