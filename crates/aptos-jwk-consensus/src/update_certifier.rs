// Copyright © Aptos Foundation

use crate::{
    observation_aggregation::ObservationAggregationState,
    types::{JWKConsensusMsg, ObservedUpdateRequest},
};
use aptos_channels::aptos_channel;
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

pub struct CertifiedUpdateProducer {
    reliable_broadcast: Arc<ReliableBroadcast<JWKConsensusMsg, ExponentialBackoff>>,
}

impl CertifiedUpdateProducer {
    pub fn new(reliable_broadcast: ReliableBroadcast<JWKConsensusMsg, ExponentialBackoff>) -> Self {
        Self {
            reliable_broadcast: Arc::new(reliable_broadcast),
        }
    }
}

impl TUpdateCertifier for CertifiedUpdateProducer {
    fn start_produce(
        &self,
        epoch_state: Arc<EpochState>,
        payload: ProviderJWKs,
        qc_update_tx: aptos_channel::Sender<Issuer, QuorumCertifiedUpdate>,
    ) -> AbortHandle {
        let rb = self.reliable_broadcast.clone();
        let issuer = payload.issuer.clone();
        let req = ObservedUpdateRequest {
            epoch: epoch_state.epoch,
            issuer: issuer.clone(),
        };
        let agg_state = Arc::new(ObservationAggregationState::new(epoch_state, payload));
        let task = async move {
            let qc_update = rb.broadcast(req, agg_state).await;
            let _ = qc_update_tx.push(issuer, qc_update);
        };
        let (abort_handle, abort_registration) = AbortHandle::new_pair();
        tokio::spawn(Abortable::new(task, abort_registration));
        abort_handle
    }
}
