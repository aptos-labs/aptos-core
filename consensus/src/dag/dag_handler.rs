// Copyright © Aptos Foundation

use crate::{
    dag::{
        dag_network::RpcHandler, dag_store::Dag, reliable_broadcast::NodeBroadcastHandler,
        types::DAGMessage,
    },
    network::{IncomingDAGRequest, TConsensusMsg},
};
use aptos_channels::aptos_channel;
use aptos_consensus_types::common::Author;
use aptos_infallible::RwLock;
use aptos_logger::{error, warn};
use aptos_network::protocols::network::RpcError;
use aptos_types::{epoch_state::EpochState, validator_signer::ValidatorSigner};
use bytes::Bytes;
use futures::StreamExt;
use std::sync::Arc;

struct NetworkHandler {
    dag_rpc_rx: aptos_channel::Receiver<Author, IncomingDAGRequest>,
    node_receiver: NodeBroadcastHandler,
}

impl NetworkHandler {
    fn new(
        dag: Arc<RwLock<Dag>>,
        dag_rpc_rx: aptos_channel::Receiver<Author, IncomingDAGRequest>,
        signer: ValidatorSigner,
        epoch_state: Arc<EpochState>,
    ) -> Self {
        Self {
            dag_rpc_rx,
            node_receiver: NodeBroadcastHandler::new(dag, signer, epoch_state.verifier.clone()),
        }
    }

    async fn start(mut self) {
        while let Some(msg) = self.dag_rpc_rx.next().await {
            if let Err(e) = self.process_rpc(msg).await {
                warn!(error = ?e, "error sending rpc response for request");
            }
        }
    }

    async fn process_rpc(&mut self, rpc_request: IncomingDAGRequest) -> anyhow::Result<()> {
        let dag_message: DAGMessage = TConsensusMsg::from_network_message(rpc_request.req)?;
        let response: anyhow::Result<DAGMessage> = match dag_message {
            DAGMessage::NodeMsg(node) => self.node_receiver.process(node).map(|r| r.into()),
            _ => {
                error!("unknown rpc message {:?}", dag_message);
                Err(anyhow::anyhow!("unknown rpc message"))
            },
        };

        let response = response
            .and_then(|response_msg| {
                rpc_request
                    .protocol
                    .to_bytes(&response_msg.into_network_message())
                    .map(Bytes::from)
            })
            .map_err(RpcError::Error);

        rpc_request
            .response_sender
            .send(response)
            .map_err(|_| anyhow::anyhow!("unable to process rpc"))
    }
}
