// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    block_storage::{
        tracing::{observe_block, BlockStage},
        BlockStore,
    },
    counters,
    error::{error_kind, DbError},
    experimental::{
        buffer_manager::{OrderedBlocks, ResetRequest},
        decoupled_execution_utils::prepare_phases_and_buffer_manager,
        ordering_state_computer::OrderingStateComputer,
    },
    liveness::{
        cached_proposer_election::CachedProposerElection,
        leader_reputation::{
            extract_epoch_to_proposers, AptosDBBackend, LeaderReputation,
            ProposerAndVoterHeuristic, ReputationHeuristic,
        },
        proposal_generator::{
            ChainHealthBackoffConfig, PipelineBackpressureConfig, ProposalGenerator,
        },
        proposer_election::ProposerElection,
        rotating_proposer_election::{choose_leader, RotatingProposer},
        round_proposer_election::RoundProposer,
        round_state::{ExponentialTimeInterval, RoundState},
    },
    logging::{LogEvent, LogSchema},
    metrics_safety_rules::MetricsSafetyRules,
    monitor,
    network::{
        IncomingBatchRetrievalRequest, IncomingBlockRetrievalRequest, IncomingRpcRequest,
        NetworkReceivers, NetworkSender,
    },
    network_interface::{ConsensusMsg, ConsensusNetworkClient},
    payload_client::QuorumStoreClient,
    persistent_liveness_storage::{LedgerRecoveryData, PersistentLivenessStorage, RecoveryData},
    quorum_store::{
        quorum_store_builder::{DirectMempoolInnerBuilder, InnerBuilder, QuorumStoreBuilder},
        quorum_store_coordinator::CoordinatorCommand,
        quorum_store_db::QuorumStoreStorage,
    },
    recovery_manager::RecoveryManager,
    round_manager::{RoundManager, UnverifiedEvent, VerifiedEvent},
    state_replication::StateComputer,
    transaction_deduper::create_transaction_deduper,
    transaction_shuffler::create_transaction_shuffler,
    util::time_service::TimeService,
};
use anyhow::{bail, ensure, Context};
use aptos_bounded_executor::BoundedExecutor;
use aptos_channels::{aptos_channel, message_queues::QueueStyle};
use aptos_config::config::{ConsensusConfig, NodeConfig};
use aptos_consensus_types::{
    common::{Author, Round},
    epoch_retrieval::EpochRetrievalRequest,
};
use aptos_event_notifications::ReconfigNotificationListener;
use aptos_infallible::{duration_since_epoch, Mutex};
use aptos_logger::prelude::*;
use aptos_mempool::QuorumStoreRequest;
use aptos_network::{application::interface::NetworkClient, protocols::network::Event};
use aptos_safety_rules::SafetyRulesManager;
use aptos_types::{
    account_address::AccountAddress,
    epoch_change::EpochChangeProof,
    epoch_state::EpochState,
    on_chain_config::{
        ExecutionConfigV1, LeaderReputationType, OnChainConfigPayload, OnChainConsensusConfig,
        OnChainExecutionConfig, ProposerElectionType, TransactionShufflerType, ValidatorSet,
    },
    validator_verifier::ValidatorVerifier,
};
use fail::fail_point;
use futures::{
    channel::{
        mpsc,
        mpsc::{unbounded, Sender, UnboundedSender},
        oneshot,
    },
    SinkExt, StreamExt,
};
use itertools::Itertools;
use std::{
    cmp::Ordering,
    collections::HashMap,
    hash::Hash,
    mem::{discriminant, Discriminant},
    sync::Arc,
    time::Duration,
};

/// Range of rounds (window) that we might be calling proposer election
/// functions with at any given time, in addition to the proposer history length.
const PROPSER_ELECTION_CACHING_WINDOW_ADDITION: usize = 3;
/// Number of rounds we expect storage to be ahead of the proposer round,
/// used for fetching data from DB.
const PROPSER_ROUND_BEHIND_STORAGE_BUFFER: usize = 10;

#[allow(clippy::large_enum_variant)]
pub enum LivenessStorageData {
    FullRecoveryData(RecoveryData),
    PartialRecoveryData(LedgerRecoveryData),
}

// Manager the components that shared across epoch and spawn per-epoch RoundManager with
// epoch-specific input.
pub struct EpochManager {
    author: Author,
    config: ConsensusConfig,
    time_service: Arc<dyn TimeService>,
    self_sender: aptos_channels::Sender<Event<ConsensusMsg>>,
    network_sender: ConsensusNetworkClient<NetworkClient<ConsensusMsg>>,
    timeout_sender: aptos_channels::Sender<Round>,
    quorum_store_enabled: bool,
    quorum_store_to_mempool_sender: Sender<QuorumStoreRequest>,
    commit_state_computer: Arc<dyn StateComputer>,
    storage: Arc<dyn PersistentLivenessStorage>,
    safety_rules_manager: SafetyRulesManager,
    reconfig_events: ReconfigNotificationListener,
    // channels to buffer manager
    buffer_manager_msg_tx: Option<aptos_channel::Sender<AccountAddress, VerifiedEvent>>,
    buffer_manager_reset_tx: Option<UnboundedSender<ResetRequest>>,
    // channels to round manager
    round_manager_tx: Option<
        aptos_channel::Sender<(Author, Discriminant<VerifiedEvent>), (Author, VerifiedEvent)>,
    >,
    round_manager_close_tx: Option<oneshot::Sender<oneshot::Sender<()>>>,
    epoch_state: Option<Arc<EpochState>>,
    block_retrieval_tx:
        Option<aptos_channel::Sender<AccountAddress, IncomingBlockRetrievalRequest>>,
    quorum_store_msg_tx: Option<aptos_channel::Sender<AccountAddress, VerifiedEvent>>,
    quorum_store_coordinator_tx: Option<Sender<CoordinatorCommand>>,
    quorum_store_storage: Arc<dyn QuorumStoreStorage>,
    batch_retrieval_tx:
        Option<aptos_channel::Sender<AccountAddress, IncomingBatchRetrievalRequest>>,
    bounded_executor: BoundedExecutor,
    // recovery_mode is set to true when the recovery manager is spawned
    recovery_mode: bool,
}

impl EpochManager {
    pub(crate) fn new(
        node_config: &NodeConfig,
        time_service: Arc<dyn TimeService>,
        self_sender: aptos_channels::Sender<Event<ConsensusMsg>>,
        network_sender: ConsensusNetworkClient<NetworkClient<ConsensusMsg>>,
        timeout_sender: aptos_channels::Sender<Round>,
        quorum_store_to_mempool_sender: Sender<QuorumStoreRequest>,
        commit_state_computer: Arc<dyn StateComputer>,
        storage: Arc<dyn PersistentLivenessStorage>,
        quorum_store_storage: Arc<dyn QuorumStoreStorage>,
        reconfig_events: ReconfigNotificationListener,
        bounded_executor: BoundedExecutor,
    ) -> Self {
        let author = node_config.validator_network.as_ref().unwrap().peer_id();
        let config = node_config.consensus.clone();
        let sr_config = &node_config.consensus.safety_rules;
        let safety_rules_manager = SafetyRulesManager::new(sr_config);
        Self {
            author,
            config,
            time_service,
            self_sender,
            network_sender,
            timeout_sender,
            // This default value is updated at epoch start
            quorum_store_enabled: false,
            quorum_store_to_mempool_sender,
            commit_state_computer,
            storage,
            safety_rules_manager,
            reconfig_events,
            buffer_manager_msg_tx: None,
            buffer_manager_reset_tx: None,
            round_manager_tx: None,
            round_manager_close_tx: None,
            epoch_state: None,
            block_retrieval_tx: None,
            quorum_store_msg_tx: None,
            quorum_store_coordinator_tx: None,
            quorum_store_storage,
            batch_retrieval_tx: None,
            bounded_executor,
            recovery_mode: false,
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
    ) -> Box<dyn ProposerElection + Send + Sync> {
        let proposers = epoch_state
            .verifier
            .get_ordered_account_addresses_iter()
            .collect::<Vec<_>>();
        match &onchain_config.proposer_election_type() {
            ProposerElectionType::RotatingProposer(contiguous_rounds) => {
                Box::new(RotatingProposer::new(proposers, *contiguous_rounds))
            },
            // We don't really have a fixed proposer!
            ProposerElectionType::FixedProposer(contiguous_rounds) => {
                let proposer = choose_leader(proposers);
                Box::new(RotatingProposer::new(vec![proposer], *contiguous_rounds))
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
                    + PROPSER_ROUND_BEHIND_STORAGE_BUFFER;

                let backend = Box::new(AptosDBBackend::new(
                    window_size,
                    seek_len,
                    self.storage.aptos_db(),
                ));
                let voting_powers: Vec<_> = if weight_by_voting_power {
                    proposers
                        .iter()
                        .map(|p| epoch_state.verifier.get_voting_power(p).unwrap())
                        .collect()
                } else {
                    vec![1; proposers.len()]
                };

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
                let epoch_to_proposers = if epoch_state.epoch > first_epoch_to_consider {
                    self.storage
                        .aptos_db()
                        .get_epoch_ending_ledger_infos(first_epoch_to_consider - 1, epoch_state.epoch)
                        .and_then(|proof| {
                            ensure!(proof.ledger_info_with_sigs.len() as u64 == (epoch_state.epoch - (first_epoch_to_consider - 1)));
                            extract_epoch_to_proposers(proof, epoch_state.epoch, &proposers, (window_size + seek_len) as u64)
                        })
                        .unwrap_or_else(|err| {
                            error!("Couldn't create leader reputation with history across epochs, {:?}", err);
                            HashMap::from([(epoch_state.epoch, proposers)])
                        })
                } else {
                    HashMap::from([(epoch_state.epoch, proposers)])
                };

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
                Box::new(CachedProposerElection::new(
                    epoch_state.epoch,
                    proposer_election,
                    onchain_config.max_failed_authors_to_store()
                        + PROPSER_ELECTION_CACHING_WINDOW_ADDITION,
                ))
            },
            ProposerElectionType::RoundProposer(round_proposers) => {
                // Hardcoded to the first proposer
                let default_proposer = proposers.first().unwrap();
                Box::new(RoundProposer::new(
                    round_proposers.clone(),
                    *default_proposer,
                ))
            },
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
        self.network_sender.send_to(peer_id, msg).context(format!(
            "[EpochManager] Failed to send epoch proof to {}",
            peer_id
        ))
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
                self.network_sender.send_to(peer_id, msg).context(format!(
                    "[EpochManager] Failed to send epoch retrieval to {}",
                    peer_id
                ))
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
        // make sure storage is on this ledger_info too, it should be no-op if it's already committed
        // panic if this doesn't succeed since the current processors are already shutdown.
        self.commit_state_computer
            .sync_to(ledger_info.clone())
            .await
            .context(format!(
                "[EpochManager] State sync to new epoch {}",
                ledger_info
            ))
            .expect("Failed to sync to new epoch");

        monitor!("reconfig", self.await_reconfig_notification().await);
        Ok(())
    }

    fn spawn_block_retrieval_task(&mut self, epoch: u64, block_store: Arc<BlockStore>) {
        let (request_tx, mut request_rx) = aptos_channel::new(
            QueueStyle::LIFO,
            1,
            Some(&counters::BLOCK_RETRIEVAL_TASK_MSGS),
        );
        let task = async move {
            info!(epoch = epoch, "Block retrieval task starts");
            while let Some(request) = request_rx.next().await {
                if let Err(e) = monitor!(
                    "process_block_retrieval",
                    block_store.process_block_retrieval(request).await
                ) {
                    warn!(epoch = epoch, error = ?e, kind = error_kind(&e));
                }
            }
            info!(epoch = epoch, "Block retrieval task stops");
        };
        self.block_retrieval_tx = Some(request_tx);
        tokio::spawn(task);
    }

    /// this function spawns the phases and a buffer manager
    /// it sets `self.commit_msg_tx` to a new aptos_channel::Sender and returns an OrderingStateComputer
    fn spawn_decoupled_execution(
        &mut self,
        safety_rules_container: Arc<Mutex<MetricsSafetyRules>>,
        verifier: ValidatorVerifier,
    ) -> OrderingStateComputer {
        let network_sender = NetworkSender::new(
            self.author,
            self.network_sender.clone(),
            self.self_sender.clone(),
            verifier.clone(),
        );

        let (block_tx, block_rx) = unbounded::<OrderedBlocks>();
        let (reset_tx, reset_rx) = unbounded::<ResetRequest>();

        let (commit_msg_tx, commit_msg_rx) = aptos_channel::new::<AccountAddress, VerifiedEvent>(
            QueueStyle::FIFO,
            100,
            Some(&counters::BUFFER_MANAGER_MSGS),
        );

        self.buffer_manager_msg_tx = Some(commit_msg_tx);
        self.buffer_manager_reset_tx = Some(reset_tx.clone());

        let (execution_phase, signing_phase, persisting_phase, buffer_manager) =
            prepare_phases_and_buffer_manager(
                self.author,
                self.commit_state_computer.clone(),
                safety_rules_container,
                network_sender,
                commit_msg_rx,
                self.commit_state_computer.clone(),
                block_rx,
                reset_rx,
                verifier,
            );

        tokio::spawn(execution_phase.start());
        tokio::spawn(signing_phase.start());
        tokio::spawn(persisting_phase.start());
        tokio::spawn(buffer_manager.start());

        OrderingStateComputer::new(block_tx, self.commit_state_computer.clone(), reset_tx)
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

        // Shutdown the previous buffer manager, to release the SafetyRule client
        self.buffer_manager_msg_tx = None;
        if let Some(mut tx) = self.buffer_manager_reset_tx.take() {
            let (ack_tx, ack_rx) = oneshot::channel();
            tx.send(ResetRequest {
                tx: ack_tx,
                stop: true,
            })
            .await
            .expect("[EpochManager] Fail to drop buffer manager");
            ack_rx
                .await
                .expect("[EpochManager] Fail to drop buffer manager");
        }

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

        self.commit_state_computer.end_epoch();
    }

    async fn start_recovery_manager(
        &mut self,
        ledger_data: LedgerRecoveryData,
        epoch_state: EpochState,
    ) {
        let network_sender = NetworkSender::new(
            self.author,
            self.network_sender.clone(),
            self.self_sender.clone(),
            epoch_state.verifier.clone(),
        );
        let (recovery_manager_tx, recovery_manager_rx) = aptos_channel::new(
            QueueStyle::LIFO,
            1,
            Some(&counters::ROUND_MANAGER_CHANNEL_MSGS),
        );
        self.round_manager_tx = Some(recovery_manager_tx);
        let (close_tx, close_rx) = oneshot::channel();
        self.round_manager_close_tx = Some(close_tx);
        let recovery_manager = RecoveryManager::new(
            epoch_state,
            network_sender,
            self.storage.clone(),
            self.commit_state_computer.clone(),
            ledger_data.committed_round(),
        );
        tokio::spawn(recovery_manager.start(recovery_manager_rx, close_rx));
    }

    async fn start_round_manager(
        &mut self,
        recovery_data: RecoveryData,
        epoch_state: EpochState,
        onchain_consensus_config: OnChainConsensusConfig,
        onchain_execution_config: OnChainExecutionConfig,
    ) {
        let epoch = epoch_state.epoch;
        counters::EPOCH.set(epoch_state.epoch as i64);
        counters::CURRENT_EPOCH_VALIDATORS.set(epoch_state.verifier.len() as i64);
        info!(
            epoch = epoch_state.epoch,
            validators = epoch_state.verifier.to_string(),
            root_block = %recovery_data.root_block(),
            "Starting new epoch",
        );
        let last_vote = recovery_data.last_vote();

        info!(epoch = epoch, "Update SafetyRules");

        let mut safety_rules =
            MetricsSafetyRules::new(self.safety_rules_manager.client(), self.storage.clone());
        if let Err(error) = safety_rules.perform_initialize() {
            error!(
                epoch = epoch,
                error = error,
                "Unable to initialize safety rules.",
            );
        }

        info!(epoch = epoch, "Create RoundState");
        let round_state =
            self.create_round_state(self.time_service.clone(), self.timeout_sender.clone());

        info!(epoch = epoch, "Create ProposerElection");
        let proposer_election =
            self.create_proposer_election(&epoch_state, &onchain_consensus_config);
        let network_sender = NetworkSender::new(
            self.author,
            self.network_sender.clone(),
            self.self_sender.clone(),
            epoch_state.verifier.clone(),
        );
        let chain_health_backoff_config =
            ChainHealthBackoffConfig::new(self.config.chain_health_backoff.clone());
        let pipeline_backpressure_config =
            PipelineBackpressureConfig::new(self.config.pipeline_backpressure.clone());

        let safety_rules_container = Arc::new(Mutex::new(safety_rules));

        // Start QuorumStore
        let (consensus_to_quorum_store_tx, consensus_to_quorum_store_rx) =
            mpsc::channel(self.config.intra_consensus_channel_buffer_size);

        let mut quorum_store_builder = if self.quorum_store_enabled {
            info!("Building QuorumStore");
            QuorumStoreBuilder::QuorumStore(InnerBuilder::new(
                self.epoch(),
                self.author,
                epoch_state.verifier.len() as u64,
                self.config.quorum_store.clone(),
                consensus_to_quorum_store_rx,
                self.quorum_store_to_mempool_sender.clone(),
                self.config.mempool_txn_pull_timeout_ms,
                self.storage.aptos_db().clone(),
                network_sender.clone(),
                epoch_state.verifier.clone(),
                self.config.safety_rules.backend.clone(),
                self.quorum_store_storage.clone(),
            ))
        } else {
            info!("Building DirectMempool");
            QuorumStoreBuilder::DirectMempool(DirectMempoolInnerBuilder::new(
                consensus_to_quorum_store_rx,
                self.quorum_store_to_mempool_sender.clone(),
                self.config.mempool_txn_pull_timeout_ms,
            ))
        };

        let (payload_manager, quorum_store_msg_tx) = quorum_store_builder.init_payload_manager();
        let transaction_shuffler =
            create_transaction_shuffler(onchain_execution_config.transaction_shuffler_type());
        let block_gas_limit = onchain_execution_config.block_gas_limit();
        let transaction_deduper =
            create_transaction_deduper(onchain_execution_config.transaction_deduper_type());
        self.quorum_store_msg_tx = quorum_store_msg_tx;

        let payload_client = QuorumStoreClient::new(
            consensus_to_quorum_store_tx,
            self.config.quorum_store_pull_timeout_ms,
            self.config.wait_for_full_blocks_above_recent_fill_threshold,
            self.config.wait_for_full_blocks_above_pending_blocks,
        );
        self.commit_state_computer.new_epoch(
            &epoch_state,
            payload_manager.clone(),
            transaction_shuffler,
            block_gas_limit,
            transaction_deduper,
        );
        let state_computer = if onchain_consensus_config.decoupled_execution() {
            Arc::new(self.spawn_decoupled_execution(
                safety_rules_container.clone(),
                epoch_state.verifier.clone(),
            ))
        } else {
            self.commit_state_computer.clone()
        };

        info!(epoch = epoch, "Create BlockStore");
        let block_store = Arc::new(BlockStore::new(
            Arc::clone(&self.storage),
            recovery_data,
            state_computer,
            self.config.max_pruned_blocks_in_mem,
            Arc::clone(&self.time_service),
            self.config.vote_back_pressure_limit,
            payload_manager.clone(),
        ));

        if let Some((quorum_store_coordinator_tx, batch_retrieval_rx)) =
            quorum_store_builder.start()
        {
            self.quorum_store_coordinator_tx = Some(quorum_store_coordinator_tx);
            self.batch_retrieval_tx = Some(batch_retrieval_rx);
        }

        info!(epoch = epoch, "Create ProposalGenerator");
        // txn manager is required both by proposal generator (to pull the proposers)
        // and by event processor (to update their status).
        let proposal_generator = ProposalGenerator::new(
            self.author,
            block_store.clone(),
            Arc::new(payload_client),
            self.time_service.clone(),
            Duration::from_millis(self.config.quorum_store_poll_time_ms),
            self.config
                .max_sending_block_txns(self.quorum_store_enabled),
            self.config
                .max_sending_block_bytes(self.quorum_store_enabled),
            onchain_consensus_config.max_failed_authors_to_store(),
            pipeline_backpressure_config,
            chain_health_backoff_config,
            self.quorum_store_enabled,
        );

        let (round_manager_tx, round_manager_rx) = aptos_channel::new(
            QueueStyle::LIFO,
            1,
            Some(&counters::ROUND_MANAGER_CHANNEL_MSGS),
        );

        self.round_manager_tx = Some(round_manager_tx.clone());

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
            round_manager_tx,
            self.config.clone(),
        );

        round_manager.init(last_vote).await;

        let (close_tx, close_rx) = oneshot::channel();
        self.round_manager_close_tx = Some(close_tx);
        tokio::spawn(round_manager.start(round_manager_rx, close_rx));

        self.spawn_block_retrieval_task(epoch, block_store);
    }

    async fn start_new_epoch(&mut self, payload: OnChainConfigPayload) {
        let validator_set: ValidatorSet = payload
            .get()
            .expect("failed to get ValidatorSet from payload");
        let epoch_state = EpochState {
            epoch: payload.epoch(),
            verifier: (&validator_set).into(),
        };

        let onchain_consensus_config: anyhow::Result<OnChainConsensusConfig> = payload.get();
        let onchain_execution_config: anyhow::Result<OnChainExecutionConfig> = payload.get();
        if let Err(error) = &onchain_consensus_config {
            error!("Failed to read on-chain consensus config {}", error);
        }

        self.epoch_state = Some(Arc::new(epoch_state.clone()));

        match self.storage.start() {
            LivenessStorageData::FullRecoveryData(initial_data) => {
                let consensus_config = onchain_consensus_config.unwrap_or_default();
                let execution_config = onchain_execution_config.unwrap_or(
                    OnChainExecutionConfig::V1(ExecutionConfigV1 {
                        transaction_shuffler_type: TransactionShufflerType::NoShuffling,
                    }),
                );
                self.quorum_store_enabled = self.enable_quorum_store(&consensus_config);
                self.recovery_mode = false;
                self.start_round_manager(
                    initial_data,
                    epoch_state,
                    consensus_config,
                    execution_config,
                )
                .await
            },
            LivenessStorageData::PartialRecoveryData(ledger_data) => {
                self.recovery_mode = true;
                self.start_recovery_manager(ledger_data, epoch_state).await
            },
        }
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

        if let ConsensusMsg::ProposalMsg(proposal) = &consensus_msg {
            observe_block(
                proposal.proposal().timestamp_usecs(),
                BlockStage::EPOCH_MANAGER_RECEIVED,
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
            let epoch_state = self.epoch_state.clone().unwrap();
            let quorum_store_enabled = self.quorum_store_enabled;
            let quorum_store_msg_tx = self.quorum_store_msg_tx.clone();
            let buffer_manager_msg_tx = self.buffer_manager_msg_tx.clone();
            let round_manager_tx = self.round_manager_tx.clone();
            let my_peer_id = self.author;
            let max_num_batches = self.config.quorum_store.receiver_max_num_batches;
            self.bounded_executor
                .spawn(async move {
                    match monitor!(
                        "verify_message",
                        unverified_event.clone().verify(
                            peer_id,
                            &epoch_state.verifier,
                            quorum_store_enabled,
                            peer_id == my_peer_id,
                            max_num_batches,
                        )
                    ) {
                        Ok(verified_event) => {
                            Self::forward_event(
                                quorum_store_msg_tx,
                                buffer_manager_msg_tx,
                                round_manager_tx,
                                peer_id,
                                verified_event,
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
            | ConsensusMsg::SyncInfo(_)
            | ConsensusMsg::VoteMsg(_)
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
                    bail!(
                        "[EpochManager] Unexpected epoch proof from epoch {}, local epoch {}",
                        msg_epoch,
                        self.epoch()
                    );
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
        quorum_store_msg_tx: Option<aptos_channel::Sender<AccountAddress, VerifiedEvent>>,
        buffer_manager_msg_tx: Option<aptos_channel::Sender<AccountAddress, VerifiedEvent>>,
        round_manager_tx: Option<
            aptos_channel::Sender<(Author, Discriminant<VerifiedEvent>), (Author, VerifiedEvent)>,
        >,
        peer_id: AccountAddress,
        event: VerifiedEvent,
    ) {
        if let VerifiedEvent::ProposalMsg(proposal) = &event {
            observe_block(
                proposal.proposal().timestamp_usecs(),
                BlockStage::EPOCH_MANAGER_VERIFIED,
            );
        }
        if let Err(e) = match event {
            quorum_store_event @ (VerifiedEvent::SignedBatchInfo(_)
            | VerifiedEvent::ProofOfStoreMsg(_)
            | VerifiedEvent::BatchMsg(_)) => {
                Self::forward_event_to(quorum_store_msg_tx, peer_id, quorum_store_event)
                    .context("quorum store sender")
            },
            buffer_manager_event @ (VerifiedEvent::CommitVote(_)
            | VerifiedEvent::CommitDecision(_)) => {
                Self::forward_event_to(buffer_manager_msg_tx, peer_id, buffer_manager_event)
                    .context("buffer manager sender")
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

    fn process_rpc_request(
        &mut self,
        peer_id: Author,
        request: IncomingRpcRequest,
    ) -> anyhow::Result<()> {
        fail_point!("consensus::process::any", |_| {
            Err(anyhow::anyhow!("Injected error in process_rpc_request"))
        });
        match request {
            IncomingRpcRequest::BlockRetrieval(request) => {
                if let Some(tx) = &self.block_retrieval_tx {
                    tx.push(peer_id, request)
                } else {
                    Err(anyhow::anyhow!("Round manager not started"))
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
                let dag_message = request.req;

                if dag_message.epoch == self.epoch() {
                    // TODO: send message to DAG handler
                    Ok(())
                } else {
                    monitor!(
                        "process_different_epoch_dag_rpc",
                        self.process_different_epoch(dag_message.epoch, peer_id)
                    )
                }
            },
        }
    }

    fn process_local_timeout(&mut self, round: u64) {
        let peer_id = self.author;
        let event = VerifiedEvent::LocalTimeout(round);
        let sender = self
            .round_manager_tx
            .as_mut()
            .expect("RoundManager not started");
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
                (peer, msg) = network_receivers.buffer_manager_messages.select_next_some() => {
                    monitor!("epoch_manager_process_buffer_manager_messages",
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
}
