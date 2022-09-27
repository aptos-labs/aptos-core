// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    block_storage::{
        tracing::{observe_block, BlockStage},
        BlockReader, BlockRetriever, BlockStore,
    },
    counters,
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
    pending_votes::VoteReceptionResult,
    persistent_liveness_storage::PersistentLivenessStorage,
};
use anyhow::{bail, ensure, Context, Result};
use aptos_infallible::{checked, Mutex};
use aptos_logger::prelude::*;
use aptos_types::{
    epoch_state::EpochState, on_chain_config::OnChainConsensusConfig,
    validator_verifier::ValidatorVerifier,
};
use channel::aptos_channel;
use consensus_types::{
    block::Block,
    common::{Author, Round},
    experimental::{commit_decision::CommitDecision, commit_vote::CommitVote},
    proposal_msg::ProposalMsg,
    quorum_cert::QuorumCert,
    sync_info::SyncInfo,
    timeout_2chain::TwoChainTimeoutCertificate,
    vote::Vote,
    vote_msg::VoteMsg,
};
use fail::fail_point;
use futures::{channel::oneshot, FutureExt, StreamExt};
#[cfg(test)]
use safety_rules::ConsensusState;
use safety_rules::TSafetyRules;
use serde::Serialize;
use std::{
    mem::{discriminant, Discriminant},
    sync::Arc,
    time::Duration,
};
use tokio::time::{sleep, Instant};

#[derive(Serialize, Clone)]
pub enum UnverifiedEvent {
    ProposalMsg(Box<ProposalMsg>),
    VoteMsg(Box<VoteMsg>),
    SyncInfo(Box<SyncInfo>),
    CommitVote(Box<CommitVote>),
    CommitDecision(Box<CommitDecision>),
}

pub const BACK_PRESSURE_POLLING_INTERVAL_MS: u64 = 10;

impl UnverifiedEvent {
    pub fn verify(self, validator: &ValidatorVerifier) -> Result<VerifiedEvent, VerifyError> {
        Ok(match self {
            UnverifiedEvent::ProposalMsg(p) => {
                p.verify(validator)?;
                VerifiedEvent::ProposalMsg(p)
            }
            UnverifiedEvent::VoteMsg(v) => {
                v.verify(validator)?;
                VerifiedEvent::VoteMsg(v)
            }
            // sync info verification is on-demand (verified when it's used)
            UnverifiedEvent::SyncInfo(s) => VerifiedEvent::UnverifiedSyncInfo(s),
            UnverifiedEvent::CommitVote(cv) => {
                cv.verify(validator)?;
                VerifiedEvent::CommitVote(cv)
            }
            UnverifiedEvent::CommitDecision(cd) => {
                cd.verify(validator)?;
                VerifiedEvent::CommitDecision(cd)
            }
        })
    }

    pub fn epoch(&self) -> u64 {
        match self {
            UnverifiedEvent::ProposalMsg(p) => p.epoch(),
            UnverifiedEvent::VoteMsg(v) => v.epoch(),
            UnverifiedEvent::SyncInfo(s) => s.epoch(),
            UnverifiedEvent::CommitVote(cv) => cv.epoch(),
            UnverifiedEvent::CommitDecision(cd) => cd.epoch(),
        }
    }
}

impl From<ConsensusMsg> for UnverifiedEvent {
    fn from(value: ConsensusMsg) -> Self {
        match value {
            ConsensusMsg::ProposalMsg(m) => UnverifiedEvent::ProposalMsg(m),
            ConsensusMsg::VoteMsg(m) => UnverifiedEvent::VoteMsg(m),
            ConsensusMsg::SyncInfo(m) => UnverifiedEvent::SyncInfo(m),
            ConsensusMsg::CommitVoteMsg(m) => UnverifiedEvent::CommitVote(m),
            ConsensusMsg::CommitDecisionMsg(m) => UnverifiedEvent::CommitDecision(m),
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
    UnverifiedSyncInfo(Box<SyncInfo>),
    CommitVote(Box<CommitVote>),
    CommitDecision(Box<CommitDecision>),
    // local messages
    LocalTimeout(Round),
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
    epoch_state: EpochState,
    block_store: Arc<BlockStore>,
    round_state: RoundState,
    proposer_election: UnequivocalProposerElection,
    proposal_generator: ProposalGenerator,
    safety_rules: Arc<Mutex<MetricsSafetyRules>>,
    network: NetworkSender,
    storage: Arc<dyn PersistentLivenessStorage>,
    sync_only: bool,
    onchain_config: OnChainConsensusConfig,
    round_manager_tx:
        aptos_channel::Sender<(Author, Discriminant<VerifiedEvent>), (Author, VerifiedEvent)>,
    back_pressure_proposal_timeout_ms: u64,
}

impl RoundManager {
    pub fn new(
        epoch_state: EpochState,
        block_store: Arc<BlockStore>,
        round_state: RoundState,
        proposer_election: Box<dyn ProposerElection + Send + Sync>,
        proposal_generator: ProposalGenerator,
        safety_rules: Arc<Mutex<MetricsSafetyRules>>,
        network: NetworkSender,
        storage: Arc<dyn PersistentLivenessStorage>,
        sync_only: bool,
        onchain_config: OnChainConsensusConfig,
        round_manager_tx: aptos_channel::Sender<
            (Author, Discriminant<VerifiedEvent>),
            (Author, VerifiedEvent),
        >,
        back_pressure_proposal_timeout_ms: u64,
    ) -> Self {
        // when decoupled execution is false,
        // the counter is still static.
        counters::OP_COUNTERS
            .gauge("sync_only")
            .set(sync_only as i64);
        counters::OP_COUNTERS
            .gauge("decoupled_execution")
            .set(onchain_config.decoupled_execution() as i64);
        Self {
            epoch_state,
            block_store,
            round_state,
            proposer_election: UnequivocalProposerElection::new(proposer_election),
            proposal_generator,
            safety_rules,
            network,
            storage,
            sync_only,
            onchain_config,
            round_manager_tx,
            back_pressure_proposal_timeout_ms,
        }
    }

    fn decoupled_execution(&self) -> bool {
        self.onchain_config.decoupled_execution()
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
        counters::ROUND_TIMEOUT_MS.set(new_round_event.timeout.as_millis() as i64);
        match new_round_event.reason {
            NewRoundReason::QCReady => {
                counters::QC_ROUNDS_COUNT.inc();
            }
            NewRoundReason::Timeout => {
                counters::TIMEOUT_ROUNDS_COUNT.inc();
            }
        };
        info!(
            self.new_log(LogEvent::NewRound),
            reason = new_round_event.reason
        );

        if self
            .proposer_election
            .is_valid_proposer(self.proposal_generator.author(), new_round_event.round)
        {
            self.log_collected_vote_stats(&new_round_event);
            self.round_state.setup_leader_timeout();
            let proposal_msg = self.generate_proposal(new_round_event).await?;
            let mut network = self.network.clone();
            #[cfg(feature = "failpoints")]
            {
                if self.check_whether_to_inject_reconfiguration_error() {
                    self.attempt_to_inject_reconfiguration_error(&proposal_msg)
                        .await?;
                }
            }
            network.broadcast_proposal(proposal_msg).await;
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
        let mut sender = self.network.clone();
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

    pub async fn process_delayed_proposal_msg(&mut self, proposal: Block) -> Result<()> {
        if proposal.round() != self.round_state.current_round() {
            bail!(
                "Discarding stale delayed proposal {}, current round {}",
                proposal,
                self.round_state.current_round()
            );
        }

        self.process_verified_proposal(proposal).await
    }

    /// Sync to the sync info sending from peer if it has newer certificates.
    async fn sync_up(&mut self, sync_info: &SyncInfo, author: Author) -> anyhow::Result<()> {
        let local_sync_info = self.block_store.sync_info();
        if sync_info.has_newer_certificates(&local_sync_info) {
            info!(
                self.new_log(LogEvent::ReceiveNewCertificate)
                    .remote_peer(author),
                "Local state {}, remote state {}", local_sync_info, sync_info
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
            "After sync, round {} doesn't match local {}",
            message_round,
            self.round_state.current_round()
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
        if self.decoupled_execution() {
            let sync_or_not = self.sync_only || self.block_store.back_pressure();
            counters::OP_COUNTERS
                .gauge("sync_only")
                .set(sync_or_not as i64);

            sync_or_not
        } else {
            self.sync_only
        }
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
            }
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
            }
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
        error!(
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
    async fn process_proposal(&mut self, proposal: Block) -> Result<()> {
        let author = proposal
            .author()
            .expect("Proposal should be verified having an author");

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
        if self.decoupled_execution() && self.block_store.back_pressure() {
            // In case of back pressure, we delay processing proposal. This is done by resending the
            // same proposal to self after some time.
            Ok(self
                .resend_verified_proposal_to_self(
                    proposal,
                    BACK_PRESSURE_POLLING_INTERVAL_MS,
                    self.back_pressure_proposal_timeout_ms,
                )
                .await)
        } else {
            self.process_verified_proposal(proposal).await
        }
    }

    async fn resend_verified_proposal_to_self(
        &self,
        proposal: Block,
        polling_interval_ms: u64,
        timeout_ms: u64,
    ) {
        let start = Instant::now();
        let author = self.network.author();
        let block_store = self.block_store.clone();
        let self_sender = self.round_manager_tx.clone();
        let event = VerifiedEvent::VerifiedProposalMsg(Box::new(proposal));
        tokio::spawn(async move {
            while start.elapsed() < Duration::from_millis(timeout_ms) {
                if !block_store.back_pressure() {
                    if let Err(e) =
                        self_sender.push((author, discriminant(&event)), (author, event))
                    {
                        error!("Failed to send event to round manager {:?}", e);
                    }
                    break;
                }
                sleep(Duration::from_millis(polling_interval_ms)).await;
            }
        });
    }

    pub async fn process_verified_proposal(&mut self, proposal: Block) -> Result<()> {
        let proposal_round = proposal.round();
        let vote = self
            .execute_and_vote(proposal)
            .await
            .context("[RoundManager] Process proposal")?;

        let recipient = self
            .proposer_election
            .get_valid_proposer(proposal_round + 1);

        info!(
            self.new_log(LogEvent::Vote).remote_peer(recipient),
            "{}", vote
        );

        self.round_state.record_vote(vote.clone());
        let vote_msg = VoteMsg::new(vote, self.block_store.sync_info());
        self.network.send_vote(vote_msg, vec![recipient]).await;
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
            .execute_and_insert_block(proposed_block)
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

        let vote_proposal = executed_block.vote_proposal(self.decoupled_execution());
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

        if !vote.is_timeout() {
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
        // Add the vote and check whether it completes a new QC or a TC
        match self
            .round_state
            .insert_vote(vote, &self.epoch_state.verifier)
        {
            VoteReceptionResult::NewQuorumCertificate(qc) => {
                if !vote.is_timeout() {
                    observe_block(
                        qc.certified_block().timestamp_usecs(),
                        BlockStage::QC_AGGREGATED,
                    );
                }
                self.new_qc_aggregated(qc, vote.author()).await
            }
            VoteReceptionResult::New2ChainTimeoutCertificate(tc) => {
                self.new_2chain_tc_aggregated(tc).await
            }
            VoteReceptionResult::EchoTimeout(_) if !self.round_state.is_vote_timeout() => {
                self.process_local_timeout(round).await
            }
            VoteReceptionResult::VoteAdded(_)
            | VoteReceptionResult::EchoTimeout(_)
            | VoteReceptionResult::DuplicateVote => Ok(()),
            e => Err(anyhow::anyhow!("{:?}", e)),
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
            error!(error = ?e, "[RoundManager] Error during start");
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
        close_rx: oneshot::Receiver<oneshot::Sender<()>>,
    ) {
        info!(epoch = self.epoch_state().epoch, "RoundManager started");
        let mut close_rx = close_rx.into_stream();
        loop {
            futures::select! {
                (peer_id, event) = event_rx.select_next_some() => {
                    let result = match event {
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
                        VerifiedEvent::VoteMsg(vote_msg) => {
                            monitor!("process_vote", self.process_vote_msg(*vote_msg).await)
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
                            error!(error = ?e, kind = error_kind(&e), RoundStateLogSchema::new(round_state));
                        }
                    }
                }
                close_req = close_rx.select_next_some() => {
                    if let Ok(ack_sender) = close_req {
                        ack_sender.send(()).expect("[RoundManager] Fail to ack shutdown");
                    }
                    break;
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
                .clone()
                .send_proposal(proposal_msg.clone(), half_peers)
                .await;
            Err(anyhow::anyhow!("Injected error in reconfiguration suffix"))
        } else {
            Ok(())
        }
    }
}
