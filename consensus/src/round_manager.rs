// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    block_storage::{
        tracing::{observe_block, BlockStage},
        BlockReader, BlockRetriever, BlockStore, NeedFetchResult,
    },
    counters::{
        self, ORDER_CERT_CREATED_WITHOUT_BLOCK_IN_BLOCK_STORE, ORDER_VOTE_ADDED,
        ORDER_VOTE_BROADCASTED, ORDER_VOTE_NOT_IN_RANGE, ORDER_VOTE_OTHER_ERRORS,
        PROPOSAL_VOTE_ADDED, PROPOSAL_VOTE_BROADCASTED, PROPOSED_VTXN_BYTES, PROPOSED_VTXN_COUNT,
        QC_AGGREGATED_FROM_VOTES, SYNC_INFO_RECEIVED_WITH_NEWER_CERT,
    },
    error::{error_kind, VerifyError},
    liveness::{
        proposal_generator::ProposalGenerator,
        proposal_status_tracker::TPastProposalStatusTracker,
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
    pending_votes::{VoteReceptionResult, VoteStatus},
    persistent_liveness_storage::PersistentLivenessStorage,
    quorum_store::types::BatchMsg,
    rand::rand_gen::types::{FastShare, RandConfig, Share, TShare},
    util::is_vtxn_expected,
};
use anyhow::{bail, ensure, Context};
use aptos_channels::aptos_channel;
use aptos_config::config::{BlockTransactionFilterConfig, ConsensusConfig};
use aptos_consensus_types::{
    block::Block,
    block_data::BlockType,
    common::{Author, Round},
    opt_block_data::OptBlockData,
    opt_proposal_msg::OptProposalMsg,
    order_vote::OrderVote,
    order_vote_msg::OrderVoteMsg,
    pipelined_block::PipelinedBlock,
    proof_of_store::{ProofCache, ProofOfStoreMsg, SignedBatchInfoMsg},
    proposal_msg::ProposalMsg,
    quorum_cert::QuorumCert,
    round_timeout::{RoundTimeout, RoundTimeoutMsg, RoundTimeoutReason},
    sync_info::SyncInfo,
    timeout_2chain::{TwoChainTimeout, TwoChainTimeoutCertificate},
    vote::Vote,
    vote_data::VoteData,
    vote_msg::VoteMsg,
    wrapped_ledger_info::WrappedLedgerInfo,
};
use aptos_crypto::{hash::CryptoHash, HashValue};
use aptos_infallible::{checked, Mutex};
use aptos_logger::prelude::*;
#[cfg(test)]
use aptos_safety_rules::ConsensusState;
use aptos_safety_rules::TSafetyRules;
use aptos_short_hex_str::AsShortHexStr;
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
use futures::{channel::oneshot, stream::FuturesUnordered, Future, FutureExt, SinkExt, StreamExt};
use lru::LruCache;
use serde::Serialize;
use std::{
    collections::BTreeMap, mem::Discriminant, num::NonZeroUsize, ops::Add, pin::Pin, sync::Arc,
    time::Duration,
};
use tokio::{
    sync::oneshot as TokioOneshot,
    time::{sleep, Instant},
};

#[derive(Debug, Serialize, Clone)]
pub enum UnverifiedEvent {
    ProposalMsg(Box<ProposalMsg>),
    VoteMsg(Box<VoteMsg>),
    RoundTimeoutMsg(Box<RoundTimeoutMsg>),
    OrderVoteMsg(Box<OrderVoteMsg>),
    SyncInfo(Box<SyncInfo>),
    BatchMsg(Box<BatchMsg>),
    SignedBatchInfo(Box<SignedBatchInfoMsg>),
    ProofOfStoreMsg(Box<ProofOfStoreMsg>),
    OptProposalMsg(Box<OptProposalMsg>),
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
                    p.verify(peer_id, validator, proof_cache, quorum_store_enabled)?;
                    counters::VERIFY_MSG
                        .with_label_values(&["proposal"])
                        .observe(start_time.elapsed().as_secs_f64());
                }
                VerifiedEvent::ProposalMsg(p)
            },
            UnverifiedEvent::OptProposalMsg(p) => {
                if !self_message {
                    p.verify(peer_id, validator, proof_cache, quorum_store_enabled)?;
                    counters::VERIFY_MSG
                        .with_label_values(&["opt_proposal"])
                        .observe(start_time.elapsed().as_secs_f64());
                }
                VerifiedEvent::OptProposalMsg(p)
            },
            UnverifiedEvent::VoteMsg(v) => {
                if !self_message {
                    v.verify(peer_id, validator)?;
                    counters::VERIFY_MSG
                        .with_label_values(&["vote"])
                        .observe(start_time.elapsed().as_secs_f64());
                }
                VerifiedEvent::VoteMsg(v)
            },
            UnverifiedEvent::RoundTimeoutMsg(v) => {
                if !self_message {
                    v.verify(validator)?;
                    counters::VERIFY_MSG
                        .with_label_values(&["timeout"])
                        .observe(start_time.elapsed().as_secs_f64());
                }
                VerifiedEvent::RoundTimeoutMsg(v)
            },
            UnverifiedEvent::OrderVoteMsg(v) => {
                if !self_message {
                    v.verify_order_vote(peer_id, validator)?;
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
                    b.verify(peer_id, max_num_batches, validator)?;
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
            UnverifiedEvent::OptProposalMsg(p) => Ok(p.epoch()),
            UnverifiedEvent::VoteMsg(v) => Ok(v.epoch()),
            UnverifiedEvent::OrderVoteMsg(v) => Ok(v.epoch()),
            UnverifiedEvent::SyncInfo(s) => Ok(s.epoch()),
            UnverifiedEvent::BatchMsg(b) => b.epoch(),
            UnverifiedEvent::SignedBatchInfo(sd) => sd.epoch(),
            UnverifiedEvent::ProofOfStoreMsg(p) => p.epoch(),
            UnverifiedEvent::RoundTimeoutMsg(t) => Ok(t.epoch()),
        }
    }
}

impl From<ConsensusMsg> for UnverifiedEvent {
    fn from(value: ConsensusMsg) -> Self {
        match value {
            ConsensusMsg::ProposalMsg(m) => UnverifiedEvent::ProposalMsg(m),
            ConsensusMsg::OptProposalMsg(m) => UnverifiedEvent::OptProposalMsg(m),
            ConsensusMsg::VoteMsg(m) => UnverifiedEvent::VoteMsg(m),
            ConsensusMsg::OrderVoteMsg(m) => UnverifiedEvent::OrderVoteMsg(m),
            ConsensusMsg::SyncInfo(m) => UnverifiedEvent::SyncInfo(m),
            ConsensusMsg::BatchMsg(m) => UnverifiedEvent::BatchMsg(m),
            ConsensusMsg::SignedBatchInfo(m) => UnverifiedEvent::SignedBatchInfo(m),
            ConsensusMsg::ProofOfStoreMsg(m) => UnverifiedEvent::ProofOfStoreMsg(m),
            ConsensusMsg::RoundTimeoutMsg(m) => UnverifiedEvent::RoundTimeoutMsg(m),
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
    RoundTimeoutMsg(Box<RoundTimeoutMsg>),
    OrderVoteMsg(Box<OrderVoteMsg>),
    UnverifiedSyncInfo(Box<SyncInfo>),
    BatchMsg(Box<BatchMsg>),
    SignedBatchInfo(Box<SignedBatchInfoMsg>),
    ProofOfStoreMsg(Box<ProofOfStoreMsg>),
    // local messages
    LocalTimeout(Round),
    // Shutdown the NetworkListener
    Shutdown(TokioOneshot::Sender<()>),
    OptProposalMsg(Box<OptProposalMsg>),
}

#[cfg(test)]
#[path = "round_manager_tests/mod.rs"]
mod round_manager_tests;

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
    proposer_election: Arc<UnequivocalProposerElection>,
    proposal_generator: Arc<ProposalGenerator>,
    safety_rules: Arc<Mutex<MetricsSafetyRules>>,
    network: Arc<NetworkSender>,
    storage: Arc<dyn PersistentLivenessStorage>,
    onchain_config: OnChainConsensusConfig,
    vtxn_config: ValidatorTxnConfig,
    buffered_proposal_tx: aptos_channel::Sender<Author, VerifiedEvent>,
    block_txn_filter_config: BlockTransactionFilterConfig,
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
    futures: FuturesUnordered<
        Pin<Box<dyn Future<Output = (anyhow::Result<()>, Block, Instant)> + Send>>,
    >,
    proposal_status_tracker: Arc<dyn TPastProposalStatusTracker>,
    pending_opt_proposals: BTreeMap<Round, OptBlockData>,
    opt_proposal_loopback_tx: aptos_channels::UnboundedSender<OptBlockData>,
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
        block_txn_filter_config: BlockTransactionFilterConfig,
        local_config: ConsensusConfig,
        randomness_config: OnChainRandomnessConfig,
        jwk_consensus_config: OnChainJWKConsensusConfig,
        fast_rand_config: Option<RandConfig>,
        proposal_status_tracker: Arc<dyn TPastProposalStatusTracker>,
        opt_proposal_loopback_tx: aptos_channels::UnboundedSender<OptBlockData>,
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
            proposer_election: Arc::new(UnequivocalProposerElection::new(proposer_election)),
            proposal_generator: Arc::new(proposal_generator),
            safety_rules,
            network,
            storage,
            onchain_config,
            vtxn_config,
            buffered_proposal_tx,
            block_txn_filter_config,
            local_config,
            randomness_config,
            jwk_consensus_config,
            fast_rand_config,
            pending_order_votes: PendingOrderVotes::new(),
            blocks_with_broadcasted_fast_shares: LruCache::new(
                NonZeroUsize::new(5).expect("LRU capacity should be non-zero."),
            ),
            futures: FuturesUnordered::new(),
            proposal_status_tracker,
            pending_opt_proposals: BTreeMap::new(),
            opt_proposal_loopback_tx,
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
        let new_round = new_round_event.round;
        let is_current_proposer = self
            .proposer_election
            .is_valid_proposer(self.proposal_generator.author(), new_round);
        let prev_proposer = self
            .proposer_election
            .get_valid_proposer(new_round.saturating_sub(1));

        counters::CURRENT_ROUND.set(new_round_event.round as i64);
        counters::ROUND_TIMEOUT_MS.set(new_round_event.timeout.as_millis() as i64);
        match new_round_event.reason {
            NewRoundReason::QCReady => {
                counters::QC_ROUNDS_COUNT.inc();
            },
            NewRoundReason::Timeout(ref reason) => {
                counters::TIMEOUT_ROUNDS_COUNT.inc();
                counters::AGGREGATED_ROUND_TIMEOUT_REASON
                    .with_label_values(&[
                        &reason.to_string(),
                        prev_proposer.short_str().as_str(),
                        &is_current_proposer.to_string(),
                    ])
                    .inc();
                if is_current_proposer {
                    if let RoundTimeoutReason::PayloadUnavailable { missing_authors } = reason {
                        let ordered_peers =
                            self.epoch_state.verifier.get_ordered_account_addresses();
                        for idx in missing_authors.iter_ones() {
                            if let Some(author) = ordered_peers.get(idx) {
                                counters::AGGREGATED_ROUND_TIMEOUT_REASON_MISSING_AUTHORS
                                    .with_label_values(&[author.short_str().as_str()])
                                    .inc();
                            }
                        }
                    }
                }
            },
        };
        info!(
            self.new_log(LogEvent::NewRound),
            reason = new_round_event.reason
        );
        self.pending_order_votes
            .garbage_collect(self.block_store.sync_info().highest_ordered_round());

        self.proposal_status_tracker
            .push(new_round_event.reason.clone());

        // Process pending opt proposal for the new round.
        // The existence of pending optimistic proposal and being the current proposer are mutually
        // exclusive. Note that the opt proposal is checked for valid proposer before inserting into
        // the pending queue.
        if let Some(opt_proposal) = self.pending_opt_proposals.remove(&new_round) {
            self.opt_proposal_loopback_tx
                .send(opt_proposal)
                .await
                .expect("Sending to a self loopback unbounded channel cannot fail");
        }

        // If the current proposer is the leading, try to propose a regular block if not opt proposed already
        if is_current_proposer
            && self
                .proposal_generator
                .can_propose_in_round(new_round_event.round)
        {
            let epoch_state = self.epoch_state.clone();
            let network = self.network.clone();
            let sync_info = self.block_store.sync_info();
            let proposal_generator = self.proposal_generator.clone();
            let safety_rules = self.safety_rules.clone();
            let proposer_election = self.proposer_election.clone();
            tokio::spawn(async move {
                if let Err(e) = monitor!(
                    "generate_and_send_proposal",
                    Self::generate_and_send_proposal(
                        epoch_state,
                        new_round_event,
                        network,
                        sync_info,
                        proposal_generator,
                        safety_rules,
                        proposer_election,
                    )
                    .await
                ) {
                    warn!("Error generating and sending proposal: {}", e);
                }
            });
        }
        Ok(())
    }

    async fn generate_and_send_proposal(
        epoch_state: Arc<EpochState>,
        new_round_event: NewRoundEvent,
        network: Arc<NetworkSender>,
        sync_info: SyncInfo,
        proposal_generator: Arc<ProposalGenerator>,
        safety_rules: Arc<Mutex<MetricsSafetyRules>>,
        proposer_election: Arc<dyn ProposerElection + Send + Sync>,
    ) -> anyhow::Result<()> {
        Self::log_collected_vote_stats(epoch_state.clone(), &new_round_event);
        let proposal_msg = Self::generate_proposal(
            epoch_state.clone(),
            new_round_event,
            sync_info,
            proposal_generator,
            safety_rules,
            proposer_election,
        )
        .await?;
        #[cfg(feature = "failpoints")]
        {
            if Self::check_whether_to_inject_reconfiguration_error() {
                Self::attempt_to_inject_reconfiguration_error(
                    epoch_state,
                    network.clone(),
                    &proposal_msg,
                )
                .await?;
            }
        };
        network.broadcast_proposal(proposal_msg).await;
        counters::PROPOSALS_COUNT.inc();
        Ok(())
    }

    async fn generate_and_send_opt_proposal(
        epoch_state: Arc<EpochState>,
        round: Round,
        parent: BlockInfo,
        grandparent_qc: QuorumCert,
        network: Arc<NetworkSender>,
        sync_info: SyncInfo,
        proposal_generator: Arc<ProposalGenerator>,
        proposer_election: Arc<dyn ProposerElection + Send + Sync>,
    ) -> anyhow::Result<()> {
        let proposal_msg = Self::generate_opt_proposal(
            epoch_state.clone(),
            round,
            parent,
            grandparent_qc,
            sync_info,
            proposal_generator,
            proposer_election,
        )
        .await?;
        network.broadcast_opt_proposal(proposal_msg).await;
        counters::PROPOSALS_COUNT.inc();
        Ok(())
    }

    fn log_collected_vote_stats(epoch_state: Arc<EpochState>, new_round_event: &NewRoundEvent) {
        let prev_round_votes_for_li = new_round_event
            .prev_round_votes
            .iter()
            .map(|(_, vote_status)| {
                let all_voters = match vote_status {
                    VoteStatus::EnoughVotes(li_with_sig) => epoch_state
                        .verifier
                        .aggregate_signature_authors(li_with_sig.signatures()),
                    VoteStatus::NotEnoughVotes(li_with_sig) => {
                        li_with_sig.all_voters().collect::<Vec<_>>()
                    },
                };
                let (voting_power, votes): (Vec<_>, Vec<_>) = all_voters
                    .into_iter()
                    .map(|author| {
                        epoch_state
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
                        epoch_state
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
            epoch = epoch_state.epoch,
            round = new_round_event.round,
            total_voting_power = ?epoch_state.verifier.total_voting_power(),
            max_voting_power = ?max_voting_power,
            max_num_votes = max_num_votes,
            conflicting_voting_power = ?conflicting_voting_power,
            conflicting_num_votes = conflicting_num_votes,
            timeout_voting_power = ?timeout_voting_power,
            timeout_num_votes = timeout_num_votes,
            "Preparing new proposal",
        );
    }

    #[cfg(feature = "fuzzing")]
    async fn generate_proposal_for_test(
        &self,
        new_round_event: NewRoundEvent,
    ) -> anyhow::Result<ProposalMsg> {
        Self::generate_proposal(
            self.epoch_state.clone(),
            new_round_event,
            self.block_store.sync_info(),
            self.proposal_generator.clone(),
            self.safety_rules.clone(),
            self.proposer_election.clone(),
        )
        .await
    }

    async fn generate_proposal(
        epoch_state: Arc<EpochState>,
        new_round_event: NewRoundEvent,
        sync_info: SyncInfo,
        proposal_generator: Arc<ProposalGenerator>,
        safety_rules: Arc<Mutex<MetricsSafetyRules>>,
        proposer_election: Arc<dyn ProposerElection + Send + Sync>,
    ) -> anyhow::Result<ProposalMsg> {
        let proposal = proposal_generator
            .generate_proposal(new_round_event.round, proposer_election)
            .await?;
        let signature = safety_rules.lock().sign_proposal(&proposal)?;
        let signed_proposal =
            Block::new_proposal_from_block_data_and_signature(proposal, signature);
        observe_block(signed_proposal.timestamp_usecs(), BlockStage::SIGNED);
        info!(
            Self::new_log_with_round_epoch(
                LogEvent::Propose,
                new_round_event.round,
                epoch_state.epoch
            ),
            "{}", signed_proposal
        );
        Ok(ProposalMsg::new(signed_proposal, sync_info))
    }

    async fn generate_opt_proposal(
        epoch_state: Arc<EpochState>,
        round: Round,
        parent: BlockInfo,
        grandparent_qc: QuorumCert,
        sync_info: SyncInfo,
        proposal_generator: Arc<ProposalGenerator>,
        proposer_election: Arc<dyn ProposerElection + Send + Sync>,
    ) -> anyhow::Result<OptProposalMsg> {
        // Proposal generator will ensure that at most one proposal is generated per round

        let proposal = proposal_generator
            .generate_opt_proposal(
                epoch_state.epoch,
                round,
                parent,
                grandparent_qc,
                proposer_election,
            )
            .await?;
        observe_block(proposal.timestamp_usecs(), BlockStage::OPT_PROPOSED);
        info!(Self::new_log_with_round_epoch(
            LogEvent::OptPropose,
            round,
            epoch_state.epoch
        ),);
        Ok(OptProposalMsg::new(proposal, sync_info))
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

        let in_correct_round = self
            .ensure_round_and_sync_up(
                proposal_msg.proposal().round(),
                proposal_msg.sync_info(),
                proposal_msg.proposer(),
            )
            .await
            .context("[RoundManager] Process proposal")?;
        if in_correct_round {
            self.process_proposal(proposal_msg.take_proposal()).await
        } else {
            sample!(
                SampleRate::Duration(Duration::from_secs(30)),
                warn!(
                    "[sampled] Stale proposal {}, current round {}",
                    proposal_msg.proposal(),
                    self.round_state.current_round()
                )
            );
            counters::ERROR_COUNT.inc();
            Ok(())
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

    /// Process the optimistic proposal message:
    /// If entered the round of opt proposal, process the opt proposal directly.
    /// Otherwise, buffer the opt proposal and process it later when parent QC is available.
    pub async fn process_opt_proposal_msg(
        &mut self,
        proposal_msg: OptProposalMsg,
    ) -> anyhow::Result<()> {
        ensure!(self.local_config.enable_optimistic_proposal_rx,
            "Opt proposal is disabled, but received opt proposal msg of epoch {} round {} from peer {}",
            proposal_msg.block_data().epoch(), proposal_msg.round(), proposal_msg.proposer()
        );

        fail_point!("consensus::process_opt_proposal_msg", |_| {
            Err(anyhow::anyhow!(
                "Injected error in process_opt_proposal_msg"
            ))
        });

        observe_block(
            proposal_msg.block_data().timestamp_usecs(),
            BlockStage::ROUND_MANAGER_RECEIVED,
        );
        observe_block(
            proposal_msg.block_data().timestamp_usecs(),
            BlockStage::ROUND_MANAGER_RECEIVED_OPT_PROPOSAL,
        );
        info!(
            self.new_log(LogEvent::ReceiveOptProposal),
            block_author = proposal_msg.proposer(),
            block_epoch = proposal_msg.block_data().epoch(),
            block_round = proposal_msg.round(),
            block_parent_hash = proposal_msg.block_data().parent_id(),
        );

        self.sync_up(proposal_msg.sync_info(), proposal_msg.proposer())
            .await?;

        if self.round_state.current_round() == proposal_msg.round() {
            self.opt_proposal_loopback_tx
                .send(proposal_msg.take_block_data())
                .await
                .expect("Sending to a self loopback unbounded channel cannot fail");
        } else {
            // Pre-check that proposal is from valid proposer before queuing it.
            // This check is done after syncing up to sync info to ensure proposer
            // election provider is up to date.
            ensure!(
                self.proposer_election
                    .is_valid_proposer(proposal_msg.proposer(), proposal_msg.round()),
                "[OptProposal] Not a valid proposer for round {}: {}",
                proposal_msg.round(),
                proposal_msg.proposer()
            );
            self.pending_opt_proposals
                .insert(proposal_msg.round(), proposal_msg.take_block_data());
        }

        Ok(())
    }

    /// Process the optimistic proposal:
    /// 1. Ensure the highest quorum cert certifies the parent block of the opt block
    /// 2. Create a regular proposal by adding QC and failed_authors to the opt block
    /// 3. Process the proposal using exsiting logic
    async fn process_opt_proposal(&mut self, opt_block_data: OptBlockData) -> anyhow::Result<()> {
        ensure!(
            self.block_store
                .get_block_for_round(opt_block_data.round())
                .is_none(),
            "Proposal has already been processed for round: {}",
            opt_block_data.round()
        );
        let hqc = self.block_store.highest_quorum_cert().as_ref().clone();
        ensure!(
            hqc.certified_block().round() + 1 == opt_block_data.round(),
            "Opt proposal round {} is not the next round after the highest qc round {}",
            opt_block_data.round(),
            hqc.certified_block().round()
        );
        ensure!(
            hqc.certified_block().id() == opt_block_data.parent_id(),
            "Opt proposal parent id {} is not the same as the highest qc certified block id {}",
            opt_block_data.parent_id(),
            hqc.certified_block().id()
        );
        let proposal = Block::new_from_opt(opt_block_data, hqc);
        observe_block(proposal.timestamp_usecs(), BlockStage::PROCESS_OPT_PROPOSAL);
        info!(
            self.new_log(LogEvent::ProcessOptProposal),
            block_author = proposal.author(),
            block_epoch = proposal.epoch(),
            block_round = proposal.round(),
            block_hash = proposal.id(),
            block_parent_hash = proposal.quorum_cert().certified_block().id(),
        );
        self.process_proposal(proposal).await
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
            sync_info.verify(&self.epoch_state.verifier).map_err(|e| {
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

    fn compute_timeout_reason(&self, round: Round) -> RoundTimeoutReason {
        if self.round_state().vote_sent().is_some() {
            return RoundTimeoutReason::NoQC;
        }

        match self.block_store.get_block_for_round(round) {
            None => RoundTimeoutReason::ProposalNotReceived,
            Some(block) => {
                if let Err(missing_authors) = self.block_store.check_payload(block.block()) {
                    RoundTimeoutReason::PayloadUnavailable { missing_authors }
                } else {
                    RoundTimeoutReason::Unknown
                }
            },
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

        if self.local_config.enable_round_timeout_msg {
            let timeout = if let Some(timeout) = self.round_state.timeout_sent() {
                timeout
            } else {
                let timeout = TwoChainTimeout::new(
                    self.epoch_state.epoch,
                    round,
                    self.block_store.highest_quorum_cert().as_ref().clone(),
                );
                let signature = self
                    .safety_rules
                    .lock()
                    .sign_timeout_with_qc(
                        &timeout,
                        self.block_store.highest_2chain_timeout_cert().as_deref(),
                    )
                    .context("[RoundManager] SafetyRules signs 2-chain timeout")?;

                let timeout_reason = self.compute_timeout_reason(round);

                RoundTimeout::new(
                    timeout,
                    self.proposal_generator.author(),
                    timeout_reason,
                    signature,
                )
            };

            self.round_state.record_round_timeout(timeout.clone());
            let round_timeout_msg = RoundTimeoutMsg::new(timeout, self.block_store.sync_info());
            self.network
                .broadcast_round_timeout(round_timeout_msg)
                .await;
            warn!(
                round = round,
                remote_peer = self.proposer_election.get_valid_proposer(round),
                event = LogEvent::Timeout,
            );
            bail!("Round {} timeout, broadcast to all peers", round);
        } else {
            let (is_nil_vote, mut timeout_vote) = match self.round_state.vote_sent() {
                Some(vote) if vote.vote_data().proposed().round() == round => {
                    (vote.vote_data().is_for_nil(), vote)
                },
                _ => {
                    // Didn't vote in this round yet, generate a backup vote
                    let nil_block = self
                        .proposal_generator
                        .generate_nil_block(round, self.proposer_election.clone())?;
                    info!(
                        self.new_log(LogEvent::VoteNIL),
                        "Planning to vote for a NIL block {}", nil_block
                    );
                    counters::VOTE_NIL_COUNT.inc();
                    let nil_vote = self.vote_block(nil_block).await?;
                    (true, nil_vote)
                },
            };

            if !timeout_vote.is_timeout() {
                let timeout = timeout_vote.generate_2chain_timeout(
                    self.block_store.highest_quorum_cert().as_ref().clone(),
                );
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
    }

    /// This function is called only after all the dependencies of the given QC have been retrieved.
    async fn process_certificates(&mut self) -> anyhow::Result<()> {
        let sync_info = self.block_store.sync_info();
        let epoch_state = self.epoch_state.clone();
        if let Some(new_round_event) = self
            .round_state
            .process_certificates(sync_info, &epoch_state.verifier)
        {
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
            bail!("ProposalExt unexpected while the vtxn feature is disabled.");
        }

        if let Some(vtxns) = proposal.validator_txns() {
            for vtxn in vtxns {
                let vtxn_type_name = vtxn.type_name();
                ensure!(
                    is_vtxn_expected(&self.randomness_config, &self.jwk_consensus_config, vtxn),
                    "unexpected validator txn: {:?}",
                    vtxn_type_name
                );
                vtxn.verify(self.epoch_state.verifier.as_ref())
                    .context(format!("{} verify failed", vtxn_type_name))?;
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

        // If the proposal contains any inline transactions that need to be denied
        // (e.g., due to filtering) drop the message and do not vote for the block.
        if let Err(error) = self
            .block_store
            .check_denied_inline_transactions(&proposal, &self.block_txn_filter_config)
        {
            counters::REJECTED_PROPOSAL_DENY_TXN_COUNT.inc();
            bail!(
                "[RoundManager] Proposal for block {} contains denied inline transactions: {}. Dropping proposal!",
                proposal.id(),
                error
            );
        }

        if !proposal.is_opt_block() {
            // Validate that failed_authors list is correctly specified in the block.
            let expected_failed_authors = self.proposal_generator.compute_failed_authors(
                proposal.round(),
                proposal.quorum_cert().certified_block().round(),
                false,
                self.proposer_election.clone(),
            );
            ensure!(
                proposal.block_data().failed_authors().is_some_and(|failed_authors| *failed_authors == expected_failed_authors),
                "[RoundManager] Proposal for block {} has invalid failed_authors list {:?}, expected {:?}",
                proposal.round(),
                proposal.block_data().failed_authors(),
                expected_failed_authors,
            );
        }

        let block_time_since_epoch = Duration::from_micros(proposal.timestamp_usecs());

        ensure!(
            block_time_since_epoch < self.round_state.current_round_deadline(),
            "[RoundManager] Waiting until proposal block timestamp usecs {:?} \
            would exceed the round duration {:?}, hence will not vote for this round",
            block_time_since_epoch,
            self.round_state.current_round_deadline(),
        );

        observe_block(proposal.timestamp_usecs(), BlockStage::SYNCED);
        if proposal.is_opt_block() {
            observe_block(proposal.timestamp_usecs(), BlockStage::SYNCED_OPT_BLOCK);
        }

        // Since processing proposal is delayed due to backpressure or payload availability, we add
        // the block to the block store so that we don't need to fetch it from remote once we
        // are out of the backpressure. Please note that delayed processing of proposal is not
        // guaranteed to add the block to the block store if we don't get out of the backpressure
        // before the timeout, so this is needed to ensure that the proposed block is added to
        // the block store irrespective. Also, it is possible that delayed processing of proposal
        // tries to add the same block again, which is okay as `insert_block` call
        // is idempotent.
        self.block_store
            .insert_block(proposal.clone())
            .await
            .context("[RoundManager] Failed to insert the block into BlockStore")?;

        let block_store = self.block_store.clone();
        if block_store.check_payload(&proposal).is_err() {
            debug!("Payload not available locally for block: {}", proposal.id());
            counters::CONSENSUS_PROPOSAL_PAYLOAD_AVAILABILITY
                .with_label_values(&["missing"])
                .inc();
            let start_time = Instant::now();
            let deadline = self.round_state.current_round_deadline();
            let future = async move {
                (
                    block_store.wait_for_payload(&proposal, deadline).await,
                    proposal,
                    start_time,
                )
            }
            .boxed();
            self.futures.push(future);
            return Ok(());
        }

        counters::CONSENSUS_PROPOSAL_PAYLOAD_AVAILABILITY
            .with_label_values(&["available"])
            .inc();

        self.check_backpressure_and_process_proposal(proposal).await
    }

    async fn check_backpressure_and_process_proposal(
        &mut self,
        proposal: Block,
    ) -> anyhow::Result<()> {
        let author = proposal
            .author()
            .expect("Proposal should be verified having an author");

        if self.block_store.vote_back_pressure() {
            counters::CONSENSUS_WITHOLD_VOTE_BACKPRESSURE_TRIGGERED.observe(1.0);
            // In case of back pressure, we delay processing proposal. This is done by resending the
            // same proposal to self after some time.
            Self::resend_verified_proposal_to_self(
                self.block_store.clone(),
                self.buffered_proposal_tx.clone(),
                proposal,
                author,
                BACK_PRESSURE_POLLING_INTERVAL_MS,
                self.local_config.round_initial_timeout_ms,
            )
            .await;
            return Ok(());
        }

        counters::CONSENSUS_WITHOLD_VOTE_BACKPRESSURE_TRIGGERED.observe(0.0);
        self.process_verified_proposal(proposal).await
    }

    async fn resend_verified_proposal_to_self(
        block_store: Arc<BlockStore>,
        self_sender: aptos_channel::Sender<Author, VerifiedEvent>,
        proposal: Block,
        author: Author,
        polling_interval_ms: u64,
        timeout_ms: u64,
    ) {
        let start = Instant::now();
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

    async fn create_vote(&mut self, proposal: Block) -> anyhow::Result<Vote> {
        let vote = self
            .vote_block(proposal)
            .await
            .context("[RoundManager] Process proposal")?;

        fail_point!("consensus::create_invalid_vote", |_| {
            use aptos_crypto::bls12381;
            let faulty_vote = Vote::new_with_signature(
                vote.vote_data().clone(),
                vote.author(),
                vote.ledger_info().clone(),
                bls12381::Signature::dummy_signature(),
            );
            Ok(faulty_vote)
        });
        Ok(vote)
    }

    pub async fn process_verified_proposal(&mut self, proposal: Block) -> anyhow::Result<()> {
        let proposal_round = proposal.round();
        let parent_qc = proposal.quorum_cert().clone();
        let sync_info = self.block_store.sync_info();

        if proposal_round <= sync_info.highest_round() {
            sample!(
                SampleRate::Duration(Duration::from_secs(1)),
                warn!(
                    sync_info = sync_info,
                    proposal = proposal,
                    "Ignoring proposal. SyncInfo round is higher than proposal round."
                )
            );
            return Ok(());
        }

        let vote = self.create_vote(proposal).await?;
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

        if let Err(e) = self.start_next_opt_round(vote, parent_qc) {
            debug!("Cannot start next opt round: {}", e);
        };
        Ok(())
    }

    fn start_next_opt_round(
        &self,
        parent_vote: Vote,
        grandparent_qc: QuorumCert,
    ) -> anyhow::Result<()> {
        // Optimistic Proposal:
        // When receiving round r block, send optimistic proposal for round r+1 if:
        // 0. opt proposal is enabled
        // 1. it is the leader of the next round r+1
        // 2. voted for round r block
        // 3. the round r block contains QC of round r-1
        // 4. does not propose in round r+1
        if !self.local_config.enable_optimistic_proposal_tx {
            return Ok(());
        };

        let parent = parent_vote.vote_data().proposed().clone();
        let opt_proposal_round = parent.round() + 1;
        if self
            .proposer_election
            .is_valid_proposer(self.proposal_generator.author(), opt_proposal_round)
        {
            let expected_grandparent_round = parent
                .round()
                .checked_sub(1)
                .ok_or_else(|| anyhow::anyhow!("Invalid parent round {}", parent.round()))?;
            ensure!(
                grandparent_qc.certified_block().round() == expected_grandparent_round,
                "Cannot start Optimistic Round. Grandparent QC is not for round minus one: {} < {}",
                grandparent_qc.certified_block().round(),
                parent.round()
            );

            let epoch_state = self.epoch_state.clone();
            let network = self.network.clone();
            let sync_info = self.block_store.sync_info();
            let proposal_generator = self.proposal_generator.clone();
            let proposer_election = self.proposer_election.clone();
            tokio::spawn(async move {
                if let Err(e) = monitor!(
                    "generate_and_send_opt_proposal",
                    Self::generate_and_send_opt_proposal(
                        epoch_state,
                        opt_proposal_round,
                        parent,
                        grandparent_qc,
                        network,
                        sync_info,
                        proposal_generator,
                        proposer_election,
                    )
                    .await
                ) {
                    warn!(
                        "[OptProposal] Error generating and sending opt proposal: {}",
                        e
                    );
                }
            });
        }
        Ok(())
    }

    /// The function generates a VoteMsg for a given proposed_block:
    /// * add the block to the block store
    /// * then verify the voting rules
    /// * save the updated state to consensus DB
    /// * return a VoteMsg with the LedgerInfo to be committed in case the vote gathers QC.
    async fn vote_block(&mut self, proposed_block: Block) -> anyhow::Result<Vote> {
        let block_arc = self
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

        let vote_proposal = block_arc.vote_proposal();
        let vote_result = self.safety_rules.lock().construct_and_sign_vote_two_chain(
            &vote_proposal,
            self.block_store.highest_2chain_timeout_cert().as_deref(),
        );
        let vote = vote_result.context(format!(
            "[RoundManager] SafetyRules Rejected {}",
            block_arc.block()
        ))?;
        if !block_arc.block().is_nil_block() {
            observe_block(block_arc.block().timestamp_usecs(), BlockStage::VOTED);
        }

        if block_arc.block().is_opt_block() {
            observe_block(
                block_arc.block().timestamp_usecs(),
                BlockStage::VOTED_OPT_BLOCK,
            );
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

            let order_vote = order_vote_msg.order_vote();
            trace!(
                self.new_log(LogEvent::ReceiveOrderVote)
                    .remote_peer(order_vote.author()),
                epoch = order_vote.ledger_info().epoch(),
                round = order_vote.ledger_info().round(),
                id = order_vote.ledger_info().consensus_block_id(),
            );

            if self
                .pending_order_votes
                .has_enough_order_votes(order_vote_msg.order_vote().ledger_info())
            {
                return Ok(());
            }

            let highest_ordered_round = self.block_store.sync_info().highest_ordered_round();
            let order_vote_round = order_vote_msg.order_vote().ledger_info().round();
            let li_digest = order_vote_msg.order_vote().ledger_info().hash();
            if order_vote_round > highest_ordered_round
                && order_vote_round < highest_ordered_round + 100
            {
                // If it is the first order vote received for the block, verify the QC and insert along with QC.
                // For the subsequent order votes for the same block, we don't have to verify the QC. Just inserting the
                // order vote is enough.
                let vote_reception_result = if !self.pending_order_votes.exists(&li_digest) {
                    let start = Instant::now();
                    order_vote_msg
                        .quorum_cert()
                        .verify(&self.epoch_state.verifier)
                        .context("[OrderVoteMsg QuorumCert verification failed")?;
                    counters::VERIFY_MSG
                        .with_label_values(&["order_vote_qc"])
                        .observe(start.elapsed().as_secs_f64());
                    self.pending_order_votes.insert_order_vote(
                        order_vote_msg.order_vote(),
                        &self.epoch_state.verifier,
                        Some(order_vote_msg.quorum_cert().clone()),
                    )
                } else {
                    self.pending_order_votes.insert_order_vote(
                        order_vote_msg.order_vote(),
                        &self.epoch_state.verifier,
                        None,
                    )
                };
                self.process_order_vote_reception_result(
                    vote_reception_result,
                    order_vote_msg.order_vote().author(),
                )
                .await?;
            } else {
                ORDER_VOTE_NOT_IN_RANGE.inc();
                sample!(
                    SampleRate::Duration(Duration::from_secs(1)),
                    info!(
                        "[sampled] Received an order vote not in the 100 rounds. Order vote round: {:?}, Highest ordered round: {:?}",
                        order_vote_msg.order_vote().ledger_info().round(),
                        self.block_store.sync_info().highest_ordered_round()
                    )
                );
                debug!(
                    "Received an order vote not in the next 100 rounds. Order vote round: {:?}, Highest ordered round: {:?}",
                    order_vote_msg.order_vote().ledger_info().round(),
                    self.block_store.sync_info().highest_ordered_round()
                )
            }
        }
        Ok(())
    }

    async fn create_order_vote(
        &mut self,
        block: Arc<PipelinedBlock>,
        qc: Arc<QuorumCert>,
    ) -> anyhow::Result<OrderVote> {
        let order_vote_proposal = block.order_vote_proposal(qc);
        let order_vote_result = self
            .safety_rules
            .lock()
            .construct_and_sign_order_vote(&order_vote_proposal);
        let order_vote = order_vote_result.context(format!(
            "[RoundManager] SafetyRules Rejected {} for order vote",
            block.block()
        ))?;

        fail_point!("consensus::create_invalid_order_vote", |_| {
            use aptos_crypto::bls12381;
            let faulty_order_vote = OrderVote::new_with_signature(
                order_vote.author(),
                order_vote.ledger_info().clone(),
                bls12381::Signature::dummy_signature(),
            );
            Ok(faulty_order_vote)
        });
        Ok(order_vote)
    }

    async fn broadcast_order_vote(
        &mut self,
        vote: &Vote,
        qc: Arc<QuorumCert>,
    ) -> anyhow::Result<()> {
        if let Some(proposed_block) = self.block_store.get_block(vote.vote_data().proposed().id()) {
            // Generate an order vote with ledger_info = proposed_block
            let order_vote = self
                .create_order_vote(proposed_block.clone(), qc.clone())
                .await?;
            if !proposed_block.block().is_nil_block() {
                observe_block(
                    proposed_block.block().timestamp_usecs(),
                    BlockStage::ORDER_VOTED,
                );
            }
            if proposed_block.block().is_opt_block() {
                observe_block(
                    proposed_block.block().timestamp_usecs(),
                    BlockStage::ORDER_VOTED_OPT_BLOCK,
                );
            }
            let order_vote_msg = OrderVoteMsg::new(order_vote, qc.as_ref().clone());
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

        if vote.is_timeout() {
            info!(
                self.new_log(LogEvent::ReceiveVote)
                    .remote_peer(vote.author()),
                vote = %vote,
                epoch = vote.vote_data().proposed().epoch(),
                round = vote.vote_data().proposed().round(),
                id = vote.vote_data().proposed().id(),
                state = vote.vote_data().proposed().executed_state_id(),
                is_timeout = vote.is_timeout(),
            );
        } else {
            trace!(
                self.new_log(LogEvent::ReceiveVote)
                    .remote_peer(vote.author()),
                epoch = vote.vote_data().proposed().epoch(),
                round = vote.vote_data().proposed().round(),
                id = vote.vote_data().proposed().id(),
            );
        }

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
                    // This check is already done in safety rules. As printing the "failed to broadcast order vote"
                    // in humio logs could sometimes look scary, we are doing the same check again here.
                    if let Some(last_sent_vote) = self.round_state.vote_sent() {
                        if let Some((two_chain_timeout, _)) = last_sent_vote.two_chain_timeout() {
                            if round <= two_chain_timeout.round() {
                                return Ok(());
                            }
                        }
                    }
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
            VoteReceptionResult::EchoTimeout(_) if !self.round_state.is_timeout_sent() => {
                self.process_local_timeout(round).await
            },
            VoteReceptionResult::VoteAdded(_) => {
                PROPOSAL_VOTE_ADDED.inc();
                Ok(())
            },
            VoteReceptionResult::EchoTimeout(_) | VoteReceptionResult::DuplicateVote => Ok(()),
            e => Err(anyhow::anyhow!("{:?}", e)),
        }
    }

    async fn process_timeout_reception_result(
        &mut self,
        timeout: &RoundTimeout,
        result: VoteReceptionResult,
    ) -> anyhow::Result<()> {
        let round = timeout.round();
        match result {
            VoteReceptionResult::New2ChainTimeoutCertificate(tc) => {
                self.new_2chain_tc_aggregated(tc).await
            },
            VoteReceptionResult::EchoTimeout(_) if !self.round_state.is_timeout_sent() => {
                self.process_local_timeout(round).await
            },
            VoteReceptionResult::VoteAdded(_) | VoteReceptionResult::EchoTimeout(_) => Ok(()),
            result @ VoteReceptionResult::NewQuorumCertificate(_)
            | result @ VoteReceptionResult::DuplicateVote => {
                bail!("Unexpected result from timeout processing: {:?}", result);
            },
            e => Err(anyhow::anyhow!("{:?}", e)),
        }
    }

    pub async fn process_round_timeout_msg(
        &mut self,
        round_timeout_msg: RoundTimeoutMsg,
    ) -> anyhow::Result<()> {
        fail_point!("consensus::process_round_timeout_msg", |_| {
            Err(anyhow::anyhow!(
                "Injected error in process_round_timeout_msg"
            ))
        });
        // Check whether this validator is a valid recipient of the vote.
        if self
            .ensure_round_and_sync_up(
                round_timeout_msg.round(),
                round_timeout_msg.sync_info(),
                round_timeout_msg.author(),
            )
            .await
            .context("[RoundManager] Stop processing vote")?
        {
            self.process_round_timeout(round_timeout_msg.timeout())
                .await
                .context("[RoundManager] Add a new timeout")?;
        }
        Ok(())
    }

    async fn process_round_timeout(&mut self, timeout: RoundTimeout) -> anyhow::Result<()> {
        info!(
            self.new_log(LogEvent::ReceiveRoundTimeout)
                .remote_peer(timeout.author()),
            vote = %timeout,
            epoch = timeout.epoch(),
            round = timeout.round(),
        );

        let vote_reception_result = self
            .round_state
            .insert_round_timeout(&timeout, &self.epoch_state.verifier);
        self.process_timeout_reception_result(&timeout, vote_reception_result)
            .await
    }

    async fn process_order_vote_reception_result(
        &mut self,
        result: OrderVoteReceptionResult,
        preferred_peer: Author,
    ) -> anyhow::Result<()> {
        match result {
            OrderVoteReceptionResult::NewLedgerInfoWithSignatures((
                verified_qc,
                ledger_info_with_signatures,
            )) => {
                self.new_ordered_cert(
                    WrappedLedgerInfo::new(VoteData::dummy(), ledger_info_with_signatures),
                    verified_qc,
                    preferred_peer,
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

    async fn new_qc_from_order_vote_msg(
        &mut self,
        verified_qc: Arc<QuorumCert>,
        preferred_peer: Author,
    ) -> anyhow::Result<()> {
        match self
            .block_store
            .need_fetch_for_quorum_cert(verified_qc.as_ref())
        {
            NeedFetchResult::QCAlreadyExist => Ok(()),
            NeedFetchResult::QCBlockExist => {
                // If the block is already in the block store, but QC isn't available in the block store, insert QC.
                let result = self
                    .block_store
                    .insert_quorum_cert(
                        verified_qc.as_ref(),
                        &mut self.create_block_retriever(preferred_peer),
                    )
                    .await
                    .context("[RoundManager] Failed to process the QC from order vote msg");
                self.process_certificates().await?;
                result
            },
            NeedFetchResult::NeedFetch => {
                // If the block doesn't exist, we could ideally do sync up based on the qc.
                // But this could trigger fetching a lot of past blocks in case the node is lagging behind.
                // So, we just log a warning here to avoid a long sequence of block fetchs.
                // One of the subsequence syncinfo messages will trigger the block fetch or state sync if required.
                ORDER_CERT_CREATED_WITHOUT_BLOCK_IN_BLOCK_STORE.inc();
                sample!(
                    SampleRate::Duration(Duration::from_millis(200)),
                    info!(
                        "Ordered certificate created without block in block store: {:?}",
                        verified_qc.certified_block()
                    );
                );
                Err(anyhow::anyhow!(
                    "Ordered certificate created without block in block store"
                ))
            },
            NeedFetchResult::QCRoundBeforeRoot => {
                Err(anyhow::anyhow!("Ordered certificate is old"))
            },
        }
    }

    // Insert ordered certificate formed by aggregating order votes
    async fn new_ordered_cert(
        &mut self,
        ordered_cert: WrappedLedgerInfo,
        verified_qc: Arc<QuorumCert>,
        preferred_peer: Author,
    ) -> anyhow::Result<()> {
        self.new_qc_from_order_vote_msg(verified_qc, preferred_peer)
            .await?;

        // If the block and qc now exist in the quorum store, insert the ordered cert
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
        let epoch_state = self.epoch_state.clone();
        let new_round_event = self
            .round_state
            .process_certificates(self.block_store.sync_info(), &epoch_state.verifier)
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

    pub fn round_state(&self) -> &RoundState {
        &self.round_state
    }

    fn new_log(&self, event: LogEvent) -> LogSchema {
        Self::new_log_with_round_epoch(
            event,
            self.round_state().current_round(),
            self.epoch_state.epoch,
        )
    }

    fn new_log_with_round_epoch(event: LogEvent, round: Round, epoch: u64) -> LogSchema {
        LogSchema::new(event).round(round).epoch(epoch)
    }

    /// Mainloop of processing messages.
    #[allow(clippy::unwrap_used)]
    pub async fn start(
        mut self,
        mut event_rx: aptos_channel::Receiver<
            (Author, Discriminant<VerifiedEvent>),
            (Author, VerifiedEvent),
        >,
        mut buffered_proposal_rx: aptos_channel::Receiver<Author, VerifiedEvent>,
        mut opt_proposal_loopback_rx: aptos_channels::UnboundedReceiver<OptBlockData>,
        close_rx: oneshot::Receiver<oneshot::Sender<()>>,
    ) {
        info!(epoch = self.epoch_state.epoch, "RoundManager started");
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
                opt_proposal = opt_proposal_loopback_rx.select_next_some() => {
                    self.pending_opt_proposals = self.pending_opt_proposals.split_off(&opt_proposal.round().add(1));
                    let result = monitor!("process_opt_proposal_loopback", self.process_opt_proposal(opt_proposal).await);
                    let round_state = self.round_state();
                    match result {
                        Ok(_) => trace!(RoundStateLogSchema::new(round_state)),
                        Err(e) => {
                            counters::ERROR_COUNT.inc();
                            warn!(kind = error_kind(&e), RoundStateLogSchema::new(round_state), "Error: {:#}", e);
                        }
                    }
                }
                proposal = buffered_proposal_rx.select_next_some() => {
                    let mut proposals = vec![proposal];
                    while let Some(Some(proposal)) = buffered_proposal_rx.next().now_or_never() {
                        proposals.push(proposal);
                    }
                    let get_round = |event: &VerifiedEvent| {
                        match event {
                            VerifiedEvent::ProposalMsg(p) => p.proposal().round(),
                            VerifiedEvent::VerifiedProposalMsg(p) => p.round(),
                            VerifiedEvent::OptProposalMsg(p) => p.round(),
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
                            VerifiedEvent::OptProposalMsg(proposal_msg) => {
                                monitor!(
                                    "process_opt_proposal",
                                    self.process_opt_proposal_msg(*proposal_msg).await
                                )
                            }
                            unexpected_event => unreachable!("Unexpected event: {:?}", unexpected_event),
                        };
                        let round_state = self.round_state();
                        match result {
                            Ok(_) => trace!(RoundStateLogSchema::new(round_state)),
                            Err(e) => {
                                counters::ERROR_COUNT.inc();
                                warn!(kind = error_kind(&e), RoundStateLogSchema::new(round_state), "Error: {:#}", e);
                            }
                        }
                    }
                },
                Some((result, block, start_time)) = self.futures.next() => {
                    let elapsed = start_time.elapsed().as_secs_f64();
                    let id = block.id();
                    match result {
                        Ok(()) => {
                            counters::CONSENSUS_PROPOSAL_PAYLOAD_FETCH_DURATION.with_label_values(&["success"]).observe(elapsed);
                            if let Err(e) = monitor!("payload_fetch_proposal_process", self.check_backpressure_and_process_proposal(block)).await {
                                warn!("failed process proposal after payload fetch for block {}: {}", id, e);
                            }
                        },
                        Err(err) => {
                            counters::CONSENSUS_PROPOSAL_PAYLOAD_FETCH_DURATION.with_label_values(&["error"]).observe(elapsed);
                            warn!("unable to fetch payload for block {}: {}", id, err);
                        },
                    };
                },
                (peer_id, event) = event_rx.select_next_some() => {
                    let result = match event {
                        VerifiedEvent::VoteMsg(vote_msg) => {
                            monitor!("process_vote", self.process_vote_msg(*vote_msg).await)
                        }
                        VerifiedEvent::RoundTimeoutMsg(timeout_msg) => {
                            monitor!("process_round_timeout", self.process_round_timeout_msg(*timeout_msg).await)
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
                            warn!(kind = error_kind(&e), RoundStateLogSchema::new(round_state), "Error: {:#}", e);
                        }
                    }
                },
            }
        }
        info!(epoch = self.epoch_state.epoch, "RoundManager stopped");
    }

    #[cfg(feature = "failpoints")]
    fn check_whether_to_inject_reconfiguration_error() -> bool {
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
        epoch_state: Arc<EpochState>,
        network: Arc<NetworkSender>,
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
            let mut half_peers: Vec<_> = epoch_state
                .verifier
                .get_ordered_account_addresses_iter()
                .collect();
            half_peers.truncate(half_peers.len() / 2);
            network
                .send_proposal(proposal_msg.clone(), half_peers)
                .await;
            Err(anyhow::anyhow!("Injected error in reconfiguration suffix"))
        } else {
            Ok(())
        }
    }
}
