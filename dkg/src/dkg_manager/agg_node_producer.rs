// Copyright © Aptos Foundation

use aptos_channels::aptos_channel::Sender;
use aptos_types::{
    dkg::{DKGAggNode, DKGConfig},
    epoch_state::EpochState,
};
use futures::future::AbortHandle;

/// A sub-process of the whole DKG process.
/// Once invoked by `DKGManager` to `start_produce`,
/// it starts producing an `AggDKGNode` and returns an abort handle.
/// Once an `AggDKGNode` is available, it is sent back via channel `agg_node_tx`.
pub trait AggNodeProducer: Send + Sync {
    fn start_produce(
        &self,
        epoch_state: EpochState,
        dkg_config: DKGConfig,
        agg_node_tx: Option<Sender<(), DKGAggNode>>,
    ) -> AbortHandle;
}

pub struct DummyAggNodeProducer {}

impl AggNodeProducer for DummyAggNodeProducer {
    fn start_produce(
        &self,
        _epoch_state: EpochState,
        _dkg_config: DKGConfig,
        _agg_node_tx: Option<Sender<(), DKGAggNode>>,
    ) -> AbortHandle {
        let (abort_handle, _) = AbortHandle::new_pair();
        abort_handle
    }
}
