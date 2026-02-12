# Strong Prefix Consensus Implementation Plan

## Overview

This plan describes the implementation of Strong Prefix Consensus, which extends basic Prefix Consensus by requiring agreement on the `v_high` output. Strong Prefix Consensus will be the building block for the future Slot Manager in multi-slot consensus.

## Background from Paper

### Strong Prefix Consensus Definition (Paper Section 2.2, Definition 2.5)

Strong Prefix Consensus satisfies all properties of Prefix Consensus PLUS:
- **Agreement**: `v_high_i = v_high_j` for all honest parties i, j

Key insight: Unlike basic Prefix Consensus where parties can output different (but consistent) v_high values, Strong Prefix Consensus ensures ALL parties output the IDENTICAL v_high value.

### Protocol Architecture (Paper Algorithm 3, Section 6)

Strong Prefix Consensus uses a **multi-view** design:

**View 1**:
- Run Verifiable Prefix Consensus on actual input vector
- Immediately output v_low from view 1 as the Strong Prefix Consensus low value
- The v_high from view 1 becomes a candidate

**Views 2, 3, ...**:
- Run Verifiable Prefix Consensus on **truncated certificate vectors** (not raw inputs)
- Each party forms a vector from received certificates, ordered by ranking
- **Key optimization**: Truncate vector after the first non-⊥ entry (only the first certificate matters for tracing back)
- This keeps vectors small (typically length 1-2) and eliminates need for certificate fetching
- When a view commits a non-empty certificate vector, it creates a parent chain
- Following parent pointers uniquely determines the view-1 high value
- Output that as the Strong Prefix Consensus high value

**Progress mechanism**:
- Cyclic ranking shifts across views ensure leaderless progress
- Even if adversary suspends one party per round, eventually honest parties occupy first positions
- Direct certificates: advance view-by-view when progress is made
- Indirect certificates: skip empty views using aggregated signatures

## Current Implementation State

From exploration of existing code:

✅ **Basic Prefix Consensus**: Complete single-shot 3-round protocol
- Types: Vote1/2/3, QC1/2/3, PrefixConsensusProtocol
- Manager: PrefixConsensusManager (single-shot, event-driven)
- Network: NetworkSenderAdapter, PrefixConsensusMsg enum
- Smoke tests: Working with 100% success rate

✅ **Architecture patterns established**:
- Protocol state machine is Arc<RwLock<>> for thread-safe access
- Manager uses tokio::select! event loop
- Network sender is generic trait-based
- Self-send channel pattern for message routing

## Goal Architecture

```
Future SlotManager (Phase 8+)
  │
  └─> StrongPrefixConsensusManager (THIS PLAN)
        │
        ├─> View 1: VerifiablePrefixConsensusProtocol (on input vector)
        ├─> View 2: VerifiablePrefixConsensusProtocol (on certificate vector)
        ├─> View 3: VerifiablePrefixConsensusProtocol (on certificate vector)
        └─> ... (until v_high agreement reached)
```

## Key Design Decisions

### 1. Verifiable Prefix Consensus First

The paper uses "Verifiable Prefix Consensus" which augments basic Prefix Consensus with proofs (Section 5). We need this because:
- Certificates must be publicly verifiable
- Byzantine parties could lie about outputs
- QC3 serves as the proof (contains all votes)

**Decision**: Implement Verifiable Prefix Consensus as a thin wrapper around basic Prefix Consensus that outputs (v_low, π_low, v_high, π_high) where π = QC3.

### 2. Certificate Types

Two types of certificates (Paper Section 6.1.3):

**Direct Certificate**: `Cert^dir(w-1, v_high, π_high)`
- Used when view w-1 produced a non-empty verifiable high
- Directly points to parent from previous view

**Indirect Certificate**: `Cert^ind(w-1, (w*, v_high*, π_high*), Σ)`
- Used when view w-1 was empty
- Aggregates >1/3 stake worth of "skip" statements
- Points to earlier view w* < w-1 that had non-empty output
- Σ is aggregate signature on skip statements

### 3. Multi-View State Management

Each view runs an independent Verifiable Prefix Consensus instance, but we need:
- Shared ranking that shifts cyclically
- Certificate tracking across views
- Parent chain computation
- Current view number tracking

**Decision**: StrongPrefixConsensusProtocol will manage multiple view states, each containing its own VerifiablePrefixConsensusProtocol instance.

### 4. Message Multiplexing

Network messages must include view number for routing.

**Decision**: Extend PrefixConsensusMsg to include view field, or create new StrongPrefixConsensusMsg wrapper.

### 5. Message Size Optimization: Hashes vs Full Certificates

**Problem**: If input vectors for views > 1 contained full certificates, message sizes would be O(N²):
- Each Vote contains input vector with certificates (each cert has N signatures)
- QC aggregates N votes
- Total: O(N × N) = O(N²) per QC

**Solution**: Use certificate **hashes** in input vectors, not full certificates:
- Input vectors contain: `[⊥, hash(cert_A), ⊥, ⊥]` (truncated after first non-⊥)
- Each Vote contains O(hash_size) = O(1)
- Total per QC: O(N × 1) = O(N)

**View transition protocol**:
1. Party creates certificate from previous view's output
2. Party **broadcasts full certificate once** when entering new view
3. Other parties store the certificate locally, indexed by hash
4. Votes in Prefix Consensus rounds only contain the hash

**Fetching mechanism** (fallback for Byzantine withholding):
- When: Party has only a hash but needs full certificate (e.g., to verify chain at commit time)
- From whom: Any party who signed a vote containing that hash (they must have the cert)
- Happy path: No fetching needed - parties have certs from view transitions
- Byzantine case: Rare, fetch when Byzantine party withholds certificate from some parties

### 6. StrongPCCommit Message Design

**Why full certs in StrongPCCommit but hashes in view-by-view**:
- StrongPCCommit is one message (not aggregated N times) → O(chain × N) = O(N)
- View-by-view votes are aggregated N times → O(N²) if full certs
- Chain is typically short (2-5 certs)

**Decision**: StrongPCCommit embeds full certificate chain. No fetching needed for commit verification.

## Implementation Phases

### Phase 1: Verifiable Prefix Consensus (Foundation)

**Goal**: Add proof outputs to basic Prefix Consensus

**Files to create**:
- None (modify existing)

**Files to modify**:
- `consensus/prefix-consensus/src/protocol.rs`
- `consensus/prefix-consensus/src/types.rs`

**Tasks**:
1. Add proof types to Output:
   ```rust
   pub struct PrefixConsensusOutput {
       pub v_low: PrefixVector,
       pub v_high: PrefixVector,
       pub qc3: QC3,  // This is the proof
   }
   ```

2. Modify protocol to return QC3 alongside outputs (already does this)

3. Add verification predicates:
   ```rust
   pub fn verify_low_proof(v_low: &PrefixVector, qc3: &QC3) -> bool;
   pub fn verify_high_proof(v_high: &PrefixVector, qc3: &QC3) -> bool;
   ```

4. Write unit tests for proof verification

**Verification**: Tests pass, proofs can be verified by other parties

---

### Phase 2: Certificate Types

**Goal**: Define certificate structures and validation logic

**Files to create**:
- `consensus/prefix-consensus/src/certificates.rs` (~300 lines)

**Core types**:
```rust
// Direct certificate for view advancement
pub struct DirectCertificate {
    pub parent_view: u64,
    pub v_high: PrefixVector,
    pub proof: QC3,
}

// Indirect certificate for skipping empty views
pub struct IndirectCertificate {
    pub empty_view: u64,
    pub parent_view: u64,
    pub v_high: PrefixVector,
    pub proof: QC3,
    pub skip_signatures: AggregateSignature,  // >1/3 stake signatures on (empty_view, parent_view)
}

pub enum Certificate {
    Direct(DirectCertificate),
    Indirect(IndirectCertificate),
}

// Helper for tracking highest known view with verifiable high
pub struct HighestKnown {
    pub view: u64,
    pub v_high: PrefixVector,
    pub proof: QC3,
}
```

**Key functions**:
```rust
impl Certificate {
    pub fn parent_view(&self) -> u64;
    pub fn parent_high(&self) -> &PrefixVector;
    pub fn validate(&self, verifier: &ValidatorVerifier) -> Result<()>;
    pub fn has_parent(&self, v: &PrefixVector) -> bool;
}
```

**Tasks**:
1. Define certificate structures with BCS serialization
2. Implement validation logic for both certificate types
3. Implement `extract_parent()` function that returns (parent_view, v_high)
4. Add unit tests for certificate creation and validation

**Verification**: Certificate validation tests pass

---

### Phase 3: View State and Ranking Management — ✅ COMPLETE

**Goal**: Multi-view state tracking and cyclic ranking

**Files to modify**:
- `consensus/prefix-consensus/src/types.rs` (add view types)

**Files to create**:
- `consensus/prefix-consensus/src/view_state.rs` (~400 lines)

**Core types**:
```rust
// Per-view state
pub struct ViewState {
    pub view_number: u64,
    pub ranking: Vec<PartyId>,  // Current ranking for this view
    pub protocol: Arc<RwLock<VerifiablePrefixConsensusProtocol>>,
    pub proposal_buffer: HashMap<PartyId, ProposalObject>,  // Received proposals
    pub output: Option<ViewOutput>,
}

pub struct ViewOutput {
    pub v_low: PrefixVector,
    pub v_high: PrefixVector,
    pub proof: QC3,
}

pub struct ProposalObject {
    pub view: u64,
    pub certificate: Certificate,
}

// Ranking manager
pub struct RankingManager {
    initial_ranking: Vec<PartyId>,
}

impl RankingManager {
    pub fn new(initial: Vec<PartyId>) -> Self;
    pub fn get_ranking_for_view(&self, view: u64) -> Vec<PartyId>;
    // Cyclic shift: (p1, p2, ..., pn) -> (p2, p3, ..., pn, p1)
}
```

**Tasks**:
1. Implement ViewState to encapsulate per-view Prefix Consensus instance
2. Implement RankingManager with cyclic shift logic
3. Add parent chain tracking logic
4. Write unit tests for ranking shifts (verify correct cyclic behavior)

**Verification**: Ranking tests show correct cyclic shifts for multiple views

---

### Phase 4: Strong Prefix Consensus Protocol — ✅ COMPLETE

**Goal**: Core multi-view protocol logic

**Files to create**:
- `consensus/prefix-consensus/src/strong_protocol.rs` (~800 lines)

**Core type**:
```rust
pub struct StrongPrefixConsensusProtocol {
    // Configuration
    party_id: PartyId,
    epoch: u64,

    // Input
    input_vector: PrefixVector,
    initial_ranking: Vec<PartyId>,

    // State
    current_view: Arc<RwLock<u64>>,
    view_states: Arc<RwLock<HashMap<u64, ViewState>>>,
    ranking_manager: RankingManager,

    // Tracking highest known verifiable high with parent
    highest_known: Arc<RwLock<HighestKnown>>,

    // Outputs
    v_low: Arc<RwLock<Option<PrefixVector>>>,  // View 1 output
    v_high: Arc<RwLock<Option<PrefixVector>>>, // Agreed view-1 high

    // Validation
    validator_verifier: Arc<ValidatorVerifier>,
}

impl StrongPrefixConsensusProtocol {
    pub fn new(...) -> Self;

    // View management
    pub async fn enter_view(&self, view: u64, cert: Option<Certificate>);
    pub async fn start_view_1(&self, signer: &ValidatorSigner);
    pub async fn start_view(&self, view: u64, signer: &ValidatorSigner);

    // Message processing
    pub async fn process_proposal(&self, from: PartyId, proposal: ProposalObject);
    pub async fn process_view_output(&self, view: u64, output: ViewOutput);
    pub async fn process_empty_view(&self, view: u64, from: PartyId, highest: HighestKnown, sig: Signature);

    // Certificate creation
    pub async fn create_direct_certificate(&self, view: u64) -> Option<DirectCertificate>;
    pub async fn create_indirect_certificate(&self, view: u64) -> Option<IndirectCertificate>;

    // Parent chain following
    pub async fn compute_parent_chain(&self, cert: &Certificate) -> Vec<(u64, PrefixVector)>;
    pub async fn commit(&self, view: u64, v_low: PrefixVector);

    // Output queries
    pub async fn get_low(&self) -> Option<PrefixVector>;
    pub async fn get_high(&self) -> Option<PrefixVector>;
    pub async fn is_complete(&self) -> bool;
}
```

**Key algorithms**:

1. **View 1 start**:
   - Create Verifiable Prefix Consensus instance with input vector
   - Start round 1
   - On v_low output: immediately output as Strong Prefix Consensus low
   - On v_high output: if non-empty, broadcast direct cert for view 2; else broadcast empty-view
   - **Note**: View 1 is special - even all-⊥ v_high is meaningful (inputs conflict at position 0)

2. **View w > 1 completion** (three-way decision):
   - Wait for timeout or proposals from all validators
   - Order proposals by current ranking
   - Create input vector from certificates, ordered by ranking
   - **Truncate after first non-⊥**: If vector is [⊥, cert_A, cert_B, ...], use [⊥, cert_A]
   - Start Verifiable Prefix Consensus instance on truncated vector
   - On completion, apply **three-way decision**:

   ```
   a) If v_low has non-⊥ entry (has_committable_low):
      → Commit! Trace back to View 1, output Strong PC v_high, DONE

   b) Else if v_high has non-⊥ entry (has_certifiable_high):
      → Create DirectCertificate from v_high
      → Broadcast for view w+1
      → No commit yet, but progress made

   c) Else (both v_low and v_high are all-⊥):
      → Empty view, no progress
      → Broadcast EmptyViewMessage
      → Collect >1/3 stake, create IndirectCertificate
      → Move to view w+1
   ```

3. **Commit(view, v_low)** — trace-back to View 1:
   - Derive v_low from committing view's QC3 via `qc3_certify()`
   - Find first non-⊥ in v_low → cert hash H₀ (this triggers the commit)
   - Look up cert C₀ by H₀ in local cert store
   - If `cert_reaches_view1(C₀)`: output C₀.v_high() as Strong PC high, DONE
   - Else: find first non-⊥ in C₀.v_high() → next cert hash → follow chain
   - Repeat until reaching a cert where `cert_reaches_view1()` is true
   - Build certificate chain and broadcast `StrongPCCommit` message

   **Terminal condition**: `cert_reaches_view1(cert)` checks `cert.parent_view() == 1`.
   Uses `parent_view()` not `view()` because the chain may end at an IndirectCert
   that points to View 1 (e.g., `IndirectCert(empty=2, parent=1)` when View 2 was
   empty). Such a cert has `view()=2` but `parent_view()=1`, and its `v_high()` is
   View 1's output. We cannot follow further — `v_high` contains raw transaction
   hashes, not certificate hashes.

4. **Empty-view handling** (only when both v_low and v_high are all-⊥):
   - Collect >1/3 stake worth of empty-view messages for view w
   - Extract max parent_view from those messages
   - Create indirect certificate pointing to that max parent_view
   - Broadcast for view w+1

**Important edge case handling**:
- `has_committable_low(v_low)`: Returns true if v_low has at least one non-⊥ entry
- `has_certifiable_high(v_high)`: Returns true if v_high has at least one non-⊥ entry
- In views > 1, a vector like `[⊥, ⊥, ⊥]` is NOT meaningful (no certificate to trace)
- In View 1, even all-⊥ outputs are valid (raw inputs, not certificates)

**Tasks**:
1. Implement StrongPrefixConsensusProtocol struct
2. Implement view 1 logic (input on raw vector)
3. Implement view w>1 logic (input on certificate vector)
4. Implement commit logic with parent chain following
5. Implement empty-view aggregation for indirect certificates
6. Write extensive unit tests for:
   - View 1 completion
   - View 2 direct advancement
   - Empty view skipping
   - Parent chain computation
   - Multi-view scenarios

**Verification**: Unit tests pass for single-view and multi-view scenarios

---

### Phase 5: Network Messages for Strong Prefix Consensus — ✅ COMPLETE

**Goal**: Message types for multi-view protocol

**Files to modify**:
- `consensus/prefix-consensus/src/network_messages.rs`

**New message types**:
```rust
// Proposal for view w > 1
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ViewProposal {
    pub view: u64,
    pub certificate: Certificate,  // Full certificate (not hash) - no fetching needed
}

// Empty-view message for skipping
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EmptyView {
    pub view: u64,
    pub highest_known: HighestKnown,
    pub signature: Signature,  // On (view, highest_known.view)
}

// Commit announcement with full proof chain for termination
// Enables other parties to verify and terminate without independent trace-back
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StrongPCCommit {
    /// The final Strong PC v_high output (traced back to View 1)
    pub v_high: PrefixVector,

    /// Chain of certificates from committing view back to View 1
    /// Ordered: chain[0] = certificate from committing view,
    ///          chain[n] = certificate pointing to View 1
    pub certificate_chain: Vec<Certificate>,

    /// Epoch for validation
    pub epoch: u64,
}

// Enhanced message enum
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum StrongPrefixConsensusMsg {
    // View 1: basic Prefix Consensus messages
    View1(PrefixConsensusMsg),

    // View w > 1: proposal messages (each party broadcasts their certificate)
    ViewProposal(Box<ViewProposal>),

    // Empty view messages
    EmptyView(Box<EmptyView>),

    // Commit announcement - enables other parties to terminate
    Commit(Box<StrongPCCommit>),
}

impl StrongPrefixConsensusMsg {
    pub fn view(&self) -> u64;
    pub fn epoch(&self) -> u64;
    pub fn name(&self) -> &'static str;
}
```

**Commit Message Design** (implemented in certificates.rs, helpers in utils.rs):
- `StrongPCCommit` contains: `committing_proof` (QC3), `certificate_chain`, `v_high`, `epoch`, `slot`
- `committing_proof`: QC3 from the committing view — proves v_low had a non-⊥ entry
- `certificate_chain`: certs traced from v_low back to View 1 via v_high hash links
- Verification: validate QC3 → derive v_low → check `first_non_bot(v_low) == hash(chain[0])`
  → walk chain: `first_non_bot(chain[i].v_high()) == hash(chain[i+1])` → terminal check
  → v_high match
- Terminal check uses `cert_reaches_view1()` which checks `parent_view() == 1`,
  handling both DirectCert(V=1) and IndirectCert(parent=1) as terminal certs
- The committing party stops participating immediately after broadcasting
- Chain is typically short (2-5 certificates) due to truncated vector optimization

**Note**: No certificate fetching messages needed. Each party broadcasts their full certificate
(not just a hash), and vectors are truncated after the first non-⊥ entry, keeping message
sizes small.

**Tasks**:
1. Define new message types with serialization
2. Add helper methods for message introspection
3. Write serialization round-trip tests

**Verification**: Message serialization tests pass

---

### Phase 6: Strong Prefix Consensus Manager — ✅ COMPLETE

**Goal**: Event-driven manager orchestrating multi-view protocol

**Files created**:
- `consensus/prefix-consensus/src/strong_manager.rs` (1394 lines)

**Files modified** (cross-cutting fixes during implementation):
- `protocol.rs` — Changed `start_round1/2/3` return types to `(Vote, Option<QC>)` for early QC handling
- `manager.rs` — Updated to handle early QCs from `start_round1/2/3`
- `certificates.rs` — Added `epoch`/`slot` to EmptyViewMessage; `from_messages` returns `Option`
- `network_messages.rs` — Updated EmptyView epoch/slot helpers

**Actual struct** (differs significantly from plan):
```rust
pub struct StrongPrefixConsensusManager<NetworkSender> {
    party_id: PartyId,
    epoch: u64,
    slot: u64,
    protocol: StrongPrefixConsensusProtocol,   // Owned, not Arc
    ranking_manager: RankingManager,
    current_view: u64,
    view_states: HashMap<u64, ViewState>,       // Certificate tracking per view
    pc_states: HashMap<u64, PCState>,           // Inner PC instances per view
    seen_proposals: HashMap<u64, HashSet<PartyId>>,
    empty_view_collectors: HashMap<u64, Vec<EmptyViewMessage>>,
    pending_fetches: HashMap<HashValue, u32>,
    pending_commit_proof: Option<QC3>,
    view_start_timer: Option<Pin<Box<Sleep>>>,  // 300ms timer
    network_sender: NetworkSender,
    validator_signer: ValidatorSigner,
    validator_verifier: Arc<ValidatorVerifier>,
}
```

**Key methods implemented**:
- `run()` — tokio::select! event loop with message_rx, close_rx, and view_start_timer branches
- `process_message()` — Uniform epoch/slot filtering, then dispatch by variant
- `process_proposal()` — Handles future views (adopt + enter), current view (add cert + try start), stale (ignore)
- `propose_and_enter()` — Broadcasts proposal, adds own cert, enters view. Guards against double proposals.
- `enter_view()` — Creates ViewState, sets timer or starts immediately if first-ranked cert available
- `try_start_pc()` — Triggered by cert arrival; starts inner PC if first-ranked cert present
- `start_pc_now()` — Box::pin async; starts inner PC unconditionally with available certs
- `finalize_view()` — Handles three-way decision output: commit trace-back, DirectCert, or EmptyView
- `handle_view_complete()` — Delegates to protocol for decision, then to finalize_view
- `handle_commit_decision()` — Builds chain, broadcasts StrongPCCommit, or initiates fetch
- `process_fetch_response()` — DoS-protected: checks pending_fetches before validation
- `has_first_ranked_cert()` — Helper to check if top-ranked certificate is available

**Key design decisions made during implementation** (see strong-pc-phase4-6.md for details):
1. View start timer (300ms timeout or first-ranked cert)
2. Proposal adoption (adopt received certs as own proposals)
3. No double proposals (Byzantine behavior prevention)
4. Early QC bug fix (cross-cutting protocol.rs + manager.rs)
5. Fetch DoS protection (check pending_fetches before work)
6. ViewProposal doesn't need signing (network auth + cert validation)
7. Box::pin for recursive async future sizing

**Tests**: 186 total unit tests passing (all chunks combined)

---

### Phase 7: Integration with Consensus Layer

**Goal**: Wire Strong Prefix Consensus into epoch manager

**Files to modify**:
- `consensus/src/epoch_manager.rs`
- `consensus/src/network_interface.rs` (add StrongPrefixConsensusMsg variant)

**Tasks**:
1. Add `StrongPrefixConsensusMsg` to `ConsensusMsg` enum
2. Add routing in `EpochManager::check_epoch()` for Strong Prefix Consensus messages
3. Implement `start_strong_prefix_consensus(input_vector, initial_ranking)` method:
   - Create channels
   - Create NetworkSenderAdapter
   - Spawn StrongPrefixConsensusManager
   - Return after initialization
4. Implement `stop_strong_prefix_consensus()` graceful shutdown
5. Store channels: strong_prefix_consensus_tx, strong_prefix_consensus_close_tx

**Verification**: Compiles successfully, can start/stop Strong Prefix Consensus

---

### Phase 8: Smoke Tests

**Goal**: End-to-end testing with LocalSwarm

**Files to create**:
- `testsuite/smoke-test/src/consensus/strong_prefix_consensus/mod.rs`
- `testsuite/smoke-test/src/consensus/strong_prefix_consensus/helpers.rs` (~200 lines)
- `testsuite/smoke-test/src/consensus/strong_prefix_consensus/basic_test.rs` (~400 lines)

**Test cases**:

1. **test_strong_prefix_consensus_identical_inputs**
   - 4 validators, identical input vectors
   - Should complete in view 1 (optimistic case)
   - Verify all validators output same v_high
   - Verify v_low ⪯ v_high

2. **test_strong_prefix_consensus_divergent_inputs**
   - 4 validators, partially overlapping inputs
   - Should complete in view 2 (need one extra view for agreement)
   - Verify all validators output same v_high
   - Verify v_low ⪯ v_high

3. **test_strong_prefix_consensus_multi_view**
   - 4 validators, set up to require 3-4 views
   - Verify view progression (rankings shift)
   - Verify all validators eventually agree on v_high

4. **test_strong_prefix_consensus_empty_view_skip**
   - Force empty view scenario
   - Verify indirect certificate creation
   - Verify view skipping works correctly

**Test helpers**:
```rust
fn generate_test_input_vectors(...) -> Vec<Vec<HashValue>>;
fn wait_for_strong_prefix_consensus_outputs(...) -> Result<Vec<Output>>;
fn verify_agreement_property(outputs: &[Output]) -> bool;
fn verify_upper_bound_property(outputs: &[Output]) -> bool;
fn cleanup_output_files();
```

**Tasks**:
1. Add config option for triggering Strong Prefix Consensus
2. Create test helpers for input generation
3. Implement output file parsing (validators write to /tmp/strong_prefix_consensus_output_{party_id}.json)
4. Write 4 smoke tests listed above
5. Create test script: `test_strong_prefix_consensus.sh`

**Verification**:
- All smoke tests pass consistently (100% success rate)
- Logs show correct view progression
- Output files verify agreement property
- Properties: Agreement, Upper Bound, Validity all hold

---

### Phase 9: Inner PC Abstraction Trait

**Goal**: Extract inner Prefix Consensus algorithm behind a trait so the Strong Manager
can swap algorithms without changing its core logic (view transitions, proposals, commits,
fetching, empty-view handling).

**Motivation**: The current Strong Manager drives `PrefixConsensusProtocol` directly through
~10 methods (`process_inner_pc`, `process_view_vote1/2/3`, `start_view_round2/3`,
`start_view1`, `try_start_view_pc`, `ViewPCState`). A future optimized inner algorithm
(e.g., 2-round good case from Appendix D) should be a drop-in replacement.

**What the manager sees today (the implicit interface)**:
- **Creation**: `PrefixConsensusProtocol::new(input, verifier)` — creates instance from input vector
- **Start**: `protocol.start_round1(signer)` → returns first outbound message
- **Process**: `protocol.process_vote1/2/3(vote)` → returns `Option<QC>` or `Option<Output>`
- **Round transitions**: `protocol.start_round2/3(signer)` → returns next outbound message
- **Completion**: `process_vote3` returns `Some(PrefixConsensusOutput)` when done
- **Output conversion**: `PrefixConsensusOutput { v_low, v_high, qc3 }` → `ViewOutput`

**Trait design**:
```rust
/// Trait for an inner Prefix Consensus algorithm used by the Strong Manager.
///
/// The Strong Manager creates one instance per view and drives it by feeding
/// incoming messages and broadcasting outbound messages.
#[async_trait]
pub trait InnerPCAlgorithm: Send + Sync {
    /// The message type this algorithm sends/receives (e.g., PrefixConsensusMsg)
    type Message: Clone + Send + Sync;

    /// Create a new instance for a view
    fn new_for_view(
        input: PrefixConsensusInput,
        verifier: Arc<ValidatorVerifier>,
    ) -> Self;

    /// Start the algorithm — returns the first outbound message to broadcast
    async fn start(&self, signer: &ValidatorSigner) -> Result<Self::Message>;

    /// Process an incoming message from a peer
    ///
    /// Returns:
    /// - `Ok((outbound, None))` — optional outbound message, not yet complete
    /// - `Ok((outbound, Some(output)))` — complete, with final output
    async fn process_message(
        &self,
        author: Author,
        msg: Self::Message,
    ) -> Result<(Option<Self::Message>, Option<ViewOutput>)>;

    /// Check for duplicate message from this author (for dedup at manager level)
    fn is_duplicate(&self, author: &PartyId, msg: &Self::Message) -> bool;
}
```

**Files to modify**:
1. **New file `inner_pc_trait.rs`** (~50 lines): Define the trait
2. **`strong_manager.rs`**: Replace concrete `PrefixConsensusProtocol` with `T: InnerPCAlgorithm`
   - `ViewPCState` becomes `ViewPCState<T>` holding `T` instead of `Arc<PrefixConsensusProtocol>`
   - Remove `process_view_vote1/2/3`, `start_view_round2/3` — replaced by `T::process_message`
   - `process_inner_pc` simplifies to: validate author → dedup → `T::process_message` → handle output
   - `start_view1` and `try_start_view_pc` use `T::new_for_view` + `T::start`
   - Per-view dedup moves into `T::is_duplicate` or stays in manager (design choice)
3. **New file `inner_pc_impl.rs`** (~150 lines): Implement the trait for `PrefixConsensusProtocol`
   - Wraps the current 3-round logic (start_round1 → process_vote1 → start_round2 → ...)
   - Handles internal round state transitions
   - Maps `PrefixConsensusOutput` to `ViewOutput`
4. **`lib.rs`**: Add module declarations and exports

**Key design decisions**:
- The trait's `Message` associated type allows different algorithms to use different wire formats
  (the Strong Manager wraps them in `StrongPrefixConsensusMsg::InnerPC` regardless)
- Dedup can stay in the manager (simpler) or move into the trait (more encapsulated)
- The trait is async because the current protocol uses `Arc<RwLock<>>` internally;
  a future sync algorithm could use a thin async wrapper

**What stays in the manager (unchanged)**:
- View transitions and entry triggers (a) and (b)
- `ViewState` and `RankingManager` for certificate tracking
- Proposal handling (`process_proposal`)
- Empty-view handling (EmptyViewMessage → IndirectCertificate)
- Commit handling (trace-back, StrongPCCommit, fetching)
- Epoch/slot filtering
- Event loop structure

**Verification**: All existing tests pass. The refactor is purely structural — no behavior change.

---

### Phase 10: Performance Testing & Optimization

**Goal**: Measure performance, optimize if needed

**Tasks**:
1. Add metrics to manager:
   - View duration
   - Message counts per view
   - Certificate sizes
2. Run smoke tests with instrumentation
3. Analyze view completion times
4. Optimize if bottlenecks found

**Verification**: Performance is acceptable, no obvious bottlenecks

---

### Phase 11: Documentation

**Goal**: Document the implementation

**Files to create/modify**:
- `consensus/prefix-consensus/README.md` (update)
- `project_context.md` (update)

**Content**:
1. README updates:
   - Strong Prefix Consensus architecture
   - Multi-view design explanation
   - Certificate types and validation
   - How to run tests
2. project_context.md updates:
   - Phase 7 completion status
   - Strong Prefix Consensus implementation details
   - Test results
   - Next steps (Slot Manager)

---

## Architecture Diagrams

### Strong Prefix Consensus Flow

```
View 1:
  Input: Raw vector [tx1, tx2, tx3, tx4]
  Run: Verifiable Prefix Consensus
  Output: v_low (immediate), v_high (candidate)

  If v_high non-empty:
    → Broadcast Direct Certificate for View 2
  Else:
    → Broadcast Empty-View message

View 2:
  Receive certificates from parties, order by shifted ranking (p2, p3, p4, p1)
  Build TRUNCATED input vector: [⊥, cert_first_non_bot] (cut after first non-⊥)
  Run: Verifiable Prefix Consensus on truncated vector
  Output: v_low

  If v_low non-empty:
    → Extract parent from first non-⊥ → View 1 v_high
    → OUTPUT as Strong Prefix Consensus v_high
    → DONE
  Else:
    → Collect >1/3 stake Empty-View messages
    → Create Indirect Certificate
    → Broadcast for View 3

View 3, 4, ...:
  Repeat with truncated vectors until non-empty commit found
```

### Manager Event Loop

```
                    ┌─────────────────────────┐
                    │   Network Messages      │
                    └───────────┬─────────────┘
                                │
                                ▼
                    ┌─────────────────────────┐
                    │  Message Router         │
                    │  (by view number)       │
                    └─────┬─────────┬─────────┘
                          │         │
              ┌───────────┘         └──────────┐
              ▼                                 ▼
    ┌──────────────────┐           ┌──────────────────┐
    │ View 1 Messages  │           │ View W Messages  │
    │ (Prefix Cons)    │           │ (Proposals)      │
    └────────┬─────────┘           └────────┬─────────┘
             │                               │
             ▼                               ▼
    ┌──────────────────┐           ┌──────────────────┐
    │ View 1 Protocol  │           │ View W Protocol  │
    │ Instance         │           │ Instance         │
    └────────┬─────────┘           └────────┬─────────┘
             │                               │
             └───────────┬───────────────────┘
                         ▼
              ┌─────────────────────┐
              │ Protocol Callbacks  │
              │ (output events)     │
              └──────────┬──────────┘
                         │
                         ▼
              ┌─────────────────────┐
              │ Broadcast to        │
              │ Network             │
              └─────────────────────┘
```

## Critical Files Summary

**New files** (~3000 lines total):
1. `consensus/prefix-consensus/src/certificates.rs` - Certificate types
2. `consensus/prefix-consensus/src/view_state.rs` - Per-view state
3. `consensus/prefix-consensus/src/strong_protocol.rs` - Multi-view protocol
4. `consensus/prefix-consensus/src/strong_manager.rs` - Event-driven manager
5. `testsuite/smoke-test/src/consensus/strong_prefix_consensus/*.rs` - Tests

**Modified files**:
1. `consensus/prefix-consensus/src/protocol.rs` - Add proof outputs
2. `consensus/prefix-consensus/src/types.rs` - Add view types
3. `consensus/prefix-consensus/src/network_messages.rs` - Add StrongPrefixConsensusMsg
4. `consensus/src/epoch_manager.rs` - Add start_strong_prefix_consensus
5. `consensus/src/network_interface.rs` - Add message routing
6. `consensus/prefix-consensus/src/lib.rs` - Export new modules

## Testing Strategy

**Unit tests**:
- Certificate validation
- Ranking shifts
- Parent chain computation
- View state management
- Message serialization

**Integration tests** (smoke tests):
- Identical inputs (optimistic case)
- Divergent inputs (2-view case)
- Multi-view scenarios
- Empty-view skipping

**Property verification**:
- Agreement: All v_high outputs identical
- Upper Bound: v_low ⪯ v_high for all parties
- Validity: mcp(inputs) ⪯ v_low for all parties

## Success Criteria

- [ ] All unit tests pass for certificate validation
- [ ] All unit tests pass for multi-view protocol
- [ ] Smoke test: Identical inputs complete in view 1
- [ ] Smoke test: Divergent inputs complete with agreement
- [ ] Smoke test: Multi-view scenario reaches agreement
- [ ] Smoke test: Empty-view skipping works correctly
- [ ] Agreement property verified: all v_high outputs identical
- [ ] Upper Bound property verified: v_low ⪯ v_high
- [ ] Validity property verified: mcp(inputs) ⪯ v_low
- [ ] 100% success rate across all smoke tests
- [ ] Code compiles without warnings
- [ ] Documentation updated

## Estimated Effort

- Phase 1 (Verifiable): ✅ COMPLETE
- Phase 2 (Certificates): ✅ COMPLETE
- Phase 3 (View State): ✅ COMPLETE
- Phase 4 (Protocol = Strong Protocol, Chunk 1): ✅ COMPLETE
- Phase 5 (Messages = Network Messages, Chunk 2): ✅ COMPLETE
- Phase 6 (Manager = Strong Manager, Chunk 3): ✅ COMPLETE
- Phase 7 (Integration): Pending — wire into EpochManager, real NetworkSender
- Phase 8 (Smoke Tests): Pending
- Phase 9 (Inner PC Abstraction): Pending — extract trait for swappable inner algorithm
- Phase 10 (Performance): Pending
- Phase 11 (Docs): Pending

**Total: 10.5-11.5 days of focused development**

## Future Work (Beyond This Plan)

After Strong Prefix Consensus is complete:
1. **Slot Manager**: Run Strong Prefix Consensus per slot
2. **Censorship resistance**: Implement ranking updates based on exclusions
3. **Optimistic variants**: 2-round good case (Appendix D)
4. **Communication optimization**: Reduce message sizes
5. **Multi-slot integration**: Connect to execution/storage

---

## TODO (Deferred Implementation Items)

These items are identified during design but deferred for later implementation:

- [ ] **Fetching protocol**: Implement certificate fetch by hash for Byzantine withholding scenarios
  - Request/response by certificate hash
  - Retry from different parties on failure
  - Consider rate limiting for DOS protection

- [ ] **StrongPCCommit chain cap**: If chains become long (>5 certs), cap embedded certs and use hashes for the rest
  - Monitor chain lengths in practice
  - Implement if needed based on real-world usage

- [ ] **Certificate storage**: Efficient storage and lookup by hash for accumulated certificates
  - HashMap<HashValue, Certificate> indexed by hash
  - Consider LRU cache for memory management
  - Persist to disk for crash recovery?

- [ ] **Fetch DOS protection**: Rate limiting / authentication for fetch requests
  - Only respond to validators in current epoch
  - Rate limit per requester
