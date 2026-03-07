# Quorum Store

Quorum Store (QS) is a data dissemination layer based on [Narwhal](https://arxiv.org/abs/2105.11827). It separates transaction dissemination from consensus ordering, enabling all validators to broadcast transactions concurrently rather than relying solely on the leader. See [AIP-26](https://github.com/aptos-foundation/AIPs/blob/main/aips/aip-26.md) for the original design and [AIP-106](https://github.com/aptos-foundation/AIPs/blob/main/aips/aip-106.md) for Optimistic Quorum Store.

## Overview

Without Quorum Store, the consensus leader is the bottleneck — it must pull transactions from mempool and broadcast them to all validators in the proposal. With Quorum Store, every validator continuously creates **batches** of transactions and broadcasts them to all peers. When 2f+1 validators sign a batch, a **Proof of Store** (PoS) is formed, certifying the batch is available. The leader then proposes blocks containing only batch references (PoS), not raw transactions.

This maximizes network bandwidth utilization across all validators and reduces proposal size.

### Optimistic Quorum Store

[AIP-106](https://github.com/aptos-foundation/AIPs/blob/main/aips/aip-106.md) introduces Optimistic Quorum Store. In the standard flow, the leader must wait for 2f+1 signatures to form a Proof of Store before including a batch in a proposal. Optimistic QS removes this wait — the leader proposes **batch summaries without proofs**. Since batch authors broadcast batch data to all validators, by the triangle inequality of the network, the batch data likely arrives at validators by the time the leader's proposal arrives. If a validator does not have a batch locally, it fetches it from the batch author or PoS signers via `BatchRequester`. Batches that never receive a proof are garbage-collected on expiration.

The proposal payload (`OptQuorumStorePayload`) contains three tiers:
1. **Proof batches** — batches with full PoS (guaranteed available from signers)
2. **Optimistic batches** — batch summaries without proofs (fetched on demand if missing)
3. **Inline batches** — full transactions included directly (used when proof queue is fully utilized)

## Protocol

### Batch Lifecycle

**1. Batch Creation (BatchGenerator)**

- Pulls transactions from mempool at a configurable interval
- Sorts by gas price and buckets into batches respecting size limits (`sender_max_batch_txns`, `sender_max_batch_bytes`)
- Assigns a monotonically increasing `BatchId`
- Persists the batch to local DB
- Broadcasts `BatchMsg` to all validators

**2. Batch Reception (BatchCoordinator)**

- Receives remote `BatchMsg` from network
- Validates size limits and transaction filters
- Persists to `BatchStore`
- Signs the batch metadata with validator key, producing a `SignedBatchInfo`
- Sends `SignedBatchInfo` back to the batch author

**3. Proof Aggregation (ProofCoordinator)**

- Collects `SignedBatchInfo` from validators
- Aggregates signatures using `SignatureAggregator`
- When 2f+1 signatures are collected, forms a `ProofOfStore` (aggregate BLS signature over batch metadata)
- Broadcasts the `ProofOfStore` to all validators

**4. Proof Queuing (ProofManager)**

- Receives `ProofOfStore` and inserts into `BatchProofQueue`
- Queue is sorted by author and gas bucket for fair, prioritized pulling
- Deduplicates transactions across batches using `TxnSummary` occurrence tracking

### Consensus Integration

When the leader needs to create a proposal:

1. `ProposalGenerator` sends a `GetPayloadCommand` to `ProofManager`
2. `ProofManager` pulls from `BatchProofQueue`:
   - **Proof batches**: batches with full `ProofOfStore` (guaranteed available)
   - **Optimistic batches**: batch summaries without proofs (OptQS, lower latency)
   - **Inline batches**: full transactions when queue has capacity
3. Returns `OptQuorumStorePayload` containing all three types
4. If a validator lacks a batch during execution, it fetches from peers using the PoS signatures

### Backpressure

`ProofManager` tracks `remaining_total_txn_num` and `remaining_total_proof_num`. When either exceeds configured thresholds, it sends a backpressure signal to `BatchGenerator` (sampled every 200ms to avoid constant updates). `BatchGenerator` responds with:
- **Multiplicative decrease** (×0.9) on pull rate when backpressured
- **Additive increase** (+delta) when backpressure clears

### Batch V2

Batch V2 adds support for transaction classification (`BatchKind`: Normal or Encrypted), with separate creation and network paths controlled by `enable_batch_v2_tx`.

## Implementation

### QuorumStoreCoordinator

The top-level orchestrator runs a main loop processing `CoordinatorCommand` messages:

- **`CommitNotification(block_timestamp, batches)`** — fans out to `ProofCoordinator`, `ProofManager`, and `BatchGenerator` so each can garbage-collect committed batches and update timestamps.
- **`Shutdown(ack_tx)`** — orchestrates graceful shutdown in reverse pipeline order (network listener → proof manager → proof coordinator → batch coordinator → batch generator) to ensure senders close before receivers.

### BatchProofQueue

The core data structure for managing proofs and batches with fairness and deduplication.

**Sorting**: Batches are keyed by `(gas_bucket_start ASC, batch_id DESC)` — higher gas buckets are pulled first, and within a bucket, newer batches take priority.

**Per-peer fairness**: Each author has a separate `BTreeMap`, so no single validator can monopolize the queue.

**Deduplication**: Tracks transaction summaries `(sender, replay_protector, hash, expiration)` with occurrence counts across all batches. When pulling, transactions already seen in earlier batches are counted as duplicates and subtracted from the effective batch size.

**Expiration**: A `TimeExpirations` binary heap tracks batch lifetimes. When `handle_updated_block_timestamp` is called, expired batches are removed and their transaction occurrence counts decremented.

### BatchStore

Persistent batch storage with per-peer quotas to prevent any single validator from monopolizing resources.

**Quota system**: Each peer has a `QuotaManager` tracking `db_balance`, `memory_balance`, and `batch_balance`. Based on available quota, batches are stored in one of two modes:
- `MemoryAndPersisted` — both in-memory cache and DB (preferred)
- `PersistedOnly` — only in database, metadata in cache (when memory quota exhausted)
- Rejected if both quotas are exhausted

**Concurrency**: Uses `DashMap` for lock-free concurrent access. Lock ordering (cache entry before peer quota) prevents deadlocks.

**Subscriber mechanism**: When a batch is needed but not yet available, callers register via `subscribe()`. When the batch arrives, all subscribers are notified via oneshot channels. This handles races between batch fetching and batch reception.

**Bootstrap**: On startup, loads previous epoch batches from DB, garbage-collects expired ones, and repopulates the in-memory cache.

### BatchRequester

Fetches missing batches from the network when a validator needs a batch it doesn't have locally.

**Request strategy**: Starts with a random peer from the batch's PoS signers, then cycles through validators on retries (up to `retry_limit`). Sends to `request_num_peers` validators concurrently.

**Completion conditions**: The request completes when either:
- A valid `BatchResponse` arrives from a peer
- The batch is received via subscription (another code path fetched it)
- The batch has expired (ledger info timestamp > batch expiration)

## Architecture

```
                    ┌──────────────────────────────────────────┐
                    │         QuorumStoreCoordinator           │
                    │  (dispatches commands to all components) │
                    └──────┬──────┬──────┬──────┬──────────────┘
                           │      │      │      │
              ┌────────────▼┐  ┌──▼──────▼──┐  ┌▼──────────────┐
              │   Batch     │  │   Proof    │  │    Proof      │
  Mempool ──► │  Generator  │  │ Coordinator│  │   Manager     │◄── GetPayloadCommand
              │  (create +  │  │ (aggregate │  │  (queue +     │       (from consensus)
              │   broadcast)│  │  signatures│  │   pull proofs)│
              └─────────────┘  │  → PoS)    │  └───────────────┘
                               └────────────┘
              ┌─────────────┐
  Network ──► │   Batch     │  ┌────────────┐
              │ Coordinator │  │  Batch     │
              │ (receive +  │  │  Store     │
              │  sign +     ├─►│ (persist + │
              │  persist)   │  │  cache)    │
              └─────────────┘  └────────────┘

              ┌─────────────┐
  Network ──► │  Network    │  Routes: BatchMsg → BatchCoordinator
              │  Listener   │          SignedBatchInfo → ProofCoordinator
              └─────────────┘          ProofOfStoreMsg → ProofManager
```

### Message Types

| Message           | Purpose                              | Sender → Receiver           |
| ----------------- | ------------------------------------ | --------------------------- |
| `BatchMsg`        | Batch of transactions                | Creator → all validators    |
| `SignedBatchInfo` | Validator signature on batch         | Receiver → batch author     |
| `ProofOfStoreMsg` | Aggregated proof (2f+1 signatures)   | Author → all validators     |
| `BatchRequest`    | Request missing batch by digest      | Any validator → PoS signers |
| `BatchResponse`   | Batch payload or NotFound            | Signer → requester          |

## Key Files

| File                          | Purpose                                                        |
| ----------------------------- | -------------------------------------------------------------- |
| `quorum_store_coordinator.rs` | Top-level coordinator, dispatches commands and shutdown        |
| `batch_generator.rs`          | Pulls from mempool, creates and broadcasts batches             |
| `batch_coordinator.rs`        | Receives remote batches, validates, persists, signs            |
| `proof_coordinator.rs`        | Aggregates signatures into ProofOfStore                        |
| `proof_manager.rs`            | Manages proof queue, serves proposals, backpressure            |
| `batch_proof_queue.rs`        | Core queue: sorted by gas bucket, per-peer fairness, dedup     |
| `batch_store.rs`              | In-memory cache + DB persistence with per-peer quotas          |
| `batch_requester.rs`          | Fetches missing batches from PoS signers with retries          |
| `network_listener.rs`         | Routes network messages to handlers                            |
| `types.rs`                    | Core types: Batch, BatchMsg, BatchRequest, etc.                |
| `quorum_store_db.rs`          | Database layer for batch persistence                           |

## Testing

```bash
cargo test -p aptos-consensus -- quorum_store
```
