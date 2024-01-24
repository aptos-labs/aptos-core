// Copyright © Aptos Foundation

use crate::{
    transcript_aggregation::TranscriptAggregationState, types::DKGTranscriptRequest, DKGMessage,
};
use aptos_channels::aptos_channel::Sender;
use aptos_reliable_broadcast::ReliableBroadcast;
use aptos_types::{dkg::DKGTrait, epoch_state::EpochState};
use futures::future::AbortHandle;
use futures_util::future::Abortable;
use std::sync::Arc;
use tokio_retry::strategy::ExponentialBackoff;

/// A sub-process of the whole DKG process.
/// Once invoked by `DKGManager` to `start_produce`,
/// it starts producing an aggregated transcript and returns an abort handle.
/// Once an aggregated transcript is available, it is sent back via channel `agg_trx_tx`.
pub trait TAggTranscriptProducer<S: DKGTrait>: Send + Sync {
    fn start_produce(
        &self,
        epoch_state: Arc<EpochState>,
        dkg_config: S::PublicParams,
        agg_trx_tx: Option<Sender<(), S::Transcript>>,
    ) -> AbortHandle;
}

/// The real implementation of `AggTranscriptProducer` that broadcasts a `NodeRequest`, collects and verifies nodes from network.
pub struct AggTranscriptProducer {
    reliable_broadcast: Arc<ReliableBroadcast<DKGMessage, ExponentialBackoff>>,
}

impl AggTranscriptProducer {
    pub fn new(reliable_broadcast: ReliableBroadcast<DKGMessage, ExponentialBackoff>) -> Self {
        Self {
            reliable_broadcast: Arc::new(reliable_broadcast),
        }
    }
}

impl<DKG: DKGTrait + 'static> TAggTranscriptProducer<DKG> for AggTranscriptProducer {
    fn start_produce(
        &self,
        epoch_state: Arc<EpochState>,
        params: DKG::PublicParams,
        agg_trx_tx: Option<Sender<(), DKG::Transcript>>,
    ) -> AbortHandle {
        let rb = self.reliable_broadcast.clone();
        let req = DKGTranscriptRequest::new(epoch_state.epoch);
        let agg_state = Arc::new(TranscriptAggregationState::<DKG>::new(params, epoch_state));
        let task = async move {
            let agg_trx = rb.broadcast(req, agg_state).await;
            if let Some(tx) = agg_trx_tx {
                let _ = tx.push((), agg_trx); // If the `DKGManager` was dropped, this send will fail by design.
            }
        };
        let (abort_handle, abort_registration) = AbortHandle::new_pair();
        tokio::spawn(Abortable::new(task, abort_registration));
        abort_handle
    }
}

#[cfg(test)]
pub struct DummyAggTranscriptProducer {}

#[cfg(test)]
impl<DKG: DKGTrait> TAggTranscriptProducer<DKG> for DummyAggTranscriptProducer {
    fn start_produce(
        &self,
        _epoch_state: Arc<EpochState>,
        _dkg_config: DKG::PublicParams,
        _agg_trx_tx: Option<Sender<(), DKG::Transcript>>,
    ) -> AbortHandle {
        let (abort_handle, _) = AbortHandle::new_pair();
        abort_handle
    }
}
