// Copyright © Aptos Foundation
use crate::{dkg_manager::agg_trx_producer::AggTranscriptProducer, network::IncomingRpcRequest};
use aptos_channels::aptos_channel;
use aptos_types::{
    dkg::{DKGNode, DKGPrivateParamsProvider, DKGSessionState, DKGStartEvent, DKGTrait},
    epoch_state::EpochState,
};
use aptos_validator_transaction_pool as vtxn_pool;
use futures_channel::oneshot;
use futures_util::{FutureExt, StreamExt};
use move_core_types::account_address::AccountAddress;
use std::sync::Arc;

pub mod agg_trx_producer;

#[allow(dead_code)]
pub struct DKGManager<DKG: DKGTrait, P: DKGPrivateParamsProvider<DKG>> {
    private_params_provider: P,
    my_addr: AccountAddress,
    epoch_state: Arc<EpochState>,
    vtxn_pool_write_cli: Arc<vtxn_pool::SingleTopicWriteClient>,
    agg_trx_producer: Arc<dyn AggTranscriptProducer<DKG>>,
    agg_trx_tx: Option<aptos_channel::Sender<(), DKGNode>>,
    //TODO: inner state
}

#[allow(clippy::never_loop)]
impl<DKG: DKGTrait, P: DKGPrivateParamsProvider<DKG>> DKGManager<DKG, P> {
    pub fn new(
        private_params_provider: P,
        my_addr: AccountAddress,
        epoch_state: Arc<EpochState>,
        agg_trx_producer: Arc<dyn AggTranscriptProducer<DKG>>,
        vtxn_pool_write_cli: Arc<vtxn_pool::SingleTopicWriteClient>,
    ) -> Self {
        Self {
            private_params_provider,
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
        _dkg_start_event_rx: aptos_channel::Receiver<(), DKGStartEvent>,
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
