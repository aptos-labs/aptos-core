// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

/// A simple enum for identifying clients of the compression crate. This
/// allows us to provide a runtime breakdown of compression metrics for
/// each client.
#[derive(Clone, Copy, Debug)]
pub enum CompressionClient {
    Consensus,
    ConsensusObserver,
    DKG,
    JWKConsensus,
    Mempool,
    StateSync,
}

impl CompressionClient {
    /// Returns a summary label for the request
    pub fn get_label(&self) -> &'static str {
        match self {
            Self::Consensus => "consensus",
            Self::ConsensusObserver => "consensus_observer",
            Self::DKG => "dkg",
            Self::JWKConsensus => "jwk_consensus",
            Self::Mempool => "mempool",
            Self::StateSync => "state_sync",
        }
    }
}
