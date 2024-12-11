// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    block_storage::{pending_blocks::PendingBlocks, BlockReader, BlockStore},
    counters,
    liveness::{
        proposal_generator::{
            ChainHealthBackoffConfig, PipelineBackpressureConfig, ProposalGenerator,
        },
        proposer_election::ProposerElection,
        rotating_proposer_election::RotatingProposer,
        round_state::{ExponentialTimeInterval, RoundState},
    },
    metrics_safety_rules::MetricsSafetyRules,
    network::{IncomingBlockRetrievalRequest, NetworkSender},
    network_interface::{CommitMessage, ConsensusMsg, ConsensusNetworkClient, DIRECT_SEND, RPC},
    network_tests::{NetworkPlayground, TwinId},
    payload_manager::DirectMempoolPayloadManager,
    persistent_liveness_storage::RecoveryData,
    pipeline::buffer_manager::OrderedBlocks,
    round_manager::RoundManager,
    test_utils::{
        consensus_runtime, create_vec_signed_transactions,
        mock_execution_client::MockExecutionClient, timed_block_on, MockOptQSPayloadProvider,
        MockPastProposalStatusTracker, MockPayloadManager, MockStorage, TreeInserter,
    },
    util::time_service::{ClockTimeService, TimeService},
};
use aptos_channels::{self, aptos_channel, message_queues::QueueStyle};
use aptos_config::{
    config::ConsensusConfig,
    network_id::{NetworkId, PeerNetworkId},
};
use aptos_consensus_types::{
    block::{
        block_test_utils::{certificate_for_genesis, gen_test_certificate},
        Block,
    },
    block_retrieval::{BlockRetrievalRequest, BlockRetrievalStatus},
    common::{Author, Payload, Round},
    order_vote_msg::OrderVoteMsg,
    pipeline::commit_decision::CommitDecision,
    proposal_msg::ProposalMsg,
    round_timeout::RoundTimeoutMsg,
    sync_info::SyncInfo,
    timeout_2chain::{TwoChainTimeout, TwoChainTimeoutWithPartialSignatures},
    utils::PayloadTxnsSize,
    vote_msg::VoteMsg,
};
use aptos_crypto::HashValue;
use aptos_infallible::Mutex;
use aptos_logger::prelude::info;
use aptos_network::{
    application::interface::NetworkClient,
    peer_manager::{ConnectionRequestSender, PeerManagerRequestSender},
    protocols::{
        network,
        network::{Event, NetworkEvents, NewNetworkEvents, NewNetworkSender},
        wire::handshake::v1::ProtocolIdSet,
    },
    transport::ConnectionMetadata,
    ProtocolId,
};
use aptos_safety_rules::{PersistentSafetyStorage, SafetyRulesManager};
use aptos_secure_storage::Storage;
use aptos_types::{
    epoch_state::EpochState,
    jwks::QuorumCertifiedUpdate,
    ledger_info::LedgerInfo,
    on_chain_config::{
        ConsensusAlgorithmConfig, ConsensusConfigV1, OnChainConsensusConfig,
        OnChainJWKConsensusConfig, OnChainRandomnessConfig, ValidatorTxnConfig,
    },
    transaction::SignedTransaction,
    validator_signer::ValidatorSigner,
    validator_txn::ValidatorTransaction,
    validator_verifier::{generate_validator_verifier, random_validator_verifier},
    waypoint::Waypoint,
};
use futures::{
    channel::{mpsc, oneshot},
    executor::block_on,
    stream::select,
    FutureExt, Stream, StreamExt,
};
use maplit::hashmap;
use std::{
    collections::VecDeque,
    iter::FromIterator,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::{
    runtime::{Handle, Runtime},
    task::JoinHandle,
    time::timeout,
};

/// Auxiliary struct that is setting up node environment for the test.
pub struct NodeSetup {
    block_store: Arc<BlockStore>,
    round_manager: RoundManager,
    storage: Arc<MockStorage>,
    signer: ValidatorSigner,
    proposers: Vec<Author>,
    safety_rules_manager: SafetyRulesManager,
    pending_network_events: Vec<Event<ConsensusMsg>>,
    all_network_events: Box<dyn Stream<Item = Event<ConsensusMsg>> + Send + Unpin>,
    ordered_blocks_events: mpsc::UnboundedReceiver<OrderedBlocks>,
    mock_execution_client: Arc<MockExecutionClient>,
    _state_sync_receiver: mpsc::UnboundedReceiver<Vec<SignedTransaction>>,
    id: usize,
    onchain_consensus_config: OnChainConsensusConfig,
    local_consensus_config: ConsensusConfig,
    onchain_randomness_config: OnChainRandomnessConfig,
    onchain_jwk_consensus_config: OnChainJWKConsensusConfig,
    vote_queue: VecDeque<VoteMsg>,
    order_vote_queue: VecDeque<OrderVoteMsg>,
    proposal_queue: VecDeque<ProposalMsg>,
    round_timeout_queue: VecDeque<RoundTimeoutMsg>,
    commit_decision_queue: VecDeque<CommitDecision>,
}

impl NodeSetup {
    fn create_round_state(time_service: Arc<dyn TimeService>) -> RoundState {
        let base_timeout = Duration::new(60, 0);
        let time_interval = Box::new(ExponentialTimeInterval::fixed(base_timeout));
        let (round_timeout_sender, _) = aptos_channels::new_test(1_024);
        RoundState::new(time_interval, time_service, round_timeout_sender)
    }

    fn create_proposer_election(proposers: Vec<Author>) -> Arc<dyn ProposerElection + Send + Sync> {
        Arc::new(RotatingProposer::new(proposers, 1))
    }

    fn create_nodes(
        playground: &mut NetworkPlayground,
        executor: Handle,
        num_nodes: usize,
        proposer_indices: Option<Vec<usize>>,
        onchain_consensus_config: Option<OnChainConsensusConfig>,
        local_consensus_config: Option<ConsensusConfig>,
        onchain_randomness_config: Option<OnChainRandomnessConfig>,
        onchain_jwk_consensus_config: Option<OnChainJWKConsensusConfig>,
    ) -> Vec<Self> {
        let mut onchain_consensus_config = onchain_consensus_config.unwrap_or_default();
        // With order votes feature, the validators additionally send order votes.
        // next_proposal and next_vote functions could potentially break because of it.
        if let OnChainConsensusConfig::V3 {
            alg:
                ConsensusAlgorithmConfig::JolteonV2 {
                    main: _,
                    quorum_store_enabled: _,
                    order_vote_enabled,
                },
            vtxn: _,
        } = &mut onchain_consensus_config
        {
            *order_vote_enabled = false;
        }
        let onchain_randomness_config =
            onchain_randomness_config.unwrap_or_else(OnChainRandomnessConfig::default_if_missing);
        let onchain_jwk_consensus_config = onchain_jwk_consensus_config
            .unwrap_or_else(OnChainJWKConsensusConfig::default_if_missing);
        let local_consensus_config = local_consensus_config.unwrap_or_default();
        let (signers, validators) = random_validator_verifier(num_nodes, None, false);
        let proposers = proposer_indices
            .unwrap_or_else(|| vec![0])
            .iter()
            .map(|i| signers[*i].author())
            .collect::<Vec<_>>();
        let validator_set = (&validators).into();
        let waypoint =
            Waypoint::new_epoch_boundary(&LedgerInfo::mock_genesis(Some(validator_set))).unwrap();

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
            let (initial_data, storage) = MockStorage::start_for_testing((&validators).into());

            let safety_storage = PersistentSafetyStorage::initialize(
                Storage::from(aptos_secure_storage::InMemoryStorage::new()),
                signer.author(),
                signer.private_key().clone(),
                waypoint,
                true,
            );
            let safety_rules_manager = SafetyRulesManager::new_local(safety_storage);

            nodes.push(Self::new(
                playground,
                executor.clone(),
                signer.to_owned(),
                proposers.clone(),
                storage,
                initial_data,
                safety_rules_manager,
                id,
                onchain_consensus_config.clone(),
                local_consensus_config.clone(),
                onchain_randomness_config.clone(),
                onchain_jwk_consensus_config.clone(),
            ));
        }
        nodes
    }

    fn new(
        playground: &mut NetworkPlayground,
        executor: Handle,
        signer: ValidatorSigner,
        proposers: Vec<Author>,
        storage: Arc<MockStorage>,
        initial_data: RecoveryData,
        safety_rules_manager: SafetyRulesManager,
        id: usize,
        onchain_consensus_config: OnChainConsensusConfig,
        local_consensus_config: ConsensusConfig,
        onchain_randomness_config: OnChainRandomnessConfig,
        onchain_jwk_consensus_config: OnChainJWKConsensusConfig,
    ) -> Self {
        let _entered_runtime = executor.enter();
        let epoch_state = Arc::new(EpochState::new(1, storage.get_validator_set().into()));
        let validators = epoch_state.verifier.clone();
        let (network_reqs_tx, network_reqs_rx) = aptos_channel::new(QueueStyle::FIFO, 8, None);
        let (connection_reqs_tx, _) = aptos_channel::new(QueueStyle::FIFO, 8, None);
        let (consensus_tx, consensus_rx) = aptos_channel::new(QueueStyle::FIFO, 8, None);
        let (_conn_mgr_reqs_tx, conn_mgr_reqs_rx) = aptos_channels::new_test(8);
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
        let author = signer.author();

        let twin_id = TwinId { id, author };

        playground.add_node(twin_id, consensus_tx, network_reqs_rx, conn_mgr_reqs_rx);

        let (self_sender, self_receiver) = aptos_channels::new_unbounded_test();
        let network = Arc::new(NetworkSender::new(
            author,
            consensus_network_client,
            self_sender,
            validators,
        ));

        let all_network_events = Box::new(select(network_events, self_receiver));

        let last_vote_sent = initial_data.last_vote();
        let (ordered_blocks_tx, ordered_blocks_events) = mpsc::unbounded::<OrderedBlocks>();
        let (state_sync_client, _state_sync_receiver) = mpsc::unbounded();
        let mock_execution_client = Arc::new(MockExecutionClient::new(
            state_sync_client.clone(),
            ordered_blocks_tx.clone(),
            Arc::clone(&storage),
        ));
        let time_service = Arc::new(ClockTimeService::new(executor));

        let block_store = Arc::new(BlockStore::new(
            storage.clone(),
            initial_data,
            mock_execution_client.clone(),
            10, // max pruned blocks in mem
            time_service.clone(),
            10,
            Arc::from(DirectMempoolPayloadManager::new()),
            false,
            Arc::new(Mutex::new(PendingBlocks::new())),
            None,
        ));

        let proposer_election = Self::create_proposer_election(proposers.clone());
        let proposal_generator = ProposalGenerator::new(
            author,
            block_store.clone(),
            Arc::new(MockPayloadManager::new(None)),
            time_service.clone(),
            Duration::ZERO,
            PayloadTxnsSize::new(20, 1000),
            10,
            PayloadTxnsSize::new(5, 500),
            10,
            1,
            PipelineBackpressureConfig::new_no_backoff(),
            ChainHealthBackoffConfig::new_no_backoff(),
            false,
            onchain_consensus_config.effective_validator_txn_config(),
            true,
            Arc::new(MockOptQSPayloadProvider {}),
        );

        let round_state = Self::create_round_state(time_service);
        let mut safety_rules =
            MetricsSafetyRules::new(safety_rules_manager.client(), storage.clone());
        safety_rules.perform_initialize().unwrap();

        let (round_manager_tx, _) = aptos_channel::new(QueueStyle::LIFO, 1, None);

        let local_config = local_consensus_config.clone();

        let mut round_manager = RoundManager::new(
            epoch_state,
            Arc::clone(&block_store),
            round_state,
            proposer_election,
            proposal_generator,
            Arc::new(Mutex::new(safety_rules)),
            network,
            storage.clone(),
            onchain_consensus_config.clone(),
            round_manager_tx,
            local_config,
            onchain_randomness_config.clone(),
            onchain_jwk_consensus_config.clone(),
            None,
            Arc::new(MockPastProposalStatusTracker {}),
        );
        block_on(round_manager.init(last_vote_sent));
        Self {
            block_store,
            round_manager,
            storage,
            signer,
            proposers,
            safety_rules_manager,
            pending_network_events: Vec::new(),
            all_network_events,
            ordered_blocks_events,
            mock_execution_client,
            _state_sync_receiver,
            id,
            onchain_consensus_config,
            local_consensus_config,
            onchain_randomness_config,
            onchain_jwk_consensus_config,
            vote_queue: VecDeque::new(),
            order_vote_queue: VecDeque::new(),
            proposal_queue: VecDeque::new(),
            round_timeout_queue: VecDeque::new(),
            commit_decision_queue: VecDeque::new(),
        }
    }

    pub fn restart(self, playground: &mut NetworkPlayground, executor: Handle) -> Self {
        let recover_data = self
            .storage
            .try_start(self.onchain_consensus_config.order_vote_enabled())
            .unwrap_or_else(|e| panic!("fail to restart due to: {}", e));
        Self::new(
            playground,
            executor,
            self.signer,
            self.proposers,
            self.storage,
            recover_data,
            self.safety_rules_manager,
            self.id,
            self.onchain_consensus_config.clone(),
            self.local_consensus_config.clone(),
            self.onchain_randomness_config.clone(),
            self.onchain_jwk_consensus_config.clone(),
        )
    }

    pub fn identity_desc(&self) -> String {
        format!("{} [{}]", self.id, self.signer.author())
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

    pub async fn next_network_message(&mut self) {
        let consensus_msg = match self.next_network_event().await {
            Event::Message(_, msg) => msg,
            Event::RpcRequest(_, msg, _, _) if matches!(msg, ConsensusMsg::CommitMessage(_)) => msg,
            Event::RpcRequest(_, msg, _, _) => {
                panic!(
                    "Unexpected event, got RpcRequest, expected Message: {:?} on node {}",
                    msg,
                    self.identity_desc()
                )
            },
        };

        match consensus_msg {
            ConsensusMsg::ProposalMsg(proposal) => {
                self.proposal_queue.push_back(*proposal);
            },
            ConsensusMsg::VoteMsg(vote) => {
                self.vote_queue.push_back(*vote);
            },
            ConsensusMsg::OrderVoteMsg(order_vote) => {
                self.order_vote_queue.push_back(*order_vote);
            },
            ConsensusMsg::RoundTimeoutMsg(round_timeout) => {
                self.round_timeout_queue.push_back(*round_timeout);
            },
            ConsensusMsg::CommitDecisionMsg(commit_decision) => {
                self.commit_decision_queue.push_back(*commit_decision);
            },
            ConsensusMsg::CommitMessage(d) if matches!(*d, CommitMessage::Decision(_)) => {
                match *d {
                    CommitMessage::Decision(commit_decision) => {
                        self.commit_decision_queue.push_back(commit_decision);
                    },
                    _ => unreachable!(),
                }
            },
            msg => panic!(
                "Unexpected Consensus Message: {:?} on node {}",
                msg,
                self.identity_desc()
            ),
        }
    }

    pub fn no_next_msg(&mut self) {
        match self.poll_next_network_event() {
            Some(Event::RpcRequest(_, msg, _, _)) | Some(Event::Message(_, msg)) => panic!(
                "Unexpected Consensus Message: {:?} on node {}",
                msg,
                self.identity_desc()
            ),
            None => {},
        }
    }

    pub async fn next_proposal(&mut self) -> ProposalMsg {
        while self.proposal_queue.is_empty() {
            self.next_network_message().await;
        }
        self.proposal_queue.pop_front().unwrap()
    }

    pub async fn next_vote(&mut self) -> VoteMsg {
        while self.vote_queue.is_empty() {
            self.next_network_message().await;
        }
        self.vote_queue.pop_front().unwrap()
    }

    pub async fn next_order_vote(&mut self) -> OrderVoteMsg {
        while self.order_vote_queue.is_empty() {
            self.next_network_message().await;
        }
        self.order_vote_queue.pop_front().unwrap()
    }

    pub async fn next_timeout(&mut self) -> RoundTimeoutMsg {
        while self.round_timeout_queue.is_empty() {
            self.next_network_message().await;
        }
        self.round_timeout_queue.pop_front().unwrap()
    }

    pub async fn next_commit_decision(&mut self) -> CommitDecision {
        while self.commit_decision_queue.is_empty() {
            self.next_network_message().await;
        }
        self.commit_decision_queue.pop_front().unwrap()
    }

    pub async fn poll_block_retreival(&mut self) -> Option<IncomingBlockRetrievalRequest> {
        match self.poll_next_network_event() {
            Some(Event::RpcRequest(_, msg, protocol, response_sender)) => match msg {
                ConsensusMsg::BlockRetrievalRequest(v) => Some(IncomingBlockRetrievalRequest {
                    req: *v,
                    protocol,
                    response_sender,
                }),
                msg => panic!(
                    "Unexpected Consensus Message: {:?} on node {}",
                    msg,
                    self.identity_desc()
                ),
            },
            Some(Event::Message(_, msg)) => panic!(
                "Unexpected Consensus Message: {:?} on node {}",
                msg,
                self.identity_desc()
            ),
            None => None,
        }
    }

    pub fn no_next_ordered(&mut self) {
        if self.ordered_blocks_events.next().now_or_never().is_some() {
            panic!("Unexpected Ordered Blocks Event");
        }
    }

    pub async fn commit_next_ordered(&mut self, expected_rounds: &[Round]) {
        info!(
            "Starting commit_next_ordered to wait for {:?} on node {:?}",
            expected_rounds,
            self.identity_desc()
        );
        let ordered_blocks = self.ordered_blocks_events.next().await.unwrap();
        let rounds = ordered_blocks
            .ordered_blocks
            .iter()
            .map(|b| b.round())
            .collect::<Vec<_>>();
        assert_eq!(&rounds, expected_rounds);
        self.mock_execution_client
            .commit_to_storage(ordered_blocks)
            .await
            .unwrap();
    }
}

fn config_with_round_timeout_msg_disabled() -> ConsensusConfig {
    // Disable RoundTimeoutMsg to unless expliclity enabled.
    ConsensusConfig {
        enable_round_timeout_msg: false,
        ..Default::default()
    }
}

fn start_replying_to_block_retreival(nodes: Vec<NodeSetup>) -> ReplyingRPCHandle {
    let done = Arc::new(AtomicBool::new(false));
    let mut handles = Vec::new();
    for mut node in nodes.into_iter() {
        let done_clone = done.clone();
        handles.push(tokio::spawn(async move {
            while !done_clone.load(Ordering::Relaxed) {
                info!("Asking for RPC request on {:?}", node.identity_desc());
                let maybe_request = node.poll_block_retreival().await;
                if let Some(request) = maybe_request {
                    info!(
                        "RPC request received: {:?} on {:?}",
                        request,
                        node.identity_desc()
                    );
                    node.block_store
                        .process_block_retrieval(request)
                        .await
                        .unwrap();
                } else {
                    tokio::time::sleep(Duration::from_millis(50)).await;
                }
            }
            node
        }));
    }
    ReplyingRPCHandle { handles, done }
}

struct ReplyingRPCHandle {
    handles: Vec<JoinHandle<NodeSetup>>,
    done: Arc<AtomicBool>,
}

impl ReplyingRPCHandle {
    async fn join(self) -> Vec<NodeSetup> {
        self.done.store(true, Ordering::Relaxed);
        let mut result = Vec::new();
        for handle in self.handles.into_iter() {
            result.push(handle.await.unwrap());
        }
        info!(
            "joined nodes in order: {:?}",
            result.iter().map(|v| v.id).collect::<Vec<_>>()
        );
        result
    }
}

fn process_and_vote_on_proposal(
    runtime: &Runtime,
    nodes: &mut [NodeSetup],
    next_proposer: usize,
    down_nodes: &[usize],
    process_votes: bool,
    apply_commit_prev_proposer: Option<usize>,
    apply_commit_on_votes: bool,
    expected_round: u64,
    expected_qc_ordered_round: u64,
    expected_qc_committed_round: u64,
) {
    info!(
        "Called {} with current {} and apply commit prev {:?}",
        expected_round, next_proposer, apply_commit_prev_proposer
    );
    let mut num_votes = 0;

    for node in nodes.iter_mut() {
        info!("Waiting on next_proposal on node {}", node.identity_desc());
        if down_nodes.contains(&node.id) {
            // Drop the proposal on down nodes
            timed_block_on(runtime, node.next_proposal());
            info!("Dropping proposal on down node {}", node.identity_desc());
        } else {
            // Proccess proposal on other nodes
            let proposal_msg = timed_block_on(runtime, node.next_proposal());
            info!("Processing proposal on {}", node.identity_desc());

            assert_eq!(proposal_msg.proposal().round(), expected_round);
            assert_eq!(
                proposal_msg.sync_info().highest_ordered_round(),
                expected_qc_ordered_round
            );
            assert_eq!(
                proposal_msg.sync_info().highest_commit_round(),
                expected_qc_committed_round
            );

            timed_block_on(
                runtime,
                node.round_manager.process_proposal_msg(proposal_msg),
            )
            .unwrap();
            info!("Finish process proposal on {}", node.identity_desc());
            num_votes += 1;
        }
    }

    for node in nodes.iter_mut() {
        info!(
            "Fetching {} votes in round {} on node {}",
            num_votes,
            expected_round,
            node.identity_desc()
        );
        if down_nodes.contains(&node.id) {
            // Drop the votes on down nodes
            info!("Dropping votes on down node {}", node.identity_desc());
            for _ in 0..num_votes {
                timed_block_on(runtime, node.next_vote());
            }
        } else {
            let mut votes = Vec::new();
            for _ in 0..num_votes {
                votes.push(timed_block_on(runtime, node.next_vote()));
            }

            info!("Processing votes on node {}", node.identity_desc());
            if process_votes {
                for vote_msg in votes {
                    timed_block_on(runtime, node.round_manager.process_vote_msg(vote_msg)).unwrap();
                }
                if apply_commit_prev_proposer.is_some()
                    && expected_round > 1
                    && apply_commit_on_votes
                {
                    info!(
                        "Applying next commit {} on proposer node {}",
                        expected_round - 2,
                        node.identity_desc()
                    );
                    timed_block_on(runtime, node.commit_next_ordered(&[expected_round - 1]));
                }
            }
        }
    }
}

#[test]
fn new_round_on_quorum_cert() {
    let runtime = consensus_runtime();
    let mut playground = NetworkPlayground::new(runtime.handle().clone());
    let mut nodes = NodeSetup::create_nodes(
        &mut playground,
        runtime.handle().clone(),
        1,
        None,
        None,
        None,
        None,
        None,
    );
    let node = &mut nodes[0];
    let genesis = node.block_store.ordered_root();
    timed_block_on(&runtime, async {
        // round 1 should start
        let proposal_msg = node.next_proposal().await;
        assert_eq!(
            proposal_msg.proposal().quorum_cert().certified_block().id(),
            genesis.id()
        );
        let b1_id = proposal_msg.proposal().id();
        assert_eq!(proposal_msg.proposer(), node.signer.author());

        node.round_manager
            .process_proposal_msg(proposal_msg)
            .await
            .unwrap();
        let vote_msg = node.next_vote().await;
        // Adding vote to form a QC
        node.round_manager.process_vote_msg(vote_msg).await.unwrap();

        // round 2 should start
        let proposal_msg = node.next_proposal().await;
        let proposal = proposal_msg.proposal();
        assert_eq!(proposal.round(), 2);
        assert_eq!(proposal.parent_id(), b1_id);
        assert_eq!(proposal.quorum_cert().certified_block().id(), b1_id);
    });
}

#[test]
/// If the proposal is valid, a vote should be sent
fn vote_on_successful_proposal() {
    let runtime = consensus_runtime();
    let mut playground = NetworkPlayground::new(runtime.handle().clone());
    // In order to observe the votes we're going to check proposal processing on the non-proposer
    // node (which will send the votes to the proposer).
    let mut nodes = NodeSetup::create_nodes(
        &mut playground,
        runtime.handle().clone(),
        1,
        None,
        None,
        None,
        None,
        None,
    );
    let node = &mut nodes[0];

    let genesis_qc = certificate_for_genesis();
    timed_block_on(&runtime, async {
        // Start round 1 and clear the message queue
        node.next_proposal().await;

        let proposal = Block::new_proposal(
            Payload::empty(false, true),
            1,
            1,
            genesis_qc.clone(),
            &node.signer,
            Vec::new(),
        )
        .unwrap();
        let proposal_id = proposal.id();
        node.round_manager.process_proposal(proposal).await.unwrap();
        let vote_msg = node.next_vote().await;
        assert_eq!(vote_msg.vote().author(), node.signer.author());
        assert_eq!(vote_msg.vote().vote_data().proposed().id(), proposal_id);
        let consensus_state = node.round_manager.consensus_state();
        assert_eq!(consensus_state.epoch(), 1);
        assert_eq!(consensus_state.last_voted_round(), 1);
        assert_eq!(consensus_state.preferred_round(), 0);
        assert!(consensus_state.in_validator_set());
    });
}

#[test]
/// In back pressure mode, verify that the proposals are processed after we get out of back pressure.
fn delay_proposal_processing_in_sync_only() {
    let runtime = consensus_runtime();
    let mut playground = NetworkPlayground::new(runtime.handle().clone());
    // In order to observe the votes we're going to check proposal processing on the non-proposer
    // node (which will send the votes to the proposer).
    let mut nodes = NodeSetup::create_nodes(
        &mut playground,
        runtime.handle().clone(),
        1,
        None,
        None,
        None,
        None,
        None,
    );
    let node = &mut nodes[0];

    let genesis_qc = certificate_for_genesis();
    timed_block_on(&runtime, async {
        // Start round 1 and clear the message queue
        node.next_proposal().await;

        // Set sync only to true so that new proposal processing is delayed.
        node.round_manager
            .block_store
            .set_back_pressure_for_test(true);
        let proposal = Block::new_proposal(
            Payload::empty(false, true),
            1,
            1,
            genesis_qc.clone(),
            &node.signer,
            Vec::new(),
        )
        .unwrap();
        let proposal_id = proposal.id();
        node.round_manager
            .process_proposal(proposal.clone())
            .await
            .unwrap();

        // Wait for some time to ensure that the proposal was not processed
        timeout(Duration::from_millis(200), node.next_vote())
            .await
            .unwrap_err();

        // Clear the sync only mode and process verified proposal and ensure it is processed now
        node.round_manager
            .block_store
            .set_back_pressure_for_test(false);

        node.round_manager
            .process_verified_proposal(proposal)
            .await
            .unwrap();

        let vote_msg = node.next_vote().await;
        assert_eq!(vote_msg.vote().author(), node.signer.author());
        assert_eq!(vote_msg.vote().vote_data().proposed().id(), proposal_id);
        let consensus_state = node.round_manager.consensus_state();
        assert_eq!(consensus_state.epoch(), 1);
        assert_eq!(consensus_state.last_voted_round(), 1);
        assert_eq!(consensus_state.preferred_round(), 0);
        assert!(consensus_state.in_validator_set());
    });
}

#[test]
/// If the proposal does not pass voting rules,
/// No votes are sent, but the block is still added to the block tree.
fn no_vote_on_old_proposal() {
    let runtime = consensus_runtime();
    let mut playground = NetworkPlayground::new(runtime.handle().clone());
    // In order to observe the votes we're going to check proposal processing on the non-proposer
    // node (which will send the votes to the proposer).
    let mut nodes = NodeSetup::create_nodes(
        &mut playground,
        runtime.handle().clone(),
        1,
        None,
        None,
        None,
        None,
        None,
    );
    let node = &mut nodes[0];
    let genesis_qc = certificate_for_genesis();
    let new_block = Block::new_proposal(
        Payload::empty(false, true),
        1,
        1,
        genesis_qc.clone(),
        &node.signer,
        Vec::new(),
    )
    .unwrap();
    let new_block_id = new_block.id();
    let old_block = Block::new_proposal(
        Payload::empty(false, true),
        1,
        2,
        genesis_qc,
        &node.signer,
        Vec::new(),
    )
    .unwrap();
    timed_block_on(&runtime, async {
        // clear the message queue
        node.next_proposal().await;

        node.round_manager
            .process_proposal(new_block)
            .await
            .unwrap();
        node.round_manager
            .process_proposal(old_block)
            .await
            .unwrap_err();
        let vote_msg = node.next_vote().await;
        assert_eq!(vote_msg.vote().vote_data().proposed().id(), new_block_id);
    });
}

#[test]
/// We don't vote for proposals that 'skips' rounds
/// After that when we then receive proposal for correct round, we vote for it
/// Basically it checks that adversary can not send proposal and skip rounds violating round_state
/// rules
fn no_vote_on_mismatch_round() {
    let runtime = consensus_runtime();
    let mut playground = NetworkPlayground::new(runtime.handle().clone());
    // In order to observe the votes we're going to check proposal processing on the non-proposer
    // node (which will send the votes to the proposer).
    let mut node = NodeSetup::create_nodes(
        &mut playground,
        runtime.handle().clone(),
        1,
        None,
        None,
        None,
        None,
        None,
    )
    .pop()
    .unwrap();
    let genesis_qc = certificate_for_genesis();
    let correct_block = Block::new_proposal(
        Payload::empty(false, true),
        1,
        1,
        genesis_qc.clone(),
        &node.signer,
        Vec::new(),
    )
    .unwrap();
    let block_skip_round = Block::new_proposal(
        Payload::empty(false, true),
        2,
        2,
        genesis_qc.clone(),
        &node.signer,
        Vec::new(),
    )
    .unwrap();
    timed_block_on(&runtime, async {
        let bad_proposal = ProposalMsg::new(
            block_skip_round,
            SyncInfo::new(
                genesis_qc.clone(),
                genesis_qc.into_wrapped_ledger_info(),
                None,
            ),
        );
        assert!(node
            .round_manager
            .process_proposal_msg(bad_proposal)
            .await
            .is_err());
        let good_proposal = ProposalMsg::new(
            correct_block.clone(),
            SyncInfo::new(
                genesis_qc.clone(),
                genesis_qc.into_wrapped_ledger_info(),
                None,
            ),
        );
        node.round_manager
            .process_proposal_msg(good_proposal)
            .await
            .unwrap();
    });
}

#[test]
/// Ensure that after the vote messages are broadcasted upon timeout, the receivers
/// have the highest quorum certificate (carried by the SyncInfo of the vote message)
fn sync_info_carried_on_timeout_vote() {
    let runtime = consensus_runtime();
    let mut playground = NetworkPlayground::new(runtime.handle().clone());
    let mut nodes = NodeSetup::create_nodes(
        &mut playground,
        runtime.handle().clone(),
        1,
        None,
        None,
        Some(config_with_round_timeout_msg_disabled()),
        None,
        None,
    );
    let mut node = nodes.pop().unwrap();

    timed_block_on(&runtime, async {
        let proposal_msg = node.next_proposal().await;
        let block_0 = proposal_msg.proposal().clone();
        node.round_manager
            .process_proposal_msg(proposal_msg)
            .await
            .unwrap();
        node.next_vote().await;
        let parent_block_info = block_0.quorum_cert().certified_block();
        // Populate block_0 and a quorum certificate for block_0 on non_proposer
        let block_0_quorum_cert = gen_test_certificate(
            &[node.signer.clone()],
            // Follow MockStateComputer implementation
            block_0.gen_block_info(
                parent_block_info.executed_state_id(),
                parent_block_info.version(),
                parent_block_info.next_epoch_state().cloned(),
            ),
            parent_block_info.clone(),
            None,
        );
        node.block_store
            .insert_single_quorum_cert(block_0_quorum_cert.clone())
            .unwrap();

        node.round_manager.round_state.process_certificates(
            SyncInfo::new(
                block_0_quorum_cert.clone(),
                block_0_quorum_cert.into_wrapped_ledger_info(),
                None,
            ),
            &generate_validator_verifier(&[node.signer.clone()]),
        );
        node.round_manager
            .process_local_timeout(2)
            .await
            .unwrap_err();
        let vote_msg_on_timeout = node.next_vote().await;
        assert!(vote_msg_on_timeout.vote().is_timeout());
        assert_eq!(
            *vote_msg_on_timeout.sync_info().highest_quorum_cert(),
            block_0_quorum_cert
        );
    });
}

#[test]
/// We don't vote for proposals that comes from proposers that are not valid proposers for round
fn no_vote_on_invalid_proposer() {
    let runtime = consensus_runtime();
    let mut playground = NetworkPlayground::new(runtime.handle().clone());
    // In order to observe the votes we're going to check proposal processing on the non-proposer
    // node (which will send the votes to the proposer).
    let mut nodes = NodeSetup::create_nodes(
        &mut playground,
        runtime.handle().clone(),
        2,
        None,
        None,
        None,
        None,
        None,
    );
    let incorrect_proposer = nodes.pop().unwrap();
    let mut node = nodes.pop().unwrap();
    let genesis_qc = certificate_for_genesis();
    let correct_block = Block::new_proposal(
        Payload::empty(false, true),
        1,
        1,
        genesis_qc.clone(),
        &node.signer,
        Vec::new(),
    )
    .unwrap();
    let block_incorrect_proposer = Block::new_proposal(
        Payload::empty(false, true),
        1,
        1,
        genesis_qc.clone(),
        &incorrect_proposer.signer,
        Vec::new(),
    )
    .unwrap();
    timed_block_on(&runtime, async {
        let bad_proposal = ProposalMsg::new(
            block_incorrect_proposer,
            SyncInfo::new(
                genesis_qc.clone(),
                genesis_qc.into_wrapped_ledger_info(),
                None,
            ),
        );
        assert!(node
            .round_manager
            .process_proposal_msg(bad_proposal)
            .await
            .is_err());
        let good_proposal = ProposalMsg::new(
            correct_block.clone(),
            SyncInfo::new(
                genesis_qc.clone(),
                genesis_qc.into_wrapped_ledger_info(),
                None,
            ),
        );

        node.round_manager
            .process_proposal_msg(good_proposal.clone())
            .await
            .unwrap();
    });
}

#[test]
/// We allow to 'skip' round if proposal carries timeout certificate for next round
fn new_round_on_timeout_certificate() {
    let runtime = consensus_runtime();
    let mut playground = NetworkPlayground::new(runtime.handle().clone());
    // In order to observe the votes we're going to check proposal processing on the non-proposer
    // node (which will send the votes to the proposer).
    let mut node = NodeSetup::create_nodes(
        &mut playground,
        runtime.handle().clone(),
        1,
        None,
        None,
        None,
        None,
        None,
    )
    .pop()
    .unwrap();
    let genesis_qc = certificate_for_genesis();
    let correct_block = Block::new_proposal(
        Payload::empty(false, true),
        1,
        1,
        genesis_qc.clone(),
        &node.signer,
        Vec::new(),
    )
    .unwrap();
    let block_skip_round = Block::new_proposal(
        Payload::empty(false, true),
        2,
        2,
        genesis_qc.clone(),
        &node.signer,
        vec![(1, node.signer.author())],
    )
    .unwrap();
    let timeout = TwoChainTimeout::new(1, 1, genesis_qc.clone());
    let timeout_signature = timeout.sign(&node.signer).unwrap();

    let mut tc_partial = TwoChainTimeoutWithPartialSignatures::new(timeout.clone());
    tc_partial.add(node.signer.author(), timeout, timeout_signature);

    let tc = tc_partial
        .aggregate_signatures(&generate_validator_verifier(&[node.signer.clone()]))
        .unwrap();
    timed_block_on(&runtime, async {
        let skip_round_proposal = ProposalMsg::new(
            block_skip_round,
            SyncInfo::new(
                genesis_qc.clone(),
                genesis_qc.into_wrapped_ledger_info(),
                Some(tc),
            ),
        );
        node.round_manager
            .process_proposal_msg(skip_round_proposal)
            .await
            .unwrap();
        let old_good_proposal = ProposalMsg::new(
            correct_block.clone(),
            SyncInfo::new(
                genesis_qc.clone(),
                genesis_qc.into_wrapped_ledger_info(),
                None,
            ),
        );
        let before = counters::ERROR_COUNT.get();
        assert!(node
            .round_manager
            .process_proposal_msg(old_good_proposal)
            .await
            .is_ok()); // we eat the error
        assert_eq!(counters::ERROR_COUNT.get(), before + 1); // but increase the counter
    });
}

#[test]
/// We allow to 'skip' round if proposal carries timeout certificate for next round
fn reject_invalid_failed_authors() {
    let runtime = consensus_runtime();
    let mut playground = NetworkPlayground::new(runtime.handle().clone());
    // In order to observe the votes we're going to check proposal processing on the non-proposer
    // node (which will send the votes to the proposer).
    let mut node = NodeSetup::create_nodes(
        &mut playground,
        runtime.handle().clone(),
        1,
        None,
        None,
        None,
        None,
        None,
    )
    .pop()
    .unwrap();
    let genesis_qc = certificate_for_genesis();

    let create_timeout = |round: Round| {
        let timeout = TwoChainTimeout::new(1, round, genesis_qc.clone());
        let timeout_signature = timeout.sign(&node.signer).unwrap();
        let mut tc_partial = TwoChainTimeoutWithPartialSignatures::new(timeout.clone());
        tc_partial.add(node.signer.author(), timeout, timeout_signature);

        tc_partial
            .aggregate_signatures(&generate_validator_verifier(&[node.signer.clone()]))
            .unwrap()
    };

    let create_proposal = |round: Round, failed_authors: Vec<(Round, Author)>| {
        let block = Block::new_proposal(
            Payload::empty(false, true),
            round,
            2,
            genesis_qc.clone(),
            &node.signer,
            failed_authors,
        )
        .unwrap();
        ProposalMsg::new(
            block,
            SyncInfo::new(
                genesis_qc.clone(),
                genesis_qc.into_wrapped_ledger_info(),
                if round > 1 {
                    Some(create_timeout(round - 1))
                } else {
                    None
                },
            ),
        )
    };

    let extra_failed_authors_proposal = create_proposal(2, vec![(1, Author::random())]);
    let missing_failed_authors_proposal = create_proposal(2, vec![]);
    let wrong_failed_authors_proposal = create_proposal(2, vec![(1, Author::random())]);
    let not_enough_failed_proposal = create_proposal(3, vec![(2, node.signer.author())]);
    let valid_proposal = create_proposal(
        4,
        (1..4).map(|i| (i as Round, node.signer.author())).collect(),
    );

    timed_block_on(&runtime, async {
        assert!(node
            .round_manager
            .process_proposal_msg(extra_failed_authors_proposal)
            .await
            .is_err());

        assert!(node
            .round_manager
            .process_proposal_msg(missing_failed_authors_proposal)
            .await
            .is_err());
    });

    timed_block_on(&runtime, async {
        assert!(node
            .round_manager
            .process_proposal_msg(wrong_failed_authors_proposal)
            .await
            .is_err());

        assert!(node
            .round_manager
            .process_proposal_msg(not_enough_failed_proposal)
            .await
            .is_err());

        node.round_manager
            .process_proposal_msg(valid_proposal)
            .await
            .unwrap()
    });
}

#[test]
fn response_on_block_retrieval() {
    let runtime = consensus_runtime();
    let mut playground = NetworkPlayground::new(runtime.handle().clone());
    let mut node = NodeSetup::create_nodes(
        &mut playground,
        runtime.handle().clone(),
        1,
        None,
        None,
        None,
        None,
        None,
    )
    .pop()
    .unwrap();

    let genesis_qc = certificate_for_genesis();
    let block = Block::new_proposal(
        Payload::empty(false, true),
        1,
        1,
        genesis_qc.clone(),
        &node.signer,
        Vec::new(),
    )
    .unwrap();
    let block_id = block.id();
    let proposal = ProposalMsg::new(
        block,
        SyncInfo::new(
            genesis_qc.clone(),
            genesis_qc.into_wrapped_ledger_info(),
            None,
        ),
    );

    timed_block_on(&runtime, async {
        node.round_manager
            .process_proposal_msg(proposal)
            .await
            .unwrap();

        // first verify that we can retrieve the block if it's in the tree
        let (tx1, rx1) = oneshot::channel();
        let single_block_request = IncomingBlockRetrievalRequest {
            req: BlockRetrievalRequest::new(block_id, 1),
            protocol: ProtocolId::ConsensusRpcBcs,
            response_sender: tx1,
        };
        node.block_store
            .process_block_retrieval(single_block_request)
            .await
            .unwrap();
        match rx1.await {
            Ok(Ok(bytes)) => {
                let response = match bcs::from_bytes(&bytes) {
                    Ok(ConsensusMsg::BlockRetrievalResponse(resp)) => *resp,
                    _ => panic!("block retrieval failure"),
                };
                assert_eq!(response.status(), BlockRetrievalStatus::Succeeded);
                assert_eq!(response.blocks().first().unwrap().id(), block_id);
            },
            _ => panic!("block retrieval failure"),
        }

        // verify that if a block is not there, return ID_NOT_FOUND
        let (tx2, rx2) = oneshot::channel();
        let missing_block_request = IncomingBlockRetrievalRequest {
            req: BlockRetrievalRequest::new(HashValue::random(), 1),
            protocol: ProtocolId::ConsensusRpcBcs,
            response_sender: tx2,
        };

        node.block_store
            .process_block_retrieval(missing_block_request)
            .await
            .unwrap();
        match rx2.await {
            Ok(Ok(bytes)) => {
                let response = match bcs::from_bytes(&bytes) {
                    Ok(ConsensusMsg::BlockRetrievalResponse(resp)) => *resp,
                    _ => panic!("block retrieval failure"),
                };
                assert_eq!(response.status(), BlockRetrievalStatus::IdNotFound);
                assert!(response.blocks().is_empty());
            },
            _ => panic!("block retrieval failure"),
        }

        // if asked for many blocks, return NOT_ENOUGH_BLOCKS
        let (tx3, rx3) = oneshot::channel();
        let many_block_request = IncomingBlockRetrievalRequest {
            req: BlockRetrievalRequest::new(block_id, 3),
            protocol: ProtocolId::ConsensusRpcBcs,
            response_sender: tx3,
        };
        node.block_store
            .process_block_retrieval(many_block_request)
            .await
            .unwrap();
        match rx3.await {
            Ok(Ok(bytes)) => {
                let response = match bcs::from_bytes(&bytes) {
                    Ok(ConsensusMsg::BlockRetrievalResponse(resp)) => *resp,
                    _ => panic!("block retrieval failure"),
                };
                assert_eq!(response.status(), BlockRetrievalStatus::NotEnoughBlocks);
                assert_eq!(block_id, response.blocks().first().unwrap().id());
                assert_eq!(
                    node.block_store.ordered_root().id(),
                    response.blocks().get(1).unwrap().id()
                );
            },
            _ => panic!("block retrieval failure"),
        }
    });
}

#[test]
/// rebuild a node from previous storage without violating safety guarantees.
fn recover_on_restart() {
    let runtime = consensus_runtime();
    let mut playground = NetworkPlayground::new(runtime.handle().clone());
    let mut node = NodeSetup::create_nodes(
        &mut playground,
        runtime.handle().clone(),
        1,
        None,
        None,
        None,
        None,
        None,
    )
    .pop()
    .unwrap();
    let inserter = TreeInserter::new_with_store(node.signer.clone(), node.block_store.clone());

    let genesis_qc = certificate_for_genesis();
    let mut data = Vec::new();
    let num_proposals = 100;
    // insert a few successful proposals
    for i in 1..=num_proposals {
        let proposal = inserter.create_block_with_qc(
            genesis_qc.clone(),
            i,
            i,
            Payload::empty(false, true),
            (std::cmp::max(1, i.saturating_sub(10))..i)
                .map(|i| (i, inserter.signer().author()))
                .collect(),
        );
        let timeout = TwoChainTimeout::new(1, i - 1, genesis_qc.clone());
        let mut tc_partial = TwoChainTimeoutWithPartialSignatures::new(timeout.clone());
        tc_partial.add(
            inserter.signer().author(),
            timeout.clone(),
            timeout.sign(inserter.signer()).unwrap(),
        );

        let tc = tc_partial
            .aggregate_signatures(&generate_validator_verifier(&[node.signer.clone()]))
            .unwrap();

        data.push((proposal, tc));
    }

    timed_block_on(&runtime, async {
        for (proposal, tc) in &data {
            let proposal_msg = ProposalMsg::new(
                proposal.clone(),
                SyncInfo::new(
                    proposal.quorum_cert().clone(),
                    genesis_qc.into_wrapped_ledger_info(),
                    Some(tc.clone()),
                ),
            );
            node.round_manager
                .process_proposal_msg(proposal_msg)
                .await
                .unwrap();
        }
    });

    // verify after restart we recover the data
    node = node.restart(&mut playground, runtime.handle().clone());
    let consensus_state = node.round_manager.consensus_state();
    assert_eq!(consensus_state.epoch(), 1);
    assert_eq!(consensus_state.last_voted_round(), num_proposals);
    assert_eq!(consensus_state.preferred_round(), 0);
    assert!(consensus_state.in_validator_set());
    for (block, _) in data {
        assert!(node.block_store.block_exists(block.id()));
    }
}

#[test]
/// Generate a NIL vote extending HQC upon timeout if no votes have been sent in the round.
fn nil_vote_on_timeout() {
    let runtime = consensus_runtime();
    let mut playground = NetworkPlayground::new(runtime.handle().clone());
    let mut nodes = NodeSetup::create_nodes(
        &mut playground,
        runtime.handle().clone(),
        1,
        None,
        None,
        Some(config_with_round_timeout_msg_disabled()),
        None,
        None,
    );
    let node = &mut nodes[0];
    let genesis = node.block_store.ordered_root();
    timed_block_on(&runtime, async {
        node.next_proposal().await;
        // Process the outgoing vote message and verify that it contains a round signature
        // and that the vote extends genesis.
        node.round_manager
            .process_local_timeout(1)
            .await
            .unwrap_err();
        let vote_msg = node.next_vote().await;

        let vote = vote_msg.vote();

        assert!(vote.is_timeout());
        // NIL block doesn't change timestamp
        assert_eq!(
            vote.vote_data().proposed().timestamp_usecs(),
            genesis.timestamp_usecs()
        );
        assert_eq!(vote.vote_data().proposed().round(), 1);
        assert_eq!(
            vote.vote_data().parent().id(),
            node.block_store.ordered_root().id()
        );
    });
}

#[test]
/// Generate a Timeout upon timeout if no votes have been sent in the round.
fn timeout_round_on_timeout() {
    let runtime = consensus_runtime();
    let mut playground = NetworkPlayground::new(runtime.handle().clone());
    let local_config = ConsensusConfig {
        enable_round_timeout_msg: true,
        ..Default::default()
    };
    let mut nodes = NodeSetup::create_nodes(
        &mut playground,
        runtime.handle().clone(),
        1,
        None,
        None,
        Some(local_config),
        None,
        None,
    );
    let node = &mut nodes[0];
    let genesis = node.block_store.ordered_root();
    timed_block_on(&runtime, async {
        node.next_proposal().await;
        // Process the outgoing vote message and verify that it contains a round signature
        // and that the vote extends genesis.
        node.round_manager
            .process_local_timeout(1)
            .await
            .unwrap_err();
        let timeout_msg = node.next_timeout().await;

        let timeout = timeout_msg.timeout();

        assert_eq!(timeout.round(), 1);
        assert_eq!(timeout.author(), node.signer.author());
        assert_eq!(timeout.epoch(), 1);
        assert_eq!(timeout.two_chain_timeout().hqc_round(), genesis.round());
    });
}

#[test]
/// If the node votes in a round, upon timeout the same vote is re-sent with a timeout signature.
fn vote_resent_on_timeout() {
    let runtime = consensus_runtime();
    let mut playground = NetworkPlayground::new(runtime.handle().clone());
    let mut nodes = NodeSetup::create_nodes(
        &mut playground,
        runtime.handle().clone(),
        1,
        None,
        None,
        Some(config_with_round_timeout_msg_disabled()),
        None,
        None,
    );
    let node = &mut nodes[0];
    timed_block_on(&runtime, async {
        let proposal_msg = node.next_proposal().await;
        let id = proposal_msg.proposal().id();
        node.round_manager
            .process_proposal_msg(proposal_msg)
            .await
            .unwrap();
        let vote_msg = node.next_vote().await;
        let vote = vote_msg.vote();
        assert!(!vote.is_timeout());
        assert_eq!(vote.vote_data().proposed().id(), id);
        // Process the outgoing vote message and verify that it contains a round signature
        // and that the vote is the same as above.
        node.round_manager
            .process_local_timeout(1)
            .await
            .unwrap_err();
        let timeout_vote_msg = node.next_vote().await;
        let timeout_vote = timeout_vote_msg.vote();

        assert!(timeout_vote.is_timeout());
        assert_eq!(timeout_vote.vote_data(), vote.vote_data());
    });
}

#[test]
/// If the node votes in a round, upon timeout the same vote is re-sent with a timeout signature.
fn timeout_sent_on_timeout_after_vote() {
    let runtime = consensus_runtime();
    let mut playground = NetworkPlayground::new(runtime.handle().clone());
    let local_config = ConsensusConfig {
        enable_round_timeout_msg: true,
        ..Default::default()
    };
    let mut nodes = NodeSetup::create_nodes(
        &mut playground,
        runtime.handle().clone(),
        1,
        None,
        None,
        Some(local_config),
        None,
        None,
    );
    let node = &mut nodes[0];
    timed_block_on(&runtime, async {
        let proposal_msg = node.next_proposal().await;
        let id = proposal_msg.proposal().id();
        node.round_manager
            .process_proposal_msg(proposal_msg)
            .await
            .unwrap();
        let vote_msg = node.next_vote().await;
        let vote = vote_msg.vote();
        assert!(!vote.is_timeout());
        assert_eq!(vote.vote_data().proposed().id(), id);
        // Process the outgoing vote message and verify that it contains a round signature
        // and that the vote is the same as above.
        node.round_manager
            .process_local_timeout(1)
            .await
            .unwrap_err();
        let timeout_msg = node.next_timeout().await;

        assert_eq!(timeout_msg.round(), vote.vote_data().proposed().round());
        assert_eq!(timeout_msg.sync_info(), vote_msg.sync_info());
    });
}

#[test]
#[ignore] // TODO: this test needs to be fixed!
fn sync_on_partial_newer_sync_info() {
    let runtime = consensus_runtime();
    let mut playground = NetworkPlayground::new(runtime.handle().clone());
    let mut nodes = NodeSetup::create_nodes(
        &mut playground,
        runtime.handle().clone(),
        1,
        None,
        None,
        None,
        None,
        None,
    );
    let mut node = nodes.pop().unwrap();
    runtime.spawn(playground.start());
    timed_block_on(&runtime, async {
        // commit block 1 after 4 rounds
        for _ in 1..=4 {
            let proposal_msg = node.next_proposal().await;

            node.round_manager
                .process_proposal_msg(proposal_msg)
                .await
                .unwrap();
            let vote_msg = node.next_vote().await;
            // Adding vote to form a QC
            node.round_manager.process_vote_msg(vote_msg).await.unwrap();
        }
        let block_4 = node.next_proposal().await;
        node.round_manager
            .process_proposal_msg(block_4.clone())
            .await
            .unwrap();
        // commit genesis and block 1
        for i in 0..2 {
            node.commit_next_ordered(&[i]).await;
        }
        let vote_msg = node.next_vote().await;
        let vote_data = vote_msg.vote().vote_data();
        let block_4_qc = gen_test_certificate(
            &[node.signer.clone()],
            vote_data.proposed().clone(),
            vote_data.parent().clone(),
            None,
        );
        // Create a sync info with newer quorum cert but older commit cert
        let sync_info = SyncInfo::new(
            block_4_qc.clone(),
            certificate_for_genesis().into_wrapped_ledger_info(),
            None,
        );
        node.round_manager
            .ensure_round_and_sync_up(
                sync_info.highest_round() + 1,
                &sync_info,
                node.signer.author(),
            )
            .await
            .unwrap();
        // QuorumCert added
        assert_eq!(*node.block_store.highest_quorum_cert(), block_4_qc);
    });
}

#[test]
fn safety_rules_crash() {
    let runtime = consensus_runtime();
    let mut playground = NetworkPlayground::new(runtime.handle().clone());
    let mut nodes = NodeSetup::create_nodes(
        &mut playground,
        runtime.handle().clone(),
        1,
        None,
        None,
        Some(config_with_round_timeout_msg_disabled()),
        None,
        None,
    );
    let mut node = nodes.pop().unwrap();
    runtime.spawn(playground.start());

    fn reset_safety_rules(node: &mut NodeSetup) {
        let safety_storage = PersistentSafetyStorage::initialize(
            Storage::from(aptos_secure_storage::InMemoryStorage::new()),
            node.signer.author(),
            node.signer.private_key().clone(),
            node.round_manager.consensus_state().waypoint(),
            true,
        );

        node.safety_rules_manager = SafetyRulesManager::new_local(safety_storage);
        let safety_rules =
            MetricsSafetyRules::new(node.safety_rules_manager.client(), node.storage.clone());
        let safety_rules_container = Arc::new(Mutex::new(safety_rules));
        node.round_manager.set_safety_rules(safety_rules_container);
    }

    timed_block_on(&runtime, async {
        for _ in 0..2 {
            let proposal_msg = node.next_proposal().await;

            reset_safety_rules(&mut node);
            // construct_and_sign_vote
            node.round_manager
                .process_proposal_msg(proposal_msg)
                .await
                .unwrap();

            let vote_msg = node.next_vote().await;

            // sign_timeout
            reset_safety_rules(&mut node);
            let round = vote_msg.vote().vote_data().proposed().round();
            node.round_manager
                .process_local_timeout(round)
                .await
                .unwrap_err();
            let vote_msg = node.next_vote().await;

            // sign proposal
            reset_safety_rules(&mut node);
            node.round_manager.process_vote_msg(vote_msg).await.unwrap();
        }

        // verify the last sign proposal happened
        node.next_proposal().await;
    });
}

#[test]
fn echo_timeout() {
    let runtime = consensus_runtime();
    let mut playground = NetworkPlayground::new(runtime.handle().clone());
    let mut nodes = NodeSetup::create_nodes(
        &mut playground,
        runtime.handle().clone(),
        4,
        None,
        None,
        Some(config_with_round_timeout_msg_disabled()),
        None,
        None,
    );
    runtime.spawn(playground.start());
    timed_block_on(&runtime, async {
        // clear the message queue
        for node in &mut nodes {
            node.next_proposal().await;
        }
        // timeout 3 nodes
        for node in &mut nodes[1..] {
            node.round_manager
                .process_local_timeout(1)
                .await
                .unwrap_err();
        }
        let node_0 = &mut nodes[0];
        // node 0 doesn't timeout and should echo the timeout after 2 timeout message
        for i in 0..3 {
            let timeout_vote = node_0.next_vote().await;
            let result = node_0.round_manager.process_vote_msg(timeout_vote).await;
            // first and third message should not timeout
            if i == 0 || i == 2 {
                assert!(result.is_ok());
            }
            if i == 1 {
                // timeout is an Error
                assert!(result.is_err());
            }
        }

        let node_1 = &mut nodes[1];
        // it receives 4 timeout messages (1 from each) and doesn't echo since it already timeout
        for _ in 0..4 {
            let timeout_vote = node_1.next_vote().await;
            // Verifying only some vote messages to check that round manager can accept both
            // verified and unverified votes
            node_1
                .round_manager
                .process_vote_msg(timeout_vote)
                .await
                .unwrap();
        }
    });
}

#[test]
fn echo_round_timeout_msg() {
    let runtime = consensus_runtime();
    let mut playground = NetworkPlayground::new(runtime.handle().clone());
    let mut nodes = NodeSetup::create_nodes(
        &mut playground,
        runtime.handle().clone(),
        4,
        None,
        None,
        None,
        None,
        None,
    );
    runtime.spawn(playground.start());
    timed_block_on(&runtime, async {
        // clear the message queue
        for node in &mut nodes {
            node.next_proposal().await;
        }
        // timeout 3 nodes
        for node in &mut nodes[1..] {
            node.round_manager
                .process_local_timeout(1)
                .await
                .unwrap_err();
        }
        let node_0 = &mut nodes[0];
        // node 0 doesn't timeout and should echo the timeout after 2 timeout message
        for i in 0..3 {
            let timeout_vote = node_0.next_timeout().await;
            let result = node_0
                .round_manager
                .process_round_timeout_msg(timeout_vote)
                .await;
            // first and third message should not timeout
            if i == 0 || i == 2 {
                assert!(result.is_ok());
            }
            if i == 1 {
                // timeout is an Error
                assert!(result.is_err());
            }
        }

        let node_1 = &mut nodes[1];
        // it receives 4 timeout messages (1 from each) and doesn't echo since it already timeout
        for _ in 0..4 {
            let timeout_vote = node_1.next_timeout().await;
            node_1
                .round_manager
                .process_round_timeout_msg(timeout_vote)
                .await
                .unwrap();
        }
    });
}

#[test]
fn no_next_test() {
    let runtime = consensus_runtime();
    let mut playground = NetworkPlayground::new(runtime.handle().clone());
    let mut nodes = NodeSetup::create_nodes(
        &mut playground,
        runtime.handle().clone(),
        4,
        None,
        None,
        None,
        None,
        None,
    );
    runtime.spawn(playground.start());

    timed_block_on(&runtime, async {
        // clear the message queue
        for node in &mut nodes {
            node.next_proposal().await;
        }

        tokio::time::sleep(Duration::from_secs(1)).await;

        for node in nodes.iter_mut() {
            node.no_next_msg();
        }
        tokio::time::sleep(Duration::from_secs(1)).await;

        for node in nodes.iter_mut() {
            node.no_next_msg();
        }
    });
}

#[test]
fn commit_pipeline_test() {
    let runtime = consensus_runtime();
    let proposers = vec![0, 0, 0, 0, 5];

    let mut playground = NetworkPlayground::new(runtime.handle().clone());
    let mut nodes = NodeSetup::create_nodes(
        &mut playground,
        runtime.handle().clone(),
        7,
        Some(proposers.clone()),
        None,
        None,
        None,
        None,
    );
    runtime.spawn(playground.start());
    let behind_node = 6;
    for i in 0..10 {
        let next_proposer = proposers[(i + 2) as usize % proposers.len()];
        let prev_proposer = proposers[(i + 1) as usize % proposers.len()];
        info!("processing {}", i);
        process_and_vote_on_proposal(
            &runtime,
            &mut nodes,
            next_proposer,
            &[behind_node],
            true,
            Some(prev_proposer),
            true,
            i + 1,
            i.saturating_sub(1),
            i.saturating_sub(2),
        );

        std::thread::sleep(Duration::from_secs(1));

        for node in nodes.iter_mut() {
            node.no_next_ordered();
        }
    }
}

#[test]
fn block_retrieval_test() {
    let runtime = consensus_runtime();
    let mut playground = NetworkPlayground::new(runtime.handle().clone());
    let mut nodes = NodeSetup::create_nodes(
        &mut playground,
        runtime.handle().clone(),
        4,
        Some(vec![0, 1]),
        None,
        None,
        None,
        None,
    );
    runtime.spawn(playground.start());

    for i in 0..4 {
        info!("processing {}", i);
        process_and_vote_on_proposal(
            &runtime,
            &mut nodes,
            i as usize % 2,
            &[3],
            true,
            None,
            true,
            i + 1,
            i.saturating_sub(1),
            0,
        );
    }

    timed_block_on(&runtime, async {
        let mut behind_node = nodes.pop().unwrap();

        // Drain the queue on other nodes
        for node in nodes.iter_mut() {
            let _ = node.next_proposal().await;
        }

        info!(
            "Processing proposals for behind node {}",
            behind_node.identity_desc()
        );
        let handle = start_replying_to_block_retreival(nodes);
        let proposal_msg = behind_node.next_proposal().await;
        behind_node
            .round_manager
            .process_proposal_msg(proposal_msg)
            .await
            .unwrap();

        handle.join().await;
    });
}

#[test]
fn block_retrieval_timeout_test() {
    let runtime = consensus_runtime();
    let mut playground = NetworkPlayground::new(runtime.handle().clone());
    let mut nodes = NodeSetup::create_nodes(
        &mut playground,
        runtime.handle().clone(),
        4,
        Some(vec![0, 1]),
        None,
        None,
        None,
        None,
    );
    let timeout_config = playground.timeout_config();
    runtime.spawn(playground.start());

    for i in 0..4 {
        info!("processing {}", i);
        process_and_vote_on_proposal(
            &runtime,
            &mut nodes,
            i as usize % 2,
            &[3],
            true,
            None,
            true,
            i + 1,
            i.saturating_sub(1),
            0,
        );
    }

    timed_block_on(&runtime, async {
        let mut behind_node = nodes.pop().unwrap();

        for node in nodes.iter() {
            timeout_config.write().timeout_message_for(
                &TwinId {
                    id: behind_node.id,
                    author: behind_node.signer.author(),
                },
                &TwinId {
                    id: node.id,
                    author: node.signer.author(),
                },
            );
        }

        // Drain the queue on other nodes
        for node in nodes.iter_mut() {
            let _ = node.next_proposal().await;
        }

        info!(
            "Processing proposals for behind node {}",
            behind_node.identity_desc()
        );

        let proposal_msg = behind_node.next_proposal().await;
        behind_node
            .round_manager
            .process_proposal_msg(proposal_msg)
            .await
            .unwrap_err();
    });
}

#[ignore] // TODO: turn this test back on once the flakes have resolved.
#[test]
pub fn forking_retrieval_test() {
    let runtime = consensus_runtime();

    let proposal_node = 0;
    let behind_node = 6;
    let forking_node = 5;

    let mut playground = NetworkPlayground::new(runtime.handle().clone());
    let mut nodes = NodeSetup::create_nodes(
        &mut playground,
        runtime.handle().clone(),
        7,
        Some(vec![
            proposal_node,
            proposal_node,
            proposal_node,
            proposal_node,
            proposal_node,
            forking_node,
            proposal_node,
            proposal_node,
        ]),
        None,
        None,
        None,
        None,
    );
    runtime.spawn(playground.start());

    info!("Propose vote and commit on first block");
    process_and_vote_on_proposal(
        &runtime,
        &mut nodes,
        proposal_node,
        &[behind_node],
        true,
        Some(proposal_node),
        true,
        1,
        0,
        0,
    );

    info!("Propose vote and commit on second block");
    process_and_vote_on_proposal(
        &runtime,
        &mut nodes,
        proposal_node,
        &[behind_node],
        true,
        Some(proposal_node),
        true,
        2,
        0,
        0,
    );

    info!("Propose vote and commit on second block");
    process_and_vote_on_proposal(
        &runtime,
        &mut nodes,
        proposal_node,
        &[behind_node],
        true,
        Some(proposal_node),
        true,
        3,
        1,
        0,
    );

    info!("Propose vote and commit on third (dangling) block");
    process_and_vote_on_proposal(
        &runtime,
        &mut nodes,
        forking_node,
        &[behind_node, forking_node],
        false,
        Some(proposal_node),
        true,
        4,
        2,
        1,
    );

    timed_block_on(&runtime, async {
        info!("Insert local timeout to all nodes on next round");
        let mut timeout_votes = 0;
        for node in nodes.iter_mut() {
            if node.id != behind_node && node.id != forking_node {
                node.round_manager
                    .process_local_timeout(4)
                    .await
                    .unwrap_err();
                timeout_votes += 1;
            }
        }

        info!("Process all local timeouts");
        for node in nodes.iter_mut() {
            info!("Timeouts on {}", node.id);
            for i in 0..timeout_votes {
                info!("Timeout {} on {}", i, node.id);
                if node.id == forking_node && (2..4).contains(&i) {
                    info!("Got {}", node.next_commit_decision().await);
                }

                let vote_msg_on_timeout = node.next_vote().await;
                assert!(vote_msg_on_timeout.vote().is_timeout());
                if node.id != behind_node {
                    let result = node
                        .round_manager
                        .process_vote_msg(vote_msg_on_timeout)
                        .await;

                    if node.id == forking_node && i == 2 {
                        result.unwrap_err();
                    } else {
                        result.unwrap();
                    }
                }
            }
        }
    });

    timed_block_on(&runtime, async {
        for node in nodes.iter_mut() {
            let vote_msg_on_timeout = node.next_vote().await;
            assert!(vote_msg_on_timeout.vote().is_timeout());
        }

        info!("Got {}", nodes[forking_node].next_commit_decision().await);
    });

    info!("Create forked block");
    process_and_vote_on_proposal(
        &runtime,
        &mut nodes,
        proposal_node,
        &[behind_node],
        true,
        Some(forking_node),
        false,
        5,
        2,
        1,
    );

    process_and_vote_on_proposal(
        &runtime,
        &mut nodes,
        proposal_node,
        &[behind_node, forking_node],
        true,
        None,
        false,
        6,
        3,
        3,
    );

    process_and_vote_on_proposal(
        &runtime,
        &mut nodes,
        proposal_node,
        &[behind_node, forking_node],
        true,
        None,
        false,
        7,
        5,
        3,
    );

    let mut nodes = timed_block_on(&runtime, async {
        let mut behind_node_obj = nodes.pop().unwrap();

        // Drain the queue on other nodes
        let mut proposals = Vec::new();
        for node in nodes.iter_mut() {
            proposals.push(node.next_proposal().await);
        }

        info!(
            "Processing proposals for behind node {}",
            behind_node_obj.identity_desc()
        );
        let handle = start_replying_to_block_retreival(nodes);
        let proposal_msg = behind_node_obj.next_proposal().await;
        behind_node_obj
            .round_manager
            .process_proposal_msg(proposal_msg.clone())
            .await
            .unwrap();

        nodes = handle.join().await;
        behind_node_obj.no_next_msg();

        for (proposal, node) in proposals.into_iter().zip(nodes.iter_mut()) {
            node.pending_network_events.push(Event::Message(
                node.signer.author(),
                ConsensusMsg::ProposalMsg(Box::new(proposal)),
            ));
        }
        behind_node_obj.pending_network_events.push(Event::Message(
            behind_node_obj.signer.author(),
            ConsensusMsg::ProposalMsg(Box::new(proposal_msg)),
        ));

        nodes.push(behind_node_obj);
        nodes
    });

    // confirm behind node can participate in consensus after state sync
    process_and_vote_on_proposal(
        &runtime,
        &mut nodes,
        proposal_node,
        &[forking_node, behind_node],
        true,
        None,
        false,
        8,
        6,
        3,
    );

    // let next_message = timed_block_on(&runtime, nodes[proposal_node].next_network_message());
    // match next_message {
    //     ConsensusMsg::VoteMsg(_) => info!("Skip extra vote msg"),
    //     ConsensusMsg::ProposalMsg(msg) => {
    //         // put the message back in the queue.
    //         // actual peer doesn't matter, it is ignored, so use self.
    //         let peer = nodes[proposal_node].signer.author();
    //         nodes[proposal_node]
    //             .pending_network_events
    //             .push(Event::Message(peer, ConsensusMsg::ProposalMsg(msg)))
    //     },
    //     _ => panic!("unexpected network message {:?}", next_message),
    // }
    process_and_vote_on_proposal(
        &runtime,
        &mut nodes,
        proposal_node,
        &[forking_node],
        true,
        None,
        false,
        9,
        7,
        3,
    );
}

#[test]
/// If ProposalExt feature is disabled, ProposalExt should be rejected
/// No votes are sent, but the block is still added to the block tree.
fn no_vote_on_proposal_ext_when_feature_disabled() {
    let runtime = consensus_runtime();
    let mut playground = NetworkPlayground::new(runtime.handle().clone());
    // In order to observe the votes we're going to check proposal processing on the non-proposer
    // node (which will send the votes to the proposer).
    let mut nodes = NodeSetup::create_nodes(
        &mut playground,
        runtime.handle().clone(),
        1,
        None,
        None,
        None,
        None,
        None,
    );
    let node = &mut nodes[0];
    let genesis_qc = certificate_for_genesis();

    let invalid_block = Block::new_proposal_ext(
        vec![ValidatorTransaction::dummy(vec![0xFF]); 5],
        Payload::empty(false, true),
        1,
        1,
        genesis_qc.clone(),
        &node.signer,
        Vec::new(),
    )
    .unwrap();

    let valid_block = Block::new_proposal(
        Payload::empty(false, true),
        1,
        1,
        genesis_qc,
        &node.signer,
        Vec::new(),
    )
    .unwrap();

    timed_block_on(&runtime, async {
        // clear the message queue
        node.next_proposal().await;

        assert!(node
            .round_manager
            .process_proposal(invalid_block)
            .await
            .is_err());

        assert!(node
            .round_manager
            .process_proposal(valid_block)
            .await
            .is_ok());
    });
}

#[test]
fn no_vote_on_proposal_with_unexpected_vtxns() {
    let vtxns = vec![ValidatorTransaction::ObservedJWKUpdate(
        QuorumCertifiedUpdate::dummy(),
    )];

    assert_process_proposal_result(
        None,
        Some(OnChainJWKConsensusConfig::default_disabled()),
        vtxns.clone(),
        false,
    );

    assert_process_proposal_result(
        None,
        Some(OnChainJWKConsensusConfig::default_enabled()),
        vtxns,
        true,
    );
}

/// Setup a node with default configs and an optional `Features` override.
/// Create a block, fill it with the given vtxns, and process it with the `RoundManager` from the setup.
/// Assert the processing result.
fn assert_process_proposal_result(
    randomness_config: Option<OnChainRandomnessConfig>,
    jwk_consensus_config: Option<OnChainJWKConsensusConfig>,
    vtxns: Vec<ValidatorTransaction>,
    expected_result: bool,
) {
    let runtime = consensus_runtime();
    let mut playground = NetworkPlayground::new(runtime.handle().clone());
    let mut nodes = NodeSetup::create_nodes(
        &mut playground,
        runtime.handle().clone(),
        1,
        None,
        Some(OnChainConsensusConfig::default_for_genesis()),
        None,
        randomness_config,
        jwk_consensus_config,
    );

    let node = &mut nodes[0];
    let genesis_qc = certificate_for_genesis();
    let block = Block::new_proposal_ext(
        vtxns,
        Payload::empty(false, true),
        1,
        1,
        genesis_qc.clone(),
        &node.signer,
        Vec::new(),
    )
    .unwrap();

    timed_block_on(&runtime, async {
        // clear the message queue
        node.next_proposal().await;

        assert_eq!(
            expected_result,
            node.round_manager
                .process_proposal(block.clone())
                .await
                .is_ok()
        );
    });
}

#[test]
/// If receiving txn num/block size limit is exceeded, ProposalExt should be rejected.
fn no_vote_on_proposal_ext_when_receiving_limit_exceeded() {
    let runtime = consensus_runtime();
    let mut playground = NetworkPlayground::new(runtime.handle().clone());

    let alg_config = ConsensusAlgorithmConfig::JolteonV2 {
        main: ConsensusConfigV1::default(),
        quorum_store_enabled: true,
        order_vote_enabled: false,
    };
    let vtxn_config = ValidatorTxnConfig::V1 {
        per_block_limit_txn_count: 5,
        per_block_limit_total_bytes: 400,
    };

    let local_config = ConsensusConfig {
        max_receiving_block_txns: 10,
        max_receiving_block_bytes: 800,
        ..Default::default()
    };

    let randomness_config = OnChainRandomnessConfig::default_enabled();
    let mut nodes = NodeSetup::create_nodes(
        &mut playground,
        runtime.handle().clone(),
        1,
        None,
        Some(OnChainConsensusConfig::V3 {
            alg: alg_config,
            vtxn: vtxn_config,
        }),
        Some(local_config),
        Some(randomness_config),
        None,
    );
    let node = &mut nodes[0];
    let genesis_qc = certificate_for_genesis();

    let block_too_many_txns = Block::new_proposal_ext(
        vec![],
        Payload::DirectMempool(create_vec_signed_transactions(11)),
        1,
        1,
        genesis_qc.clone(),
        &node.signer,
        Vec::new(),
    )
    .unwrap();

    let block_too_many_vtxns = Block::new_proposal_ext(
        vec![ValidatorTransaction::dummy(vec![0xFF; 20]); 6],
        Payload::DirectMempool(create_vec_signed_transactions(4)),
        1,
        1,
        genesis_qc.clone(),
        &node.signer,
        Vec::new(),
    )
    .unwrap();

    let block_too_large = Block::new_proposal_ext(
        vec![ValidatorTransaction::dummy(vec![0xFF; 200]); 1], // total_bytes >= 200 * 1 = 200
        Payload::DirectMempool(create_vec_signed_transactions(9)), // = total_bytes >= 69 * 9 = 621
        1,
        1,
        genesis_qc.clone(),
        &node.signer,
        Vec::new(),
    )
    .unwrap();

    let block_vtxns_too_large = Block::new_proposal_ext(
        vec![ValidatorTransaction::dummy(vec![0xFF; 200]); 5], // total_bytes >= 200 * 5 = 1000
        Payload::empty(false, true),
        1,
        1,
        genesis_qc.clone(),
        &node.signer,
        Vec::new(),
    )
    .unwrap();

    let valid_block = Block::new_proposal_ext(
        vec![ValidatorTransaction::dummy(vec![0xFF; 20]); 5], // total_bytes >= 60 * 5 = 300
        Payload::DirectMempool(create_vec_signed_transactions(5)), // total_bytes >= 69 * 5 = 345
        1,
        1,
        genesis_qc.clone(),
        &node.signer,
        Vec::new(),
    )
    .unwrap();

    timed_block_on(&runtime, async {
        // clear the message queue
        node.next_proposal().await;

        assert!(node
            .round_manager
            .process_proposal(block_too_many_txns)
            .await
            .is_err());

        assert!(node
            .round_manager
            .process_proposal(block_too_many_vtxns)
            .await
            .is_err());

        assert!(node
            .round_manager
            .process_proposal(block_too_large)
            .await
            .is_err());

        assert!(node
            .round_manager
            .process_proposal(block_vtxns_too_large)
            .await
            .is_err());

        assert!(node
            .round_manager
            .process_proposal(valid_block)
            .await
            .is_ok());
    });
}
