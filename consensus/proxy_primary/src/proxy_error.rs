// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Error types for proxy primary consensus.

use aptos_consensus_types::common::Round;
use thiserror::Error;

/// Errors that can occur during proxy consensus.
#[derive(Debug, Error)]
pub enum ProxyConsensusError {
    #[error("Invalid last_primary_proof_round: expected {expected}, got {got}")]
    InvalidPrimaryRound { expected: Round, got: Round },

    #[error("Primary proof round too low: expected >= {expected}, got {got}")]
    PrimaryProofRoundMismatch { expected: Round, got: Round },

    #[error("Invalid proxy block: {0}")]
    InvalidProxyBlock(String),

    #[error("Internal error: {0}")]
    Internal(#[from] anyhow::Error),
}
