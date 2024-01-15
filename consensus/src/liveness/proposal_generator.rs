// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use super::{
    proposer_election::ProposerElection, unequivocal_proposer_election::UnequivocalProposerElection,
};
use crate::{
    block_storage::BlockReader,
    counters::{
        CHAIN_HEALTH_BACKOFF_TRIGGERED, PIPELINE_BACKPRESSURE_ON_PROPOSAL_TRIGGERED,
        PROPOSER_DELAY_PROPOSAL, PROPOSER_PENDING_BLOCKS_COUNT,
        PROPOSER_PENDING_BLOCKS_FILL_FRACTION,
    },
    payload_client::PayloadClient,
    util::time_service::TimeService,
};
use anyhow::{bail, ensure, format_err, Context};
use aptos_config::config::{ChainHealthBackoffValues, PipelineBackpressureValues};
use aptos_consensus_types::{
    block::Block,
    block_data::BlockData,
    common::{Author, Payload, PayloadFilter, Round},
    quorum_cert::QuorumCert,
};
use aptos_crypto::{hash::CryptoHash, HashValue};
use aptos_logger::{error, sample, sample::SampleRate, warn};
use aptos_types::{on_chain_config::ValidatorTxnConfig, validator_txn::ValidatorTransaction};
use aptos_validator_transaction_pool as vtxn_pool;
use futures::future::BoxFuture;
use std::{
    collections::{BTreeMap, HashSet},
    sync::Arc,
    time::Duration,
};

#[cfg(test)]
#[path = "proposal_generator_test.rs"]
mod proposal_generator_test;

#[derive(Clone)]
pub struct ChainHealthBackoffConfig {
    backoffs: BTreeMap<usize, ChainHealthBackoffValues>,
}

impl ChainHealthBackoffConfig {
    pub fn new(backoffs: Vec<ChainHealthBackoffValues>) -> Self {
        let original_len = backoffs.len();
        let backoffs = backoffs
            .into_iter()
            .map(|v| (v.backoff_if_below_participating_voting_power_percentage, v))
            .collect::<BTreeMap<_, _>>();
        assert_eq!(original_len, backoffs.len());
        Self { backoffs }
    }

    #[allow(dead_code)]
    pub fn new_no_backoff() -> Self {
        Self {
            backoffs: BTreeMap::new(),
        }
    }

    pub fn get_backoff(&self, voting_power_ratio: f64) -> Option<&ChainHealthBackoffValues> {
        if self.backoffs.is_empty() {
            return None;
        }

        if voting_power_ratio < 2.0 / 3.0 {
            error!("Voting power ratio {} is below 2f + 1", voting_power_ratio);
        }
        let voting_power_percentage = (voting_power_ratio * 100.0).floor() as usize;
        if voting_power_percentage > 100 {
            error!(
                "Voting power participation percentatge {} is > 100, before rounding {}",
                voting_power_percentage, voting_power_ratio
            );
        }
        self.backoffs
            .range(voting_power_percentage..)
            .next()
            .map(|(_, v)| {
                sample!(
                    SampleRate::Duration(Duration::from_secs(10)),
                    warn!(
                        "Using chain health backoff config for {} voting power percentage: {:?}",
                        voting_power_percentage, v
                    )
                );
                v
            })
    }
}

#[derive(Clone)]
pub struct PipelineBackpressureConfig {
    backoffs: BTreeMap<Round, PipelineBackpressureValues>,
}

impl PipelineBackpressureConfig {
    pub fn new(backoffs: Vec<PipelineBackpressureValues>) -> Self {
        let original_len = backoffs.len();
        let backoffs = backoffs
            .into_iter()
            .map(|v| (v.back_pressure_pipeline_latency_limit_ms, v))
            .collect::<BTreeMap<_, _>>();
        assert_eq!(original_len, backoffs.len());
        Self { backoffs }
    }

    #[allow(dead_code)]
    pub fn new_no_backoff() -> Self {
        Self {
            backoffs: BTreeMap::new(),
        }
    }

    pub fn get_backoff(
        &self,
        pipeline_pending_latency: Duration,
    ) -> Option<&PipelineBackpressureValues> {
        if self.backoffs.is_empty() {
            return None;
        }

        self.backoffs
            .range(..(pipeline_pending_latency.as_millis() as u64))
            .last()
            .map(|(_, v)| {
                sample!(
                    SampleRate::Duration(Duration::from_secs(10)),
                    warn!(
                        "Using consensus backpressure config for {}ms pending duration: {:?}",
                        pipeline_pending_latency.as_millis(),
                        v
                    )
                );
                v
            })
    }
}

/// ProposalGenerator is responsible for generating the proposed block on demand: it's typically
/// used by a validator that believes it's a valid candidate for serving as a proposer at a given
/// round.
/// ProposalGenerator is the one choosing the branch to extend:
/// - round is given by the caller (typically determined by RoundState).
/// The transactions for the proposed block are delivered by PayloadClient.
///
/// PayloadClient should be aware of the pending transactions in the branch that it is extending,
/// such that it will filter them out to avoid transaction duplication.
pub struct ProposalGenerator {
    // The account address of this validator
    author: Author,
    // Block store is queried both for finding the branch to extend and for generating the
    // proposed block.
    block_store: Arc<dyn BlockReader + Send + Sync>,
    // ProofOfStore manager is delivering the ProofOfStores.
    payload_client: Arc<dyn PayloadClient>,
    // Transaction manager is delivering the transactions.
    // Time service to generate block timestamps
    time_service: Arc<dyn TimeService>,
    // Max time for preparation of the proposal
    quorum_store_poll_time: Duration,
    // Max number of transactions to be added to a proposed block.
    max_block_txns: u64,
    // Max number of bytes to be added to a proposed block.
    max_block_bytes: u64,
    // Max number of failed authors to be added to a proposed block.
    max_failed_authors_to_store: usize,

    pipeline_backpressure_config: PipelineBackpressureConfig,
    chain_health_backoff_config: ChainHealthBackoffConfig,

    // Last round that a proposal was generated
    last_round_generated: Round,
    quorum_store_enabled: bool,
    vtxn_config: ValidatorTxnConfig,
}

impl ProposalGenerator {
    pub fn new(
        author: Author,
        block_store: Arc<dyn BlockReader + Send + Sync>,
        payload_client: Arc<dyn PayloadClient>,
        time_service: Arc<dyn TimeService>,
        quorum_store_poll_time: Duration,
        max_block_txns: u64,
        max_block_bytes: u64,
        max_failed_authors_to_store: usize,
        pipeline_backpressure_config: PipelineBackpressureConfig,
        chain_health_backoff_config: ChainHealthBackoffConfig,
        quorum_store_enabled: bool,
        vtxn_config: ValidatorTxnConfig,
    ) -> Self {
        Self {
            author,
            block_store,
            payload_client,
            time_service,
            quorum_store_poll_time,
            max_block_txns,
            max_block_bytes,
            max_failed_authors_to_store,
            pipeline_backpressure_config,
            chain_health_backoff_config,
            last_round_generated: 0,
            quorum_store_enabled,
            vtxn_config,
        }
    }

    pub fn author(&self) -> Author {
        self.author
    }

    /// Creates a NIL block proposal extending the highest certified block from the block store.
    pub fn generate_nil_block(
        &self,
        round: Round,
        proposer_election: &mut UnequivocalProposerElection,
    ) -> anyhow::Result<Block> {
        let hqc = self.ensure_highest_quorum_cert(round)?;
        let quorum_cert = hqc.as_ref().clone();
        let failed_authors = self.compute_failed_authors(
            round, // to include current round, as that is what failed
            quorum_cert.certified_block().round(),
            true,
            proposer_election,
        );
        Ok(Block::new_nil(round, quorum_cert, failed_authors))
    }

    /// The function generates a new proposal block: the returned future is fulfilled when the
    /// payload is delivered by the PayloadClient implementation.  At most one proposal can be
    /// generated per round (no proposal equivocation allowed).
    /// Errors returned by the PayloadClient implementation are propagated to the caller.
    /// The logic for choosing the branch to extend is as follows:
    /// 1. The function gets the highest head of a one-chain from block tree.
    /// The new proposal must extend hqc to ensure optimistic responsiveness.
    /// 2. The round is provided by the caller.
    /// 3. In case a given round is not greater than the calculated parent, return an OldRound
    /// error.
    pub async fn generate_proposal(
        &mut self,
        round: Round,
        proposer_election: &mut UnequivocalProposerElection,
        wait_callback: BoxFuture<'static, ()>,
    ) -> anyhow::Result<BlockData> {
        if self.last_round_generated < round {
            self.last_round_generated = round;
        } else {
            bail!("Already proposed in the round {}", round);
        }

        let hqc = self.ensure_highest_quorum_cert(round)?;

        let (validator_txns, payload, timestamp) = if hqc.certified_block().has_reconfiguration() {
            // Reconfiguration rule - we propose empty blocks with parents' timestamp
            // after reconfiguration until it's committed
            (
                vec![],
                Payload::empty(self.quorum_store_enabled),
                hqc.certified_block().timestamp_usecs(),
            )
        } else {
            // One needs to hold the blocks with the references to the payloads while get_block is
            // being executed: pending blocks vector keeps all the pending ancestors of the extended branch.
            let mut pending_blocks = self
                .block_store
                .path_from_commit_root(hqc.certified_block().id())
                .ok_or_else(|| format_err!("HQC {} already pruned", hqc.certified_block().id()))?;
            // Avoid txn manager long poll if the root block has txns, so that the leader can
            // deliver the commit proof to others without delay.
            pending_blocks.push(self.block_store.commit_root());

            // Exclude all the pending transactions: these are all the ancestors of
            // parent (including) up to the root (including).
            let exclude_payload: Vec<_> = pending_blocks
                .iter()
                .flat_map(|block| block.payload())
                .collect();
            let payload_filter = PayloadFilter::from(&exclude_payload);

            let pending_ordering = self
                .block_store
                .path_from_ordered_root(hqc.certified_block().id())
                .ok_or_else(|| format_err!("HQC {} already pruned", hqc.certified_block().id()))?
                .iter()
                .any(|block| !block.payload().map_or(true, |txns| txns.is_empty()));

            // All proposed blocks in a branch are guaranteed to have increasing timestamps
            // since their predecessor block will not be added to the BlockStore until
            // the local time exceeds it.
            let timestamp = self.time_service.get_current_timestamp();

            let voting_power_ratio = proposer_election.get_voting_power_participation_ratio(round);

            let (max_block_txns, max_block_bytes, proposal_delay) = self
                .calculate_max_block_sizes(voting_power_ratio, timestamp, round)
                .await;

            PROPOSER_DELAY_PROPOSAL.set(proposal_delay.as_secs_f64());
            if !proposal_delay.is_zero() {
                tokio::time::sleep(proposal_delay).await;
            }

            let max_pending_block_len = pending_blocks
                .iter()
                .map(|block| block.payload().map_or(0, |p| p.len()))
                .max()
                .unwrap_or(0);
            let max_pending_block_bytes = pending_blocks
                .iter()
                .map(|block| block.payload().map_or(0, |p| p.size()))
                .max()
                .unwrap_or(0);
            // Use non-backpressure reduced values for computing fill_fraction
            let max_fill_fraction = (max_pending_block_len as f32 / self.max_block_txns as f32)
                .max(max_pending_block_bytes as f32 / self.max_block_bytes as f32);
            PROPOSER_PENDING_BLOCKS_COUNT.set(pending_blocks.len() as i64);
            PROPOSER_PENDING_BLOCKS_FILL_FRACTION.set(max_fill_fraction as f64);

            let pending_validator_txn_hashes: HashSet<HashValue> = pending_blocks
                .iter()
                .filter_map(|block| block.validator_txns())
                .flatten()
                .map(ValidatorTransaction::hash)
                .collect();
            let validator_txn_filter =
                vtxn_pool::TransactionFilter::PendingTxnHashSet(pending_validator_txn_hashes);
            let (validator_txns, payload) = self
                .payload_client
                .pull_payload(
                    self.quorum_store_poll_time.saturating_sub(proposal_delay),
                    max_block_txns,
                    max_block_bytes,
                    validator_txn_filter,
                    payload_filter,
                    wait_callback,
                    pending_ordering,
                    pending_blocks.len(),
                    max_fill_fraction,
                )
                .await
                .context("Fail to retrieve payload")?;

            (validator_txns, payload, timestamp.as_micros() as u64)
        };

        let quorum_cert = hqc.as_ref().clone();
        let failed_authors = self.compute_failed_authors(
            round,
            quorum_cert.certified_block().round(),
            false,
            proposer_election,
        );

        let block = if self.vtxn_config.enabled() {
            BlockData::new_proposal_ext(
                validator_txns,
                payload,
                self.author,
                failed_authors,
                round,
                timestamp,
                quorum_cert,
            )
        } else {
            BlockData::new_proposal(
                payload,
                self.author,
                failed_authors,
                round,
                timestamp,
                quorum_cert,
            )
        };

        Ok(block)
    }

    async fn calculate_max_block_sizes(
        &mut self,
        voting_power_ratio: f64,
        timestamp: Duration,
        round: Round,
    ) -> (u64, u64, Duration) {
        let mut values_max_block_txns = vec![self.max_block_txns];
        let mut values_max_block_bytes = vec![self.max_block_bytes];
        let mut values_proposal_delay = vec![Duration::ZERO];

        let chain_health_backoff = self
            .chain_health_backoff_config
            .get_backoff(voting_power_ratio);
        if let Some(value) = chain_health_backoff {
            values_max_block_txns.push(value.max_sending_block_txns_override);
            values_max_block_bytes.push(value.max_sending_block_bytes_override);
            values_proposal_delay.push(Duration::from_millis(value.backoff_proposal_delay_ms));
            CHAIN_HEALTH_BACKOFF_TRIGGERED.observe(1.0);
        } else {
            CHAIN_HEALTH_BACKOFF_TRIGGERED.observe(0.0);
        }

        let pipeline_backpressure = self
            .pipeline_backpressure_config
            .get_backoff(self.block_store.pipeline_pending_latency(timestamp));
        if let Some(value) = pipeline_backpressure {
            values_max_block_txns.push(value.max_sending_block_txns_override);
            values_max_block_bytes.push(value.max_sending_block_bytes_override);
            values_proposal_delay.push(Duration::from_millis(value.backpressure_proposal_delay_ms));
            PIPELINE_BACKPRESSURE_ON_PROPOSAL_TRIGGERED.observe(1.0);
        } else {
            PIPELINE_BACKPRESSURE_ON_PROPOSAL_TRIGGERED.observe(0.0);
        };

        let max_block_txns = values_max_block_txns.into_iter().min().unwrap();
        let max_block_bytes = values_max_block_bytes.into_iter().min().unwrap();
        let proposal_delay = values_proposal_delay.into_iter().max().unwrap();

        if pipeline_backpressure.is_some() || chain_health_backoff.is_some() {
            warn!(
                "Generating proposal: reducing limits to {} txns and {} bytes, due to pipeline_backpressure: {}, chain health backoff: {}. Delaying sending proposal by {}ms. Round: {}",
                max_block_txns,
                max_block_bytes,
                pipeline_backpressure.is_some(),
                chain_health_backoff.is_some(),
                proposal_delay.as_millis(),
                round,
            );
        }
        (max_block_txns, max_block_bytes, proposal_delay)
    }

    fn ensure_highest_quorum_cert(&self, round: Round) -> anyhow::Result<Arc<QuorumCert>> {
        let hqc = self.block_store.highest_quorum_cert();
        ensure!(
            hqc.certified_block().round() < round,
            "Given round {} is lower than hqc round {}",
            round,
            hqc.certified_block().round()
        );
        ensure!(
            !hqc.ends_epoch(),
            "The epoch has already ended,a proposal is not allowed to generated"
        );

        Ok(hqc)
    }

    /// Compute the list of consecutive proposers from the
    /// immediately preceeding rounds that didn't produce a successful block
    pub fn compute_failed_authors(
        &self,
        round: Round,
        previous_round: Round,
        include_cur_round: bool,
        proposer_election: &mut UnequivocalProposerElection,
    ) -> Vec<(Round, Author)> {
        let end_round = round + u64::from(include_cur_round);
        let mut failed_authors = Vec::new();
        let start = std::cmp::max(
            previous_round + 1,
            end_round.saturating_sub(self.max_failed_authors_to_store as u64),
        );
        for i in start..end_round {
            failed_authors.push((i, proposer_election.get_valid_proposer(i)));
        }

        failed_authors
    }
}
