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
| 2 | PC ↔ BFT cadence | Continuous parallel pipeline. PC slots run back-to-back, independent of BFT rounds. AptosBFT consumes the queue of PC outputs at its own pace. |
| 3 | `v_low` commit semantics | Eager. As soon as a validator's PC slot outputs `v_low`, it is fed into the execution pipeline. AptosBFT later confirms a `v_high` that extends it. |
| 4 | What does BFT leader propose? | `(v_high, π_high)` from its local PC output. Validators vote iff `F^high(v_high, π_high)` verifies AND `v_high` extends their own `v_low`. |

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

PC slots run continuously: slot `s+1` starts as soon as slot `s`'s `v_low` is locally produced and the new ranking is fixed.

## 5. Per-slot protocol flow

For a slot `s` at validator `i`:

1. Wait until either (a) `i` has received a `GroupProposal` for every group `g ∈ ranking_s`, or (b) a slot timer expires.
2. Build `v_in_s = [H(GroupProposal_{p_1,s}), …, H(GroupProposal_{p_G,s})]` using `H(⊥)` for missing entries.
3. Run paper's Algorithm 1 on `v_in_s`: broadcast vote-1 → collect QC1 → broadcast vote-2 → collect QC2 → broadcast vote-3 → collect QC3.
4. Compute `(v_low_s, v_high_s)`.
5. **Eager commit `v_low_s`** (§6).
6. Update local ranking (§7) and start slot `s+1`.

In parallel, the AptosBFT layer is reading from the queue of `(v_high_s, π_high_s)` outputs and finalizing them (§8).

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

Per paper §6.1.1 / §7.2, with adaptations:

- Initial ranking `ranking_1` is fixed at epoch start (e.g., region-alphabetic).
- After slot `s` is finalized by BFT (not just locally PC-output), the ranking is updated:
  - If `|v_high_s| = G` (full): keep ranking unchanged.
  - If `|v_high_s| = ℓ < G`: demote group `p_{ℓ+1}` (first excluded) to the end.
- All PC slots between two consecutive BFT-finalized slots use the same ranking. This is the **key coupling point** between PC and BFT for our hybrid: ranking advances in lockstep with BFT-finalized slots, not with PC-local outputs.

Why ranking must come from BFT-finalized `v_high`, not local PC `v_high`:
- PC alone gives consistency among `v_high`s but not agreement (Strong PC adds agreement). Validators may briefly hold different `v_high`s.
- If each validator updated ranking from its local `v_high`, they could disagree on the next slot's `ranking_{s+1}` and therefore on the next slot's `v_in` ordering. Vector-entry hashes diverge, no slot-`s+1` QC can form, liveness breaks.
- Using BFT-finalized `v_high` (which is unique by AptosBFT's safety) keeps every honest validator on the same ranking.

The cost: a Byzantine group leader is demoted only after BFT finalizes a slot omitting it, not the moment local PC outputs an empty entry. This bounds the demotion delay by BFT round-trip time, not PC round-trip time. Acceptable for the production target.

## 8. AptosBFT integration (the `v_high` agreement layer)

Block payload becomes `(v_high, π_high, slot_id, ranking)` from a specific PC slot.

**BFT leader behavior**: pick the latest PC slot `s*` for which the leader has a verifiable `(v_high_{s*}, π_high_{s*})` and propose it.

**Voting rule** (extension of today's safety rules): a validator votes on `(v_high, π_high, slot_id)` iff
1. All existing safety-rules checks pass (`last_voted_round`, `preferred_round`, …).
2. `F^high(v_high, π_high)` verifies — `π_high` is a valid PC slot-`slot_id` `QC3` and `v_high` matches `QC3Certify(π_high).x_pe`.
3. `v_high` extends this validator's locally-output `v_low_{t}` for some `t ≤ slot_id` for which it has eagerly pre-committed.
4. `slot_id > last_finalized_slot_id` (monotone progress).

If a leader's proposed `v_high` does not extend a validator's local `v_low`, the validator simply doesn't vote — BFT pacemaker moves on. Upper Bound guarantees this never happens for an honest leader; for a Byzantine leader, the worst case is one timed-out BFT round.

**Pipeline reconciliation**: when AptosBFT commits a block whose payload is `(v_high_{s*}, π_high_{s*})`:
- Eagerly pre-committed slots up through `s*` are already executed; they become final.
- The "tail" of `v_high_{s*}` beyond local `v_low` is now also confirmed; the executor extends the local pre-committed state with these additional transactions and finalizes everything.

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
| Per-slot latency (commit) | 4 rounds (2-round optimistic possible with `n ≥ 5f+1`) | PC's 3 rounds for eager `v_low`; BFT's 2 rounds for `v_high` finalization (overlapped with next PC slot) |
| Censorship resistance unit | Per-validator (f-CR) | Per-group (G-CR with G ≪ n) |
| Disruption to existing stack | Replaces consensus + adds new dissemination | QS unchanged; BFT mostly unchanged; new PC layer plus eager pre-commit hook |
| Liveness assumption | Adversary can suspend one party per round | AptosBFT's standard partial sync + leader liveness |
| Ranking update trigger | PC's own `v_high` (agreed by SPC) | BFT-finalized `v_high` |

## 11. Open questions

1. **Group composition**: static-by-region vs dynamic / weighted? Single-validator groups? Minimum group size for in-group leader rotation to provide value?
2. **Group leader rotation policy**: per-slot round-robin, reputation-based, or stake-weighted random within the group?
3. **PC slot ↔ BFT block mapping**: 1:1, or does one BFT block finalize a *range* of PC slots? Range is more bandwidth-efficient (fewer BFT rounds per unit progress) but enlarges block payloads and proof sizes.
4. **PoS vs opt-batch**: should `GroupProposal` carry only PoS digests (strong DA, slower) or include opt-batches (weaker DA, faster)? Mirrors today's OptQS vs strict-PoS trade-off.
5. **PC-vs-BFT lag**: how large a window of "eagerly pre-committed but not BFT-finalized" slots to allow before back-pressuring PC? Affects memory and recovery time.
6. **Empty `v_low`**: under heavy asynchrony, PC might produce empty `v_low`s for several consecutive slots. We still need BFT progress in this case to advance ranking and unblock demotion. Confirm AptosBFT can finalize an empty-`v_low` `v_high` (the natural answer is yes — `v_high = v_low = []` is a valid degenerate output).
7. **Epoch boundaries**: PC slot numbering and ranking reset on reconfiguration; need a clean epoch-end fence so the last slot of an epoch is BFT-finalized before the new epoch starts.
8. **Asymptotic costs at scale**: PC is O(n²) messages and O(n³) QC sizes per slot in the paper's analysis; with n ≈ 100 this is non-trivial. The group abstraction (vector length G, not n) helps the vector dimension but not the voter count. Aggregate signatures will be needed in production.

## 12. Why this shape of design

The paper's leaderless construction is academically clean but expensive in practice: per-slot multi-view negotiation and a leaderless validated-consensus subroutine, both of which a production stack with a well-tuned leader-based BFT (AptosBFT + QS + decoupled pipeline) doesn't need. By plugging AptosBFT in for `v_high` agreement we get:

- The censorship-resistance benefits of PC (`v_low` includes everything every honest validator saw) at the *group* level — which is the realistic granularity for production-deployed adversary models.
- A latency *win* on the prefix: eager `v_low` commit means transactions in the prefix execute without waiting for any BFT round trip.
- Minimal disruption: QS unchanged, AptosBFT unchanged in protocol, new PC layer is additive, eager pre-commit reuses existing pipeline machinery.

The compromise vs paper-faithful Strong-PC: ranking updates depend on BFT liveness, and censorship resistance is per-group rather than per-validator. Both are deliberate, and both align with the production deployment target.
