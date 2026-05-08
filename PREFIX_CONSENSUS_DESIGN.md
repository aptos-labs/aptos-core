# Prefix Consensus Productionization — Protocol Design

Status: draft (2026-05-08)
Based on: *Prefix Consensus For Censorship Resistant BFT* (Xiang, Tonkikh, Spiegelman, 2026), arXiv:2602.02892v1

## 1. Goal

Add **f-censorship resistance** to AptosBFT by inserting a Prefix Consensus (PC) layer between Quorum Store (QS) and the existing leader-based BFT, while keeping QS and AptosBFT essentially intact.

Big picture per slot:

```
QS batches  ──►  Group leaders bundle digests  ──►  Prefix Consensus (continuous)
                                                       │
                                              ┌────────┴────────┐
                                              ▼                 ▼
                                        v_low (eager)       v_high (+ π)
                                              │                 │
                                              ▼                 ▼
                                       Execution          AptosBFT block
                                        pipeline          payload (leader
                                       (no BFT QC)        proposes its v_high)
                                                                │
                                                                ▼
                                                          BFT QC  ──►  ledger
```

Core idea: PC's Upper Bound says every honest validator's "safe-to-commit" vector `v_low` is a prefix of every honest `v_high`. We commit `v_low` immediately (fast path, no BFT round-trip) and let AptosBFT agree on a single `v_high` that extends every honest `v_low` (slow path, gives global agreement and finalizes anything beyond `v_low`).

## 2. Non-goals

- Replacing AptosBFT with the paper's leaderless Strong Prefix Consensus (Algorithm 3). We deliberately reuse leader-based agreement for `v_high`.
- Replacing QuorumStore. QS's batch-author / sign / PoS-aggregate path is preserved.
- Multi-leader proposal at the BFT layer.
- Encrypted mempool / strict order fairness. PC gives bounded-slot inclusion, not transaction-level ordering fairness.

## 3. Answered design choices (from interactive Q&A)

| # | Choice | Decision |
|---|--------|----------|
| 1 | What does each PC vector entry contain? | Per-group bundle. Validators are partitioned into groups (likely by region). Each group has a (potentially rotating) group leader that participates in PC. The leader's PC proposal is a bundle of PoS / opt-batch digests from its region. Vector has length G (number of groups). |
| 2 | PC ↔ BFT cadence | Continuous, both layers. PC slots are pipelined (slot `i+1` starts after slot `i`'s `QC1`). AptosBFT runs continuously and proposes a (possibly empty) list of all not-yet-finalized `v_high`s in each block. |
| 3 | `v_low` commit semantics | Eager. As soon as a validator's PC slot outputs `v_low`, it is fed into the execution pipeline. AptosBFT later confirms a `v_high` that extends it. |
| 4 | What does BFT leader propose? | A list of `(v_high_s, π_high_s)` pairs for all PC slots `s` with finalized PC output that the leader has and that have not yet been BFT-finalized. Empty list if none. Validators vote iff every `(v_high_s, π_high_s)` in the list satisfies `F^high` AND each one extends their own `v_low_s` for that slot. |
| 5 | Ranking source | BFT-finalized `v_high`, with k-slot delay: ranking for slot `i` is derived from the BFT-finalized `v_high_{i-k}`. `k` is fixed (initial estimate ~ BFT commit lag in PC slots; tunable, with back-pressure later). |

## 4. Three layers

### 4.1 QS layer (unchanged)

Every validator authors batches as today, broadcasts them, collects 2f+1 signatures into PoS. PoS sit in the existing `BatchProofQueue`. PC consumes those PoS digests; it does not touch batch authoring or signing.

### 4.2 Group layer (new)

- Validators are partitioned into G groups. Natural choice: by region (apne1, euwe6, usw2, …), so groups correspond to fault-correlated geographic / organizational clusters.
- Group composition is fixed for the duration of an epoch (computed at reconfiguration).
- Each group has a designated **group leader** for each PC slot, rotating among group members on a deterministic schedule.
- For each PC slot `s`, the group leader of group `g` produces a `GroupProposal_{g,s}`:
  - `slot`, `group_id`, leader id
  - A list of PoS digests (and optionally opt-batch digests) sourced from QS, drawn from batches authored by group members and not yet included in any prior committed PC slot
  - Signed by the leader
- The proposal is broadcast to all validators (gossip/RB).
- If the group leader is silent or equivocates within slot `s`, group `g`'s slot-`s` entry is `⊥` from the network's view; PC handles `⊥` natively, and the group is demoted in the next ranking.

### 4.3 PC layer (new)

Direct application of the paper's Algorithm 1, with these specializations:

- **Voters**: all n validators, weighted by stake; quorum = 2f+1 voting power, identical to AptosBFT today.
- **Vector length**: G (number of groups). Position `k` of slot `s`'s vector = `H(GroupProposal_{p_k, s})` per the current ranking, or `H(⊥)` if missing.
- **Three voting rounds** (paper §3): vote-1 carries `v_in`; vote-2 carries `x = QC1Certify(QC1)`; vote-3 carries `x_p = QC2Certify(QC2)`. Each round forms a QC over n−f = 2f+1 votes.
- **Output**: `(v_low, v_high) := QC3Certify(QC3) = (mcp(x_p ∈ QC3), mce(x_p ∈ QC3))`. The proof `π_low = π_high = QC3` is publicly verifiable.

PC slots run **pipelined**: slot `s+1` starts as soon as slot `s`'s `QC1` is locally formed (i.e., when the validator has collected 2f+1 vote-1 messages for slot `s`). Slot `s+1`'s vote-1 can go out immediately; the validator will then be juggling rounds 2, 3 of slot `s` and round 1 of slot `s+1` simultaneously, with similar overlap for further slots. In steady state, ~3 PC instances are in flight per validator.

For ranking consistency across pipelined slots, slot `s`'s ranking is computed from the **BFT-finalized** `v_high_{s-k}` for a fixed pipeline depth `k`. See §7.

## 5. Per-slot protocol flow

Per validator, with multiple slots in flight:

For a slot `s`:

1. **Slot start trigger**: validator starts slot `s` once it has the synchronization signal from slot `s-1`. Initial choice: starts slot `s` after observing `QC1_{s-1}` (locally formed). Details deferred — exact rule and handling of stragglers/late starts will be refined.
2. **Compute `ranking_s`**: derived deterministically from BFT-finalized `v_high_{s-k}` (see §7). All validators agree on `ranking_s` because BFT-finalized `v_high` is unique.
3. **Build `v_in_s`**: wait until either (a) all `GroupProposal_{g,s}` for `g ∈ ranking_s` are received, or (b) a slot timer expires; then `v_in_s = [H(GroupProposal_{p_1,s}), …, H(GroupProposal_{p_G,s})]` with `H(⊥)` for missing.
4. **Run paper's Algorithm 1**: broadcast vote-1 → collect QC1 → broadcast vote-2 → collect QC2 → broadcast vote-3 → collect QC3.
5. **Compute `(v_low_s, v_high_s)`** from QC3.
6. **Eager commit `v_low_s`** into the execution pipeline (§6).
7. **Emit `(v_high_s, π_high_s)`** to the queue consumed by the AptosBFT layer.

Slots overlap: when slot `s` reaches step 4-round-1 (`QC1` formed), slot `s+1` starts at step 1. Steady-state pipeline depth ≈ 3 in-flight slots per validator.

In parallel, the AptosBFT layer continuously reads the queue of finalized PC outputs and proposes them in BFT blocks (§8).

## 6. Eager commit of `v_low`

The protocol property we lean on: by Corollary 2.3 of the paper, `v_i^low ∼ v_j^low` for all honest `i, j` — any two honest validators' low outputs are mutually consistent (one is a prefix of the other). So eagerly executing the local `v_low` cannot fork against another honest validator's eager commit.

For each non-`H(⊥)` entry of `v_low_s`:
- Resolve the entry hash to its `GroupProposal` (received over the wire or fetched from a peer).
- Resolve each PoS digest in that bundle to its underlying batch (via existing QS batch retrieval).
- Concatenate the contained transactions in a deterministic per-bundle order (e.g., gas-bucket sort, as today).

The resulting transaction sequence is fed into the execution pipeline as a *pre-commit* — the state advances locally, but the slot is **not yet final** in the ledger. Final commit happens when AptosBFT confirms a `v_high_t` for some `t ≥ s`, at which point all eagerly pre-committed slots up through `s` become final by Upper Bound.

Why this is safe under crashes / view-changes:
- Eager pre-commits up through slot `s` are guaranteed (Upper Bound) to be a prefix of any future BFT-finalized `v_high`. There is no scenario where we have to revoke an eagerly-committed transaction.
- A Byzantine BFT leader cannot construct a verifiable `v_high` that violates Upper Bound, because `F^high` requires the candidate to equal `mce(QC2.x_p ∈ QC3)` for an actual QC3, and Lemma 3.1 forces those `x_p` to be mutually consistent.

## 7. Ranking and demotion

Per paper §6.1.1 / §7.2, adapted to the pipelined hybrid:

- **Source**: BFT-finalized `v_high`. Local PC `v_high` is not unique across validators (only Upper-Bound-consistent), so using it would let validators compute different `ranking_s` values; vector entries would reorder differently; no QC could form for slot `s`. Liveness breaks.
- **k-delay**: ranking for slot `s` is derived from BFT-finalized `v_high_{s-k}` for a fixed pipeline depth `k`. `k` must be at least large enough that, in normal operation, BFT has finalized slot `s-k` by the time the validator needs to start slot `s`. Initial choice: pick `k` based on the expected BFT-commit lag measured in PC slots (with PC slots starting every ~1 round and BFT commit at ~3 rounds, a starting point on the order of `k ≈ a few` is reasonable). Tunable; back-pressure can dynamically widen `k` later.
- **Update rule** (`ranking_s = Update(ranking_{s-1}, v_high_{s-k})`):
  - If `|v_high_{s-k}| = G` (full): `ranking_s = ranking_{s-1}` (no demotion).
  - If `|v_high_{s-k}| = ℓ < G`: demote group `p_{ℓ+1}` of `ranking_{s-1}` (first excluded position) to the end; all others keep relative order.

**Initial ranking** for slots `0..k-1` (before any BFT-finalized output exists yet): a fixed bootstrap ranking at epoch start (e.g., region-alphabetic). Used unchanged for the first `k` slots.

**Cost of the k-delay**: a Byzantine group leader is demoted `k` slots later than in the synchronous-paper baseline. The censored-slot bound becomes "(k + 1) slots per Byzantine group" instead of "1 slot per Byzantine group" (paper). For long-lived blockchains where slots-per-Byzantine-group remains O(1) per group, this is still negligible.

**PC-instance synchronization**: separate from ranking. The trigger for starting slot `s+1` (e.g., observing `QC1_s`) ensures all validators eventually catch up to the same slot index, while the deterministic ranking derived from BFT-finalized `v_high_{s-k}` ensures they order vector entries the same way. Exact synchronization rule and handling of late-arriving votes for past slots are deferred.

## 8. AptosBFT integration (the `v_high` agreement layer)

AptosBFT runs continuously, **decoupled from PC slot cadence**. Each BFT block carries a payload that is a (possibly empty) ordered list of PC outputs:

```
BFT block payload = [(v_high_{s_a}, π_high_{s_a}), (v_high_{s_a+1}, π_high_{s_a+1}), …, (v_high_{s_b}, π_high_{s_b})]
```

where `s_a = last_BFT_finalized_slot + 1` and `s_b ≤ s_a + N` for some bandwidth bound `N`. If the leader has no new PC outputs, the payload list is empty; the BFT block still progresses the chain.

**BFT leader behavior**: at proposal time, scan local PC output queue for all slots in `(last_BFT_finalized_slot, latest_local_PC_output]`, take a contiguous prefix from `last_BFT_finalized_slot + 1` (so slots are finalized in order), bound to `N`, and propose them.

**Voting rule** (extension of today's safety rules): a validator votes on a payload list iff
1. All existing safety-rules checks pass (`last_voted_round`, `preferred_round`, …).
2. The first slot in the list = `last_BFT_finalized_slot + 1` (no gaps, no replays).
3. For each `(v_high_s, π_high_s)` in the list:
   - `F^high(v_high_s, π_high_s)` verifies (i.e., `π_high_s` is a valid PC slot-`s` `QC3` and `v_high_s` matches `QC3Certify(π_high_s).x_pe`).
   - `v_high_s` extends this validator's locally-output `v_low_s` (or `v_low_t` for some `t ≤ s` it has pre-committed).
4. Slots are strictly contiguous (`s_{i+1} = s_i + 1`).

If any item in the list fails to verify or fails the `v_low` extension check, the validator does not vote on that block — BFT pacemaker moves on. Upper Bound guarantees an honest leader's list always validates; for a Byzantine leader, the worst case is one timed-out BFT round.

**Pipeline reconciliation on BFT commit**:
- Eagerly pre-committed slots `s_a..s_b` are already executed; they are now finalized in the ledger.
- For each slot `s ∈ [s_a, s_b]`, the "tail" of `v_high_s` beyond local `v_low_s` (if any) is also finalized; the executor extends the pre-committed state with those additional transactions in slot order before continuing.
- `last_BFT_finalized_slot` advances to `s_b`.

**Empty-block fallback**: when no PC output is available (cold start, network partition recovery, or PC stall), BFT keeps producing empty-payload blocks. The chain advances; PC catches up; subsequent BFT blocks pick up the backlog.

## 9. Properties

### Safety

- **Prefix safety of `v_low`** (Validity + Upper Bound of PC): every honest validator's eager pre-commit is a prefix of every other honest validator's eager pre-commit and of every BFT-finalized state.
- **Agreement on `v_high`**: from AptosBFT (unchanged).
- **No fork between eager pre-commit and BFT-finalization**: by Upper Bound, any verifiable `v_high` extends every honest `v_low`, so the eager pre-commit is always a prefix of the BFT-finalized state.

### Liveness

- **PC slot termination** (Theorem 3.4 of paper): every slot terminates after sufficient asynchrony (or always, after GST).
- **BFT round termination**: from AptosBFT's pacemaker / TC / leader rotation (unchanged).
- **End-to-end progress**: PC produces slot outputs continuously; AptosBFT periodically picks the latest output and finalizes.

### Censorship resistance

- **Validity inclusion**: `mcp({v_h^in}) ⪯ v_i^low` for every honest `i`. Any group bundle that all honest validators see appears in every honest `v_low`, hence in every BFT-finalized `v_high`.
- **f-censorship resistance at the group level**: after GST, at most f BFT-finalized slots can omit a given honest group's bundle, where f bounds the number of Byzantine *groups*. Each Byzantine group leader can force at most one censored slot before its group is demoted to the end of the ranking.
- This is a *group*-level guarantee, which is the right unit for the realistic adversary: geographic / organizational clusters are the actual fault-correlated entity. A single Byzantine validator inside an honest group does not break inclusion as long as the group's leader rotation eventually selects an honest member.

### What we do *not* claim

- Strong single-shot censorship resistance (Abraham et al.'s notion). We trade that for compatibility with leader-based BFT.
- Leaderless termination at the BFT layer. AptosBFT's existing leader rotation + reputation handle leader-targeted DoS.
- Per-validator (vs per-group) censorship resistance.
- Order fairness *inside* a slot — within a group bundle, transaction order is leader-chosen subject to the deterministic per-bundle sort.

## 10. Trade-offs vs paper-faithful Strong PC

| Aspect | Paper (Strong PC + Multi-slot, Algorithms 3 + 4) | This design |
|---|---|---|
| `v_high` agreement | Leaderless, multi-view PC over certificate vectors | AptosBFT (single leader per round) |
| Per-slot latency (commit) | 4 rounds (2-round optimistic possible with `n ≥ 5f+1`) | PC's 3 rounds for eager `v_low`; BFT's 2 rounds for `v_high` finalization (overlapped with PC pipeline) |
| Throughput | 1 PC slot per ~3 rounds (sequential) | 1 PC slot per ~1 round (pipelined, slot `s+1` after `QC1_s`) |
| Censorship resistance unit | Per-validator (f-CR) | Per-group (G-CR with G ≪ n), with extra k-slot delay for ranking propagation |
| Disruption to existing stack | Replaces consensus + adds new dissemination | QS unchanged; BFT mostly unchanged; new PC layer plus eager pre-commit hook |
| Liveness assumption | Adversary can suspend one party per round | AptosBFT's standard partial sync + leader liveness |
| Ranking update trigger | PC's own `v_high` (agreed by SPC) | BFT-finalized `v_high_{s-k}` (k-delayed) |

## 11. Open questions

1. **Group composition**: static-by-region vs dynamic / weighted? Single-validator groups? Minimum group size for in-group leader rotation to provide value?
2. **Group leader rotation policy**: per-slot round-robin, reputation-based, or stake-weighted random within the group?
3. **PC pipeline depth `k`**: starting value (likely a few PC slots, matching expected BFT commit lag), and whether/how to back-pressure adaptively when BFT lags.
4. **PC instance synchronization details**: starter rule for slot `s+1` (initial choice: observing `QC1_s`); handling of late-arriving votes for past slots; cap on simultaneous in-flight slots per validator. Deferred.
5. **BFT block payload size cap `N`**: max number of `(v_high, π_high)` pairs per BFT block. Bounds bandwidth per block and recovery cost; should match expected PC pipeline depth.
6. **PoS vs opt-batch**: should `GroupProposal` carry only PoS digests (strong DA, slower) or include opt-batches (weaker DA, faster)? Mirrors today's OptQS vs strict-PoS trade-off.
7. **PC-vs-BFT lag**: how large a window of "eagerly pre-committed but not BFT-finalized" slots to allow before back-pressuring PC? Affects memory and recovery time.
8. **Empty `v_low`**: under heavy asynchrony, PC might produce empty `v_low`s for several consecutive slots. BFT keeps progressing with empty-payload blocks; ranking still advances (with k-delay) once BFT eventually finalizes the empty `v_high`s.
9. **Epoch boundaries**: PC slot numbering and ranking reset on reconfiguration; need a clean epoch-end fence so all in-flight PC slots of the old epoch are BFT-finalized before the new epoch starts.
10. **Asymptotic costs at scale**: PC is O(n²) messages and O(n³) QC sizes per slot in the paper's analysis; with n ≈ 100 this is non-trivial. The group abstraction (vector length G, not n) helps the vector dimension but not the voter count. Aggregate signatures will be needed in production.

## 12. Why this shape of design

The paper's leaderless construction is academically clean but expensive in practice: per-slot multi-view negotiation and a leaderless validated-consensus subroutine, both of which a production stack with a well-tuned leader-based BFT (AptosBFT + QS + decoupled pipeline) doesn't need. By plugging AptosBFT in for `v_high` agreement we get:

- The censorship-resistance benefits of PC (`v_low` includes everything every honest validator saw) at the *group* level — which is the realistic granularity for production-deployed adversary models.
- A latency *win* on the prefix: eager `v_low` commit means transactions in the prefix execute without waiting for any BFT round trip.
- Minimal disruption: QS unchanged, AptosBFT unchanged in protocol, new PC layer is additive, eager pre-commit reuses existing pipeline machinery.

The compromise vs paper-faithful Strong-PC: ranking updates depend on BFT liveness, and censorship resistance is per-group rather than per-validator. Both are deliberate, and both align with the production deployment target.
