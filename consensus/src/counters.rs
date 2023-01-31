// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_metrics_core::{
    op_counters::DurationHistogram, register_counter, register_gauge, register_histogram,
    register_histogram_vec, register_int_counter, register_int_counter_vec, register_int_gauge,
    register_int_gauge_vec, Counter, Gauge, Histogram, HistogramVec, IntCounter, IntCounterVec,
    IntGauge, IntGaugeVec,
};
use once_cell::sync::Lazy;

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

/// Number of rounds we were collecting votes for proposer
/// (similar to PROPOSALS_COUNT, but can be larger, if we failed in creating/sending of the proposal)
pub static CHAIN_HEALTH_BACKOFF_TRIGGERED: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "aptos_chain_health_backoff_triggered",
        "Total voting power of all votes collected for the round this node was proposer",
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

//////////////////////
// RoundState COUNTERS
//////////////////////
/// This counter is set to the last round reported by the local round_state.
pub static CURRENT_ROUND: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "aptos_consensus_current_round",
        "This counter is set to the last round reported by the local round_state."
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

/// The timeout of the current round.
pub static ROUND_TIMEOUT_MS: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "aptos_consensus_round_timeout_s",
        "The timeout of the current round."
    )
    .unwrap()
});

////////////////////////
// SYNC MANAGER COUNTERS
////////////////////////
/// Counts the number of times the sync info message has been set since last restart.
pub static SYNC_INFO_MSGS_SENT_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "aptos_consensus_sync_info_msg_sent_count",
        "Counts the number of times the sync info message has been set since last restart."
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

/// Histogram for the number of txns per (committed) blocks.
pub static NUM_TXNS_PER_BLOCK: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        "aptos_consensus_num_txns_per_block",
        "Histogram for the number of txns per (committed) blocks."
    )
    .unwrap()
});

/// Traces block movement throughout the node
pub static BLOCK_TRACING: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "aptos_consensus_block_tracing",
        "Histogram for different stages of a block",
        &["stage"]
    )
    .unwrap()
});

/// Histogram of the time it requires to wait before inserting blocks into block store.
/// Measured as the block's timestamp minus local timestamp.
pub static WAIT_DURATION_S: Lazy<DurationHistogram> = Lazy::new(|| {
    DurationHistogram::new(register_histogram!("aptos_consensus_wait_duration_s", "Histogram of the time it requires to wait before inserting blocks into block store. Measured as the block's timestamp minus the local timestamp.").unwrap())
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

/// Counters(queued,dequeued,dropped) related to consensus channel
pub static ROUND_MANAGER_CHANNEL_MSGS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_consensus_round_manager_msgs_count",
        "Counters(queued,dequeued,dropped) related to consensus channel",
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

/// Counters(queued,dequeued,dropped) related to block retrieval channel
pub static BLOCK_RETRIEVAL_CHANNEL_MSGS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_consensus_block_retrieval_channel_msgs_count",
        "Counters(queued,dequeued,dropped) related to block retrieval channel",
        &["state"]
    )
    .unwrap()
});

/// Counters(queued,dequeued,dropped) related to block retrieval task
pub static BLOCK_RETRIEVAL_TASK_MSGS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_consensus_block_retrieval_task_msgs_count",
        "Counters(queued,dequeued,dropped) related to block retrieval task",
        &["state"]
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

/// Time it takes for proposer election to compute proposer (when not cached)
pub static PROPOSER_ELECTION_DURATION: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        "aptos_consensus_proposer_election_duration",
        "Time it takes for proposer election to compute proposer (when not cached)"
    )
    .unwrap()
});
