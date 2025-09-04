// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

#![allow(clippy::unwrap_used)]

use crate::{
    block_storage::{pending_blocks::PendingBlocks, BlockStore},
    counters,
    liveness::{
        proposal_generator::{
            ChainHealthBackoffConfig, PipelineBackpressureConfig, ProposalGenerator,
        },
        rotating_proposer_election::RotatingProposer,
        round_state::{ExponentialTimeInterval, NewRoundEvent, NewRoundReason, RoundState},
    },
    metrics_safety_rules::MetricsSafetyRules,
    network::NetworkSender,
    network_interface::{ConsensusNetworkClient, DIRECT_SEND, RPC},
    payload_manager::DirectMempoolPayloadManager,
    persistent_liveness_storage::{PersistentLivenessStorage, RecoveryData},
    pipeline::execution_client::DummyExecutionClient,
    round_manager::RoundManager,
    test_utils::{
        MockOptQSPayloadProvider, MockPastProposalStatusTracker, MockPayloadManager, MockStorage,
    },
    util::{mock_time_service::SimulatedTimeService, time_service::TimeService},
};
use velor_channels::{self, velor_channel, message_queues::QueueStyle};
use velor_config::{
    config::{BlockTransactionFilterConfig, ConsensusConfig},
    network_id::NetworkId,
};
use velor_consensus_types::{proposal_msg::ProposalMsg, utils::PayloadTxnsSize};
use velor_infallible::Mutex;
use velor_network::{
    application::{interface::NetworkClient, storage::PeersAndMetadata},
    peer_manager::{ConnectionRequestSender, PeerManagerRequestSender},
    protocols::{network, network::NewNetworkSender},
};
use velor_safety_rules::{test_utils, SafetyRules, TSafetyRules};
use velor_types::{
    aggregate_signature::AggregateSignature,
    epoch_change::EpochChangeProof,
    epoch_state::EpochState,
    ledger_info::{LedgerInfo, LedgerInfoWithSignatures},
    on_chain_config::{
        OnChainConsensusConfig, OnChainJWKConsensusConfig, OnChainRandomnessConfig, ValidatorSet,
        ValidatorTxnConfig, DEFAULT_ENABLED_WINDOW_SIZE,
    },
    validator_info::ValidatorInfo,
    validator_signer::ValidatorSigner,
    validator_verifier::ValidatorVerifier,
};
use futures::{channel::mpsc, executor::block_on};
use maplit::hashmap;
use once_cell::sync::Lazy;
use std::{sync::Arc, time::Duration};
use tokio::runtime::Runtime;

// This generates a proposal for round 1
pub fn generate_corpus_proposal() -> Vec<u8> {
    let round_manager = create_node_for_fuzzing();
    block_on(async {
        let proposal = round_manager
            .generate_proposal_for_test(NewRoundEvent {
                round: 1,
                reason: NewRoundReason::QCReady,
                timeout: std::time::Duration::new(5, 0),
                prev_round_votes: Vec::new(),
                prev_round_timeout_votes: None,
            })
            .await;
        // serialize and return proposal
        serde_json::to_vec(&proposal.unwrap()).unwrap()
    })
}

// optimization for the fuzzer
static STATIC_RUNTIME: Lazy<Runtime> = Lazy::new(|| Runtime::new().unwrap());
static FUZZING_SIGNER: Lazy<ValidatorSigner> = Lazy::new(|| ValidatorSigner::from_int(1));

// helpers
fn build_empty_store(
    storage: Arc<dyn PersistentLivenessStorage>,
    initial_data: RecoveryData,
) -> Arc<BlockStore> {
    let (_commit_cb_sender, _commit_cb_receiver) = mpsc::unbounded::<LedgerInfoWithSignatures>();

    Arc::new(BlockStore::new(
        storage,
        initial_data,
        Arc::new(DummyExecutionClient),
        10, // max pruned blocks in mem
        Arc::new(SimulatedTimeService::new()),
        10,
        Arc::from(DirectMempoolPayloadManager::new()),
        false,
        DEFAULT_ENABLED_WINDOW_SIZE,
        Arc::new(Mutex::new(PendingBlocks::new())),
        None,
    ))
}

// helpers for safety rule initialization
fn make_initial_epoch_change_proof(signer: &ValidatorSigner) -> EpochChangeProof {
    let validator_info =
        ValidatorInfo::new_with_test_network_keys(signer.author(), signer.public_key(), 1, 0);
    let validator_set = ValidatorSet::new(vec![validator_info]);
    let li = LedgerInfo::mock_genesis(Some(validator_set));
    let lis = LedgerInfoWithSignatures::new(li, AggregateSignature::empty());
    EpochChangeProof::new(vec![lis], false)
}

// TODO: MockStorage -> EmptyStorage
fn create_round_state() -> RoundState {
    let base_timeout = std::time::Duration::new(60, 0);
    let time_interval = Box::new(ExponentialTimeInterval::fixed(base_timeout));
    let (round_timeout_sender, _) = velor_channels::new_test(1_024);
    let time_service = Arc::new(SimulatedTimeService::new());

    RoundState::new(time_interval, time_service, round_timeout_sender)
}

// Creates an RoundManager for fuzzing
fn create_node_for_fuzzing() -> RoundManager {
    // signer is re-used accross fuzzing runs
    let signer = FUZZING_SIGNER.clone();

    // TODO: remove
    let validator = ValidatorVerifier::new_single(signer.author(), signer.public_key());
    let validator_set = (&validator).into();

    // TODO: EmptyStorage
    let (initial_data, storage) = MockStorage::start_for_testing(validator_set);

    // TODO: remove
    let proof = make_initial_epoch_change_proof(&signer);
    let mut safety_rules = SafetyRules::new(test_utils::test_storage(&signer), false);
    safety_rules.initialize(&proof).unwrap();

    // TODO: mock channels
    let (network_reqs_tx, _network_reqs_rx) = velor_channel::new(QueueStyle::FIFO, 8, None);
    let (connection_reqs_tx, _) = velor_channel::new(QueueStyle::FIFO, 8, None);
    let network_sender = network::NetworkSender::new(
        PeerManagerRequestSender::new(network_reqs_tx),
        ConnectionRequestSender::new(connection_reqs_tx),
    );
    let network_client = NetworkClient::new(
        DIRECT_SEND.into(),
        RPC.into(),
        hashmap! {NetworkId::Validator => network_sender},
        PeersAndMetadata::new(&[NetworkId::Validator]),
    );
    let consensus_network_client = ConsensusNetworkClient::new(network_client);

    let (self_sender, _self_receiver) = velor_channels::new_unbounded_test();

    let epoch_state = Arc::new(EpochState::new(1, storage.get_validator_set().into()));
    let network = Arc::new(NetworkSender::new(
        signer.author(),
        consensus_network_client,
        self_sender,
        epoch_state.verifier.clone(),
    ));

    // TODO: mock
    let block_store = build_empty_store(storage.clone(), initial_data);

    // TODO: remove
    let time_service = Arc::new(SimulatedTimeService::new());
    block_on(time_service.sleep(Duration::from_millis(1)));

    // TODO: remove
    let proposal_generator = ProposalGenerator::new(
        signer.author(),
        block_store.clone(),
        Arc::new(MockPayloadManager::new(None)),
        time_service,
        Duration::ZERO,
        PayloadTxnsSize::new(1, 1024),
        1,
        PayloadTxnsSize::new(1, 1024),
        10,
        1,
        Some(30_000),
        PipelineBackpressureConfig::new_no_backoff(),
        ChainHealthBackoffConfig::new_no_backoff(),
        false,
        ValidatorTxnConfig::default_disabled(),
        true,
        Arc::new(MockOptQSPayloadProvider {}),
    );

    //
    let round_state = create_round_state();

    // TODO: have two different nodes, one for proposing, one for accepting a proposal
    let proposer_election = Arc::new(RotatingProposer::new(vec![signer.author()], 1));

    let (round_manager_tx, _) = velor_channel::new(QueueStyle::LIFO, 1, None);

    let (opt_proposal_loopback_tx, _) =
        velor_channels::new_unbounded(&counters::OP_COUNTERS.gauge("opt_proposal_loopback_queue"));

    // event processor
    RoundManager::new(
        epoch_state,
        Arc::clone(&block_store),
        round_state,
        proposer_election,
        proposal_generator,
        Arc::new(Mutex::new(MetricsSafetyRules::new(
            Box::new(safety_rules),
            storage.clone(),
        ))),
        network,
        storage,
        OnChainConsensusConfig::default(),
        round_manager_tx,
        BlockTransactionFilterConfig::default(),
        ConsensusConfig::default(),
        OnChainRandomnessConfig::default_enabled(),
        OnChainJWKConsensusConfig::default_enabled(),
        None,
        Arc::new(MockPastProposalStatusTracker {}),
        opt_proposal_loopback_tx,
    )
}

// This functions fuzzes a Proposal protobuffer (not a ConsensusMsg)
pub fn fuzz_proposal(data: &[u8]) {
    // create node
    let mut round_manager = create_node_for_fuzzing();

    let proposal: ProposalMsg = match serde_json::from_slice(data) {
        Ok(xx) => xx,
        Err(_) => {
            if cfg!(test) {
                panic!();
            }
            return;
        },
    };

    let proposal = match proposal.verify_well_formed() {
        Ok(_) => proposal,
        Err(e) => {
            println!("{:?}", e);
            if cfg!(test) {
                panic!();
            }
            return;
        },
    };

    block_on(async move {
        // TODO: make sure this obtains a vote when testing
        // TODO: make sure that if this obtains a vote, it's for round 1, etc.
        let _ = round_manager.process_proposal_msg(proposal).await;
    });
}

// This test is here so that the fuzzer can be maintained
#[test]
fn test_consensus_proposal_fuzzer() {
    // generate a proposal
    let proposal = generate_corpus_proposal();
    // successfully parse it
    fuzz_proposal(&proposal);
}
