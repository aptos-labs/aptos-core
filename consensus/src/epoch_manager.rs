// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    block_storage::BlockStore,
    commit_notifier::CommitNotifier,
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
        proposal_generator::ProposalGenerator,
        proposer_election::ProposerElection,
        rotating_proposer_election::{choose_leader, RotatingProposer},
        round_proposer_election::RoundProposer,
        round_state::{ExponentialTimeInterval, RoundState},
    },
    logging::{LogEvent, LogSchema},
    metrics_safety_rules::MetricsSafetyRules,
    monitor,
    network::{IncomingBlockRetrievalRequest, NetworkReceivers, NetworkSender},
    network_interface::{ConsensusMsg, ConsensusNetworkSender},
    payload_manager::QuorumStoreClient,
    persistent_liveness_storage::{LedgerRecoveryData, PersistentLivenessStorage, RecoveryData},
    quorum_store::direct_mempool_quorum_store::DirectMempoolQuorumStore,
    round_manager::{RoundManager, UnverifiedEvent, VerifiedEvent},
    state_replication::StateComputer,
    util::time_service::TimeService,
};
use anyhow::{bail, ensure, Context};
use aptos_config::config::{ConsensusConfig, NodeConfig};
use aptos_infallible::Mutex;
use aptos_logger::prelude::*;
use aptos_mempool::QuorumStoreRequest;
use aptos_types::{
    account_address::AccountAddress,
    epoch_change::EpochChangeProof,
    epoch_state::EpochState,
    on_chain_config::{
        LeaderReputationType, OnChainConfigPayload, OnChainConsensusConfig, ProposerElectionType,
        ValidatorSet,
    },
    validator_verifier::ValidatorVerifier,
};
use channel::{aptos_channel, message_queues::QueueStyle};
use consensus_types::{
    common::{Author, Round},
    epoch_retrieval::EpochRetrievalRequest,
    request_response::ConsensusRequest,
};
use event_notifications::ReconfigNotificationListener;
use futures::{
    channel::{
        mpsc,
        mpsc::{unbounded, Receiver, Sender, UnboundedSender},
        oneshot,
    },
    SinkExt, StreamExt,
};
use itertools::Itertools;
use network::protocols::network::{ApplicationNetworkSender, Event};
use safety_rules::SafetyRulesManager;
use std::{
    cmp::Ordering,
    collections::HashMap,
    mem::{discriminant, Discriminant},
    sync::{
        atomic::{AtomicBool, AtomicU64},
        Arc,
    },
    thread,
    time::{Duration, Instant},
};

/// Range of rounds (window) that we might be calling proposer election
/// functions with at any given time, in addition to the proposer history length.
const PROPSER_ELECTION_CACHING_WINDOW_ADDITION: usize = 3;
/// Number of rounds we expect storage to be ahead of the proposer round,
/// used for fetching data from DB.
const PROPSER_ROUND_BEHIND_STORAGE_BUFFER: usize = 10;

#[allow(clippy::large_enum_variant)]
pub enum LivenessStorageData {
    RecoveryData(RecoveryData),
    LedgerRecoveryData(LedgerRecoveryData),
}

impl LivenessStorageData {
    pub fn expect_recovery_data(self, msg: &str) -> RecoveryData {
        match self {
            LivenessStorageData::RecoveryData(data) => data,
            LivenessStorageData::LedgerRecoveryData(_) => panic!("{}", msg),
        }
    }
}

// Manager the components that shared across epoch and spawn per-epoch RoundManager with
// epoch-specific input.
pub struct EpochManager {
    author: Author,
    config: ConsensusConfig,
    time_service: Arc<dyn TimeService>,
    self_sender: channel::Sender<Event<ConsensusMsg>>,
    network_sender: ConsensusNetworkSender,
    timeout_sender: channel::Sender<Round>,
    quorum_store_to_mempool_sender: Sender<QuorumStoreRequest>,
    commit_state_computer: Arc<dyn StateComputer>,
    storage: Arc<dyn PersistentLivenessStorage>,
    safety_rules_manager: SafetyRulesManager,
    reconfig_events: ReconfigNotificationListener,
    commit_notifier: Arc<dyn CommitNotifier>,
    // channels to buffer manager
    buffer_manager_msg_tx: Option<aptos_channel::Sender<AccountAddress, VerifiedEvent>>,
    buffer_manager_reset_tx: Option<UnboundedSender<ResetRequest>>,
    // channels to round manager
    round_manager_tx: Option<
        aptos_channel::Sender<(Author, Discriminant<VerifiedEvent>), (Author, VerifiedEvent)>,
    >,
    round_manager_close_tx: Option<oneshot::Sender<oneshot::Sender<()>>>,
    epoch_state: Option<EpochState>,
    block_retrieval_tx:
        Option<aptos_channel::Sender<AccountAddress, IncomingBlockRetrievalRequest>>,
}

impl EpochManager {
    pub fn new(
        node_config: &NodeConfig,
        time_service: Arc<dyn TimeService>,
        self_sender: channel::Sender<Event<ConsensusMsg>>,
        network_sender: ConsensusNetworkSender,
        timeout_sender: channel::Sender<Round>,
        quorum_store_to_mempool_sender: Sender<QuorumStoreRequest>,
        commit_state_computer: Arc<dyn StateComputer>,
        storage: Arc<dyn PersistentLivenessStorage>,
        reconfig_events: ReconfigNotificationListener,
        commit_notifier: Arc<dyn CommitNotifier>,
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
            quorum_store_to_mempool_sender,
            commit_state_computer,
            storage,
            safety_rules_manager,
            reconfig_events,
            commit_notifier,
            buffer_manager_msg_tx: None,
            buffer_manager_reset_tx: None,
            round_manager_tx: None,
            round_manager_close_tx: None,
            epoch_state: None,
            block_retrieval_tx: None,
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
        timeout_sender: channel::Sender<Round>,
    ) -> RoundState {
        // 1.5^6 ~= 11
        // Timeout goes from initial_timeout to initial_timeout*11 in 6 steps
        let time_interval = Box::new(ExponentialTimeInterval::new(
            Duration::from_millis(self.config.round_initial_timeout_ms),
            1.2,
            6,
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
            }
            // We don't really have a fixed proposer!
            ProposerElectionType::FixedProposer(contiguous_rounds) => {
                let proposer = choose_leader(proposers);
                Box::new(RotatingProposer::new(vec![proposer], *contiguous_rounds))
            }
            ProposerElectionType::LeaderReputation(leader_reputation_type) => {
                let (
                    heuristic,
                    window_size,
                    weight_by_voting_power,
                    use_history_from_previous_epoch_max_count,
                ) = match &leader_reputation_type {
                    LeaderReputationType::ProposerAndVoter(proposer_and_voter_config) => {
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
                            ));
                        (
                            heuristic,
                            std::cmp::max(proposer_window_size, voter_window_size),
                            proposer_and_voter_config.weight_by_voting_power,
                            proposer_and_voter_config.use_history_from_previous_epoch_max_count,
                        )
                    }
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

                // First block (after genesis) is epoch=1, so that is the first epoch we consider. (Genesis is epoch=0)
                let first_epoch_to_consider = std::cmp::max(
                    1,
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
                ));
                // LeaderReputation is not cheap, so we can cache the amount of rounds round_manager needs.
                Box::new(CachedProposerElection::new(
                    proposer_election,
                    onchain_config.max_failed_authors_to_store()
                        + PROPSER_ELECTION_CACHING_WINDOW_ADDITION,
                ))
            }
            ProposerElectionType::RoundProposer(round_proposers) => {
                // Hardcoded to the first proposer
                let default_proposer = proposers.get(0).unwrap();
                Box::new(RoundProposer::new(
                    round_proposers.clone(),
                    *default_proposer,
                ))
            }
        }
    }

    async fn process_epoch_retrieval(
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

    async fn process_different_epoch(
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
            // Ignore message from lower epoch, the node would see message from higher epoch and request
            // a proof
            Ordering::Less => {
                sample!(
                    SampleRate::Duration(Duration::from_secs(1)),
                    debug!("Discard message from lower epoch");
                );
                Ok(())
            }
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
            }
            Ordering::Equal => {
                bail!("[EpochManager] Same epoch should not come to process_different_epoch");
            }
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

    fn spawn_quorum_store(
        &mut self,
        consensus_to_quorum_store_receiver: Receiver<ConsensusRequest>,
    ) {
        let quorum_store = DirectMempoolQuorumStore::new(
            consensus_to_quorum_store_receiver,
            self.quorum_store_to_mempool_sender.clone(),
            self.config.mempool_txn_pull_timeout_ms,
        );
        spawn_named!("Quorum Store", quorum_store.start());
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
                    error!(epoch = epoch, error = ?e, kind = error_kind(&e));
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
            self.config.channel_size,
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
    }

    async fn start_round_manager(
        &mut self,
        recovery_data: RecoveryData,
        epoch_state: EpochState,
        onchain_config: OnChainConsensusConfig,
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
        let proposer_election = self.create_proposer_election(&epoch_state, &onchain_config);
        let network_sender = NetworkSender::new(
            self.author,
            self.network_sender.clone(),
            self.self_sender.clone(),
            epoch_state.verifier.clone(),
        );

        let safety_rules_container = Arc::new(Mutex::new(safety_rules));

        let (consensus_to_quorum_store_sender, consensus_to_quorum_store_receiver) =
            mpsc::channel(self.config.intra_consensus_channel_buffer_size);
        self.spawn_quorum_store(consensus_to_quorum_store_receiver);
        let payload_manager = QuorumStoreClient::new(
            consensus_to_quorum_store_sender.clone(),
            self.config.quorum_store_poll_count,
            self.config.quorum_store_pull_timeout_ms,
        );
        self.commit_notifier
            .new_epoch(consensus_to_quorum_store_sender);

        self.commit_state_computer.new_epoch(&epoch_state);
        let state_computer = if onchain_config.decoupled_execution() {
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
            onchain_config.back_pressure_limit(),
        ));

        info!(epoch = epoch, "Create ProposalGenerator");
        // txn manager is required both by proposal generator (to pull the proposers)
        // and by event processor (to update their status).
        let proposal_generator = ProposalGenerator::new(
            self.author,
            block_store.clone(),
            Arc::new(payload_manager),
            self.time_service.clone(),
            self.config.max_block_txns,
            self.config.max_block_bytes,
            onchain_config.max_failed_authors_to_store(),
        );

        let mut round_manager = RoundManager::new(
            epoch_state,
            block_store.clone(),
            round_state,
            proposer_election,
            proposal_generator,
            safety_rules_container,
            network_sender,
            self.storage.clone(),
            self.config.sync_only,
            onchain_config,
        );

        round_manager.init(last_vote).await;
        let (round_manager_tx, round_manager_rx) = aptos_channel::new(
            QueueStyle::LIFO,
            1,
            Some(&counters::ROUND_MANAGER_CHANNEL_MSGS),
        );
        self.round_manager_tx = Some(round_manager_tx);
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

        let onchain_config: anyhow::Result<OnChainConsensusConfig> = payload.get();
        if let Err(error) = &onchain_config {
            error!("Failed to read on-chain consensus config {}", error);
        }

        self.epoch_state = Some(epoch_state.clone());

        let initial_data = self
            .storage
            .start()
            .expect_recovery_data("Consensusdb is corrupted, need to do a backup and restore");
        self.start_round_manager(
            initial_data,
            epoch_state,
            onchain_config.unwrap_or_default(),
        )
        .await;
    }

    async fn process_message(
        &mut self,
        peer_id: AccountAddress,
        consensus_msg: ConsensusMsg,
    ) -> anyhow::Result<()> {
        // we can't verify signatures from a different epoch
        let maybe_unverified_event = self.check_epoch(peer_id, consensus_msg).await?;

        if let Some(unverified_event) = maybe_unverified_event {
            // same epoch -> run well-formedness + signature check
            let verified_event = monitor!(
                "verify_message",
                unverified_event
                    .clone()
                    .verify(&self.epoch_state().verifier)
            )
            .context("[EpochManager] Verify event")
            .map_err(|err| {
                error!(
                    SecurityEvent::ConsensusInvalidMessage,
                    remote_peer = peer_id,
                    error = ?err,
                    unverified_event = unverified_event
                );
                err
            })?;

            // process the verified event
            self.process_event(peer_id, verified_event)?;
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
            | ConsensusMsg::CommitDecisionMsg(_) => {
                let event: UnverifiedEvent = msg.into();
                if event.epoch() == self.epoch() {
                    return Ok(Some(event));
                } else {
                    monitor!(
                        "process_different_epoch_consensus_msg",
                        self.process_different_epoch(event.epoch(), peer_id).await
                    )?;
                }
            }
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
            }
            ConsensusMsg::EpochRetrievalRequest(request) => {
                ensure!(
                    request.end_epoch <= self.epoch(),
                    "[EpochManager] Received EpochRetrievalRequest beyond what we have locally"
                );
                monitor!(
                    "process_epoch_retrieval",
                    self.process_epoch_retrieval(*request, peer_id).await
                )?;
            }
            _ => {
                bail!("[EpochManager] Unexpected messages: {:?}", msg);
            }
        }
        Ok(None)
    }

    fn process_event(
        &mut self,
        peer_id: AccountAddress,
        event: VerifiedEvent,
    ) -> anyhow::Result<()> {
        match event {
            buffer_manager_event @ (VerifiedEvent::CommitVote(_)
            | VerifiedEvent::CommitDecision(_)) => {
                if let Some(sender) = &mut self.buffer_manager_msg_tx {
                    monitor!(
                        "buffer_manager_push_message",
                        sender.push(peer_id, buffer_manager_event)?
                    );
                } else {
                    bail!("Commit Phase not started but received Commit Message (CommitVote/CommitDecision)");
                }
            }
            round_manager_event => self.forward_to_round_manager(peer_id, round_manager_event),
        }
        Ok(())
    }

    fn forward_to_round_manager(&mut self, peer_id: Author, event: VerifiedEvent) {
        let sender = self
            .round_manager_tx
            .as_mut()
            .expect("RoundManager not started");
        monitor!(
            "round_manager_push_message",
            if let Err(e) = sender.push((peer_id, discriminant(&event)), (peer_id, event)) {
                error!("Failed to send event to round manager {:?}", e);
            }
        );
    }

    async fn process_block_retrieval(
        &self,
        peer_id: Author,
        request: IncomingBlockRetrievalRequest,
    ) -> anyhow::Result<()> {
        if let Some(tx) = &self.block_retrieval_tx {
            tx.push(peer_id, request)
        } else {
            Err(anyhow::anyhow!("Round manager not started"))
        }
    }

    fn process_local_timeout(&mut self, round: u64) {
        self.forward_to_round_manager(self.author, VerifiedEvent::LocalTimeout(round));
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
        mut round_timeout_sender_rx: channel::Receiver<Round>,
        network_receivers: NetworkReceivers,
    ) {
        // initial start of the processor
        self.await_reconfig_notification().await;

        let active = Arc::new(AtomicBool::new(false));
        let active_copy = active.clone();
        let counter = Arc::new(AtomicU64::new(0));
        let counter_copy = counter.clone();
        let panic_duration_s = self.config.panic_on_stuck_epoch_manager_duration_s;

        let _handle = if panic_duration_s > 0 {
            Some(
                thread::Builder::new()
                    .name("panic_thread".to_string())
                    .spawn(move || {
                        let mut last_change = Instant::now();
                        let mut prev_value = 0;
                        loop {
                            let active = active_copy.load(std::sync::atomic::Ordering::Relaxed);
                            let cur_value = counter_copy.load(std::sync::atomic::Ordering::Relaxed);
                            if prev_value != cur_value {
                                debug!("Progress check passed {} < {}", prev_value, cur_value);
                                last_change = Instant::now();
                                prev_value = cur_value;
                            } else {
                                error!(
                                    "No progress in {}s, at {}, active: {}",
                                    last_change.elapsed().as_secs(),
                                    cur_value,
                                    active
                                );
                                if last_change.elapsed().as_secs() > panic_duration_s && !active {
                                    panic!(
                                        "Crashing after {}s no progress",
                                        last_change.elapsed().as_secs()
                                    );
                                }
                            }

                            std::thread::sleep(Duration::from_secs(30));
                        }
                    })
                    .unwrap(),
            )
        } else {
            None
        };

        let epoch_manager_shared = Arc::new(tokio::sync::Mutex::new(self));
        let epoch_manager_shared_1 = epoch_manager_shared.clone();
        let epoch_manager_shared_2 = epoch_manager_shared.clone();
        let NetworkReceivers {
            mut consensus_messages,
            mut block_retrieval,
        } = network_receivers;

        let consensus_task = async move {
            while let Some((peer, msg)) = consensus_messages.next().await {
                let mut locked = epoch_manager_shared.lock().await;
                active.store(true, std::sync::atomic::Ordering::Relaxed);
                counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                monitor!(
                    "process_message",
                    if let Err(e) = locked.process_message(peer, msg).await {
                        error!(epoch = locked.epoch(), error = ?e, kind = error_kind(&e));
                    }
                );
                active.store(false, std::sync::atomic::Ordering::Relaxed);
            }
        };
        let block_task = async move {
            while let Some((peer, request)) = block_retrieval.next().await {
                let locked = epoch_manager_shared_1.lock().await;
                monitor!(
                    "send_block_retrieval",
                    if let Err(e) = locked.process_block_retrieval(peer, request).await {
                        error!(epoch = locked.epoch(), error = ?e, kind = error_kind(&e));
                    }
                );
            }
        };
        let timeout_task = async move {
            while let Some(round) = round_timeout_sender_rx.next().await {
                monitor!(
                    "send_timeout",
                    epoch_manager_shared_2
                        .lock()
                        .await
                        .process_local_timeout(round)
                );
            }
        };
        tokio::spawn(consensus_task);
        tokio::spawn(block_task);
        tokio::spawn(timeout_task);
    }
}
