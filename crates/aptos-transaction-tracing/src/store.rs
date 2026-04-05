// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    counters::observe_stage_latency,
    filter::TransactionFilter,
    types::{ExecutionStatus, StageMetadata, TransactionStage, TransactionTrace},
};
use aptos_crypto::HashValue;
use aptos_logger::{info, warn};
use aptos_types::account_address::AccountAddress;
use arc_swap::ArcSwap;
use dashmap::DashMap;
use once_cell::sync::Lazy;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};

static GLOBAL_STORE: Lazy<TransactionTraceStore> = Lazy::new(TransactionTraceStore::new);

/// How often `finalize_trace()` triggers a GC sweep (30 seconds).
const GC_INTERVAL_USECS: u64 = 30_000_000;

/// Traces older than this TTL are considered orphaned and evicted (60 seconds).
const GC_TTL_USECS: u64 = 60_000_000;

/// Upper bound on concurrent traces. Prevents unbounded memory growth under
/// high sampling rates or misconfigured allowlists. At ~1KB per trace, 10K
/// traces ≈ 10MB — well within safe limits.
const MAX_TRACES: usize = 10_000;

/// Global store for transaction lifecycle traces.
///
/// Thread-safe via DashMap. Only traces transactions whose sender is in the
/// allowlist (checked at mempool insertion time via `maybe_start_trace`).
pub struct TransactionTraceStore {
    /// Active traces keyed by transaction hash.
    traces: DashMap<HashValue, TransactionTrace>,
    /// Batch digest → traced txn hashes only. Non-traced batches are not registered,
    /// making `record_batch_stage()` a DashMap miss (no-op) for them.
    batch_txns: DashMap<HashValue, Vec<HashValue>>,
    /// Block ID → traced txn hashes. Populated at BlockProposed time so that
    /// later stages (Ordered, Certified, PreCommit, Committed) can record
    /// stages with a single O(1) lookup instead of re-iterating the payload.
    block_txns: DashMap<HashValue, Vec<HashValue>>,
    /// Filter controlling which senders to trace. Updated via ArcSwap for
    /// lock-free reads and atomic swaps (e.g., from admin API).
    filter: ArcSwap<TransactionFilter>,
    /// Timestamp (usecs) of the last GC sweep. Used by `finalize_trace()` to
    /// throttle periodic GC to once per `GC_INTERVAL_USECS`.
    last_gc_usecs: AtomicU64,
}

impl TransactionTraceStore {
    fn new() -> Self {
        Self {
            traces: DashMap::new(),
            batch_txns: DashMap::new(),
            block_txns: DashMap::new(),
            filter: ArcSwap::new(Arc::new(TransactionFilter::default())),
            last_gc_usecs: AtomicU64::new(0),
        }
    }

    /// Returns the global singleton store.
    pub fn global() -> &'static Self {
        &GLOBAL_STORE
    }

    /// Fast check: is tracing active (enabled with non-empty allowlist)?
    /// Uses a single lock-free ArcSwap load (~ns). Use this as a gate before
    /// doing any per-txn tracing work in hot paths.
    pub fn is_enabled(&self) -> bool {
        self.filter.load().is_active()
    }

    /// Returns true if this QS pull round should do tracing work.
    /// Batch-level sampling: only `batch_sample_rate` fraction of rounds proceed.
    /// 90% of rounds return false (~5ns), skipping all per-txn work.
    pub fn should_sample_batch(&self, pull_round: u64) -> bool {
        self.filter.load().should_sample_batch(pull_round)
    }

    /// Called at mempool insertion. Checks if sender is in allowlist + txn sampling,
    /// creates trace if matched. Returns true if trace was started.
    pub fn maybe_start_trace(
        &self,
        hash: HashValue,
        sender: AccountAddress,
        now_usecs: u64,
    ) -> bool {
        let filter = self.filter.load();
        if !filter.should_trace(&sender, &hash) {
            return false;
        }
        // Cap concurrent traces to prevent unbounded memory growth.
        if self.traces.len() >= MAX_TRACES {
            return false;
        }
        let mut trace = TransactionTrace::new(hash, sender, now_usecs);
        trace.record(TransactionStage::MempoolInsert, now_usecs);
        observe_stage_latency(
            now_usecs,
            &trace.sender_str,
            TransactionStage::MempoolInsert.as_ref(),
        );
        self.traces.insert(hash, trace);
        true
    }

    /// Record a stage for a traced transaction using the local clock.
    pub fn record_stage(&self, hash: &HashValue, stage: TransactionStage) {
        self.record_stage_at(hash, stage, now_usecs());
    }

    /// Record a stage with an explicit timestamp (e.g., block.timestamp_usecs).
    pub fn record_stage_at(&self, hash: &HashValue, stage: TransactionStage, timestamp_usecs: u64) {
        if let Some(mut trace) = self.traces.get_mut(hash) {
            observe_stage_latency(
                trace.insertion_time_usecs,
                &trace.sender_str,
                stage.as_ref(),
            );
            trace.record(stage, timestamp_usecs);
        }
    }

    /// Record a stage with metadata for a traced transaction.
    pub fn record_stage_with_metadata(
        &self,
        hash: &HashValue,
        stage: TransactionStage,
        metadata: StageMetadata,
    ) {
        self.record_stage_with_metadata_at(hash, stage, metadata, now_usecs());
    }

    /// Record a stage with metadata and explicit timestamp.
    pub fn record_stage_with_metadata_at(
        &self,
        hash: &HashValue,
        stage: TransactionStage,
        metadata: StageMetadata,
        timestamp_usecs: u64,
    ) {
        if let Some(mut trace) = self.traces.get_mut(hash) {
            let stage_label = match &metadata {
                StageMetadata::Execution(status) => {
                    format!("{}({})", stage.as_ref(), status.as_ref())
                },
                StageMetadata::BatchInclusion(inclusion) => {
                    format!("{}({})", stage.as_ref(), inclusion.as_ref())
                },
                StageMetadata::BatchPull(_) => stage.as_ref().to_string(),
            };
            observe_stage_latency(trace.insertion_time_usecs, &trace.sender_str, &stage_label);
            trace.record_with_metadata(stage, timestamp_usecs, metadata);
        }
    }

    /// Record a stage for all traced txns in a batch (by batch digest).
    pub fn record_batch_stage(&self, batch_digest: &HashValue, stage: TransactionStage) {
        self.record_batch_stage_impl(batch_digest, stage, None, now_usecs());
    }

    /// Record a stage with explicit timestamp for all traced txns in a batch.
    pub fn record_batch_stage_at(
        &self,
        batch_digest: &HashValue,
        stage: TransactionStage,
        timestamp_usecs: u64,
    ) {
        self.record_batch_stage_impl(batch_digest, stage, None, timestamp_usecs);
    }

    /// Record a stage with metadata for all traced txns in a batch.
    pub fn record_batch_stage_with_metadata(
        &self,
        batch_digest: &HashValue,
        stage: TransactionStage,
        metadata: StageMetadata,
    ) {
        self.record_batch_stage_impl(batch_digest, stage, Some(metadata), now_usecs());
    }

    /// Record a stage with metadata and explicit timestamp for all traced txns in a batch.
    pub fn record_batch_stage_with_metadata_at(
        &self,
        batch_digest: &HashValue,
        stage: TransactionStage,
        metadata: StageMetadata,
        timestamp_usecs: u64,
    ) {
        self.record_batch_stage_impl(batch_digest, stage, Some(metadata), timestamp_usecs);
    }

    fn record_batch_stage_impl(
        &self,
        batch_digest: &HashValue,
        stage: TransactionStage,
        metadata: Option<StageMetadata>,
        timestamp_usecs: u64,
    ) {
        // Clone hashes and release the batch_txns shard lock before acquiring
        // per-txn trace locks, reducing cross-map lock hold time.
        let txn_hashes: Option<Vec<HashValue>> =
            self.batch_txns.get(batch_digest).map(|r| r.value().clone());
        if let Some(hashes) = txn_hashes {
            for hash in &hashes {
                match &metadata {
                    Some(meta) => self.record_stage_with_metadata_at(
                        hash,
                        stage,
                        meta.clone(),
                        timestamp_usecs,
                    ),
                    None => self.record_stage_at(hash, stage, timestamp_usecs),
                }
            }
        }
    }

    /// Register batch_digest → traced txn hashes mapping.
    /// Filters txn_hashes to only those with active traces. If none are traced,
    /// skips registration entirely.
    pub fn register_batch(&self, batch_digest: HashValue, txn_hashes: &[HashValue]) {
        let traced: Vec<HashValue> = txn_hashes
            .iter()
            .filter(|h| self.traces.contains_key(*h))
            .copied()
            .collect();
        if !traced.is_empty() {
            self.batch_txns.insert(batch_digest, traced);
        }
    }

    /// Get traced txn hashes for a batch digest. Returns None if the batch
    /// has no traced txns.
    pub fn get_batch_traced_txns(&self, batch_digest: &HashValue) -> Option<Vec<HashValue>> {
        self.batch_txns.get(batch_digest).map(|v| v.clone())
    }

    /// Register block_id → traced txn hashes. Called at BlockProposed time
    /// after iterating the payload to find traced batch digests.
    /// Later stages use `record_block_stage` for O(1) lookup.
    pub fn register_block(&self, block_id: HashValue, traced_txn_hashes: Vec<HashValue>) {
        if !traced_txn_hashes.is_empty() {
            self.block_txns.insert(block_id, traced_txn_hashes);
        }
    }

    /// Record a stage for all traced txns in a block. O(1) DashMap lookup
    /// instead of re-iterating the block payload.
    /// Record a stage for all traced txns in a block. Does NOT trigger GC
    /// to avoid adding latency to the execution/consensus hot path.
    /// GC runs from finalize_trace (mempool commit) instead.
    pub fn record_block_stage(&self, block_id: &HashValue, stage: TransactionStage) {
        self.record_block_stage_at(block_id, stage, now_usecs());
    }

    /// Record a stage for all traced txns in a block with explicit timestamp.
    pub fn record_block_stage_at(
        &self,
        block_id: &HashValue,
        stage: TransactionStage,
        timestamp_usecs: u64,
    ) {
        // Clone hashes and release the block_txns shard lock before acquiring
        // per-txn trace locks, matching the pattern in record_batch_stage_impl.
        let txn_hashes: Option<Vec<HashValue>> =
            self.block_txns.get(block_id).map(|r| r.value().clone());
        if let Some(hashes) = txn_hashes {
            for hash in &hashes {
                self.record_stage_at(hash, stage, timestamp_usecs);
            }
        }
    }

    /// Get traced txn hashes for a block (cloned to avoid holding lock).
    pub fn get_block_traced_txns(&self, block_id: &HashValue) -> Option<Vec<HashValue>> {
        self.block_txns.get(block_id).map(|r| r.value().clone())
    }

    /// Process a proposed block: record BlockProposed per batch and register
    /// block → traced txn hashes for efficient post-proposal stage recording.
    /// Called from round_manager::process_proposal.
    pub fn process_proposed_block(
        &self,
        block_id: HashValue,
        block_timestamp_usecs: u64,
        batch_digests: &[(HashValue, crate::types::BatchInclusionType)],
    ) {
        let mut block_traced_txns: Vec<HashValue> = Vec::new();
        for (digest, inclusion) in batch_digests {
            self.record_batch_stage_with_metadata_at(
                digest,
                TransactionStage::BlockProposed,
                StageMetadata::BatchInclusion(*inclusion),
                block_timestamp_usecs,
            );
            if let Some(txn_hashes) = self.get_batch_traced_txns(digest) {
                block_traced_txns.extend(txn_hashes.iter());
            }
        }
        self.register_block(block_id, block_traced_txns);
    }

    /// Record execution results (Keep/Retry/Discard) for traced txns in a block.
    /// Called from block_executor::execute_and_update_state after execution.
    pub fn record_execution_result(
        &self,
        block_id: &HashValue,
        retry_hashes: &[HashValue],
        discard_hashes: &[HashValue],
    ) {
        if let Some(traced_hashes) = self.get_block_traced_txns(block_id) {
            use crate::types::{ExecutionStatus, StageMetadata};
            let now = now_usecs();

            let retry_set: std::collections::HashSet<HashValue> =
                retry_hashes.iter().copied().collect();
            let discard_set: std::collections::HashSet<HashValue> =
                discard_hashes.iter().copied().collect();

            for hash in &traced_hashes {
                if retry_set.contains(hash) {
                    self.record_stage_with_metadata_at(
                        hash,
                        TransactionStage::Executed,
                        StageMetadata::Execution(ExecutionStatus::Retry),
                        now,
                    );
                    self.mark_retry(hash);
                } else if discard_set.contains(hash) {
                    self.record_stage_with_metadata_at(
                        hash,
                        TransactionStage::Executed,
                        StageMetadata::Execution(ExecutionStatus::Discard),
                        now,
                    );
                } else {
                    self.record_stage_with_metadata_at(
                        hash,
                        TransactionStage::Executed,
                        StageMetadata::Execution(ExecutionStatus::Keep),
                        now,
                    );
                }
            }
        }
    }

    /// Check if a transaction hash has an active trace.
    pub fn is_traced(&self, hash: &HashValue) -> bool {
        self.traces.contains_key(hash)
    }

    /// Record the gas unit price for a traced transaction (set once at first pull).
    pub fn set_gas_unit_price(&self, hash: &HashValue, gas_unit_price: u64) {
        if let Some(mut trace) = self.traces.get_mut(hash) {
            if trace.gas_unit_price.is_none() {
                trace.gas_unit_price = Some(gas_unit_price);
            }
        }
    }

    /// Mark a transaction for retry: increment its attempt counter.
    pub fn mark_retry(&self, hash: &HashValue) {
        if let Some(mut trace) = self.traces.get_mut(hash) {
            trace.current_attempt += 1;
        }
    }

    /// Finalize and log the completed trace. Removes from store.
    /// Also triggers periodic GC to clean up orphaned traces and stale batch mappings.
    pub fn finalize_trace(&self, hash: &HashValue) {
        if let Some((_, trace)) = self.traces.remove(hash) {
            log_trace(&trace);
        }
        self.maybe_gc();
    }

    /// Run GC if at least `GC_INTERVAL_USECS` have elapsed since the last sweep.
    /// Uses compare-exchange so only one thread runs GC when multiple call concurrently.
    fn maybe_gc(&self) {
        let now = now_usecs();
        let last = self.last_gc_usecs.load(Ordering::Relaxed);
        if now.saturating_sub(last) >= GC_INTERVAL_USECS
            && self
                .last_gc_usecs
                .compare_exchange(last, now, Ordering::AcqRel, Ordering::Relaxed)
                .is_ok()
        {
            self.gc(GC_TTL_USECS);
        }
    }

    /// Query a trace by hash (returns clone).
    pub fn get_trace(&self, hash: &HashValue) -> Option<TransactionTrace> {
        self.traces.get(hash).map(|t| t.clone())
    }

    /// Get all active traces.
    pub fn get_all_traces(&self) -> Vec<(HashValue, TransactionTrace)> {
        self.traces
            .iter()
            .map(|entry| (*entry.key(), entry.value().clone()))
            .collect()
    }

    /// Update the filter at runtime (e.g., from admin API).
    pub fn update_filter(&self, filter: TransactionFilter) {
        self.filter.store(Arc::new(filter));
    }

    /// Get the current filter (for admin API GET).
    pub fn get_filter(&self) -> Arc<TransactionFilter> {
        self.filter.load_full()
    }

    /// Cleanup traces older than TTL and stale batch mappings.
    /// Orphaned traces are logged before eviction so operators can see incomplete pipelines.
    pub fn gc(&self, ttl_usecs: u64) {
        let cutoff = now_usecs().saturating_sub(ttl_usecs);
        let mut evicted = 0u64;
        self.traces.retain(|_, trace| {
            if trace.insertion_time_usecs > cutoff {
                return true;
            }
            // Log the orphaned trace before evicting so partial pipeline data is visible.
            warn!(
                "TxnTrace GC evicting orphaned trace: hash=0x{} sender={} age_ms={} stages={}",
                trace.hash.to_hex(),
                trace.sender,
                now_usecs().saturating_sub(trace.insertion_time_usecs) / 1000,
                trace.stages.len(),
            );
            evicted += 1;
            false
        });
        // Clean up batch mappings that no longer reference any active traces.
        // Two-pass approach avoids holding batch_txns shard locks while probing
        // traces, which would invert the lock order used in record_batch_stage_impl.
        //
        // Pass 1: snapshot which batch digests have zero live txn hashes.
        let stale_batches: Vec<HashValue> = self
            .batch_txns
            .iter()
            .filter(|entry| entry.value().iter().all(|h| !self.traces.contains_key(h)))
            .map(|entry| *entry.key())
            .collect();
        // Pass 2: remove fully-stale batches (no shard lock held on traces).
        let batch_evicted = stale_batches.len();
        for digest in &stale_batches {
            self.batch_txns.remove(digest);
        }
        // Clean up block mappings with no live traces.
        let stale_blocks: Vec<HashValue> = self
            .block_txns
            .iter()
            .filter(|entry| entry.value().iter().all(|h| !self.traces.contains_key(h)))
            .map(|entry| *entry.key())
            .collect();
        let block_evicted = stale_blocks.len();
        for id in &stale_blocks {
            self.block_txns.remove(id);
        }
        if evicted > 0 || batch_evicted > 0 || block_evicted > 0 {
            info!(
                "TxnTrace GC: evicted {} orphaned traces, {} stale batch mappings, {} stale block mappings. \
                 Remaining: {} traces, {} batch mappings, {} block mappings.",
                evicted,
                batch_evicted,
                block_evicted,
                self.traces.len(),
                self.batch_txns.len(),
                self.block_txns.len(),
            );
        }
    }
}

fn now_usecs() -> u64 {
    aptos_infallible::duration_since_epoch().as_micros() as u64
}

/// Block-finalization stages that should stay grouped with the preceding
/// Executed(Retry) rather than starting a new display-attempt.
fn is_block_finalization(stage: TransactionStage) -> bool {
    matches!(
        stage,
        TransactionStage::BlockOrdered
            | TransactionStage::Certified
            | TransactionStage::PreCommit
            | TransactionStage::Committed
    )
}

/// Build a `wait(...)` summary for the gap between the previous stage and
/// the first QsBatchPull of a new attempt.
///
/// Diagnoses why a txn waited: shows pull round count, interval percentiles,
/// back-pressure breakdown, pull capacity utilization, and gas bucket distribution.
fn build_wait_summary(
    info: &crate::types::BatchPullInfo,
    prev_stage_usecs: u64,
    pull_time_usecs: u64,
) -> String {
    use crate::types::BatchCreationRecord;
    use std::collections::BTreeMap;

    // Filter batch records to the half-open window (prev_stage, pull_time).
    let gap_batches: Vec<&BatchCreationRecord> = info
        .recent_batches
        .iter()
        .filter(|r| r.timestamp_usecs > prev_stage_usecs && r.timestamp_usecs < pull_time_usecs)
        .collect();

    let mut parts = Vec::new();

    // Always show gap duration (ms between previous stage and this pull).
    let gap_ms = (pull_time_usecs as i64 - prev_stage_usecs as i64) / 1000;
    parts.push(format!("{}ms", gap_ms));

    // When gap is too short for batch records (common after retry), show
    // snapshot context from this pull round so retries aren't opaque.
    if gap_batches.is_empty() {
        parts.push(format!(
            "excl={},bp={}/{}",
            info.excluded_txn_count,
            if info.bp_txn { 1 } else { 0 },
            if info.bp_proof { 1 } else { 0 },
        ));
    }

    if !gap_batches.is_empty() {
        let n = gap_batches.len();
        let total_batch_objects: u64 = gap_batches.iter().map(|r| r.num_batches).sum();
        parts.push(format!("rounds={},batches={}", n, total_batch_objects));

        // Interval percentiles (p50/p70/p90) between consecutive pull rounds.
        if n >= 2 {
            let mut intervals_ms: Vec<i64> = gap_batches
                .windows(2)
                .map(|w| (w[1].timestamp_usecs as i64 - w[0].timestamp_usecs as i64) / 1000)
                .collect();
            intervals_ms.sort_unstable();
            let pct = |p: f64| {
                let idx = ((intervals_ms.len() as f64 - 1.0) * p / 100.0).round() as usize;
                intervals_ms[idx.min(intervals_ms.len() - 1)]
            };
            parts.push(format!(
                "interval=p50:{}ms/p70:{}ms/p90:{}ms",
                pct(50.0),
                pct(70.0),
                pct(90.0),
            ));
        }

        // pulled_full: how many rounds pulled at max capacity (pull limit was the bottleneck).
        let pulled_full = gap_batches
            .iter()
            .filter(|r| r.pulled_txn_count >= r.pull_max_txn)
            .count();
        if pulled_full > 0 {
            parts.push(format!("pulled_full={}/{}", pulled_full, n));
        }

        // Back-pressure breakdown: how many rounds had each BP type active.
        let bp_txn_rounds = gap_batches.iter().filter(|r| r.bp_txn).count();
        let bp_proof_rounds = gap_batches.iter().filter(|r| r.bp_proof).count();
        match (bp_txn_rounds > 0, bp_proof_rounds > 0) {
            (true, true) => parts.push(format!(
                "bp_rounds={}(txn),{}(proof)/{}",
                bp_txn_rounds, bp_proof_rounds, n
            )),
            (true, false) => parts.push(format!("bp_rounds={}(txn)/{}", bp_txn_rounds, n)),
            (false, true) => parts.push(format!("bp_rounds={}(proof)/{}", bp_proof_rounds, n)),
            (false, false) => parts.push(format!("bp_rounds=0/{}", n)),
        }

        // Aggregate gas price distribution across all rounds in the gap.
        // Format: [gas_price_range:num_txns]=[0-149:2700txns,150-299:1200txns,500+:600txns]
        // Use bucket_boundaries from config to compute upper bounds correctly.
        let mut gas_totals: BTreeMap<u64, u64> = BTreeMap::new();
        for r in &gap_batches {
            for &(bucket, count) in &r.gas_bucket_txn_counts {
                *gas_totals.entry(bucket).or_insert(0) += count;
            }
        }
        if !gas_totals.is_empty() {
            // Get bucket boundaries from the first record (all records share the same config).
            let boundaries = &gap_batches[0].bucket_boundaries;
            let bucket_strs: Vec<String> = gas_totals
                .iter()
                .map(|(&start, &num_txns)| {
                    // Find the next boundary after `start` to compute the upper bound.
                    let next = boundaries.iter().find(|&&b| b > start).copied();
                    let gas_range = match next {
                        Some(next_start) => format!("{}-{}", start, next_start - 1),
                        None => format!("{}+", start),
                    };
                    format!("{}:{}txns", gas_range, num_txns)
                })
                .collect();
            parts.push(format!(
                "[gas_price_range:num_txns]=[{}]",
                bucket_strs.join(",")
            ));
        }
    }

    format!("wait({})", parts.join(","))
}

fn log_trace(trace: &TransactionTrace) {
    let base = trace.insertion_time_usecs;

    // Sort stages by timestamp so concurrent pipeline stages appear in order.
    let mut sorted_stages = trace.stages.clone();
    sorted_stages.sort_by_key(|s| s.timestamp_usecs);

    let max_attempt = sorted_stages.iter().map(|s| s.attempt).max().unwrap_or(1);

    // Build stage timeline chronologically. Insert [attempt_N] markers when the
    // attempt number increases on a non-block-finalization stage, so that
    // Certified/PreCommit/Committed after Executed(Retry) stay in the same group.
    // These block-finalization stages are kept for retried txns because they show
    // when the block commits, which triggers CommitNotification → batch generator
    // clears txns_in_progress_sorted → retried txn becomes eligible for re-pull.
    let mut stage_parts = Vec::new();
    let mut display_attempt: u32 = 0;
    // Track the timestamp of the previous stage for relative time display.
    let mut prev_stage_usecs = base;
    // Track whether we've already shown wait() for this attempt.
    let mut shown_wait_for_attempt: u32 = 0;
    // Find the first QsBatchPull for each attempt (for wait() summary).
    // Pre-scan so we can emit wait() at the start of the attempt, not just before QsBatchPull.
    let mut first_pull_per_attempt: std::collections::HashMap<u32, &crate::types::BatchPullInfo> =
        std::collections::HashMap::new();
    for record in &sorted_stages {
        if record.stage == TransactionStage::QsBatchPull {
            if let Some(StageMetadata::BatchPull(info)) = &record.metadata {
                first_pull_per_attempt.entry(record.attempt).or_insert(info);
            }
        }
    }
    for record in &sorted_stages {
        // Start a new attempt group when we see a higher attempt on a stage that
        // isn't block finalization (those trail the previous attempt's execution).
        let new_attempt = if is_block_finalization(record.stage) {
            display_attempt // keep current
        } else {
            record.attempt
        };
        if new_attempt > display_attempt {
            display_attempt = new_attempt;
            if max_attempt > 1 {
                stage_parts.push(format!("[attempt_{}]", display_attempt));
            }
            // Emit wait() at the start of the new attempt (using the first
            // QsBatchPull's metadata), before whatever stage comes first.
            if display_attempt > shown_wait_for_attempt {
                if let Some(info) = first_pull_per_attempt.get(&display_attempt) {
                    // Find the QsBatchPull timestamp for this attempt.
                    if let Some(pull_record) = sorted_stages.iter().find(|r| {
                        r.stage == TransactionStage::QsBatchPull && r.attempt == display_attempt
                    }) {
                        stage_parts.push(build_wait_summary(
                            info,
                            prev_stage_usecs,
                            pull_record.timestamp_usecs,
                        ));
                    }
                    shown_wait_for_attempt = display_attempt;
                }
            }
        }

        // Skip emitting wait() again when we reach the actual QsBatchPull stage
        // (it was already emitted at the attempt marker above).

        // Absolute time from MempoolInsert (base).
        let abs_ms = (record.timestamp_usecs as i64 - base as i64) / 1000;
        let stage_str = match &record.metadata {
            Some(StageMetadata::Execution(status)) => {
                format!("{}({})", record.stage.as_ref(), status.as_ref())
            },
            Some(StageMetadata::BatchInclusion(inclusion)) => {
                format!("{}({})", record.stage.as_ref(), inclusion.as_ref())
            },
            Some(StageMetadata::BatchPull(info)) => {
                format!(
                    "{}(n={},max={},excl={},bp={}/{})",
                    record.stage.as_ref(),
                    info.pulled_txn_count,
                    info.pull_max_txn,
                    info.excluded_txn_count,
                    if info.bp_txn { 1 } else { 0 },
                    if info.bp_proof { 1 } else { 0 },
                )
            },
            None => record.stage.as_ref().to_string(),
        };
        stage_parts.push(format!("{}={}ms", stage_str, abs_ms));
        prev_stage_usecs = record.timestamp_usecs;
    }

    let total_latency_ms = sorted_stages
        .last()
        .map(|s| (s.timestamp_usecs as i64 - base as i64) / 1000)
        .unwrap_or(0);

    // Determine outcome from the final stage, not the last Executed status.
    // A retried txn has Executed(Retry) in attempt 1 but eventually commits.
    let has_mempool_commit = sorted_stages
        .iter()
        .any(|s| s.stage == TransactionStage::MempoolCommit);
    let has_mempool_reject = sorted_stages
        .iter()
        .any(|s| s.stage == TransactionStage::MempoolReject);
    let outcome = if has_mempool_commit {
        "committed"
    } else if has_mempool_reject {
        "rejected"
    } else {
        // No final stage — check last execution status
        let last_exec = sorted_stages.iter().rev().find_map(|s| match &s.metadata {
            Some(StageMetadata::Execution(status)) => Some(*status),
            _ => None,
        });
        match last_exec {
            Some(ExecutionStatus::Discard) => "discarded",
            Some(ExecutionStatus::Retry) => "retry_incomplete",
            _ => "unknown",
        }
    };

    let gas_str = match trace.gas_unit_price {
        Some(g) => format!(" gas_unit_price={}", g),
        None => String::new(),
    };

    info!(
        "TxnTrace hash=0x{} sender={}{} attempts={} total_latency_ms={} outcome={} stages=[{}]",
        trace.hash.to_hex(),
        trace.sender,
        gas_str,
        max_attempt,
        total_latency_ms,
        outcome,
        stage_parts.join(" ")
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_maybe_start_trace_filters_by_sender() {
        let store = TransactionTraceStore::new();
        let sender = AccountAddress::random();
        let hash = HashValue::random();

        // Not traced when disabled
        assert!(!store.maybe_start_trace(hash, sender, 1000));

        // Enable with sender in allowlist
        let mut allowlist = std::collections::HashSet::new();
        allowlist.insert(sender);
        store.update_filter(TransactionFilter::new(true, allowlist, 1.0, 1.0));

        assert!(store.maybe_start_trace(hash, sender, 1000));
        assert!(store.traces.contains_key(&hash));

        // Non-allowlisted sender not traced
        let other = AccountAddress::random();
        let other_hash = HashValue::random();
        assert!(!store.maybe_start_trace(other_hash, other, 1000));
    }

    #[test]
    fn test_record_stage() {
        let store = TransactionTraceStore::new();
        let sender = AccountAddress::random();
        let hash = HashValue::random();

        let mut allowlist = std::collections::HashSet::new();
        allowlist.insert(sender);
        store.update_filter(TransactionFilter::new(true, allowlist, 1.0, 1.0));
        store.maybe_start_trace(hash, sender, 1000);

        store.record_stage_at(&hash, TransactionStage::QsBatchPull, 1500);
        store.record_stage_at(&hash, TransactionStage::QsBatchCreated, 2000);
        store.record_stage_at(&hash, TransactionStage::QsProofOfStore, 3000);

        let trace = store.get_trace(&hash).unwrap();
        assert_eq!(trace.stages.len(), 4); // MempoolInsert + 3 stages
        assert_eq!(trace.stages[1].stage, TransactionStage::QsBatchPull);
        assert_eq!(trace.stages[2].stage, TransactionStage::QsBatchCreated);
        assert_eq!(trace.stages[3].stage, TransactionStage::QsProofOfStore);
    }

    #[test]
    fn test_register_batch_filters_to_traced_only() {
        let store = TransactionTraceStore::new();
        let sender = AccountAddress::random();
        let traced_hash = HashValue::random();
        let untraced_hash = HashValue::random();
        let batch_digest = HashValue::random();

        let mut allowlist = std::collections::HashSet::new();
        allowlist.insert(sender);
        store.update_filter(TransactionFilter::new(true, allowlist, 1.0, 1.0));
        store.maybe_start_trace(traced_hash, sender, 1000);

        store.register_batch(batch_digest, &[traced_hash, untraced_hash]);

        let batch = store.batch_txns.get(&batch_digest).unwrap();
        assert_eq!(batch.value().len(), 1);
        assert_eq!(batch.value()[0], traced_hash);
    }

    #[test]
    fn test_batch_stage_noop_for_unregistered_batch() {
        let store = TransactionTraceStore::new();
        let batch_digest = HashValue::random();

        // Should not panic — just a no-op
        store.record_batch_stage(&batch_digest, TransactionStage::QsProofOfStore);
    }

    #[test]
    fn test_mark_retry_increments_attempt() {
        let store = TransactionTraceStore::new();
        let sender = AccountAddress::random();
        let hash = HashValue::random();

        let mut allowlist = std::collections::HashSet::new();
        allowlist.insert(sender);
        store.update_filter(TransactionFilter::new(true, allowlist, 1.0, 1.0));
        store.maybe_start_trace(hash, sender, 1000);

        assert_eq!(store.get_trace(&hash).unwrap().current_attempt, 1);
        store.mark_retry(&hash);
        assert_eq!(store.get_trace(&hash).unwrap().current_attempt, 2);
    }

    #[test]
    fn test_finalize_removes_trace() {
        let store = TransactionTraceStore::new();
        let sender = AccountAddress::random();
        let hash = HashValue::random();

        let mut allowlist = std::collections::HashSet::new();
        allowlist.insert(sender);
        store.update_filter(TransactionFilter::new(true, allowlist, 1.0, 1.0));
        store.maybe_start_trace(hash, sender, 1000);

        assert!(store.get_trace(&hash).is_some());
        store.finalize_trace(&hash);
        assert!(store.get_trace(&hash).is_none());
    }

    #[test]
    fn test_gc_removes_old_traces() {
        let store = TransactionTraceStore::new();
        let sender = AccountAddress::random();
        let old_hash = HashValue::random();
        let new_hash = HashValue::random();

        let mut allowlist = std::collections::HashSet::new();
        allowlist.insert(sender);
        store.update_filter(TransactionFilter::new(true, allowlist, 1.0, 1.0));

        // Old trace (insertion at t=1000)
        store.maybe_start_trace(old_hash, sender, 1000);
        // New trace (insertion at t=now)
        store.maybe_start_trace(new_hash, sender, now_usecs());

        // GC with 1-second TTL should remove the old trace
        store.gc(1_000_000);

        assert!(store.get_trace(&old_hash).is_none());
        assert!(store.get_trace(&new_hash).is_some());
    }
}
