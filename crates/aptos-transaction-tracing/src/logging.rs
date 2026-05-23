// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Structured log schema for the transaction-tracing crate.
//!
//! Modeled after `consensus::logging::LogSchema`. Each `LogEvent` represents
//! a distinct emission site; structured fields are added with the builder
//! methods produced by `#[derive(Schema)]` from `aptos_logger`.
//!
//! The primary event is `TxnTrace`, emitted by `store::log_trace` when a
//! traced transaction reaches a terminal state. Per-stage latencies are
//! recorded as scalars for the **first pipeline pass** (attempt 1) to keep
//! Humio/Grafana queries simple — no `[N]` indexing needed. Retried
//! transactions' later attempts are visible in the `stages` text string.
//! `attempts > 1` is the signal for "this txn retried; check `stages` for
//! per-attempt detail."

use aptos_crypto::HashValue;
use aptos_logger::Schema;
use aptos_types::account_address::AccountAddress;
use serde::Serialize;

#[derive(Schema)]
pub struct LogSchema {
    event: LogEvent,

    // Identity
    hash: Option<HashValue>,
    sender: Option<AccountAddress>,

    // Top-level summary
    gas_unit_price: Option<u64>,
    attempts: Option<u32>,
    total_latency_ms: Option<i64>,
    outcome: Option<&'static str>,
    age_ms: Option<u64>,
    num_stages: Option<usize>,

    // GC summary counters (only used by `TxnTraceGcSummary`)
    evicted_traces: Option<u64>,
    evicted_batch_mappings: Option<usize>,
    evicted_block_mappings: Option<usize>,
    remaining_traces: Option<usize>,
    remaining_batch_mappings: Option<usize>,
    remaining_block_mappings: Option<usize>,

    // Per-stage absolute latency from MempoolInsert (ms) for the first
    // pipeline pass (attempt 1). Each field is a scalar — directly queryable
    // in Humio/Grafana as `data.<field>` with no array indexing required.
    //
    // For a single-attempt commit (the common case), these reflect the full
    // pipeline end-to-end. For a retried trace (attempts > 1), they reflect
    // only attempt 1's path (including the retried block's
    // PreCommit/Certified/Committed); look at `stages` for per-attempt detail.
    //
    // `block_proposed_ms` can be negative: it records the proposer's
    // `block.timestamp_usecs`, which is on a different validator's clock and
    // can precede this node's MempoolInsert. Every other stage uses the
    // local clock and is non-negative.
    mempool_insert_ms: Option<i64>,
    qs_batch_pull_ms: Option<i64>,
    qs_batch_created_ms: Option<i64>,
    qs_proof_of_store_ms: Option<i64>,
    parent_block_proposed_ms: Option<i64>,
    block_proposed_ms: Option<i64>,
    block_proposed_kind: Option<String>,
    block_received_ms: Option<i64>,
    execution_start_ms: Option<i64>,
    executed_ms: Option<i64>,
    executed_status: Option<String>,
    block_ordered_ms: Option<i64>,
    certified_ms: Option<i64>,
    pre_commit_ms: Option<i64>,
    committed_ms: Option<i64>,
    mempool_commit_ms: Option<i64>,
    mempool_reject_ms: Option<i64>,

    // Full diagnostic string with all attempts and metadata. Preserves
    // `wait(...)`, batch-pull `n=/max=/excl=/bp=` info, and per-attempt
    // markers — the place to look for retry-path detail.
    stages: Option<String>,
}

#[derive(Serialize)]
pub enum LogEvent {
    /// Emitted by `store::log_trace` when a traced transaction reaches a
    /// terminal stage (committed, rejected, discarded, or retry_incomplete).
    TxnTrace,
    /// Emitted by `store::gc` when an orphaned trace is evicted (TTL exceeded
    /// without reaching a terminal stage).
    TxnTraceEvicted,
    /// Emitted by `store::gc` once per sweep summarizing eviction counts.
    TxnTraceGcSummary,
}

impl LogSchema {
    pub fn new(event: LogEvent) -> Self {
        Self {
            event,
            hash: None,
            sender: None,
            gas_unit_price: None,
            attempts: None,
            total_latency_ms: None,
            outcome: None,
            age_ms: None,
            num_stages: None,
            evicted_traces: None,
            evicted_batch_mappings: None,
            evicted_block_mappings: None,
            remaining_traces: None,
            remaining_batch_mappings: None,
            remaining_block_mappings: None,
            mempool_insert_ms: None,
            qs_batch_pull_ms: None,
            qs_batch_created_ms: None,
            qs_proof_of_store_ms: None,
            parent_block_proposed_ms: None,
            block_proposed_ms: None,
            block_proposed_kind: None,
            block_received_ms: None,
            execution_start_ms: None,
            executed_ms: None,
            executed_status: None,
            block_ordered_ms: None,
            certified_ms: None,
            pre_commit_ms: None,
            committed_ms: None,
            mempool_commit_ms: None,
            mempool_reject_ms: None,
            stages: None,
        }
    }
}
