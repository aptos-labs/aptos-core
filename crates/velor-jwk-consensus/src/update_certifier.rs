// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    mode::TConsensusMode, observation_aggregation::ObservationAggregationState,
    types::JWKConsensusMsg,
};
use anyhow::Context;
use velor_channels::velor_channel;
use velor_logger::error;
use velor_reliable_broadcast::ReliableBroadcast;
use velor_types::{
    epoch_state::EpochState,
    jwks::{ProviderJWKs, QuorumCertifiedUpdate},
};
use futures_util::future::{AbortHandle, Abortable};
use std::sync::Arc;
use tokio_retry::strategy::ExponentialBackoff;

/// A sub-process of the whole JWK consensus process.
/// Once invoked by `JWKConsensusManager` to `start_produce`,
/// it starts producing a `QuorumCertifiedUpdate` and returns an abort handle.
/// Once an `QuorumCertifiedUpdate` is available, it is sent back via a channel given earlier.
pub trait TUpdateCertifier<ConsensusMode: TConsensusMode>: Send + Sync {
    fn start_produce(
        &self,
        epoch_state: Arc<EpochState>,
        payload: ProviderJWKs,
        qc_update_tx: velor_channel::Sender<
            ConsensusMode::ConsensusSessionKey,
            QuorumCertifiedUpdate,
        >,
    ) -> anyhow::Result<AbortHandle>;
}

pub struct UpdateCertifier {
    reliable_broadcast: Arc<ReliableBroadcast<JWKConsensusMsg, ExponentialBackoff>>,
}

impl UpdateCertifier {
    pub fn new(reliable_broadcast: ReliableBroadcast<JWKConsensusMsg, ExponentialBackoff>) -> Self {
        Self {
            reliable_broadcast: Arc::new(reliable_broadcast),
        }
    }
}

impl<ConsensusMode: TConsensusMode> TUpdateCertifier<ConsensusMode> for UpdateCertifier {
    fn start_produce(
        &self,
        epoch_state: Arc<EpochState>,
        payload: ProviderJWKs,
        qc_update_tx: velor_channel::Sender<
            ConsensusMode::ConsensusSessionKey,
            QuorumCertifiedUpdate,
        >,
    ) -> anyhow::Result<AbortHandle> {
        ConsensusMode::log_certify_start(epoch_state.epoch, &payload);
        let rb = self.reliable_broadcast.clone();
        let epoch = epoch_state.epoch;
        let req = ConsensusMode::new_rb_request(epoch, &payload)
            .context("UpdateCertifier::start_produce failed at rb request construction")?;
        let agg_state = Arc::new(ObservationAggregationState::<ConsensusMode>::new(
            epoch_state,
            payload,
        ));
        let task = async move {
            let qc_update = rb.broadcast(req, agg_state).await.expect("cannot fail");
            ConsensusMode::log_certify_done(epoch, &qc_update);
            let session_key = ConsensusMode::session_key_from_qc(&qc_update);
            match session_key {
                Ok(key) => {
                    let _ = qc_update_tx.push(key, qc_update);
                },
                Err(e) => {
                    error!("JWK update QCed but could not identify the session key: {e}");
                },
            }
        };
        let (abort_handle, abort_registration) = AbortHandle::new_pair();
        tokio::spawn(Abortable::new(task, abort_registration));
        Ok(abort_handle)
    }
}
