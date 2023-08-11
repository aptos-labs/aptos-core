// Copyright © Aptos Foundation

use super::{
    dag_driver::DagDriver, dag_fetcher::FetchRequestHandler, dag_network::DAGNetworkSender,
    order_rule::OrderRule, storage::DAGStorage, types::TDAGMessage,
};
use crate::{
    dag::{
        dag_network::RpcHandler, dag_store::Dag, reliable_broadcast::NodeBroadcastHandler,
        types::DAGMessage,
    },
    network::{IncomingDAGRequest, TConsensusMsg},
    state_replication::PayloadClient,
};
use anyhow::bail;
use aptos_channels::aptos_channel;
use aptos_consensus_types::common::Author;
use aptos_infallible::RwLock;
use aptos_logger::{error, warn};
use aptos_network::protocols::network::RpcError;
use aptos_reliable_broadcast::{RBNetworkSender, ReliableBroadcast};
use aptos_time_service::TimeService;
use aptos_types::{epoch_state::EpochState, validator_signer::ValidatorSigner};
use bytes::Bytes;
use futures::StreamExt;
use std::sync::Arc;
use tokio_retry::strategy::ExponentialBackoff;

struct NetworkHandler {
    dag_rpc_rx: aptos_channel::Receiver<Author, IncomingDAGRequest>,
    node_receiver: NodeBroadcastHandler,
    dag_driver: DagDriver,
    fetch_receiver: FetchRequestHandler,
    epoch_state: Arc<EpochState>,
}

impl NetworkHandler {
    fn new(
        dag: Arc<RwLock<Dag>>,
        dag_rpc_rx: aptos_channel::Receiver<Author, IncomingDAGRequest>,
        signer: ValidatorSigner,
        epoch_state: Arc<EpochState>,
        storage: Arc<dyn DAGStorage>,
        payload_client: Arc<dyn PayloadClient>,
        _dag_network_sender: Arc<dyn DAGNetworkSender>,
        rb_network_sender: Arc<dyn RBNetworkSender<DAGMessage>>,
        time_service: TimeService,
        order_rule: OrderRule,
    ) -> Self {
        let rb = Arc::new(ReliableBroadcast::new(
            epoch_state.verifier.get_ordered_account_addresses().clone(),
            rb_network_sender,
            ExponentialBackoff::from_millis(10),
            time_service.clone(),
        ));
        Self {
            dag_rpc_rx,
            node_receiver: NodeBroadcastHandler::new(
                dag.clone(),
                signer.clone(),
                epoch_state.clone(),
                storage.clone(),
            ),
            dag_driver: DagDriver::new(
                signer.author(),
                epoch_state.clone(),
                dag.clone(),
                payload_client,
                rb,
                1,
                time_service,
                storage,
                order_rule,
            ),
            epoch_state: epoch_state.clone(),
            fetch_receiver: FetchRequestHandler::new(dag, epoch_state),
        }
    }

    async fn start(mut self) {
        self.dag_driver.try_enter_new_round();

        // TODO(ibalajiarun): clean up Reliable Broadcast storage periodically.
        while let Some(msg) = self.dag_rpc_rx.next().await {
            if let Err(e) = self.process_rpc(msg).await {
                warn!(error = ?e, "error processing rpc");
            }
        }
    }

    async fn process_rpc(&mut self, rpc_request: IncomingDAGRequest) -> anyhow::Result<()> {
        let dag_message: DAGMessage = rpc_request.req.try_into()?;

        let author = dag_message
            .author()
            .map_err(|_| anyhow::anyhow!("unexpected rpc message {:?}", dag_message))?;
        if author != rpc_request.sender {
            bail!("message author and network author mismatch");
        }

        let response: anyhow::Result<DAGMessage> = match dag_message {
            DAGMessage::NodeMsg(node) => node
                .verify(&self.epoch_state.verifier)
                .and_then(|_| self.node_receiver.process(node))
                .map(|r| r.into()),
            DAGMessage::CertifiedNodeMsg(node) => node
                .verify(&self.epoch_state.verifier)
                .and_then(|_| self.dag_driver.process(node))
                .map(|r| r.into()),
            DAGMessage::FetchRequest(request) => request
                .verify(&self.epoch_state.verifier)
                .and_then(|_| self.fetch_receiver.process(request))
                .map(|r| r.into()),
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
            .map_err(RpcError::ApplicationError);

        rpc_request
            .response_sender
            .send(response)
            .map_err(|_| anyhow::anyhow!("unable to respond to rpc"))
    }
}
