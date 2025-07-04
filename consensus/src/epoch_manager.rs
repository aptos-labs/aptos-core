// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    block_storage::{
        pending_blocks::PendingBlocks,
        tracing::{observe_block, BlockStage},
        BlockStore,
    },
    consensus_observer::publisher::consensus_publisher::ConsensusPublisher,
    counters,
    dag::{DagBootstrapper, DagCommitSigner, StorageAdapter},
    error::{error_kind, DbError},
    liveness::{
        cached_proposer_election::CachedProposerElection,
        leader_reputation::{
            extract_epoch_to_proposers, AptosDBBackend, LeaderReputation,
            ProposerAndVoterHeuristic, ReputationHeuristic,
        },
        proposal_generator::{
            ChainHealthBackoffConfig, PipelineBackpressureConfig, ProposalGenerator,
        },
        proposal_status_tracker::{ExponentialWindowFailureTracker, OptQSPullParamsProvider},
        proposer_election::ProposerElection,
        rotating_proposer_election::{choose_leader, RotatingProposer},
        round_proposer_election::RoundProposer,
        round_state::{ExponentialTimeInterval, RoundState},
    },
    logging::{LogEvent, LogSchema},
    metrics_safety_rules::MetricsSafetyRules,
    monitor,
    network::{
        DeprecatedIncomingBlockRetrievalRequest, IncomingBatchRetrievalRequest,
        IncomingBlockRetrievalRequest, IncomingDAGRequest, IncomingRandGenRequest,
        IncomingRpcRequest, NetworkReceivers, NetworkSender,
    },
    network_interface::{ConsensusMsg, ConsensusNetworkClient},
    payload_client::{
        mixed::MixedPayloadClient, user::quorum_store_client::QuorumStoreClient, PayloadClient,
    },
    payload_manager::{DirectMempoolPayloadManager, TPayloadManager},
    persistent_liveness_storage::{LedgerRecoveryData, PersistentLivenessStorage, RecoveryData},
    pipeline::execution_client::TExecutionClient,
    quorum_store::{
        quorum_store_builder::{DirectMempoolInnerBuilder, InnerBuilder, QuorumStoreBuilder},
        quorum_store_coordinator::CoordinatorCommand,
        quorum_store_db::QuorumStoreStorage,
    },
    rand::rand_gen::{
        storage::interface::RandStorage,
        types::{AugmentedData, RandConfig},
    },
    recovery_manager::RecoveryManager,
    round_manager::{RoundManager, UnverifiedEvent, VerifiedEvent},
    util::time_service::TimeService,
};
use anyhow::{anyhow, bail, ensure, Context};
use aptos_bounded_executor::BoundedExecutor;
use aptos_channels::{aptos_channel, message_queues::QueueStyle};
use aptos_config::config::{ConsensusConfig, DagConsensusConfig, ExecutionConfig, NodeConfig};
use aptos_consensus_types::{
    block_retrieval::BlockRetrievalRequest,
    common::{Author, Round},
    epoch_retrieval::EpochRetrievalRequest,
    proof_of_store::ProofCache,
    utils::PayloadTxnsSize,
};
use aptos_crypto::bls12381::PrivateKey;
use aptos_dkg::{
    pvss::{traits::Transcript, Player},
    weighted_vuf::traits::WeightedVUF,
};
use aptos_event_notifications::ReconfigNotificationListener;
use aptos_infallible::{duration_since_epoch, Mutex};
use aptos_logger::prelude::*;
use aptos_mempool::QuorumStoreRequest;
use aptos_network::{application::interface::NetworkClient, protocols::network::Event};
use aptos_safety_rules::{
    safety_rules_manager, Error, PersistentSafetyStorage, SafetyRulesManager,
};
use aptos_types::{
    account_address::AccountAddress,
    dkg::{real_dkg::maybe_dk_from_bls_sk, DKGState, DKGTrait, DefaultDKG},
    epoch_change::EpochChangeProof,
    epoch_state::EpochState,
    jwks::SupportedOIDCProviders,
    on_chain_config::{
        Features, LeaderReputationType, OnChainConfigPayload, OnChainConfigProvider,
        OnChainConsensusConfig, OnChainExecutionConfig, OnChainJWKConsensusConfig,
        OnChainRandomnessConfig, ProposerElectionType, RandomnessConfigMoveStruct,
        RandomnessConfigSeqNum, ValidatorSet,
    },
    randomness::{RandKeys, WvufPP, WVUF},
    validator_signer::ValidatorSigner,
    validator_verifier::ValidatorVerifier,
};
use aptos_validator_transaction_pool::VTxnPoolState;
use fail::fail_point;
use futures::{
    channel::{mpsc, mpsc::Sender, oneshot},
    SinkExt, StreamExt,
};
use itertools::Itertools;
use mini_moka::sync::Cache;
use rand::{prelude::StdRng, thread_rng, SeedableRng};
use std::{
    cmp::Ordering,
    collections::HashMap,
    hash::Hash,
    mem::{discriminant, Discriminant},
    sync::Arc,
    time::Duration,
};

#[cfg(feature = "consensus_fuzzer")]
use crate::rapture_hook::{StateModelLike, run_fuzzer};
#[cfg(feature = "consensus_fuzzer")]
use std::sync::OnceLock;
#[cfg(feature = "consensus_fuzzer")]
static GLOBAL_STATE_MODEL: OnceLock<Arc<Mutex<Box<dyn StateModelLike>>>> = OnceLock::new();

/// Range of rounds (window) that we might be calling proposer election
/// functions with at any given time, in addition to the proposer history length.
const PROPOSER_ELECTION_CACHING_WINDOW_ADDITION: usize = 3;
/// Number of rounds we expect storage to be ahead of the proposer round,
/// used for fetching data from DB.
const PROPOSER_ROUND_BEHIND_STORAGE_BUFFER: usize = 10;

#[allow(clippy::large_enum_variant)]
pub enum LivenessStorageData {
    FullRecoveryData(RecoveryData),
    PartialRecoveryData(LedgerRecoveryData),
}

// Manager the components that shared across epoch and spawn per-epoch RoundManager with
// epoch-specific input.
pub struct EpochManager<P: OnChainConfigProvider> {
    author: Author,
    config: ConsensusConfig,
    #[allow(unused)]
    execution_config: ExecutionConfig,
    randomness_override_seq_num: u64,
    time_service: Arc<dyn TimeService>,
    self_sender: aptos_channels::UnboundedSender<Event<ConsensusMsg>>,
    network_sender: ConsensusNetworkClient<NetworkClient<ConsensusMsg>>,
    timeout_sender: aptos_channels::Sender<Round>,
    quorum_store_enabled: bool,
    quorum_store_to_mempool_sender: Sender<QuorumStoreRequest>,
    execution_client: Arc<dyn TExecutionClient>,
    storage: Arc<dyn PersistentLivenessStorage>,
    safety_rules_manager: SafetyRulesManager,
    vtxn_pool: VTxnPoolState,
    reconfig_events: ReconfigNotificationListener<P>,
    // channels to rand manager
    rand_manager_msg_tx: Option<aptos_channel::Sender<AccountAddress, IncomingRandGenRequest>>,
    // channels to round manager
    round_manager_tx: Option<
        aptos_channel::Sender<(Author, Discriminant<VerifiedEvent>), (Author, VerifiedEvent)>,
    >,
    buffered_proposal_tx: Option<aptos_channel::Sender<Author, VerifiedEvent>>,
    round_manager_close_tx: Option<oneshot::Sender<oneshot::Sender<()>>>,
    epoch_state: Option<Arc<EpochState>>,
    block_retrieval_tx:
        Option<aptos_channel::Sender<AccountAddress, IncomingBlockRetrievalRequest>>,
    quorum_store_msg_tx: Option<aptos_channel::Sender<AccountAddress, (Author, VerifiedEvent)>>,
    quorum_store_coordinator_tx: Option<Sender<CoordinatorCommand>>,
    quorum_store_storage: Arc<dyn QuorumStoreStorage>,
    batch_retrieval_tx:
        Option<aptos_channel::Sender<AccountAddress, IncomingBatchRetrievalRequest>>,
    bounded_executor: BoundedExecutor,
    // recovery_mode is set to true when the recovery manager is spawned
    recovery_mode: bool,

    aptos_time_service: aptos_time_service::TimeService,
    dag_rpc_tx: Option<aptos_channel::Sender<AccountAddress, IncomingDAGRequest>>,
    dag_shutdown_tx: Option<oneshot::Sender<oneshot::Sender<()>>>,
    dag_config: DagConsensusConfig,
    payload_manager: Arc<dyn TPayloadManager>,
    rand_storage: Arc<dyn RandStorage<AugmentedData>>,
    proof_cache: ProofCache,
    consensus_publisher: Option<Arc<ConsensusPublisher>>,
    pending_blocks: Arc<Mutex<PendingBlocks>>,
    key_storage: PersistentSafetyStorage,
}

impl<P: OnChainConfigProvider> EpochManager<P> {
    #[allow(clippy::too_many_arguments, clippy::unwrap_used)]
    pub(crate) fn new(
        node_config: &NodeConfig,
        time_service: Arc<dyn TimeService>,
        self_sender: aptos_channels::UnboundedSender<Event<ConsensusMsg>>,
        network_sender: ConsensusNetworkClient<NetworkClient<ConsensusMsg>>,
        timeout_sender: aptos_channels::Sender<Round>,
        quorum_store_to_mempool_sender: Sender<QuorumStoreRequest>,
        execution_client: Arc<dyn TExecutionClient>,
        storage: Arc<dyn PersistentLivenessStorage>,
        quorum_store_storage: Arc<dyn QuorumStoreStorage>,
        reconfig_events: ReconfigNotificationListener<P>,
        bounded_executor: BoundedExecutor,
        aptos_time_service: aptos_time_service::TimeService,
        vtxn_pool: VTxnPoolState,
        rand_storage: Arc<dyn RandStorage<AugmentedData>>,
        consensus_publisher: Option<Arc<ConsensusPublisher>>,
    ) -> Self {
        let author = node_config.validator_network.as_ref().unwrap().peer_id();
        let config = node_config.consensus.clone();
        let execution_config = node_config.execution.clone();
        let dag_config = node_config.dag_consensus.clone();
        let sr_config = &node_config.consensus.safety_rules;
        let safety_rules_manager = SafetyRulesManager::new(sr_config);
        let key_storage = safety_rules_manager::storage(sr_config);
        Self {
            author,
            config,
            execution_config,
            randomness_override_seq_num: node_config.randomness_override_seq_num,
            time_service,
            self_sender,
            network_sender,
            timeout_sender,
            // This default value is updated at epoch start
            quorum_store_enabled: false,
            quorum_store_to_mempool_sender,
            execution_client,
            storage,
            safety_rules_manager,
            vtxn_pool,
            reconfig_events,
            rand_manager_msg_tx: None,
            round_manager_tx: None,
            round_manager_close_tx: None,
            buffered_proposal_tx: None,
            epoch_state: None,
            block_retrieval_tx: None,
            quorum_store_msg_tx: None,
            quorum_store_coordinator_tx: None,
            quorum_store_storage,
            batch_retrieval_tx: None,
            bounded_executor,
            recovery_mode: false,
            dag_rpc_tx: None,
            dag_shutdown_tx: None,
            aptos_time_service,
            dag_config,
            payload_manager: Arc::new(DirectMempoolPayloadManager::new()),
            rand_storage,
            proof_cache: Cache::builder()
                .max_capacity(node_config.consensus.proof_cache_capacity)
                .initial_capacity(1_000)
                .time_to_live(Duration::from_secs(20))
                .build(),
            consensus_publisher,
            pending_blocks: Arc::new(Mutex::new(PendingBlocks::new())),
            key_storage,
        }
    }

    fn epoch_state(&self) -> &EpochState {
        self.epoch_state
            .as_ref()
            .expect("EpochManager not started yet")
    }

    fn epoch(&self) -> u64 {
        self.epoch_state().epoch
    }

    fn create_round_state(
        &self,
        time_service: Arc<dyn TimeService>,
        timeout_sender: aptos_channels::Sender<Round>,
    ) -> RoundState {
        let time_interval = Box::new(ExponentialTimeInterval::new(
            Duration::from_millis(self.config.round_initial_timeout_ms),
            self.config.round_timeout_backoff_exponent_base,
            self.config.round_timeout_backoff_max_exponent,
        ));
        RoundState::new(time_interval, time_service, timeout_sender)
    }

    /// Create a proposer election handler based on proposers
    fn create_proposer_election(
        &self,
        epoch_state: &EpochState,
        onchain_config: &OnChainConsensusConfig,
    ) -> Arc<dyn ProposerElection + Send + Sync> {
        let proposers = epoch_state
            .verifier
            .get_ordered_account_addresses_iter()
            .collect::<Vec<_>>();
        match &onchain_config.proposer_election_type() {
            ProposerElectionType::RotatingProposer(contiguous_rounds) => {
                Arc::new(RotatingProposer::new(proposers, *contiguous_rounds))
            },
            // We don't really have a fixed proposer!
            ProposerElectionType::FixedProposer(contiguous_rounds) => {
                let proposer = choose_leader(proposers);
                Arc::new(RotatingProposer::new(vec![proposer], *contiguous_rounds))
            },
            ProposerElectionType::LeaderReputation(leader_reputation_type) => {
                let (
                    heuristic,
                    window_size,
                    weight_by_voting_power,
                    use_history_from_previous_epoch_max_count,
                ) = match &leader_reputation_type {
                    LeaderReputationType::ProposerAndVoter(proposer_and_voter_config)
                    | LeaderReputationType::ProposerAndVoterV2(proposer_and_voter_config) => {
                        let proposer_window_size = proposers.len()
                            * proposer_and_voter_config.proposer_window_num_validators_multiplier;
                        let voter_window_size = proposers.len()
                            * proposer_and_voter_config.voter_window_num_validators_multiplier;
                        let heuristic: Box<dyn ReputationHeuristic> =
                            Box::new(ProposerAndVoterHeuristic::new(
                                self.author,
                                proposer_and_voter_config.active_weight,
                                proposer_and_voter_config.inactive_weight,
                                proposer_and_voter_config.failed_weight,
                                proposer_and_voter_config.failure_threshold_percent,
                                voter_window_size,
                                proposer_window_size,
                                leader_reputation_type.use_reputation_window_from_stale_end(),
                            ));
                        (
                            heuristic,
                            std::cmp::max(proposer_window_size, voter_window_size),
                            proposer_and_voter_config.weight_by_voting_power,
                            proposer_and_voter_config.use_history_from_previous_epoch_max_count,
                        )
                    },
                };

                let seek_len = onchain_config.leader_reputation_exclude_round() as usize
                    + onchain_config.max_failed_authors_to_store()
                    + PROPOSER_ROUND_BEHIND_STORAGE_BUFFER;

                let backend = Arc::new(AptosDBBackend::new(
                    window_size,
                    seek_len,
                    self.storage.aptos_db(),
                ));
                let voting_powers: Vec<_> = if weight_by_voting_power {
                    proposers
                        .iter()
                        .map(|p| {
                            epoch_state
                                .verifier
                                .get_voting_power(p)
                                .expect("INVARIANT VIOLATION: proposer not in verifier set")
                        })
                        .collect()
                } else {
                    vec![1; proposers.len()]
                };

                let epoch_to_proposers = self.extract_epoch_proposers(
                    epoch_state,
                    use_history_from_previous_epoch_max_count,
                    proposers,
                    (window_size + seek_len) as u64,
                );

                info!(
                    "Starting epoch {}: proposers across epochs for leader election: {:?}",
                    epoch_state.epoch,
                    epoch_to_proposers
                        .iter()
                        .map(|(epoch, proposers)| (epoch, proposers.len()))
                        .sorted()
                        .collect::<Vec<_>>()
                );

                let proposer_election = Box::new(LeaderReputation::new(
                    epoch_state.epoch,
                    epoch_to_proposers,
                    voting_powers,
                    backend,
                    heuristic,
                    onchain_config.leader_reputation_exclude_round(),
                    leader_reputation_type.use_root_hash_for_seed(),
                    self.config.window_for_chain_health,
                ));
                // LeaderReputation is not cheap, so we can cache the amount of rounds round_manager needs.
                Arc::new(CachedProposerElection::new(
                    epoch_state.epoch,
                    proposer_election,
                    onchain_config.max_failed_authors_to_store()
                        + PROPOSER_ELECTION_CACHING_WINDOW_ADDITION,
                ))
            },
            ProposerElectionType::RoundProposer(round_proposers) => {
                // Hardcoded to the first proposer
                let default_proposer = proposers
                    .first()
                    .expect("INVARIANT VIOLATION: proposers is empty");
                Arc::new(RoundProposer::new(
                    round_proposers.clone(),
                    *default_proposer,
                ))
            },
        }
    }

    fn extract_epoch_proposers(
        &self,
        epoch_state: &EpochState,
        use_history_from_previous_epoch_max_count: u32,
        proposers: Vec<AccountAddress>,
        needed_rounds: u64,
    ) -> HashMap<u64, Vec<AccountAddress>> {
        // Genesis is epoch=0
        // First block (after genesis) is epoch=1, and is the only block in that epoch.
        // It has no votes, so we skip it unless we are in epoch 1, as otherwise it will
        // skew leader elections for exclude_round number of rounds.
        let first_epoch_to_consider = std::cmp::max(
            if epoch_state.epoch == 1 { 1 } else { 2 },
            epoch_state
                .epoch
                .saturating_sub(use_history_from_previous_epoch_max_count as u64),
        );
        // If we are considering beyond the current epoch, we need to fetch validators for those epochs
        if epoch_state.epoch > first_epoch_to_consider {
            self.storage
                .aptos_db()
                .get_epoch_ending_ledger_infos(first_epoch_to_consider - 1, epoch_state.epoch)
                .map_err(Into::into)
                .and_then(|proof| {
                    ensure!(
                        proof.ledger_info_with_sigs.len() as u64
                            == (epoch_state.epoch - (first_epoch_to_consider - 1))
                    );
                    extract_epoch_to_proposers(proof, epoch_state.epoch, &proposers, needed_rounds)
                })
                .unwrap_or_else(|err| {
                    error!(
                        "Couldn't create leader reputation with history across epochs, {:?}",
                        err
                    );
                    HashMap::from([(epoch_state.epoch, proposers)])
                })
        } else {
            HashMap::from([(epoch_state.epoch, proposers)])
        }
    }

    fn process_epoch_retrieval(
        &mut self,
        request: EpochRetrievalRequest,
        peer_id: AccountAddress,
    ) -> anyhow::Result<()> {
        debug!(
            LogSchema::new(LogEvent::ReceiveEpochRetrieval)
                .remote_peer(peer_id)
                .epoch(self.epoch()),
            "[EpochManager] receive {}", request,
        );
        let proof = self
            .storage
            .aptos_db()
            .get_epoch_ending_ledger_infos(request.start_epoch, request.end_epoch)
            .map_err(DbError::from)
            .context("[EpochManager] Failed to get epoch proof")?;
        let msg = ConsensusMsg::EpochChangeProof(Box::new(proof));
        if let Err(err) = self.network_sender.send_to(peer_id, msg) {
            warn!(
                "[EpochManager] Failed to send epoch proof to {}, with error: {:?}",
                peer_id, err,
            );
        }
        Ok(())
    }

    fn process_different_epoch(
        &mut self,
        different_epoch: u64,
        peer_id: AccountAddress,
    ) -> anyhow::Result<()> {
        debug!(
            LogSchema::new(LogEvent::ReceiveMessageFromDifferentEpoch)
                .remote_peer(peer_id)
                .epoch(self.epoch()),
            remote_epoch = different_epoch,
        );
        match different_epoch.cmp(&self.epoch()) {
            Ordering::Less => {
                if self
                    .epoch_state()
                    .verifier
                    .get_voting_power(&self.author)
                    .is_some()
                {
                    // Ignore message from lower epoch if we're part of the validator set, the node would eventually see messages from
                    // higher epoch and request a proof
                    sample!(
                        SampleRate::Duration(Duration::from_secs(1)),
                        debug!("Discard message from lower epoch {} from {}", different_epoch, peer_id);
                    );
                    Ok(())
                } else {
                    // reply back the epoch change proof if we're not part of the validator set since we won't broadcast
                    // timeout in this epoch
                    monitor!(
                        "process_epoch_retrieval",
                        self.process_epoch_retrieval(
                            EpochRetrievalRequest {
                                start_epoch: different_epoch,
                                end_epoch: self.epoch(),
                            },
                            peer_id
                        )
                    )
                }
            },
            // We request proof to join higher epoch
            Ordering::Greater => {
                let request = EpochRetrievalRequest {
                    start_epoch: self.epoch(),
                    end_epoch: different_epoch,
                };
                let msg = ConsensusMsg::EpochRetrievalRequest(Box::new(request));
                if let Err(err) = self.network_sender.send_to(peer_id, msg) {
                    warn!(
                        "[EpochManager] Failed to send epoch retrieval to {}, {:?}",
                        peer_id, err
                    );
                    counters::EPOCH_MANAGER_ISSUES_DETAILS
                        .with_label_values(&["failed_to_send_epoch_retrieval"])
                        .inc();
                }

                Ok(())
            },
            Ordering::Equal => {
                bail!("[EpochManager] Same epoch should not come to process_different_epoch");
            },
        }
    }

    async fn initiate_new_epoch(&mut self, proof: EpochChangeProof) -> anyhow::Result<()> {
        let ledger_info = proof
            .verify(self.epoch_state())
            .context("[EpochManager] Invalid EpochChangeProof")?;
        info!(
            LogSchema::new(LogEvent::NewEpoch).epoch(ledger_info.ledger_info().next_block_epoch()),
            "Received verified epoch change",
        );

        // shutdown existing processor first to avoid race condition with state sync.
        self.shutdown_current_processor().await;
        *self.pending_blocks.lock() = PendingBlocks::new();
        // make sure storage is on this ledger_info too, it should be no-op if it's already committed
        // panic if this doesn't succeed since the current processors are already shutdown.
        self.execution_client
            .sync_to_target(ledger_info.clone())
            .await
            .context(format!(
                "[EpochManager] State sync to new epoch {}",
                ledger_info
            ))
            .expect("Failed to sync to new epoch");

        monitor!("reconfig", self.await_reconfig_notification().await);
        Ok(())
    }

    fn spawn_block_retrieval_task(
        &mut self,
        epoch: u64,
        block_store: Arc<BlockStore>,
        max_blocks_allowed: u64,
    ) {
        let (request_tx, mut request_rx) = aptos_channel::new::<_, IncomingBlockRetrievalRequest>(
            QueueStyle::KLAST,
            10,
            Some(&counters::BLOCK_RETRIEVAL_TASK_MSGS),
        );
        let task = async move {
            info!(epoch = epoch, "Block retrieval task starts");
            while let Some(request) = request_rx.next().await {
                match request.req {
                    // TODO @bchocho @hariria deprecate once BlockRetrievalRequest enum release is complete
                    BlockRetrievalRequest::V1(v1) => {
                        if v1.num_blocks() > max_blocks_allowed {
                            warn!(
                                "Ignore block retrieval with too many blocks: {}",
                                v1.num_blocks()
                            );
                            continue;
                        }
                        if let Err(e) = monitor!(
                            "process_block_retrieval",
                            block_store
                                .process_block_retrieval(IncomingBlockRetrievalRequest {
                                    req: BlockRetrievalRequest::V1(v1),
                                    protocol: request.protocol,
                                    response_sender: request.response_sender,
                                })
                                .await
                        ) {
                            warn!(epoch = epoch, error = ?e, kind = error_kind(&e));
                        }
                    },
                    BlockRetrievalRequest::V2(v2) => {
                        if v2.num_blocks() > max_blocks_allowed {
                            warn!(
                                "Ignore block retrieval with too many blocks: {}",
                                v2.num_blocks()
                            );
                            continue;
                        }
                        if let Err(e) = monitor!(
                            "process_block_retrieval_v2",
                            block_store
                                .process_block_retrieval(IncomingBlockRetrievalRequest {
                                    req: BlockRetrievalRequest::V2(v2),
                                    protocol: request.protocol,
                                    response_sender: request.response_sender,
                                })
                                .await
                        ) {
                            warn!(epoch = epoch, error = ?e, kind = error_kind(&e));
                        }
                    },
                }
            }
            info!(epoch = epoch, "Block retrieval task stops");
        };
        self.block_retrieval_tx = Some(request_tx);
        tokio::spawn(task);
    }

    async fn shutdown_current_processor(&mut self) {
        if let Some(close_tx) = self.round_manager_close_tx.take() {
            // Release the previous RoundManager, especially the SafetyRule client
            let (ack_tx, ack_rx) = oneshot::channel();
            close_tx
                .send(ack_tx)
                .expect("[EpochManager] Fail to drop round manager");
            ack_rx
                .await
                .expect("[EpochManager] Fail to drop round manager");
        }
        self.round_manager_tx = None;

        if let Some(close_tx) = self.dag_shutdown_tx.take() {
            // Release the previous RoundManager, especially the SafetyRule client
            let (ack_tx, ack_rx) = oneshot::channel();
            close_tx
                .send(ack_tx)
                .expect("[EpochManager] Fail to drop DAG bootstrapper");
            ack_rx
                .await
                .expect("[EpochManager] Fail to drop DAG bootstrapper");
        }
        self.dag_shutdown_tx = None;

        // Shutdown the previous rand manager
        self.rand_manager_msg_tx = None;

        // Shutdown the previous buffer manager, to release the SafetyRule client
        self.execution_client.end_epoch().await;

        // Shutdown the block retrieval task by dropping the sender
        self.block_retrieval_tx = None;
        self.batch_retrieval_tx = None;

        if let Some(mut quorum_store_coordinator_tx) = self.quorum_store_coordinator_tx.take() {
            let (ack_tx, ack_rx) = oneshot::channel();
            quorum_store_coordinator_tx
                .send(CoordinatorCommand::Shutdown(ack_tx))
                .await
                .expect("Could not send shutdown indicator to QuorumStore");
            ack_rx.await.expect("Failed to stop QuorumStore");
        }
    }

    async fn start_recovery_manager(
        &mut self,
        ledger_data: LedgerRecoveryData,
        onchain_consensus_config: OnChainConsensusConfig,
        epoch_state: Arc<EpochState>,
        network_sender: Arc<NetworkSender>,
    ) {
        let (recovery_manager_tx, recovery_manager_rx) = aptos_channel::new(
            QueueStyle::KLAST,
            10,
            Some(&counters::ROUND_MANAGER_CHANNEL_MSGS),
        );
        self.round_manager_tx = Some(recovery_manager_tx);
        let (close_tx, close_rx) = oneshot::channel();
        self.round_manager_close_tx = Some(close_tx);
        let recovery_manager = RecoveryManager::new(
            epoch_state,
            network_sender,
            self.storage.clone(),
            self.execution_client.clone(),
            ledger_data.committed_round(),
            self.config
                .max_blocks_per_sending_request(onchain_consensus_config.quorum_store_enabled()),
            self.payload_manager.clone(),
            onchain_consensus_config.order_vote_enabled(),
            onchain_consensus_config.window_size(),
            self.pending_blocks.clone(),
        );
        tokio::spawn(recovery_manager.start(recovery_manager_rx, close_rx));
    }

    async fn init_payload_provider(
        &mut self,
        epoch_state: &EpochState,
        network_sender: NetworkSender,
        consensus_config: &OnChainConsensusConfig,
        consensus_key: Arc<PrivateKey>,
    ) -> (
        Arc<dyn TPayloadManager>,
        QuorumStoreClient,
        QuorumStoreBuilder,
    ) {
        // Start QuorumStore
        let (consensus_to_quorum_store_tx, consensus_to_quorum_store_rx) =
            mpsc::channel(self.config.intra_consensus_channel_buffer_size);

        let quorum_store_config = if consensus_config.is_dag_enabled() {
            self.dag_config.quorum_store.clone()
        } else {
            self.config.quorum_store.clone()
        };

        let mut quorum_store_builder = if self.quorum_store_enabled {
            info!("Building QuorumStore");
            QuorumStoreBuilder::QuorumStore(InnerBuilder::new(
                self.epoch(),
                self.author,
                epoch_state.verifier.len() as u64,
                quorum_store_config,
                consensus_to_quorum_store_rx,
                self.quorum_store_to_mempool_sender.clone(),
                self.config.mempool_txn_pull_timeout_ms,
                self.storage.aptos_db().clone(),
                network_sender,
                epoch_state.verifier.clone(),
                self.proof_cache.clone(),
                self.quorum_store_storage.clone(),
                !consensus_config.is_dag_enabled(),
                consensus_key,
            ))
        } else {
            info!("Building DirectMempool");
            QuorumStoreBuilder::DirectMempool(DirectMempoolInnerBuilder::new(
                consensus_to_quorum_store_rx,
                self.quorum_store_to_mempool_sender.clone(),
                self.config.mempool_txn_pull_timeout_ms,
            ))
        };

        let (payload_manager, quorum_store_msg_tx) =
            quorum_store_builder.init_payload_manager(self.consensus_publisher.clone());
        self.quorum_store_msg_tx = quorum_store_msg_tx;
        self.payload_manager = payload_manager.clone();

        let payload_client = QuorumStoreClient::new(
            consensus_to_quorum_store_tx,
            self.config.quorum_store_pull_timeout_ms,
            self.config.wait_for_full_blocks_above_recent_fill_threshold,
            self.config.wait_for_full_blocks_above_pending_blocks,
        );
        (payload_manager, payload_client, quorum_store_builder)
    }

    fn set_epoch_start_metrics(&self, epoch_state: &EpochState) {
        counters::EPOCH.set(epoch_state.epoch as i64);
        counters::CURRENT_EPOCH_VALIDATORS.set(epoch_state.verifier.len() as i64);

        counters::TOTAL_VOTING_POWER.set(epoch_state.verifier.total_voting_power() as f64);
        counters::VALIDATOR_VOTING_POWER.set(
            epoch_state
                .verifier
                .get_voting_power(&self.author)
                .unwrap_or(0) as f64,
        );
        epoch_state
            .verifier
            .get_ordered_account_addresses_iter()
            .for_each(|peer_id| {
                counters::ALL_VALIDATORS_VOTING_POWER
                    .with_label_values(&[&peer_id.to_string()])
                    .set(epoch_state.verifier.get_voting_power(&peer_id).unwrap_or(0) as i64)
            });
    }

    async fn start_round_manager(
        &mut self,
        consensus_key: Arc<PrivateKey>,
        recovery_data: RecoveryData,
        epoch_state: Arc<EpochState>,
        onchain_consensus_config: OnChainConsensusConfig,
        onchain_execution_config: OnChainExecutionConfig,
        onchain_randomness_config: OnChainRandomnessConfig,
        onchain_jwk_consensus_config: OnChainJWKConsensusConfig,
        network_sender: Arc<NetworkSender>,
        payload_client: Arc<dyn PayloadClient>,
        payload_manager: Arc<dyn TPayloadManager>,
        rand_config: Option<RandConfig>,
        fast_rand_config: Option<RandConfig>,
        rand_msg_rx: aptos_channel::Receiver<AccountAddress, IncomingRandGenRequest>,
    ) {
        let epoch = epoch_state.epoch;
        info!(
            epoch = epoch_state.epoch,
            validators = epoch_state.verifier.to_string(),
            root_block = %recovery_data.commit_root_block(),
            "Starting new epoch",
        );

        info!(epoch = epoch, "Update SafetyRules");

        let mut safety_rules =
            MetricsSafetyRules::new(self.safety_rules_manager.client(), self.storage.clone());
        match safety_rules.perform_initialize() {
            Err(e) if matches!(e, Error::ValidatorNotInSet(_)) => {
                warn!(
                    epoch = epoch,
                    error = e,
                    "Unable to initialize safety rules.",
                );
            },
            Err(e) => {
                error!(
                    epoch = epoch,
                    error = e,
                    "Unable to initialize safety rules.",
                );
            },
            Ok(()) => (),
        }

        info!(epoch = epoch, "Create RoundState");
        let round_state =
            self.create_round_state(self.time_service.clone(), self.timeout_sender.clone());

        info!(epoch = epoch, "Create ProposerElection");
        let proposer_election =
            self.create_proposer_election(&epoch_state, &onchain_consensus_config);
        let chain_health_backoff_config =
            ChainHealthBackoffConfig::new(self.config.chain_health_backoff.clone());
        let pipeline_backpressure_config = PipelineBackpressureConfig::new(
            self.config.pipeline_backpressure.clone(),
            self.config.execution_backpressure.clone(),
        );

        let safety_rules_container = Arc::new(Mutex::new(safety_rules));

        self.execution_client
            .start_epoch(
                consensus_key.clone(),
                epoch_state.clone(),
                safety_rules_container.clone(),
                payload_manager.clone(),
                &onchain_consensus_config,
                &onchain_execution_config,
                &onchain_randomness_config,
                rand_config,
                fast_rand_config.clone(),
                rand_msg_rx,
                recovery_data.commit_root_block().round(),
            )
            .await;
        let consensus_sk = consensus_key;

        let signer = Arc::new(ValidatorSigner::new(self.author, consensus_sk));
        let pipeline_builder = self.execution_client.pipeline_builder(signer);
        info!(epoch = epoch, "Create BlockStore");
        // Read the last vote, before "moving" `recovery_data`
        let last_vote = recovery_data.last_vote();
        let block_store = Arc::new(BlockStore::new(
            Arc::clone(&self.storage),
            recovery_data,
            self.execution_client.clone(),
            self.config.max_pruned_blocks_in_mem,
            Arc::clone(&self.time_service),
            self.config.vote_back_pressure_limit,
            payload_manager,
            onchain_consensus_config.order_vote_enabled(),
            onchain_consensus_config.window_size(),
            self.pending_blocks.clone(),
            Some(pipeline_builder),
        ));

        let failures_tracker = Arc::new(Mutex::new(ExponentialWindowFailureTracker::new(
            100,
            epoch_state.verifier.get_ordered_account_addresses(),
        )));
        let opt_qs_payload_param_provider = Arc::new(OptQSPullParamsProvider::new(
            self.config.quorum_store.enable_opt_quorum_store,
            self.config.quorum_store.opt_qs_minimum_batch_age_usecs,
            failures_tracker.clone(),
        ));

        info!(epoch = epoch, "Create ProposalGenerator");
        let max_sending_block_txns_after_filtering = if self.config.enable_optimistic_proposal_tx {
            self.config.max_sending_opt_block_txns_after_filtering
        } else {
            self.config.max_sending_block_txns_after_filtering
        };
        // txn manager is required both by proposal generator (to pull the proposers)
        // and by event processor (to update their status).
        let proposal_generator = ProposalGenerator::new(
            self.author,
            block_store.clone(),
            payload_client,
            self.time_service.clone(),
            Duration::from_millis(self.config.quorum_store_poll_time_ms),
            PayloadTxnsSize::new(
                self.config.max_sending_block_txns,
                self.config.max_sending_block_bytes,
            ),
            max_sending_block_txns_after_filtering,
            PayloadTxnsSize::new(
                self.config.max_sending_inline_txns,
                self.config.max_sending_inline_bytes,
            ),
            onchain_consensus_config.max_failed_authors_to_store(),
            self.config
                .min_max_txns_in_block_after_filtering_from_backpressure,
            onchain_execution_config
                .block_executor_onchain_config()
                .block_gas_limit_type
                .block_gas_limit(),
            pipeline_backpressure_config,
            chain_health_backoff_config,
            self.quorum_store_enabled,
            onchain_consensus_config.effective_validator_txn_config(),
            self.config
                .quorum_store
                .allow_batches_without_pos_in_proposal,
            opt_qs_payload_param_provider,
        );
        let (round_manager_tx, round_manager_rx) = aptos_channel::new(
            QueueStyle::KLAST,
            10,
            Some(&counters::ROUND_MANAGER_CHANNEL_MSGS),
        );

        let (buffered_proposal_tx, buffered_proposal_rx) = aptos_channel::new(
            QueueStyle::KLAST,
            10,
            Some(&counters::ROUND_MANAGER_CHANNEL_MSGS),
        );

        let (opt_proposal_loopback_tx, opt_proposal_loopback_rx) =
            aptos_channels::new_unbounded(&counters::OP_COUNTERS.gauge("opt_proposal_queue"));

        self.round_manager_tx = Some(round_manager_tx.clone());
        self.buffered_proposal_tx = Some(buffered_proposal_tx.clone());
        let max_blocks_allowed = self
            .config
            .max_blocks_per_receiving_request(onchain_consensus_config.quorum_store_enabled());
        
        #[cfg(feature = "consensus_fuzzer")]
        {
            let epoch_state_copy = epoch_state.clone();
            let safety_rules_container_new = safety_rules_container.clone();
        }
        let mut round_manager = RoundManager::new(
            epoch_state,
            block_store.clone(),
            round_state,
            proposer_election,
            proposal_generator,
            safety_rules_container,
            network_sender,
            self.storage.clone(),
            onchain_consensus_config,
            buffered_proposal_tx,
            self.config.clone(),
            onchain_randomness_config,
            onchain_jwk_consensus_config,
            fast_rand_config,
            failures_tracker,
            opt_proposal_loopback_tx,
        );

        round_manager.init(last_vote).await;

        let (close_tx, close_rx) = oneshot::channel();
        self.round_manager_close_tx = Some(close_tx);
        tokio::spawn(round_manager.start(
            round_manager_rx,
            buffered_proposal_rx,
            opt_proposal_loopback_rx,
            close_rx,
        ));

        self.spawn_block_retrieval_task(epoch, block_store, max_blocks_allowed);

        #[cfg(feature = "consensus_fuzzer")]
        {
            let mut rapture_network_sender = NetworkSender::new(
                self.author,
                self.network_sender.clone(),
                self.self_sender.clone(),
                epoch_state_copy.verifier.clone(),
            );
            let mut copy_network = rapture_network_sender.clone();
            if let Some(state_model) = crate::rapture_hook::get_state_model_arc() {
                run_fuzzer(copy_network, state_model.clone(), safety_rules_container_new, self.author);
            }
        }
    }

    fn start_quorum_store(&mut self, quorum_store_builder: QuorumStoreBuilder) {
        if let Some((quorum_store_coordinator_tx, batch_retrieval_rx)) =
            quorum_store_builder.start()
        {
            self.quorum_store_coordinator_tx = Some(quorum_store_coordinator_tx);
            self.batch_retrieval_tx = Some(batch_retrieval_rx);
        }
    }

    fn create_network_sender(&mut self, epoch_state: &EpochState) -> NetworkSender {
        NetworkSender::new(
            self.author,
            self.network_sender.clone(),
            self.self_sender.clone(),
            epoch_state.verifier.clone(),
        )
    }

    fn try_get_rand_config_for_new_epoch(
        &self,
        consensus_key: Arc<PrivateKey>,
        new_epoch_state: &EpochState,
        onchain_randomness_config: &OnChainRandomnessConfig,
        maybe_dkg_state: anyhow::Result<DKGState>,
        consensus_config: &OnChainConsensusConfig,
    ) -> Result<(RandConfig, Option<RandConfig>), NoRandomnessReason> {
        if !consensus_config.is_vtxn_enabled() {
            return Err(NoRandomnessReason::VTxnDisabled);
        }
        if !onchain_randomness_config.randomness_enabled() {
            return Err(NoRandomnessReason::FeatureDisabled);
        }
        let new_epoch = new_epoch_state.epoch;

        let dkg_state = maybe_dkg_state.map_err(NoRandomnessReason::DKGStateResourceMissing)?;
        let dkg_session = dkg_state
            .last_completed
            .ok_or_else(|| NoRandomnessReason::DKGCompletedSessionResourceMissing)?;
        if dkg_session.metadata.dealer_epoch + 1 != new_epoch_state.epoch {
            return Err(NoRandomnessReason::CompletedSessionTooOld);
        }
        let dkg_pub_params = DefaultDKG::new_public_params(&dkg_session.metadata);
        let my_index = new_epoch_state
            .verifier
            .address_to_validator_index()
            .get(&self.author)
            .copied()
            .ok_or_else(|| NoRandomnessReason::NotInValidatorSet)?;

        let dkg_decrypt_key = maybe_dk_from_bls_sk(consensus_key.as_ref())
            .map_err(NoRandomnessReason::ErrConvertingConsensusKeyToDecryptionKey)?;
        let transcript = bcs::from_bytes::<<DefaultDKG as DKGTrait>::Transcript>(
            dkg_session.transcript.as_slice(),
        )
        .map_err(NoRandomnessReason::TranscriptDeserializationError)?;

        let vuf_pp = WvufPP::from(&dkg_pub_params.pvss_config.pp);

        // No need to verify the transcript.

        // keys for randomness generation
        let (sk, pk) = DefaultDKG::decrypt_secret_share_from_transcript(
            &dkg_pub_params,
            &transcript,
            my_index as u64,
            &dkg_decrypt_key,
        )
        .map_err(NoRandomnessReason::SecretShareDecryptionFailed)?;

        let fast_randomness_is_enabled = onchain_randomness_config.fast_randomness_enabled()
            && sk.fast.is_some()
            && pk.fast.is_some()
            && transcript.fast.is_some()
            && dkg_pub_params.pvss_config.fast_wconfig.is_some();

        let pk_shares = (0..new_epoch_state.verifier.len())
            .map(|id| {
                transcript
                    .main
                    .get_public_key_share(&dkg_pub_params.pvss_config.wconfig, &Player { id })
            })
            .collect::<Vec<_>>();

        // Recover existing augmented key pair or generate a new one
        let (augmented_key_pair, fast_augmented_key_pair) = if let Some((_, key_pair)) = self
            .rand_storage
            .get_key_pair_bytes()
            .map_err(NoRandomnessReason::RandDbNotAvailable)?
            .filter(|(epoch, _)| *epoch == new_epoch)
        {
            info!(epoch = new_epoch, "Recovering existing augmented key");
            bcs::from_bytes(&key_pair).map_err(NoRandomnessReason::KeyPairDeserializationError)?
        } else {
            info!(
                epoch = new_epoch_state.epoch,
                "Generating a new augmented key"
            );
            let mut rng =
                StdRng::from_rng(thread_rng()).map_err(NoRandomnessReason::RngCreationError)?;
            let augmented_key_pair = WVUF::augment_key_pair(&vuf_pp, sk.main, pk.main, &mut rng);
            let fast_augmented_key_pair = if fast_randomness_is_enabled {
                if let (Some(sk), Some(pk)) = (sk.fast, pk.fast) {
                    Some(WVUF::augment_key_pair(&vuf_pp, sk, pk, &mut rng))
                } else {
                    None
                }
            } else {
                None
            };
            self.rand_storage
                .save_key_pair_bytes(
                    new_epoch,
                    bcs::to_bytes(&(augmented_key_pair.clone(), fast_augmented_key_pair.clone()))
                        .map_err(NoRandomnessReason::KeyPairSerializationError)?,
                )
                .map_err(NoRandomnessReason::KeyPairPersistError)?;
            (augmented_key_pair, fast_augmented_key_pair)
        };

        let (ask, apk) = augmented_key_pair;

        let keys = RandKeys::new(ask, apk, pk_shares, new_epoch_state.verifier.len());

        let rand_config = RandConfig::new(
            self.author,
            new_epoch,
            new_epoch_state.verifier.clone(),
            vuf_pp.clone(),
            keys,
            dkg_pub_params.pvss_config.wconfig.clone(),
        );

        let fast_rand_config = if let (Some((ask, apk)), Some(trx), Some(wconfig)) = (
            fast_augmented_key_pair,
            transcript.fast.as_ref(),
            dkg_pub_params.pvss_config.fast_wconfig.as_ref(),
        ) {
            let pk_shares = (0..new_epoch_state.verifier.len())
                .map(|id| trx.get_public_key_share(wconfig, &Player { id }))
                .collect::<Vec<_>>();

            let fast_keys = RandKeys::new(ask, apk, pk_shares, new_epoch_state.verifier.len());
            let fast_wconfig = wconfig.clone();

            Some(RandConfig::new(
                self.author,
                new_epoch,
                new_epoch_state.verifier.clone(),
                vuf_pp,
                fast_keys,
                fast_wconfig,
            ))
        } else {
            None
        };

        Ok((rand_config, fast_rand_config))
    }

    async fn start_new_epoch(&mut self, payload: OnChainConfigPayload<P>) {
        let validator_set: ValidatorSet = payload
            .get()
            .expect("failed to get ValidatorSet from payload");
        let mut verifier: ValidatorVerifier = (&validator_set).into();
        verifier.set_optimistic_sig_verification_flag(self.config.optimistic_sig_verification);

        let epoch_state = Arc::new(EpochState {
            epoch: payload.epoch(),
            verifier: verifier.into(),
        });

        self.epoch_state = Some(epoch_state.clone());

        let onchain_consensus_config: anyhow::Result<OnChainConsensusConfig> = payload.get();
        let onchain_execution_config: anyhow::Result<OnChainExecutionConfig> = payload.get();
        let onchain_randomness_config_seq_num: anyhow::Result<RandomnessConfigSeqNum> =
            payload.get();
        let randomness_config_move_struct: anyhow::Result<RandomnessConfigMoveStruct> =
            payload.get();
        let onchain_jwk_consensus_config: anyhow::Result<OnChainJWKConsensusConfig> = payload.get();
        let dkg_state = payload.get::<DKGState>();

        if let Err(error) = &onchain_consensus_config {
            warn!("Failed to read on-chain consensus config {}", error);
        }

        if let Err(error) = &onchain_execution_config {
            warn!("Failed to read on-chain execution config {}", error);
        }

        if let Err(error) = &randomness_config_move_struct {
            warn!("Failed to read on-chain randomness config {}", error);
        }

        self.epoch_state = Some(epoch_state.clone());

        let consensus_config = onchain_consensus_config.unwrap_or_default();
        let execution_config = onchain_execution_config
            .unwrap_or_else(|_| OnChainExecutionConfig::default_if_missing());
        let onchain_randomness_config_seq_num = onchain_randomness_config_seq_num
            .unwrap_or_else(|_| RandomnessConfigSeqNum::default_if_missing());

        info!(
            epoch = epoch_state.epoch,
            local = self.randomness_override_seq_num,
            onchain = onchain_randomness_config_seq_num.seq_num,
            "Checking randomness config override."
        );
        if self.randomness_override_seq_num > onchain_randomness_config_seq_num.seq_num {
            warn!("Randomness will be force-disabled by local config!");
        }

        let onchain_randomness_config = OnChainRandomnessConfig::from_configs(
            self.randomness_override_seq_num,
            onchain_randomness_config_seq_num.seq_num,
            randomness_config_move_struct.ok(),
        );

        let jwk_consensus_config = onchain_jwk_consensus_config.unwrap_or_else(|_| {
            // `jwk_consensus_config` not yet initialized, falling back to the old configs.
            Self::equivalent_jwk_consensus_config_from_deprecated_resources(&payload)
        });

        let loaded_consensus_key = match self.load_consensus_key(&epoch_state.verifier) {
            Ok(k) => Arc::new(k),
            Err(e) => {
                panic!("load_consensus_key failed: {e}");
            },
        };

        let rand_configs = self.try_get_rand_config_for_new_epoch(
            loaded_consensus_key.clone(),
            &epoch_state,
            &onchain_randomness_config,
            dkg_state,
            &consensus_config,
        );

        let (rand_config, fast_rand_config) = match rand_configs {
            Ok((rand_config, fast_rand_config)) => (Some(rand_config), fast_rand_config),
            Err(reason) => {
                if onchain_randomness_config.randomness_enabled() {
                    if epoch_state.epoch > 2 {
                        error!(
                            "Failed to get randomness config for new epoch [{}]: {:?}",
                            epoch_state.epoch, reason
                        );
                    } else {
                        warn!(
                            "Failed to get randomness config for new epoch [{}]: {:?}",
                            epoch_state.epoch, reason
                        );
                    }
                }
                (None, None)
            },
        };

        info!(
            "[Randomness] start_new_epoch: epoch={}, rand_config={:?}, fast_rand_config={:?}",
            epoch_state.epoch, rand_config, fast_rand_config
        );

        let (network_sender, payload_client, payload_manager) = self
            .initialize_shared_component(
                &epoch_state,
                &consensus_config,
                loaded_consensus_key.clone(),
            )
            .await;

        let (rand_msg_tx, rand_msg_rx) = aptos_channel::new::<AccountAddress, IncomingRandGenRequest>(
            QueueStyle::KLAST,
            10,
            None,
        );

        self.rand_manager_msg_tx = Some(rand_msg_tx);

        if consensus_config.is_dag_enabled() {
            self.start_new_epoch_with_dag(
                epoch_state,
                loaded_consensus_key.clone(),
                consensus_config,
                execution_config,
                onchain_randomness_config,
                jwk_consensus_config,
                network_sender,
                payload_client,
                payload_manager,
                rand_config,
                fast_rand_config,
                rand_msg_rx,
            )
            .await
        } else {
            self.start_new_epoch_with_jolteon(
                loaded_consensus_key.clone(),
                epoch_state,
                consensus_config,
                execution_config,
                onchain_randomness_config,
                jwk_consensus_config,
                network_sender,
                payload_client,
                payload_manager,
                rand_config,
                fast_rand_config,
                rand_msg_rx,
            )
            .await
        }
    }

    async fn initialize_shared_component(
        &mut self,
        epoch_state: &EpochState,
        consensus_config: &OnChainConsensusConfig,
        consensus_key: Arc<PrivateKey>,
    ) -> (
        NetworkSender,
        Arc<dyn PayloadClient>,
        Arc<dyn TPayloadManager>,
    ) {
        self.set_epoch_start_metrics(epoch_state);
        self.quorum_store_enabled = self.enable_quorum_store(consensus_config);
        let network_sender = self.create_network_sender(epoch_state);
        let (payload_manager, quorum_store_client, quorum_store_builder) = self
            .init_payload_provider(
                epoch_state,
                network_sender.clone(),
                consensus_config,
                consensus_key,
            )
            .await;
        let effective_vtxn_config = consensus_config.effective_validator_txn_config();
        debug!("effective_vtxn_config={:?}", effective_vtxn_config);
        let mixed_payload_client = MixedPayloadClient::new(
            effective_vtxn_config,
            Arc::new(self.vtxn_pool.clone()),
            Arc::new(quorum_store_client),
        );
        self.start_quorum_store(quorum_store_builder);
        (
            network_sender,
            Arc::new(mixed_payload_client),
            payload_manager,
        )
    }

    async fn start_new_epoch_with_jolteon(
        &mut self,
        consensus_key: Arc<PrivateKey>,
        epoch_state: Arc<EpochState>,
        consensus_config: OnChainConsensusConfig,
        execution_config: OnChainExecutionConfig,
        onchain_randomness_config: OnChainRandomnessConfig,
        jwk_consensus_config: OnChainJWKConsensusConfig,
        network_sender: NetworkSender,
        payload_client: Arc<dyn PayloadClient>,
        payload_manager: Arc<dyn TPayloadManager>,
        rand_config: Option<RandConfig>,
        fast_rand_config: Option<RandConfig>,
        rand_msg_rx: aptos_channel::Receiver<AccountAddress, IncomingRandGenRequest>,
    ) {
        match self.storage.start(
            consensus_config.order_vote_enabled(),
            consensus_config.window_size(),
        ) {
            LivenessStorageData::FullRecoveryData(initial_data) => {
                self.recovery_mode = false;
                self.start_round_manager(
                    consensus_key,
                    initial_data,
                    epoch_state,
                    consensus_config,
                    execution_config,
                    onchain_randomness_config,
                    jwk_consensus_config,
                    Arc::new(network_sender),
                    payload_client,
                    payload_manager,
                    rand_config,
                    fast_rand_config,
                    rand_msg_rx,
                )
                .await
            },
            LivenessStorageData::PartialRecoveryData(ledger_data) => {
                self.recovery_mode = true;
                self.start_recovery_manager(
                    ledger_data,
                    consensus_config,
                    epoch_state,
                    Arc::new(network_sender),
                )
                .await
            },
        }
    }

    async fn start_new_epoch_with_dag(
        &mut self,
        epoch_state: Arc<EpochState>,
        loaded_consensus_key: Arc<PrivateKey>,
        onchain_consensus_config: OnChainConsensusConfig,
        on_chain_execution_config: OnChainExecutionConfig,
        onchain_randomness_config: OnChainRandomnessConfig,
        onchain_jwk_consensus_config: OnChainJWKConsensusConfig,
        network_sender: NetworkSender,
        payload_client: Arc<dyn PayloadClient>,
        payload_manager: Arc<dyn TPayloadManager>,
        rand_config: Option<RandConfig>,
        fast_rand_config: Option<RandConfig>,
        rand_msg_rx: aptos_channel::Receiver<AccountAddress, IncomingRandGenRequest>,
    ) {
        let epoch = epoch_state.epoch;
        let signer = Arc::new(ValidatorSigner::new(
            self.author,
            loaded_consensus_key.clone(),
        ));
        let commit_signer = Arc::new(DagCommitSigner::new(signer.clone()));

        assert!(
            onchain_consensus_config.decoupled_execution(),
            "decoupled execution must be enabled"
        );
        let highest_committed_round = self
            .storage
            .aptos_db()
            .get_latest_ledger_info()
            .expect("unable to get latest ledger info")
            .commit_info()
            .round();

        self.execution_client
            .start_epoch(
                loaded_consensus_key,
                epoch_state.clone(),
                commit_signer,
                payload_manager.clone(),
                &onchain_consensus_config,
                &on_chain_execution_config,
                &onchain_randomness_config,
                rand_config,
                fast_rand_config,
                rand_msg_rx,
                highest_committed_round,
            )
            .await;

        let onchain_dag_consensus_config = onchain_consensus_config.unwrap_dag_config_v1();
        let epoch_to_validators = self.extract_epoch_proposers(
            &epoch_state,
            onchain_dag_consensus_config.dag_ordering_causal_history_window as u32,
            epoch_state.verifier.get_ordered_account_addresses(),
            onchain_dag_consensus_config.dag_ordering_causal_history_window as u64,
        );
        let dag_storage = Arc::new(StorageAdapter::new(
            epoch,
            epoch_to_validators,
            self.storage.consensus_db(),
            self.storage.aptos_db(),
        ));

        let network_sender_arc = Arc::new(network_sender);

        let bootstrapper = DagBootstrapper::new(
            self.author,
            self.dag_config.clone(),
            onchain_dag_consensus_config.clone(),
            signer,
            epoch_state.clone(),
            dag_storage,
            network_sender_arc.clone(),
            network_sender_arc.clone(),
            network_sender_arc,
            self.aptos_time_service.clone(),
            payload_manager,
            payload_client,
            self.execution_client
                .get_execution_channel()
                .expect("unable to get execution channel"),
            self.execution_client.clone(),
            onchain_consensus_config.quorum_store_enabled(),
            onchain_consensus_config.effective_validator_txn_config(),
            onchain_randomness_config,
            onchain_jwk_consensus_config,
            self.bounded_executor.clone(),
            self.config
                .quorum_store
                .allow_batches_without_pos_in_proposal,
        );

        let (dag_rpc_tx, dag_rpc_rx) = aptos_channel::new(QueueStyle::FIFO, 10, None);
        self.dag_rpc_tx = Some(dag_rpc_tx);
        let (dag_shutdown_tx, dag_shutdown_rx) = oneshot::channel();
        self.dag_shutdown_tx = Some(dag_shutdown_tx);

        tokio::spawn(bootstrapper.start(dag_rpc_rx, dag_shutdown_rx));
    }

    fn enable_quorum_store(&mut self, onchain_config: &OnChainConsensusConfig) -> bool {
        fail_point!("consensus::start_new_epoch::disable_qs", |_| false);
        onchain_config.quorum_store_enabled()
    }

    async fn process_message(
        &mut self,
        peer_id: AccountAddress,
        consensus_msg: ConsensusMsg,
    ) -> anyhow::Result<()> {
        fail_point!("consensus::process::any", |_| {
            Err(anyhow::anyhow!("Injected error in process_message"))
        });
        #[cfg(feature = "consensus_fuzzer")]
        if let Some(model) = GLOBAL_STATE_MODEL.get() {
            let mut model_guard = model.lock();
            model_guard.on_new_msg(&consensus_msg);
        }

        if let ConsensusMsg::ProposalMsg(proposal) = &consensus_msg {
            observe_block(
                proposal.proposal().timestamp_usecs(),
                BlockStage::EPOCH_MANAGER_RECEIVED,
            );
        }
        if let ConsensusMsg::OptProposalMsg(proposal) = &consensus_msg {
            if !self.config.enable_optimistic_proposal_rx {
                bail!(
                    "Unexpected OptProposalMsg. Feature is disabled. Author: {}, Epoch: {}, Round: {}",
                    proposal.block_data().author(),
                    proposal.epoch(),
                    proposal.round()
                )
            }
            observe_block(
                proposal.timestamp_usecs(),
                BlockStage::EPOCH_MANAGER_RECEIVED,
            );
            observe_block(
                proposal.timestamp_usecs(),
                BlockStage::EPOCH_MANAGER_RECEIVED_OPT_PROPOSAL,
            );
        }
        // we can't verify signatures from a different epoch
        let maybe_unverified_event = self.check_epoch(peer_id, consensus_msg).await?;

        if let Some(unverified_event) = maybe_unverified_event {
            // filter out quorum store messages if quorum store has not been enabled
            match self.filter_quorum_store_events(peer_id, &unverified_event) {
                Ok(true) => {},
                Ok(false) => return Ok(()), // This occurs when the quorum store is not enabled, but the recovery mode is enabled. We filter out the messages, but don't raise any error.
                Err(err) => return Err(err),
            }
            // same epoch -> run well-formedness + signature check
            let epoch_state = self
                .epoch_state
                .clone()
                .ok_or_else(|| anyhow::anyhow!("Epoch state is not available"))?;
            let proof_cache = self.proof_cache.clone();
            let quorum_store_enabled = self.quorum_store_enabled;
            let quorum_store_msg_tx = self.quorum_store_msg_tx.clone();
            let buffered_proposal_tx = self.buffered_proposal_tx.clone();
            let round_manager_tx = self.round_manager_tx.clone();
            let my_peer_id = self.author;
            let max_num_batches = self.config.quorum_store.receiver_max_num_batches;
            let max_batch_expiry_gap_usecs =
                self.config.quorum_store.batch_expiry_gap_when_init_usecs;
            let payload_manager = self.payload_manager.clone();
            let pending_blocks = self.pending_blocks.clone();
            self.bounded_executor
                .spawn(async move {
                    match monitor!(
                        "verify_message",
                        unverified_event.clone().verify(
                            peer_id,
                            &epoch_state.verifier,
                            &proof_cache,
                            quorum_store_enabled,
                            peer_id == my_peer_id,
                            max_num_batches,
                            max_batch_expiry_gap_usecs,
                        )
                    ) {
                        Ok(verified_event) => {
                            Self::forward_event(
                                quorum_store_msg_tx,
                                round_manager_tx,
                                buffered_proposal_tx,
                                peer_id,
                                verified_event,
                                payload_manager,
                                pending_blocks,
                            );
                        },
                        Err(e) => {
                            error!(
                                SecurityEvent::ConsensusInvalidMessage,
                                remote_peer = peer_id,
                                error = ?e,
                                unverified_event = unverified_event
                            );
                        },
                    }
                })
                .await;
        }
        Ok(())
    }

    async fn check_epoch(
        &mut self,
        peer_id: AccountAddress,
        msg: ConsensusMsg,
    ) -> anyhow::Result<Option<UnverifiedEvent>> {
        match msg {
            ConsensusMsg::ProposalMsg(_)
            | ConsensusMsg::OptProposalMsg(_)
            | ConsensusMsg::SyncInfo(_)
            | ConsensusMsg::VoteMsg(_)
            | ConsensusMsg::RoundTimeoutMsg(_)
            | ConsensusMsg::OrderVoteMsg(_)
            | ConsensusMsg::CommitVoteMsg(_)
            | ConsensusMsg::CommitDecisionMsg(_)
            | ConsensusMsg::BatchMsg(_)
            | ConsensusMsg::BatchRequestMsg(_)
            | ConsensusMsg::SignedBatchInfo(_)
            | ConsensusMsg::ProofOfStoreMsg(_) => {
                let event: UnverifiedEvent = msg.into();
                if event.epoch()? == self.epoch() {
                    return Ok(Some(event));
                } else {
                    monitor!(
                        "process_different_epoch_consensus_msg",
                        self.process_different_epoch(event.epoch()?, peer_id)
                    )?;
                }
            },
            ConsensusMsg::EpochChangeProof(proof) => {
                let msg_epoch = proof.epoch()?;
                debug!(
                    LogSchema::new(LogEvent::ReceiveEpochChangeProof)
                        .remote_peer(peer_id)
                        .epoch(self.epoch()),
                    "Proof from epoch {}", msg_epoch,
                );
                if msg_epoch == self.epoch() {
                    monitor!("process_epoch_proof", self.initiate_new_epoch(*proof).await)?;
                } else {
                    info!(
                        remote_peer = peer_id,
                        "[EpochManager] Unexpected epoch proof from epoch {}, local epoch {}",
                        msg_epoch,
                        self.epoch()
                    );
                    counters::EPOCH_MANAGER_ISSUES_DETAILS
                        .with_label_values(&["epoch_proof_wrong_epoch"])
                        .inc();
                }
            },
            ConsensusMsg::EpochRetrievalRequest(request) => {
                ensure!(
                    request.end_epoch <= self.epoch(),
                    "[EpochManager] Received EpochRetrievalRequest beyond what we have locally"
                );
                monitor!(
                    "process_epoch_retrieval",
                    self.process_epoch_retrieval(*request, peer_id)
                )?;
            },
            _ => {
                bail!("[EpochManager] Unexpected messages: {:?}", msg);
            },
        }
        Ok(None)
    }

    fn filter_quorum_store_events(
        &mut self,
        peer_id: AccountAddress,
        event: &UnverifiedEvent,
    ) -> anyhow::Result<bool> {
        match event {
            UnverifiedEvent::BatchMsg(_)
            | UnverifiedEvent::SignedBatchInfo(_)
            | UnverifiedEvent::ProofOfStoreMsg(_) => {
                if self.quorum_store_enabled {
                    Ok(true) // This states that we shouldn't filter out the event
                } else if self.recovery_mode {
                    Ok(false) // This states that we should filter out the event, but without an error
                } else {
                    Err(anyhow::anyhow!(
                        "Quorum store is not enabled locally, but received msg from sender: {}",
                        peer_id,
                    ))
                }
            },
            _ => Ok(true), // This states that we shouldn't filter out the event
        }
    }

    fn forward_event_to<K: Eq + Hash + Clone, V>(
        mut maybe_tx: Option<aptos_channel::Sender<K, V>>,
        key: K,
        value: V,
    ) -> anyhow::Result<()> {
        if let Some(tx) = &mut maybe_tx {
            tx.push(key, value)
        } else {
            bail!("channel not initialized");
        }
    }

    fn forward_event(
        quorum_store_msg_tx: Option<aptos_channel::Sender<AccountAddress, (Author, VerifiedEvent)>>,
        round_manager_tx: Option<
            aptos_channel::Sender<(Author, Discriminant<VerifiedEvent>), (Author, VerifiedEvent)>,
        >,
        buffered_proposal_tx: Option<aptos_channel::Sender<Author, VerifiedEvent>>,
        peer_id: AccountAddress,
        event: VerifiedEvent,
        payload_manager: Arc<dyn TPayloadManager>,
        pending_blocks: Arc<Mutex<PendingBlocks>>,
    ) {
        if let VerifiedEvent::ProposalMsg(proposal) = &event {
            observe_block(
                proposal.proposal().timestamp_usecs(),
                BlockStage::EPOCH_MANAGER_VERIFIED,
            );
        }
        if let VerifiedEvent::OptProposalMsg(proposal) = &event {
            observe_block(
                proposal.timestamp_usecs(),
                BlockStage::EPOCH_MANAGER_VERIFIED,
            );
            observe_block(
                proposal.timestamp_usecs(),
                BlockStage::EPOCH_MANAGER_VERIFIED_OPT_PROPOSAL,
            );
        }
        if let Err(e) = match event {
            quorum_store_event @ (VerifiedEvent::SignedBatchInfo(_)
            | VerifiedEvent::ProofOfStoreMsg(_)
            | VerifiedEvent::BatchMsg(_)) => {
                Self::forward_event_to(quorum_store_msg_tx, peer_id, (peer_id, quorum_store_event))
                    .context("quorum store sender")
            },
            proposal_event @ VerifiedEvent::ProposalMsg(_) => {
                if let VerifiedEvent::ProposalMsg(p) = &proposal_event {
                    if let Some(payload) = p.proposal().payload() {
                        payload_manager.prefetch_payload_data(
                            payload,
                            p.proposer(),
                            p.proposal().timestamp_usecs(),
                        );
                    }
                    pending_blocks.lock().insert_block(p.proposal().clone());
                }

                Self::forward_event_to(buffered_proposal_tx, peer_id, proposal_event)
                    .context("proposal precheck sender")
            },
            opt_proposal_event @ VerifiedEvent::OptProposalMsg(_) => {
                if let VerifiedEvent::OptProposalMsg(p) = &opt_proposal_event {
                    payload_manager.prefetch_payload_data(
                        p.block_data().payload(),
                        p.proposer(),
                        p.timestamp_usecs(),
                    );
                }

                Self::forward_event_to(buffered_proposal_tx, peer_id, opt_proposal_event)
                    .context("proposal precheck sender")
            },
            round_manager_event => Self::forward_event_to(
                round_manager_tx,
                (peer_id, discriminant(&round_manager_event)),
                (peer_id, round_manager_event),
            )
            .context("round manager sender"),
        } {
            warn!("Failed to forward event: {}", e);
        }
    }

    /// TODO: @bchocho @hariria can change after all nodes upgrade to release with enum BlockRetrievalRequest (not struct)
    fn process_rpc_request(
        &mut self,
        peer_id: Author,
        request: IncomingRpcRequest,
    ) -> anyhow::Result<()> {
        fail_point!("consensus::process::any", |_| {
            Err(anyhow::anyhow!("Injected error in process_rpc_request"))
        });

        match request.epoch() {
            Some(epoch) if epoch != self.epoch() => {
                monitor!(
                    "process_different_epoch_rpc_request",
                    self.process_different_epoch(epoch, peer_id)
                )?;
                return Ok(());
            },
            None => {
                // TODO: @bchocho @hariria can change after all nodes upgrade to release with enum BlockRetrievalRequest (not struct)
                ensure!(matches!(
                    request,
                    IncomingRpcRequest::DeprecatedBlockRetrieval(_)
                        | IncomingRpcRequest::BlockRetrieval(_)
                ));
            },
            _ => {},
        }

        match request {
            // TODO @bchocho @hariria can remove after all nodes upgrade to release with enum BlockRetrievalRequest (not struct)
            IncomingRpcRequest::DeprecatedBlockRetrieval(
                DeprecatedIncomingBlockRetrievalRequest {
                    req,
                    protocol,
                    response_sender,
                },
            ) => {
                if let Some(tx) = &self.block_retrieval_tx {
                    let incoming_block_retrieval_request = IncomingBlockRetrievalRequest {
                        req: BlockRetrievalRequest::V1(req),
                        protocol,
                        response_sender,
                    };
                    tx.push(peer_id, incoming_block_retrieval_request)
                } else {
                    error!("Round manager not started (in IncomingRpcRequest::DeprecatedBlockRetrieval)");
                    Ok(())
                }
            },
            IncomingRpcRequest::BatchRetrieval(request) => {
                if let Some(tx) = &self.batch_retrieval_tx {
                    tx.push(peer_id, request)
                } else {
                    Err(anyhow::anyhow!("Quorum store not started"))
                }
            },
            IncomingRpcRequest::DAGRequest(request) => {
                if let Some(tx) = &self.dag_rpc_tx {
                    tx.push(peer_id, request)
                } else {
                    Err(anyhow::anyhow!("DAG not bootstrapped"))
                }
            },
            IncomingRpcRequest::CommitRequest(request) => {
                self.execution_client.send_commit_msg(peer_id, request)
            },
            IncomingRpcRequest::RandGenRequest(request) => {
                if let Some(tx) = &self.rand_manager_msg_tx {
                    tx.push(peer_id, request)
                } else {
                    bail!("Rand manager not started");
                }
            },
            IncomingRpcRequest::BlockRetrieval(request) => {
                if let Some(tx) = &self.block_retrieval_tx {
                    tx.push(peer_id, request)
                } else {
                    error!("Round manager not started");
                    Ok(())
                }
            },
        }
    }

    fn process_local_timeout(&mut self, round: u64) {
        let Some(sender) = self.round_manager_tx.as_mut() else {
            warn!(
                "Received local timeout for round {} without Round Manager",
                round
            );
            return;
        };

        let peer_id = self.author;
        let event = VerifiedEvent::LocalTimeout(round);
        if let Err(e) = sender.push((peer_id, discriminant(&event)), (peer_id, event)) {
            error!("Failed to send event to round manager {:?}", e);
        }
    }

    async fn await_reconfig_notification(&mut self) {
        let reconfig_notification = self
            .reconfig_events
            .next()
            .await
            .expect("Reconfig sender dropped, unable to start new epoch");
        self.start_new_epoch(reconfig_notification.on_chain_configs)
            .await;
    }

    pub async fn start(
        mut self,
        mut round_timeout_sender_rx: aptos_channels::Receiver<Round>,
        mut network_receivers: NetworkReceivers,
    ) {
        // initial start of the processor
        self.await_reconfig_notification().await;
        loop {
            tokio::select! {
                (peer, msg) = network_receivers.consensus_messages.select_next_some() => {
                    monitor!("epoch_manager_process_consensus_messages",
                    if let Err(e) = self.process_message(peer, msg).await {
                        error!(epoch = self.epoch(), error = ?e, kind = error_kind(&e));
                    });
                },
                (peer, msg) = network_receivers.quorum_store_messages.select_next_some() => {
                    monitor!("epoch_manager_process_quorum_store_messages",
                    if let Err(e) = self.process_message(peer, msg).await {
                        error!(epoch = self.epoch(), error = ?e, kind = error_kind(&e));
                    });
                },
                (peer, request) = network_receivers.rpc_rx.select_next_some() => {
                    monitor!("epoch_manager_process_rpc",
                    if let Err(e) = self.process_rpc_request(peer, request) {
                        error!(epoch = self.epoch(), error = ?e, kind = error_kind(&e));
                    });
                },
                round = round_timeout_sender_rx.select_next_some() => {
                    monitor!("epoch_manager_process_round_timeout",
                    self.process_local_timeout(round));
                },
            }
            // Continually capture the time of consensus process to ensure that clock skew between
            // validators is reasonable and to find any unusual (possibly byzantine) clock behavior.
            counters::OP_COUNTERS
                .gauge("time_since_epoch_ms")
                .set(duration_since_epoch().as_millis() as i64);
        }
    }

    /// Before `JWKConsensusConfig` is initialized, convert from `Features` and `SupportedOIDCProviders` instead.
    fn equivalent_jwk_consensus_config_from_deprecated_resources(
        payload: &OnChainConfigPayload<P>,
    ) -> OnChainJWKConsensusConfig {
        let features = payload.get::<Features>().ok();
        let oidc_providers = payload.get::<SupportedOIDCProviders>().ok();
        OnChainJWKConsensusConfig::from((features, oidc_providers))
    }

    fn load_consensus_key(&self, vv: &ValidatorVerifier) -> anyhow::Result<PrivateKey> {
        match vv.get_public_key(&self.author) {
            Some(pk) => self
                .key_storage
                .consensus_sk_by_pk(pk)
                .map_err(|e| anyhow!("could not find sk by pk: {:?}", e)),
            None => {
                warn!("could not find my pk in validator set, loading default sk!");
                self.key_storage
                    .default_consensus_sk()
                    .map_err(|e| anyhow!("could not load default sk: {e}"))
            },
        }
    }
}

#[allow(dead_code)]
#[derive(Debug)]
pub enum NoRandomnessReason {
    VTxnDisabled,
    FeatureDisabled,
    DKGStateResourceMissing(anyhow::Error),
    DKGCompletedSessionResourceMissing,
    CompletedSessionTooOld,
    NotInValidatorSet,
    ErrConvertingConsensusKeyToDecryptionKey(anyhow::Error),
    TranscriptDeserializationError(bcs::Error),
    SecretShareDecryptionFailed(anyhow::Error),
    RngCreationError(rand::Error),
    RandDbNotAvailable(anyhow::Error),
    KeyPairDeserializationError(bcs::Error),
    KeyPairSerializationError(bcs::Error),
    KeyPairPersistError(anyhow::Error),
}
