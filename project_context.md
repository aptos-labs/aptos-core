# Project Context: Prefix Consensus Implementation

## Overview

Implementing Prefix Consensus protocols (from research paper "Prefix Consensus For Censorship Resistant BFT") within Aptos Core for leaderless, censorship-resistant consensus.

**Current Phase**: Strong Prefix Consensus - Phase 9 (Inner PC Abstraction) complete, Phase 10 (Performance) next
**Completed**: Basic Prefix Consensus, Strong PC through Phase 9 (Inner PC Abstraction)

---

## Background Summary

### AptosBFT (Current Consensus)
- Leader-based BFT protocol (Jolteon)
- 3-chain commit rule, partial synchrony
- Central components: RoundManager, BlockStore, SafetyRules
- Limitations: Single leader per round (censorship risk, partial synchrony requirement)

### Prefix Consensus Protocol
- **Definition**: Parties propose vectors, output compatible vectors extending max common prefix
- **Properties**: Upper Bound (v_low ⪯ v_high), Validity (mcp(inputs) ⪯ v_low), Termination
- **Algorithm**: 3 rounds (Vote1→QC1, Vote2→QC2, Vote3→QC3)
  - Round 1: Broadcast inputs, certify longest prefix with >1/3 stake agreement
  - Round 2: Broadcast certified prefixes, compute mcp
  - Round 3: Output v_low (mcp), v_high (mce)
- **Quorum**: Proof-of-stake weighted voting (>2/3 stake for QCs, >1/3 for certification)
- **Benefits**: Deterministically async, leaderless, censorship-resistant

**Key Difference**: No single agreed value, just compatible prefixes. Enables async solvability.

---

## Basic Prefix Consensus - Implementation Complete ✅

**Status**: Production-ready 3-round asynchronous protocol with full signature verification
**Completion Date**: February 4, 2026
**Tests**: 103/103 unit tests passing (smoke tests require manual output file verification)

### Key Components

**Core Implementation** (~3300 LOC):
- `types.rs` - Vote1/2/3, QC1/2/3 with BLS signatures (350 lines)
- `protocol.rs` - 3-round state machine (490 lines)
- `manager.rs` - Event-driven orchestrator (658 lines)
- `network_interface.rs` - Network adapter (224 lines)
- `network_messages.rs` - PrefixConsensusMsg enum (443 lines)
- `signing.rs` - BLS sign/verify helpers (184 lines)
- `certify.rs` - QC formation with trie (220 lines)
- `verification.rs` - Vote/QC verification (150 lines)
- `utils.rs` - mcp/mce prefix operations (180 lines)

**Integration**:
- `consensus/src/epoch_manager.rs` - start_prefix_consensus() method
- `consensus/src/network_interface.rs` - ConsensusMsg routing
- Self-send channel pattern for local message delivery
- Direct routing in check_epoch() (bypasses UnverifiedEvent)

**Testing**:
- Unit tests: All 186 tests passing (includes strong PC tests)
- Smoke tests: 4 tests, all passing with in-code property verification
  - Basic PC: `test_prefix_consensus_identical_inputs`, `test_prefix_consensus_divergent_inputs`
  - Strong PC: `test_strong_prefix_consensus_identical_inputs`, `test_strong_prefix_consensus_divergent_inputs`
  - Output files: `/tmp/{prefix,strong_prefix}_consensus_output_{party_id:x}.json`
- Test scripts: `test_prefix_consensus.sh`, `test_strong_prefix_consensus.sh`

### Technical Achievements

**Signature System**: BLS12-381 with ValidatorSigner/ValidatorVerifier integration
**Race Condition Fix**: Removed strict state checks (votes are self-contained)
**Network Integration**: Generic trait-based sender with NetworkSenderAdapter
**Architecture**: Arc<RwLock<>> state, tokio::select! event loop, structured logging

### Security Enhancements (Phase 1 - Verifiable Prefix Consensus)

**Author Mismatch Check**: Added to manager.rs process_vote1/2/3()
- Prevents impersonation attacks where Byzantine party claims to be someone else
- Check: vote.author must equal network sender
- Signature alone is insufficient (sender can sign data claiming to be another party)

**Full Signature Verification**: Added to verification.rs
- All verify_vote*() functions now verify BLS signatures via ValidatorVerifier
- All verify_qc*() functions recursively verify all embedded vote signatures
- QC3 → Vote3 → QC2 → Vote2 → QC1 → Vote1: Signatures verified at every level
- PrefixConsensusOutput::verify() now requires ValidatorVerifier parameter
- Protocol stores ValidatorVerifier for internal verification

**Proof Verification Functions**: Added to verification.rs and types.rs
- verify_low_proof(): Verifies v_low derivation from QC3
- verify_high_proof(): Verifies v_high derivation from QC3
- PrefixConsensusOutput::verify(): Complete verification including QC3 structure and signatures

### Certificate Types (Phase 2 - Complete)

**DirectCertificate**: Created when a view produces non-empty v_high
- Contains: (view, proof: QC3)
- v_high derived from QC3 via qc3_certify() - not stored
- Parent view = this view (direct link)

**EmptyViewMessage**: Broadcast when view produces empty v_high
- Contains: (empty_view, author, highest_known_view, highest_known_proof, signature)
- Enables indirect certificate construction

**IndirectCertificate**: Aggregates EmptyViewMessages from validators with >1/3 stake
- Takes MAX parent_view from all messages
- Allows skipping empty views when tracing back to View 1

**Certificate Enum**: Unified interface for both certificate types
- view(), parent_view(), v_high(), validate(), hash()

**HighestKnownView Tracker**: Helper for creating EmptyViewMessages
- Tracks highest non-empty view with proof
- update_if_higher() for efficient tracking

**Empty v_high Corner Case** (View 1 special handling):
- View 1 can have empty v_high (e.g., inputs conflict at first entry)
- Views 2+ must have non-empty v_high; empty means use EmptyViewMessage instead
- HighestKnownView::update_if_higher() accepts empty v_high only if view == 1
- EmptyViewMessage::verify() allows empty v_high only if highest_known_view == 1
- DirectCertificate::validate() allows empty v_high only if view == 1
- IndirectCertificate::from_messages() requires >1/3 stake (checked via ValidatorVerifier)

**View Field for Replay Protection** (Phase 2 security fix):
- Added `view: u64` to Vote1, Vote2, Vote3 structs and their SignData types
- Added `view: u64` to PrefixConsensusInput (default 1 for standalone basic PC)
- View is included in signed data to prevent replay attacks across views
- Added qc1_view(), qc2_view(), qc3_view() helpers to extract view from QCs
- Certificate validation checks: qc3_view(proof) must match certificate's view
- **Note**: QC1/2/3 do NOT have explicit view field - view is derived from votes to avoid redundancy

**TODO - Slot Number for Replay Protection**:
- When implementing multi-slot consensus (Slot Manager), add `slot: u64` to signed vote data
- This prevents replay attacks across slots (same view number in different slots)
- Similar pattern to view field: include in SignData, verify during QC validation

---

## Strong Prefix Consensus - In Progress 🚧

**Goal**: Multi-view protocol where all parties agree on identical v_high output
**Status**: Phases 1-9 complete (core implementation + smoke tests + inner PC abstraction), Phase 10 (Performance) next
**Plan**: `.plans/strong-prefix-consensus.md`

### What is Strong Prefix Consensus?

From paper Definition 2.5: Extends basic Prefix Consensus by adding **Agreement** property:
- Basic Prefix Consensus: Parties output compatible v_high values (v_low ⪯ v_high)
- Strong Prefix Consensus: ALL parties output IDENTICAL v_high value

### Architecture Overview

```
StrongPrefixConsensusManager
  │
  ├─> View 1: Run Verifiable Prefix Consensus on input vector
  │   - Output v_low immediately (becomes Strong PC low output)
  │   - v_high becomes candidate for agreement
  │
  ├─> View 2+: Run Verifiable Prefix Consensus on certificate vectors
  │   - Certificates create parent chains back to View 1
  │   - Following parent pointers determines unique v_high
  │   - Cyclic ranking shifts ensure leaderless progress
  │
  └─> Output: (v_low, v_high) where all parties agree on v_high
```

### Key Concepts

**Verifiable Prefix Consensus**: Basic Prefix Consensus + proofs (QC3 serves as π)
**Direct Certificates**: Cert^dir(w-1, v_high, π) - advance view-by-view
**Indirect Certificates**: Cert^ind(w-1, (w*, v_high*, π), Σ) - skip empty views with >1/3 stake sigs
**Parent Chain**: Following certificates back to View 1 uniquely determines v_high output

### Truncated Vector Optimization (Not in Paper)

**Key insight**: In views > 1, only the first non-⊥ entry in v_low/v_high is used for tracing back.

**Optimization**: Instead of full certificate vectors `[cert_p1, cert_p2, cert_p3, cert_p4]`, truncate after the first non-⊥ entry. Example: `[⊥, ⊥, cert_A, cert_B]` becomes `[⊥, ⊥, cert_A]`.

**Benefits**:
- Vectors are small (typically length 1-3 instead of n)
- No need for certificate fetching mechanism (each party broadcasts full cert, not hash)
- Simpler implementation, same correctness guarantees
- Entries after first non-⊥ are provably unused in trace-back logic

### Implementation Plan (10 Phases)

1. **Verifiable Prefix Consensus** - Add proof outputs to basic protocol
2. **Certificate Types** - Direct/Indirect certificate structures and validation
3. **View State Management** - Per-view state tracking and cyclic ranking
4. **Strong Protocol Core** - Multi-view state machine (~800 lines, most complex)
5. **Network Messages** - Message types with view multiplexing
6. **Strong Manager** - Event-driven orchestrator for multi-view protocol
7. **Integration** - Wire into EpochManager
8. **Smoke Tests** - 4 test cases validating agreement property
9. **Performance** - Metrics and optimization
10. **Documentation** - README and context updates

**Estimated Effort**: 10-11 days of focused development
**New Code**: ~3000 lines (certificates, view_state, strong_protocol, strong_manager, tests)

### Future Architecture

```
SlotManager (Future)
  │
  └─> Per-Slot: StrongPrefixConsensusManager
        │
        └─> Per-View: VerifiablePrefixConsensusProtocol
              │
              └─> Basic PrefixConsensusProtocol (3 rounds)
```

This layered architecture enables multi-slot censorship-resistant consensus as described in paper Algorithm 2.

---

## Stake-Weighted Quorum Refactoring - Complete ✅

**Completed**: February 6, 2026
**Purpose**: Replace count-based quorum (f+1, n-f) with proof-of-stake weighted voting

### Key Changes

**Before** (count-based):
- `f+1` validators for minority quorum (certification)
- `n-f` (2f+1) validators for super majority (QC formation)
- `PrefixConsensusInput::new(vec, party, n, f, epoch, view)`
- `PrefixConsensusManager::new(party, n, f, epoch, ...)`

**After** (stake-based):
- `>1/3` of total stake for minority quorum (certification)
- `>2/3` of total stake for super majority (QC formation)
- `PrefixConsensusInput::new(vec, party, epoch, view)`
- `PrefixConsensusManager::new(party, epoch, ...)`

### Files Modified

1. **verification.rs**: Removed f, n from all verify functions; use `ValidatorVerifier::check_voting_power()`
2. **certify.rs**: Updated trie to track cumulative stake; minority threshold = `total - quorum + 1`
3. **types.rs**: Removed f, n from `PrefixConsensusInput`; `PendingVotes*::has_quorum()` takes `&ValidatorVerifier`
4. **protocol.rs**: Uses stake-based `has_quorum(&validator_verifier)`
5. **manager.rs**: Removed n, f fields; logging shows validator_count and total_stake
6. **epoch_manager.rs**: Updated integration to use new constructor signatures
7. **certificates.rs**: Updated validation to use ValidatorVerifier for >1/3 stake checks

### Tests
- All 134 unit tests pass
- Build compiles cleanly

---

## Current Status (February 12, 2026)

### Repository State
- **Branch**: `prefix-consensus-prototype`
- **HEAD**: Phase 9 complete (Inner PC Abstraction Trait + signing refactor)
- **Tests**: 190/190 unit tests passing, 4/4 smoke tests passing
- **Build**: ✅ Clean

### Progress Summary
- ✅ **Basic Prefix Consensus**: Complete (Phase 1-6 of network-integration.md)
- ✅ **Strong PC Phase 1**: Verifiable Prefix Consensus complete (security fixes + proof verification)
- ✅ **Strong PC Phase 2**: Certificate types complete (Direct/Indirect certificates + view field)
- ✅ **Stake-Weighted Quorum**: Refactored from count-based to proof-of-stake weighted voting
- ✅ **Strong PC Phase 3**: View State and Ranking Management complete (RankingManager, ViewState, ViewOutput)
- ✅ **Strong PC Phase 4 Chunk 1**: Strong Protocol state machine complete (strong_protocol.rs)
- ✅ **Strong PC Phase 4 Chunk 2**: Network messages complete (StrongPrefixConsensusMsg + types)
- ✅ **Strong PC Phase 4 Chunk 3**: Strong Manager complete (strong_manager.rs) + cross-cutting fixes
- ✅ **Strong PC Phase 5**: Integration with consensus layer (EpochManager wiring + network bridges)
- ✅ **Strong PC Phase 8 (Smoke Tests)**: 2 strong PC tests + 2 basic PC tests, 4 integration bugs fixed
- ✅ **Strong PC Phase 9 (Inner PC Abstraction)**: InnerPCAlgorithm trait + ThreeRoundPC impl + generic manager + signing factory functions
- ⏳ **Slot Manager**: Future work (after Strong PC complete)

**Main Design Plan**: `.plans/strong-prefix-consensus.md`
- Contains full implementation plan with all phases
- Key design decisions documented
- Architecture diagrams and code sketches
- TODO section for deferred items

### Phase 3 Implementation Details

**New file**: `consensus/prefix-consensus/src/view_state.rs` (~360 lines)

**RankingManager**: Cyclic ranking shifts for leaderless progress
- View 1: [p1, p2, p3, p4]
- View 2: [p2, p3, p4, p1] (rotate left by 1)
- Formula: rotate left by (v - 1) % n positions

**ViewState**: Per-view state tracking for views > 1 only
- Stores received certificates per party
- `build_truncated_input_vector()`: Truncates after first non-⊥ (optimization not in paper)
- `get_first_certificate()`: Returns certificate at first non-⊥ position for trace-back

**ViewOutput**: Output from completing a view's Prefix Consensus
- Contains (view, slot, v_low, v_high, proof: QC3)

**Slot field**: Added `slot: u64` to `PrefixConsensusInput` for future multi-slot integration (default 0)

### Critical Edge Case: Three-Way Decision for Views > 1

When a view > 1 completes, the protocol must make a **three-way decision** based on the output:

```
a) If v_low has non-⊥ entry (has_committable_low):
   → Commit! Trace back to View 1, output Strong PC v_high, DONE

b) Else if v_high has non-⊥ entry (has_certifiable_high):
   → Create DirectCertificate from v_high
   → Broadcast for next view (progress made, no commit yet)

c) Else (both v_low and v_high are all-⊥):
   → Empty view, no progress
   → Broadcast EmptyViewMessage, collect >1/3 stake, create IndirectCertificate
```

**Key insight**: A vector like `[⊥, ⊥, ⊥]` is NOT meaningful in views > 1 (no certificate to trace). But in View 1, even all-⊥ outputs are valid (raw inputs, not certificates).

**Helper method (implemented in view_state.rs)**:
- `has_non_bot_entry(vector)`: True if any entry is not `HashValue::zero()` — used for both commit decision (v_low) and certificate creation (v_high)

### Message Size Optimization: Hashes vs Full Certificates

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

### Termination Mechanism: StrongPCCommit

**Problem**: When one party commits (traces back to View 1), it might stop participating. Other parties might not have committed yet and could lose quorum for progress.

**Solution**: When a party commits, it broadcasts a `StrongPCCommit` message containing:
- `committing_proof`: QC3 from the committing view (proves v_low had a non-⊥ entry)
- `certificate_chain`: Certs traced from v_low back to View 1 via v_high hash links
- `v_high`: The final Strong PC output (View 1's v_high)
- `epoch` and `slot`: For validation

**Why full certs in StrongPCCommit but hashes in view-by-view**:
- StrongPCCommit is one message (not aggregated N times) → O(chain × N) = O(N)
- View-by-view votes are aggregated N times → O(N²) if full certs
- Chain is typically short (2-5 certs)

**Verification by receiver (hash-based chain linkage)**:
1. Validate `committing_proof` QC3 signatures
2. Derive `v_low = qc3_certify(committing_proof).0`
3. `first_non_bot(v_low)` → H₀, verify H₀ == `hash(chain[0])`
4. Walk chain: `first_non_bot(chain[i].v_high()) == hash(chain[i+1])`
5. Terminal: `cert_reaches_view1(chain[k])` (checks `parent_view() == 1`)
6. Output match: `chain[k].v_high() == claimed v_high`

**Terminal condition — why `parent_view() == 1` not `view() == 1`**:
The last cert in the chain can be either DirectCert(V=1) or IndirectCert(empty=V, parent=1).
Both give View 1's v_high, but only `parent_view()` handles both cases. The IndirectCert
case arises when an intermediate view was empty — e.g., View 2 is empty, so
IndirectCert(empty=2, parent=1) is created. If the chain traces to it, `view()=2` but
`parent_view()=1`. We stop here because its `v_high` contains raw transaction hashes
(from View 1's QC3), not certificate hashes.

**Design choices**:
- Include committing proof (QC3) so receivers can verify the commit condition
- Include full certificate chain (no fetching needed)
- Committing party stops participating immediately after broadcast
- Network guarantees message delivery

**Reusable helpers** (for both verification and local trace-back in Phase 4):
- `first_non_bot(vec)` — find first non-⊥ entry in a prefix vector
- `cert_reaches_view1(cert)` — terminal condition (`parent_view() == 1`)

**Implementation**:
- `StrongPCCommit` struct with `new()`, `committing_view()`, `verify()` — in `certificates.rs`
- `StrongPCCommitError` enum with 9 descriptive error variants — in `certificates.rs`
- `cert_reaches_view1()` terminal condition helper — in `certificates.rs`
- `first_non_bot()` prefix vector utility — in `utils.rs`
- All exported from `lib.rs`

### Phase 4 Implementation Details (Chunks 1-3 Complete)

**Chunk 1 — Strong Protocol** (`strong_protocol.rs`, ~918 lines):
- `StrongPrefixConsensusProtocol`: Pure state machine (no I/O, no async)
- `View1Decision`: Always DirectCert (View 1 is special)
- `ViewDecision`: Three-way — Commit > DirectCert > EmptyView
- Certificate store (`HashMap<HashValue, Certificate>`) for trace-back
- `build_certificate_chain()`: Follows v_low → v_high hash links back to View 1
- `build_commit_message()`: Wraps chain into StrongPCCommit
- `highest_known_view`: Tracks best non-empty view for EmptyViewMessages
- `ChainBuildError::MissingCert`: Signals manager to fetch missing certs
- 27 unit tests covering all decision paths and trace-back scenarios

**Chunk 2 — Network Messages**:
- `ViewProposal`: Certificate proposal for a view (in `types.rs`)
- `CertFetchRequest`: Request certificate by hash (in `types.rs`)
- `CertFetchResponse`: Response with certificate (in `types.rs`)
- `StrongPrefixConsensusMsg`: 6-variant enum wrapping all Strong PC messages (in `network_messages.rs`)
  - `InnerPC { view, msg }`, `Proposal`, `EmptyView`, `Commit`, `FetchRequest`, `FetchResponse`
  - Helper methods: `epoch()`, `slot()`, `view()`, `name()`, `author()`
- 17 unit tests covering construction, helpers, and serialization roundtrips

**Chunk 3 — Strong Manager** (`strong_manager.rs`, ~1394 lines):
- `StrongPrefixConsensusManager<NetworkSender>`: Async event-driven orchestrator
- `PCState`: Per-view inner PC instance state (protocol + round tracking + completed flag)
- View start timer: 300ms timeout OR first-ranked certificate arrival triggers inner PC start
- Proposal adoption: when receiving a valid proposal for a future view, adopt the certificate and broadcast as own before entering
- Certificate fetching: request missing certs by hash, retry chain building on response
- DoS protection on fetch responses: check `pending_fetches` before doing validation work
- `propose_and_enter()`: Helper that broadcasts proposal, adds own cert to ViewState, enters new view. Guards against double proposals (Byzantine behavior) via `next_view <= current_view` check.
- `finalize_view()`: Extracted helper for view completion logic (handles commit trace-back, StrongPCCommit broadcast, and certificate fetching for missing chain links)
- `Box::pin` on `start_pc_now` to handle recursive async call chain: `start_pc_now → start_view_round2 → start_view_round3 → finalize_view → handle_view_complete → propose_and_enter → enter_view → start_pc_now`
- Empty view handling: sign EmptyViewMessage, collect >1/3 stake, form IndirectCertificate
- Epoch/slot filtering on all incoming messages in `process_message`

**Cross-cutting fixes during Chunk 3**:
- **Early QC bug fix** (protocol.rs + manager.rs): `start_round1/2/3` methods internally call `process_vote*` for the self-vote, which could form a QC if enough early votes accumulated. The QC was silently discarded. Fixed by changing return types to `(Vote, Option<QC>)` and handling early QC in both managers.
- **EmptyViewMessage epoch/slot** (certificates.rs + network_messages.rs): Added `epoch` and `slot` fields to `EmptyViewMessage` for uniform filtering in `process_message`. Removed special-case handling.
- **IndirectCertificate::from_messages** (certificates.rs): Changed from `Result<Self>` to `Option<Self>` since insufficient stake is a normal condition, not an error.
- **ViewProposal doesn't need signing**: Network authenticates sender, certificate is validated. Any party can propose any valid certificate — this is by design for proposal adoption (honest parties forward certificates to help others catch up).

**Key design decisions documented during Chunk 3**:
- **No double proposals**: A party must never broadcast two different certificates for the same view. This would be Byzantine behavior and could cause prefix cuts in inner PC (different parties see different input vectors).
- **Proposal adoption vs echoing**: Parties adopt received certificates as their own proposals (not echo/relay). The adopting party broadcasts under its own identity for its ranking position.
- **Timer-based view start**: All parties must participate in inner PC to ensure quorum formation. Even if no certificates arrive, parties run inner PC with empty input after 300ms timeout.
- **Fetch DoS protection**: Check `pending_fetches` before validating unsolicited `CertFetchResponse` messages. Keep entry in `pending_fetches` until successful validation (don't remove on failed validation — would lose the hash for retry).
- **Unreachable vs legitimate None guards**: 4 cases where `None` is unreachable use `expect()`, 3 cases in `process_view_vote1/2/3` where `None` means stale view use `return`.

### Phase 5 Implementation Details (Integration)

**Files modified**:
- `consensus/src/network_interface.rs` — Added `ConsensusMsg::StrongPrefixConsensusMsg` variant, moved both `ConsensusNetworkBridge` and `StrongConsensusNetworkBridge` here (were in epoch_manager)
- `consensus/src/epoch_manager.rs` — Added `strong_prefix_consensus_tx`/`close_tx` fields, `start_strong_prefix_consensus()`, `stop_strong_prefix_consensus()`, `check_epoch` routing, epoch cleanup in `shutdown_current_processor`
- `consensus/prefix-consensus/src/network_interface.rs` — Added `StrongPrefixConsensusNetworkClient`, `StrongNetworkSenderAdapter` implementing `StrongPrefixConsensusNetworkSender` trait
- `consensus/prefix-consensus/src/lib.rs` — Updated exports

**Network architecture** (4-layer send-side stack, mirrors basic PC):
```
ConsensusNetworkClient (sends ConsensusMsg over the wire)
    ↓ wrapped in
StrongConsensusNetworkBridge (wraps in ConsensusMsg::StrongPrefixConsensusMsg)
    ↓ wrapped in
StrongPrefixConsensusNetworkClient (PeerId → PeerNetworkId mapping)
    ↓ wrapped in
StrongNetworkSenderAdapter (implements StrongPrefixConsensusNetworkSender trait)
    ↓ passed to
StrongPrefixConsensusManager::new(...)
```

**Receive side**: `check_epoch` matches `ConsensusMsg::StrongPrefixConsensusMsg`, verifies epoch, forwards to `strong_prefix_consensus_tx` channel.

**Epoch cleanup**: Both `stop_prefix_consensus()` and `stop_strong_prefix_consensus()` called in `shutdown_current_processor()`.

**Bridge refactor**: Both `ConsensusNetworkBridge` and `StrongConsensusNetworkBridge` moved from `epoch_manager.rs` to `network_interface.rs` (where `ConsensusMsg` lives). Made generic over `NetworkClient` type parameter.

### Phase 8 Implementation Details (Smoke Tests)

**Completed**: February 12, 2026
**Detailed plan**: `.plans/strong-pc-smoke-test.md`

**Infrastructure changes**:
- `config/src/config/consensus_config.rs` — Added `strong_prefix_consensus_test_input` config field
- `consensus/prefix-consensus/src/strong_manager.rs` — Added `write_output_file()` method, called on commit
- `consensus/src/epoch_manager.rs` — Extracted `trigger_test_prefix_consensus()` method, called from both `start()` and `start_new_epoch()` to survive epoch transitions
- `consensus/src/network.rs` — Added `StrongPrefixConsensusMsg` to DirectSend dispatch whitelist

**Smoke tests** (all pass with 100% success rate):
1. `test_strong_prefix_consensus_identical_inputs` — 4 validators, 5 identical hashes → v_low = v_high = input
2. `test_strong_prefix_consensus_divergent_inputs` — 4 validators, diverge at position 2 → v_low = [hash1, hash2]
3. `test_prefix_consensus_identical_inputs` — (basic PC, fixed race condition + format mismatch)
4. `test_prefix_consensus_divergent_inputs` — (basic PC, fixed race condition + format mismatch)

**Bugs fixed during smoke test development** (4 total):
1. **File path format mismatch**: `AccountAddress` Display (`{}`) adds `0x` prefix, LowerHex (`{:x}`) does not. Fixed readers to use `{:x}`.
2. **Race condition**: `cleanup_output_files()` ran after swarm build, deleting files written during startup. Added `cleanup_all_output_files()` called before swarm construction.
3. **Epoch transition**: DKG completes ~1.5s after epoch 1, triggering epoch 1→2 that kills the protocol. Fixed by calling triggers on every epoch start.
4. **DirectSend dispatch**: `StrongPrefixConsensusMsg` missing from `network.rs` match arm, silently dropping all strong PC network messages.

### Phase 9 Implementation Details (Inner PC Abstraction)

**Completed**: February 12, 2026
**Detailed plan**: `.plans/inner-pc-trait.md`

**New files**:
- `inner_pc_trait.rs` (~90 lines) — `InnerPCAlgorithm` trait with `new_for_view()`, `start()`, `process_message()`
- `inner_pc_impl.rs` (~400 lines) — `ThreeRoundPC` struct implementing the trait, wrapping `PrefixConsensusProtocol` with cascade helpers and `RoundState` enum. 4 unit tests.

**Modified files**:
- `strong_manager.rs` — Made generic over `T: InnerPCAlgorithm`. `PCState<T>` holds `algorithm: T`. Deleted 6 methods (~130 lines): `process_view_vote1/2/3`, `start_view_round2/3`, match dispatch. Added `DefaultStrongPCManager<NS>` type alias.
- `signing.rs` — Added `create_signed_vote1/2/3` factory functions (construct vote from fields + signer in one call). Replaced dummy-signature-then-reconstruct pattern in `protocol.rs`.
- `protocol.rs` — Replaced 3 dummy-signature blocks (~15 lines each) with single `create_signed_vote*()` calls (~3 lines each). Net reduction ~21 lines.
- `epoch_manager.rs` — Changed to `DefaultStrongPCManager::new()`
- `lib.rs` — Added module declarations and exports for new types

**Key design decisions**:
- Signer passed by reference to `start()` and `process_message()` (not stored in struct) — avoids Clone bound on ValidatorSigner
- `process_message()` returns `Vec<Self::Message>` for cascading (QC1 → Vote2 → QC2 → Vote3)
- `Message` associated type constrained to `PrefixConsensusMsg` in manager's impl block (not in trait)
- Manager remains generic but `DefaultStrongPCManager` alias provides ergonomic default

### Next Action
Strong Prefix Consensus Phase 10 (Performance) or Phase 11 (Documentation), or begin Slot Manager

---

## TODO (Future Work)

- [ ] **Fetching protocol**: Implement certificate fetch by hash for Byzantine withholding scenarios
- [ ] **StrongPCCommit chain cap**: If chains become long (>5 certs), cap embedded certs and use hashes for the rest
- [ ] **Fetch DOS protection**: Rate limiting / authentication for fetch requests
- [ ] **Certificate storage**: Efficient storage and lookup by hash for accumulated certificates
- [ ] **Configurable view start timeout**: `VIEW_START_TIMEOUT` (300ms) is currently a hardcoded constant in `strong_manager.rs`. Make it configurable via consensus config or constructor parameter.
- [ ] **Empty view without inner PC**: Consider sending EmptyViewMessage directly (without running the inner PC) when no certificates have arrived at timeout. This saves 3 rounds of inner PC for the all-bot case. Caveat: all parties must participate in the inner PC to ensure others can form QCs, so this optimization only applies when ALL parties have no certificates for the view.
- [ ] **View 1 start timer for multi-slot protocol**: When Strong Prefix Consensus is used within the multi-slot final protocol (Slot Manager), add a timer before starting View 1 to allow parties to collect input vectors from other slots before beginning. Currently View 1 starts immediately with the local input vector.
- [ ] **Garbage collect data structures on slot commit**: When a slot is committed, clean up accumulated state (e.g., `view_states`, `pc_states`, `empty_view_collectors`, `pending_fetches`, `pending_commit_proof`, cert store) to free memory and avoid unbounded growth across slots.
- [ ] **Collapse network bridges into one generic bridge**: After the inner PC trait (Phase 9) unifies basic and strong PC, replace the two separate `ConsensusNetworkBridge` and `StrongConsensusNetworkBridge` (in `network_interface.rs`) with a single generic bridge parameterized by message type and `ConsensusMsg` wrapping variant. Eliminates ~120 lines of duplicated boilerplate.

---

## Repository Structure

### Prefix Consensus Crate
```
consensus/prefix-consensus/src/
├── types.rs              - Vote/QC types + ViewProposal/CertFetch types (1039 lines)
├── protocol.rs           - 3-round state machine (~585 lines, reduced by signing refactor)
├── manager.rs            - Event-driven orchestrator (737 lines)
├── network_interface.rs  - Network adapter + Strong PC client/adapter (~330 lines)
├── network_messages.rs   - PrefixConsensusMsg + StrongPrefixConsensusMsg (795 lines)
├── signing.rs            - BLS helpers + create_signed_vote* factory functions (~240 lines)
├── certify.rs            - QC formation (220 lines)
├── verification.rs       - Validation (150 lines)
├── utils.rs              - Prefix operations + first_non_bot (210 lines)
├── certificates.rs       - Certificates + StrongPCCommit + cert_reaches_view1 (1348 lines)
├── view_state.rs         - RankingManager, ViewState, ViewOutput (695 lines)
├── strong_protocol.rs    - Strong PC state machine (918 lines)
├── strong_manager.rs     - Strong PC event orchestrator, generic over InnerPCAlgorithm (~1270 lines)
├── inner_pc_trait.rs     - InnerPCAlgorithm trait (~90 lines) - Phase 9
└── inner_pc_impl.rs      - ThreeRoundPC implementation + 4 tests (~400 lines) - Phase 9

testsuite/smoke-test/src/consensus/prefix_consensus/
├── helpers.rs            - Test helpers (cleanup_all, wait_for_outputs, generate_hashes)
└── basic_test.rs         - 2 smoke tests (identical + divergent inputs)

testsuite/smoke-test/src/consensus/strong_prefix_consensus/
├── mod.rs                - Module declarations
├── helpers.rs            - Test helpers (cleanup_all, wait_for_outputs, generate_hashes)
└── basic_test.rs         - 2 smoke tests (identical + divergent inputs)
```

### Plans
- `.plans/network-integration.md` - Basic Prefix Consensus plan (complete)
- `.plans/strong-prefix-consensus.md` - Strong Prefix Consensus plan (current)
- `.plans/inner-pc-trait.md` - Inner PC Abstraction Trait plan (complete)

---

## Git Commit History (prefix-consensus-prototype branch)

1. **96a68780cd** (Jan 22): Initial Prefix Consensus primitive implementation
2. **8b7ad1b60e** (Jan 28): PrefixConsensusManager with event-driven architecture
3. **e4376f5e9a** (Jan 28): Fix Vote1 self-send race condition
4. **6c5eec0fe8** (Jan 28): Update docs (Phase 4 complete)
5. **349a557e6d** (Jan 29): EpochManager integration
6. **45dec59b91** (Jan 29): Update docs (Phase 5 complete)
7. **9c96198f3f** (Feb 3): Smoke test infrastructure and bug fixes
8. **6f12e09ceb** (Feb 3): Fix race condition, add output writing
9. **f5e736d906** (Feb 4): Divergent inputs test
10. **537848ce43** (Feb 4): Update docs (Phase 6 complete, Strong PC plan)
11. **(pending)** (Feb 5): Security enhancements for Verifiable Prefix Consensus
12. **2c52a342f7** (Feb 12): Strong PC integration with EpochManager (Phase 5)
13. **7df6a297e1** (Feb 12): Strong PC smoke test and integration bug fixes (Phase 8)
14. **(pending)**: Strong PC divergent inputs smoke test
15. **039271f95e** (Feb 12): Inner PC Abstraction Trait + signing factory functions (Phase 9) ← HEAD

---

## References

**Paper**: "Prefix Consensus For Censorship Resistant BFT" (Feb 2024)
- Location: `/Users/alexanderspiegelman/Downloads/Prefix_Consensus (5).pdf`
- Algorithm 1: Basic Prefix Consensus (✅ implemented)
- Algorithm 3: Strong Prefix Consensus (🚧 in progress)
- Algorithm 2: Multi-slot BFT (⏳ future work)

**Aptos Integration Points**:
- `consensus/src/epoch_manager.rs` - Protocol lifecycle management
- `consensus/src/network_interface.rs` - Message routing
- `consensus/consensus-types/src/` - Common types (ValidatorVerifier, Author, etc.)

---

## Future Work

### Immediate (After Strong PC)
1. **Slot Manager**: Run Strong PC per slot for multi-slot consensus (Algorithm 2)
2. **Censorship Resistance**: Ranking updates based on exclusions
3. **Smoke Tests**: Byzantine behavior, fault tolerance

### Long Term
4. **Optimistic Variants**: 2-round good case (Appendix D)
5. **Communication Optimization**: Reduce from O(n²L) to O(n² + nL)
6. **Production Hardening**: Metrics, error recovery, persistence
7. **Execution Integration**: Connect to Aptos execution and storage
