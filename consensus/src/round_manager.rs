// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    block_storage::{
        tracing::{observe_block, BlockStage},
        BlockReader, BlockRetriever, BlockStore,
    },
    counters::{
        self, ORDER_CERT_CREATED_WITHOUT_BLOCK_IN_BLOCK_STORE, ORDER_VOTE_ADDED,
        ORDER_VOTE_BROADCASTED, ORDER_VOTE_OTHER_ERRORS, ORDER_VOTE_VERY_OLD, PROPOSAL_VOTE_ADDED,
        PROPOSAL_VOTE_BROADCASTED, PROPOSED_VTXN_BYTES, PROPOSED_VTXN_COUNT,
        QC_AGGREGATED_FROM_VOTES, SYNC_INFO_RECEIVED_WITH_NEWER_CERT,
    },
    error::{error_kind, VerifyError},
    liveness::{
        proposal_generator::ProposalGenerator,
        proposer_election::ProposerElection,
        round_state::{NewRoundEvent, NewRoundReason, RoundState, RoundStateLogSchema},
        unequivocal_proposer_election::UnequivocalProposerElection,
    },
    logging::{LogEvent, LogSchema},
    metrics_safety_rules::MetricsSafetyRules,
    monitor,
    network::NetworkSender,
    network_interface::ConsensusMsg,
    pending_order_votes::{OrderVoteReceptionResult, PendingOrderVotes},
    pending_votes::VoteReceptionResult,
    persistent_liveness_storage::PersistentLivenessStorage,
    quorum_store::types::BatchMsg,
    rand::rand_gen::types::{FastShare, RandConfig, Share, TShare},
    util::is_vtxn_expected,
};
use anyhow::{bail, ensure, Context};
use aptos_channels::aptos_channel;
use aptos_config::config::ConsensusConfig;
use aptos_consensus_types::{
    block::Block,
    block_data::BlockType,
    common::{Author, Round},
    delayed_qc_msg::DelayedQcMsg,
    order_vote_msg::OrderVoteMsg,
    proof_of_store::{ProofCache, ProofOfStoreMsg, SignedBatchInfoMsg},
    proposal_msg::ProposalMsg,
    quorum_cert::QuorumCert,
    sync_info::SyncInfo,
    timeout_2chain::TwoChainTimeoutCertificate,
    vote::Vote,
    vote_data::VoteData,
    vote_msg::VoteMsg,
    wrapped_ledger_info::WrappedLedgerInfo,
};
use aptos_crypto::HashValue;
use aptos_infallible::{checked, Mutex};
use aptos_logger::prelude::*;
#[cfg(test)]
use aptos_safety_rules::ConsensusState;
use aptos_safety_rules::TSafetyRules;
use aptos_types::{
    block_info::BlockInfo,
    epoch_state::EpochState,
    on_chain_config::{
        OnChainConsensusConfig, OnChainJWKConsensusConfig, OnChainRandomnessConfig,
        ValidatorTxnConfig,
    },
    randomness::RandMetadata,
    validator_verifier::ValidatorVerifier,
    PeerId,
};
use fail::fail_point;
use futures::{channel::oneshot, FutureExt, StreamExt};
use futures_channel::mpsc::UnboundedReceiver;
use lru::LruCache;
use serde::Serialize;
use std::{mem::Discriminant, sync::Arc, time::Duration};
use tokio::{
    sync::oneshot as TokioOneshot,
    time::{sleep, Instant},
};

#[derive(Serialize, Clone)]
pub enum UnverifiedEvent {
    ProposalMsg(Box<ProposalMsg>),
    VoteMsg(Box<VoteMsg>),
    OrderVoteMsg(Box<OrderVoteMsg>),
    SyncInfo(Box<SyncInfo>),
    BatchMsg(Box<BatchMsg>),
    SignedBatchInfo(Box<SignedBatchInfoMsg>),
    ProofOfStoreMsg(Box<ProofOfStoreMsg>),
}

pub const BACK_PRESSURE_POLLING_INTERVAL_MS: u64 = 10;

impl UnverifiedEvent {
    pub fn verify(
        self,
        peer_id: PeerId,
        validator: &ValidatorVerifier,
        proof_cache: &ProofCache,
        quorum_store_enabled: bool,
        self_message: bool,
        max_num_batches: usize,
        max_batch_expiry_gap_usecs: u64,
    ) -> Result<VerifiedEvent, VerifyError> {
        let start_time = Instant::now();
        Ok(match self {
            //TODO: no need to sign and verify the proposal
            UnverifiedEvent::ProposalMsg(p) => {
                if !self_message {
                    p.verify(validator, proof_cache, quorum_store_enabled)?;
                    counters::VERIFY_MSG
                        .with_label_values(&["proposal"])
                        .observe(start_time.elapsed().as_secs_f64());
                }
                VerifiedEvent::ProposalMsg(p)
            },
            UnverifiedEvent::VoteMsg(v) => {
                if !self_message {
                    v.verify(validator)?;
                    counters::VERIFY_MSG
                        .with_label_values(&["vote"])
                        .observe(start_time.elapsed().as_secs_f64());
                }
                VerifiedEvent::VoteMsg(v)
            },
            UnverifiedEvent::OrderVoteMsg(v) => {
                if !self_message {
                    v.verify(validator)?;
                    counters::VERIFY_MSG
                        .with_label_values(&["order_vote"])
                        .observe(start_time.elapsed().as_secs_f64());
                }
                VerifiedEvent::OrderVoteMsg(v)
            },
            // sync info verification is on-demand (verified when it's used)
            UnverifiedEvent::SyncInfo(s) => VerifiedEvent::UnverifiedSyncInfo(s),
            UnverifiedEvent::BatchMsg(b) => {
                if !self_message {
                    b.verify(peer_id, max_num_batches)?;
                    counters::VERIFY_MSG
                        .with_label_values(&["batch"])
                        .observe(start_time.elapsed().as_secs_f64());
                }
                VerifiedEvent::BatchMsg(b)
            },
            UnverifiedEvent::SignedBatchInfo(sd) => {
                if !self_message {
                    sd.verify(
                        peer_id,
                        max_num_batches,
                        max_batch_expiry_gap_usecs,
                        validator,
                    )?;
                    counters::VERIFY_MSG
                        .with_label_values(&["signed_batch"])
                        .observe(start_time.elapsed().as_secs_f64());
                }
                VerifiedEvent::SignedBatchInfo(sd)
            },
            UnverifiedEvent::ProofOfStoreMsg(p) => {
                if !self_message {
                    p.verify(max_num_batches, validator, proof_cache)?;
                    counters::VERIFY_MSG
                        .with_label_values(&["proof_of_store"])
                        .observe(start_time.elapsed().as_secs_f64());
                }
                VerifiedEvent::ProofOfStoreMsg(p)
            },
        })
    }

    pub fn epoch(&self) -> anyhow::Result<u64> {
        match self {
            UnverifiedEvent::ProposalMsg(p) => Ok(p.epoch()),
            UnverifiedEvent::VoteMsg(v) => Ok(v.epoch()),
            UnverifiedEvent::OrderVoteMsg(v) => Ok(v.epoch()),
            UnverifiedEvent::SyncInfo(s) => Ok(s.epoch()),
            UnverifiedEvent::BatchMsg(b) => b.epoch(),
            UnverifiedEvent::SignedBatchInfo(sd) => sd.epoch(),
            UnverifiedEvent::ProofOfStoreMsg(p) => p.epoch(),
        }
    }
}

impl From<ConsensusMsg> for UnverifiedEvent {
    fn from(value: ConsensusMsg) -> Self {
        match value {
            ConsensusMsg::ProposalMsg(m) => UnverifiedEvent::ProposalMsg(m),
            ConsensusMsg::VoteMsg(m) => UnverifiedEvent::VoteMsg(m),
            ConsensusMsg::OrderVoteMsg(m) => UnverifiedEvent::OrderVoteMsg(m),
            ConsensusMsg::SyncInfo(m) => UnverifiedEvent::SyncInfo(m),
            ConsensusMsg::BatchMsg(m) => UnverifiedEvent::BatchMsg(m),
            ConsensusMsg::SignedBatchInfo(m) => UnverifiedEvent::SignedBatchInfo(m),
            ConsensusMsg::ProofOfStoreMsg(m) => UnverifiedEvent::ProofOfStoreMsg(m),
            _ => unreachable!("Unexpected conversion"),
        }
    }
}

#[derive(Debug)]
pub enum VerifiedEvent {
    // network messages
    ProposalMsg(Box<ProposalMsg>),
    VerifiedProposalMsg(Box<Block>),
    VoteMsg(Box<VoteMsg>),
    OrderVoteMsg(Box<OrderVoteMsg>),
    UnverifiedSyncInfo(Box<SyncInfo>),
    BatchMsg(Box<BatchMsg>),
    SignedBatchInfo(Box<SignedBatchInfoMsg>),
    ProofOfStoreMsg(Box<ProofOfStoreMsg>),
    // local messages
    LocalTimeout(Round),
    // Shutdown the NetworkListener
    Shutdown(TokioOneshot::Sender<()>),
}

#[cfg(test)]
#[path = "round_manager_test.rs"]
mod round_manager_test;

#[cfg(feature = "fuzzing")]
#[path = "round_manager_fuzzing.rs"]
pub mod round_manager_fuzzing;

/// Consensus SMR is working in an event based fashion: RoundManager is responsible for
/// processing the individual events (e.g., process_new_round, process_proposal, process_vote,
/// etc.). It is exposing the async processing functions for each event type.
/// The caller is responsible for running the event loops and driving the execution via some
/// executors.
pub struct RoundManager {
    epoch_state: Arc<EpochState>,
    block_store: Arc<BlockStore>,
    round_state: RoundState,
    proposer_election: UnequivocalProposerElection,
    proposal_generator: ProposalGenerator,
    safety_rules: Arc<Mutex<MetricsSafetyRules>>,
    network: Arc<NetworkSender>,
    storage: Arc<dyn PersistentLivenessStorage>,
    onchain_config: OnChainConsensusConfig,
    vtxn_config: ValidatorTxnConfig,
    buffered_proposal_tx: aptos_channel::Sender<Author, VerifiedEvent>,
    local_config: ConsensusConfig,
    randomness_config: OnChainRandomnessConfig,
    jwk_consensus_config: OnChainJWKConsensusConfig,
    fast_rand_config: Option<RandConfig>,
    // Stores the order votes from all the rounds above highest_ordered_round
    pending_order_votes: PendingOrderVotes,
    // Round manager broadcasts fast shares when forming a QC or when receiving a proposal.
    // To avoid duplicate broadcasts for the same block, we keep track of blocks for
    // which we recently broadcasted fast shares.
    blocks_with_broadcasted_fast_shares: LruCache<HashValue, ()>,
}

impl RoundManager {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        epoch_state: Arc<EpochState>,
        block_store: Arc<BlockStore>,
        round_state: RoundState,
        proposer_election: Arc<dyn ProposerElection + Send + Sync>,
        proposal_generator: ProposalGenerator,
        safety_rules: Arc<Mutex<MetricsSafetyRules>>,
        network: Arc<NetworkSender>,
        storage: Arc<dyn PersistentLivenessStorage>,
        onchain_config: OnChainConsensusConfig,
        buffered_proposal_tx: aptos_channel::Sender<Author, VerifiedEvent>,
        local_config: ConsensusConfig,
        randomness_config: OnChainRandomnessConfig,
        jwk_consensus_config: OnChainJWKConsensusConfig,
        fast_rand_config: Option<RandConfig>,
    ) -> Self {
        // when decoupled execution is false,
        // the counter is still static.
        counters::OP_COUNTERS
            .gauge("sync_only")
            .set(local_config.sync_only as i64);
        counters::OP_COUNTERS
            .gauge("decoupled_execution")
            .set(onchain_config.decoupled_execution() as i64);
        let vtxn_config = onchain_config.effective_validator_txn_config();
        debug!("vtxn_config={:?}", vtxn_config);
        Self {
            epoch_state,
            block_store,
            round_state,
            proposer_election: UnequivocalProposerElection::new(proposer_election),
            proposal_generator,
            safety_rules,
            network,
            storage,
            onchain_config,
            vtxn_config,
            buffered_proposal_tx,
            local_config,
            randomness_config,
            jwk_consensus_config,
            fast_rand_config,
            pending_order_votes: PendingOrderVotes::new(),
            blocks_with_broadcasted_fast_shares: LruCache::new(5),
        }
    }

    // TODO: Evaluate if creating a block retriever is slow and cache this if needed.
    fn create_block_retriever(&self, author: Author) -> BlockRetriever {
        BlockRetriever::new(
            self.network.clone(),
            author,
            self.epoch_state
                .verifier
                .get_ordered_account_addresses_iter()
                .collect(),
            self.local_config
                .max_blocks_per_sending_request(self.onchain_config.quorum_store_enabled()),
            self.block_store.pending_blocks(),
        )
    }

    /// Leader:
    ///
    /// This event is triggered by a new quorum certificate at the previous round or a
    /// timeout certificate at the previous round.  In either case, if this replica is the new
    /// proposer for this round, it is ready to propose and guarantee that it can create a proposal
    /// that all honest replicas can vote for.  While this method should only be invoked at most
    /// once per round, we ensure that only at most one proposal can get generated per round to
    /// avoid accidental equivocation of proposals.
    ///
    /// Replica:
    ///
    /// Do nothing
    async fn process_new_round_event(
        &mut self,
        new_round_event: NewRoundEvent,
    ) -> anyhow::Result<()> {
        counters::CURRENT_ROUND.set(new_round_event.round as i64);
        counters::ROUND_TIMEOUT_MS.with_label_values(&[&new_round_event.round.to_string()]).inc_by(new_round_event.timeout.as_millis() as u64);
        match new_round_event.reason {
            NewRoundReason::QCReady => {
                counters::QC_ROUNDS_COUNT.inc();
            },
            NewRoundReason::Timeout => {
                counters::TIMEOUT_ROUNDS_COUNT.inc();
            },
        };
        info!(
            self.new_log(LogEvent::NewRound),
            reason = new_round_event.reason
        );
        self.pending_order_votes
            .garbage_collect(self.block_store.sync_info().highest_ordered_round());

        if self
            .proposer_election
            .is_valid_proposer(self.proposal_generator.author(), new_round_event.round)
        {
            self.log_collected_vote_stats(&new_round_event);
            self.round_state.setup_leader_timeout();
            let proposal_msg = self.generate_proposal(new_round_event).await?;
            #[cfg(feature = "failpoints")]
            {
                if self.check_whether_to_inject_reconfiguration_error() {
                    self.attempt_to_inject_reconfiguration_error(&proposal_msg)
                        .await?;
                }
            }
            self.network.broadcast_proposal(proposal_msg).await;
            counters::PROPOSALS_COUNT.inc();
        }
        Ok(())
    }

    fn log_collected_vote_stats(&self, new_round_event: &NewRoundEvent) {
        let prev_round_votes_for_li = new_round_event
            .prev_round_votes
            .iter()
            .map(|(_, li_with_sig)| {
                let (voting_power, votes): (Vec<_>, Vec<_>) = li_with_sig
                    .signatures()
                    .keys()
                    .map(|author| {
                        self.epoch_state
                            .verifier
                            .get_voting_power(author)
                            .map(|voting_power| (voting_power as u128, 1))
                            .unwrap_or((0u128, 0))
                    })
                    .unzip();
                (voting_power.iter().sum(), votes.iter().sum())
            })
            .collect::<Vec<(u128, usize)>>();

        let (max_voting_power, max_num_votes) = prev_round_votes_for_li
            .iter()
            .max()
            .cloned()
            .unwrap_or((0, 0));

        let (voting_powers, votes_counts): (Vec<_>, Vec<_>) =
            prev_round_votes_for_li.iter().cloned().unzip();
        let conflicting_voting_power = voting_powers.into_iter().sum::<u128>() - max_voting_power;
        let conflicting_num_votes = votes_counts.into_iter().sum::<usize>() - max_num_votes;

        let (timeout_voting_power, timeout_num_votes) = new_round_event
            .prev_round_timeout_votes
            .as_ref()
            .map(|timeout_votes| {
                let (voting_power, votes): (Vec<_>, Vec<_>) = timeout_votes
                    .signers()
                    .map(|author| {
                        self.epoch_state
                            .verifier
                            .get_voting_power(author)
                            .map(|voting_power| (voting_power as u128, 1))
                            .unwrap_or((0u128, 0))
                    })
                    .unzip();
                (voting_power.iter().sum(), votes.iter().sum())
            })
            .unwrap_or((0, 0));

        counters::PROPOSER_COLLECTED_ROUND_COUNT.inc();
        counters::PROPOSER_COLLECTED_MOST_VOTING_POWER.inc_by(max_voting_power as f64);
        counters::PROPOSER_COLLECTED_CONFLICTING_VOTING_POWER
            .inc_by(conflicting_voting_power as f64);
        counters::PROPOSER_COLLECTED_TIMEOUT_VOTING_POWER.inc_by(timeout_voting_power as f64);

        info!(
            epoch = self.epoch_state.epoch,
            round = new_round_event.round,
            total_voting_power = ?self.epoch_state.verifier.total_voting_power(),
            max_voting_power = ?max_voting_power,
            max_num_votes = max_num_votes,
            conflicting_voting_power = ?conflicting_voting_power,
            conflicting_num_votes = conflicting_num_votes,
            timeout_voting_power = ?timeout_voting_power,
            timeout_num_votes = timeout_num_votes,
            "Preparing new proposal",
        );
    }

    async fn generate_proposal(
        &mut self,
        new_round_event: NewRoundEvent,
    ) -> anyhow::Result<ProposalMsg> {
        // Proposal generator will ensure that at most one proposal is generated per round
        let sync_info = self.block_store.sync_info();
        let sender = self.network.clone();
        let callback = async move {
            sender.broadcast_sync_info(sync_info).await;
        }
        .boxed();

        let proposal = self
            .proposal_generator
            .generate_proposal(new_round_event.round, &mut self.proposer_election, callback)
            .await?;
        let signature = self.safety_rules.lock().sign_proposal(&proposal)?;
        let signed_proposal =
            Block::new_proposal_from_block_data_and_signature(proposal, signature);
        observe_block(signed_proposal.timestamp_usecs(), BlockStage::SIGNED);
        info!(self.new_log(LogEvent::Propose), "{}", signed_proposal);
        Ok(ProposalMsg::new(
            signed_proposal,
            self.block_store.sync_info(),
        ))
    }

    /// Process the proposal message:
    /// 1. ensure after processing sync info, we're at the same round as the proposal
    /// 2. execute and decide whether to vote for the proposal
    pub async fn process_proposal_msg(&mut self, proposal_msg: ProposalMsg) -> anyhow::Result<()> {
        fail_point!("consensus::process_proposal_msg", |_| {
            Err(anyhow::anyhow!("Injected error in process_proposal_msg"))
        });

        observe_block(
            proposal_msg.proposal().timestamp_usecs(),
            BlockStage::ROUND_MANAGER_RECEIVED,
        );
        info!(
            self.new_log(LogEvent::ReceiveProposal)
                .remote_peer(proposal_msg.proposer()),
            block_round = proposal_msg.proposal().round(),
            block_hash = proposal_msg.proposal().id(),
            block_parent_hash = proposal_msg.proposal().quorum_cert().certified_block().id(),
        );

        if self
            .ensure_round_and_sync_up(
                proposal_msg.proposal().round(),
                proposal_msg.sync_info(),
                proposal_msg.proposer(),
            )
            .await
            .context("[RoundManager] Process proposal")?
        {
            self.process_proposal(proposal_msg.take_proposal()).await
        } else {
            bail!(
                "Stale proposal {}, current round {}",
                proposal_msg.proposal(),
                self.round_state.current_round()
            );
        }
    }

    pub async fn process_delayed_proposal_msg(&mut self, proposal: Block) -> anyhow::Result<()> {
        if proposal.round() != self.round_state.current_round() {
            bail!(
                "Discarding stale delayed proposal {}, current round {}",
                proposal,
                self.round_state.current_round()
            );
        }

        self.process_verified_proposal(proposal).await
    }

    pub async fn process_delayed_qc_msg(&mut self, msg: DelayedQcMsg) -> anyhow::Result<()> {
        ensure!(
            msg.vote.vote_data().proposed().round() == self.round_state.current_round(),
            "Discarding stale delayed QC for round {}, current round {}",
            msg.vote.vote_data().proposed().round(),
            self.round_state.current_round()
        );
        let vote = msg.vote().clone();
        let vote_reception_result = self
            .round_state
            .process_delayed_qc_msg(&self.epoch_state.verifier, msg)
            .await;
        trace!(
            "Received delayed QC message and vote reception result is {:?}",
            vote_reception_result
        );
        self.process_vote_reception_result(&vote, vote_reception_result)
            .await
    }

    /// Sync to the sync info sending from peer if it has newer certificates.
    async fn sync_up(&mut self, sync_info: &SyncInfo, author: Author) -> anyhow::Result<()> {
        let local_sync_info = self.block_store.sync_info();
        if sync_info.has_newer_certificates(&local_sync_info) {
            info!(
                self.new_log(LogEvent::ReceiveNewCertificate)
                    .remote_peer(author),
                "Local state {},\n remote state {}", local_sync_info, sync_info
            );
            // Some information in SyncInfo is ahead of what we have locally.
            // First verify the SyncInfo (didn't verify it in the yet).
            sync_info
                .verify(&self.epoch_state().verifier)
                .map_err(|e| {
                    error!(
                        SecurityEvent::InvalidSyncInfoMsg,
                        sync_info = sync_info,
                        remote_peer = author,
                        error = ?e,
                    );
                    VerifyError::from(e)
                })?;
            SYNC_INFO_RECEIVED_WITH_NEWER_CERT.inc();
            let result = self
                .block_store
                .add_certs(sync_info, self.create_block_retriever(author))
                .await;
            self.process_certificates().await?;
            result
        } else {
            Ok(())
        }
    }

    /// The function makes sure that it ensures the message_round equal to what we have locally,
    /// brings the missing dependencies from the QC and LedgerInfo of the given sync info and
    /// update the round_state with the certificates if succeed.
    /// Returns Ok(true) if the sync succeeds and the round matches so we can process further.
    /// Returns Ok(false) if the message is stale.
    /// Returns Error in case sync mgr failed to bring the missing dependencies.
    /// We'll try to help the remote if the SyncInfo lags behind and the flag is set.
    pub async fn ensure_round_and_sync_up(
        &mut self,
        message_round: Round,
        sync_info: &SyncInfo,
        author: Author,
    ) -> anyhow::Result<bool> {
        if message_round < self.round_state.current_round() {
            return Ok(false);
        }
        self.sync_up(sync_info, author).await?;
        ensure!(
            message_round == self.round_state.current_round(),
            "After sync, round {} doesn't match local {}. Local Sync Info: {}. Remote Sync Info: {}",
            message_round,
            self.round_state.current_round(),
            self.block_store.sync_info(),
            sync_info,
        );
        Ok(true)
    }

    /// Process the SyncInfo sent by peers to catch up to latest state.
    pub async fn process_sync_info_msg(
        &mut self,
        sync_info: SyncInfo,
        peer: Author,
    ) -> anyhow::Result<()> {
        fail_point!("consensus::process_sync_info_msg", |_| {
            Err(anyhow::anyhow!("Injected error in process_sync_info_msg"))
        });
        info!(
            self.new_log(LogEvent::ReceiveSyncInfo).remote_peer(peer),
            "{}", sync_info
        );
        self.ensure_round_and_sync_up(checked!((sync_info.highest_round()) + 1)?, &sync_info, peer)
            .await
            .context("[RoundManager] Failed to process sync info msg")?;
        Ok(())
    }

    fn sync_only(&self) -> bool {
        let sync_or_not = self.local_config.sync_only || self.block_store.vote_back_pressure();
        if self.block_store.vote_back_pressure() {
            warn!("Vote back pressure is set");
        }
        counters::OP_COUNTERS
            .gauge("sync_only")
            .set(sync_or_not as i64);

        sync_or_not
    }

    /// The replica broadcasts a "timeout vote message", which includes the round signature, which
    /// can be aggregated to a TimeoutCertificate.
    /// The timeout vote message can be one of the following three options:
    /// 1) In case a validator has previously voted in this round, it repeats the same vote and sign
    /// a timeout.
    /// 2) Otherwise vote for a NIL block and sign a timeout.
    /// Note this function returns Err even if messages are broadcasted successfully because timeout
    /// is considered as error. It only returns Ok(()) when the timeout is stale.
    pub async fn process_local_timeout(&mut self, round: Round) -> anyhow::Result<()> {
        if !self.round_state.process_local_timeout(round) {
            return Ok(());
        }

        if self.sync_only() {
            self.network
                .broadcast_sync_info(self.block_store.sync_info())
                .await;
            bail!("[RoundManager] sync_only flag is set, broadcasting SyncInfo");
        }

        let (is_nil_vote, mut timeout_vote) = match self.round_state.vote_sent() {
            Some(vote) if vote.vote_data().proposed().round() == round => {
                (vote.vote_data().is_for_nil(), vote)
            },
            _ => {
                // Didn't vote in this round yet, generate a backup vote
                let nil_block = self
                    .proposal_generator
                    .generate_nil_block(round, &mut self.proposer_election)?;
                info!(
                    self.new_log(LogEvent::VoteNIL),
                    "Planning to vote for a NIL block {}", nil_block
                );
                counters::VOTE_NIL_COUNT.inc();
                let nil_vote = self.execute_and_vote(nil_block).await?;
                (true, nil_vote)
            },
        };

        if !timeout_vote.is_timeout() {
            let timeout = timeout_vote
                .generate_2chain_timeout(self.block_store.highest_quorum_cert().as_ref().clone());
            let signature = self
                .safety_rules
                .lock()
                .sign_timeout_with_qc(
                    &timeout,
                    self.block_store.highest_2chain_timeout_cert().as_deref(),
                )
                .context("[RoundManager] SafetyRules signs 2-chain timeout")?;
            timeout_vote.add_2chain_timeout(timeout, signature);
        }

        self.round_state.record_vote(timeout_vote.clone());
        let timeout_vote_msg = VoteMsg::new(timeout_vote, self.block_store.sync_info());
        self.network.broadcast_timeout_vote(timeout_vote_msg).await;
        warn!(
            round = round,
            remote_peer = self.proposer_election.get_valid_proposer(round),
            voted_nil = is_nil_vote,
            event = LogEvent::Timeout,
        );
        bail!("Round {} timeout, broadcast to all peers", round);
    }

    /// This function is called only after all the dependencies of the given QC have been retrieved.
    async fn process_certificates(&mut self) -> anyhow::Result<()> {
        let sync_info = self.block_store.sync_info();
        if let Some(new_round_event) = self.round_state.process_certificates(sync_info) {
            self.process_new_round_event(new_round_event).await?;
        }
        Ok(())
    }

    /// This function processes a proposal for the current round:
    /// 1. Filter if it's proposed by valid proposer.
    /// 2. Execute and add it to a block store.
    /// 3. Try to vote for it following the safety rules.
    /// 4. In case a validator chooses to vote, send the vote to the representatives at the next
    /// round.
    async fn process_proposal(&mut self, proposal: Block) -> anyhow::Result<()> {
        let author = proposal
            .author()
            .expect("Proposal should be verified having an author");

        if !self.vtxn_config.enabled()
            && matches!(
                proposal.block_data().block_type(),
                BlockType::ProposalExt(_)
            )
        {
            counters::UNEXPECTED_PROPOSAL_EXT_COUNT.inc();
            bail!("ProposalExt unexpected while the feature is disabled.");
        }

        if let Some(vtxns) = proposal.validator_txns() {
            for vtxn in vtxns {
                ensure!(
                    is_vtxn_expected(&self.randomness_config, &self.jwk_consensus_config, vtxn),
                    "unexpected validator txn: {:?}",
                    vtxn.topic()
                );
            }
        }

        let (num_validator_txns, validator_txns_total_bytes): (usize, usize) =
            proposal.validator_txns().map_or((0, 0), |txns| {
                txns.iter().fold((0, 0), |(count_acc, size_acc), txn| {
                    (count_acc + 1, size_acc + txn.size_in_bytes())
                })
            });

        let num_validator_txns = num_validator_txns as u64;
        let validator_txns_total_bytes = validator_txns_total_bytes as u64;
        let vtxn_count_limit = self.vtxn_config.per_block_limit_txn_count();
        let vtxn_bytes_limit = self.vtxn_config.per_block_limit_total_bytes();
        let author_hex = author.to_hex();
        PROPOSED_VTXN_COUNT
            .with_label_values(&[&author_hex])
            .inc_by(num_validator_txns);
        PROPOSED_VTXN_BYTES
            .with_label_values(&[&author_hex])
            .inc_by(validator_txns_total_bytes);
        info!(
            vtxn_count_limit = vtxn_count_limit,
            vtxn_count_proposed = num_validator_txns,
            vtxn_bytes_limit = vtxn_bytes_limit,
            vtxn_bytes_proposed = validator_txns_total_bytes,
            proposer = author_hex,
            "Summarizing proposed validator txns."
        );

        ensure!(
            num_validator_txns <= vtxn_count_limit,
            "process_proposal failed with per-block vtxn count limit exceeded: limit={}, actual={}",
            self.vtxn_config.per_block_limit_txn_count(),
            num_validator_txns
        );
        ensure!(
            validator_txns_total_bytes <= vtxn_bytes_limit,
            "process_proposal failed with per-block vtxn bytes limit exceeded: limit={}, actual={}",
            self.vtxn_config.per_block_limit_total_bytes(),
            validator_txns_total_bytes
        );
        let payload_len = proposal.payload().map_or(0, |payload| payload.len());
        let payload_size = proposal.payload().map_or(0, |payload| payload.size());
        ensure!(
            num_validator_txns + payload_len as u64 <= self.local_config.max_receiving_block_txns,
            "Payload len {} exceeds the limit {}",
            payload_len,
            self.local_config.max_receiving_block_txns,
        );

        ensure!(
            validator_txns_total_bytes + payload_size as u64
                <= self.local_config.max_receiving_block_bytes,
            "Payload size {} exceeds the limit {}",
            payload_size,
            self.local_config.max_receiving_block_bytes,
        );

        ensure!(
            self.proposer_election.is_valid_proposal(&proposal),
            "[RoundManager] Proposer {} for block {} is not a valid proposer for this round or created duplicate proposal",
            author,
            proposal,
        );

        // Validate that failed_authors list is correctly specified in the block.
        let expected_failed_authors = self.proposal_generator.compute_failed_authors(
            proposal.round(),
            proposal.quorum_cert().certified_block().round(),
            false,
            &mut self.proposer_election,
        );
        ensure!(
            proposal.block_data().failed_authors().map_or(false, |failed_authors| *failed_authors == expected_failed_authors),
            "[RoundManager] Proposal for block {} has invalid failed_authors list {:?}, expected {:?}",
            proposal.round(),
            proposal.block_data().failed_authors(),
            expected_failed_authors,
        );

        let block_time_since_epoch = Duration::from_micros(proposal.timestamp_usecs());

        ensure!(
            block_time_since_epoch < self.round_state.current_round_deadline(),
            "[RoundManager] Waiting until proposal block timestamp usecs {:?} \
            would exceed the round duration {:?}, hence will not vote for this round",
            block_time_since_epoch,
            self.round_state.current_round_deadline(),
        );

        observe_block(proposal.timestamp_usecs(), BlockStage::SYNCED);
        if self.block_store.vote_back_pressure() {
            counters::CONSENSUS_WITHOLD_VOTE_BACKPRESSURE_TRIGGERED.observe(1.0);
            // In case of back pressure, we delay processing proposal. This is done by resending the
            // same proposal to self after some time. Even if processing proposal is delayed, we add
            // the block to the block store so that we don't need to fetch it from remote once we
            // are out of the backpressure. Please note that delayed processing of proposal is not
            // guaranteed to add the block to the block store if we don't get out of the backpressure
            // before the timeout, so this is needed to ensure that the proposed block is added to
            // the block store irrespective. Also, it is possible that delayed processing of proposal
            // tries to add the same block again, which is okay as `execute_and_insert_block` call
            // is idempotent.
            self.block_store
                .insert_block(proposal.clone())
                .await
                .context("[RoundManager] Failed to execute_and_insert the block")?;
            self.resend_verified_proposal_to_self(
                proposal,
                author,
                BACK_PRESSURE_POLLING_INTERVAL_MS,
                self.local_config.round_initial_timeout_ms,
            )
            .await;
            Ok(())
        } else {
            counters::CONSENSUS_WITHOLD_VOTE_BACKPRESSURE_TRIGGERED.observe(0.0);
            self.process_verified_proposal(proposal).await
        }
    }

    async fn resend_verified_proposal_to_self(
        &self,
        proposal: Block,
        author: Author,
        polling_interval_ms: u64,
        timeout_ms: u64,
    ) {
        let start = Instant::now();
        let block_store = self.block_store.clone();
        let self_sender = self.buffered_proposal_tx.clone();
        let event = VerifiedEvent::VerifiedProposalMsg(Box::new(proposal));
        tokio::spawn(async move {
            while start.elapsed() < Duration::from_millis(timeout_ms) {
                if !block_store.vote_back_pressure() {
                    if let Err(e) = self_sender.push(author, event) {
                        warn!("Failed to send event to round manager {:?}", e);
                    }
                    break;
                }
                sleep(Duration::from_millis(polling_interval_ms)).await;
            }
        });
    }

    async fn broadcast_fast_shares(&mut self, block_info: &BlockInfo) {
        // generate and multicast randomness share for the fast path
        if let Some(fast_config) = &self.fast_rand_config {
            if !block_info.is_empty()
                && !self
                    .blocks_with_broadcasted_fast_shares
                    .contains(&block_info.id())
            {
                let metadata = RandMetadata {
                    epoch: block_info.epoch(),
                    round: block_info.round(),
                };
                let self_share = Share::generate(fast_config, metadata);
                let fast_share = FastShare::new(self_share);
                info!(LogSchema::new(LogEvent::BroadcastRandShareFastPath)
                    .epoch(fast_share.epoch())
                    .round(fast_share.round()));
                self.network.broadcast_fast_share(fast_share).await;
                self.blocks_with_broadcasted_fast_shares
                    .put(block_info.id(), ());
            }
        }
    }

    pub async fn process_verified_proposal(&mut self, proposal: Block) -> anyhow::Result<()> {
        let proposal_round = proposal.round();
        let vote = self
            .execute_and_vote(proposal)
            .await
            .context("[RoundManager] Process proposal")?;
        self.round_state.record_vote(vote.clone());
        let vote_msg = VoteMsg::new(vote.clone(), self.block_store.sync_info());

        self.broadcast_fast_shares(vote.ledger_info().commit_info())
            .await;

        if self.local_config.broadcast_vote {
            info!(self.new_log(LogEvent::Vote), "{}", vote);
            PROPOSAL_VOTE_BROADCASTED.inc();
            self.network.broadcast_vote(vote_msg).await;
        } else {
            let recipient = self
                .proposer_election
                .get_valid_proposer(proposal_round + 1);
            info!(
                self.new_log(LogEvent::Vote).remote_peer(recipient),
                "{}", vote
            );
            self.network.send_vote(vote_msg, vec![recipient]).await;
        }
        Ok(())
    }

    /// The function generates a VoteMsg for a given proposed_block:
    /// * first execute the block and add it to the block store
    /// * then verify the voting rules
    /// * save the updated state to consensus DB
    /// * return a VoteMsg with the LedgerInfo to be committed in case the vote gathers QC.
    async fn execute_and_vote(&mut self, proposed_block: Block) -> anyhow::Result<Vote> {
        let executed_block = self
            .block_store
            .insert_block(proposed_block)
            .await
            .context("[RoundManager] Failed to execute_and_insert the block")?;

        // Short circuit if already voted.
        ensure!(
            self.round_state.vote_sent().is_none(),
            "[RoundManager] Already vote on this round {}",
            self.round_state.current_round()
        );

        ensure!(
            !self.sync_only(),
            "[RoundManager] sync_only flag is set, stop voting"
        );

        let vote_proposal = executed_block.vote_proposal();
        let vote_result = self.safety_rules.lock().construct_and_sign_vote_two_chain(
            &vote_proposal,
            self.block_store.highest_2chain_timeout_cert().as_deref(),
        );
        let vote = vote_result.context(format!(
            "[RoundManager] SafetyRules Rejected {}",
            executed_block.block()
        ))?;
        if !executed_block.block().is_nil_block() {
            observe_block(executed_block.block().timestamp_usecs(), BlockStage::VOTED);
        }

        self.storage
            .save_vote(&vote)
            .context("[RoundManager] Fail to persist last vote")?;

        Ok(vote)
    }

    async fn process_order_vote_msg(&mut self, order_vote_msg: OrderVoteMsg) -> anyhow::Result<()> {
        if self.onchain_config.order_vote_enabled() {
            fail_point!("consensus::process_order_vote_msg", |_| {
                Err(anyhow::anyhow!("Injected error in process_order_vote_msg"))
            });
            info!(
                self.new_log(LogEvent::ReceiveOrderVote),
                "{}", order_vote_msg
            );

            if self
                .pending_order_votes
                .has_enough_order_votes(order_vote_msg.order_vote().ledger_info())
            {
                return Ok(());
            }

            if order_vote_msg.order_vote().ledger_info().round()
                > self.block_store.sync_info().highest_ordered_round()
            {
                let vote_reception_result = self
                    .pending_order_votes
                    .insert_order_vote(order_vote_msg.order_vote(), &self.epoch_state.verifier);
                self.process_order_vote_reception_result(&order_vote_msg, vote_reception_result)
                    .await?;
            } else {
                ORDER_VOTE_VERY_OLD.inc();
                info!(
                    "Received old order vote. Order vote round: {:?}, Highest ordered round: {:?}",
                    order_vote_msg.order_vote().ledger_info().round(),
                    self.block_store.sync_info().highest_ordered_round()
                );
            }
        }
        Ok(())
    }

    async fn broadcast_order_vote(
        &mut self,
        vote: &Vote,
        qc: Arc<QuorumCert>,
    ) -> anyhow::Result<()> {
        if let Some(proposed_block) = self.block_store.get_block(vote.vote_data().proposed().id()) {
            // Generate an order vote with ledger_info = proposed_block
            let order_vote_proposal = proposed_block.order_vote_proposal(qc.clone());
            let order_vote_result = self
                .safety_rules
                .lock()
                .construct_and_sign_order_vote(&order_vote_proposal);
            let order_vote = order_vote_result.context(format!(
                "[RoundManager] SafetyRules Rejected {} for order vote",
                proposed_block.block()
            ))?;
            if !proposed_block.block().is_nil_block() {
                observe_block(
                    proposed_block.block().timestamp_usecs(),
                    BlockStage::ORDER_VOTED,
                );
            }
            let order_vote_msg = OrderVoteMsg::new(order_vote.clone(), qc.as_ref().clone());
            info!(
                self.new_log(LogEvent::BroadcastOrderVote),
                "{}", order_vote_msg
            );
            self.network.broadcast_order_vote(order_vote_msg).await;
            ORDER_VOTE_BROADCASTED.inc();
        }
        Ok(())
    }

    /// Upon new vote:
    /// 1. Ensures we're processing the vote from the same round as local round
    /// 2. Filter out votes for rounds that should not be processed by this validator (to avoid
    /// potential attacks).
    /// 2. Add the vote to the pending votes and check whether it finishes a QC.
    /// 3. Once the QC/TC successfully formed, notify the RoundState.
    pub async fn process_vote_msg(&mut self, vote_msg: VoteMsg) -> anyhow::Result<()> {
        fail_point!("consensus::process_vote_msg", |_| {
            Err(anyhow::anyhow!("Injected error in process_vote_msg"))
        });
        // Check whether this validator is a valid recipient of the vote.
        if self
            .ensure_round_and_sync_up(
                vote_msg.vote().vote_data().proposed().round(),
                vote_msg.sync_info(),
                vote_msg.vote().author(),
            )
            .await
            .context("[RoundManager] Stop processing vote")?
        {
            self.process_vote(vote_msg.vote())
                .await
                .context("[RoundManager] Add a new vote")?;
        }
        Ok(())
    }

    /// Add a vote to the pending votes.
    /// If a new QC / TC is formed then
    /// 1) fetch missing dependencies if required, and then
    /// 2) call process_certificates(), which will start a new round in return.
    async fn process_vote(&mut self, vote: &Vote) -> anyhow::Result<()> {
        let round = vote.vote_data().proposed().round();

        info!(
            self.new_log(LogEvent::ReceiveVote)
                .remote_peer(vote.author()),
            vote = %vote,
            vote_epoch = vote.vote_data().proposed().epoch(),
            vote_round = vote.vote_data().proposed().round(),
            vote_id = vote.vote_data().proposed().id(),
            vote_state = vote.vote_data().proposed().executed_state_id(),
            is_timeout = vote.is_timeout(),
        );

        if !self.local_config.broadcast_vote && !vote.is_timeout() {
            // Unlike timeout votes regular votes are sent to the leaders of the next round only.
            let next_round = round + 1;
            ensure!(
                self.proposer_election
                    .is_valid_proposer(self.proposal_generator.author(), next_round),
                "[RoundManager] Received {}, but I am not a valid proposer for round {}, ignore.",
                vote,
                next_round
            );
        }

        let block_id = vote.vote_data().proposed().id();
        // Check if the block already had a QC
        if self
            .block_store
            .get_quorum_cert_for_block(block_id)
            .is_some()
        {
            return Ok(());
        }
        let vote_reception_result = self
            .round_state
            .insert_vote(vote, &self.epoch_state.verifier);
        self.process_vote_reception_result(vote, vote_reception_result)
            .await
    }

    async fn process_vote_reception_result(
        &mut self,
        vote: &Vote,
        result: VoteReceptionResult,
    ) -> anyhow::Result<()> {
        let round = vote.vote_data().proposed().round();
        match result {
            VoteReceptionResult::NewQuorumCertificate(qc) => {
                if !vote.is_timeout() {
                    observe_block(
                        qc.certified_block().timestamp_usecs(),
                        BlockStage::QC_AGGREGATED,
                    );
                }
                QC_AGGREGATED_FROM_VOTES.inc();
                self.new_qc_aggregated(qc.clone(), vote.author())
                    .await
                    .context(format!(
                        "[RoundManager] Unable to process the created QC {:?}",
                        qc
                    ))?;
                if self.onchain_config.order_vote_enabled() {
                    // Broadcast order vote if the QC is successfully aggregated
                    // Even if broadcast order vote fails, the function will return Ok
                    if let Err(e) = self.broadcast_order_vote(vote, qc.clone()).await {
                        warn!(
                            "Failed to broadcast order vote for QC {:?}. Error: {:?}",
                            qc, e
                        );
                    } else {
                        self.broadcast_fast_shares(qc.certified_block()).await;
                    }
                }
                Ok(())
            },
            VoteReceptionResult::New2ChainTimeoutCertificate(tc) => {
                self.new_2chain_tc_aggregated(tc).await
            },
            VoteReceptionResult::EchoTimeout(_) if !self.round_state.is_vote_timeout() => {
                self.process_local_timeout(round).await
            },
            VoteReceptionResult::VoteAdded(_) => {
                PROPOSAL_VOTE_ADDED.inc();
                Ok(())
            },
            VoteReceptionResult::VoteAddedQCDelayed(_)
            | VoteReceptionResult::EchoTimeout(_)
            | VoteReceptionResult::DuplicateVote => Ok(()),
            e => Err(anyhow::anyhow!("{:?}", e)),
        }
    }

    async fn process_order_vote_reception_result(
        &mut self,
        order_vote_msg: &OrderVoteMsg,
        result: OrderVoteReceptionResult,
    ) -> anyhow::Result<()> {
        match result {
            OrderVoteReceptionResult::NewLedgerInfoWithSignatures(ledger_info_with_signatures) => {
                self.new_ordered_cert(
                    WrappedLedgerInfo::new(VoteData::dummy(), ledger_info_with_signatures),
                    order_vote_msg.quorum_cert(),
                    order_vote_msg.order_vote().author(),
                )
                .await
            },
            OrderVoteReceptionResult::VoteAdded(_) => {
                ORDER_VOTE_ADDED.inc();
                Ok(())
            },
            e => {
                ORDER_VOTE_OTHER_ERRORS.inc();
                Err(anyhow::anyhow!("{:?}", e))
            },
        }
    }

    async fn new_qc_aggregated(
        &mut self,
        qc: Arc<QuorumCert>,
        preferred_peer: Author,
    ) -> anyhow::Result<()> {
        let result = self
            .block_store
            .insert_quorum_cert(&qc, &mut self.create_block_retriever(preferred_peer))
            .await
            .context("[RoundManager] Failed to process a newly aggregated QC");
        self.process_certificates().await?;
        result
    }

    // Insert ordered certificate formed by aggregating order votes
    async fn new_ordered_cert(
        &mut self,
        ordered_cert: WrappedLedgerInfo,
        quorum_cert: &QuorumCert,
        preferred_peer: Author,
    ) -> anyhow::Result<()> {
        ensure!(
            ordered_cert.commit_info().id() == quorum_cert.certified_block().id(),
            "QuorumCert attached to order votes doesn't match"
        );
        if self
            .block_store
            .get_block(ordered_cert.commit_info().id())
            .is_none()
        {
            ORDER_CERT_CREATED_WITHOUT_BLOCK_IN_BLOCK_STORE.inc();
        }
        self.block_store
            .insert_quorum_cert(
                quorum_cert,
                &mut self.create_block_retriever(preferred_peer),
            )
            .await
            .context("RoundManager] Failed to process QC in order Cert")?;
        let result = self
            .block_store
            .insert_ordered_cert(&ordered_cert)
            .await
            .context("[RoundManager] Failed to process a new OrderCert formed by order votes");
        self.process_certificates().await?;
        result
    }

    async fn new_2chain_tc_aggregated(
        &mut self,
        tc: Arc<TwoChainTimeoutCertificate>,
    ) -> anyhow::Result<()> {
        let result = self
            .block_store
            .insert_2chain_timeout_certificate(tc)
            .context("[RoundManager] Failed to process a newly aggregated 2-chain TC");
        self.process_certificates().await?;
        result
    }

    /// To jump start new round with the current certificates we have.
    pub async fn init(&mut self, last_vote_sent: Option<Vote>) {
        let new_round_event = self
            .round_state
            .process_certificates(self.block_store.sync_info())
            .expect("Can not jump start a round_state from existing certificates.");
        if let Some(vote) = last_vote_sent {
            self.round_state.record_vote(vote);
        }
        if let Err(e) = self.process_new_round_event(new_round_event).await {
            warn!(error = ?e, "[RoundManager] Error during start");
        }
    }

    /// Inspect the current consensus state.
    #[cfg(test)]
    pub fn consensus_state(&mut self) -> ConsensusState {
        self.safety_rules.lock().consensus_state().unwrap()
    }

    #[cfg(test)]
    pub fn set_safety_rules(&mut self, safety_rules: Arc<Mutex<MetricsSafetyRules>>) {
        self.safety_rules = safety_rules
    }

    pub fn epoch_state(&self) -> &EpochState {
        &self.epoch_state
    }

    pub fn round_state(&self) -> &RoundState {
        &self.round_state
    }

    fn new_log(&self, event: LogEvent) -> LogSchema {
        LogSchema::new(event)
            .round(self.round_state.current_round())
            .epoch(self.epoch_state.epoch)
    }

    /// Mainloop of processing messages.
    pub async fn start(
        mut self,
        mut event_rx: aptos_channel::Receiver<
            (Author, Discriminant<VerifiedEvent>),
            (Author, VerifiedEvent),
        >,
        mut buffered_proposal_rx: aptos_channel::Receiver<Author, VerifiedEvent>,
        mut delayed_qc_rx: UnboundedReceiver<DelayedQcMsg>,
        close_rx: oneshot::Receiver<oneshot::Sender<()>>,
    ) {
        info!(epoch = self.epoch_state().epoch, "RoundManager started");
        let mut close_rx = close_rx.into_stream();
        loop {
            tokio::select! {
                biased;
                close_req = close_rx.select_next_some() => {
                    if let Ok(ack_sender) = close_req {
                        ack_sender.send(()).expect("[RoundManager] Fail to ack shutdown");
                    }
                    break;
                }
                delayed_qc_msg = delayed_qc_rx.select_next_some() => {
                    let result = monitor!(
                        "process_delayed_qc",
                        self.process_delayed_qc_msg(delayed_qc_msg).await
                    );
                    match result {
                        Ok(_) => trace!(RoundStateLogSchema::new(self.round_state())),
                        Err(e) => {
                            counters::ERROR_COUNT.inc();
                            warn!(error = ?e, kind = error_kind(&e), RoundStateLogSchema::new(self.round_state()));
                        }
                    }
                },
                proposal = buffered_proposal_rx.select_next_some() => {
                    let mut proposals = vec![proposal];
                    while let Some(Some(proposal)) = buffered_proposal_rx.next().now_or_never() {
                        proposals.push(proposal);
                    }
                    let get_round = |event: &VerifiedEvent| {
                        match event {
                            VerifiedEvent::ProposalMsg(p) => p.proposal().round(),
                            VerifiedEvent::VerifiedProposalMsg(p) => p.round(),
                            unexpected_event => unreachable!("Unexpected event {:?}", unexpected_event),
                        }
                    };
                    proposals.sort_by_key(get_round);
                    // If the first proposal is not for the next round, we only process the last proposal.
                    // to avoid going through block retrieval of many garbage collected rounds.
                    if self.round_state.current_round() + 1 < get_round(&proposals[0]) {
                        proposals = vec![proposals.pop().unwrap()];
                    }
                    for proposal in proposals {
                        let result = match proposal {
                            VerifiedEvent::ProposalMsg(proposal_msg) => {
                                monitor!(
                                    "process_proposal",
                                    self.process_proposal_msg(*proposal_msg).await
                                )
                            }
                            VerifiedEvent::VerifiedProposalMsg(proposal_msg) => {
                                monitor!(
                                    "process_verified_proposal",
                                    self.process_delayed_proposal_msg(*proposal_msg).await
                                )
                            }
                            unexpected_event => unreachable!("Unexpected event: {:?}", unexpected_event),
                        };
                        let round_state = self.round_state();
                        match result {
                            Ok(_) => trace!(RoundStateLogSchema::new(round_state)),
                            Err(e) => {
                                counters::ERROR_COUNT.inc();
                                warn!(error = ?e, kind = error_kind(&e), RoundStateLogSchema::new(round_state));
                            }
                        }
                    }
                },
                (peer_id, event) = event_rx.select_next_some() => {
                    let result = match event {
                        VerifiedEvent::VoteMsg(vote_msg) => {
                            monitor!("process_vote", self.process_vote_msg(*vote_msg).await)
                        }
                        VerifiedEvent::OrderVoteMsg(order_vote_msg) => {
                            monitor!("process_order_vote", self.process_order_vote_msg(*order_vote_msg).await)
                        }
                        VerifiedEvent::UnverifiedSyncInfo(sync_info) => {
                            monitor!(
                                "process_sync_info",
                                self.process_sync_info_msg(*sync_info, peer_id).await
                            )
                        }
                        VerifiedEvent::LocalTimeout(round) => monitor!(
                            "process_local_timeout",
                            self.process_local_timeout(round).await
                        ),
                        unexpected_event => unreachable!("Unexpected event: {:?}", unexpected_event),
                    }
                    .with_context(|| format!("from peer {}", peer_id));

                    let round_state = self.round_state();
                    match result {
                        Ok(_) => trace!(RoundStateLogSchema::new(round_state)),
                        Err(e) => {
                            counters::ERROR_COUNT.inc();
                            warn!(error = ?e, kind = error_kind(&e), RoundStateLogSchema::new(round_state));
                        }
                    }
                }
            }
        }
        info!(epoch = self.epoch_state().epoch, "RoundManager stopped");
    }

    #[cfg(feature = "failpoints")]
    fn check_whether_to_inject_reconfiguration_error(&self) -> bool {
        fail_point!("consensus::inject_reconfiguration_error", |_| true);
        false
    }

    /// Given R1 <- B2 if R1 has the reconfiguration txn, we inject error on B2 if R1.round + 1 = B2.round
    /// Direct suffix is checked by parent.has_reconfiguration && !parent.parent.has_reconfiguration
    /// The error is injected by sending proposals to half of the validators to force a timeout.
    ///
    /// It's only enabled with fault injection (failpoints feature).
    #[cfg(feature = "failpoints")]
    async fn attempt_to_inject_reconfiguration_error(
        &self,
        proposal_msg: &ProposalMsg,
    ) -> anyhow::Result<()> {
        let block_data = proposal_msg.proposal().block_data();
        let direct_suffix = block_data.is_reconfiguration_suffix()
            && !block_data
                .quorum_cert()
                .parent_block()
                .has_reconfiguration();
        let continuous_round =
            block_data.round() == block_data.quorum_cert().certified_block().round() + 1;
        let should_inject = direct_suffix && continuous_round;
        if should_inject {
            let mut half_peers: Vec<_> = self
                .epoch_state
                .verifier
                .get_ordered_account_addresses_iter()
                .collect();
            half_peers.truncate(half_peers.len() / 2);
            self.network
                .send_proposal(proposal_msg.clone(), half_peers)
                .await;
            Err(anyhow::anyhow!("Injected error in reconfiguration suffix"))
        } else {
            Ok(())
        }
    }
}
