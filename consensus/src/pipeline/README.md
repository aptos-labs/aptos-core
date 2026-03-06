# Pipeline

The blockchain pipeline decouples block ordering from execution, certification, and persistence. Different blocks progress through stages in parallel, exploiting distinct resource utilization (network for consensus, CPU for execution, IO for storage). See the [Zaptos paper](https://arxiv.org/pdf/2501.10612) for the full design.

## Overview

A transaction on Aptos goes through four major stages:

1. **Consensus** — orders transactions into blocks (network-bound)
2. **Execution** — executes transactions in a block via Block-STM (CPU-bound)
3. **Certification** — validators sign and aggregate signatures on execution results (network-bound)
4. **Commit** — persists certified blocks to storage (IO-bound)

### Baseline Pipeline

The baseline Aptos pipeline overlaps these stages across consecutive blocks. At any given time, block i is being ordered, block i-1 is being executed, block i-2 is being certified, and block i-3 is being committed:

```
Time ──────────────────────────────────────────────────────────────────────────────►

Block N:   [ Consensus ][ Execution ][ Certification ][ Commit ]
Block N+1:              [ Consensus ][ Execution     ][ Certification ][ Commit ]
Block N+2:                           [ Consensus     ][ Execution     ][ Cert.. ][ Commit ]
```

This achieves high throughput by keeping CPU, network, and IO resources busy simultaneously.

### Zaptos Optimizations

Zaptos reduces end-to-end latency through three optimizations that **shadow** execution, certification, and commit under the consensus latency — so by the time a block is ordered, it has already been executed, certified, and persisted:

```
            propose         vote      order vote       ordered
               │              │            │               │
               ▼              ▼            ▼               ▼
Consensus:     [──────────────┼────────────┼───────────────]
               │              │            │               │
Execution:     │              [============]               │  opt. execution (starts on vote)
               │              │            │               │
Commit:        │              │            [====]          │  opt. commit (pre-commit)
               │              │            │               │
Certify:       │              │            [===============]  early certification
               │              │            │               │
               │              │            │  all 3 done   │
               │              │            │  by ordered   │
```

Execution, certification, and commit are **shadowed** within the consensus latency. By the time a block is ordered, it has already been executed, certified, and persisted.

**1. Optimistic Execution** — validators begin executing a block immediately upon receiving the proposal, before consensus ordering completes. Execution runs in parallel with the remaining voting rounds.

**2. Optimistic Commit (Pre-Commit)** — once execution completes, the new state is written to storage immediately, before certification finishes. The state is marked as `OptCommitted` and later promoted to `Committed` when certification completes. If the block is not ordered, the optimistically committed state is reverted.

**3. Early Certification** — validators broadcast their certification vote (commit vote) when sending their order vote, if execution has already completed. This overlaps certification with the final consensus round, effectively reducing latency by one round.

The result: end-to-end blockchain latency ≈ consensus latency, with execution, certification, and commit hidden within it.

## Implementation

`PipelineBuilder` constructs a graph of async futures for each block. Each stage is a spawned task that awaits its dependencies before executing. Dependencies are of two kinds:

1. **Intra-block** — later stages await earlier stages of the same block (e.g., execute waits for prepare)
2. **Inter-block** — each stage awaits the same stage of its parent block (e.g., block N+1's execute waits for block N's execute), ensuring sequential state transitions while allowing different stages to overlap across blocks

### Per-Block Pipeline Stages

| Stage                  | Waits for                                     | What it does                                                       |
| ---------------------- | --------------------------------------------- | ------------------------------------------------------------------ |
| **Materialize**        | QC arrives for block                          | Resolves payload (fetches batches from QuorumStore)                |
| **Prepare**            | Materialize, Decryption (if enabled)          | Prepares block for execution, verifies transaction signatures      |
| **Rand Txns Check**    | Prepare, parent Execute                       | Scans transactions for randomness annotations                      |
| **Wait for Rand**      | Rand Txns Check, rand_rx                      | Waits for randomness value if block needs it; no-op otherwise      |
| **Execute**            | Prepare, Wait for Rand, parent Execute        | Runs Block-STM parallel execution                                  |
| **Ledger Update**      | Execute, parent Ledger Update                 | Generates `StateComputeResult` from execution output               |
| **Commit Vote**        | Ledger Update, order vote/proof/commit proof  | Signs execution result and broadcasts `CommitVote`                 |
| **Pre-Commit**         | Ledger Update, parent Pre-Commit, order proof | Writes result to storage after ordering but before commit proof    |
| **Commit Ledger**      | Pre-Commit, commit proof, parent Commit       | Finalizes committed state — makes data visible to clients          |
| **Post Ledger Update** | Ledger Update                                 | Notifies mempool about failed transactions (off critical path)     |
| **Notify State Sync**  | Pre-Commit, Commit Ledger                     | Notifies state synchronizer about committed transactions           |
| **Post Commit**        | Commit Ledger, parent Post Commit             | Updates counters, notifies block store                             |

### How Zaptos Optimizations Map to Stages

**Optimistic Execution**: Materialize starts as soon as the QC for the block arrives (via `qc_rx`), which happens when the validator votes — before the block is ordered. Prepare, execute, and ledger update then proceed immediately, overlapping with the remaining consensus rounds.

**Early Certification**: Commit Vote waits for ledger update AND any one of: order vote (`order_vote_rx`), order proof, or commit proof. When order votes are enabled, the commit vote is broadcast as soon as the validator sends its order vote and execution is complete — overlapping certification with the final consensus round.

**Pre-Commit**: In production, pre-commit waits for the **order proof** before writing to storage, avoiding the need to roll back optimistically committed state. This means pre-commit happens after ordering but before the full commit proof, still saving latency. For epoch-ending blocks or when pre-commit is paused (e.g., during state sync), it additionally waits for the commit proof.

### Randomness

When on-chain randomness is enabled, the randomness check is split into two futures:

1. **Rand Txns Check** (`has_rand_txns_fut`) — scans the block's transactions for randomness annotations after prepare and parent execute complete. This determines whether the block needs randomness.
2. **Wait for Rand** (`rand_check_fut`) — if the block needs randomness, waits for the randomness value delivered via `rand_rx` (which depends on consensus-driven randomness aggregation). If no randomness is needed, this is a no-op.

If any transaction requires randomness, execution **cannot** proceed optimistically — it must wait for the randomness value, which blocks the Execute stage and prevents optimistic execution from overlapping with consensus. As a result, the Zaptos latency optimizations do not apply to blocks containing randomness transactions.

### Pipeline Inputs

Each block receives external signals via `PipelineInputTx` channels:
- **qc_rx** — QC for this block (triggers materialize)
- **rand_rx** — randomness value (if block needs randomness)
- **order_vote_rx** — order vote received for this block
- **order_proof_tx** — order proof (WrappedLedgerInfo) for this block
- **commit_proof_tx** — commit proof (LedgerInfoWithSignatures) for this block

## BufferManager

The `BufferManager` manages a linked buffer of ordered blocks and orchestrates commit vote aggregation. It receives ordered blocks from consensus, feeds them into the `PipelineBuilder` futures, and handles commit messages from the network.

### BufferItem States

| State        | Description                                        |
| ------------ | -------------------------------------------------- |
| `Ordered`    | Received from consensus, pipeline futures created  |
| `Executed`   | Execution complete, collecting commit votes        |
| `Signed`     | Own commit vote broadcast, aggregating signatures  |
| `Aggregated` | 2f+1 commit votes collected, ready to persist      |

### Commit Vote Aggregation

Commit votes can arrive before a block is executed (e.g., if other validators are faster). The `BufferManager` caches these as pending votes and applies them when the block reaches the appropriate state. A full `CommitDecision` can fast-forward a block directly to the Aggregated state.

### Reliable Broadcast

Commit votes use reliable broadcast with:
- Exponential backoff starting at 2ms, max 5 seconds
- Broadcast interval: 1500ms
- Rebroadcast of stale votes: every 30 seconds
- Tracks ACKs from all validators; completes when all respond

### Backpressure

Backpressure operates at two levels:

**1. BufferManager (block intake)**

The `BufferManager` stops accepting new ordered blocks when the gap between the latest ordered round and the highest committed round exceeds `MAX_BACKLOG` (20 rounds). This is implemented as a `tokio::select!` guard on the block intake channel — when backpressure is active, the `block_rx.next()` branch is disabled, so consensus blocks waiting to enter the pipeline are queued until commits catch up.

This was originally introduced to prevent state sync from receiving a ledger info older than the pre-committed version — if the buffer grew unboundedly, pre-commit could advance far ahead of the commit root, and a state sync trigger would conflict with already pre-committed state. This root cause has since been fixed by adding `pre_commit_status` to connect `sync_manager` and the pipeline (`413db84eeb`), which pauses pre-commit when state sync is needed. The backpressure remains as a general safety bound on buffer size.

**2. ProposalGenerator (block size reduction)**

The `ProposalGenerator` applies finer-grained backpressure on proposals using two signals:

- **Pipeline pending latency**: Measures the time since the oldest unexecuted block was ordered. When this exceeds configured thresholds, the proposal generator reduces `max_sending_block_txns_after_filtering`, `max_sending_block_bytes`, and adds a `backpressure_proposal_delay_ms` before proposing. This slows down consensus to let the pipeline drain.

- **Execution time**: Looks at recent block execution times (configurable window via `num_blocks_to_look_at`). When execution is slow, the proposal generator reduces the transaction limit and gas limit for new blocks, producing smaller blocks that execute faster.

Both signals take the minimum across all backpressure sources, so the most restrictive limit applies.

### Reset and State Sync

Resets are triggered via `ResetRequest` with two variants:

- **`TargetRound(round)`** — sent by `sync_to_target` when the node falls behind and needs state sync. Updates `highest_committed_round` and `latest_round` to the target, and drains any pending commit proofs up to that round.
- **`Stop`** — sent by `end_epoch` at epoch boundaries. Sets a stop flag that terminates the `BufferManager` main loop after cleanup.

Both variants then execute the same cleanup sequence:

1. **Wait for pending commits** — blocks in `pending_commit_blocks` (already aggregated and handed off for commit) are awaited to completion, since they have no remaining dependencies and aborting them can cause errors at epoch boundaries.
2. **Abort buffered blocks** — all remaining items in the buffer have their pipeline futures aborted via `abort_pipeline()`, then awaited until finished.
3. **Clear buffer state** — the buffer is replaced with a fresh empty buffer, and `execution_root`, `signing_root`, and `commit_proof_rb_handle` are cleared.
4. **Drain incoming block queue** — any blocks queued in `block_rx` that haven't been processed yet are popped and their pipeline futures aborted.
5. **Wait for ongoing tasks** — polls `ongoing_tasks` counter until it reaches zero, ensuring all spawned tasks have completed.
6. **Send ack** — sends `ResetAck` back to the caller (state sync or epoch manager), which blocks until this point.

The reset order is: rand manager first, then secret share manager, then buffer manager — each awaiting its ack before proceeding to the next. For state sync (`TargetRound`), after all managers are reset, the execution proxy performs the actual state sync to the target ledger info. For epoch end (`Stop`), the execution proxy's `end_epoch` is called after all managers stop.

### Message Types

| Message          | Purpose                                            | Sender → Receiver              |
| ---------------- | -------------------------------------------------- | ------------------------------ |
| `CommitVote`     | Validator's signature on execution result          | Validator → all validators     |
| `CommitDecision` | 2f+1 commit vote signatures (commit proof)         | Any validator → all validators |

## Key Files

| File                           | Purpose                                                                                    |
| ------------------------------ | ------------------------------------------------------------------------------------------ |
| `pipeline_builder.rs`          | Constructs per-block future graph (the Zaptos pipeline) — materialize through post-commit  |
| `buffer_manager.rs`            | Central orchestrator — ordered buffer, commit vote aggregation, reliable broadcast         |
| `buffer_item.rs`               | BufferItem enum with state transitions (Ordered → Executed → Signed → Aggregated)          |
| `buffer.rs`                    | Hash-based ordered linked buffer data structure                                            |
| `commit_reliable_broadcast.rs` | Reliable broadcast for commit votes with retries                                           |
| `execution_schedule_phase.rs`  | Sends blocks to execution, creates futures                                                 |
| `execution_wait_phase.rs`      | Awaits execution future completion                                                         |
| `signing_phase.rs`             | Signs commit ledger info via safety rules                                                  |
| `persisting_phase.rs`          | Finalizes committed state to ledger DB                                                     |
| `pipeline_phase.rs`            | `StatelessPipeline` trait and task counting                                                |
| `execution_client.rs`          | `ExecutionClient` interface and `BufferManagerHandle`                                      |
| `decoupled_execution_utils.rs` | Channel wiring between phases                                                              |
| `errors.rs`                    | Pipeline error types                                                                       |

## Testing

```bash
cargo test -p aptos-consensus -- pipeline
```
