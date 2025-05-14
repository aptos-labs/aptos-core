// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use super::{
    proposal_status_tracker::TOptQSPullParamsProvider, proposer_election::ProposerElection,
};
use crate::{
    block_storage::BlockReader,
    counters::{
        CHAIN_HEALTH_BACKOFF_TRIGGERED, EXECUTION_BACKPRESSURE_ON_PROPOSAL_TRIGGERED,
        PIPELINE_BACKPRESSURE_ON_PROPOSAL_TRIGGERED, PROPOSER_DELAY_PROPOSAL,
        PROPOSER_ESTIMATED_CALIBRATED_BLOCK_GAS, PROPOSER_ESTIMATED_CALIBRATED_BLOCK_TXNS,
        PROPOSER_MAX_BLOCK_TXNS_AFTER_FILTERING, PROPOSER_MAX_BLOCK_TXNS_TO_EXECUTE,
        PROPOSER_PENDING_BLOCKS_COUNT, PROPOSER_PENDING_BLOCKS_FILL_FRACTION,
    },
    payload_client::PayloadClient,
    util::time_service::TimeService,
};
use anyhow::{bail, ensure, format_err, Context};
use aptos_config::config::{
    ChainHealthBackoffValues, ExecutionBackpressureConfig, ExecutionBackpressureMetric,
    PipelineBackpressureValues,
};
use aptos_consensus_types::{
    block::Block,
    block_data::BlockData,
    common::{Author, Payload, PayloadFilter, Round},
    payload_pull_params::PayloadPullParameters,
    pipelined_block::ExecutionSummary,
    quorum_cert::QuorumCert,
    utils::PayloadTxnsSize,
};
use aptos_crypto::{hash::CryptoHash, HashValue};
use aptos_infallible::Mutex;
use aptos_logger::{error, sample, sample::SampleRate, warn};
use aptos_types::{on_chain_config::ValidatorTxnConfig, validator_txn::ValidatorTransaction};
use aptos_validator_transaction_pool as vtxn_pool;
use futures::future::BoxFuture;
use itertools::Itertools;
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
    execution: Option<ExecutionBackpressureConfig>,
}

impl PipelineBackpressureConfig {
    pub fn new(
        backoffs: Vec<PipelineBackpressureValues>,
        execution: Option<ExecutionBackpressureConfig>,
    ) -> Self {
        let original_len = backoffs.len();
        let backoffs = backoffs
            .into_iter()
            .map(|v| (v.back_pressure_pipeline_latency_limit_ms, v))
            .collect::<BTreeMap<_, _>>();
        assert_eq!(original_len, backoffs.len());
        Self {
            backoffs,
            execution,
        }
    }

    #[allow(dead_code)]
    pub fn new_no_backoff() -> Self {
        Self {
            backoffs: BTreeMap::new(),
            execution: None,
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

    fn compute_lookback_blocks(
        &self,
        block_execution_times: &[ExecutionSummary],
        f: impl Fn(&ExecutionSummary) -> Option<u64>,
    ) -> Vec<u64> {
        block_execution_times
            .iter()
            .flat_map(|summary| {
                // for each block, compute target (re-calibrated) block size
                f(summary)
            })
            .sorted()
            .collect::<Vec<_>>()
    }

    fn compute_lookback_metric(&self, blocks: &[u64], metric: &ExecutionBackpressureMetric) -> u64 {
        match metric {
            ExecutionBackpressureMetric::Mean => {
                if blocks.is_empty() {
                    return 0;
                }
                (blocks.iter().sum::<u64>() as f64 / blocks.len() as f64) as u64
            },
            ExecutionBackpressureMetric::Percentile(percentile) => *blocks
                .get(((percentile * blocks.len() as f64) as usize).min(blocks.len() - 1))
                .expect("guaranteed to be within vector size"),
        }
    }

    /// TODO: disable txn limit backoff and use only gas limit based backoff once execution pool is deployed.
    ///
    /// Until then, we need to compute wanted block size to create.
    /// Unfortunately, there is multiple layers where transactions are filtered.
    /// After deduping/reordering logic is applied, max_txns_to_execute limits the transactions
    /// passed to executor (`summary.payload_len` here), and then some are discarded for various
    /// reasons, which we approximate are cheaply ignored.
    /// For the rest, only `summary.to_commit` fraction of `summary.to_commit + summary.to_retry`
    /// was executed. And so assuming same discard rate, we scale `summary.payload_len` with it.
    fn get_execution_block_size_backoff(
        &self,
        block_execution_times: &[ExecutionSummary],
        max_block_txns: u64,
    ) -> Option<u64> {
        self.execution.as_ref().and_then(|config| {
            let config = config.txn_limit.as_ref()?;

            let lookback_config = &config.lookback_config;
            let min_calibrated_txns_per_block =
                config.min_calibrated_txns_per_block;
            let sizes = self.compute_lookback_blocks(block_execution_times, |summary| {
                let execution_time_ms = summary.execution_time.as_millis();
                // Only block above the time threshold are considered giving enough signal to support calibration
                // so we filter out shorter locks
                if execution_time_ms as u64 > lookback_config.min_block_time_ms_to_activate as u64
                    && summary.payload_len > 0
                {
                    Some(
                        ((lookback_config.target_block_time_ms as f64
                            / summary.execution_time.as_millis() as f64
                            * (summary.to_commit as f64
                                / (summary.to_commit + summary.to_retry) as f64)
                            * summary.payload_len as f64)
                            .floor() as u64)
                            .max(1),
                    )
                } else {
                    None
                }
            });
            if sizes.len() >= lookback_config.min_blocks_to_activate {
                let calibrated_block_size = self
                    .compute_lookback_metric(&sizes, &lookback_config.metric)
                    .max(min_calibrated_txns_per_block);
                PROPOSER_ESTIMATED_CALIBRATED_BLOCK_TXNS.observe(calibrated_block_size as f64);
                // Check if calibrated block size is reduction in size, to turn on backpressure.
                if max_block_txns > calibrated_block_size {
                    warn!(
                        block_execution_times = format!("{:?}", block_execution_times),
                        estimated_calibrated_block_sizes = format!("{:?}", sizes),
                        calibrated_block_size = calibrated_block_size,
                        "Execution backpressure recalibration: txn limit: proposing reducing from {} to {}",
                        max_block_txns,
                        calibrated_block_size,
                    );
                    Some(calibrated_block_size)
                } else {
                    None
                }
            } else {
                None
            }
        })
    }

    /// TODO: once execution pool is deployed, enable gas limit based backoff in execution (via onchain config).
    ///
    /// Until it is enabled in execution, the printed log is useful for auditing and calibrating
    /// the computed backoff.
    fn get_execution_gas_limit_backoff(
        &self,
        block_execution_times: &[ExecutionSummary],
        max_block_gas_limit: u64,
    ) -> Option<u64> {
        self.execution.as_ref().and_then(|config| {
            let config = config.gas_limit.as_ref()?;

            let lookback_config = &config.lookback_config;
            let block_execution_overhead_ms = config.block_execution_overhead_ms;
            let min_calibrated_block_gas_limit =
                config.min_calibrated_block_gas_limit;
            let gas_limit_estimates =
                self.compute_lookback_blocks(block_execution_times, |summary| {
                    let execution_time_ms = summary.execution_time.as_millis() as u64;
                    let execution_time_ms =
                        execution_time_ms.saturating_sub(block_execution_overhead_ms);
                    // Only block above the time threshold are considered giving enough signal to support calibration
                    // so we filter out shorter locks
                    if execution_time_ms > lookback_config.min_block_time_ms_to_activate as u64 {
                        if let Some(gas_used) = summary.gas_used {
                            if gas_used >= min_calibrated_block_gas_limit {
                                Some(
                                    ((lookback_config.target_block_time_ms as f64
                                        / execution_time_ms as f64
                                        * gas_used as f64)
                                        .floor() as u64)
                                        .max(min_calibrated_block_gas_limit),
                                )
                            } else {
                                None
                            }
                        } else {
                            warn!("Block execution summary missing gas used, skipping");
                            None
                        }
                    } else {
                        None
                    }
                });
            if gas_limit_estimates.len() >= lookback_config.min_blocks_to_activate {
                let calibrated_gas_limit = self
                    .compute_lookback_metric(&gas_limit_estimates, &lookback_config.metric)
                    .max(min_calibrated_block_gas_limit);
                PROPOSER_ESTIMATED_CALIBRATED_BLOCK_GAS.observe(calibrated_gas_limit as f64);
                // Check if calibrated block size is reduction in size, to turn on backpressure.
                if max_block_gas_limit > calibrated_gas_limit {
                    warn!(
                        block_execution_times = format!("{:?}", block_execution_times),
                        computed_target_block_gas_limits = format!("{:?}", gas_limit_estimates),
                        computed_target_block_gas_limit = calibrated_gas_limit,
                        "Execution backpressure recalibration: gas limit: proposing reducing from {} to {}",
                        max_block_gas_limit,
                        calibrated_gas_limit,
                    );
                    Some(calibrated_gas_limit)
                } else {
                    None
                }
            } else {
                None
            }
        })
    }

    pub fn num_blocks_to_look_at(&self) -> Option<usize> {
        let config = self.execution.as_ref()?;

        let mut num_blocks_to_look_at = None;
        if let Some(config) = &config.txn_limit {
            num_blocks_to_look_at = Some(
                config
                    .lookback_config
                    .num_blocks_to_look_at
                    .max(num_blocks_to_look_at.unwrap_or(0)),
            );
        }
        if let Some(config) = &config.gas_limit {
            num_blocks_to_look_at = Some(
                config
                    .lookback_config
                    .num_blocks_to_look_at
                    .max(num_blocks_to_look_at.unwrap_or(0)),
            );
        }
        num_blocks_to_look_at
    }

    pub fn get_execution_block_txn_and_gas_limit_backoff(
        &self,
        block_execution_times: &[ExecutionSummary],
        max_block_txns: u64,
        max_block_gas_limit: Option<u64>,
    ) -> (Option<u64>, Option<u64>) {
        let txn_limit_backoff =
            self.get_execution_block_size_backoff(block_execution_times, max_block_txns);
        let gas_limit_backoff = max_block_gas_limit.and_then(|max_block_gas_limit| {
            self.get_execution_gas_limit_backoff(block_execution_times, max_block_gas_limit)
        });
        (txn_limit_backoff, gas_limit_backoff)
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
    // Max number of transactions (count, bytes) to be added to a proposed block.
    max_block_txns: PayloadTxnsSize,
    // Max number of unique transactions to be added to a proposed block.
    max_block_txns_after_filtering: u64,
    // Max number of inline transactions (count, bytes) to be added to a proposed block.
    max_inline_txns: PayloadTxnsSize,
    // Max number of failed authors to be added to a proposed block.
    max_failed_authors_to_store: usize,

    /// If backpressure target block size is below it, update `max_txns_to_execute` instead.
    /// Applied to execution, pipeline and chain health backpressure.
    /// Needed as we cannot subsplit QS batches.
    min_max_txns_in_block_after_filtering_from_backpressure: u64,
    max_block_gas_limit: Option<u64>,

    pipeline_backpressure_config: PipelineBackpressureConfig,
    chain_health_backoff_config: ChainHealthBackoffConfig,

    // Last round that a proposal was generated
    last_round_generated: Mutex<Round>,
    quorum_store_enabled: bool,
    vtxn_config: ValidatorTxnConfig,

    allow_batches_without_pos_in_proposal: bool,
    opt_qs_payload_param_provider: Arc<dyn TOptQSPullParamsProvider>,
}

impl ProposalGenerator {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        author: Author,
        block_store: Arc<dyn BlockReader + Send + Sync>,
        payload_client: Arc<dyn PayloadClient>,
        time_service: Arc<dyn TimeService>,
        quorum_store_poll_time: Duration,
        max_block_txns: PayloadTxnsSize,
        max_block_txns_after_filtering: u64,
        max_inline_txns: PayloadTxnsSize,
        max_failed_authors_to_store: usize,
        min_max_txns_in_block_after_filtering_from_backpressure: u64,
        max_block_gas_limit: Option<u64>,
        pipeline_backpressure_config: PipelineBackpressureConfig,
        chain_health_backoff_config: ChainHealthBackoffConfig,
        quorum_store_enabled: bool,
        vtxn_config: ValidatorTxnConfig,
        allow_batches_without_pos_in_proposal: bool,
        opt_qs_payload_param_provider: Arc<dyn TOptQSPullParamsProvider>,
    ) -> Self {
        Self {
            author,
            block_store,
            payload_client,
            time_service,
            quorum_store_poll_time,
            max_block_txns,
            max_block_txns_after_filtering,
            min_max_txns_in_block_after_filtering_from_backpressure,
            max_inline_txns,
            max_failed_authors_to_store,
            max_block_gas_limit,
            pipeline_backpressure_config,
            chain_health_backoff_config,
            last_round_generated: Mutex::new(0),
            quorum_store_enabled,
            vtxn_config,
            allow_batches_without_pos_in_proposal,
            opt_qs_payload_param_provider,
        }
    }

    pub fn author(&self) -> Author {
        self.author
    }

    /// Creates a NIL block proposal extending the highest certified block from the block store.
    pub fn generate_nil_block(
        &self,
        round: Round,
        proposer_election: Arc<dyn ProposerElection>,
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
        &self,
        round: Round,
        proposer_election: Arc<dyn ProposerElection + Send + Sync>,
        wait_callback: BoxFuture<'static, ()>,
    ) -> anyhow::Result<BlockData> {
        {
            let mut last_round_generated = self.last_round_generated.lock();
            if *last_round_generated < round {
                *last_round_generated = round;
            } else {
                bail!("Already proposed in the round {}", round);
            }
        }
        let maybe_optqs_payload_pull_params = self.opt_qs_payload_param_provider.get_params();

        let hqc = self.ensure_highest_quorum_cert(round)?;

        let (validator_txns, payload, timestamp) = if hqc.certified_block().has_reconfiguration() {
            // Reconfiguration rule - we propose empty blocks with parents' timestamp
            // after reconfiguration until it's committed
            (
                vec![],
                Payload::empty(
                    self.quorum_store_enabled,
                    self.allow_batches_without_pos_in_proposal,
                ),
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

            let (
                max_block_txns,
                max_block_txns_after_filtering,
                max_txns_from_block_to_execute,
                block_gas_limit_override,
                proposal_delay,
            ) = self
                .calculate_max_block_sizes(voting_power_ratio, timestamp, round)
                .await;

            PROPOSER_MAX_BLOCK_TXNS_AFTER_FILTERING.observe(max_block_txns_after_filtering as f64);
            if let Some(max_to_execute) = max_txns_from_block_to_execute {
                PROPOSER_MAX_BLOCK_TXNS_TO_EXECUTE.observe(max_to_execute as f64);
            }

            PROPOSER_DELAY_PROPOSAL.observe(proposal_delay.as_secs_f64());
            if !proposal_delay.is_zero() {
                tokio::time::sleep(proposal_delay).await;
            }

            let max_pending_block_size = pending_blocks
                .iter()
                .map(|block| {
                    block.payload().map_or(PayloadTxnsSize::zero(), |p| {
                        PayloadTxnsSize::new(p.len() as u64, p.size() as u64)
                    })
                })
                .reduce(PayloadTxnsSize::maximum)
                .unwrap_or_default();
            // Use non-backpressure reduced values for computing fill_fraction
            let max_fill_fraction =
                (max_pending_block_size.count() as f32 / self.max_block_txns.count() as f32).max(
                    max_pending_block_size.size_in_bytes() as f32
                        / self.max_block_txns.size_in_bytes() as f32,
                );
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

            let (validator_txns, mut payload) = self
                .payload_client
                .pull_payload(
                    PayloadPullParameters {
                        max_poll_time: self.quorum_store_poll_time.saturating_sub(proposal_delay),
                        max_txns: max_block_txns,
                        max_txns_after_filtering: max_block_txns_after_filtering,
                        soft_max_txns_after_filtering: max_txns_from_block_to_execute
                            .unwrap_or(max_block_txns_after_filtering),
                        max_inline_txns: self.max_inline_txns,
                        maybe_optqs_payload_pull_params,
                        user_txn_filter: payload_filter,
                        pending_ordering,
                        pending_uncommitted_blocks: pending_blocks.len(),
                        recent_max_fill_fraction: max_fill_fraction,
                        block_timestamp: timestamp,
                    },
                    validator_txn_filter,
                    wait_callback,
                )
                .await
                .context("Fail to retrieve payload")?;

            if !payload.is_direct()
                && max_txns_from_block_to_execute.is_some()
                && max_txns_from_block_to_execute.is_some_and(|v| payload.len() as u64 > v)
            {
                payload = payload.transform_to_quorum_store_v2(
                    max_txns_from_block_to_execute,
                    block_gas_limit_override,
                );
            } else if block_gas_limit_override.is_some() {
                payload = payload.transform_to_quorum_store_v2(None, block_gas_limit_override);
            }
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
        &self,
        voting_power_ratio: f64,
        timestamp: Duration,
        round: Round,
    ) -> (PayloadTxnsSize, u64, Option<u64>, Option<u64>, Duration) {
        let mut values_max_block_txns_after_filtering = vec![self.max_block_txns_after_filtering];
        let mut values_max_block = vec![self.max_block_txns];
        let mut values_proposal_delay = vec![Duration::ZERO];
        let mut block_gas_limit_override = None;

        let chain_health_backoff = self
            .chain_health_backoff_config
            .get_backoff(voting_power_ratio);
        if let Some(value) = chain_health_backoff {
            values_max_block_txns_after_filtering
                .push(value.max_sending_block_txns_after_filtering_override);
            values_max_block.push(
                self.max_block_txns
                    .compute_with_bytes(value.max_sending_block_bytes_override),
            );
            values_proposal_delay.push(Duration::from_millis(value.backoff_proposal_delay_ms));
            CHAIN_HEALTH_BACKOFF_TRIGGERED.observe(1.0);
        } else {
            CHAIN_HEALTH_BACKOFF_TRIGGERED.observe(0.0);
        }

        let pipeline_pending_latency = self.block_store.pipeline_pending_latency(timestamp);
        let pipeline_backpressure = self
            .pipeline_backpressure_config
            .get_backoff(pipeline_pending_latency);
        if let Some(value) = pipeline_backpressure {
            values_max_block_txns_after_filtering
                .push(value.max_sending_block_txns_after_filtering_override);
            values_max_block.push(
                self.max_block_txns
                    .compute_with_bytes(value.max_sending_block_bytes_override),
            );
            values_proposal_delay.push(Duration::from_millis(value.backpressure_proposal_delay_ms));
            PIPELINE_BACKPRESSURE_ON_PROPOSAL_TRIGGERED.observe(1.0);
        } else {
            PIPELINE_BACKPRESSURE_ON_PROPOSAL_TRIGGERED.observe(0.0);
        };

        let mut execution_backpressure_applied = false;
        if let Some(num_blocks_to_look_at) =
            self.pipeline_backpressure_config.num_blocks_to_look_at()
        {
            let (txn_limit, gas_limit) = self
                .pipeline_backpressure_config
                .get_execution_block_txn_and_gas_limit_backoff(
                    &self
                        .block_store
                        .get_recent_block_execution_times(num_blocks_to_look_at),
                    self.max_block_txns_after_filtering,
                    self.max_block_gas_limit,
                );
            if let Some(txn_limit) = txn_limit {
                values_max_block_txns_after_filtering.push(txn_limit);
                execution_backpressure_applied = true;
            }
            block_gas_limit_override = gas_limit;
            if gas_limit.is_some() {
                execution_backpressure_applied = true;
            }
        }
        EXECUTION_BACKPRESSURE_ON_PROPOSAL_TRIGGERED.observe(
            if execution_backpressure_applied {
                1.0
            } else {
                0.0
            },
        );

        let max_block_txns_after_filtering = values_max_block_txns_after_filtering
            .into_iter()
            .min()
            .expect("always initialized to at least one value");

        let max_block_size = values_max_block
            .into_iter()
            .reduce(PayloadTxnsSize::minimum)
            .expect("always initialized to at least one value");
        let proposal_delay = values_proposal_delay
            .into_iter()
            .max()
            .expect("always initialized to at least one value");

        let (max_block_txns_after_filtering, max_txns_from_block_to_execute) = if self
            .min_max_txns_in_block_after_filtering_from_backpressure
            > max_block_txns_after_filtering
        {
            (
                self.min_max_txns_in_block_after_filtering_from_backpressure,
                Some(max_block_txns_after_filtering),
            )
        } else {
            (max_block_txns_after_filtering, None)
        };

        warn!(
            pipeline_pending_latency = pipeline_pending_latency.as_millis(),
            proposal_delay_ms = proposal_delay.as_millis(),
            max_block_txns_after_filtering = max_block_txns_after_filtering,
            max_txns_from_block_to_execute =
                max_txns_from_block_to_execute.unwrap_or(max_block_txns_after_filtering),
            max_block_size = max_block_size,
            block_gas_limit_override =
                block_gas_limit_override.unwrap_or(self.max_block_gas_limit.unwrap_or(0)),
            is_pipeline_backpressure = pipeline_backpressure.is_some(),
            is_execution_backpressure = execution_backpressure_applied,
            is_chain_health_backoff = chain_health_backoff.is_some(),
            round = round,
            "Proposal generation backpressure details",
        );

        (
            max_block_size,
            max_block_txns_after_filtering,
            max_txns_from_block_to_execute,
            block_gas_limit_override,
            proposal_delay,
        )
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
        proposer_election: Arc<dyn ProposerElection>,
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
