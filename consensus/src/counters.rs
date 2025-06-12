// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    block_storage::tracing::{observe_block, BlockStage},
    quorum_store,
};
use aptos_consensus_types::pipelined_block::PipelinedBlock;
use aptos_metrics_core::{
    exponential_buckets, op_counters::DurationHistogram, register_avg_counter, register_counter,
    register_gauge, register_gauge_vec, register_histogram, register_histogram_vec,
    register_int_counter, register_int_counter_vec, register_int_gauge, register_int_gauge_vec,
    Counter, Gauge, GaugeVec, Histogram, HistogramVec, IntCounter, IntCounterVec, IntGauge,
    IntGaugeVec,
};
use aptos_types::transaction::TransactionStatus;
use move_core_types::vm_status::DiscardedVMStatus;
use once_cell::sync::Lazy;
use std::sync::Arc;

/// Transaction commit was successful
pub const TXN_COMMIT_SUCCESS_LABEL: &str = "success";
/// Transaction commit failed (will not be retried)
pub const TXN_COMMIT_FAILED_LABEL: &str = "failed";
/// Transaction commit failed (will not be retried) because of a duplicate
pub const TXN_COMMIT_FAILED_DUPLICATE_LABEL: &str = "failed_duplicate";
/// Transaction commit was unsuccessful, but will be retried
pub const TXN_COMMIT_RETRY_LABEL: &str = "retry";

//////////////////////
// HEALTH COUNTERS
//////////////////////

/// Monitor counters, used by monitor! macro
pub static OP_COUNTERS: Lazy<aptos_metrics_core::op_counters::OpMetrics> =
    Lazy::new(|| aptos_metrics_core::op_counters::OpMetrics::new_and_registered("consensus"));

/// Counts the total number of errors
pub static ERROR_COUNT: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "aptos_consensus_error_count",
        "Total number of errors in main loop"
    )
    .unwrap()
});

/// This counter is set to the round of the highest committed block.
pub static LAST_COMMITTED_ROUND: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "aptos_consensus_last_committed_round",
        "This counter is set to the round of the highest committed block."
    )
    .unwrap()
});

/// The counter corresponds to the version of the last committed ledger info.
pub static LAST_COMMITTED_VERSION: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "aptos_consensus_last_committed_version",
        "The counter corresponds to the version of the last committed ledger info."
    )
    .unwrap()
});

/// Count of the committed failed rounds since last restart.
pub static COMMITTED_FAILED_ROUNDS_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "aptos_consensus_committed_failed_rounds_count",
        "Count of the committed failed rounds since last restart."
    )
    .unwrap()
});

/// Count of the committed blocks since last restart.
pub static COMMITTED_BLOCKS_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "aptos_consensus_committed_blocks_count",
        "Count of the committed blocks since last restart."
    )
    .unwrap()
});

/// Count of the committed transactions since last restart.
pub static COMMITTED_TXNS_COUNT: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_consensus_committed_txns_count",
        "Count of the transactions since last restart. state is success or failed",
        &["state"]
    )
    .unwrap()
});

//////////////////////
// PROPOSAL VOTE COUNTERS
//////////////////////

pub static PROPOSAL_VOTE_ADDED: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "aptos_consensus_proposal_vote_added",
        "Count of the number of proposal votes added to pending votes"
    )
    .unwrap()
});

pub static QC_AGGREGATED_FROM_VOTES: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "aptos_consensus_qc_aggregated_from_votes",
        "Count of the number of QC aggregated from votes"
    )
    .unwrap()
});

pub static PROPOSAL_VOTE_BROADCASTED: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "aptos_consensus_proposal_vote_broadcasted",
        "Count of the number of proposal votes broadcasted"
    )
    .unwrap()
});

//////////////////////
// PROPOSAL ELECTION
//////////////////////

/// Count of the block proposals sent by this validator since last restart
/// (both primary and secondary)
pub static PROPOSALS_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!("aptos_consensus_proposals_count", "Count of the block proposals sent by this validator since last restart (both primary and secondary)").unwrap()
});

/// Count the number of times a validator voted for a nil block since last restart.
pub static VOTE_NIL_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "aptos_consensus_vote_nil_count",
        "Count the number of times a validator voted for a nil block since last restart."
    )
    .unwrap()
});

/// Total voting power of validators in validator set
pub static TOTAL_VOTING_POWER: Lazy<Gauge> = Lazy::new(|| {
    register_gauge!(
        "aptos_total_voting_power",
        "Total voting power of validators in validator set"
    )
    .unwrap()
});

/// Number of distinct senders in a block
pub static NUM_SENDERS_IN_BLOCK: Lazy<Gauge> = Lazy::new(|| {
    register_gauge!("num_senders_in_block", "Total number of senders in a block").unwrap()
});

/// Transaction shuffling call latency
pub static TXN_SHUFFLE_SECONDS: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        // metric name
        "aptos_execution_transaction_shuffle_seconds",
        // metric description
        "The time spent in seconds in shuffle of transactions",
        exponential_buckets(/*start=*/ 1e-6, /*factor=*/ 2.0, /*count=*/ 30).unwrap(),
    )
    .unwrap()
});

/// Transaction dedup call latency
pub static TXN_DEDUP_SECONDS: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        // metric name
        "aptos_execution_transaction_dedup_seconds",
        // metric description
        "The time spent in seconds in dedup of transaction",
        exponential_buckets(/*start=*/ 1e-6, /*factor=*/ 2.0, /*count=*/ 30).unwrap(),
    )
    .unwrap()
});

/// Transaction dedup number of filtered
pub static TXN_DEDUP_FILTERED: Lazy<Histogram> = Lazy::new(|| {
    register_avg_counter(
        "aptos_execution_transaction_dedup_filtered",
        "The number of duplicates filtered per block",
    )
});

/// Number of rounds we were collecting votes for proposer
/// (similar to PROPOSALS_COUNT, but can be larger, if we failed in creating/sending of the proposal)
pub static PROPOSER_COLLECTED_ROUND_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "aptos_proposer_collecting_round_count",
        "Total voting power of all votes collected for the round this node was proposer",
    )
    .unwrap()
});

/// Total voting power of all votes collected for the same ledger info
/// for the rounds this node was a proposer (cumulative)
pub static PROPOSER_COLLECTED_MOST_VOTING_POWER: Lazy<Counter> = Lazy::new(|| {
    register_counter!(
        "aptos_proposer_collected_most_voting_power_sum",
        "Total voting power of all votes collected for the same ledger info for the rounds this node was a proposer",
    )
        .unwrap()
});

/// Total voting power of all votes collected for all other ledger info
/// for the rounds this node was a proposer
pub static PROPOSER_COLLECTED_CONFLICTING_VOTING_POWER: Lazy<Counter> = Lazy::new(|| {
    register_counter!(
        "aptos_proposer_collected_conflicting_voting_power_sum",
        "Total voting power of all votes collected for all other ledger info for the rounds this node was a proposer",
    )
        .unwrap()
});

/// Total voting power of all votes collected for all other ledger info
/// for the rounds this node was a proposer
pub static PROPOSER_COLLECTED_TIMEOUT_VOTING_POWER: Lazy<Counter> = Lazy::new(|| {
    register_counter!(
        "aptos_proposer_collected_timeout_voting_power_sum",
        "Total voting power of all votes collected for the same ledger info for the rounds this node was a proposer",
    )
        .unwrap()
});

/// Committed proposals map when using LeaderReputation as the ProposerElection
pub static COMMITTED_PROPOSALS_IN_WINDOW: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "aptos_committed_proposals_in_window",
        "Total number committed proposals in the current reputation window",
    )
    .unwrap()
});

/// Failed proposals map when using LeaderReputation as the ProposerElection
pub static FAILED_PROPOSALS_IN_WINDOW: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "aptos_failed_proposals_in_window",
        "Total number of failed proposals in the current reputation window",
    )
    .unwrap()
});

/// Committed votes map when using LeaderReputation as the ProposerElection
pub static COMMITTED_VOTES_IN_WINDOW: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "aptos_committed_votes_in_window",
        "Total number of committed votes in the current reputation window",
    )
    .unwrap()
});

/// The number of block events the LeaderReputation uses
pub static LEADER_REPUTATION_ROUND_HISTORY_SIZE: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "aptos_leader_reputation_round_history_size",
        "Total number of new block events in the current reputation window"
    )
    .unwrap()
});

/// Counts when chain_health backoff is triggered
pub static CONSENSUS_WITHOLD_VOTE_BACKPRESSURE_TRIGGERED: Lazy<Histogram> = Lazy::new(|| {
    register_avg_counter(
        "aptos_consensus_withold_vote_backpressure_triggered",
        "Counts when consensus vote_backpressure is triggered",
    )
});

/// Counts when chain_health backoff is triggered
pub static CHAIN_HEALTH_BACKOFF_TRIGGERED: Lazy<Histogram> = Lazy::new(|| {
    register_avg_counter(
        "aptos_chain_health_backoff_triggered",
        "Counts when chain_health backoff is triggered",
    )
});

/// Counts when waiting for full blocks is triggered
pub static WAIT_FOR_FULL_BLOCKS_TRIGGERED: Lazy<Histogram> = Lazy::new(|| {
    register_avg_counter(
        "aptos_wait_for_full_blocks_triggered",
        "Counts when waiting for full blocks is triggered",
    )
});

/// Counts when chain_health backoff is triggered
pub static PIPELINE_BACKPRESSURE_ON_PROPOSAL_TRIGGERED: Lazy<Histogram> = Lazy::new(|| {
    register_avg_counter(
        "aptos_pipeline_backpressure_on_proposal_triggered",
        "Counts when chain_health backoff is triggered",
    )
});

/// number of rounds pending when creating proposal
pub static CONSENSUS_PROPOSAL_PENDING_ROUNDS: Lazy<Histogram> = Lazy::new(|| {
    register_avg_counter(
        "aptos_consensus_proposal_pending_rounds",
        "number of rounds pending when creating proposal",
    )
});

/// duration pending when creating proposal
pub static CONSENSUS_PROPOSAL_PENDING_DURATION: Lazy<DurationHistogram> = Lazy::new(|| {
    DurationHistogram::new(
        register_histogram!(
            "aptos_consensus_proposal_pending_duration",
            "duration pending when creating proposal",
        )
        .unwrap(),
    )
});

/// Amount of time (in seconds) proposal is delayed due to backpressure/backoff
pub static PROPOSER_DELAY_PROPOSAL: Lazy<Gauge> = Lazy::new(|| {
    register_gauge!(
        "aptos_proposer_delay_proposal",
        "Amount of time (in seconds) proposal is delayed due to backpressure/backoff",
    )
    .unwrap()
});

/// How many pending blocks are there, when we make a proposal
pub static PROPOSER_PENDING_BLOCKS_COUNT: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "aptos_proposer_pending_blocks_count",
        "How many pending blocks are there, when we make a proposal",
    )
    .unwrap()
});

/// How full is a largest pending block, as a fraction of max len/bytes (between 0 and 1)
pub static PROPOSER_PENDING_BLOCKS_FILL_FRACTION: Lazy<Gauge> = Lazy::new(|| {
    register_gauge!(
        "aptos_proposer_pending_blocks_fill_fraction",
        "How full is a largest recent pending block, as a fraction of max len/bytes (between 0 and 1)",
    )
    .unwrap()
});

/// Next set of counters are computed at leader election time, with some delay.

/// Current voting power fraction that participated in consensus
/// (voted or proposed) in the reputation window, used for chain-health
/// based backoff
pub static CHAIN_HEALTH_REPUTATION_PARTICIPATING_VOTING_POWER_FRACTION: Lazy<Gauge> =
    Lazy::new(|| {
        register_gauge!(
            "aptos_chain_health_participating_voting_power_fraction_last_reputation_rounds",
            "Total voting power of validators in validator set"
        )
        .unwrap()
    });

/// Window sizes for which to measure chain health.
pub static CHAIN_HEALTH_WINDOW_SIZES: [usize; 4] = [10, 30, 100, 300];

/// Current (with some delay) total voting power
pub static CHAIN_HEALTH_TOTAL_VOTING_POWER: Lazy<Gauge> = Lazy::new(|| {
    register_gauge!(
        "aptos_chain_health_total_voting_power",
        "Total voting power of validators in validator set"
    )
    .unwrap()
});

/// Current (with some delay) total number of validators
pub static CHAIN_HEALTH_TOTAL_NUM_VALIDATORS: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "aptos_chain_health_total_num_validators",
        "Total number of validators in validator set"
    )
    .unwrap()
});

/// Current (with some delay) voting power that participated in consensus
/// (voted or proposed) in the given window.
pub static CHAIN_HEALTH_PARTICIPATING_VOTING_POWER: Lazy<Vec<Gauge>> = Lazy::new(|| {
    CHAIN_HEALTH_WINDOW_SIZES
        .iter()
        .map(|i| {
            register_gauge!(
                format!(
                    "aptos_chain_health_participating_voting_power_last_{}_rounds",
                    i
                ),
                "Current (with some delay) voting power that participated in consensus (voted or proposed) in the given window."
            )
                .unwrap()
        })
        .collect()
});

/// Current (with some delay) number of validators that participated in consensus
/// (voted or proposed) in the given window.
pub static CHAIN_HEALTH_PARTICIPATING_NUM_VALIDATORS: Lazy<Vec<IntGauge>> = Lazy::new(|| {
    CHAIN_HEALTH_WINDOW_SIZES
        .iter()
        .map(|i| {
            register_int_gauge!(
                format!(
                    "aptos_chain_health_participating_num_validators_last_{}_rounds",
                    i
                ),
                "Current (with some delay) number of validators that participated in consensus (voted or proposed) in the given window."
            )
                .unwrap()
        })
        .collect()
});

/// Emits consensus participation status for all peers, 0 means no participation in the window
/// 1 otherwise.
pub static CONSENSUS_PARTICIPATION_STATUS: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "aptos_consensus_participation_status",
        "Counter for consensus participation status, 0 means no participation and 1 otherwise",
        &["peer_id"]
    )
    .unwrap()
});

/// Voting power of the validator
pub static VALIDATOR_VOTING_POWER: Lazy<Gauge> = Lazy::new(|| {
    register_gauge!(
        "aptos_validator_voting_power",
        "Voting power of the validator"
    )
    .unwrap()
});

/// Emits voting power for all validators in the current epoch.
pub static ALL_VALIDATORS_VOTING_POWER: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "aptos_all_validators_voting_power",
        "Voting power for all validators in current epoch",
        &["peer_id"]
    )
    .unwrap()
});

/// For the current ordering round, voting power needed for quorum.
pub static CONSENSUS_CURRENT_ROUND_QUORUM_VOTING_POWER: Lazy<Gauge> = Lazy::new(|| {
    register_gauge!(
        "aptos_consensus_current_round_quorum_voting_power",
        "Counter for consensus participation status, 0 means no participation and 1 otherwise",
    )
    .unwrap()
});

/// For the current ordering round, for each peer, whether they have voted, and for which hash_index
pub static CONSENSUS_CURRENT_ROUND_VOTED_POWER: Lazy<GaugeVec> = Lazy::new(|| {
    register_gauge_vec!(
        "aptos_consensus_current_round_voted_power",
        "Counter for consensus participation status, 0 means no participation and 1 otherwise",
        &["peer_id", "hash_index"]
    )
    .unwrap()
});

/// For the current ordering round, for each peer, whether they have voted for a timeout
pub static CONSENSUS_CURRENT_ROUND_TIMEOUT_VOTED_POWER: Lazy<GaugeVec> = Lazy::new(|| {
    register_gauge_vec!(
        "aptos_consensus_current_round_timeout_voted_power",
        "Counter for consensus participation status, 0 means no participation and 1 otherwise",
        &["peer_id"]
    )
    .unwrap()
});

/// Last vote seen for each of the peers
pub static CONSENSUS_LAST_VOTE_EPOCH: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "aptos_consensus_last_voted_epoch",
        "for each peer_id, last epoch we've seen consensus vote",
        &["peer_id"]
    )
    .unwrap()
});

/// Last vote seen for each of the peers
pub static CONSENSUS_LAST_VOTE_ROUND: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "aptos_consensus_last_voted_round",
        "for each peer_id, last round we've seen consensus vote",
        &["peer_id"]
    )
    .unwrap()
});

/// Last timeout vote seen for each of the peers
pub static CONSENSUS_LAST_TIMEOUT_VOTE_EPOCH: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "aptos_consensus_last_timeout_voted_epoch",
        "for each peer_id, last epoch we've seen consensus timeout vote",
        &["peer_id"]
    )
    .unwrap()
});

/// Last timeout vote seen for each of the peers
pub static CONSENSUS_LAST_TIMEOUT_VOTE_ROUND: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "aptos_consensus_last_timeout_voted_round",
        "for each peer_id, last round we've seen consensus timeout vote",
        &["peer_id"]
    )
    .unwrap()
});

//////////////////////
// RoundState COUNTERS
//////////////////////
/// This counter is set to the last round reported by the local round_state.
pub static CURRENT_ROUND: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "aptos_consensus_current_round",
        "Current consensus round"
    )
    .unwrap()
});

/// Count of the rounds that gathered QC since last restart.
pub static QC_ROUNDS_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "aptos_consensus_qc_rounds_count",
        "Count of the rounds that gathered QC since last restart."
    )
    .unwrap()
});

/// Count of the timeout rounds since last restart (close to 0 in happy path).
pub static TIMEOUT_ROUNDS_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "aptos_consensus_timeout_rounds_count",
        "Count of the timeout rounds since last restart (close to 0 in happy path)."
    )
    .unwrap()
});

/// Count the number of timeouts a node experienced since last restart (close to 0 in happy path).
/// This count is different from `TIMEOUT_ROUNDS_COUNT`, because not every time a node has
/// a timeout there is an ultimate decision to move to the next round (it might take multiple
/// timeouts to get the timeout certificate).
pub static TIMEOUT_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!("aptos_consensus_timeout_count", "Count the number of timeouts a node experienced since last restart (close to 0 in happy path).").unwrap()
});

/// Round timeout in milliseconds
pub static ROUND_TIMEOUT_MS: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "aptos_consensus_round_timeout_ms",
        "Round timeout in milliseconds"
    )
    .unwrap()
});

////////////////////////
/// ORDER VOTE COUNTERS
////////////////////////

pub static SUCCESSFUL_EXECUTED_WITH_ORDER_VOTE_QC: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "aptos_consensus_successful_executed_with_order_vote_qc",
        "Count of the number of blocks successfully executed with order vote QC"
    )
    .unwrap()
});

pub static LATE_EXECUTION_WITH_ORDER_VOTE_QC: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "aptos_consensus_late_execution_with_order_vote_qc",
        "Count of the number of blocks that were executed with order vote QC after the block was already ordered"
    )
    .unwrap()
});

// Created order certificate from order votes. But the block isn't available in the block store.
pub static ORDER_CERT_CREATED_WITHOUT_BLOCK_IN_BLOCK_STORE: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "aptos_consensus_order_cert_created_without_block_in_block_store",
        "Count of the number of order certificates created without the block being in the block store"
    )
    .unwrap()
});

pub static SUCCESSFUL_EXECUTED_WITH_REGULAR_QC: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "aptos_consensus_successful_executed_with_regular_qc",
        "Count of the number of blocks successfully executed with regular QC"
    )
    .unwrap()
});

pub static SYNC_TO_HIGHEST_QC: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "aptos_consensus_sync_to_highest_qc",
        "Count of the number of times we sync to highest QC"
    )
    .unwrap()
});

pub static ORDER_VOTE_ADDED: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "aptos_consensus_order_vote_added",
        "Count of the number of order votes added"
    )
    .unwrap()
});

pub static ORDER_VOTE_VERY_OLD: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "aptos_consensus_order_vote_very_old",
        "Count of the number of order votes that are very old"
    )
    .unwrap()
});

pub static ORDER_VOTE_OTHER_ERRORS: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "aptos_consensus_order_vote_other_errors",
        "Count of the number of order votes that have other errors"
    )
    .unwrap()
});

pub static ORDER_VOTE_BROADCASTED: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "aptos_consensus_order_vote_broadcasted",
        "Count of the number of order votes broadcasted"
    )
    .unwrap()
});

////////////////////////
// SYNC MANAGER COUNTERS
////////////////////////
/// Counts the number of times the sync info message has been set since last restart.
pub static SYNC_INFO_MSGS_SENT_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "aptos_consensus_sync_info_msgs_sent_count",
        "Number of sync info messages sent"
    )
    .unwrap()
});

/// Received sync info with a newer cert
pub static SYNC_INFO_RECEIVED_WITH_NEWER_CERT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "aptos_consensus_sync_info_received_with_newer_cert",
        "Received sync info with a newer cert"
    )
    .unwrap()
});

/// Number of blocks being fetched from the network in block retriever
pub static BLOCKS_FETCHED_FROM_NETWORK_IN_BLOCK_RETRIEVER: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "aptos_consensus_blocks_fetched_from_network_in_block_retriever",
        "Number of blocks being fetched from the network in block retriever"
    )
    .unwrap()
});

/// Number of blocks fetched from the network while inserting quorum cert
pub static BLOCKS_FETCHED_FROM_NETWORK_WHILE_INSERTING_QUORUM_CERT: Lazy<IntCounter> =
    Lazy::new(|| {
        register_int_counter!(
            "aptos_consensus_blocks_fetched_network_while_inserting_quorum_cert",
            "Number of blocks fetched from the network while inserting quorum cert"
        )
        .unwrap()
    });

/// Number of blocks fetched from the network while fast forward sync
pub static BLOCKS_FETCHED_FROM_NETWORK_WHILE_FAST_FORWARD_SYNC: Lazy<IntCounter> =
    Lazy::new(|| {
        register_int_counter!(
            "aptos_consensus_blocks_fetched_network_while_fast_forward_sync",
            "Number of blocks fetched from the network while fast forward sync"
        )
        .unwrap()
    });

//////////////////////
// RECONFIGURATION COUNTERS
//////////////////////
/// Current epoch num
pub static EPOCH: Lazy<IntGauge> =
    Lazy::new(|| register_int_gauge!("aptos_consensus_epoch", "Current epoch num").unwrap());

/// The number of validators in the current epoch
pub static CURRENT_EPOCH_VALIDATORS: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "aptos_consensus_current_epoch_validators",
        "The number of validators in the current epoch"
    )
    .unwrap()
});

//////////////////////
// BLOCK STORE COUNTERS
//////////////////////
/// Counter for the number of blocks in the block tree (including the root).
/// In a "happy path" with no collisions and timeouts, should be equal to 3 or 4.
pub static NUM_BLOCKS_IN_TREE: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "aptos_consensus_num_blocks_in_tree",
        "Counter for the number of blocks in the block tree (including the root)."
    )
    .unwrap()
});

/// Counter for the number of blocks in the pipeline broken down by stage.
pub static NUM_BLOCKS_IN_PIPELINE: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "aptos_consensus_num_blocks_in_pipeline",
        "Counter for the number of blocks in the pipeline",
        &["stage"]
    )
    .unwrap()
});

//////////////////////
// PERFORMANCE COUNTERS
//////////////////////
// TODO Consider reintroducing this counter
// pub static UNWRAPPED_PROPOSAL_SIZE_BYTES: Lazy<Histogram> = Lazy::new(|| {
//     register_histogram!(
//         "aptos_consensus_unwrapped_proposal_size_bytes",
//         "Histogram of proposal size after BCS but before wrapping with GRPC and aptos net."
//     )
//     .unwrap()
// });

const NUM_CONSENSUS_TRANSACTIONS_BUCKETS: [f64; 24] = [
    5.0, 10.0, 20.0, 40.0, 75.0, 100.0, 200.0, 400.0, 800.0, 1200.0, 1800.0, 2500.0, 3300.0,
    4000.0, 5000.0, 6500.0, 8000.0, 10000.0, 12500.0, 15000.0, 18000.0, 21000.0, 25000.0, 30000.0,
];

/// Histogram for the number of txns per (committed) blocks.
pub static NUM_TXNS_PER_BLOCK: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        "aptos_consensus_num_txns_per_block",
        "Histogram for the number of txns per (committed) blocks.",
        NUM_CONSENSUS_TRANSACTIONS_BUCKETS.to_vec()
    )
    .unwrap()
});

/// Histogram for the number of bytes in the committed blocks.
pub static NUM_BYTES_PER_BLOCK: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        "aptos_consensus_num_bytes_per_block",
        "Histogram for the number of bytes per (committed) blocks.",
        exponential_buckets(/*start=*/ 500.0, /*factor=*/ 1.4, /*count=*/ 32).unwrap()
    )
    .unwrap()
});

// Histogram buckets that expand DEFAULT_BUCKETS with more granularity:
// * 0.3 to 2.0: step 0.1
// * 2.0 to 4.0: step 0.2
// * 4.0 to 7.5: step 0.5
const BLOCK_TRACING_BUCKETS: &[f64] = &[
    0.005, 0.01, 0.025, 0.05, 0.1, 0.15, 0.2, 0.25, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0, 1.1,
    1.2, 1.3, 1.4, 1.5, 1.6, 1.7, 1.8, 1.9, 2.0, 2.2, 2.4, 2.6, 2.8, 3.0, 3.2, 3.4, 3.6, 3.8, 4.0,
    4.5, 5.0, 5.5, 6.0, 6.5, 7.0, 7.5, 10.0,
];

/// Traces block movement throughout the node
pub static BLOCK_TRACING: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "aptos_consensus_block_tracing",
        "Histogram for different stages of a block",
        &["stage"],
        BLOCK_TRACING_BUCKETS.to_vec()
    )
    .unwrap()
});

const CONSENSUS_WAIT_DURATION_BUCKETS: [f64; 19] = [
    0.005, 0.01, 0.015, 0.02, 0.04, 0.06, 0.08, 0.10, 0.125, 0.15, 0.175, 0.2, 0.225, 0.25, 0.3,
    0.4, 0.6, 0.8, 2.0,
];

/// Histogram of the time it requires to wait before inserting blocks into block store.
/// Measured as the block's timestamp minus local timestamp.
pub static WAIT_DURATION_S: Lazy<DurationHistogram> = Lazy::new(|| {
    DurationHistogram::new(register_histogram!("aptos_consensus_wait_duration_s",
    "Histogram of the time it requires to wait before inserting blocks into block store. Measured as the block's timestamp minus the local timestamp.",
    CONSENSUS_WAIT_DURATION_BUCKETS.to_vec()).unwrap())
});

/// Wait duration in milliseconds
pub static WAIT_DURATION_MS: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        "aptos_consensus_wait_duration_ms",
        "Wait duration in milliseconds",
        exponential_buckets(/*start=*/ 1.0, /*factor=*/ 2.0, /*count=*/ 30).unwrap(),
    )
    .unwrap()
});

const VERIFY_BUCKETS: &[f64] = &[
    0.0001, 0.00025, 0.0005, 0.001, 0.0015, 0.002, 0.0025, 0.003, 0.0035, 0.004, 0.005, 0.006,
    0.007, 0.008, 0.009, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0,
];

pub static VERIFY_MSG: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "aptos_consensus_verify_msg",
        "Histogram of the time it takes to verify a message",
        &["msg"],
        VERIFY_BUCKETS.to_vec()
    )
    .unwrap()
});

///////////////////
// CHANNEL COUNTERS
///////////////////
/// Count of the pending messages sent to itself in the channel
pub static PENDING_SELF_MESSAGES: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "aptos_consensus_pending_self_messages",
        "Count of the pending messages sent to itself in the channel"
    )
    .unwrap()
});

/// Count of the pending outbound round timeouts
pub static PENDING_ROUND_TIMEOUTS: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "aptos_consensus_pending_round_timeouts",
        "Count of the pending outbound round timeouts"
    )
    .unwrap()
});

/// Counter of pending network events to Consensus
pub static PENDING_CONSENSUS_NETWORK_EVENTS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_consensus_pending_network_events",
        "Counters(queued,dequeued,dropped) related to pending network notifications to Consensus",
        &["state"]
    )
    .unwrap()
});

/// Count of the pending state sync notification.
pub static PENDING_STATE_SYNC_NOTIFICATION: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "aptos_consensus_pending_state_sync_notification",
        "Count of the pending state sync notification"
    )
    .unwrap()
});

/// Count of the pending quorum store commit notification.
pub static PENDING_QUORUM_STORE_COMMIT_NOTIFICATION: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "aptos_consensus_pending_quorum_store_commit_notification",
        "Count of the pending quorum store commit notification"
    )
    .unwrap()
});

/// Counters related to pending commit votes
pub static BUFFER_MANAGER_MSGS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_consensus_buffer_manager_msgs_count",
        "Counters(queued,dequeued,dropped) related to pending commit votes",
        &["state"]
    )
    .unwrap()
});

/// Counters(queued,dequeued,dropped) related to consensus channel
pub static CONSENSUS_CHANNEL_MSGS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_consensus_channel_msgs_count",
        "Counters(queued,dequeued,dropped) related to consensus channel",
        &["state"]
    )
    .unwrap()
});

/// Counters(queued,dequeued,dropped) related to buffer manager channel
pub static BUFFER_MANAGER_CHANNEL_MSGS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_buffer_manager_channel_msgs_count",
        "Counters(queued,dequeued,dropped) related to buffer manager channel",
        &["state"]
    )
    .unwrap()
});

/// Counters for received consensus messages broken down by type
pub static CONSENSUS_RECEIVED_MSGS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_consensus_received_msgs_count",
        "Counters for received consensus messages broken down by type",
        &["type"]
    )
    .unwrap()
});

/// Counters for sent consensus messages broken down by type
pub static CONSENSUS_SENT_MSGS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_consensus_sent_msgs_count",
        "Counters for received consensus messages broken down by type",
        &["type"]
    )
    .unwrap()
});

/// Counters(queued,dequeued,dropped) related to consensus round manager channel
pub static ROUND_MANAGER_CHANNEL_MSGS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_consensus_round_manager_msgs_count",
        "Counters(queued,dequeued,dropped) related to consensus round manager channel",
        &["state"]
    )
    .unwrap()
});

/// Counters(queued,dequeued,dropped) related to quorum store channel
pub static QUORUM_STORE_CHANNEL_MSGS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_quorum_store_channel_msgs_count",
        "Counters(queued,dequeued,dropped) related to quorum store channel",
        &["state"]
    )
    .unwrap()
});

/// Counters(queued,dequeued,dropped) related to rpc request channel
pub static RPC_CHANNEL_MSGS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_consensus_rpc_channel_msgs_count",
        "Counters(queued,dequeued,dropped) related to rpc request channel",
        &["state"]
    )
    .unwrap()
});

/// Counters(queued,dequeued,dropped) related to block retrieval per epoch task
pub static BLOCK_RETRIEVAL_TASK_MSGS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_consensus_block_retrieval_task_msgs_count",
        "Counters(queued,dequeued,dropped) related to block retrieval task",
        &["state"]
    )
    .unwrap()
});

pub static BLOCK_RETRIEVAL_LOCAL_FULFILL_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "aptos_consensus_block_retrieval_local_fulfill_count",
        "Count of the number of local fulfillments of block retrieval requests"
    )
    .unwrap()
});

/// Count of the buffer manager retry requests since last restart.
pub static BUFFER_MANAGER_RETRY_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "aptos_consensus_buffer_manager_retry_count",
        "Count of the buffer manager retry requests since last restart"
    )
    .unwrap()
});

const PROPSER_ELECTION_DURATION_BUCKETS: [f64; 17] = [
    0.001, 0.002, 0.003, 0.004, 0.006, 0.008, 0.01, 0.012, 0.014, 0.0175, 0.02, 0.025, 0.05, 0.25,
    0.5, 1.0, 2.0,
];

/// Time it takes for proposer election to compute proposer (when not cached)
pub static PROPOSER_ELECTION_DURATION: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        "aptos_consensus_proposer_election_duration",
        "Time it takes for proposer election to compute proposer (when not cached)",
        PROPSER_ELECTION_DURATION_BUCKETS.to_vec()
    )
    .unwrap()
});

/// Count of the number of blocks that have ready batches to execute.
pub static QUORUM_BATCH_READY_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "aptos_consensus_quorum_store_batch_ready_count",
        "Count of the number of blocks that have ready batches to execute"
    )
    .unwrap()
});

/// Histogram of the time durations waiting for batch when executing.
pub static BATCH_WAIT_DURATION: Lazy<DurationHistogram> = Lazy::new(|| {
    DurationHistogram::new(
        register_histogram!(
            "aptos_consensus_batch_wait_duration",
            "Histogram of the time durations for waiting batches.",
            // exponential_buckets(/*start=*/ 100.0, /*factor=*/ 1.1, /*count=*/ 100).unwrap(),
        )
        .unwrap(),
    )
});

/// Histogram of timers for each of the buffer manager phase processors.
pub static BUFFER_MANAGER_PHASE_PROCESS_SECONDS: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        // metric name
        "aptos_consensus_buffer_manager_phase_process_seconds",
        // metric description
        "Timer for buffer manager PipelinePhase::process()",
        // metric labels (dimensions)
        &["name"],
        exponential_buckets(/*start=*/ 1e-6, /*factor=*/ 2.0, /*count=*/ 22).unwrap(),
    )
    .unwrap()
});

/// Count of the number of `ProposalExt` blocks received while the feature is disabled.
pub static UNEXPECTED_PROPOSAL_EXT_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "aptos_consensus_unexpected_proposal_ext_count",
        "Count of the number of `ProposalExt` blocks received while the feature is disabled."
    )
    .unwrap()
});

/// Histogram for the number of txns to be executed in a block.
pub static MAX_TXNS_FROM_BLOCK_TO_EXECUTE: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        "max_txns_from_block_to_execute",
        "Histogram for the number of txns to be executed in a block.",
        exponential_buckets(/*start=*/ 1.5, /*factor=*/ 1.5, /*count=*/ 25).unwrap(),
    )
    .unwrap()
});

/// Count of the number of `DKG` validator transactions received while the feature is disabled.
pub static UNEXPECTED_DKG_VTXN_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "aptos_consensus_unexpected_dkg_vtxn_count",
        "Count of the number of `DKG` validator transactions received while the feature is disabled."
    )
        .unwrap()
});

/// Histogram of the time durations for fetching commit history.
pub static FETCH_COMMIT_HISTORY_DURATION: Lazy<DurationHistogram> = Lazy::new(|| {
    DurationHistogram::new(
        register_histogram!(
            "aptos_consensus_fetch_commit_history_duration",
            "Histogram of the time durations for fetching commit history.",
            // exponential_buckets(/*start=*/ 100.0, /*factor=*/ 1.1, /*count=*/ 100).unwrap(),
        )
        .unwrap(),
    )
});

/// Update various counters for committed blocks
pub fn update_counters_for_committed_blocks(blocks_to_commit: &[Arc<PipelinedBlock>]) {
    for block in blocks_to_commit {
        observe_block(block.block().timestamp_usecs(), BlockStage::COMMITTED);
        let txn_status = block.compute_result().compute_status_for_input_txns();
        NUM_TXNS_PER_BLOCK.observe(txn_status.len() as f64);
        NUM_BYTES_PER_BLOCK
            .observe(block.block().payload().map_or(0, |payload| payload.size()) as f64);
        COMMITTED_BLOCKS_COUNT.inc();
        LAST_COMMITTED_ROUND.set(block.round() as i64);
        LAST_COMMITTED_VERSION.set(block.compute_result().num_leaves() as i64);

        let failed_rounds = block
            .block()
            .block_data()
            .failed_authors()
            .map(|v| v.len())
            .unwrap_or(0);
        if failed_rounds > 0 {
            COMMITTED_FAILED_ROUNDS_COUNT.inc_by(failed_rounds as u64);
        }

        // Quorum store metrics
        quorum_store::counters::NUM_BATCH_PER_BLOCK.observe(block.block().payload_size() as f64);

        for status in txn_status.iter() {
            let commit_status = match status {
                TransactionStatus::Keep(_) => TXN_COMMIT_SUCCESS_LABEL,
                TransactionStatus::Discard(reason) => {
                    if *reason == DiscardedVMStatus::SEQUENCE_NUMBER_TOO_NEW {
                        TXN_COMMIT_RETRY_LABEL
                    } else if *reason == DiscardedVMStatus::SEQUENCE_NUMBER_TOO_OLD {
                        TXN_COMMIT_FAILED_DUPLICATE_LABEL
                    } else {
                        TXN_COMMIT_FAILED_LABEL
                    }
                },
                TransactionStatus::Retry => TXN_COMMIT_RETRY_LABEL,
            };
            COMMITTED_TXNS_COUNT
                .with_label_values(&[commit_status])
                .inc();
        }
    }
}

pub static EPOCH_MANAGER_ISSUES_DETAILS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_consensus_epoch_manager_issues",
        "Count of occurences of different epoch manager processing issues.",
        &["kind"]
    )
    .unwrap()
});

pub static PROPOSED_VTXN_COUNT: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_proposed_vtxn_count",
        "Number of validator transactions proposed",
        &["proposer"]
    )
    .unwrap()
});

pub static PROPOSED_VTXN_BYTES: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_proposed_vtxn_bytes",
        "The total size in bytes of validator transactions proposed",
        &["proposer"]
    )
    .unwrap()
});

pub static RAND_QUEUE_SIZE: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "aptos_consensus_rand_queue_size",
        "Number of randomness-pending blocks."
    )
    .unwrap()
});
