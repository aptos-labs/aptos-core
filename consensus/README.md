---
id: consensus
title: Consensus
custom_edit_url: https://github.com/aptos-labs/aptos-core/edit/main/consensus/README.md
---

The consensus component supports state machine replication using the AptosBFT consensus protocol.

## Overview

AptosBFT is a BFT state machine replication protocol for n = 3f+1 validators, tolerating up to f Byzantine faults. It provides safety always and liveness during periods of synchrony (partial synchrony model). The protocol incorporates ideas from [Jolteon](https://arxiv.org/pdf/2106.10362.pdf) (2-chain commit), order votes ([AIP-89](https://github.com/aptos-foundation/AIPs/blob/main/aips/aip-89.md)), and optimistic proposals ([AIP-131](https://github.com/aptos-foundation/AIPs/blob/main/aips/aip-131.md)).

## AptosBFT Protocol

### Rounds and Proposals

Consensus proceeds in rounds. Each round has a designated leader who proposes a block. In the **happy path**, a leader can send an **optimistic proposal** extending the parent block *before* the parent's QC arrives — the leader only needs to have seen the parent proposal and trust it will be certified. This reduces block time to a single network hop.

An optimistic proposal contains a `grandparent_qc` (QC of block r-2) instead of the usual parent QC, since the parent QC doesn't exist yet. Validators buffer optimistic proposals until the parent QC arrives, then convert to a regular proposal and apply standard safety rules.

When optimistic proposals are not possible (e.g. under backpressure, or after a timeout), the leader falls back to a **regular proposal** that includes the parent QC directly.

### Voting and QC Formation

Validators vote on proposals after checking safety rules (voting rules are cleanly separated for auditability). With decoupled execution, votes are on the proposed block itself — not on execution results. Execution happens asynchronously after ordering. 2f+1 votes form a **Quorum Certificate** (QC), which proves a supermajority agreed on a block.

### Ordering via Order Votes

After forming QC(r), validators immediately broadcast an **order vote** on that QC to all validators. When 2f+1 order votes are collected, block r is ordered. This achieves the theoretical minimum of 3 network hops for BFT ordering under partial synchrony:

1. Leader proposes block B(r) (optimistically, without waiting for parent QC)
2. Validators vote on B(r)
3. QC(r) forms; validators broadcast order votes on QC(r)
4. 2f+1 order votes → block r is ordered

The order vote safety rule (`safe_for_order_vote`) prevents a validator from order-voting for a round if it has already timed out at that round (tracked via `highest_timeout_round`). This prevents conflicting ordering decisions across forks.

2f+1 order votes form a `WrappedLedgerInfo` — a commit certificate.

Without order votes, the **2-chain commit rule** applies as fallback: B(r) is committed when B(r) has a QC and its direct child B(r+1) also has a QC.

### Timeout and View-Change

When a round times out without a QC:

- Validators broadcast a `RoundTimeout` message containing their highest QC and a timeout signature.
- Receiving f+1 timeout messages from other validators triggers a local timeout, accelerating round advancement without waiting for the full timeout duration.
- 2f+1 timeout signatures form a `TwoChainTimeoutCertificate` (TC).
- The TC allows the next leader to propose without the previous round's QC, ensuring liveness.

### Round State

The `RoundState` drives round advancement. A `NewRoundEvent` is triggered either by receiving a QC (happy path) or a TC (unhappy path). Timeouts use exponential backoff: `base_ms * exponent_base^min(round_index, max_exponent)`.

### Leader Election

Multiple strategies are supported — round-robin rotation and reputation-based (where validators that fail to produce blocks are penalized).

### Safety Rules (persisted across restarts)

- `last_voted_round`: prevents voting twice in the same round (no equivocation)
- `preferred_round`: only vote for blocks whose parent QC round >= preferred_round (the round of the highest 2-chain head seen), preventing votes on forks that could conflict with committed blocks
- `highest_timeout_round`: prevents order-voting after timeout in the same round

### Liveness

Requires two consecutive honest leaders to order a block after GST (because the first leader sends an optimistic proposal that the second leader builds on). Without optimistic proposals, one honest leader suffices.

### Epoch Boundaries

Consensus operates within epochs. An epoch defines the validator set and configuration. When a reconfiguration transaction is committed, the current epoch ends and a new one begins. All consensus state (rounds, QCs, block tree) is reset at epoch boundaries.

## Architecture

```
                         Network Layer
                             |
                    +--------v---------+
                    |   EpochManager   |  Lifecycle: epoch init, validator set, channels
                    +--------+---------+
                             |
                    +--------v---------+
                    |  RoundManager    |  Core event loop: proposals, votes, timeouts
                    +--------+---------+
                             |
          +------------------+------------------+
          |                  |                  |
  +-------v------+  +-------v-------+  +-------v--------+
  |  BlockStore  |  | SafetyRules   |  |  RoundState    |
  | (block tree) |  | (vote rules,  |  |  (timeouts,    |
  |              |  |  persistence) |  |   round mgmt)  |
  +--------------+  +---------------+  +----------------+
                                                |
  +--------------+                     +--------v--------+
  | PendingVotes |                     | ProposalGen +   |
  | + OrderVotes |                     | ProposerElect   |
  | (aggregation)|                     | (leader duty)   |
  +--------------+                     +-----------------+
```

### Consensus Message Types

| Message           | Purpose                                | Happy-path sender → receiver   |
| ----------------- | -------------------------------------- | ------------------------------ |
| `ProposalMsg`     | Block proposal with parent QC          | Leader → all validators        |
| `OptProposalMsg`  | Optimistic proposal (no parent QC yet) | Leader → all validators        |
| `VoteMsg`         | Vote on a proposal                     | Validator → all validators     |
| `OrderVoteMsg`    | Order vote on a QC                     | Validator → all validators     |
| `RoundTimeoutMsg` | Timeout vote with highest QC           | Validator → all validators     |
| `SyncInfo`        | Sync metadata (highest QC, TC, commit) | Piggybacked on other messages  |

## Implementation Details

The consensus component is mostly implemented in the [Actor](https://en.wikipedia.org/wiki/Actor_model) programming model — i.e., it uses message-passing to communicate between different subcomponents with the [tokio](https://tokio.rs/) framework used as the task runtime. The primary exception to the actor model (as it is accessed in parallel by several subcomponents) is the consensus data structure *BlockStore* which manages the blocks, execution, quorum certificates, and other shared data structures. The major subcomponents in the consensus component are:

* **EpochManager** manages epoch lifecycle, validator set initialization, and channel wiring between subcomponents.
* **RoundManager** is the core event processor — it handles all consensus messages (proposals, votes, timeouts) and drives the protocol.
* **BlockStore** maintains the tree of proposal blocks, block execution, votes, quorum certificates, and persistent storage. It is responsible for maintaining the consistency of the combination of these data structures and can be concurrently accessed by other subcomponents.
* **RoundState** is responsible for the liveness of the consensus protocol. It changes rounds due to timeout certificates or quorum certificates and proposes blocks when it is the proposer for the current round.
* **SafetyRules** is responsible for the safety of the consensus protocol. It processes quorum certificates and LedgerInfo to learn about new commits and guarantees that the voting rules are followed — even in the case of restart (since all safety data is persisted to local storage).
* **PendingVotes / PendingOrderVotes** aggregate votes and order votes into QCs, TCs, and commit certificates.

All consensus messages are signed by their creators and verified by their receivers. Message verification occurs closest to the network layer to avoid invalid or unnecessary data from entering the consensus protocol.

## Safety Invariants

1. **No equivocation**: A validator votes at most once per round. Enforced by persisting `last_voted_round` in SafetyRules.

2. **Preferred round**: A validator only votes for a block whose parent QC round >= its `preferred_round`. This prevents voting on forks that could conflict with the committed chain.

3. **Order vote safety**: A validator must not order-vote for a round where it has timed out (`highest_timeout_round` check). This prevents conflicting ordering decisions across forks.

4. **Deterministic execution**: All validators must produce identical execution results for the same ordered block sequence. Corollary: use deterministic data structures (`BTreeMap`, not `HashMap`) when iteration order affects serialization.

5. **Rolling deployment safety**: During rolling upgrades, all nodes must produce identical `BlockMetadataTransaction`s regardless of code version. Gate new behavior on on-chain feature flags.

## Configuration

Key parameters (in `ConsensusConfig` and on-chain `OnChainConsensusConfig`):

| Parameter                             | Purpose                                     |
| ------------------------------------- | ------------------------------------------- |
| `max_block_txns`                      | Max transactions per proposed block         |
| `max_receiving_block_txns`            | Max transactions accepted in received block |
| `round_initial_timeout_ms`            | Base timeout for rounds                     |
| `round_timeout_backoff_exponent_base` | Exponential backoff multiplier              |
| `round_timeout_backoff_max_exponent`  | Max exponent for timeout backoff            |
| `enable_optimistic_proposal_rx`       | Accept optimistic proposals                 |
| `enable_optimistic_proposal_tx`       | Send optimistic proposals                   |
| `order_vote_enabled`                  | Enable 3-hop ordering via order votes       |

## How is this module organized?

### Core Consensus

| File                                   | Purpose                                               |
| -------------------------------------- | ----------------------------------------------------- |
| `consensus/src/round_manager.rs`       | Core event processor — handles all consensus messages |
| `consensus/src/epoch_manager.rs`       | Epoch lifecycle, validator set init, channel wiring   |
| `consensus/src/network.rs`             | Network message sending/receiving                     |
| `consensus/src/pending_votes.rs`       | Vote aggregation → QC/TC formation                    |
| `consensus/src/pending_order_votes.rs` | Order vote aggregation                                |
| `consensus/src/state_computer.rs`      | Execution interface                                   |

### Block Storage

| File                                          | Purpose                                  |
| --------------------------------------------- | ---------------------------------------- |
| `consensus/src/block_storage/block_store.rs`  | Block tree, QC tracking, execution state |
| `consensus/src/block_storage/block_tree.rs`   | Tree structure with parent/QC links      |
| `consensus/src/block_storage/sync_manager.rs` | Missing block synchronization            |

### Liveness

| File                                           | Purpose                                 |
| ---------------------------------------------- | --------------------------------------- |
| `consensus/src/liveness/round_state.rs`        | Pacemaker — round progression, timeouts |
| `consensus/src/liveness/proposal_generator.rs` | Block proposal generation, backpressure |
| `consensus/src/liveness/proposer_election.rs`  | Leader election trait                   |
| `consensus/src/liveness/leader_reputation.rs`  | Reputation-based leader selection       |

### Safety

| File                                                | Purpose                              |
| --------------------------------------------------- | ------------------------------------ |
| `consensus/safety-rules/src/safety_rules.rs`        | Voting rules — prevents equivocation |
| `consensus/safety-rules/src/safety_rules_2chain.rs` | 2-chain timeout rules                |
| `consensus/safety-rules/src/consensus_state.rs`     | Persistent safety state              |

### Consensus Types

| File                                                   | Purpose                                          |
| ------------------------------------------------------ | ------------------------------------------------ |
| `consensus/consensus-types/src/block.rs`               | Block structure                                  |
| `consensus/consensus-types/src/quorum_cert.rs`         | QuorumCert (2f+1 vote signatures)                |
| `consensus/consensus-types/src/vote.rs`                | Vote message                                     |
| `consensus/consensus-types/src/order_vote.rs`          | Order vote for 3-hop ordering                    |
| `consensus/consensus-types/src/wrapped_ledger_info.rs` | Commit cert from order votes                     |
| `consensus/consensus-types/src/timeout_2chain.rs`      | Timeout certificate                              |
| `consensus/consensus-types/src/opt_proposal_msg.rs`    | Optimistic proposal message                      |
| `consensus/consensus-types/src/safety_data.rs`         | Persistent safety state                          |
| `consensus/consensus-types/src/payload.rs`             | Payload types (DirectMempool, QuorumStore, etc.) |

### Related Subsystems

See separate READMEs for:
- `consensus/src/pipeline/` — Decoupled execution pipeline (execute, sign, persist, broadcast)
- `consensus/src/quorum_store/` — QuorumStore for data dissemination

### Directory Structure

```
consensus
├── src
│   ├── block_storage          # In-memory storage of blocks and related data structures
│   ├── consensusdb            # Database interaction to persist consensus data for safety and liveness
│   ├── liveness               # RoundState, proposer, and other liveness related code
│   ├── pipeline               # Decoupled execution pipeline
│   ├── quorum_store           # QuorumStore for data dissemination
│   └── test_utils             # Mock implementations that are used for testing only
├── consensus-types            # Consensus data types (i.e. quorum certificates)
└── safety-rules               # Safety (voting) rules
```

## Testing

```bash
cargo test -p aptos-consensus          # Consensus unit tests
cargo test -p aptos-consensus-types    # Type tests
cargo test -p aptos-safety-rules       # Safety rules tests
cargo test -p smoke-test               # E2E smoke tests
# Forge tests: see testsuite/forge-cli/src/suites/
```
