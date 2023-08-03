use super::dag_test;
use crate::{
    dag::{bootstrap::bootstrap_dag, CertifiedNode},
    network::{DAGNetworkSenderImpl, IncomingDAGRequest, NetworkSender},
    network_interface::{ConsensusMsg, ConsensusNetworkClient, DIRECT_SEND, RPC},
    network_tests::{NetworkPlayground, TwinId},
    test_utils::{consensus_runtime, MockPayloadManager, MockStorage},
    util::time_service::{ClockTimeService, TimeService},
};
use aptos_channels::{aptos_channel, message_queues::QueueStyle};
use aptos_config::network_id::{NetworkId, PeerNetworkId};
use aptos_consensus_types::common::Author;
use aptos_logger::debug;
use aptos_network::{
    application::interface::NetworkClient,
    peer_manager::{conn_notifs_channel, ConnectionRequestSender, PeerManagerRequestSender},
    protocols::{network::{self, Event, NetworkEvents, NewNetworkEvents, NewNetworkSender}, wire::handshake::v1::ProtocolIdSet}, transport::ConnectionMetadata, ProtocolId,
};
use aptos_types::{
    epoch_state::EpochState,
    validator_signer::ValidatorSigner,
    validator_verifier::{random_validator_verifier, ValidatorVerifier},
};
use claims::assert_gt;
use futures::{stream::{AbortHandle, select, Select}, StreamExt};
use futures_channel::mpsc::UnboundedReceiver;
use maplit::hashmap;
use std::sync::Arc;
use tokio::runtime::Runtime;

struct DagBootstrapUnit {
    nh_abort_handle: AbortHandle,
    df_abort_handle: AbortHandle,
    dag_rpc_tx: aptos_channel::Sender<Author, IncomingDAGRequest>,
    network_events: Box<Select<NetworkEvents<ConsensusMsg>, aptos_channels::Receiver<Event<ConsensusMsg>>>>,
}

impl DagBootstrapUnit {
    fn new(
        self_peer: Author,
        epoch: u64,
        signer: ValidatorSigner,
        storage: Arc<MockStorage>,
        network: NetworkSender,
        time_service: Arc<dyn TimeService>,
        aptos_time_service: aptos_time_service::TimeService,
        network_events: Box<Select<NetworkEvents<ConsensusMsg>, aptos_channels::Receiver<Event<ConsensusMsg>>>>,
    ) -> (Self, UnboundedReceiver<Vec<Arc<CertifiedNode>>>) {
        let epoch_state = EpochState {
            epoch,
            verifier: storage.get_validator_set().into(),
        };
        let dag_storage = dag_test::MockStorage::new();

        let network = Arc::new(DAGNetworkSenderImpl::new(Arc::new(network)));

        let payload_client = Arc::new(MockPayloadManager::new(None));

        let (nh_abort_handle, df_abort_handle, dag_rpc_tx, ordered_nodes_rx) = bootstrap_dag(
            self_peer,
            signer,
            Arc::new(epoch_state),
            storage.get_ledger_info(),
            Arc::new(dag_storage),
            network.clone(),
            network.clone(),
            time_service,
            aptos_time_service,
            payload_client,
        );

        (
            Self {
                nh_abort_handle,
                df_abort_handle,
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
                            req: msg.into(),
                            sender,
                            protocol,
                            response_sender,
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
    validators: ValidatorVerifier,
) -> (NetworkSender, Box<Select<NetworkEvents<ConsensusMsg>, aptos_channels::Receiver<Event<ConsensusMsg>>>>) {
    let (network_reqs_tx, network_reqs_rx) = aptos_channel::new(QueueStyle::FIFO, 8, None);
    let (connection_reqs_tx, _) = aptos_channel::new(QueueStyle::FIFO, 8, None);
    let (consensus_tx, consensus_rx) = aptos_channel::new(QueueStyle::FIFO, 8, None);
    let (_conn_mgr_reqs_tx, conn_mgr_reqs_rx) = aptos_channels::new_test(8);
    let (_, conn_status_rx) = conn_notifs_channel::new();
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
    let network_events = NetworkEvents::new(consensus_rx, conn_status_rx, None);

    let (self_sender, self_receiver) = aptos_channels::new_test(1000);
    let network = NetworkSender::new(author, consensus_network_client, self_sender, validators);

    let twin_id = TwinId { id, author };

    playground.add_node(twin_id, consensus_tx, network_reqs_rx, conn_mgr_reqs_rx);

    let all_network_events = Box::new(select(network_events, self_receiver));

    (network, all_network_events)
}

fn bootstrap_nodes(
    playground: &mut NetworkPlayground,
    runtime: &Runtime,
    signers: Vec<ValidatorSigner>,
    validators: ValidatorVerifier,
) -> (
    Vec<DagBootstrapUnit>,
    Vec<UnboundedReceiver<Vec<Arc<CertifiedNode>>>>,
) {
    let peers_and_metadata = playground.peer_protocols();
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

            let (_, storage) = MockStorage::start_for_testing((&validators).into());
            let (network, network_events) =
                create_network(playground, id, signer.author(), validators.clone());

            DagBootstrapUnit::new(
                signer.author(),
                1,
                signer.clone(),
                storage,
                network,
                Arc::new(ClockTimeService::new(runtime.handle().clone())),
                aptos_time_service::TimeService::real(),
                network_events,
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
    let author_indexes = validators.address_to_validator_index().clone();

    let (nodes, mut ordered_node_receivers) =
        bootstrap_nodes(&mut playground, &runtime, signers, validators);
    for node in nodes {
        runtime.spawn(node.start());
    }

    runtime.spawn(playground.start());

    let display = |node: &Arc<CertifiedNode>| {
        (
            node.metadata().round(),
            *author_indexes.get(node.metadata().author()).unwrap(),
        )
    };

    for _ in 1..10 {
        let mut all_ordered = vec![];
        for receiver in &mut ordered_node_receivers {
            let block = receiver.next().await.unwrap();
            all_ordered.push(block)
        }
        let first: Vec<_> = all_ordered
            .iter()
            .next()
            .unwrap()
            .iter()
            .map(display)
            .collect();
        assert_gt!(first.len(), 0, "must order nodes");
        debug!("Nodes: {:?}", first);
        for ordered in all_ordered.iter() {
            let a: Vec<_> = ordered.iter().map(display).collect();
            assert_gt!(a.len(), 0, "must order nodes");
            assert_eq!(a, first[..a.len()]);
        }
    }
    runtime.shutdown_background();
}
