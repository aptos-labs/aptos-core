# Transaction Lifecycle Tracing — Design Document

## Context

We need per-transaction tracing across the Aptos blockchain pipeline, primarily for **Decibel market making transactions**. Given a transaction hash, the system should report which lifecycle stages it passed through and the latency between them. Transactions can be retried (cut by block gas limit), so the lifecycle is cyclic with multiple "attempts". Only transactions from configured sender addresses are traced.

## Transaction Lifecycle (Partially Ordered, Cyclic)

The lifecycle is NOT a fixed linear sequence. Some stages run concurrently and can complete in either order. The execution pipeline is **decoupled**: all pipeline futures are spawned via `tokio::spawn()` at block insertion time (proposal processing), and run concurrently with consensus ordering.

### Batch inclusion paths

All batches are created equally by BatchGenerator. At proposal time, ProofManager pulls batches in one of three forms:
- **proof**: batch has 2f+1 signatures (ProofOfStore)
- **opt_batch**: batch included optimistically without proof
- **inline**: full transactions included directly in payload

### Execution vs Ordering timing

Pipeline futures are spawned at block insertion (proposal time) via `pipeline_builder.build_for_consensus()` (`pipeline_builder.rs:504`, `tokio::spawn()`). The execution chain is:

```
materialize (needs QC) → decrypt (needs secret_shared_key) → prepare → execute (needs rand_check + parent)
```

`ExecutionSchedulePhase` runs AFTER ordering and sends two signals (`execution_schedule_phase.rs:66-69`):
- `rand_tx` — unblocks `rand_check_fut` (needed by `execute`)
- `secret_shared_key_tx` — unblocks `decrypt` (needed for encrypted txns)

Three timing scenarios:
- **No randomness + no encrypted txns**: Both signals resolve immediately (`spawn_ready_fut`) → VM execution can start BEFORE ordering
- **Randomness enabled**: `rand_check_fut` blocks on `rand_rx` → `execute` waits for ordering
- **Encrypted txns**: `decrypt` blocks on `secret_shared_key_rx` → entire chain (`decrypt → prepare → execute`) waits for ordering

### Lifecycle diagram (partial order)

```
                    mempool_insert
                          │
                          ▼
                    qs_batch_pull ◄──────────────────── (retry) ───┐
                          │                                        │
                          ▼                                        │
                    qs_batch_created                                │
                          │                                        │
                   ┌──────┴──────┐                                 │
                   │             │                                 │
                   ▼             │  (opt/inline: skip)             │
            qs_proof_of_store    │                                 │
                   │             │                                 │
                   └──────┬──────┘                                 │
                          │                                        │
                          ▼                                        │
                    block_proposed                                  │
                  (proof | opt | inline)                            │
                          │                                        │
                 ┌────────┴────────┐                               │
                 │    see note*    │                               │
                 ▼                 ▼                               │
           block_ordered    execution_start                        │
                 │                 │                               │
                 │        ┌────────────────┼────────────────┐      │
                 │        │                │                │      │
                 │   executed(keep)   executed(retry)  executed(discard)
                 │        │                │                │      │
                 │        │                └────────────────┼──────┘
                 │        │                                 │
                 └───┬────┘                                 ▼
                     │                              mempool_reject
                     ▼
                (certified)          ← not tracked: commit vote signing
                (2f+1 commit votes     + CommitDecision aggregation)
                     │
                     ▼
                 committed
                     │
                     ▼
               mempool_commit

  * block_ordered vs execution_start ordering:
    - No randomness + no encrypted txns → execution_start may precede block_ordered
    - Randomness enabled → execution_start blocked on rand_rx, sent AFTER ordering
    - Encrypted txns → execution_start blocked on secret_shared_key, sent AFTER ordering
```

### Stage descriptions

| Stage | When | Trigger |
|-------|------|---------|
| `mempool_insert` | Txn accepted into mempool | `mempool.rs::add_txn()` returns Accepted |
| `qs_batch_pull` | QS BatchGenerator pulls txn from mempool | `batch_generator.rs::handle_scheduled_pull()` after `pull_internal()` |
| `qs_batch_created` | Batch formed and broadcast to validators | `batch_generator.rs::create_new_batch()` |
| `qs_proof_of_store` | 2f+1 validator signatures aggregated (proof path only) | `proof_coordinator.rs::add_signature()` when proof created |
| `block_proposed` | Block proposal created by leader | Uses `block.timestamp_usecs()` (leader's clock) — available on ALL validators, not just the leader |
| `block_ordered` | Block ordered by consensus (order votes / 2-chain) | `buffer_manager.rs::process_ordered_blocks()` using local clock |
| `execution_start` | VM execution begins (may be before `block_ordered` if randomness disabled) | `pipeline_builder.rs::execute()` after prepare_fut + rand_check resolve |
| `executed` | Txn execution completed (with ExecutionStatus metadata: Keep/Retry/Discard) | `do_get_execution_output.rs` after extract_retries_and_discards |
| *(certified)* | *(not tracked)* Commit vote signed on execution result, 2f+1 votes aggregated into CommitDecision | `signing_phase.rs` → `commit_reliable_broadcast.rs` |
| `committed` | Block committed to ledger (requires both ordered + certified) | `buffer_manager.rs::advance_head()` |
| `mempool_commit` | Txn removed from mempool (committed) | `tasks.rs::process_committed_transactions()` |
| `mempool_reject` | Txn removed from mempool (rejected) | `tasks.rs::process_rejected_transactions()` |

### Key timing notes

1. `qs_proof_of_store` may occur before or after `block_proposed` (opt path skips it entirely)
2. `execution_start` vs `block_ordered` ordering depends on block contents:
   - No randomness + no encrypted txns → `execution_start` can precede `block_ordered`
   - Randomness enabled → `execution_start` AFTER `block_ordered` (blocked on `rand_rx`)
   - Encrypted txns → `execution_start` AFTER `block_ordered` (blocked on `secret_shared_key_rx`)
3. `committed` requires BOTH `block_ordered` AND `executed(keep)` to have completed
4. The preparation chain (materialize → decrypt → prepare) starts at proposal time, but `decrypt` may block execution (not ordering) if encrypted txns are present — it waits on `secret_shared_key_rx` sent after ordering
5. When a txn gets `executed(retry)`, it stays in mempool and re-enters the lifecycle at `qs_batch_pull`

## Architecture

### New Crate: `crates/aptos-transaction-tracing/`

```
crates/aptos-transaction-tracing/
├── Cargo.toml
├── DESIGN.md
└── src/
    ├── lib.rs          # Public API, global init
    ├── types.rs        # TransactionStage, TransactionTrace, StageRecord
    ├── store.rs        # TransactionTraceStore (DashMap singleton)
    ├── filter.rs       # TransactionFilter
    └── counters.rs     # Prometheus histogram (TXN_TRACING)
```

**Dependencies**: `aptos-crypto` (HashValue), `aptos-types` (AccountAddress), `aptos-metrics-core`, `dashmap`, `once_cell`, `strum`/`strum_macros` (for stage enum → string).

### Data Model

```rust
// types.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, strum_macros::AsRefStr)]
pub enum TransactionStage {
    MempoolInsert,
    QsBatchPull,          // QS BatchGenerator pulls txn from mempool
    QsBatchCreated,       // Batch formed and broadcast
    QsProofOfStore,       // 2f+1 signatures → ProofOfStore (proof path only)
    BlockProposed,        // Included in block proposal (with BatchInclusionType metadata)
    BlockOrdered,         // Block ordered by consensus
    ExecutionStart,       // Block enters execution
    Executed,             // Txn execution completed (with ExecutionStatus metadata)
    Committed,            // Block committed to storage
    MempoolCommit,        // Removed from mempool (committed)
    MempoolReject,        // Removed from mempool (rejected)
}

/// Batch inclusion type in a block proposal
#[derive(Debug, Clone, Copy, PartialEq, Eq, strum_macros::AsRefStr)]
pub enum BatchInclusionType {
    Proof,   // Proved batch (has 2f+1 sigs)
    Opt,     // Optimistic batch (no proof yet)
    Inline,  // Full transactions inline
}

pub struct StageRecord {
    pub stage: TransactionStage,
    pub timestamp_usecs: u64,
    pub attempt: u32,
    pub metadata: Option<StageMetadata>,  // Optional extra info
}

/// Execution outcome
#[derive(Debug, Clone, Copy, PartialEq, Eq, strum_macros::AsRefStr)]
pub enum ExecutionStatus {
    Keep,     // Txn executed successfully
    Retry,    // Cut by block gas limit (stays in mempool)
    Discard,  // Permanently rejected
}

/// Additional metadata for specific stages
pub enum StageMetadata {
    BatchInclusion(BatchInclusionType),  // For BlockProposed stage
    Execution(ExecutionStatus),          // For Executed stage
}

pub struct TransactionTrace {
    pub hash: HashValue,
    pub sender: AccountAddress,
    pub insertion_time_usecs: u64,
    pub current_attempt: u32,
    pub stages: Vec<StageRecord>,
}
```

### Global Store

```rust
// store.rs
pub struct TransactionTraceStore {
    traces: DashMap<HashValue, TransactionTrace>,
    /// batch_digest → traced txn hashes only. Only batches containing ≥1 traced txn are registered.
    /// This means `record_batch_stage()` is a DashMap miss (no-op) for non-traced batches.
    batch_txns: DashMap<HashValue, Vec<HashValue>>,
    filter: ArcSwap<TransactionFilter>,
}

static GLOBAL_STORE: Lazy<TransactionTraceStore> = Lazy::new(TransactionTraceStore::new);

// Primary API
impl TransactionTraceStore {
    pub fn global() -> &'static Self { &GLOBAL_STORE }

    /// Called at mempool insertion. Checks if sender is in allowlist, creates trace if matched.
    pub fn maybe_start_trace(&self, hash: HashValue, sender: AccountAddress, now_usecs: u64) -> bool;

    /// Record a stage for a traced transaction (uses local clock).
    pub fn record_stage(&self, hash: &HashValue, stage: TransactionStage);

    /// Record a stage with explicit timestamp (e.g., block.timestamp_usecs from leader's clock).
    pub fn record_stage_at(&self, hash: &HashValue, stage: TransactionStage, timestamp_usecs: u64);

    /// Record a stage for all traced txns in a batch (by batch digest).
    pub fn record_batch_stage(&self, batch_digest: &HashValue, stage: TransactionStage);

    /// Record a stage with metadata for all traced txns in a batch (uses local clock).
    pub fn record_batch_stage_with_metadata(&self, batch_digest: &HashValue, stage: TransactionStage, metadata: StageMetadata);

    /// Record a stage with metadata and explicit timestamp for all traced txns in a batch.
    pub fn record_batch_stage_with_metadata_at(&self, batch_digest: &HashValue, stage: TransactionStage, metadata: StageMetadata, timestamp_usecs: u64);

    /// Register batch_digest → traced txn hashes mapping.
    /// Filters txn_hashes to only those with active traces. If none are traced, skips registration
    /// entirely — making all subsequent `record_batch_stage()` calls for this digest a no-op.
    pub fn register_batch(&self, batch_digest: HashValue, txn_hashes: &[HashValue]);

    /// Mark retry: increment attempt counter.
    pub fn mark_retry(&self, hash: &HashValue);

    /// Finalize and log the completed trace. Removes from store.
    pub fn finalize_trace(&self, hash: &HashValue);

    /// Query a trace by hash (returns clone). For API/debugging.
    pub fn get_trace(&self, hash: &HashValue) -> Option<TransactionTrace>;

    /// Get all active traces (for admin API). Returns vec of (hash, trace) pairs.
    pub fn get_all_traces(&self) -> Vec<(HashValue, TransactionTrace)>;

    /// Update the filter at runtime (e.g., from admin API).
    pub fn update_filter(&self, filter: TransactionFilter);

    /// Cleanup traces older than TTL.
    pub fn gc(&self, ttl_usecs: u64);
}
```

### Filter (sender-address based)

```rust
// filter.rs
pub struct TransactionFilter {
    pub enabled: bool,
    pub sender_allowlist: HashSet<AccountAddress>,  // Decibel MM bot addresses
}

impl TransactionFilter {
    /// Returns true if txn sender is in the allowlist.
    pub fn should_trace(&self, sender: &AccountAddress) -> bool {
        self.enabled && self.sender_allowlist.contains(sender)
    }
}
```

### Prometheus Counters

```rust
// counters.rs — follows existing observe_batch pattern (2D labels: sender + stage)
pub static TXN_TRACING: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "aptos_transaction_tracing",
        "Per-transaction latency from mempool insertion to each lifecycle stage",
        &["sender", "stage"],
        TRACING_BUCKETS.to_vec()
    ).unwrap()
});

/// Record latency from insertion to this stage, grouped by sender.
/// Safe because sender set is bounded (only allowlisted Decibel MM addresses).
pub fn observe_txn(insertion_time_usecs: u64, sender: &str, stage: &str) {
    if let Some(t) = duration_since_epoch()
        .checked_sub(Duration::from_micros(insertion_time_usecs))
    {
        TXN_TRACING.with_label_values(&[sender, stage]).observe(t.as_secs_f64());
    }
}
```

Query example: `histogram_quantile(0.95, rate(aptos_transaction_tracing_bucket{sender="0xabc...",stage="committed"}[5m]))` gives p95 end-to-end latency for a specific Decibel bot.

---

## Instrumentation Points

### Mempool (2 record points + 2 finalize points)

| Stage | File | Location | How |
|-------|------|----------|-----|
| `MempoolInsert` | `mempool/src/core_mempool/mempool.rs` | `add_txn()` after `status.code == Accepted` | `maybe_start_trace(txn.committed_hash(), txn.sender(), now_usecs)` — checks sender against allowlist |
| `MempoolCommit` | `mempool/src/shared_mempool/tasks.rs` | `process_committed_transactions()` loop | Lookup hash via `pool.transactions` before `commit_transaction()`, then `record_stage` + `finalize_trace` |
| `MempoolReject` | `mempool/src/shared_mempool/tasks.rs` | `process_rejected_transactions()` loop | `record_stage(transaction.hash, MempoolReject)` + `finalize_trace` |

### Quorum Store (3 record points)

| Stage | File | Location | How |
|-------|------|----------|-----|
| `QsBatchPull` | `consensus/src/quorum_store/batch_generator.rs` | `handle_scheduled_pull()` after `pull_internal()` returns | Iterate `Vec<SignedTransaction>`, call `record_stage(txn.committed_hash(), QsBatchPull)` |
| `QsBatchCreated` | `consensus/src/quorum_store/batch_generator.rs` | `create_new_batch()` | `register_batch(digest, hashes)` + `record_stage` for each txn |
| `QsProofOfStore` | `consensus/src/quorum_store/proof_coordinator.rs` | `add_signature()` when proof created | `record_batch_stage(proof.info().digest(), QsProofOfStore)` |

### Consensus Pipeline (2 record points — both at ordering time in buffer_manager)

| Stage | File | Location | How |
|-------|------|----------|-----|
| `BlockProposed` | `consensus/src/pipeline/buffer_manager.rs` | `process_ordered_blocks()` | Use `block.timestamp_usecs()` as the stage timestamp (leader's clock). Iterate ordered blocks' payloads → `record_batch_stage_with_metadata_at(digest, BlockProposed, BatchInclusion(type), block.timestamp_usecs())`. The inclusion type is determined from the payload variant (proof/opt/inline). Available on ALL validators. |
| `BlockOrdered` | `consensus/src/pipeline/buffer_manager.rs` | `process_ordered_blocks()` | Same location, use local clock. Iterate ordered blocks' payloads → `record_batch_stage(digest, BlockOrdered)` |

### Execution (2 record points)

| Stage | File | Location | How |
|-------|------|----------|-----|
| `ExecutionStart` | `execution/executor/src/block_executor/mod.rs` | `execute_and_update_state()` before `DoGetExecutionOutput` | Iterate `ExecutableBlock.transactions`, filter user txns, `record_stage(hash, ExecutionStart)` |
| `Executed` | `execution/executor/src/workflow/do_get_execution_output.rs` | After `extract_retries_and_discards()` returns | Iterate `to_commit`→`Executed(Keep)`, `to_retry`→`Executed(Retry)` + `mark_retry()`, `to_discard`→`Executed(Discard)` |

---

## Query Interface

Three ways to get a transaction's lifecycle latency breakdown by hash:

### 1. Programmatic (in-process)
```rust
if let Some(trace) = TransactionTraceStore::global().get_trace(&hash) {
    for record in &trace.stages {
        let latency_ms = (record.timestamp_usecs - trace.insertion_time_usecs) / 1000;
        println!("[attempt {}] {} = +{}ms", record.attempt, record.stage.as_ref(), latency_ms);
    }
}
```

### 2. Structured log output (automatic on completion)
When a transaction reaches a terminal stage, `finalize_trace()` emits an info-level log:
```
TxnTrace hash=0x1234 sender=0xabcd attempts=1 total_latency_ms=2150
  mempool_insert=0ms qs_batch_pull=+50ms qs_batch_created=+55ms
  qs_proof_of_store=+200ms block_proposed=+500ms block_ordered=+600ms
  execution_start=+650ms executed(keep)=+700ms committed=+2100ms
  mempool_commit=+2150ms
```
These logs can be searched in Humio/logging infrastructure by `TxnTrace hash=<hash>`.

### 3. Prometheus histograms (aggregate, grouped by sender)
`aptos_transaction_tracing{sender="0x...", stage="..."}` histogram shows p50/p95/p99 latency from mempool insertion to each stage, grouped by sender address. Safe cardinality since only allowlisted Decibel MM addresses are traced.

---

## Admin API

A POST endpoint on the **inspection service** allows updating the tracing filter at runtime without node restart. This is useful for:
- Production: updating the Decibel MM bot allowlist dynamically
- Forge tests: enabling tracing for specific accounts after account creation

### Endpoint

```
POST /transaction_tracing
Content-Type: application/json

{
  "enabled": true,
  "sender_allowlist": ["0xabc...", "0xdef..."]
}
```

**Response:** `200 OK` with the updated filter config as JSON.

### Implementation

Add to `crates/aptos-inspection-service/src/server/mod.rs`:

```rust
pub const TRANSACTION_TRACING_PATH: &str = "/transaction_tracing";

// In serve_requests(), add:
(TRANSACTION_TRACING_PATH, Method::POST) => {
    let body = hyper::body::to_bytes(req.into_body()).await?;
    transaction_tracing::handle_update_filter(body)
}
(TRANSACTION_TRACING_PATH, Method::GET) => {
    transaction_tracing::handle_get_filter()
}
```

The handler parses the JSON body, constructs a `TransactionFilter`, and calls `TransactionTraceStore::global().update_filter(filter)`.

### Security

The inspection service is **not exposed externally** — it binds to `0.0.0.0:9101` by default and is only accessible within the cluster network. No authentication needed.

---

## Forge Test

A forge test validates end-to-end tracing on a remote testnet with logs visible in Humio.

### Test design

The test uses default land-blocking traffic settings (~10K TPS, ~1900 accounts) but only traces 5 accounts:

1. Start validators with `transaction_tracing.enabled = true` (empty allowlist)
2. Emitter creates and funds accounts
3. Test picks 5 account addresses
4. Test calls `POST /transaction_tracing` on each validator's inspection service to set the allowlist
5. Traffic runs for the test duration
6. Only the 5 traced accounts produce `TxnTrace` log lines

At ~10K TPS with 1900 accounts, each account does ~5 TPS. **5 traced accounts × 5 TPS = ~25 traced TPS**. Over 30 seconds = **~750 `TxnTrace` log entries** — easy to browse in Humio.

### Implementation

```rust
// testsuite/testcases/src/transaction_tracing_test.rs
pub struct TransactionTracingTest;

impl Test for TransactionTracingTest {
    fn name(&self) -> &'static str { "transaction_tracing_test" }
}

#[async_trait]
impl NetworkTest for TransactionTracingTest {
    async fn run<'a>(&self, ctx: NetworkContextSynchronizer<'a>) -> Result<()> {
        let mut ctx_locker = ctx.ctx.lock().await;
        let ctx = ctx_locker.deref_mut();

        // Step 1: Generate traffic (accounts are created during warmup)
        let duration = Duration::from_secs(30);
        let validators = ctx.swarm.read().await.validators().map(|v| v.peer_id()).collect::<Vec<_>>();
        let txn_stat = generate_traffic(ctx, &validators, duration).await?;

        // Step 2: Pick 5 accounts and update tracing filter on all validators
        // (account addresses are available after emitter warmup creates them)
        let traced_addresses: Vec<AccountAddress> = /* pick 5 from created accounts */;
        info!("Traced accounts for Humio lookup: {:?}", traced_addresses);

        for validator in swarm.validators() {
            let inspection_url = validator.inspection_service_endpoint();
            // POST /transaction_tracing with { enabled: true, sender_allowlist: traced_addresses }
        }

        // Step 3: Log traced accounts so user can search Humio
        for addr in &traced_addresses {
            info!("TxnTracing: tracking account {}", addr);
        }

        ctx.report.report_txn_stats(self.name().to_string(), &txn_stat);
        Ok(())
    }
}
```

### Registration

```rust
// testsuite/forge-cli/src/suites/ungrouped.rs
"transaction_tracing_test" => Some(transaction_tracing_test()),

fn transaction_tracing_test() -> ForgeConfig {
    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(7).unwrap())
        .with_initial_fullnode_count(2)
        .add_network_test(TransactionTracingTest)
        .with_validator_override_node_config_fn(Arc::new(|config, _| {
            config.consensus.transaction_tracing.enabled = true;
            // Empty allowlist initially — updated via admin API after account creation
        }))
}
```

### How to run and check logs

```bash
# Run the forge test
forge test transaction_tracing_test --suite ungrouped

# In forge output, look for:
#   TxnTracing: tracking account 0x...
# These are the 5 accounts being traced.

# In Humio, search for:
TxnTrace
# Or filter by a specific tracked account:
TxnTrace sender=0x<address from forge output>
```

---

## Rollout

### Configuration

Add a new section to the **local node config** (YAML) under `consensus`:

```yaml
# in node.yaml
consensus:
  transaction_tracing:
    enabled: false                    # off by default
    sender_allowlist:                 # Decibel MM bot addresses
      - "0xabc..."
      - "0xdef..."
    gc_ttl_secs: 300                  # cleanup traces older than 5min
```

This follows the existing local config pattern (`config/src/config/consensus_config.rs`). A new `TransactionTracingConfig` struct is added to `ConsensusConfig`.

**Why local config, not on-chain config:** This is per-node observability, not protocol-level behavior. Different operators may want different allowlists. On-chain config would require governance proposals for what is essentially a debug toggle.

### Single-Node Sufficiency

A single tracing-enabled validator that receives Decibel MM txns captures **all stages** of the lifecycle:
- `mempool_insert` through `qs_proof_of_store`: Captured locally (entry validator creates batches from its own mempool)
- `block_proposed`: Uses `block.timestamp_usecs()` (leader's clock), available on ALL validators — no leader dependency
- `block_ordered` through `mempool_commit`: All validators execute and commit

**No inter-validator communication needed.** No stages are missing. Clock drift between validators (NTP-synced, typically <10ms) is the only source of inaccuracy for `block_proposed`.

### Deployment Steps

1. **Merge PR** — No protocol changes. Pure additive instrumentation code.
2. **Update node config** — Add `transaction_tracing` section with `enabled: true` and optionally pre-populate `sender_allowlist`.
3. **Restart node** — Config takes effect.
4. **Update allowlist (optional)** — Use `POST /transaction_tracing` on the inspection service to add/change sender addresses without restart.
5. **Observe** — Structured logs (`TxnTrace hash=...`) appear in Humio. Prometheus histograms populate at `aptos_transaction_tracing_bucket`.

### Performance Impact

- **Negligible for small allowlists.** The hot path is a `HashSet::contains()` check on sender address at mempool insertion — O(1). If sender not in allowlist, zero overhead for remaining stages.
- **DashMap lookups** at each instrumentation point are O(1) with minimal contention (only allowlisted txns).
- **No impact on consensus or execution correctness.** All instrumentation is fire-and-forget recording, no blocking.
- **Memory:** Each trace is ~200 bytes + ~50 bytes per stage record. With GC at 5min TTL, a bot sending 100 TPS would use ~6MB of trace data.

### Safety

- **No consensus-affecting changes.** No modifications to safety rules, voting, or block production logic.
- **No new network messages.** Tracing is purely local to each node.
- **No serialization changes.** No changes to any on-wire or on-disk formats.
- **Graceful degradation.** If tracing is disabled (default), all `record_stage` calls short-circuit on the `DashMap::get()` miss.
