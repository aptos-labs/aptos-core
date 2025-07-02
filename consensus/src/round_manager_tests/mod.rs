// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    block_storage::{pending_blocks::PendingBlocks, BlockStore},
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
        mock_execution_client::MockExecutionClient, MockOptQSPayloadProvider,
        MockPastProposalStatusTracker, MockPayloadManager, MockStorage,
    },
    util::time_service::{ClockTimeService, TimeService},
};
use aptos_channels::{self, aptos_channel, message_queues::QueueStyle};
use aptos_config::{
    config::ConsensusConfig,
    network_id::{NetworkId, PeerNetworkId},
};
use aptos_consensus_types::{
    block_retrieval::BlockRetrievalRequest,
    common::{Author, Round},
    opt_block_data::OptBlockData,
    opt_proposal_msg::OptProposalMsg,
    order_vote_msg::OrderVoteMsg,
    pipeline::commit_decision::CommitDecision,
    proposal_msg::ProposalMsg,
    round_timeout::RoundTimeoutMsg,
    utils::PayloadTxnsSize,
    vote_msg::VoteMsg,
    wrapped_ledger_info::WrappedLedgerInfo,
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
    ledger_info::LedgerInfo,
    on_chain_config::{
        ConsensusAlgorithmConfig, OnChainConsensusConfig, OnChainJWKConsensusConfig,
        OnChainRandomnessConfig,
    },
    transaction::SignedTransaction,
    validator_signer::ValidatorSigner,
    validator_verifier::{random_validator_verifier, ValidatorVerifier},
    waypoint::Waypoint,
};
use futures::{channel::mpsc, executor::block_on, stream::select, FutureExt, Stream, StreamExt};
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
use tokio::{runtime::Handle, task::JoinHandle};

mod consensus_test;
mod opt_proposal_test;
mod vtxn_on_proposal_test;

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
                let maybe_request = node.poll_block_retrieval().await;
                if let Some(request) = maybe_request {
                    info!(
                        "RPC request received: {:?} on {:?}",
                        request,
                        node.identity_desc()
                    );
                    let wrapped_request = IncomingBlockRetrievalRequest {
                        req: request.req,
                        protocol: request.protocol,
                        response_sender: request.response_sender,
                    };
                    node.block_store
                        .process_block_retrieval(wrapped_request)
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

#[derive(Debug)]
pub enum ProposalMsgType {
    Normal(ProposalMsg),
    Optimistic(OptProposalMsg),
}

/// Auxiliary struct that is setting up node environment for the test.
pub struct NodeSetup {
    pub block_store: Arc<BlockStore>,
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
    opt_proposal_queue: VecDeque<OptProposalMsg>,
    round_timeout_queue: VecDeque<RoundTimeoutMsg>,
    commit_decision_queue: VecDeque<CommitDecision>,
    processed_opt_proposal_rx: aptos_channels::UnboundedReceiver<OptBlockData>,
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
        Self::create_nodes_with_validator_set(
            playground,
            executor,
            num_nodes,
            proposer_indices,
            onchain_consensus_config,
            local_consensus_config,
            onchain_randomness_config,
            onchain_jwk_consensus_config,
            None,
        )
    }

    fn create_nodes_with_validator_set(
        playground: &mut NetworkPlayground,
        executor: Handle,
        num_nodes: usize,
        proposer_indices: Option<Vec<usize>>,
        onchain_consensus_config: Option<OnChainConsensusConfig>,
        local_consensus_config: Option<ConsensusConfig>,
        onchain_randomness_config: Option<OnChainRandomnessConfig>,
        onchain_jwk_consensus_config: Option<OnChainJWKConsensusConfig>,
        validator_set: Option<(Vec<ValidatorSigner>, ValidatorVerifier)>,
    ) -> Vec<Self> {
        let mut onchain_consensus_config = onchain_consensus_config.unwrap_or_default();
        // With order votes feature, the validators additionally send order votes.
        // next_proposal and next_vote functions could potentially break because of it.
        if let OnChainConsensusConfig::V4 {
            alg:
                ConsensusAlgorithmConfig::JolteonV2 {
                    main: _,
                    quorum_store_enabled: _,
                    order_vote_enabled,
                },
            vtxn: _,
            window_size: _,
        } = &mut onchain_consensus_config
        {
            *order_vote_enabled = false;
        }
        let onchain_randomness_config =
            onchain_randomness_config.unwrap_or_else(OnChainRandomnessConfig::default_if_missing);
        let onchain_jwk_consensus_config = onchain_jwk_consensus_config
            .unwrap_or_else(OnChainJWKConsensusConfig::default_if_missing);
        let local_consensus_config = local_consensus_config.unwrap_or_default();
        let (signers, validators) =
            validator_set.unwrap_or_else(|| random_validator_verifier(num_nodes, None, false));
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

        let window_size = onchain_consensus_config.window_size();
        let block_store = Arc::new(BlockStore::new(
            storage.clone(),
            initial_data,
            mock_execution_client.clone(),
            10, // max pruned blocks in mem
            time_service.clone(),
            10,
            Arc::from(DirectMempoolPayloadManager::new()),
            false,
            window_size,
            Arc::new(Mutex::new(PendingBlocks::new())),
            None,
        ));
        let block_store_clone = Arc::clone(&block_store);
        let callback = Box::new(
            move |block_id: HashValue, block_round: Round, commit_proof: WrappedLedgerInfo| {
                block_store_clone.commit_callback(block_id, block_round, commit_proof, None)
            },
        );
        mock_execution_client.set_callback(callback);

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
            Some(30_000),
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

        let (opt_proposal_loopback_tx, opt_proposal_loopback_rx) = aptos_channels::new_unbounded(
            &counters::OP_COUNTERS.gauge("opt_proposal_loopback_queue"),
        );

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
            opt_proposal_loopback_tx,
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
            opt_proposal_queue: VecDeque::new(),
            round_timeout_queue: VecDeque::new(),
            commit_decision_queue: VecDeque::new(),
            processed_opt_proposal_rx: opt_proposal_loopback_rx,
        }
    }

    pub fn restart(self, playground: &mut NetworkPlayground, executor: Handle) -> Self {
        let recover_data = self
            .storage
            .try_start(
                self.onchain_consensus_config.order_vote_enabled(),
                self.onchain_consensus_config.window_size(),
            )
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
            ConsensusMsg::OptProposalMsg(opt_proposal) => {
                self.opt_proposal_queue.push_back(*opt_proposal);
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

    pub async fn next_opt_proposal(&mut self) -> OptProposalMsg {
        while self.opt_proposal_queue.is_empty() {
            self.next_network_message().await;
        }
        self.opt_proposal_queue.pop_front().unwrap()
    }

    pub async fn next_opt_or_normal_proposal(&mut self) -> ProposalMsgType {
        while self.opt_proposal_queue.is_empty() && self.proposal_queue.is_empty() {
            self.next_network_message().await;
        }

        if !self.opt_proposal_queue.is_empty() {
            return ProposalMsgType::Optimistic(self.opt_proposal_queue.pop_front().unwrap());
        }

        ProposalMsgType::Normal(self.proposal_queue.pop_front().unwrap())
    }

    pub async fn next_vote(&mut self) -> VoteMsg {
        while self.vote_queue.is_empty() {
            self.next_network_message().await;
        }
        self.vote_queue.pop_front().unwrap()
    }

    #[allow(unused)]
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

    pub async fn poll_block_retrieval(&mut self) -> Option<IncomingBlockRetrievalRequest> {
        match self.poll_next_network_event() {
            Some(Event::RpcRequest(_, msg, protocol, response_sender)) => match msg {
                ConsensusMsg::DeprecatedBlockRetrievalRequest(v) => {
                    Some(IncomingBlockRetrievalRequest {
                        req: BlockRetrievalRequest::V1(*v),
                        protocol,
                        response_sender,
                    })
                },
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
