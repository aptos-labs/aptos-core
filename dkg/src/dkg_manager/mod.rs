// Copyright © Aptos Foundation
use crate::{dkg_manager::agg_trx_producer::AggTranscriptProducer, network::IncomingRpcRequest};
use aptos_channels::aptos_channel;
use aptos_types::{
    dkg::{DKGNode, DKGSessionState, DKGStartEvent, DKGTrait},
    epoch_state::EpochState,
};
use aptos_validator_transaction_pool as vtxn_pool;
use futures_channel::oneshot;
use futures_util::{FutureExt, StreamExt};
use move_core_types::account_address::AccountAddress;
use std::sync::Arc;

pub mod agg_trx_producer;

#[allow(dead_code)]
pub struct DKGManager<S: DKGTrait> {
    sk: Arc<S::PrivateParams>,
    my_addr: AccountAddress,
    epoch_state: Arc<EpochState>,
    vtxn_pool_write_cli: Arc<vtxn_pool::SingleTopicWriteClient>,
    agg_trx_producer: Arc<dyn AggTranscriptProducer<S>>,
    agg_trx_tx: Option<aptos_channel::Sender<(), DKGNode>>,
    //TODO: inner state
}

#[allow(clippy::never_loop)]
impl<S: DKGTrait> DKGManager<S> {
    pub fn new(
        sk: Arc<S::PrivateParams>,
        my_addr: AccountAddress,
        epoch_state: Arc<EpochState>,
        agg_trx_producer: Arc<dyn AggTranscriptProducer<S>>,
        vtxn_pool_write_cli: Arc<vtxn_pool::SingleTopicWriteClient>,
    ) -> Self {
        Self {
            sk,
            my_addr,
            epoch_state,
            vtxn_pool_write_cli,
            agg_trx_tx: None,
            agg_trx_producer,
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
