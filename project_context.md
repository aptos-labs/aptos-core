# Project Context: Prefix Consensus Implementation

## Overview

Implementing Prefix Consensus protocols (from research paper "Prefix Consensus For Censorship Resistant BFT") within Aptos Core for leaderless, censorship-resistant consensus.

**Current Phase**: Multi-Slot Consensus (Algorithm 4) — Phases 1-8 complete, Phase 9 next
**Completed**: Basic Prefix Consensus, Strong Prefix Consensus (Phases 1-9), Stake-Weighted Quorum Refactoring, Multi-Slot Phases 1-8

---

## Background Summary

### AptosBFT (Current Consensus)
- Leader-based BFT protocol (Jolteon), 3-chain commit rule, partial synchrony
- Central components: RoundManager, BlockStore, SafetyRules
- Limitations: Single leader per round (censorship risk)

### Prefix Consensus Protocol (Algorithm 1)
- Parties propose vectors, output compatible vectors extending max common prefix
- Properties: Upper Bound (v_low ≼ v_high), Validity (mcp(inputs) ≼ v_low), Termination
- 3 rounds: Vote1→QC1, Vote2→QC2, Vote3→QC3
- Proof-of-stake weighted voting (>2/3 stake for QCs, >1/3 for certification)

### Strong Prefix Consensus (Algorithm 3)
- Multi-view protocol where ALL parties agree on identical v_high output
- Views run inner PC instances; certificates chain back to View 1 for agreement
- Direct certificates (non-empty v_high) and indirect certificates (empty view, >1/3 stake)
- Cyclic ranking rotation per view for leaderless progress

### Multi-Slot Consensus (Algorithm 4 — current focus)
- Each slot: every party broadcasts proposal, waits 2Δ, forms input vector ordered by rank^MC, runs SPC
- Ranking update: demote first excluded party when v_high is not full length
- Commit from v_high per slot (v_low ignored in initial implementation)
- Plan: `.plans/multi-slot-consensus.md`

---

## Completed Work

### Basic Prefix Consensus ✅
- Production-ready 3-round async protocol with full BLS signature verification
- ~3300 LOC across 9 source files
- 103 unit tests, 2 smoke tests

### Strong Prefix Consensus ✅ (Phases 1-9)
- Multi-view protocol with agreement property
- Certificate types (Direct/Indirect), StrongPCCommit broadcast for termination
- Truncated vector optimization (vectors length 1-3 instead of n)
- InnerPCAlgorithm trait abstraction (Phase 9) — both managers generic over inner algorithm
- ~5000+ LOC, 189 unit tests, 4 smoke tests (2 basic + 2 strong)

### Stake-Weighted Quorum ✅
- Replaced count-based (f+1, n-f) with proof-of-stake weighted voting (>1/3, >2/3 stake)

### Key Design Decisions (still relevant)

**Signature/Replay Protection**:
- Vote1/2/3 SignData includes `view: u64` and `slot: u64` — prevents cross-view and cross-slot replay
- QCs derive view from embedded votes (no explicit view field)

**Certificate Chain Verification** (StrongPCCommit):
- Terminal condition: `parent_view() == 1` (handles both DirectCert and IndirectCert)
- Hash-based chain linkage: v_low → first_non_bot → hash(chain[0]) → ... → View 1
- Full certs in StrongPCCommit (O(chain×N)), hashes in view-by-view votes (O(N))

**Three-Way Decision for Views > 1**:
- v_low has non-⊥ → Commit (trace back to View 1)
- v_high has non-⊥ → DirectCertificate (progress, no commit)
- Both all-⊥ → EmptyViewMessage → IndirectCertificate

**SPC Task Model**:
- `StrongPrefixConsensusManager.run()` consumes self, runs as separate tokio task
- Communication via channels: messages in, SPCOutput out
- SlotManager holds channel senders, not the manager object

---

## Multi-Slot Consensus — Current Work

**Plan**: `.plans/multi-slot-consensus.md` (13 phases, ~4550 LOC estimated)
**Goal**: Replace RoundManager with SlotManager, blockchain works end-to-end

### Architecture
```
EpochManager
  └─> SlotManager (NEW, replaces RoundManager)
        ├─> Per Slot: broadcast proposal, collect, form input vector, run SPC
        ├─> On v_high: build Block (PrefixConsensusBlock), send to execution pipeline
        └─> Update ranking, advance to next slot

Execution Pipeline (unchanged):
  BufferManager → PipelineBuilder → BlockExecutor → Storage → State Sync
```

### Key Design Decisions (resolved)
1. **BlockType**: New `PrefixConsensusBlock` variant (follows DAGBlock pattern) with `failed_authors`, `validator_txns`, `payload`, `authors`, `proposal_hashes`, `parent_block_id`
2. **v_low ignored**: Blocks created solely from v_high. v_low fast commit deferred to future work.
3. **DirectMempool**: Inline transactions for prototype. QuorumStore deferred.
4. **Config**: Local `enable_prefix_consensus: bool` in NodeConfig (not on-chain config)
5. **Slot→round mapping**: `round == slot` (each epoch starts with genesis at round 0, one block per slot)
6. **Timestamps**: `max(parent_timestamp + 1, now_usecs)` (same as DAG)
7. **2Δ timeout**: 300ms initially (`SLOT_PROPOSAL_TIMEOUT`)
8. **Ordering proof**: `AggregateSignature::empty()` (same as DAG)
9. **SPC per slot**: Separate tokio task, channels for communication, graceful shutdown via close_tx/ack

### Implementation Phases
1. ~~Slot types + network messages (~300 LOC)~~ ✅
2. ~~Multi-slot ranking manager (~150 LOC)~~ ✅
3. ~~Proposal buffer + slot state (~400 LOC)~~ ✅
4. ~~Block builder + PrefixConsensusBlock variant (~300 LOC)~~ ✅
5. ~~SlotManager core (~800 LOC)~~ ✅
6. ~~SPC integration refactor (~300 LOC)~~ ✅
7. ~~Payload resolution: late buffering + fetch protocol (~350 LOC)~~ ✅
8. ~~EpochManager integration (~400 LOC)~~ ✅
9. BlockType integration across codebase (~400 LOC, grep-driven)
10. Execution pipeline compatibility (~300 LOC, grep-driven)
11. Smoke tests (~400 LOC)
12. Documentation + cleanup (~100 LOC)

---

## Repository State

- **Branch**: `prefix-consensus-prototype`
- **HEAD**: Multi-Slot Phase 8 (EpochManager integration — config flag, startup, routing, shutdown)
- **Tests**: 246/246 unit tests (237 prefix-consensus + 9 slot manager), 6/6 smoke tests
- **Build**: Clean

### Repository Structure
```
consensus/prefix-consensus/src/
├── types.rs              - Vote/QC types + ViewProposal/CertFetch (1039 lines)
├── protocol.rs           - 3-round state machine (~585 lines)
├── manager.rs            - Basic PC orchestrator, generic over InnerPCAlgorithm (~430 lines)
├── network_interface.rs  - Generic network adapters (SubprotocolNetworkClient/Sender/Adapter) (~370 lines)
├── network_messages.rs   - PrefixConsensusMsg + StrongPrefixConsensusMsg (795 lines)
├── signing.rs            - BLS helpers + create_signed_vote* factories (~240 lines)
├── certify.rs            - QC formation with trie (220 lines)
├── verification.rs       - Vote/QC validation (150 lines)
├── utils.rs              - Prefix operations + first_non_bot (210 lines)
├── certificates.rs       - Certificates + StrongPCCommit (1348 lines)
├── view_state.rs         - RankingManager, ViewState, ViewOutput (695 lines)
├── strong_protocol.rs    - Strong PC state machine (918 lines)
├── strong_manager.rs     - Strong PC orchestrator, generic over InnerPCAlgorithm (~1290 lines)
├── inner_pc_trait.rs     - InnerPCAlgorithm trait (~90 lines)
├── inner_pc_impl.rs      - ThreeRoundPC implementation (~400 lines)
├── slot_types.rs         - SlotProposal, SlotConsensusMsg, signing (~230 lines) — Phase 1
├── slot_ranking.rs       - MultiSlotRankingManager, cross-slot demotion (~80 lines) — Phase 2
├── slot_state.rs         - ProposalBuffer, SlotPhase, SlotState (~645 lines) — Phase 3
└── block_builder.rs      - build_block_from_v_high (~270 lines) — Phase 4

consensus/src/prefix_consensus/
├── mod.rs                - Module declarations
└── slot_manager.rs       - SlotManager orchestrator, SPCSpawner trait, RealSPCSpawner + 9 unit tests (~600 lines) — Phases 5-6

testsuite/smoke-test/src/consensus/
├── prefix_consensus/     - 2 basic PC smoke tests
└── strong_prefix_consensus/ - 2 strong PC smoke tests
```

### Plans
- `.plans/network-integration.md` — Basic Prefix Consensus (complete)
- `.plans/strong-prefix-consensus.md` — Strong Prefix Consensus (complete)
- `.plans/inner-pc-trait.md` — Inner PC Abstraction Trait (complete)
- `.plans/multi-slot-consensus.md` — Multi-Slot Consensus Algorithm 4 (current)
- `.plans/phase1-slot-types.md` — Phase 1: Slot types + network messages (complete)
- `.plans/phase2-slot-ranking.md` — Phase 2: Multi-slot ranking manager (complete)
- `.plans/phase3-slot-state.md` — Phase 3: Proposal buffer + slot state (complete)
- `.plans/phase4-block-builder.md` — Phase 4: Block builder + PrefixConsensusBlock (complete)
- `.plans/phase5-slot-manager.md` — Phase 5: SlotManager core (complete)
- `.plans/phase6-spc-integration.md` — Phase 6: SPC integration (complete)
- `.plans/phase7-payload-resolution.md` — Phase 7: Payload resolution: late buffering + fetch
- `.plans/phase12-verifiable-ranking.md` — Phase 13: Verifiable ranking with SPC-aware demotion (after end-to-end)

---

## TODO (Future Work)

### Multi-Slot (deferred from current plan)
- [ ] **v_low fast commit**: Commit from v_low when all entries non-⊥ (full prefix case)
- [ ] **v_low early commit (general)**: Two blocks per slot — fast block from v_low, completion from v_high. Requires `InnerPCAlgorithm` to expose v_low at QC2 (before QC3/commit) so the strong manager can relay it through `output_tx` before v_high is known
- [ ] **Slot pipelining**: Overlap proposal collection for slot s+1 while SPC for slot s runs
- [ ] **QuorumStore integration**: Replace DirectMempool for production payload dissemination
- [ ] **On-chain config**: Add PrefixConsensus variant to ConsensusAlgorithmConfig
- [ ] **State persistence**: Persist slot state for crash recovery without state sync
- [ ] **Payload fetch optimization**: Replace broadcast-to-all fetch with single-peer-then-escalation (request one random peer first, broadcast only on timeout)
- [ ] **Slot catch-up mechanism**: External mechanism to jump forward to later slots when behind (analogous to current consensus catch-up). Needed after full integration

### Strong PC (deferred)
- [ ] **Certificate fetching protocol**: Fetch by hash for Byzantine withholding
- [ ] **StrongPCCommit chain cap**: Cap embedded certs if chains > 5
- [ ] **Configurable view start timeout**: Make VIEW_START_TIMEOUT (300ms) configurable
- [ ] **Empty view optimization**: Skip inner PC when no certificates at timeout
- [x] **Collapse network bridges**: Replaced 3 bridges + 3 clients + 2 sender traits/adapters with generics
- [ ] **Garbage collect on slot commit**: Clean up view_states, pc_states, pending_fetches, cert store
- [ ] **Remove smoke test scaffolding**: `write_output_file()` (file-based output polling), `start_prefix_consensus()` / `start_strong_prefix_consensus()` (on-demand entry points in EpochManager), and associated helpers. Remove once smoke tests verify through the multi-slot execution pipeline

### Long Term
- [ ] **Optimistic Variants**: 2-round good case (paper Appendix D)
- [ ] **Communication Optimization**: Reduce from O(n²L) to O(n² + nL)
- [ ] **Byzantine proposal handling**: Detect equivocating proposals, reputation tracking

---

## References

**Paper**: "Prefix Consensus For Censorship Resistant BFT" (Feb 2024)
- Location: `/Users/alexanderspiegelman/Downloads/Prefix_Consensus (5).pdf`
- Algorithm 1: Basic Prefix Consensus (✅ complete)
- Algorithm 3: Strong Prefix Consensus (✅ complete)
- Algorithm 4: Multi-slot Consensus (🚧 current)

**Aptos Integration Points**:
- `consensus/src/epoch_manager.rs` — Protocol lifecycle management
- `consensus/src/network_interface.rs` — Message routing + network bridges
- `consensus/consensus-types/src/block_data.rs` — BlockType enum (adding PrefixConsensusBlock)
- `consensus/src/dag/dag_driver.rs` — DAG reference implementation for PayloadPullParameters
