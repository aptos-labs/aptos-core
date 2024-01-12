// Copyright © Aptos Foundation

use crate::{
    observation_aggregation::ObservationAggregationState,
    types::{JWKConsensusMsg, ObservedUpdateRequest},
};
use aptos_channels::aptos_channel;
use aptos_reliable_broadcast::ReliableBroadcast;
use aptos_types::{
    epoch_state::EpochState,
    jwks::{ProviderJWKs, QuorumCertifiedUpdate},
};
use futures_util::future::{AbortHandle, Abortable};
use move_core_types::account_address::AccountAddress;
use std::sync::Arc;
use tokio_retry::strategy::ExponentialBackoff;

/// A sub-process of the whole JWK consensus process.
/// Once invoked by `JWKConsensusManager` to `start_produce`,
/// it starts producing a `QuorumCertifiedUpdate` and returns an abort handle.
/// Once an `QuorumCertifiedUpdate` is available, it is sent back via a channel given earlier.
pub trait CertifiedUpdateProducer: Send + Sync {
    fn start_produce(
        &self,
        epoch_state: Arc<EpochState>,
        payload: ProviderJWKs,
        qc_update_tx: Option<aptos_channel::Sender<(), QuorumCertifiedUpdate>>,
    ) -> AbortHandle;
}

pub struct RealCertifiedUpdateProducer {
    my_addr: AccountAddress,
    reliable_broadcast: Arc<ReliableBroadcast<JWKConsensusMsg, ExponentialBackoff>>,
}

impl RealCertifiedUpdateProducer {
    pub fn new(
        my_addr: AccountAddress,
        reliable_broadcast: ReliableBroadcast<JWKConsensusMsg, ExponentialBackoff>,
    ) -> Self {
        Self {
            my_addr,
            reliable_broadcast: Arc::new(reliable_broadcast),
        }
    }
}

impl CertifiedUpdateProducer for RealCertifiedUpdateProducer {
    fn start_produce(
        &self,
        epoch_state: Arc<EpochState>,
        payload: ProviderJWKs,
        qc_update_tx: Option<aptos_channel::Sender<(), QuorumCertifiedUpdate>>,
    ) -> AbortHandle {
        let rb = self.reliable_broadcast.clone();
        let req = ObservedUpdateRequest {
            epoch: epoch_state.epoch,
            issuer: payload.issuer.clone(),
        };
        let agg_state = Arc::new(ObservationAggregationState::new(
            self.my_addr,
            epoch_state,
            payload,
        ));
        let task = async move {
            let qc_update = rb.broadcast(req, agg_state).await;
            if let Some(tx) = qc_update_tx {
                let _ = tx.push((), qc_update);
            }
        };
        let (abort_handle, abort_registration) = AbortHandle::new_pair();
        tokio::spawn(Abortable::new(task, abort_registration));
        abort_handle
    }
}
