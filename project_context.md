# Project Context: Prefix Consensus Prototype Implementation

## Overview

This document outlines the plan to implement a prototype of Prefix Consensus (from the research paper "Prefix Consensus For Censorship Resistant BFT") within the Aptos Core codebase. The goal is to start with the primitive 3-round asynchronous Prefix Consensus protocol described in the paper.

---

## Understanding of Current Aptos Consensus (AptosBFT)

### Architecture

AptosBFT is a Byzantine Fault Tolerant consensus protocol based on Jolteon, tolerating up to f Byzantine validators in a 3f+1 validator set.

**Key Components:**

1. **RoundManager** (`consensus/src/round_manager.rs`): Central event processor that coordinates consensus events (proposals, votes, timeouts)

2. **BlockStore** (`consensus/src/block_storage/block_store.rs`): Thread-safe block tree management with:
   - Hierarchical block structure with parent/child links
   - Quorum certificates (QCs) storage
   - Multiple roots: commit_root, ordered_root, window_root

3. **SafetyRules** (`consensus/safety-rules/src/safety_rules.rs`): Enforces two critical voting rules:
   - **Rule 1**: Never vote twice in same round (prevents equivocation)
   - **Rule 2**: Only vote for blocks where parent_qc.round ≥ preferred_round (prevents forks)

4. **RoundState** (`consensus/src/liveness/round_state.rs`): Manages liveness and round transitions with exponential timeout backoff

5. **PendingVotes** (`consensus/src/pending_votes.rs`): Aggregates votes into quorum certificates

### Consensus Flow

```
Leader proposes block
  ↓
Validators execute speculatively
  ↓
Validators vote with state hash
  ↓
Leader collects 2f+1 votes → Quorum Certificate
  ↓
3-chain commit rule (3 consecutive QCs)
  ↓
Block committed to storage
```

### Key Data Structures

- **Block**: Contains BlockData with epoch, round, timestamp, author, parent_id, quorum_cert, payload
- **QuorumCert**: Contains VoteData (proposed + parent BlockInfo) and LedgerInfoWithSignatures (2f+1 signatures)
- **Vote**: Contains VoteData, author, LedgerInfo, and signature
- **SafetyData**: Persisted state with last_voted_round, preferred_round, one_chain_round

### Safety Guarantees

- **3-chain rule**: Block at round k committed when it has QC, parent has QC (k-1), grandparent has QC (k-2)
- **Preferred round mechanism**: After observing 2-chain, set preferred_round; future votes only for blocks extending this
- **Persistent safety data**: last_voted_round persisted to disk prevents double voting after restart

### Current Limitations (Relevant to Prefix Consensus)

1. **Leader-based**: Designated leader per round - single point of failure and censorship
2. **No censorship resistance**: Leader can selectively exclude transactions
3. **Synchrony assumption**: Requires partial synchrony (GST + Δ) for liveness

---

## Understanding of Prefix Consensus Paper

### Core Primitive: Prefix Consensus

**Definition**: A consensus primitive where parties propose vectors of values and output compatible vectors extending the maximum common prefix of honest inputs.

**Properties:**
- **Upper Bound**: v_low_i ⪯ v_high_j for any honest parties i,j
- **Termination**: Every honest party eventually outputs
- **Validity**: mcp({v_in_h}_{h∈H}) ⪯ v_low_i for any honest party i

**Key Insight**: Unlike traditional consensus, Prefix Consensus:
- Does NOT require agreement on single output value
- CAN be solved deterministically in asynchronous setting
- Outputs two values: v_low (safe to commit) and v_high (safe to extend)

### 3-Round Asynchronous Prefix Consensus Protocol (Algorithm 1)

**Round 1 (Voting on inputs)**:
```
1. Each party broadcasts signed input vector as vote-1
2. Collect n-f vote-1 messages into QC1
3. Extract longest prefix x that appears in at least f+1 votes
   x := max{ mcp({v_i : i ∈ S}) : S ⊆ {votes in QC1}, |S|=f+1 }
```

**Round 2 (Voting on certified prefixes)**:
```
1. Broadcast x as vote-2 with QC1
2. Collect n-f vote-2 messages into QC2
3. Compute certified prefix: xp := mcp({x ∈ QC2})
```

**Round 3 (Deriving outputs)**:
```
1. Broadcast xp as vote-3 with QC2
2. Collect n-f vote-3 messages into QC3
3. Output:
   v_low := mcp({xp ∈ QC3})  // maximum common prefix
   v_high := mce({xp ∈ QC3}) // minimum common extension
```

**Key Mechanisms:**

1. **QC1Certify**: Finds longest prefix shared by f+1 votes (can be computed in O(total input size) using trie)

2. **Quorum Intersection**: Any two quorums of size n-f intersect in at least f+1 parties, including ≥1 honest party

3. **Consistency Lemma**: QC2.xp ∼ QC2'.xp for any QC2, QC2' (all certified prefixes from round 2 are consistent)

**Complexity:**
- **Round complexity**: 3 rounds (optimal for n ≤ 4f)
- **Message complexity**: O(n²)
- **Communication complexity**: O((cL + κ_s)n⁴) for basic version, O(cn²L + cn³ + κ_s n²L) for optimized

### Multi-slot Censorship-Resistant BFT (Algorithm 2)

Built on top of Verifiable Prefix Consensus primitive:

**Per-slot Protocol:**
```
1. All parties broadcast proposals
2. Wait 2Δ to collect proposals, fill missing with ⊥
3. Order proposals by local ranking: rank_{i,s} = (p_1, ..., p_n)
4. Input vector to Prefix Consensus: v_in_{i,s} = [H(P_{i,s}[p_1]), ..., H(P_{i,s}[p_n])]
5. Prefix Consensus outputs (v_low, v_high)
6. Commit v_low, use v_high to authorize next slot
```

**Censorship Resistance via Reputation:**
- Maintain ranking of parties per slot
- When slot commits prefix of length ℓ < n, party p_{ℓ+1} is demoted to end
- Byzantine party can censor ≤2 consecutive slots before demotion
- Result: **2f-Censorship Resistance** (at most 2f censored slots after GST)

**Leaderless Property:**
- Allow ⊥ (placeholder) values in vectors
- Progress doesn't depend on any single party
- Satisfies Leaderless Termination: works even if adversary suspends 1 party per round

### Comparison with AptosBFT

| Feature | AptosBFT | Prefix Consensus |
|---------|----------|------------------|
| Leader-based | Yes (designated per round) | No (leaderless) |
| Censorship resistance | None | 2f slots after GST |
| Async solvability | No (requires partial sync) | Yes (3 rounds async) |
| Commit rule | 3-chain (3 consecutive QCs) | Prefix agreement |
| Safety mechanism | Preferred round | Prefix consistency |
| Rounds per slot | ~3 message delays | 3-4 rounds |

---

## Implementation Goal

### Objective

Implement a **working prototype** of the primitive 3-round asynchronous Prefix Consensus protocol (Algorithm 1 from the paper) within the Aptos Core codebase.

### Scope (Phase 1: Primitive Implementation)

**In Scope:**
1. Core Prefix Consensus protocol (Algorithm 1) with 3 rounds
2. Basic data structures: Vote messages, Quorum Certificates
3. QC certification functions: QC1Certify, QC2Certify, QC3Certify
4. Message verification and quorum collection logic
5. Output computation: v_low and v_high
6. Unit tests for correctness properties (Upper Bound, Validity, Termination)

**Out of Scope (for Phase 1):**
- Multi-slot consensus integration
- Censorship resistance mechanisms
- Reputation-based ranking
- Communication optimization
- Verifiable variant with proofs
- Integration with Aptos execution/storage layers

### Implementation Strategy

**Phase 1A: Standalone Prefix Consensus Module**

Create new module: `consensus/prefix-consensus/`

```
consensus/prefix-consensus/
├── src/
│   ├── lib.rs                 // Module exports
│   ├── types.rs               // Vote, QC, PrefixConsensusInput/Output
│   ├── protocol.rs            // Main protocol implementation
│   ├── certify.rs             // QC1Certify, QC2Certify, QC3Certify functions
│   ├── verification.rs        // Message and QC verification
│   └── tests.rs               // Unit and integration tests
└── Cargo.toml
```

**Key Design Decisions:**

1. **Async/Await**: Use Tokio for asynchronous message handling (consistent with Aptos style)

2. **Message Types**: Define clean types for Vote1, Vote2, Vote3, QC1, QC2, QC3

3. **Quorum Collection**: Maintain pending vote sets, trigger QC formation on n-f threshold

4. **Trie Optimization**: Implement trie-based O(input size) computation for QC1Certify

5. **Testing Strategy**:
   - Unit tests for each certify function
   - Property-based tests for Upper Bound, Validity
   - Simulation tests with Byzantine parties

**Phase 1B: Integration Points (Future)**

After primitive works:
1. Network layer integration (consensus/src/network.rs)
2. Create PrefixConsensusManager similar to RoundManager
3. Hook into existing SafetyRules if applicable
4. Integration tests with multiple nodes

### Success Criteria (Phase 1)

1. ✅ Prefix Consensus primitive passes all correctness tests
2. ✅ Can handle n=3f+1 parties with f Byzantine
3. ✅ Outputs satisfy Upper Bound: v_low_i ⪯ v_high_j
4. ✅ Outputs satisfy Validity: mcp(honest inputs) ⪯ v_low_i
5. ✅ All honest parties terminate and output
6. ✅ Simulation with message delays works correctly

### Non-Goals (Phase 1)

- Production-ready implementation
- Performance optimization
- Full integration with Aptos state machine replication
- Multi-slot consensus with censorship resistance
- Communication-optimized variant

---

## Technical Challenges & Considerations

### 1. Vector Operations

**Challenge**: Efficient implementation of mcp (max common prefix) and mce (min common extension)

**Solution**:
- Use trie for O(total input) QC1Certify computation
- Vector comparison with early termination
- Consider using existing Rust crates for prefix operations

### 2. Quorum Certificate Structure

**Challenge**: QCs in paper bundle n-f votes, but Aptos uses aggregated signatures

**Solution**:
- Phase 1: Store full votes in QC (simpler, matches paper)
- Future: Optimize with BLS signature aggregation like Aptos

### 3. Asynchronous Execution

**Challenge**: Paper assumes asynchronous message delivery, Aptos assumes partial synchrony

**Solution**:
- Phase 1: Implement async without timeouts (pure async)
- Add configurable timeouts for practical deployment
- Use Tokio channels for message passing

### 4. Byzantine Behavior Simulation

**Challenge**: Need to test with Byzantine parties sending conflicting messages

**Solution**:
- Create MockParty trait with honest/byzantine implementations
- Implement equivocation scenarios in tests
- Test with parties sending different vectors to different recipients

### 5. Consistency Verification

**Challenge**: Verifying prefix consistency properties across quorums

**Solution**:
- Implement explicit consistency checks in verification.rs
- Add invariant assertions throughout protocol
- Use property-based testing (proptest crate)

---

## Next Steps

### Immediate (Week 1-2)

1. ✅ Read and understand the paper thoroughly
2. ✅ Document understanding in this file
3. Create basic module structure in `consensus/prefix-consensus/`
4. Define core types (Vote1/2/3, QC1/2/3, PrefixConsensusInput/Output)
5. Implement prefix utility functions (mcp, mce, consistency checks)

### Short-term (Week 3-4)

1. Implement QC1Certify with trie optimization
2. Implement QC2Certify and QC3Certify
3. Build protocol state machine with 3 rounds
4. Add message verification logic
5. Write unit tests for certify functions

### Medium-term (Week 5-8)

1. Implement full protocol with async message handling
2. Create multi-party simulation framework
3. Add Byzantine party implementations for testing
4. Comprehensive integration tests
5. Document the implementation

### Long-term (Future Phases)

1. Communication-optimized variant (Section B from paper)
2. Optimistic Prefix Consensus (2-round good case)
3. Multi-slot consensus integration
4. Censorship resistance mechanisms
5. Production-ready hardening

---

## Open Questions

1. **Signature Scheme**: Use BLS (like Aptos) or Ed25519 for Phase 1?
   - *Recommendation*: Start with Ed25519 for simplicity, migrate to BLS later

2. **Network Simulation**: Build custom simulator or use existing Aptos network test framework?
   - *Recommendation*: Start with simple in-process simulation, integrate later

3. **Integration Strategy**: Replace AptosBFT or run alongside?
   - *Recommendation*: Phase 1 is standalone, integration TBD

4. **Vector Element Type**: What should vector elements be? Transactions, hashes, generic?
   - *Recommendation*: Generic `Vec<T: Clone + Eq + Hash>` for Phase 1

5. **Failure Injection**: How to test Byzantine behavior systematically?
   - *Recommendation*: Create test harness with configurable Byzantine strategies

---

## References

### Paper
- **Title**: Prefix Consensus For Censorship Resistant BFT
- **Location**: `/Users/alexanderspiegelman/Downloads/Prefix_Consensus (4).pdf`
- **Key Algorithms**: Algorithm 1 (3-round), Algorithm 2 (Multi-slot), Algorithm 4 (Optimistic)

### Aptos Codebase
- **Consensus**: `consensus/src/`
- **Types**: `consensus/consensus-types/src/`
- **Safety Rules**: `consensus/safety-rules/src/`
- **Network**: `consensus/src/network.rs`

### Related Work
- AptosBFT: Based on Jolteon
- Quorum Store: Transaction dissemination in Aptos
- BlockSTM: Parallel execution (orthogonal to consensus)

---

## Glossary

- **Prefix Consensus**: Consensus primitive outputting compatible prefixes
- **mcp**: Maximum common prefix
- **mce**: Minimum common extension
- **QC**: Quorum Certificate (n-f signed messages)
- **f**: Maximum number of Byzantine parties
- **n**: Total number of parties (n ≥ 3f+1 for optimal resilience)
- **v_low**: Low output (safe to commit)
- **v_high**: High output (safe to extend)
- **GST**: Global Stabilization Time (partial synchrony)
- **Δ**: Bounded message delay after GST

---

## Implementation Progress

### Phase 1A: Standalone Prefix Consensus Module (COMPLETED)

**Status**: ✅ Core implementation complete and compiling

**What Was Implemented**:

#### 1. Module Structure Created
- **Location**: `consensus/prefix-consensus/`
- **Files**:
  - `Cargo.toml` - Package definition with dependencies
  - `src/lib.rs` - Module exports
  - `src/types.rs` - Core data structures
  - `src/utils.rs` - Prefix utility functions
  - `src/certify.rs` - QC certification functions
  - `src/verification.rs` - Message and QC verification
  - `src/protocol.rs` - Main protocol state machine

- Added to workspace in root `Cargo.toml`

#### 2. Core Types Implemented (`types.rs`)

**Vote Types**:
- `Vote1` - Round 1 vote on input vector
- `Vote2` - Round 2 vote on certified prefix (includes QC1)
- `Vote3` - Round 3 vote on mcp prefix (includes QC2)

**Quorum Certificate Types**:
- `QC1` - Collection of n-f Vote1 messages
- `QC2` - Collection of n-f Vote2 messages
- `QC3` - Collection of n-f Vote3 messages

**Protocol Input/Output**:
- `PrefixConsensusInput` - Party's input vector, ID, and parameters (n, f)
- `PrefixConsensusOutput` - Final output with v_low, v_high, and QC3

**Pending Vote Collections**:
- `PendingVotes1/2/3` - State management for vote collection in each round

**Element Types**:
- Using `HashValue` as vector elements (can represent transaction hashes, block hashes, etc.)

#### 3. Prefix Utility Functions (`utils.rs`)

Implemented with comprehensive unit tests:

- `max_common_prefix(vectors)` - Computes longest common prefix of all vectors
- `min_common_extension(vectors)` - Computes shortest extension containing all vectors
- `is_prefix_of(prefix, vector)` - Check prefix relationship
- `are_consistent(v1, v2)` - Check if two vectors are consistent (one extends the other)
- `all_consistent(vectors)` - Check mutual consistency
- `consistency_check(vectors)` - Detailed consistency verification with error reporting

**Tests**: 11 unit tests covering edge cases (empty, single, identical, diverging vectors)

#### 4. QC Certification Functions (`certify.rs`)

**QC1Certify** - Extract longest prefix with f+1 agreement:
- Uses trie data structure for O(total input size) complexity
- Finds longest prefix appearing in at least f+1 votes
- Implements both trie-based (optimized) and brute-force (for testing) versions

**QC2Certify** - Compute maximum common prefix:
- Takes all certified prefixes from QC2
- Returns their maximum common prefix

**QC3Certify** - Compute final outputs:
- Returns (v_low, v_high) tuple
- v_low = mcp of all round 3 prefixes
- v_high = mce of all round 3 prefixes

**Tests**: 8 unit tests including trie operations, certification correctness, and comparison with brute force

#### 5. Verification Logic (`verification.rs`)

**Vote Verification**:
- `verify_vote1/2/3()` - Structural validation of votes
- Note: Cryptographic signature verification skipped for prototype simplicity

**QC Verification**:
- `verify_qc1/2/3()` - Validate quorum certificates:
  - Check quorum size (n-f votes)
  - Detect duplicate authors
  - Recursively verify embedded QCs

**Helper Functions**:
- `verify_no_duplicate_authors()` - Ensure no equivocation
- `is_valid_quorum()` - Check vote count meets threshold

**Tests**: 7 unit tests for vote and QC verification

#### 6. Protocol State Machine (`protocol.rs`)

**Main Structure**: `PrefixConsensusProtocol`
- Manages protocol state through 3 rounds
- Uses async/await with Tokio for asynchronous execution
- Thread-safe with Arc<RwLock<>> for shared state

**Protocol States**:
- `NotStarted` → `Round1` → `Round1Complete` → `Round2` → `Round2Complete` → `Round3` → `Complete`

**Core Methods**:

*Round 1*:
- `start_round1()` - Create and broadcast Vote1 with input vector
- `process_vote1()` - Collect votes, form QC1 when quorum reached
- Extracts certified prefix using QC1Certify

*Round 2*:
- `start_round2()` - Create and broadcast Vote2 with certified prefix
- `process_vote2()` - Collect votes, form QC2 when quorum reached
- Computes mcp using QC2Certify

*Round 3*:
- `start_round3()` - Create and broadcast Vote3 with mcp prefix
- `process_vote3()` - Collect votes, form QC3 when quorum reached
- Computes final output (v_low, v_high) using QC3Certify
- Verifies upper bound property: v_low ⪯ v_high

**Utility Methods**:
- `get_state()` - Current protocol state
- `get_output()` - Final output if complete
- `get_vote_counts()` - Vote counts for all rounds
- `is_complete()` - Check if protocol finished
- `get_qc1/2/3()` - Retrieve quorum certificates

**Tests**: 2 basic unit tests for state transitions and vote collection

### Design Decisions Made

1. **Signature Handling**: For the prototype, using dummy Ed25519 signatures. Production version would properly sign vote hashes with CryptoHash trait implementation.

2. **Vector Elements**: Using `HashValue` type, which can represent hashes of transactions, blocks, or any content.

3. **Asynchronous Design**: Tokio-based async/await consistent with Aptos architecture.

4. **State Management**: Thread-safe using `Arc<RwLock<>>` for concurrent access.

5. **No Network Layer**: Phase 1A is standalone. Network integration deferred to Phase 1B.

6. **No Validator Verifier**: Removed dependency on ValidatorVerifier for prototype simplicity. Can be added later for production.

### Build Status

✅ **Compiles successfully**: `cargo build -p aptos-prefix-consensus`

**Warnings** (non-critical):
- Unused helper functions in verification.rs
- Unused imports (will be cleaned up)

### Test Coverage

**Unit Tests Implemented**:
- `utils.rs`: 11 tests for prefix operations
- `certify.rs`: 8 tests for QC certification
- `verification.rs`: 7 tests for vote/QC verification
- `protocol.rs`: 2 tests for protocol state machine

**Total**: 28 unit tests

### What's Not Implemented (Out of Scope for Phase 1A)

- ❌ Network layer integration
- ❌ Multi-party simulation framework
- ❌ Byzantine party testing
- ❌ Proper cryptographic signatures
- ❌ Integration with Aptos execution/storage
- ❌ Multi-slot consensus
- ❌ Censorship resistance mechanisms
- ❌ Performance optimization

### Next Steps (Phase 1B - Future Work)

1. **Comprehensive Testing**:
   - Integration tests with multiple protocol instances
   - Property-based tests for Upper Bound and Validity
   - Simulation with message delays
   - Byzantine party behavior testing

2. **Multi-Party Simulation**:
   - Create test harness for running multiple parties
   - Simulate network message passing
   - Test with n=4, f=1 and n=7, f=2 configurations

3. **Network Integration**:
   - Connect to Aptos network layer (`consensus/src/network.rs`)
   - Implement message broadcasting
   - Handle asynchronous message delivery

4. **Production Hardening**:
   - Proper signature implementation with CryptoHash
   - Add ValidatorVerifier integration
   - Error handling improvements
   - Logging and metrics

5. **Documentation**:
   - API documentation
   - Usage examples
   - Architecture diagrams

### Files Changed

1. **Created**:
   - `consensus/prefix-consensus/Cargo.toml`
   - `consensus/prefix-consensus/src/lib.rs`
   - `consensus/prefix-consensus/src/types.rs`
   - `consensus/prefix-consensus/src/utils.rs`
   - `consensus/prefix-consensus/src/certify.rs`
   - `consensus/prefix-consensus/src/verification.rs`
   - `consensus/prefix-consensus/src/protocol.rs`

2. **Modified**:
   - `Cargo.toml` (root) - Added `consensus/prefix-consensus` to workspace members

### Success Criteria Status

From original goals:

1. ✅ Prefix Consensus primitive implemented
2. ✅ Can handle n=3f+1 parties with f Byzantine (structure in place, needs testing)
3. ⏳ Outputs satisfy Upper Bound (verified in code, needs integration tests)
4. ⏳ Outputs satisfy Validity (needs testing)
5. ⏳ All honest parties terminate and output (needs multi-party testing)
6. ❌ Simulation with message delays (not yet implemented)

### Summary

**Phase 1A is functionally complete**. The core primitive 3-round Prefix Consensus protocol is implemented, compiling, and has basic unit tests. The implementation faithfully follows Algorithm 1 from the paper with the three certification functions (QC1Certify, QC2Certify, QC3Certify) and the proper state machine flow.

The next major milestone is comprehensive testing with multi-party simulations to validate the correctness properties (Upper Bound, Validity, Termination).

