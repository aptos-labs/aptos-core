# Implementation Plan: Multi-Slot Prefix Consensus (Algorithm 4)

## Goal

Replace the current leader-based RoundManager (Jolteon/AptosBFT) with a leaderless SlotManager that runs one Strong Prefix Consensus instance per slot, producing ordered blocks that flow through the existing execution pipeline to create a fully functioning blockchain.

## Background: Algorithm 4 from Paper (Section 7)

```
Algorithm 4: Multi-slot Consensus Protocol for party i

Local variables:
  s_i: current slot number, initialized to 0
  B_{i,s}[·]: length-n proposal buffer for slot s, initialized to all ⊥

function Update(rank^MC, v):
  let rank^MC = (p1,...,pn), ℓ := |v|
  if ℓ = n then return rank^MC
  return (p1,...,p_ℓ, p_{ℓ+2},...,pn, p_{ℓ+1})  // demote first excluded

upon protocol start:
  NewSlot(1)

upon NewSlot(s):
  broadcast proposal = (s, b_{i,s})     // b_{i,s} is party i's input for slot s
  start timer(s) for 2Δ

upon receiving proposal = (s, b_{p,s}) from party p:
  B_{i,s}[p] := b_{p,s}
  if B_{i,s}[j] ≠ ⊥ for all j ∈ [n]:
    RunSPC(s)

upon timer(s) expires:
  RunSPC(s)

upon RunSPC(s):
  rank^MC := Update(rank^MC_{s-1}, v^high_{s-1})
  v^in := [H(B_{i,s}[p1]),...,H(B_{i,s}[pn])]  // ordered by rank^MC
  run SPC with input v^in and ranking rank^MC

upon SPC outputs v^low:
  Commit(v^low)

upon SPC outputs v^high:
  Commit(v^high)
  NewSlot(s+1)

upon Commit(v):
  for k = 1,...,|v|:
    if v[k] ≠ H(⊥):
      fetch b_k if needed
      commit b_k if not already committed
```

## Key Design Decisions

### 1. What is a "proposal" (b_{i,s})?

A proposal is a **Payload** — a set of transactions that a validator pulls from the mempool for this slot. Each validator creates one proposal per slot containing their view of pending transactions. This maps to the existing `Payload` type in Aptos.

For the initial prototype, we use **DirectMempool** payloads (transactions inlined in the proposal). This avoids the QuorumStore complexity while keeping the system functional end-to-end. QuorumStore integration can follow as an optimization.

### 2. What is a "block" in the output?

Each slot produces one **Block** containing the concatenation of committed proposals in ranking order. The Block uses a new `BlockType::PrefixConsensusBlock` variant (following the precedent of `BlockType::DAGBlock` which also aggregates multiple proposers).

### 3. How does execution work?

Following the DAG pattern exactly:
- Construct a `Block` from committed proposals
- Wrap in `PipelinedBlock`
- Create `OrderedBlocks` with an ordering proof
- Send to execution via `UnboundedSender<OrderedBlocks>` channel
- DAG uses `AggregateSignature::empty()` for the ordering proof — we do the same (the SPC proof is the actual guarantee, not a traditional QC)

### 4. v_low ignored — blocks created from v_high only

For this initial implementation, **v_low is completely ignored**. The block for each slot is created solely from v_high, which is the agreed-upon output of the Strong Prefix Consensus instance. This simplifies the design:
- One commit per slot (when v_high arrives)
- No partial commits, no double-commit tracking
- No concurrent v_low/v_high handling

v_low fast commit (including the full-prefix special case) is deferred to future work (see TODO section).

### 5. Ranking updates across slots

`Update(rank^MC_{s-1}, v^high_{s-1})`: If v_high from previous slot has fewer than n entries (some proposal was censored/excluded), the first excluded party (at position `ℓ+1` in the ranking) is moved to the end. This enforces f-censorship resistance over time.

### 6. Config-gated via local NodeConfig (prototype)

Add `enable_prefix_consensus: bool` to `ConsensusConfig` in `config/src/config/consensus_config.rs` (the local node config, NOT the on-chain `ConsensusAlgorithmConfig` in `types/src/on_chain_config/consensus_config.rs`). This avoids a breaking serialization change to the on-chain config enum (`ConsensusAlgorithmConfig` has exactly 3 variants: Jolteon, JolteonV2, DAG — adding a 4th would break every match site and on-chain governance). The EpochManager checks this local flag to decide whether to start SlotManager or RoundManager. On-chain config migration is future work for production deployment.

## Architecture Overview

```
┌─────────────────────────────────────────────────────────┐
│ EpochManager                                             │
│  if is_prefix_consensus_enabled():                       │
│    start_slot_manager()     // NEW (replaces RoundMgr)   │
│  else if is_dag_enabled():                               │
│    start_new_epoch_with_dag()                            │
│  else:                                                   │
│    start_new_epoch_with_jolteon()                        │
└───────────────┬─────────────────────────────────────────┘
                │
┌───────────────▼─────────────────────────────────────────┐
│ SlotManager (NEW)                                        │
│                                                          │
│  Per Slot s:                                             │
│  1. Pull payload from mempool → create proposal          │
│  2. Broadcast proposal to all validators                 │
│  3. Collect proposals from others (or timeout at 2Δ)     │
│  4. Form input vector ordered by rank^MC                 │
│  5. Run StrongPrefixConsensusManager on hash vector      │
│  6. On v_high output → create Block, send to execution   │
│  7. Update ranking, advance to slot s+1                  │
│                                                          │
│  Components:                                             │
│  - ProposalBuffer: B_{i,s}[p] per slot per party         │
│  - RankingManager: rank^MC across slots (with Update)    │
│  - SlotState: per-slot state machine                     │
│  - BlockBuilder: assembles Block from committed proposals│
│  - ExecutionBridge: sends to execution pipeline          │
└───────────────┬─────────────────────────────────────────┘
                │
                │ UnboundedSender<OrderedBlocks>
                ▼
┌─────────────────────────────────────────────────────────┐
│ Existing Execution Pipeline (unchanged)                  │
│  BufferManager → PipelineBuilder → BlockExecutor         │
│  → Storage → State Sync                                  │
└─────────────────────────────────────────────────────────┘
```

## Network Messages

```rust
enum SlotConsensusMsg {
    // Slot proposal: party's payload for a slot
    SlotProposal {
        slot: u64,
        epoch: u64,
        author: Author,
        payload: Payload,        // DirectMempool initially
        signature: bls12381::Signature,
    },

    // Wrapped SPC messages for a specific slot
    StrongPCMsg {
        slot: u64,
        msg: StrongPrefixConsensusMsg,
    },
}
```

Added as `ConsensusMsg::SlotConsensusMsg(Box<SlotConsensusMsg>)` variant.

## Implementation Steps

### Phase 1: Slot Proposal Types and Network Messages (~300 LOC) ✅

**New file**: `consensus/prefix-consensus/src/slot_types.rs`

- [x] `SlotProposal` struct: `{ slot, epoch, author, payload_hash, payload, signature }`
- [x] `SlotProposalSignData` for BLS signing (slot + epoch + author + payload_hash)
- [x] `SlotConsensusMsg` enum: `{ SlotProposal, StrongPCMsg { slot, epoch, msg } }`
- [x] Helper methods: `epoch()`, `slot()`, `author()`, `name()`
- [x] Serialization (serde + BCS), payload hash via `HashValue::sha3_256_of(&bcs::to_bytes(&payload))`
- [x] `verify()` checks payload hash integrity before BLS signature verification
- [x] Unit tests: sign/verify, wrong signer, serialization roundtrip, tampered payload, msg helpers

**Modified file**: `consensus/src/network_interface.rs`
- [x] `ConsensusMsg::SlotConsensusMsg(Box<SlotConsensusMsg>)` variant

**Modified file**: `consensus/src/network.rs`
- [x] `SlotConsensusMsg` added to DirectSend dispatch whitelist

### Phase 2: Multi-Slot Ranking Manager (~150 LOC) ✅

**New file**: `consensus/prefix-consensus/src/slot_ranking.rs`

- [x] `MultiSlotRankingManager` struct with `current_ranking: Vec<Author>`
- [x] `update(&mut self, committed_prefix_length: usize)` — implements `Update(rank^MC, v)` from Algorithm 4
  - If `committed_prefix_length == n`: ranking unchanged
  - If `committed_prefix_length < n`: move party at position ℓ+1 to end (where ℓ = committed_prefix_length)
- [x] `current_ranking(&self) -> &[Author]` — returns current ranking
- [x] `position_of(&self, author: &Author) -> Option<usize>` — lookup
- [x] `validator_count(&self) -> usize`
- [x] 7 unit tests: full prefix, first excluded, last excluded, multiple slots, zero prefix length, single validator, position_of

### Phase 3: Proposal Buffer and Slot State (~400 LOC) ✅

**New file**: `consensus/prefix-consensus/src/slot_state.rs`

- [x] `ProposalBuffer` struct
  - `proposals: HashMap<Author, SlotProposal>` — received proposals for this slot
  - `n: usize` — expected validator count
- [x] `ProposalBuffer::insert(&mut self, proposal: SlotProposal) -> bool`
  - Returns true if all n proposals now received
- [x] `ProposalBuffer::build_input_vector(&self, ranking: &[Author]) -> (PrefixVector, HashMap<HashValue, Payload>)`
  - Orders proposal hashes by ranking
  - Returns (hash vector for SPC, map from hash→payload for later commit)
  - Uses `H(⊥) = HashValue::zero()` for missing proposals (same as existing convention)
- [x] `SlotPhase` enum: `CollectingProposals | RunningSPC | Committed`
- [x] `SlotState` struct combining phase, buffer, timer state, SPC output
- [x] Unit tests (18 tests: buffer insert/complete, duplicate rejection, input vector ordering, payload map correctness, phase transitions, wrong-phase panics)

### Phase 4: Block Builder (~300 LOC) ✅

**New file**: `consensus/prefix-consensus/src/block_builder.rs`
**Modified files**: `consensus/consensus-types/src/block_data.rs`, `consensus/consensus-types/src/block.rs`

- [x] Add `BlockType::PrefixConsensusBlock` variant to `consensus-types/src/block_data.rs` (after DAGBlock, preserving serde indices, `#[serde(skip_deserializing)]`)
- [x] `BlockData::new_for_prefix_consensus(...)` constructor (dummy QC with `BlockInfo::empty()`, like DAGBlock)
- [x] Update all `BlockType` match arms: `author()`, `parent_id()`, `payload()`, `validator_txns()`, `failed_authors()`
- [x] `Block::new_for_prefix_consensus(...)` constructor (`signature: None`, `failed_authors: vec![]`)
- [x] `validate_signature()` arm: bails with "should not accept PrefixConsensus block from others"
- [x] `build_block_from_v_high(epoch, round, timestamp_usecs, ranking, v_high, payload_map, parent_block_id, validator_txns) -> Block`
  - Block author derived deterministically: first non-bottom author in v_high (highest-ranked committed validator), fallback to `ranking[0]`
  - No `slot` parameter — `round == slot` since each epoch starts with genesis at round 0
  - Missing payloads panic (payload resolution in Phase 7 must complete before block building)
- [x] 6 unit tests: all_non_bot, partial, empty_v_high, short_v_high, ordering, metadata
- [x] `build_ordering_proof` and `build_pipelined_block` deferred to Phase 5 (SlotManager) — `OrderedBlocks` lives in `aptos-consensus` crate

### Phase 5: SlotManager Core (~800 LOC)

**New file**: `consensus/prefix-consensus/src/slot_manager.rs`

The main orchestrator, equivalent to RoundManager but for multi-slot prefix consensus.

```rust
pub struct SlotManager<NS: StrongPrefixConsensusNetworkSender> {
    // Identity
    author: Author,
    epoch: u64,
    validator_signer: ValidatorSigner,
    validator_verifier: Arc<ValidatorVerifier>,

    // Slot state
    current_slot: u64,
    slot_states: HashMap<u64, SlotState>,
    ranking_manager: MultiSlotRankingManager,

    // Proposal management
    proposal_store: HashMap<u64, HashMap<Author, SlotProposal>>,

    // Per-slot SPC channels (task handle + message sender)
    // SPC runs as a separate tokio task; SlotManager forwards messages via spc_msg_tx
    // and receives v_high via spc_output_rx. v_low is ignored in this implementation.
    spc_msg_tx: Option<UnboundedSender<(Author, StrongPrefixConsensusMsg)>>,
    spc_close_tx: Option<oneshot::Sender<oneshot::Sender<()>>>,  // for graceful SPC shutdown
    spc_output_rx: Option<UnboundedReceiver<SPCOutput>>,

    // Execution bridge
    execution_channel: UnboundedSender<OrderedBlocks>,

    // Payload
    // payload_client: pulls transactions from mempool (used in start_new_slot to create proposals)
    payload_client: Arc<dyn PayloadClient>,
    // Note: payload_manager (Arc<dyn TPayloadManager>) is NOT on SlotManager — it's only
    // needed for execution_client.start_epoch() which runs in EpochManager before
    // SlotManager construction. No need to pass it through.

    // Block chain tracking
    // Initialized from last committed block at epoch start (from recovery data or genesis).
    // Same pattern as DAG's self.parent_block_info.read().id().
    parent_block_id: HashValue,
    parent_block_info: BlockInfo,  // full info needed for WrappedLedgerInfo construction
    // Slot→round mapping: round = highest_committed_round + slot.
    // Genesis is round 0. Slot 1 = highest_committed_round + 1, etc.
    // This ensures monotonically increasing rounds across epochs.
    highest_committed_round: u64,

    // Network
    network_sender: NS,
    self_sender: UnboundedSender<(Author, SlotConsensusMsg)>,

    // Timers
    proposal_timeout: Duration,  // 2Δ from paper, initially 300ms (SLOT_PROPOSAL_TIMEOUT)
}

/// Output from SPC task back to SlotManager (v_low ignored in this implementation)
struct SPCOutput {
    slot: u64,
    v_high: PrefixVector,
    committing_view: u64,  // forward-looking: needed for verifiable ranking (Phase 12)
    // Phase 13 adds: commit_proof: StrongPCCommit (embedded in next slot's proposals)
}
```

**Slot→Round Mapping**:
The execution pipeline requires monotonically increasing `round` numbers in `BlockInfo`. We map:
- `round = highest_committed_round + slot`
- `highest_committed_round` is obtained from recovery data at epoch start (same value passed to `execution_client.start_epoch()`)
- Slot 1 → round `highest_committed_round + 1`, Slot 2 → `highest_committed_round + 2`, etc.
- This matches DAG's approach where round comes from the anchor node's round

**Parent Block Initialization**:
At epoch start, `parent_block_id` and `parent_block_info` are initialized from recovery data:
- From `RecoveryData` → `root_block()` or last committed block
- Same as DAG's `OrderedNotifierAdapter` which initializes `parent_block_info` from the latest committed block
- Genesis block has round 0, so slot 1 correctly chains from it

**Timestamp Generation**:
Each block needs `timestamp_usecs`. Computed as:
- `timestamp = max(parent_block_info.timestamp_usecs + 1, current_wall_clock_usecs)`
- Same logic as DAG (`adapter.rs`): ensures monotonically increasing timestamps
- Current wall clock via `aptos_infallible::duration_since_epoch().as_micros()`

**Methods**:

- [ ] `new(author, epoch, validator_signer, validator_verifier, execution_channel, payload_client, network_sender, self_sender, initial_ranking, parent_block_info, highest_committed_round, proposal_timeout)` — constructor
- [ ] `start(event_rx, close_rx)` — main event loop (`tokio::select!`)
  - Handle incoming `SlotConsensusMsg` (proposals + SPC messages)
  - Handle SPC output (v_high via `spc_output_rx`; v_low is ignored)
  - Handle timer expiry
  - Handle close signal
- [ ] `start_new_slot(&mut self, slot: u64)`
  - Pull payload from mempool via `payload_client.pull_payload(params, validator_txn_filter)` where:
    - **Reference**: Follow DAG's `dag_driver.rs` lines 262-276 as template
    - `params`: `PayloadPullParameters` with all 11 required fields:
      ```rust
      PayloadPullParameters {
          max_poll_time: Duration::from_millis(300),  // match proposal timeout
          max_txns: PayloadTxnsSize::new(500, 1024 * 1024),  // 500 txns, 1MB
          max_txns_after_filtering: 500,
          soft_max_txns_after_filtering: 500,
          max_inline_txns: PayloadTxnsSize::new(100, 100 * 1024),  // same as DAG
          maybe_optqs_payload_pull_params: None,  // no OptQS for prototype
          user_txn_filter: PayloadFilter::Empty,  // no filtering for prototype
          pending_ordering: false,
          pending_uncommitted_blocks: 0,
          recent_max_fill_fraction: 0.0,
          block_timestamp: duration_since_epoch(),
      }
      ```
    - `validator_txn_filter`: `TransactionFilter::empty()` (no filtering for prototype)
    - Returns `(Vec<ValidatorTransaction>, Payload)` — store both; validator_txns are passed through to block construction (needed for epoch transitions — DKG/JWK are delivered as ValidatorTransactions)
  - Create `SlotProposal` and sign it (via `validator_signer.sign(&sign_data)`)
  - Broadcast proposal to all validators (via `network_sender` + `self_sender`)
  - Store own proposal in buffer
  - Start 300ms timer (`SLOT_PROPOSAL_TIMEOUT`)
  - Check if all proposals received (if so, run SPC immediately)
- [ ] `process_proposal(&mut self, author: Author, proposal: SlotProposal)`
  - Verify signature: `validator_verifier.verify(proposal.author, &SlotProposalSignData::from(&proposal), &proposal.signature)`
  - Verify epoch matches, slot matches current or future slot
  - Store in proposal buffer for the proposal's slot
  - If all proposals received for current slot AND SPC not yet started: cancel timer, run SPC
- [ ] `on_timer_expired(&mut self, slot: u64)`
  - If SPC not yet started for this slot: run SPC with whatever proposals we have
- [ ] `run_spc(&mut self, slot: u64)`
  - Build input vector from proposal buffer ordered by `ranking_manager.current_ranking()`
  - **Pass `ranking_manager.current_ranking()` as initial_ranking to SPC** (this is how the per-slot cyclic view rotation inside SPC gets its base ordering from the cross-slot demotion ranking)
  - Create SPC output channel: `(spc_output_tx, spc_output_rx)` — only for v_high
  - Create SPC message channel: `(spc_msg_tx, spc_msg_rx)`
  - Create SPC close channel: `(spc_close_tx, spc_close_rx)` — for graceful shutdown
  - Construct `DefaultStrongPCManager` with input vector, initial_ranking, and output_tx
  - Spawn SPC as tokio task: `tokio::spawn(spc_manager.run(spc_msg_rx, spc_close_rx))`
  - Store `spc_msg_tx`, `spc_close_tx`, and `spc_output_rx`
- [ ] `on_spc_v_high(&mut self, slot: u64, v_high: PrefixVector)`
  - `commit_vector(v_high)` — single commit call per slot (no v_low, no double-commit)
  - Count non-⊥ entries in v_high → pass count to `ranking_manager.update(committed_prefix_length)`
  - Clean up SPC state (see Phase 6 "Normal completion" lifecycle):
    - `self.spc_msg_tx.take()` — drops sender, closing channel → SPC task's `run()` loop exits
    - `self.spc_close_tx.take()` — drop unused close channel
    - `self.spc_output_rx.take()` — drop receiver
  - Clean up per-slot state to prevent unbounded memory growth:
    - `self.slot_states.remove(&slot)`
    - `self.proposal_store.remove(&slot)`
  - If `commit_vector()` returned a reconfig flag, do NOT advance — let epoch manager handle transition
  - Otherwise advance to slot s+1 via `start_new_slot(slot + 1)`
- [ ] `commit_vector(&mut self, slot: u64, v: &PrefixVector) -> bool` (returns `true` if reconfig block)
  - For each non-⊥ entry in v: look up payload in proposal_store
  - Compute timestamp: `max(parent_block_info.timestamp_usecs + 1, now_usecs)`
  - Compute round: `highest_committed_round + slot`
  - Build Block via `block_builder::build_block_from_commit(...)` with round and timestamp
  - Wrap in PipelinedBlock
  - Build ordering proof (following DAG: `LedgerInfoWithSignatures::new(ledger_info, AggregateSignature::empty())`)
  - Send `OrderedBlocks` to execution channel
  - Update `parent_block_id` and `parent_block_info`
  - Check `block.is_reconfiguration_suffix()`: if true, return `true` — caller must NOT advance to next slot (epoch manager will shut down SlotManager and start a new epoch)
- [ ] `process_spc_message(&mut self, slot: u64, msg: StrongPrefixConsensusMsg)`
  - If `slot == current_slot` and `spc_msg_tx.is_some()`: forward via `spc_msg_tx.send((author, msg))`
  - If `slot < current_slot`: drop with debug log (stale slot, SPC task already exited)
  - If `slot > current_slot`: buffer for future (or drop with warning — shouldn't happen in sequential model)

### Phase 6: SPC Integration Refactor (~300 LOC)

The current `StrongPrefixConsensusManager` runs as a standalone task with its own event loop. For multi-slot, we keep this task-based model — the SlotManager spawns one SPC task per slot and communicates exclusively via channels.

**Design choice (resolved)**: **Task-based SPC with channels.** The SlotManager never calls `process_message()` directly. Instead:
- **SlotManager → SPC**: Forward `StrongPrefixConsensusMsg` via the SPC's existing `message_rx` channel (the same `UnboundedReceiver` that `run()` listens on)
- **SPC → SlotManager**: New `output_tx: UnboundedSender<SPCOutput>` channel for v_high notification only (v_low is ignored)
- `process_message()` stays private inside the SPC task
- `run(mut self, ...)` continues to consume self — this is fine since the SlotManager only holds the channel sender, not the manager object

**Per-slot SPC lifecycle** (detailed channel management):

1. **Creation** (`run_spc(slot)`):
   - Create `(spc_msg_tx, spc_msg_rx)` = `mpsc::unbounded_channel::<(Author, StrongPrefixConsensusMsg)>()`
   - Create `(spc_output_tx, spc_output_rx)` = `mpsc::unbounded_channel::<SPCOutput>()`
   - Create `(spc_close_tx, spc_close_rx)` = `oneshot::channel::<oneshot::Sender<()>>()`
   - Construct `DefaultStrongPCManager::new(..., Some(spc_output_tx))`
   - Spawn: `tokio::spawn(spc_manager.run(spc_msg_rx, spc_close_rx))`
   - Store in SlotManager fields: `self.spc_msg_tx = Some(spc_msg_tx)`, `self.spc_close_tx = Some(spc_close_tx)`, `self.spc_output_rx = Some(spc_output_rx)`

2. **During slot** (main event loop):
   - Incoming SPC messages → `self.spc_msg_tx.as_ref().unwrap().send((author, msg))`
   - Poll `self.spc_output_rx` in `tokio::select!` for v_high output

3. **Normal completion** (`on_spc_v_high(slot, v_high)`):
   - Process v_high (build block, send to execution)
   - Drop all handles: `self.spc_msg_tx.take()`, `self.spc_close_tx.take()`, `self.spc_output_rx.take()`
   - When `spc_msg_tx` is dropped, the `spc_msg_rx` channel closes → SPC's `run()` loop receives `None` from `message_rx.recv()` and exits naturally
   - The SPC tokio task completes and is cleaned up by the runtime

4. **Epoch shutdown** (SlotManager receives close signal):
   - If SPC is running (`spc_close_tx.is_some()`):
     - Create ack channel: `let (ack_tx, ack_rx) = oneshot::channel()`
     - Send close: `self.spc_close_tx.take().unwrap().send(ack_tx)`
     - SPC's `run()` receives close signal, breaks its loop, sends `ack_tx.send(())`
     - SlotManager awaits `ack_rx.await` for graceful confirmation
   - Drop remaining handles: `self.spc_msg_tx.take()`, `self.spc_output_rx.take()`

5. **Edge case — SPC task panics**:
   - `spc_msg_tx.send()` returns `Err` → SPC task has exited unexpectedly
   - Log error, treat as if v_high was empty (advance to next slot with ranking demotion)
   - Or: restart SPC for the same slot (retry logic — deferred to future work)

**Modified file**: `consensus/prefix-consensus/src/strong_manager.rs`

- [ ] Add `output_tx: Option<UnboundedSender<SPCOutput>>` field to `StrongPrefixConsensusManager`
  - Passed via constructor: `new(..., output_tx: Option<UnboundedSender<SPCOutput>>)`
  - When `None`, behavior unchanged (standalone mode for smoke tests)
  - When `Some`, fire callback on v_high commit (in addition to writing output files if configured)
- [ ] **v_high notification**: Must be added to BOTH commit code paths in `strong_manager.rs`:
  1. **Local commit path**: Party builds its own `StrongPCCommit` (in `finalize_view()` when v_low has non-⊥ entry → trace back → broadcast commit). Add `output_tx.send()` after `set_committed(v_high)`.
  2. **Received commit path**: Party receives and validates a `StrongPCCommit` from another party (in `process_message()` → `StrongPCCommit` arm → `verify()` succeeds → `set_committed(v_high)`). Add `output_tx.send()` here too.
  - Both paths: `if let Some(tx) = &self.output_tx { tx.send(SPCOutput { slot, v_high, committing_view }).ok(); }`
  - `committing_view` is the view number where v_low had a non-⊥ entry (available at both commit sites). Forward-looking for Phase 13 verifiable ranking.
  - No v_low notification needed (v_low is ignored in this implementation)
- [ ] Ensure SPC task exits cleanly after v_high is output: the `run()` loop should break after commit, or respond to close_rx signal
- [ ] Confirm slot field is already in signed vote data (Vote1/2/3 SignData includes `slot: u64` — added in Phase 3 of Strong PC). Cross-slot vote replay is already prevented.
- [ ] Unit test: verify output_tx receives v_high after SPC completes

### Phase 7: Payload Resolution — Late Buffering + Fetch Protocol (~350 LOC) ✅

**Detailed plan**: `.plans/phase7-payload-resolution.md`

Ensures the SlotManager can always build a block from v_high, even when v_high contains hashes for proposals that were not received before the 2Δ timer. Implements the paper's `fetch b_k if needed` clause from Algorithm 4's `Commit(v)` function.

**Problem**: SPC's v_high can include hashes at positions where our input had ⊥ (because `min_common_extension` in Round 3 extends past our local prefix using other parties' certified longer prefixes). The `payload_map` from `build_input_vector()` only contains payloads for proposals we received before the timer — it won't have entries for these extended hashes.

**Solution** (two layers):

1. **Late proposal buffering** (common case): Late proposals arriving during `RunningSPC` are inserted directly into `payload_map` on `SlotState` (simplified from original plan's separate `late_proposals` buffer — the payload needs to end up in `payload_map` anyway). When v_high arrives, `resolve_missing_payloads()` checks the unified map. Under near-synchronous conditions, most missing proposals arrive during SPC's multi-round execution, so this handles the common case with zero network overhead.

2. **Payload fetch protocol** (async safety net): For payloads still missing after checking the payload map, broadcast `PayloadFetchRequest` to all peers by `payload_hash`. Store a `PendingCommit` while waiting. Responses are verified via `H(payload) == payload_hash`. Late proposals arriving during `PendingCommit` also check for resolution.

**Modified files**:
- [x] `slot_types.rs`: Added `PayloadFetchRequest`, `PayloadFetchResponse` types with `verify_payload_hash()`; added variants to `SlotConsensusMsg`; made `compute_payload_hash` public
- [x] `slot_state.rs`: Changed `insert_proposal()` from `Result<bool>` to `bool` — buffers late proposals into `payload_map` during `RunningSPC`, silently ignores during `Committed`; added `resolve_missing_payloads()` and `lookup_payload()` methods
- [x] `slot_manager.rs`: Added `PendingCommit` struct; refactored `on_spc_v_high()` into resolution + `build_and_commit_block()` helper; added `process_payload_fetch_request/response()` and `try_resolve_pending()` handlers
- [x] `lib.rs`: Re-exported `PayloadFetchRequest`, `PayloadFetchResponse`

**Key design decisions**:
- Fetch messages are `SlotConsensusMsg` variants (not `StrongPrefixConsensusMsg`) — payload fetch is a slot-level concern, not SPC's
- Wait indefinitely for missing payloads (no graceful degradation) — slot catch-up mechanism deferred to post-integration
- Payload integrity: verify `H(payload) == payload_hash` on fetch response (hash was committed to by SPC)
- TODO(production): check `pending_commit.missing.contains()` before computing hash to avoid unsolicited response amplification

- [x] Unit tests: 11 new tests for late buffering, resolution, lookup, fetch message serialization, hash verification

### Phase 8: EpochManager Integration (~400 LOC) ✅

**Detailed plan**: `.plans/phase8-epoch-manager-integration.md`

Wires SlotManager into EpochManager so it starts automatically at epoch boundaries when `enable_prefix_consensus` config flag is set.

**Pre-step: Channel type alignment** — Changed SlotManager from tokio channels to futures channels to match the execution pipeline and codebase convention:
- [x] `execution_channel`: `tokio::sync::mpsc::UnboundedSender` → `futures::channel::mpsc::UnboundedSender`, `.send()` → `.unbounded_send()`
- [x] `message_rx`: `tokio::sync::mpsc::UnboundedReceiver` → `aptos_channels::UnboundedReceiver`, `recv()` → `next()` (Stream)
- [x] `close_rx`: unified on `futures::channel::oneshot`
- [x] SPC output channel: kept as tokio (internal to SlotManager, doesn't cross EpochManager boundary)
- [x] Tests updated: `futures_mpsc::unbounded()`, `try_next().unwrap()`, `aptos_channels::new_unbounded_test()`

**Modified files**:
- [x] `config/src/config/consensus_config.rs`: Added `enable_prefix_consensus: bool` (default `false`, `#[serde(default)]`)
- [x] `consensus/src/prefix_consensus/slot_manager.rs`: Channel type migration (tokio → futures)
- [x] `consensus/src/epoch_manager.rs`:
  - Added `slot_manager_tx` and `slot_manager_close_tx` fields
  - Added `start_new_epoch_with_slot_manager()` following DAG startup pattern: creates `DagCommitSigner`, calls `execution_client.start_epoch()`, constructs `SlotConsensusNetworkBridge` → `SlotNetworkSenderAdapter`, `RealSPCSpawner`, `MultiSlotRankingManager`, spawns SlotManager
  - `enable_prefix_consensus` checked FIRST in `start_new_epoch()` (before DAG/Jolteon on-chain config)
  - `SlotConsensusMsg` routing added to `check_epoch()` with epoch verification
  - Shutdown with timeout added to `shutdown_current_processor()` (cascades to SPC tasks)

**Key design decisions**:
- Reuses `DagCommitSigner` for the execution pipeline's commit signer (no SafetyRules needed)
- Skips `RecoveryData` — reads `parent_block_info` from `get_latest_ledger_info()` directly (same as DAG)
- Local config flag takes priority over on-chain config for prototype
- `SlotConsensusNetworkBridge` type alias already existed (network_interface.rs:426)

### Phase 9: BlockType Integration and Payload (~400 LOC, revised up from ~200)

Adding a new `BlockType` variant touches more match sites than initially estimated. The `BlockType` enum is matched exhaustively in many places.

**Modified file**: `consensus/consensus-types/src/block_data.rs`
- [ ] Add `PrefixConsensusBlock` variant to `BlockType` enum (with `#[serde(skip_deserializing)]` like DAGBlock)
- [ ] Update ALL match arms on `BlockType` throughout `block_data.rs`:
  - `author()` (line ~138) → return `Some(author)` for new variant
  - `parent_id()` (line ~157) → **CRITICAL**: This uses `if let DAGBlock { parent_block_id, .. }` with an else fallback to `self.quorum_cert.certified_block().id()`. Since PrefixConsensusBlock uses a dummy QuorumCert (BlockInfo::empty()), the fallback would return HashValue::zero(), silently breaking block chain integrity. Must extend the if-let to also match `BlockType::PrefixConsensusBlock { parent_block_id, .. }`
  - `payload()` (line ~168) → Add PrefixConsensusBlock to the existing `Proposal | DAGBlock` combined arm that returns `Some(payload)`. Without this, PrefixConsensusBlock falls through to `_ => None` and the block appears to have no payload
  - `validator_txns()` (line ~178) → return `Some(&validator_txns)` for new variant
  - `failed_authors()` (line ~225) → return `Some(&failed_authors)` for new variant
  - `is_reconfiguration_suffix()` → return false (not a nil/genesis block)
  - Any other exhaustive matches (grep for `BlockType::` in `block_data.rs`)

**Modified file**: `consensus/consensus-types/src/block.rs`
- [ ] Add `new_for_prefix_consensus(epoch, round, timestamp_usecs, author, failed_authors, validator_txns, payload, authors, slot, proposal_hashes, parent_block_id) -> Self` constructor (following `new_for_dag()` pattern)
- [ ] Update `verify_well_formed()` for new variant
- [ ] Grep for ALL `BlockType::` matches across the `consensus-types` crate and add arms

**Modified files**: Various files matching on `BlockType` (grep-driven)
- [ ] Run `grep -r "BlockType::" consensus/` to find ALL match sites
- [ ] Add `PrefixConsensusBlock` arm to each — most can mirror the `DAGBlock` arm
- [ ] Key sites likely include: `block_data.rs`, `block.rs`, `pipelined_block.rs`, payload extraction, metrics, logging

**Modified file**: `consensus/src/payload_manager/` (if needed)
- [ ] `DirectMempoolPayloadManager::get_transactions()` should already handle `Payload::DirectMempool` regardless of `BlockType` (it matches on `Payload`, not `BlockType`)
- [ ] Verify `notify_commit()` works correctly

### Phase 10: Execution Pipeline Compatibility (~300 LOC, revised up from ~200)

The execution pipeline may have assumptions that need updating. While DAG blocks prove the pipeline handles non-Jolteon blocks, there may be assertions or checks specific to existing block types.

**Approach**: Grep-driven audit of all `BlockType` matches in execution and pipeline code.

**Modified files** (as needed based on grep):
- [ ] `consensus/src/pipeline/pipeline_builder.rs` — check for `BlockType` matches
- [ ] `consensus/src/pipeline/buffer_manager.rs` — check for `BlockType` matches
- [ ] `consensus/src/block_storage/` — check for `BlockType`-specific logic
- [ ] `execution/executor/src/` — verify executor works with any valid `Payload`
- [ ] Check for `QuorumCert` assumptions: our blocks use `QuorumCert::new(VoteData::new(BlockInfo::empty(), BlockInfo::empty()))` like DAG — verify nothing inspects these QCs expecting real data
- [ ] Check for `failed_authors` expectations: our variant has an empty `failed_authors` vec

The goal is for PrefixConsensusBlock to piggyback on DAGBlock's existing path through the pipeline wherever possible. Most pipeline code operates on `Block`/`PipelinedBlock` generically without inspecting `BlockType`.

### Phase 11: Smoke Tests (~400 LOC)

**New files**: `testsuite/smoke-test/src/consensus/slot_consensus/`

- [ ] Test infrastructure:
  - `helpers.rs`: Wait for slot outputs, verify block commits, cleanup
  - Configure swarm with `enable_prefix_consensus: true`

- [ ] `test_slot_consensus_identical_proposals` (4 validators, all propose same txns)
  - All proposals identical → v_low = v_high = full vector
  - Single slot should produce a block with all 4 proposals
  - Block should commit to execution (verify via REST API or storage)

- [ ] `test_slot_consensus_multiple_slots` (4 validators, 3+ slots)
  - Run for multiple slots
  - Verify blocks committed sequentially
  - Verify ranking updates across slots

- [ ] `test_slot_consensus_end_to_end` (full transaction lifecycle)
  - Submit transactions via REST API
  - Verify transactions appear in committed blocks
  - Verify execution state advances (version increases)
  - This is the "blockchain works end-to-end" test

### Phase 12: Documentation and Cleanup

- [ ] Update `project_context.md` with multi-slot architecture
- [ ] Clean up any unused imports/code
- [ ] Run full clippy + fmt
- [ ] Verify all tests pass

### Phase 13: Verifiable Ranking with SPC-Aware Demotion (~300 LOC)

**Detailed plan**: `.plans/phase12-verifiable-ranking.md`

Upgrades the simple committed_prefix_length-based ranking to a verifiable, SPC-aware ranking that demotes the first-ranked party in every view that did not produce a v_low commit. Proposals embed the previous slot's commit proof for verification.

- [ ] Add `prev_commit_proof: Option<StrongPCCommit>` to `SlotProposal` (None for slot 1)
- [ ] Update `SlotProposal::verify()` to validate embedded commit proof
- [ ] Upgrade `MultiSlotRankingManager::update()` to take `committing_view` and demote first-ranked parties in views 1..W-1
- [ ] SlotManager: store last `StrongPCCommit`, embed in next slot's proposals
- [ ] SlotManager: on v_high commit, extract canonical proof from first non-⊥ proposal, derive ranking update
- [ ] Unit tests for SPC-aware demotion, commit proof embedding, and verification

## Resolved Design Decisions

1. **BlockType**: New `PrefixConsensusBlock` variant (not reusing DAGBlock). Follow DAGBlock's implementation patterns closely (study `Block::new_for_dag()`, `BlockType::DAGBlock` field structure, how DAG constructs `OrderedBlocks` and `WrappedLedgerInfo`), but create a distinct variant since the block structure differs (no node_digests, no parents_bitvec, different semantics).

2. **QuorumStore**: Deferred. Use DirectMempool for prototype. QuorumStore integration is a separate future phase.

3. **2Δ timeout**: Hardcoded at **300ms** initially (`SLOT_PROPOSAL_TIMEOUT`), matching SPC's `VIEW_START_TIMEOUT` pattern.
   - **TODO**: Make `SLOT_PROPOSAL_TIMEOUT` configurable via consensus config or constructor parameter.

4. **SPC per slot**: Separate tokio task per slot. SlotManager communicates exclusively via channels:
   - SlotManager → SPC: messages forwarded to SPC's `message_rx` channel
   - SPC → SlotManager: v_high only via new `output_tx: UnboundedSender<SPCOutput>` channel (v_low ignored)
   - `process_message()` stays private; `run()` consumes self (unchanged)

5. **Epoch reconfiguration**: `commit_vector()` checks `block.is_reconfiguration_suffix()` and returns a flag. If true, `on_spc_v_high()` does NOT advance to next slot — the SlotManager's event loop exits, and the EpochManager handles the epoch transition (shutdown + restart with new validator set).

6. **State recovery**: Defer persistence. Use state sync for crash recovery on restart.

7. **Config**: Use local `NodeConfig` (`config/src/config/consensus_config.rs`) with `enable_prefix_consensus: bool` flag. Do NOT modify on-chain `ConsensusAlgorithmConfig` (`types/src/on_chain_config/consensus_config.rs`) to avoid breaking serialization of the 3-variant enum.

8. **Slot→round mapping**: `round = highest_committed_round + slot`. Initialized from recovery data at epoch start. Ensures monotonic increase across epochs.

9. **Parent block initialization**: From recovery data (last committed block or genesis) at epoch start, same as DAG's `parent_block_info`.

10. **Timestamps**: `max(parent_timestamp_usecs + 1, current_wall_clock_usecs)`. Same logic as DAG.

11. **v_low "full prefix" check**: Check `v_low.iter().all(|h| *h != HashValue::zero())` (all entries non-⊥), NOT vector length (vector is always length n).

12. **Stale slot messages**: Messages for `slot < current_slot` are dropped with a debug log. The SPC task for that slot has already exited and its channels are closed.

13. **Ranking passthrough**: `MultiSlotRankingManager.current_ranking()` is passed as `initial_ranking` to each SPC instance, connecting the cross-slot demotion ranking to the per-view cyclic rotation inside SPC.

14. **Slot field in vote data**: Already present. Vote1/2/3 and their SignData include `slot: u64` (added during Strong PC Phase 3). Cross-slot replay is already prevented.

## Remaining Open Questions

1. **Payload deduplication**: When multiple validators propose overlapping transactions, the assembled block may have duplicates. The execution layer has `TransactionDeduper` — verify it handles this correctly for our block type.

2. **QuorumCert assumptions in pipeline**: Our blocks use a dummy `QuorumCert::new(VoteData::new(BlockInfo::empty(), BlockInfo::empty()))` (same as DAG). Need to verify no pipeline code inspects these QCs expecting real signatures or valid parent references.

3. **Block size from n proposals**: With 4 validators each pulling up to 500 txns, the combined block could have 2000 transactions. For the prototype this is fine; if block size becomes an issue under load, reduce `max_txns` to `500 / n` per validator to keep combined block size bounded.

## Estimated New Code

| Component | Lines | Files |
|-----------|-------|-------|
| Phase 1: Slot types + network | ~300 | 3 new, 2 modified |
| Phase 2: Ranking manager | ~150 | 1 new |
| Phase 3: Proposal buffer + state | ~400 | 1 new |
| Phase 4: Block builder | ~350 | 1 new, 2 modified |
| Phase 5: SlotManager core | ~800 | 1 new |
| Phase 6: SPC integration | ~300 | 1 modified |
| Phase 7: Payload resolution | ~350 | 2 modified |
| Phase 8: EpochManager integration | ~400 | 2 modified |
| Phase 9: BlockType + payload integration | ~400 | 3+ modified (grep-driven) |
| Phase 10: Execution pipeline compat | ~300 | 0-5 modified (grep-driven) |
| Phase 11: Smoke tests | ~400 | 3 new |
| Phase 12: Documentation | ~100 | 1 modified |
| Phase 13: Verifiable ranking | ~300 | 3 modified |
| **Total** | **~4550** | **~10 new, ~23 modified** |

## Critical Path

```
Phase 1 (types) → Phase 2 (ranking) → Phase 3 (buffer)
                                            ↓
Phase 4 (block builder) ← ← ← ← ← ← ← ←
         ↓
Phase 5 (SlotManager) + Phase 6 (SPC refactor)
         ↓
Phase 7 (payload resolution)
         ↓
Phase 8 (EpochManager) + Phase 9 (BlockType)
         ↓
Phase 10 (execution compat)
         ↓
Phase 11 (smoke tests) → Phase 12 (cleanup) → Phase 13 (verifiable ranking)
```

## Success Criteria

- [ ] 4-validator swarm starts with prefix consensus enabled
- [ ] Validators broadcast proposals and run SPC per slot
- [ ] Blocks are produced and committed sequentially (slot 1, 2, 3, ...)
- [ ] Submitted transactions appear in committed blocks
- [ ] Execution state advances (version increases, storage updated)
- [ ] Ranking updates correctly across slots (demotes excluded party)
- [ ] Epoch transitions handled (if reconfig txn in committed block)
- [ ] All existing tests unaffected (feature-gated behind config)
- [ ] `cargo test -p aptos-prefix-consensus` passes (all unit tests)
- [ ] Smoke tests pass for multi-slot scenarios

## TODO (Future Work)

- [ ] **Slot pipelining**: Currently slots run strictly sequentially (slot s must fully complete before slot s+1 starts). For higher throughput, pipeline multiple slots: start collecting proposals for slot s+1 while SPC for slot s is still running. The paper notes (Section 7.2.2) that running k parallel instances with staggered starts reduces effective slot latency by factor k (k=2 suffices to match commit latency). Key challenges: (1) v_high from slot s is needed for ranking update in slot s+1, so can overlap proposal collection but not SPC start; (2) execution ordering must remain sequential; (3) memory management for concurrent slot states.

- [ ] **v_low fast commit (full prefix case)**: When SPC outputs v_low and ALL entries are non-⊥ (`v_low.iter().all(|h| *h != HashValue::zero())`), immediately commit from v_low without waiting for v_high. Note: v_low is always a length-n vector (one entry per validator in ranking order). The check is for absence of ⊥ entries, NOT vector length. Still run SPC to completion (v_high is needed for ranking update). Requires: (1) `output_tx` sends v_low in addition to v_high; (2) `SPCOutput` gains a variant or flag for v_low vs v_high; (3) `committed_proposals: HashSet<HashValue>` to avoid double-committing entries when v_high arrives; (4) `on_spc_v_low()` handler in SlotManager.

- [ ] **v_low early commit (general case)**: The paper's Algorithm 4 commits v_low entries immediately regardless of length. A more sophisticated implementation would: (1) commit the entries in v_low as soon as it arrives (partial block from known-safe proposals); (2) when v_high arrives, commit the remaining entries (v_high extends v_low by Upper Bound); (3) this produces two blocks per slot in the general case — a "fast block" from v_low and a "completion block" from v_high \ v_low. Challenges: the execution pipeline expects sequential blocks with monotonic versions, so the two-block-per-slot model needs careful version tracking and parent chaining. Also need to handle the case where v_low is empty (no fast commit possible).

- [ ] **Make `SLOT_PROPOSAL_TIMEOUT` (300ms) configurable**: Via consensus config or constructor parameter. The optimal value depends on network conditions (Δ in the paper).

- [ ] **QuorumStore integration**: Replace DirectMempool with QuorumStore for production-grade payload dissemination. Each validator creates batches, disseminates via QuorumStore, and references via ProofOfStore in proposals. Reduces network bandwidth significantly.

- [ ] **State persistence and crash recovery**: Persist slot state (current slot number, ranking, pending proposals) to survive validator restarts without relying on state sync. Currently deferred — restart from slot 1 and use state sync.

- [ ] **Byzantine proposal handling**: Detect and handle equivocating proposals (same validator sending different proposals to different parties for the same slot). SPC handles this at the hash level, but we should log/report detected equivocation for reputation tracking.
