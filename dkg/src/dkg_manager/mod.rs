// Copyright © Aptos Foundation
use crate::{dkg_manager::agg_node_producer::AggNodeProducer, network::IncomingRpcRequest};
use aptos_channels::aptos_channel;
use aptos_types::{
    dkg::{DKGAggNode, DKGSessionState, DKGStartEvent},
    epoch_state::EpochState,
};
use aptos_validator_transaction_pool as vtxn_pool;
use futures_channel::oneshot;
use futures_util::{FutureExt, StreamExt};
use move_core_types::account_address::AccountAddress;
use std::sync::Arc;

pub mod agg_node_producer;

#[allow(dead_code)]
pub struct DKGManager {
    my_addr: AccountAddress,
    epoch_state: EpochState,
    vtxn_pool_write_cli: Arc<vtxn_pool::SingleTopicWriteClient>,
    agg_node_producer: Arc<dyn AggNodeProducer>,
    agg_node_tx: Option<aptos_channel::Sender<(), DKGAggNode>>,
    //TODO: inner state and sk
}

#[allow(clippy::never_loop)]
impl DKGManager {
    pub fn new(
        my_addr: AccountAddress,
        epoch_state: EpochState,
        agg_node_producer: Arc<dyn AggNodeProducer>,
        vtxn_pool_write_cli: Arc<vtxn_pool::SingleTopicWriteClient>,
    ) -> Self {
        Self {
            my_addr,
            epoch_state,
            vtxn_pool_write_cli,
            agg_node_tx: None,
            agg_node_producer,
        }
    }

    pub async fn run(
        self,
        _in_progress_session: Option<DKGSessionState>,
        _start_dkg_event_rx: aptos_channel::Receiver<(), DKGStartEvent>,
        _rpc_msg_rx: aptos_channel::Receiver<(), (AccountAddress, IncomingRpcRequest)>,
        _dkg_txn_pulled_rx: vtxn_pool::PullNotificationReceiver,
        close_rx: oneshot::Receiver<oneshot::Sender<()>>,
    ) {
        let mut close_rx = close_rx.into_stream();
        loop {
            tokio::select! {
                //TODO: handle other events
                close_req = close_rx.select_next_some() => {
                    self.vtxn_pool_write_cli.put(None);
                    if let Ok(ack_sender) = close_req {
                        ack_sender.send(()).unwrap();
                    }
                    break;
                }
            }
        }
    }
}
