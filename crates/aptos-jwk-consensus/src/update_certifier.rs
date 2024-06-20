// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    observation_aggregation::ObservationAggregationState,
    types::{JWKConsensusMsg, ObservedUpdateRequest},
};
use aptos_channels::aptos_channel;
use aptos_logger::info;
use aptos_reliable_broadcast::ReliableBroadcast;
use aptos_types::{
    epoch_state::EpochState,
    jwks::{Issuer, ProviderJWKs, QuorumCertifiedUpdate},
};
use futures_util::future::{AbortHandle, Abortable};
use std::sync::Arc;
use tokio_retry::strategy::ExponentialBackoff;

/// A sub-process of the whole JWK consensus process.
/// Once invoked by `JWKConsensusManager` to `start_produce`,
/// it starts producing a `QuorumCertifiedUpdate` and returns an abort handle.
/// Once an `QuorumCertifiedUpdate` is available, it is sent back via a channel given earlier.
pub trait TUpdateCertifier: Send + Sync {
    fn start_produce(
        &self,
        epoch_state: Arc<EpochState>,
        payload: ProviderJWKs,
        qc_update_tx: aptos_channel::Sender<Issuer, QuorumCertifiedUpdate>,
    ) -> AbortHandle;
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

impl TUpdateCertifier for UpdateCertifier {
    fn start_produce(
        &self,
        epoch_state: Arc<EpochState>,
        payload: ProviderJWKs,
        qc_update_tx: aptos_channel::Sender<Issuer, QuorumCertifiedUpdate>,
    ) -> AbortHandle {
        let version = payload.version;
        info!(
            epoch = epoch_state.epoch,
            issuer = String::from_utf8(payload.issuer.clone()).ok(),
            version = version,
            "Start certifying update."
        );
        let rb = self.reliable_broadcast.clone();
        let epoch = epoch_state.epoch;
        let issuer = payload.issuer.clone();
        let req = ObservedUpdateRequest {
            epoch: epoch_state.epoch,
            issuer: issuer.clone(),
        };
        let agg_state = Arc::new(ObservationAggregationState::new(epoch_state, payload));
        let task = async move {
            let qc_update = rb.broadcast(req, agg_state).await.expect("cannot fail");
            info!(
                epoch = epoch,
                issuer = String::from_utf8(issuer.clone()).ok(),
                version = version,
                "Certified update obtained."
            );
            let _ = qc_update_tx.push(issuer, qc_update);
        };
        let (abort_handle, abort_registration) = AbortHandle::new_pair();
        tokio::spawn(Abortable::new(task, abort_registration));
        abort_handle
    }
}
