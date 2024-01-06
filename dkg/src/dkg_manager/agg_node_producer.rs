use crate::{
    types::{DKGNodeAggState, DKGNodeRequest},
    DKGMessage,
};
use aptos_channels::aptos_channel;
use aptos_infallible::Mutex;
use aptos_reliable_broadcast::ReliableBroadcast;
use aptos_types::{
    dkg::{DKGAggNode, DKGPvssConfig},
    epoch_state::EpochState,
};
use futures::future::{AbortHandle, Abortable};
use move_core_types::account_address::AccountAddress;
use std::sync::Arc;
use tokio_retry::strategy::ExponentialBackoff;

/// A sub-process of the whole DKG process.
/// Once invoked by `DKGManager` to `start_produce`,
/// it starts producing an `AggDKGNode` and returns an abort handle.
/// Once an `AggDKGNode` is available, it is sent back via a channel given earlier.
pub trait AggNodeProducer: Send + Sync {
    fn start_produce(
        &self,
        epoch_state: EpochState,
        pvss_config: DKGPvssConfig,
        agg_node_tx: Option<aptos_channel::Sender<(), DKGAggNode>>,
    ) -> AbortHandle;
}

/// The real implementation of `AggNodeProducer` that broadcasts a `NodeRequest`, collects and verifies nodes from network.
pub struct RealAggNodeProducer {
    my_addr: AccountAddress,
    reliable_broadcast: Arc<ReliableBroadcast<DKGMessage, ExponentialBackoff>>,
}

impl RealAggNodeProducer {
    pub fn new(
        my_addr: AccountAddress,
        reliable_broadcast: ReliableBroadcast<DKGMessage, ExponentialBackoff>,
    ) -> Self {
        Self {
            my_addr,
            reliable_broadcast: Arc::new(reliable_broadcast),
        }
    }
}

impl AggNodeProducer for RealAggNodeProducer {
    fn start_produce(
        &self,
        epoch_state: EpochState,
        pvss_config: DKGPvssConfig,
        agg_node_tx: Option<aptos_channel::Sender<(), DKGAggNode>>,
    ) -> AbortHandle {
        let rb = self.reliable_broadcast.clone();
        let req = DKGNodeRequest::new(epoch_state.epoch);
        let agg_state = Arc::new(DKGNodeAggState::new(pvss_config, epoch_state, self.my_addr));
        let task = async move {
            let agg_node = rb.broadcast(req, agg_state).await;
            if let Some(tx) = agg_node_tx {
                tx.push((), agg_node);
            }
        };
        let (abort_handle, abort_registration) = AbortHandle::new_pair();
        tokio::spawn(Abortable::new(task, abort_registration));
        abort_handle
    }
}

#[cfg(test)]
pub struct DummyAggNodeProducer {
    pub invocations: Mutex<Vec<(EpochState, DKGPvssConfig)>>,
}

#[cfg(test)]
impl DummyAggNodeProducer {
    pub fn new() -> Self {
        Self {
            invocations: Mutex::new(vec![]),
        }
    }
}

#[cfg(test)]
impl AggNodeProducer for DummyAggNodeProducer {
    fn start_produce(
        &self,
        epoch_state: EpochState,
        pvss_config: DKGPvssConfig,
        agg_node_tx: Option<aptos_channel::Sender<(), DKGAggNode>>,
    ) -> AbortHandle {
        self.invocations.lock().push((epoch_state, pvss_config));
        let (abort_handle, _) = AbortHandle::new_pair();
        abort_handle
    }
}
