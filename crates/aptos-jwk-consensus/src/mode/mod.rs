// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::types::JWKConsensusMsg;
use aptos_types::jwks::{ProviderJWKs, QuorumCertifiedUpdate};
use std::hash::Hash;

/// This trait captures the differences between per-issuer mode and per-key mode.
pub trait TConsensusMode: Send + Sync + 'static {
    type ReliableBroadcastRequest: Clone
        + Sync
        + Send
        + Into<JWKConsensusMsg>
        + TryFrom<JWKConsensusMsg>;
    type ConsensusSessionKey: Eq + Hash + Clone + Send;
    fn log_certify_start(epoch: u64, payload: &ProviderJWKs);
    fn new_rb_request(
        epoch: u64,
        payload: &ProviderJWKs,
    ) -> anyhow::Result<Self::ReliableBroadcastRequest>;
    fn log_certify_done(epoch: u64, qc: &QuorumCertifiedUpdate);
    fn session_key_from_qc(qc: &QuorumCertifiedUpdate)
        -> anyhow::Result<Self::ConsensusSessionKey>;
}

pub mod per_issuer;
pub mod per_key;
