# Project Context: Prefix Consensus Prototype Implementation

## Overview

Implementing a prototype of Prefix Consensus (from research paper "Prefix Consensus For Censorship Resistant BFT") within Aptos Core. Starting with the primitive 3-round asynchronous protocol.

**Goal**: Leaderless, censorship-resistant consensus that works asynchronously.

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

## Implementation Progress

### Phase 1: Serialization and Signature Support (✅ COMPLETE)

**Date**: 2026-01-28

**Implementation**:
- Added BCS serialization to all types (Vote1/2/3, QC1/2/3, Input/Output)
- Changed signatures from Ed25519 to BLS12-381 (matches Aptos consensus)
- Added epoch/slot fields to votes (future multi-slot support)
- Created custom hashers (Vote1/2/3Hasher) for CryptoHash that exclude signatures
- Created SignData helper structs (Vote1/2/3SignData) for proper BCS signing
- Implemented sign_vote1/2/3() and verify_vote1/2/3_signature() functions

**Files**:
- Modified: `consensus/prefix-consensus/Cargo.toml`, `src/types.rs`, `src/protocol.rs`
- Created: `consensus/prefix-consensus/src/signing.rs` (184 lines)

**Tests**: ✅ 54/54 passing (including signature round-trip tests)

**Technical Challenge Solved**: BCS serialization vs CryptoHash - ValidatorVerifier uses BCS of entire struct, not just hash. Solution: separate SignData structs without signature field.

---

### Phase 2: Network Message Types (✅ COMPLETE)

**Date**: 2026-01-28

**Implementation**:
- Created `PrefixConsensusMsg` enum with Vote1/2/3Msg variants (boxed)
- Helper methods: name(), epoch(), slot(), author(), as_vote*(), into_vote*()
- Full BCS serialization support

**Files**:
- Created: `consensus/prefix-consensus/src/network_messages.rs` (443 lines)

**Tests**: ✅ 70/70 passing (16 new tests for serialization, accessors)

---

### Phase 3: Network Interface Adapter (✅ COMPLETE)

**Date**: 2026-01-28

**Implementation**:
- Created `PrefixConsensusNetworkSender` trait (broadcast_vote1/2/3)
- Implemented `NetworkSenderAdapter` wrapping Aptos NetworkClient
- Self-send via UnboundedSender channel (network doesn't support self-send)
- Broadcast to other validators via send_to_many()
- Generic helper method for DRY code

**Files**:
- Created: `consensus/prefix-consensus/src/network_interface.rs` (224 lines)
- Modified: `Cargo.toml` (added 7 network dependencies)

**Tests**: ✅ 71/71 passing

---

### Phase 4: PrefixConsensusManager (✅ COMPLETE)

**Date**: 2026-01-28

**Implementation**:
- Event-driven manager with RoundManager pattern
- Key methods: new(), init(), run() (consumes self), process_message(), process_vote1/2/3()
- Duplicate detection per round (HashSet in RwLock)
- Signature verification (currently dummy Ed25519, Phase 10 will add real BLS)
- Auto-exits when QC3 forms or shutdown signaled
- Structured logging with party_id, epoch, round, author fields

**Architecture**:
```
run() starts → Broadcast Vote1 → tokio::select! loop
  ↓                                    ↓
Message arrives → process_vote* → Protocol state machine
  ↓                                    ↓
QC formed → start_round* → Broadcast next vote
  ↓
QC3 complete → Exit loop
```

**Files**:
- Created: `consensus/prefix-consensus/src/manager.rs` (658 lines)

**Tests**: ✅ 74/74 passing (3 new manager tests)

**Race Condition Fixed**: Receiver now starts before Vote1 broadcast (commit e4376f5e9a)

---

### Phase 5: EpochManager Integration (✅ COMPLETE)

**Date**: 2026-01-29

**Implementation**:
- Added `PrefixConsensusMsg` variant to `ConsensusMsg` enum
- Added message routing in `EpochManager::check_epoch()` (epoch validation, forward to manager)
- Implemented `start_prefix_consensus(input)` method:
  - Creates channels, NetworkSenderAdapter, ValidatorSigner
  - Spawns PrefixConsensusManager on tokio runtime
  - Returns after initialization
- Implemented `stop_prefix_consensus()` method (graceful shutdown)
- Stores channels: prefix_consensus_tx, prefix_consensus_close_tx

**Architectural Decision**: Bypasses UnverifiedEvent pattern
- Rationale: Prefix consensus will eventually REPLACE RoundManager
- Avoids coupling to types that will be removed
- Clean replacement path for future SlotManager
- Routes directly in check_epoch() like EpochRetrievalRequest

**Files**:
- Modified: `consensus/src/epoch_manager.rs` (~140 lines added)
- Modified: `consensus/src/network_interface.rs` (added enum variant)
- Modified: `Cargo.toml`, `consensus/Cargo.toml` (workspace dependency)

**Tests**: ✅ 74/74 passing

**Commit**: `349a557e6d` - Integration complete

---

### Phase 6: Smoke Test & Bug Fixes (🔄 IN PROGRESS)

**Date**: 2026-02-03/04

**Goal**: Create basic smoke test infrastructure and fix protocol bugs

**Implementation**:

1. **Smoke Test Infrastructure** (✅ Complete)
   - Created `testsuite/smoke-test/src/consensus/prefix_consensus/` module
   - Test helper functions: `generate_test_hashes()`, `wait_for_prefix_consensus_outputs()`, `cleanup_output_files()`
   - Basic test: `test_prefix_consensus_identical_inputs` (4 validators, identical inputs)
   - Test triggers protocol via config: `consensus.prefix_consensus_test_input`
   - Validators write output to `/tmp/prefix_consensus_output_{party_id}.json`

2. **Fixed Signature Verification** (✅ Complete)
   - **Problem**: Protocol used dummy BLS signatures that failed verification
   - **Root Cause**: `protocol.rs` called `create_dummy_signature()` instead of real BLS signing
   - **Fix**: Changed `start_round1/2/3()` to accept `&ValidatorSigner`, use real BLS signatures
   - **Changes**:
     - `protocol.rs`: Accept `ValidatorSigner`, call `sign_vote1/2/3()` with real keys
     - `manager.rs`: Removed `dummy_private_key`, pass `&self.validator_signer` to protocol

3. **Fixed Network Message Routing** (✅ Complete)
   - **Problem**: `PrefixConsensusMsg` rejected with "Unexpected direct send msg"
   - **Root Cause**: DirectSend handler in `network.rs` didn't include `PrefixConsensusMsg`
   - **Fix**: Added `ConsensusMsg::PrefixConsensusMsg(_)` to pattern match at line 871

4. **Added Output File Writing** (✅ Complete)
   - **Problem**: Protocol completed but no output files for test validation
   - **Fix**: Added `write_output_file()` method in `manager.rs` that writes JSON to `/tmp/`
   - Writes after QC3 formation with v_low, v_high, epoch, party_id

**Files Modified**:
- `consensus/prefix-consensus/src/protocol.rs` - Real BLS signatures
- `consensus/prefix-consensus/src/manager.rs` - ValidatorSigner, output writing
- `consensus/src/network.rs` - PrefixConsensusMsg routing
- `testsuite/smoke-test/Cargo.toml` - Added prefix-consensus dependency
- `testsuite/smoke-test/src/consensus/mod.rs` - Added prefix_consensus module
- `testsuite/smoke-test/src/consensus/prefix_consensus/mod.rs` - Module structure
- `testsuite/smoke-test/src/consensus/prefix_consensus/helpers.rs` - Test helpers
- `testsuite/smoke-test/src/consensus/prefix_consensus/basic_test.rs` - Smoke test

**Current Test Results**:
- ✅ Protocol runs successfully through all 3 rounds
- ✅ QC1, QC2, QC3 formation works
- ✅ Output files written correctly (v_low and v_high match expectations)
- ⚠️  **Intermittent Issue**: 3/4 validators complete (75% success rate)

**Known Issue - Race Condition**:
- **Symptom**: 1 validator gets stuck in Round 2, never forms QC2
- **Root Cause**: Early message rejection - Vote2 arrives before validator enters Round 2
- **Example Timeline**:
  ```
  T=50ms: Fast validators form QC1, broadcast Vote2
  T=52ms: Slow validator receives Vote2 while still in Round1 → REJECTED "Not in Round 2"
  T=55ms: Slow validator forms QC1, enters Round2
  T=56ms: Slow validator only has 1 Vote2 (its own) → stuck at vote_count=1/3
  ```
- **Location**: `protocol.rs` lines 163, 289, 413 - strict state checks reject early votes
- **Impact**: Test fails intermittently (need message buffering to fix)

**Next Steps**:
- Implement message buffering for early votes (allow Vote2 in Round1, process when ready)
- OR: Remove strict state checks (process votes in any state)
- Add retransmission mechanism (complex alternative)

---

## Current Status (February 4, 2026)

### ✅ Completed Phases (5/11)
1. **Phase 1**: Serialization & BLS signatures
2. **Phase 2**: Network message types
3. **Phase 3**: Network adapter
4. **Phase 4**: PrefixConsensusManager
5. **Phase 5**: EpochManager integration

### 🔄 In Progress
**Phase 6**: Smoke Test & Bug Fixes
- ✅ Test infrastructure created
- ✅ Signature verification fixed (real BLS)
- ✅ Network routing fixed
- ✅ Output file writing implemented
- ⚠️  Race condition identified (early message rejection)

### ⏳ Remaining Phases (7-11)
7. **Message Buffering** (fix race condition - buffer early votes)
8. **Complete Smoke Tests** (ensure 100% pass rate)
9. **Fault Tolerance Tests** (silent validator, Byzantine behavior)
10. **Additional Test Cases** (overlapping/divergent inputs)
11. **Documentation** (README, API docs, examples)

### Repository State
- **Branch**: `prefix-consensus-prototype`
- **HEAD**: `45dec59b91` (about to commit Phase 6 progress)
- **Status**: Modified files (smoke test + bug fixes, ready to commit)
- **Tests**: 74/74 unit tests passing, smoke test 75% success rate
- **Build**: ✅ All build issues resolved

### Progress
- **Overall**: Phase 6/11 (~55%)
- **Time Spent**: ~15 hours total (Phase 6: 3h debugging + fixes)

### Next Action
**Decision Point**: Choose approach to fix race condition:
1. Add message buffering (store early votes, process when ready)
2. Remove strict state checks (process votes anytime)
3. Add retransmission mechanism

---

## Key Implementation Files

```
consensus/prefix-consensus/
├── src/
│   ├── types.rs              - Vote/QC types with BLS signatures (350+ lines)
│   ├── utils.rs              - mcp/mce prefix operations (180 lines)
│   ├── certify.rs            - QC1/2/3Certify functions with trie (220 lines)
│   ├── verification.rs       - Vote/QC verification (150 lines)
│   ├── protocol.rs           - 3-round state machine (380 lines)
│   ├── signing.rs            - BLS sign/verify helpers (184 lines)
│   ├── network_messages.rs   - PrefixConsensusMsg enum (443 lines)
│   ├── network_interface.rs  - Network adapter (224 lines)
│   └── manager.rs            - Event-driven manager (658 lines)
└── Cargo.toml

consensus/src/
├── epoch_manager.rs          - Integration point (start_prefix_consensus)
└── network_interface.rs      - ConsensusMsg enum (PrefixConsensusMsg variant)
```

**Total LOC**: ~2800 lines (implementation + tests)

---

## Implementation Plan Reference

**Full Plan**: `.plans/network-integration.md` (11 phases, 8-11 days estimated)

**Progress**: Phase 5/11 (~45%)

**Time Spent**: ~12 hours (Phase 1: 4h, Phase 2: 1h, Phase 3: 2h, Phase 4: 3h, Phase 5: 2h)

---

## Git Commit History

### Commits on `prefix-consensus-prototype` branch

1. **96a68780cd** (2026-01-22): `[consensus] Add Prefix Consensus primitive implementation`
   - Initial 7-file module creation (types, utils, certify, verification, protocol)

2. **8b7ad1b60e** (2026-01-28): `[consensus] Implement PrefixConsensusManager (Phase 4)`
   - Created manager.rs with event-driven architecture

3. **e4376f5e9a** (2026-01-28): `[consensus] Refactor manager to match RoundManager pattern and fix race condition`
   - Fixed Vote1 self-send race by starting receiver before broadcast

4. **6c5eec0fe8** (2026-01-28): `[docs] Update project context with Phase 4 completion and Phase 5 plan`

5. **349a557e6d** (2026-01-29): `[consensus] Integrate PrefixConsensusManager into EpochManager (Phase 5)`
   - Added start_prefix_consensus(), message routing

6. **45dec59b91** (2026-01-29): `[docs] Update project context with Phase 5 completion and architectural discussion` (HEAD)

---

## References

**Paper**: "Prefix Consensus For Censorship Resistant BFT" - `/Users/alexanderspiegelman/Downloads/Prefix_Consensus (4).pdf`

**Key Algorithms**:
- Algorithm 1: 3-round async Prefix Consensus (implemented)
- Algorithm 2: Multi-slot censorship-resistant BFT (future)
- Algorithm 4: Optimistic 2-round variant (future)

**Aptos Codebase**:
- Consensus: `consensus/src/`
- Types: `consensus/consensus-types/src/`
- Safety Rules: `consensus/safety-rules/src/`
- Network: `consensus/src/network_interface.rs`

---

## Notes

### Build Environment
✅ macOS build issues resolved (RocksDB C++, sandbox .pem access)

### Testing Strategy
- Unit tests: Per-module tests in each file (74 tests total)
- Smoke tests: LocalSwarm with multiple validators (Phase 7-9)
- Property tests: Upper Bound, Validity, Termination verification

### Future Work
- Multi-slot consensus (Algorithm 2)
- Censorship resistance with reputation
- Communication optimization (reduce from O(n²L) to O(n² + nL))
- Optimistic 2-round variant (Algorithm 4)
- Integration with Aptos execution/storage
- Production hardening (metrics, error recovery, persistence)
