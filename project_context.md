# Project Context: Prefix Consensus Implementation

## Overview

Implementing Prefix Consensus protocols (from research paper "Prefix Consensus For Censorship Resistant BFT") within Aptos Core for leaderless, censorship-resistant consensus.

**Current Phase**: Strong Prefix Consensus - Multi-view protocol for v_high agreement
**Completed**: Basic Prefix Consensus primitive with Verifiable Prefix Consensus (Phase 1)

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
  - Round 1: Broadcast inputs, certify longest prefix with f+1 agreement
  - Round 2: Broadcast certified prefixes, compute mcp
  - Round 3: Output v_low (mcp), v_high (mce)
- **Benefits**: Deterministically async, leaderless, censorship-resistant (2f slots after GST)

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
- Unit tests: All 103 tests passing
- Smoke tests: Do NOT pass automatically (timeout issues)
  - Manual verification required: Check that 4 output files are created
  - Output files: `/tmp/prefix_consensus_output_{party_id}.json`
  - Property verification: Upper Bound, Validity, Consistency checked manually
- Test script: `test_prefix_consensus.sh`

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

**IndirectCertificate**: Aggregates f+1 EmptyViewMessages
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
- IndirectCertificate::from_messages() requires f+1 messages (checked at construction)

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
**Status**: Planning complete, ready for implementation
**Plan**: `.plans/strong-prefix-consensus.md` (10 phases, ~10-11 days estimated)

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
**Indirect Certificates**: Cert^ind(w-1, (w*, v_high*, π), Σ) - skip empty views with f+1 sigs
**Parent Chain**: Following certificates back to View 1 uniquely determines v_high output

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

## Current Status (February 5, 2026)

### Repository State
- **Branch**: `prefix-consensus-prototype`
- **HEAD**: Certificate types with view field for replay protection
- **Status**: Clean working directory
- **Tests**: 103/103 unit tests passing
- **Smoke Tests**: Compile but do NOT pass automatically - verify 4 output files manually
- **Build**: ✅ No warnings or errors

### Progress Summary
- ✅ **Basic Prefix Consensus**: Complete (Phase 1-6 of network-integration.md)
- ✅ **Strong PC Phase 1**: Verifiable Prefix Consensus complete (security fixes + proof verification)
- ✅ **Strong PC Phase 2**: Certificate types complete (Direct/Indirect certificates + view field)
- 🚧 **Strong PC Phase 3+**: View state management and multi-view protocol pending
- ⏳ **Slot Manager**: Future work (after Strong PC complete)

### Next Action
Begin Strong Prefix Consensus Phase 3: Implement View State and Ranking Management

---

## Repository Structure

### Basic Prefix Consensus (Complete)
```
consensus/prefix-consensus/src/
├── types.rs              - Vote/QC types with BLS (350 lines)
├── protocol.rs           - 3-round state machine (490 lines)
├── manager.rs            - Event-driven orchestrator (658 lines)
├── network_interface.rs  - Network adapter (224 lines)
├── network_messages.rs   - Message enum (443 lines)
├── signing.rs            - BLS helpers (184 lines)
├── certify.rs            - QC formation (220 lines)
├── verification.rs       - Validation (150 lines)
├── utils.rs              - Prefix operations (180 lines)
└── certificates.rs       - Strong PC certificates (500 lines) ← NEW Phase 2

testsuite/smoke-test/src/consensus/prefix_consensus/
├── helpers.rs            - Test helpers
└── basic_test.rs         - 2 smoke tests
```

### Strong Prefix Consensus (Planned)
```
consensus/prefix-consensus/src/
├── certificates.rs       - Cert types (300 lines) - Phase 2
├── view_state.rs         - View management (400 lines) - Phase 3
├── strong_protocol.rs    - Multi-view protocol (800 lines) - Phase 4
└── strong_manager.rs     - Event orchestrator (700 lines) - Phase 6

testsuite/smoke-test/src/consensus/strong_prefix_consensus/
├── helpers.rs            - Test helpers (200 lines) - Phase 8
└── basic_test.rs         - 4 smoke tests (400 lines) - Phase 8
```

### Plans
- `.plans/network-integration.md` - Basic Prefix Consensus plan (complete)
- `.plans/strong-prefix-consensus.md` - Strong Prefix Consensus plan (current)

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
11. **(pending)** (Feb 5): Security enhancements for Verifiable Prefix Consensus ← HEAD

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
