// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use super::dag_test;
use crate::{
    dag::{bootstrap::bootstrap_dag_for_test, dag_state_sync::SyncOutcome},
    network::{IncomingDAGRequest, NetworkSender, RpcResponder},
    network_interface::{ConsensusMsg, ConsensusNetworkClient, DIRECT_SEND, RPC},
    network_tests::{NetworkPlayground, TwinId},
    payload_manager::DirectMempoolPayloadManager,
    pipeline::{buffer_manager::OrderedBlocks, execution_client::DummyExecutionClient},
    test_utils::{consensus_runtime, MockPayloadManager, MockStorage},
};
use velor_channels::{velor_channel, message_queues::QueueStyle};
use velor_config::network_id::{NetworkId, PeerNetworkId};
use velor_consensus_types::common::Author;
use velor_logger::debug;
use velor_network::{
    application::interface::NetworkClient,
    peer_manager::{ConnectionRequestSender, PeerManagerRequestSender},
    protocols::{
        network::{self, Event, NetworkEvents, NewNetworkEvents, NewNetworkSender},
        wire::handshake::v1::ProtocolIdSet,
    },
    transport::ConnectionMetadata,
    ProtocolId,
};
use velor_time_service::TimeService;
use velor_types::{
    epoch_state::EpochState,
    ledger_info::generate_ledger_info_with_sig,
    validator_signer::ValidatorSigner,
    validator_verifier::{random_validator_verifier, ValidatorVerifier},
};
use claims::assert_gt;
use futures::{
    stream::{select, Select},
    StreamExt,
};
use futures_channel::mpsc::UnboundedReceiver;
use maplit::hashmap;
use std::sync::Arc;
use tokio::task::JoinHandle;

struct DagBootstrapUnit {
    nh_task_handle: JoinHandle<SyncOutcome>,
    df_task_handle: JoinHandle<()>,
    dag_rpc_tx: velor_channel::Sender<Author, IncomingDAGRequest>,
    network_events: Box<
        Select<NetworkEvents<ConsensusMsg>, velor_channels::UnboundedReceiver<Event<ConsensusMsg>>>,
    >,
}

impl DagBootstrapUnit {
    fn make(
        self_peer: Author,
        epoch: u64,
        signer: ValidatorSigner,
        storage: Arc<MockStorage>,
        network: NetworkSender,
        time_service: TimeService,
        network_events: Box<
            Select<
                NetworkEvents<ConsensusMsg>,
                velor_channels::UnboundedReceiver<Event<ConsensusMsg>>,
            >,
        >,
        all_signers: Vec<ValidatorSigner>,
    ) -> (Self, UnboundedReceiver<OrderedBlocks>) {
        let epoch_state = Arc::new(EpochState::new(epoch, storage.get_validator_set().into()));
        let ledger_info = generate_ledger_info_with_sig(&all_signers, storage.get_ledger_info());
        let dag_storage =
            dag_test::MockStorage::new_with_ledger_info(ledger_info, epoch_state.clone());

        let network = Arc::new(network);

        let payload_client = Arc::new(MockPayloadManager::new(None));
        let payload_manager = Arc::new(DirectMempoolPayloadManager::new());

        let execution_client = Arc::new(DummyExecutionClient);

        let (nh_abort_handle, df_abort_handle, dag_rpc_tx, ordered_nodes_rx) =
            bootstrap_dag_for_test(
                self_peer,
                signer,
                epoch_state,
                Arc::new(dag_storage),
                network.clone(),
                network.clone(),
                network.clone(),
                time_service,
                payload_manager,
                payload_client,
                execution_client,
            );

        (
            Self {
                nh_task_handle: nh_abort_handle,
                df_task_handle: df_abort_handle,
                dag_rpc_tx,
                network_events,
            },
            ordered_nodes_rx,
        )
    }

    async fn start(mut self) {
        loop {
            match self.network_events.next().await.unwrap() {
                Event::RpcRequest(sender, msg, protocol, response_sender) => match msg {
                    ConsensusMsg::DAGMessage(msg) => {
                        debug!("handling RPC...");
                        self.dag_rpc_tx.push(sender, IncomingDAGRequest {
                            req: msg,
                            sender,
                            responder: RpcResponder {
                                protocol,
                                response_sender,
                            },
                        })
                    },
                    _ => unreachable!("expected only DAG-related messages"),
                },
                _ => panic!("Unexpected Network Event"),
            }
            .unwrap()
        }
    }
}

fn create_network(
    playground: &mut NetworkPlayground,
    id: usize,
    author: Author,
    validators: Arc<ValidatorVerifier>,
) -> (
    NetworkSender,
    Box<
        Select<NetworkEvents<ConsensusMsg>, velor_channels::UnboundedReceiver<Event<ConsensusMsg>>>,
    >,
) {
    let (network_reqs_tx, network_reqs_rx) = velor_channel::new(QueueStyle::FIFO, 8, None);
    let (connection_reqs_tx, _) = velor_channel::new(QueueStyle::FIFO, 8, None);
    let (consensus_tx, consensus_rx) = velor_channel::new(QueueStyle::FIFO, 8, None);
    let (_conn_mgr_reqs_tx, conn_mgr_reqs_rx) = velor_channels::new_test(8);
    let network_sender = network::NetworkSender::new(
        PeerManagerRequestSender::new(network_reqs_tx),
        ConnectionRequestSender::new(connection_reqs_tx),
    );
    let network_client = NetworkClient::new(
        DIRECT_SEND.into(),
        RPC.into(),
        hashmap! {NetworkId::Validator => network_sender},
        playground.peer_protocols(),
    );
    let consensus_network_client = ConsensusNetworkClient::new(network_client);
    let network_events = NetworkEvents::new(consensus_rx, None, true);

    let (self_sender, self_receiver) = velor_channels::new_unbounded_test();
    let network = NetworkSender::new(author, consensus_network_client, self_sender, validators);

    let twin_id = TwinId { id, author };

    playground.add_node(twin_id, consensus_tx, network_reqs_rx, conn_mgr_reqs_rx);

    let all_network_events = Box::new(select(network_events, self_receiver));

    (network, all_network_events)
}

fn bootstrap_nodes(
    playground: &mut NetworkPlayground,
    signers: Vec<ValidatorSigner>,
    validators: ValidatorVerifier,
) -> (Vec<DagBootstrapUnit>, Vec<UnboundedReceiver<OrderedBlocks>>) {
    let peers_and_metadata = playground.peer_protocols();
    let validators = Arc::new(validators);
    let (nodes, ordered_node_receivers) = signers
        .iter()
        .enumerate()
        .map(|(id, signer)| {
            let peer_id = signer.author();
            let mut conn_meta = ConnectionMetadata::mock(peer_id);
            conn_meta.application_protocols = ProtocolIdSet::from_iter([
                ProtocolId::ConsensusDirectSendJson,
                ProtocolId::ConsensusDirectSendBcs,
                ProtocolId::ConsensusRpcBcs,
            ]);
            let peer_network_id = PeerNetworkId::new(NetworkId::Validator, peer_id);
            peers_and_metadata
                .insert_connection_metadata(peer_network_id, conn_meta)
                .unwrap();

            let (_, storage) = MockStorage::start_for_testing((&*validators).into());
            let (network, network_events) =
                create_network(playground, id, signer.author(), validators.clone());

            DagBootstrapUnit::make(
                signer.author(),
                1,
                signer.clone(),
                storage,
                network,
                velor_time_service::TimeService::real(),
                network_events,
                signers.clone(),
            )
        })
        .unzip();

    (nodes, ordered_node_receivers)
}

#[tokio::test]
async fn test_dag_e2e() {
    let num_nodes = 7;
    let runtime = consensus_runtime();
    let mut playground = NetworkPlayground::new(runtime.handle().clone());
    let (signers, validators) = random_validator_verifier(num_nodes, None, false);
    let (nodes, mut ordered_node_receivers) = bootstrap_nodes(&mut playground, signers, validators);
    let tasks: Vec<_> = nodes
        .into_iter()
        .map(|node| runtime.spawn(node.start()))
        .collect();
    runtime.spawn(playground.start());

    for _ in 1..10 {
        let mut all_ordered = vec![];
        for receiver in &mut ordered_node_receivers {
            let block = receiver.next().await.unwrap();
            all_ordered.push(block.ordered_blocks)
        }
        let first = all_ordered.first().unwrap();
        assert_gt!(first.len(), 0, "must order nodes");
        for a in all_ordered.iter() {
            assert_eq!(a.len(), first.len(), "length should match");
            assert_eq!(a, first);
        }
    }
    for task in tasks {
        task.abort();
        let _ = task.await;
    }
    runtime.shutdown_background();
}
