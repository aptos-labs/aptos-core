// Copyright © Aptos Foundation

use super::dag_driver::DagDriver;
use crate::{
    dag::{reliable_broadcast::ReliableBroadcast, state_machine::StateMachineLoop},
    experimental::buffer_manager::OrderedBlocks,
    network::{IncomingBlockRetrievalRequest, NetworkSender},
    network_interface::{ConsensusMsg, ConsensusNetworkClient, DIRECT_SEND, RPC},
    network_tests::{NetworkPlayground, TwinId},
    payload_manager::PayloadManager,
    persistent_liveness_storage::RecoveryData,
    round_manager::VerifiedEvent,
    test_utils::{
        consensus_runtime, timed_block_on, MockPayloadManager, MockStateComputer, MockStorage,
    },
    util::time_service::ClockTimeService,
};
use aptos_channels::{self, aptos_channel, message_queues::QueueStyle};
use aptos_config::{
    config::DagConfig,
    network_id::{NetworkId, PeerNetworkId},
};
use aptos_consensus_types::{
    common::Round, experimental::commit_decision::CommitDecision, proposal_msg::ProposalMsg,
    vote_msg::VoteMsg,
};
use aptos_crypto::HashValue;
use aptos_logger::{prelude::info, spawn_named};
use aptos_network::{
    application::interface::NetworkClient,
    peer_manager::{conn_notifs_channel, ConnectionRequestSender, PeerManagerRequestSender},
    protocols::{
        network,
        network::{Event, NetworkEvents, NewNetworkEvents, NewNetworkSender},
        wire::handshake::v1::ProtocolIdSet,
    },
    transport::ConnectionMetadata,
    ProtocolId,
};
use aptos_types::{
    epoch_state::EpochState, ledger_info::LedgerInfoWithSignatures, transaction::SignedTransaction,
    validator_signer::ValidatorSigner, validator_verifier::random_validator_verifier,
};
use futures::{channel::mpsc, stream::select, FutureExt, Stream, StreamExt};
use futures_channel::oneshot;
use maplit::hashmap;
use std::{iter::FromIterator, sync::Arc};
use tokio::runtime::Runtime;

/// Auxiliary struct that is setting up node environment for the test.
pub struct NodeSetup {
    state_machine: Option<StateMachineLoop>,
    signer: ValidatorSigner,
    pending_network_events: Vec<Event<ConsensusMsg>>,
    all_network_events: Box<dyn Stream<Item = Event<ConsensusMsg>> + Send + Unpin>,
    ordered_blocks_events: Option<mpsc::UnboundedReceiver<OrderedBlocks>>,
    mock_state_computer: Arc<MockStateComputer>,
    _state_sync_receiver: mpsc::UnboundedReceiver<Vec<SignedTransaction>>,
    id: usize,
    rb_network_msg_tx:
        aptos_channel::Sender<aptos_types::PeerId, crate::round_manager::VerifiedEvent>,
    network_msg_tx: aptos_channel::Sender<aptos_types::PeerId, crate::round_manager::VerifiedEvent>,
}

impl NodeSetup {
    fn create_nodes(
        playground: &mut NetworkPlayground,
        runtime: &Runtime,
        num_nodes: usize,
    ) -> Vec<Self> {
        let (signers, validators) = random_validator_verifier(num_nodes, None, false);

        let mut nodes = vec![];
        // pre-initialize the mapping to avoid race conditions (peer try to broadcast to someone not added yet)
        let peers_and_metadata = playground.peer_protocols();
        for signer in signers.iter().take(num_nodes) {
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
        }
        for (id, signer) in signers.iter().take(num_nodes).enumerate() {
            let (recovery_data, storage) = MockStorage::start_for_testing((&validators).into());

            nodes.push(Self::new(
                playground,
                runtime,
                signer.to_owned(),
                storage,
                id,
                recovery_data,
            ));
        }

        nodes
    }

    fn new(
        playground: &mut NetworkPlayground,
        runtime: &Runtime,
        signer: ValidatorSigner,
        storage: Arc<MockStorage>,
        id: usize,
        recovery_data: RecoveryData,
    ) -> Self {
        let epoch_state = EpochState {
            epoch: 1,
            verifier: storage.get_validator_set().into(),
        };
        let validators = epoch_state.verifier.clone();
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
        let network_events = NetworkEvents::new(consensus_rx, conn_status_rx);
        let author = signer.author();

        let (self_sender, self_receiver) = aptos_channels::new_test(1000);
        let network = NetworkSender::new(author, consensus_network_client, self_sender, validators);

        let twin_id = TwinId { id, author };

        playground.add_node(twin_id, consensus_tx, network_reqs_rx, conn_mgr_reqs_rx);

        let all_network_events = Box::new(select(network_events, self_receiver));

        let (ordered_blocks_tx, ordered_blocks_events) = mpsc::unbounded::<OrderedBlocks>();
        let (state_sync_client, _state_sync_receiver) = mpsc::unbounded();
        let mock_state_computer = Arc::new(MockStateComputer::new(
            state_sync_client,
            ordered_blocks_tx,
            Arc::clone(&storage),
        ));

        let time_service = Arc::new(ClockTimeService::new(runtime.handle().clone()));

        let payload_client = Arc::new(MockPayloadManager::new(None));

        let (rb_network_msg_tx, rb_network_msg_rx) = aptos_channel::new(QueueStyle::FIFO, 8, None);
        let (network_msg_tx, network_msg_rx) = aptos_channel::new(QueueStyle::FIFO, 8, None);

        let dag_driver = DagDriver::new(
            epoch_state.epoch,
            author.clone(),
            DagConfig::default(),
            epoch_state.verifier.clone(),
            Arc::new(signer.clone()),
            Arc::from(PayloadManager::DirectMempool),
            // mock_state_computer.clone(),
            time_service,
            HashValue::zero(),
            recovery_data.take().0 .3.ledger_info().clone(),
        );
        let rb = ReliableBroadcast::new(
            author,
            epoch_state.epoch,
            epoch_state.verifier.clone(),
            Arc::new(signer.clone()),
        );

        let state_machine = StateMachineLoop::new(
            dag_driver,
            rb,
            network_msg_rx,
            rb_network_msg_rx,
            DagConfig::default(),
            payload_client,
            network,
            mock_state_computer.clone(),
        );

        Self {
            state_machine: Some(state_machine),
            signer,
            pending_network_events: Vec::new(),
            all_network_events,
            ordered_blocks_events: Some(ordered_blocks_events),
            mock_state_computer,
            _state_sync_receiver,
            id,
            rb_network_msg_tx,
            network_msg_tx,
        }
    }

    pub fn identity_desc(&self) -> String {
        format!("{} [{}]", self.id, self.signer.author())
    }

    async fn start(&mut self) {
        let (_close_tx, close_rx) = oneshot::channel();
        spawn_named!(
            "dag-driver",
            self.state_machine.take().unwrap().run(close_rx)
        );
        loop {
            match self.next_network_message().await {
                ConsensusMsg::NodeMsg(msg) => self
                    .rb_network_msg_tx
                    .push(msg.source(), VerifiedEvent::NodeMsg(msg)),
                ConsensusMsg::SignedNodeDigestMsg(msg) => self
                    .rb_network_msg_tx
                    .push(msg.peer_id(), VerifiedEvent::SignedNodeDigestMsg(msg)),
                ConsensusMsg::CertifiedNodeAckMsg(msg) => self
                    .rb_network_msg_tx
                    .push(msg.peer_id(), VerifiedEvent::CertifiedNodeAckMsg(msg)),
                ConsensusMsg::CertifiedNodeMsg(msg, ack) => self
                    .network_msg_tx
                    .push(msg.source(), VerifiedEvent::CertifiedNodeMsg(msg, ack)),
                ConsensusMsg::CertifiedNodeRequestMsg(msg) => self
                    .network_msg_tx
                    .push(msg.source(), VerifiedEvent::CertifiedNodeRequestMsg(msg)),
                _ => unreachable!("expected only DAG-related messages"),
            }
            .unwrap();
        }
    }

    fn poll_next_network_event(&mut self) -> Option<Event<ConsensusMsg>> {
        if !self.pending_network_events.is_empty() {
            Some(self.pending_network_events.remove(0))
        } else {
            self.all_network_events
                .next()
                .now_or_never()
                .map(|v| v.unwrap())
        }
    }

    pub async fn next_network_event(&mut self) -> Event<ConsensusMsg> {
        if !self.pending_network_events.is_empty() {
            self.pending_network_events.remove(0)
        } else {
            self.all_network_events.next().await.unwrap()
        }
    }

    pub async fn next_network_message(&mut self) -> ConsensusMsg {
        match self.next_network_event().await {
            Event::Message(_, msg) => msg,
            Event::RpcRequest(_, msg, _, _) => panic!(
                "Unexpected event, got RpcRequest, expected Message: {:?} on node {}",
                msg,
                self.identity_desc()
            ),
            _ => panic!("Unexpected Network Event"),
        }
    }
}

#[tokio::test]
async fn basic_dag_driver_test() {
    let runtime = consensus_runtime();
    println!("Created runtime. Starting nodes...");

    let mut playground = NetworkPlayground::new(runtime.handle().clone());
    let nodes = NodeSetup::create_nodes(&mut playground, &runtime, 7);
    runtime.spawn(playground.start());
    let mut receivers = Vec::new();
    for mut node in nodes {
        receivers.push(node.ordered_blocks_events.take().unwrap());
        runtime.spawn(async move {
            node.start().await;
        });
    }

    println!("Started nodes. Waiting for blocks...");
    for _ in 1..10 {
        let mut ref_block: Option<OrderedBlocks> = None;
        for receiver in receivers.iter_mut() {
            let block = receiver.next().await.unwrap();
            // println!("received block: {:?}", block.ordered_blocks[0].payload().unwrap());

            if ref_block.is_none() {
                ref_block = Some(block);
            } else {
                assert_eq!(
                    ref_block.as_ref().unwrap().ordered_blocks[0]
                        .payload()
                        .unwrap(),
                    block.ordered_blocks[0].payload().unwrap()
                );
            }
        }
    }
    runtime.shutdown_background();
}
