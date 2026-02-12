# Strong Prefix Consensus: Phases 4-6 Implementation Plan

## Goal

Implement the multi-view Strong Prefix Consensus protocol as three reviewable chunks:
1. **Strong Protocol** — pure state machine (no I/O, no async)
2. **Network Messages** — message types for the multi-view protocol
3. **Strong Manager** — async event loop orchestrating views

## Key Design Decisions (From Brainstorm)

### 1. Protocol vs Manager Split

**Protocol** = pure state machine. No networking, no async. Takes method calls, returns
decisions. Fully testable in isolation with hand-crafted inputs.

**Manager** = async orchestrator. Event loop, message routing, network broadcasts,
timeouts, per-view PC instances, certificate fetching.

The protocol receives `ViewOutput` and makes decisions. It doesn't know or care HOW
the inner PC was run. This enables swapping the inner PC algorithm in the future.

### 2. Modularity

The Strong PC protocol interacts with basic PC only through `ViewOutput` (v_low, v_high,
proof). The manager is where the inner PC lives. Today it creates `PrefixConsensusProtocol`
per view. Swapping to an optimized algorithm later means changing the manager, not the
protocol. No premature trait abstraction — just clean separation.

### 3. Two Triggers for Entering a View

A party enters view V when EITHER:

**(a) Produced outcome for view V-1**: The party completed its own PC instance for V-1,
created a certificate, and is ready to propose for view V.

**(b) Received a valid certificate proposal for view V**: Another party already finished
V-1 and moved to V. The receiving party enters V using the received certificate as input
for building its truncated vector. The party does NOT need to complete V-1 first.

Case (b) is critical for liveness: a slow party should not hold up the protocol. If
others have moved on, the slow party skips ahead.

**Implication**: A party entering via case (b) may not have its own certificate for view V.
It participates using whatever certificates it has received. It also may be missing
intermediate certificates needed for later trace-back (see Fetching section below).

### 4. Timeout Strategy

**View 1**: Wait a fixed timeout (300ms) before starting the PC, to collect as many raw
input proposals as possible. More inputs → better mcp → richer v_low.

**Views > 1**: Two conditions to start:
- **Eager start**: Start as soon as we receive the first certificate proposal from the
  highest-ranked party (in the current view's ranking). Since the truncated vector only
  uses the first non-⊥ entry, waiting for the top-ranked cert is optimal.
- **Timeout fallback**: If no cert arrives within 300ms, start with whatever certs we
  have (possibly just our own from case (a), or one received via case (b)).

**TODO (future optimization)**: Explore adaptive timeouts, network-condition-based delays,
and whether waiting for multiple certs improves output quality vs latency tradeoff.

### 5. Fetching is Required for Liveness

When parties enter views via case (b), they skip intermediate views and may not have
certificates referenced in the v_high chain. This means no single honest party may
have the complete certificate chain for trace-back.

**Example scenario** (4 parties, A Byzantine, B/C/D honest):
1. View 1: All complete. Everyone has DirectCert(V=1).
2. View 2: A races ahead, proposes cert for view 2. B enters via case (b).
3. View 3: B completes V2, proposes for V3. C enters via case (b).
4. View 4: C completes V3, proposes for V4. D enters via case (b).

If view 4 commits, the chain traces V4 → V3 → V2 → V1. Each honest party has the cert
from the step right before it, but NOT the full chain:
- D has V3 cert (from C), missing V2 and V1 certs
- C has V2 cert (from B), missing V1 cert
- B has V1 cert (from A), but only A's — may not have the specific cert hash in the chain

**No single honest party has the complete chain.** Fetching is the mechanism to assemble it.

**Approach**: Lazy fetching with fallback.
- Protocol's `build_certificate_chain()` returns `Err(MissingCert(hash))` when a cert
  is not in the local store.
- Manager broadcasts `CertFetchRequest(hash)` to all peers.
- Peers with the cert respond with `CertFetchResponse(hash, cert)`.
- Manager validates (hash match + signatures), stores in cert store, retries chain building.
- Repeats until chain is complete.
- **Fallback**: If fetching fails repeatedly, wait for another party's `StrongPCCommit`
  message (which embeds the full chain). This works because eventually SOME party will
  assemble the chain (via fetching from different parties) and broadcast it.

**TODO (future optimization)**: Rate limiting for fetch requests (DOS protection),
targeted fetching (request from parties who voted for the hash), retry backoff.

---

## Existing Building Blocks

| Component | Location | What it provides |
|-----------|----------|-----------------|
| `PrefixConsensusProtocol` | `protocol.rs` | Single-shot 3-round PC (per-view inner algorithm) |
| `PrefixConsensusManager` | `manager.rs` | Drives basic PC: event loop, routing, broadcast |
| `ViewState` | `view_state.rs` | Per-view cert tracking, `build_truncated_input_vector()` |
| `RankingManager` | `view_state.rs` | Cyclic ranking shifts |
| `ViewOutput` | `view_state.rs` | `(view, slot, v_low, v_high, proof: QC3)` |
| `has_committable_low` | `view_state.rs` | Check if v_low has non-⊥ entry (commit condition) |
| `has_certifiable_high` | `view_state.rs` | Check if v_high has non-⊥ entry (cert creation condition) |
| `Certificate` / `DirectCert` / `IndirectCert` | `certificates.rs` | Certificate types with `validate()`, `hash()`, `v_high()`, `parent_view()` |
| `EmptyViewMessage` / `EmptyViewStatement` | `certificates.rs` | Empty-view signaling with signatures |
| `HighestKnownView` | `certificates.rs` | Tracks best known non-empty view with proof |
| `StrongPCCommit` | `certificates.rs` | Commit message with embedded chain + `verify()` |
| `StrongPCCommitError` | `certificates.rs` | 9 error variants for chain verification |
| `first_non_bot` | `utils.rs` | Find first non-⊥ in a PrefixVector |
| `cert_reaches_view1` | `certificates.rs` | Terminal condition: `parent_view() == 1` |
| `qc3_certify` | `certify.rs` | Derive (v_low, v_high) from QC3 |

---

## Chunk 1: Strong Protocol (`strong_protocol.rs`)

### Goal

Pure state machine for multi-view Strong PC decisions. No async, no I/O.

### Struct

```rust
pub struct StrongPrefixConsensusProtocol {
    // Configuration
    epoch: u64,
    slot: u64,

    // Certificate store: hash → cert (populated by manager as certs arrive)
    cert_store: HashMap<HashValue, Certificate>,

    // Highest known non-empty view (for empty-view messages)
    highest_known: HighestKnownView,

    // Outputs
    strong_v_low: Option<PrefixVector>,   // Set when View 1 completes
    strong_v_high: Option<PrefixVector>,  // Set when commit trace-back succeeds
    committed: bool,                       // True after successful commit or received StrongPCCommit
}
```

Key: No `RankingManager`, no `ViewState`, no per-view PC instances. Those live in the
manager. The protocol only tracks the cert store, outputs, and highest known view.

### Decision Types

```rust
/// Decision after View 1 completes
pub enum View1Decision {
    /// View 1 always produces a DirectCert (even all-⊥ v_high is meaningful in V1)
    DirectCert(DirectCertificate),
}

/// Decision after View W > 1 completes (three-way)
pub enum ViewDecision {
    /// v_low has non-⊥ entry → commit! Trace back to View 1.
    /// Contains the committing proof (QC3 from this view).
    Commit { committing_proof: QC3 },

    /// v_high has non-⊥ entry → create DirectCert, broadcast for next view.
    DirectCert(DirectCertificate),

    /// Both v_low and v_high are all-⊥ → empty view.
    /// Manager should broadcast EmptyViewMessage and collect >1/3 stake.
    EmptyView,
}
```

### Public API

```rust
impl StrongPrefixConsensusProtocol {
    /// Create new protocol instance
    pub fn new(epoch: u64, slot: u64) -> Self;

    // --- Certificate Store ---

    /// Store a certificate indexed by hash (called by manager as certs arrive)
    pub fn store_certificate(&mut self, cert: Certificate);

    /// Look up a certificate by hash
    pub fn get_certificate(&self, hash: &HashValue) -> Option<&Certificate>;

    // --- View Output Processing ---

    /// Process View 1 completion.
    ///
    /// Sets strong_v_low from output.v_low (this is the Strong PC low output).
    /// Always returns DirectCert decision (View 1 output is always meaningful).
    /// Updates highest_known if v_high is non-empty.
    pub fn process_view1_output(&mut self, output: ViewOutput) -> View1Decision;

    /// Process View W > 1 completion (three-way decision).
    ///
    /// a) has_committable_low(v_low) → Commit { committing_proof: output.proof }
    /// b) has_certifiable_high(v_high) → DirectCert(DirectCertificate::new(view, proof))
    /// c) else → EmptyView
    ///
    /// For cases (b) and (c), also updates highest_known if applicable.
    pub fn process_view_output(&mut self, output: ViewOutput) -> ViewDecision;

    // --- Trace-Back (Commit) ---

    /// Build certificate chain from committing view's QC3 back to View 1.
    ///
    /// Algorithm:
    /// 1. Derive v_low from committing_proof via qc3_certify()
    /// 2. first_non_bot(v_low) → starting cert hash H₀
    /// 3. Look up cert C₀ by H₀ in cert_store
    /// 4. If cert_reaches_view1(C₀): done, return (C₀.v_high(), [C₀])
    /// 5. Else: first_non_bot(C₀.v_high()) → next hash H₁, look up C₁
    /// 6. Repeat until reaching View 1 or missing cert
    ///
    /// Returns Err(MissingCert { hash }) if a cert is not in the store.
    /// The manager should fetch the missing cert and retry.
    pub fn build_certificate_chain(
        &self,
        committing_proof: &QC3,
    ) -> Result<(PrefixVector, Vec<Certificate>), ChainBuildError>;

    /// Build a complete StrongPCCommit message from a successful chain build.
    ///
    /// Calls build_certificate_chain internally. Returns the commit message
    /// ready for broadcast, or an error if the chain can't be built.
    pub fn build_commit_message(
        &self,
        committing_proof: &QC3,
    ) -> Result<StrongPCCommit, ChainBuildError>;

    /// Process a received StrongPCCommit from another party.
    ///
    /// Verifies the commit (delegates to StrongPCCommit::verify).
    /// If valid, sets strong_v_high and marks as committed.
    /// Returns error if verification fails.
    pub fn process_received_commit(
        &mut self,
        commit: &StrongPCCommit,
        verifier: &ValidatorVerifier,
    ) -> Result<(), StrongPCCommitError>;

    /// Mark as committed with the given v_high (after successful local trace-back).
    ///
    /// Called by the manager after build_commit_message succeeds.
    pub fn set_committed(&mut self, v_high: PrefixVector);

    // --- Highest Known View ---

    /// Update highest known view if this view's output is higher.
    /// Called internally by process_view1_output and process_view_output.
    /// Also callable by manager when receiving certs from other parties.
    pub fn update_highest_known(&mut self, view: u64, proof: QC3);

    /// Get current highest known view info (for building EmptyViewMessages)
    pub fn highest_known(&self) -> &HighestKnownView;

    // --- Output Queries ---

    /// Strong PC is complete when v_high is determined (committed or received commit)
    pub fn is_complete(&self) -> bool;

    /// Get Strong PC v_low output (from View 1)
    pub fn v_low(&self) -> Option<&PrefixVector>;

    /// Get Strong PC v_high output (from commit trace-back)
    pub fn v_high(&self) -> Option<&PrefixVector>;
}
```

### Error Type for Chain Building

```rust
pub enum ChainBuildError {
    /// A certificate needed for the chain is not in the local store.
    /// The manager should fetch it by hash and retry.
    MissingCert { hash: HashValue },

    /// v_low from committing proof has no non-⊥ entry (shouldn't happen if
    /// the manager only calls this after a Commit decision)
    NoCommitInVLow,

    /// A cert's v_high has no non-⊥ entry but cert doesn't reach View 1.
    /// This indicates a protocol error or corrupted cert.
    BrokenChain { cert_view: u64 },
}
```

### Tests for Chunk 1

All tests use hand-crafted `ViewOutput` values and pre-populated cert stores. No
networking, no async, no real signatures.

1. **View 1 decision**: Always returns DirectCert, sets strong_v_low
2. **Three-way decision — commit case**: v_low with non-⊥ → Commit
3. **Three-way decision — cert case**: v_low all-⊥, v_high non-⊥ → DirectCert
4. **Three-way decision — empty case**: both all-⊥ → EmptyView
5. **Certificate store**: store/retrieve by hash
6. **Trace-back — simple chain**: V3 → V2 → V1, all certs in store → success
7. **Trace-back — indirect cert**: chain ends at IndirectCert(parent=1) → success
8. **Trace-back — missing cert**: cert not in store → MissingCert error with correct hash
9. **Trace-back — single hop**: V2 → V1 (shortest possible chain)
10. **Build commit message**: wraps chain into StrongPCCommit correctly
11. **Process received commit**: valid commit → sets v_high, marks complete
12. **Process received commit — invalid**: bad commit → error, not complete
13. **Highest known tracking**: updates on higher views, ignores lower
14. **is_complete**: false until committed, true after

### Implementation Notes

- `process_view1_output` always creates DirectCert. View 1 v_high is always meaningful
  (raw inputs, not certificates). Even an all-⊥ v_high creates a valid DirectCert(V=1)
  per existing validation rules (`DirectCertificate::validate()` allows empty v_high
  only if view == 1).

- `process_view_output` for views > 1 uses `has_committable_low` and `has_certifiable_high`
  from `view_state.rs`. The three-way decision is mutually exclusive and checked in order:
  commit > cert > empty.

- `build_certificate_chain` follows the same algorithm as `StrongPCCommit::verify()` but
  in reverse — building the chain instead of verifying one. Both use `first_non_bot` and
  `cert_reaches_view1` (already implemented). The first step uses v_low (from the
  committing view), all subsequent steps follow v_high.

- The protocol does NOT validate certificates (no `ValidatorVerifier`). The manager
  validates certs before calling `store_certificate()`. The protocol trusts its cert store.
  Exception: `process_received_commit` delegates to `StrongPCCommit::verify()` which
  does validate (it needs the verifier parameter for this).

---

## Chunk 2: Network Messages

### Goal

Define `StrongPrefixConsensusMsg` enum and supporting types for the multi-view protocol.

### Location

Add to `consensus/prefix-consensus/src/network_messages.rs` (alongside existing
`PrefixConsensusMsg`).

### Types

```rust
/// Certificate proposal for a view
///
/// When a party creates a certificate from completing a view, it broadcasts the
/// full certificate as a proposal for the next view. Other parties store it and
/// use its hash in their truncated input vectors.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ViewProposal {
    /// The view this proposal is FOR (the next view after the cert was created)
    pub target_view: u64,
    /// The certificate (from completing the previous view)
    pub certificate: Certificate,
    /// Epoch for validation
    pub epoch: u64,
    /// Slot for multi-slot consensus
    pub slot: u64,
}

/// Request to fetch a certificate by hash
///
/// Sent when a party needs a certificate for trace-back but doesn't have it
/// in its local store (typically because it entered a view via case (b) and
/// skipped intermediate views).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CertFetchRequest {
    /// Hash of the certificate to fetch
    pub cert_hash: HashValue,
    /// Epoch for validation
    pub epoch: u64,
}

/// Response to a certificate fetch request
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CertFetchResponse {
    /// Hash of the requested certificate
    pub cert_hash: HashValue,
    /// The certificate (if the responder has it)
    pub certificate: Option<Certificate>,
    /// Epoch for validation
    pub epoch: u64,
}

/// Network message type for Strong Prefix Consensus
///
/// Wraps all message types for the multi-view protocol, including inner PC
/// messages (Vote1/2/3) tagged with view numbers.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum StrongPrefixConsensusMsg {
    /// Inner PC message for a specific view (Vote1, Vote2, or Vote3)
    InnerPC {
        view: u64,
        msg: PrefixConsensusMsg,
    },

    /// Certificate proposal for the next view
    Proposal(Box<ViewProposal>),

    /// Empty-view message (from certificates.rs EmptyViewMessage)
    EmptyView(Box<EmptyViewMessage>),

    /// Commit announcement with full proof chain
    Commit(Box<StrongPCCommit>),

    /// Certificate fetch request
    FetchRequest(CertFetchRequest),

    /// Certificate fetch response
    FetchResponse(Box<CertFetchResponse>),
}
```

### Helper Methods

```rust
impl StrongPrefixConsensusMsg {
    /// Get the epoch of this message
    pub fn epoch(&self) -> u64;

    /// Get the view this message relates to (for routing)
    pub fn view(&self) -> Option<u64>;

    /// Message type name for logging
    pub fn name(&self) -> &'static str;

    /// Get the author/sender (where applicable)
    pub fn author(&self) -> Option<PartyId>;
}

impl ViewProposal {
    pub fn new(target_view: u64, certificate: Certificate, epoch: u64, slot: u64) -> Self;
}
```

### Tests for Chunk 2

1. **Serialization roundtrip**: Each variant serializes/deserializes correctly
2. **Helper methods**: epoch(), view(), name() return correct values
3. **ViewProposal construction**: new() sets fields correctly
4. **Message size**: Verify messages are reasonably sized

### Notes

- `EmptyViewMessage` already exists in `certificates.rs` with full implementation
  (including signing/verification). We reuse it directly — no new empty-view type needed.

- Inner PC messages are wrapped with a view number for routing. The `PrefixConsensusMsg`
  inside contains Vote1/Vote2/Vote3 which already have epoch, slot, and view fields.
  The outer `view` field is redundant but makes routing cheaper (no need to deserialize
  the inner message to extract the view).

- `CertFetchResponse` has non-optional `Certificate` — if responder doesn't have the cert,
  it simply doesn't reply. Manager handles timeout by trying other peers.

### Implementation Notes (Deviations from Plan)

- Data types (`ViewProposal`, `CertFetchRequest`, `CertFetchResponse`) moved to `types.rs`
  (consistent with Vote1/2/3 pattern). `StrongPrefixConsensusMsg` enum stays in `network_messages.rs`.
- `CertFetchResponse.certificate` changed from `Option<Certificate>` to `Certificate` — no
  point sending an empty response.
- Added `slot` field to `CertFetchRequest` and `CertFetchResponse` (not in original plan).
- `EmptyViewMessage` now carries `epoch` and `slot` fields (added during Chunk 3 review) for
  uniform epoch/slot filtering in `process_message`.

---

## Chunk 3: Strong Manager (`strong_manager.rs`) — ✅ COMPLETE

### Goal

Async event loop that orchestrates the multi-view protocol, driving per-view PC
instances and the Strong Protocol state machine.

### Struct

```rust
pub struct StrongPrefixConsensusManager<NetworkSender> {
    // Identity
    party_id: PartyId,
    epoch: u64,
    slot: u64,

    // Protocol (pure state machine — Chunk 1)
    protocol: StrongPrefixConsensusProtocol,

    // Per-view state
    ranking_manager: RankingManager,
    current_view: u64,
    view_states: HashMap<u64, ViewState>,             // Views > 1 only

    // Per-view PC instances (the swappable inner algorithm)
    // Today: PrefixConsensusProtocol. Future: any PC implementation.
    view_protocols: HashMap<u64, Arc<PrefixConsensusProtocol>>,

    // Duplicate detection
    seen_proposals: HashMap<u64, HashSet<PartyId>>,   // Per-view proposal dedup
    seen_empty_views: HashMap<u64, HashSet<PartyId>>, // Per-view empty-view dedup

    // Empty-view collection (for IndirectCertificate creation)
    empty_view_collectors: HashMap<u64, Vec<EmptyViewMessage>>,

    // Pending fetches (hash → number of attempts)
    pending_fetches: HashMap<HashValue, u32>,

    // Network and signing
    network_sender: NetworkSender,
    validator_signer: ValidatorSigner,
    validator_verifier: Arc<ValidatorVerifier>,
}
```

### Event Loop

```rust
pub async fn run(
    mut self,
    mut message_rx: UnboundedReceiver<(Author, StrongPrefixConsensusMsg)>,
    mut close_rx: oneshot::Receiver<oneshot::Sender<()>>,
) {
    // Phase 1: Collect View 1 proposals (raw inputs from other parties)
    // Wait for timeout (300ms) to gather as many inputs as possible
    // Then start View 1's PC instance

    self.start_view1().await;

    loop {
        tokio::select! {
            Some((author, msg)) = message_rx.recv() => {
                self.process_message(author, msg).await;

                if self.protocol.is_complete() {
                    self.write_output().await;
                    break;
                }
            }
            _ = &mut close_rx => {
                break;
            }
        }
    }
}
```

### Message Routing

```rust
async fn process_message(&mut self, author: Author, msg: StrongPrefixConsensusMsg) {
    // 1. Epoch + slot check: reject if msg.epoch() != self.epoch or msg.slot() != self.slot
    //    This prevents cross-slot replays (StrongPCCommit and EmptyViewMessage proofs
    //    don't verify slot internally — the manager must filter).

    match msg {
        InnerPC { view, msg } => self.process_inner_pc(author, view, msg).await,
        Proposal(proposal) => self.process_proposal(author, *proposal).await,
        EmptyView(empty) => self.process_empty_view(author, *empty).await,
        Commit(commit) => self.process_commit(author, *commit).await,
        FetchRequest(req) => self.process_fetch_request(author, req).await,
        FetchResponse(resp) => self.process_fetch_response(author, *resp).await,
    }
}
```

### Key Flows

#### Flow 1: View 1 Lifecycle

```
1. Wait 300ms timeout for raw input proposals (TODO: optimize)
2. Create PrefixConsensusProtocol for view 1 with raw input vector
3. Start round 1 → broadcast Vote1 (wrapped in InnerPC { view: 1, ... })
4. Process incoming InnerPC messages for view 1:
   - Route Vote1/2/3 to view 1's protocol instance
   - On QC1: start round 2
   - On QC2: start round 3
   - On QC3: view 1 complete
5. Call protocol.process_view1_output(output)
   → Always returns View1Decision::DirectCert(cert)
6. Store cert in protocol.cert_store
7. Broadcast ViewProposal { target_view: 2, certificate: cert }
8. Enter view 2
```

#### Flow 2: View W > 1 Lifecycle

```
Entry conditions (EITHER):
  (a) Completed view W-1, created certificate, ready to propose
  (b) Received valid ViewProposal for view W from another party

On entry:
1. Create ViewState for view W with ranking from RankingManager
2. Add received certificates to ViewState
3. Wait for start condition:
   - Eager: first certificate from highest-ranked party arrives
   - Timeout: 300ms fallback (TODO: optimize)
4. Build truncated input vector from ViewState
5. Create PrefixConsensusProtocol for view W with truncated vector
6. Start round 1 → broadcast Vote1 (wrapped in InnerPC { view: W, ... })
7. Process incoming InnerPC messages for view W (same as view 1)
8. On view W complete, call protocol.process_view_output(output):

   Case Commit { committing_proof }:
     → Try protocol.build_commit_message(committing_proof)
     → If Ok(commit): broadcast StrongPCCommit, set committed, DONE
     → If Err(MissingCert { hash }): initiate fetch, continue running

   Case DirectCert(cert):
     → Store cert in protocol.cert_store
     → Broadcast ViewProposal { target_view: W+1, certificate: cert }
     → Enter view W+1

   Case EmptyView:
     → Create EmptyViewMessage with protocol.highest_known()
     → Sign and broadcast EmptyViewMessage
     → Collect incoming EmptyViewMessages for view W
     → When >1/3 stake collected: create IndirectCertificate
     → Store IndirectCert in protocol.cert_store
     → Broadcast ViewProposal { target_view: W+1, certificate: indirect_cert }
     → Enter view W+1
```

#### Flow 3: Processing ViewProposal

```
1. Validate: epoch matches, target_view > current completed view
2. Duplicate check: reject if already have proposal from this author for this view
3. Validate certificate signatures (cert.validate(verifier))
4. Store certificate in protocol.cert_store (by hash)
5. Add certificate to ViewState for target_view (if ViewState exists)

6. If we haven't entered target_view yet (case (b) trigger):
   → Enter target_view: create ViewState, add cert
   → Start target_view's PC (with timeout or immediately if top-ranked)

7. If we have entered target_view but haven't started PC yet:
   → Add cert to ViewState
   → Check if start condition is met (top-ranked cert arrived)
```

#### Flow 4: Processing StrongPCCommit (from another party)

```
1. Call protocol.process_received_commit(commit, verifier)
2. If Ok: protocol is now complete (v_high set), exit event loop
3. If Err: log warning, ignore (invalid commit)
```

#### Flow 5: Certificate Fetching

```
On MissingCert { hash } during trace-back:
1. Record hash in pending_fetches with attempt count
2. Broadcast CertFetchRequest { cert_hash: hash, epoch }
3. Continue event loop (don't block)

On receiving CertFetchResponse:
1. If certificate is Some:
   a. Verify: hash matches, cert.validate(verifier) passes
   b. Store in protocol.cert_store
   c. If we have a pending commit (waiting for trace-back):
      → Retry protocol.build_commit_message()
      → If Ok: broadcast StrongPCCommit, set committed, DONE
      → If Err(MissingCert { next_hash }): fetch next_hash
2. If certificate is None: try fetching from other peers

On receiving CertFetchRequest:
1. Look up hash in protocol.cert_store
2. If found: respond with CertFetchResponse { cert_hash, certificate: Some(cert) }
3. If not found: respond with CertFetchResponse { cert_hash, certificate: None }

Fallback:
- If fetch attempts exceed threshold (e.g., 10): stop fetching, rely on
  receiving StrongPCCommit from another party who assembled the chain
- TODO: Add retry backoff, targeted fetching, DOS protection
```

#### Flow 6: Empty-View Handling

```
When process_view_output returns EmptyView:
1. Get highest_known from protocol
2. Create EmptyViewStatement { empty_view: W, highest_known_view }
3. Sign statement with validator_signer
4. Build EmptyViewMessage { empty_view: W, author, highest_known_view,
   highest_known_proof, signature }
5. Broadcast to all peers
6. Add our own message to empty_view_collectors[W]

On receiving EmptyViewMessage for view W:
1. Verify signature
2. Duplicate check
3. Add to empty_view_collectors[W]
4. Check if total stake of collected messages > 1/3:
   → If yes: create IndirectCertificate from collected messages
   → Store IndirectCert in protocol.cert_store
   → Broadcast ViewProposal { target_view: W+1, certificate: indirect_cert }
   → Enter view W+1
```

### Tests for Chunk 3

Testing the manager requires mocking the network sender. Use the same pattern as
`PrefixConsensusManager` tests (generic `NetworkSender` trait).

1. **View 1 lifecycle**: Start → complete → DirectCert broadcast
2. **View transition**: View 1 complete → proposal broadcast → view 2 starts
3. **Case (b) entry**: Receive proposal for view 3 without completing view 2 → enter view 3
4. **Three-way decision — commit**: View completes with committable v_low → StrongPCCommit broadcast
5. **Three-way decision — cert**: View completes with certifiable v_high → DirectCert → next view
6. **Three-way decision — empty**: View completes all-⊥ → EmptyViewMessage → IndirectCert
7. **StrongPCCommit receipt**: Valid commit from another party → output set, complete
8. **Certificate fetching**: Missing cert during trace-back → fetch request → response → retry → success
9. **Fetch fallback**: Missing cert, fetch fails → wait for StrongPCCommit from others
10. **Duplicate rejection**: Same proposal twice from same party → second rejected
11. **Epoch filtering**: Wrong epoch message → rejected
12. **Message routing**: InnerPC messages routed to correct view's protocol

### Implementation Notes (Deviations from Plan)

**Struct changes from plan**:
- Renamed `ViewPCState` → `PCState`, `view_pc_states` → `pc_states` for clarity
- Removed `seen_vote1/2/3` dedup HashSets — inner PC already handles vote deduplication
- Added `view_start_timer: Option<Pin<Box<Sleep>>>` for 300ms timeout mechanism
- Added `pending_commit_proof: Option<QC3>` for storing committing proof while fetching missing certs
- `view_protocols` not needed as separate map — inner PC instances live inside `PCState`

**Key implementation decisions**:

1. **View start timer** (not in original plan):
   - 300ms timer starts when entering a view
   - Cancelled early if first-ranked certificate arrives (`try_start_pc`)
   - When timer fires, `start_pc_now` runs inner PC with whatever certificates are available
   - All parties must participate to ensure quorum formation — no skipping inner PC

2. **Proposal adoption** (not in original plan):
   - When receiving a valid proposal for `target_view > current_view`, adopt the certificate
   - `propose_and_enter()` broadcasts as own proposal, adds to ViewState, enters view
   - Guard: `next_view <= current_view` prevents double proposals (Byzantine behavior)
   - Honest parties adopt certificates to help others catch up — not just forwarding

3. **Early QC bug fix** (cross-cutting, affects protocol.rs + manager.rs):
   - `start_round1/2/3` internally process self-vote which can form QC
   - Changed return types to `(Vote, Option<QC>)` to surface early QCs
   - Both `manager.rs` and `strong_manager.rs` now handle early QCs
   - Required `Box::pin` on `start_pc_now` for recursive async future sizing

4. **Fetch DoS protection**:
   - `process_fetch_response` checks `pending_fetches.contains_key(&hash)` before any work
   - Entry stays in `pending_fetches` until successful validation (failed validation = retry later)
   - Unsolicited responses silently dropped

5. **ViewProposal signing not needed**:
   - Network authenticates sender identity
   - Certificate inside is validated via `cert.validate(&verifier)`
   - Any party can propose any valid certificate — design supports proposal adoption

6. **EmptyViewMessage epoch/slot** (deviation from Chunk 2 plan):
   - Added `epoch` and `slot` fields to `EmptyViewMessage` struct
   - Enables uniform epoch/slot filtering in `process_message` for all message types
   - `StrongPrefixConsensusMsg::epoch()/slot()` for EmptyView variant now reads from message

7. **IndirectCertificate::from_messages** changed to return `Option<Self>` instead of `Result`:
   - Insufficient stake is a normal condition (not enough messages yet), not an error

8. **`finalize_view` extracted helper**:
   - Handles commit trace-back, StrongPCCommit broadcast, cert fetching for missing links
   - Called from both `handle_view_complete` and `start_pc_now` (early QC path)

---

## File Structure After Implementation

```
consensus/prefix-consensus/src/
├── strong_protocol.rs    - Strong PC state machine (Chunk 1, 918 lines)
├── network_messages.rs   - PrefixConsensusMsg + StrongPrefixConsensusMsg (Chunk 2, 795 lines)
├── strong_manager.rs     - Strong PC event loop (Chunk 3, 1394 lines)
├── certificates.rs       - Certificate types + StrongPCCommit (1348 lines)
├── view_state.rs         - ViewState, RankingManager, ViewOutput (695 lines)
├── protocol.rs           - Basic PC protocol (606 lines, modified for early QC)
├── manager.rs            - Basic PC manager (737 lines, modified for early QC)
├── types.rs              - Vote/QC types + ViewProposal/CertFetch (1039 lines)
├── certify.rs            - QC formation (220 lines)
├── verification.rs       - Validation (150 lines)
├── utils.rs              - Prefix operations + first_non_bot (210 lines)
├── signing.rs            - BLS helpers (184 lines)
├── network_interface.rs  - Network adapter (224 lines)
└── lib.rs                - Exports (updated for new modules)
```

**Total**: ~7,532 lines across all source files

---

## Implementation Order

1. **Chunk 1** (`strong_protocol.rs`): ✅ Pure state machine. 27 unit tests.

2. **Chunk 2** (`network_messages.rs` additions): ✅ Message types. 17 unit tests.

3. **Chunk 3** (`strong_manager.rs`): ✅ Event loop + cross-cutting fixes. 186 total tests.

---

## TODO Items (Future Optimization)

- [ ] **Timeout tuning**: Explore adaptive timeouts for View 1 (300ms) and views > 1.
      Consider network-condition-based delays and whether waiting for multiple certs
      improves output quality vs latency tradeoff.

- [ ] **View entry optimization**: Currently start view W+1 on first cert from
      highest-ranked party or timeout. Explore whether waiting for a specific number
      of certs or specific ranked parties improves convergence speed.

- [ ] **Fetch optimization**: Targeted fetching (request from parties who voted for the
      hash rather than broadcasting to all). Retry backoff. DOS rate limiting.

- [ ] **Inner PC abstraction**: When optimized PC algorithms are available (e.g., 2-round
      good case from paper Appendix D), define a trait for the inner PC and make the
      manager generic over it.

- [ ] **StrongPCCommit chain cap**: If chains become long (>5 certs), cap embedded certs
      and use hashes for the rest. Monitor chain lengths in practice.

- [ ] **Certificate storage limits**: LRU cache or eviction for cert store if memory
      becomes a concern in long-running protocols.

- [ ] **Configurable view start timeout**: `VIEW_START_TIMEOUT` (300ms) is hardcoded.
      Make it configurable via consensus config or constructor parameter.

- [ ] **Empty view without inner PC**: Send EmptyViewMessage directly when no certs arrive
      at timeout (saves 3 rounds). Only applies when ALL parties have no certs for the view.

- [ ] **View 1 start timer for multi-slot**: Add delay before View 1 in Slot Manager
      context to collect input vectors from other slots.

- [ ] **Garbage collect on slot commit**: Clean up `view_states`, `pc_states`,
      `empty_view_collectors`, `pending_fetches`, `pending_commit_proof`, cert store.
