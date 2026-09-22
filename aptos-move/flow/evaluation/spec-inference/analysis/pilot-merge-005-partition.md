# Pilot `pilot-merge-005`: three arms on the partition task

A development round, one cell per arm, on `QP-part-025`
(`decibel_dex::lomuto_partition::partition`, quicksort's Lomuto partition over
a `u64` vector). The three arms reached the same shape of contract by three
different routes, which is what makes it worth keeping. It is a pilot: one
replicate, model `glm-5.3[1m]`, Claude Code 2.1.258, feedback level
`acceptance`. Nothing here is a main-arm result.

## Mutant set

Four mutants, each targeting a different correctness property. Mutants are
currently selected offline by a human who reads the reference specification and
picks changes that a weaker contract would miss. Dynamic mutant generation
(e.g. random operator substitution filtered by compilation) is a natural
extension.

| id | change | property killed |
|---|---|---|
| `QP-part-025-loose-split` | `< → <=` in the sweep comparison | strict ordering below the pivot |
| `QP-part-025-pivot-left-at-end` | delete the final swap | pivot lands at `result` |
| `QP-part-025-lost-element` | replace final swap with assignment | exact vector content (rearrangement) |
| `QP-part-025-tolerates-bad-pivot` | guard the parking swap so out-of-range pivot skips it | abort on `pivot >= len` |

## Outcome

Every cell was an operational success in a single controller turn, the audit
raised no issue, and every candidate killed all four mutants.

| arm | wall | model turns | in-session checks (in order) | out tok | cache read | cost |
|---|---:|---:|---|---:|---:|---:|
| agent-only | 21.7 min | 34 | timeout, failure, accepted, accepted | 70.6k | 1.44M | $2.97 |
| hybrid-guided | 15.6 min | 37 | accepted | 33.1k | 1.27M | $1.79 |
| hybrid-flexible | 9.9 min | 40 | accepted, failure, accepted, accepted | 33.8k | 1.28M | $1.77 |

Tool use differed more than the totals suggest. agent-only wrote the file
seven times and spent one `verify` and four `move_spec_check` calls finding
its shape. hybrid-guided ran WP twice, read widely (15 `Read`, 10 `Grep`),
wrote twice and needed exactly one check. hybrid-flexible ran WP twice,
edited five times and checked four times. Neither hybrid arm called `verify`
at all; the candidate check was their prover.

## What they wrote

All three contracts state the aborts exactly (empty vector, out-of-range
pivot), length preservation, the pivot's landing index, the strict prefix and
the at-or-above suffix. All three then pin the **exact output vector** with a
recursive specification function that replays one loop iteration per
unfolding (`part_at`, `partitioned`), plus a recursive count for the result
index. None states the reference's permutation property (`count(values, x)
== count(old(values), x)` for all `x`); two of them argue explicitly that an
exact replay is the better contract.

- agent-only: `values == swapped(sweep(swapped(old, pivot, last), …), result, last)`,
  `result == len(smalls_at(…))`, twelve loop invariants, `[weight = 20]` on
  the replay helper. 135 lines.
- hybrid-guided: `values == partitioned(old, pivot)`, `result == split_at(old, pivot)`
  over the *parked* vector, seven helpers, no weight. 117 lines.
- hybrid-flexible: `values == partitioned(old, pivot)`, `result ==
  count_below(old, len, old[pivot])` over the *original* vector with a
  correction term in the invariant, `[weight = 20]`, loop-exit assertions.
  113 lines.

The reference specification takes the other road: a `count` function under
`[weight = 20]`, a segment lemma proved by induction, a swap lemma, and an
entry-point `forall … apply` whose trigger is the double `update` the prover
encodes `vector::swap` as. It proves in about a second and kills the same
four mutants.

## What the agents said

Their final reports, verbatim.

### agent-only

> **Result:** `decibel_dex::lomuto_partition::partition` now carries a complete `pragma opaque` contract, accepted by the candidate check (all obligations verified, implementation unchanged, contract coverage complete). The contract states: aborts iff the vector is empty (`length - 1` underflow) or `pivot` is out of range (first `swap`); length preserved; `result < len`; `values[result] == old(values)[pivot]`; everything below `result` strictly below the pivot value and everything after at or above it; `result == len(smalls_at(...))` with `[0, result)` replaying that subsequence in order; and the exact final vector `values == part_at(old(values), pivot)`. Twelve loop invariants carry the proof: size/bounds, parked pivot, order-preserving small prefix (`smalls_at` recursion aligned with the loop index), untouched suffix, pointwise split, and a forward-simulation invariant `part_from(current) == part_from(entry)` (with `[weight = 20]` to stop runaway unrolling).
>
> **Strategy:** Direct WP derivation, checking candidates as the loop model was refined. A tiny probe module first established that the prover models `vector::swap` as `update(update(v,i,v[j]),j,v[i])` and that no induction is available (a `count`/multiset swap lemma — the standard permutation encoding — failed even range-guarded), which ruled out a count-based spec and forced recursion-aligned helpers.
>
> **Decision points:** (1) Dropped the order-preserving claim for the big partition after a counterexample showed each accepted small displaces the big region's *head* to the scan position (an interleaved rotation, not a stable partition); the exact content is instead pinned by `part_at`, a forward-simulating recursive spec function whose invariant is preserved by one definitional unfold per iteration. (2) Moved from `count`-based multiset equality (timed out at 227k defining-axiom instantiations) to pointwise/subsequence facts plus the simulation, after the timeout analysis named the recursive definition's quantifier as the cost driver.

### hybrid-guided

> **Result:** Complete opaque contract for `decibel_dex::lomuto_partition::partition`, accepted by `move_package_check_candidate` (`CANDIDATE_ACCEPTED`, implementation unchanged, coverage complete). Added: exact aborts (`len(values) == 0`, `pivot >= len(values)` — nothing else can abort, since the sweep stays inside `0..last` and `store` never passes `last`), length preservation, `result == split_at(old(values), pivot)`, the exact output vector `values == partitioned(old(values), pivot)`, and the pivot's landing spot (`values[result] == old(values)[pivot]`, all-strictly-below before it, all-at-or-above from it on). Backing this, a loop invariant holding prefix / unscanned suffix / parked pivot at once, plus helpers `swapped`, `parked`, `count_below`, `sweep_step`, `swept`, `split_at`, `partitioned`, all marked `[inferred]`.
>
> **Strategy:** Guided hybrid — `move_package_wp` reported the loop as uninvarianted (with nested-`update` bounded head facts) and `aborts_if_is_partial`; I replaced the stale WP block with a recursion-aligned `swept` helper whose single step is exactly one loop iteration, so invariant preservation and the exit obligations unfold definitionally, then reran WP (no warnings) and let the candidate check do the proving.
>
> **Decision points:** Characterizing `result` over the *parked* vector rather than the original — the first swap moves `old(values)[last]` into the swept range while `old(values)[pivot]` leaves it, so `count_below(parked(v, pivot), …)` is the correct count and the direct `count_below(old(values), …)` form is wrong. Second, keeping `store == count_below(…)` and `values == swept(…)` as separate invariants instead of relying on the solver to derive them from the structural prefix/suffix facts — those need induction at symbolic `i`, whereas the explicit recursion-aligned equations prove in one step.

### hybrid-flexible

> Final state confirmed accepted. The file is compiler-clean (only pre-existing dep `folds_of` warnings remain, untouched).
>
> **Result:** `decibel_dex::lomuto_partition::partition` now carries an opaque, exact contract, verified and accepted (`CANDIDATE_ACCEPTED`, all obligations, coverage complete, implementation unchanged): `aborts_if len(values) == 0 || pivot >= len(values)` (exhaustive — no other aborts in the sweep, swap bounds, or the increments); `result == count_below(old(values), len(old(values)), old(values)[pivot])`; `len(values)` unchanged; and the post-state vector pinned exactly as `values == partitioned(old(values), pivot)`. Five spec helpers (`swapped`, `parked`, `count_below`, `classified_below`, `sweep`, `partitioned`) plus eight loop invariants carry it: framing (`i <= last`, `store <= i`, length/`last`/`p` vs pre-state), the unscanned suffix still holding parked values, `store` as the count of already-classified positions (with the pivot's slot skipped and the last element standing in for it), and `sweep(values, i, store, last, p) == sweep(parked(old(values), pivot), 0, 0, last, p)`.
>
> **Strategy:** Hybrid — WP first named the uninvariant loop and emitted only a vacuous `aborts_if_is_partial` stub; the rest was direct invariant synthesis aligned one-to-one with the loop (the `sweep`/`count_below` recursions each unfold exactly one iteration per preservation obligation), so no induction tactic or swap-invariance lemma was needed. Native `vector` ops came from the prover's built-in model, so no callee contracts were required.
>
> **Decision points:** (1) Characterized `result` as a count over the *original* vector rather than over the post-park vector — the natural caller-facing fact — paying for it with the `classified_below` correction term in the invariant, which avoids the swap/multiset lemma that would have needed unbounded induction; the check verified it, and a deliberate false probe (`result == 0`) was rejected with a counterexample whose havocked loop state still satisfied the exact post-state, showing the invariants determine the outcome. (2) Pinned the mutated vector with a recursion-aligned `sweep` equation instead of the classic permutation-plus-partition-property form, because the latter does not determine the arrangement inside the two groups and so would not be an exact opaque promise.

## Observations

1. **Replay beats permutation, for every arm.** The skill's own guidance --
   align a recursive helper with the loop so each obligation unfolds one step
   -- is what all three followed, and it leads to an exact functional
   description of the algorithm rather than to a property of its result.
   hybrid-flexible defends this as the *correct* reading of "complete opaque
   contract": a permutation property does not fix the arrangement, an exact
   replay does. That is coherent, and it means the mutation score cannot
   distinguish a restated algorithm from a contract. It is a question for the
   study design, not for the scorer.
2. **The count-lemma road was tried and abandoned.** agent-only attempted the
   reference's encoding, hit the same 227k-instantiation timeout on the
   recursive definition, read the analysis correctly, and concluded that
   induction is unavailable -- without trying the `[weight = N]` and explicit
   trigger that make it prove. The skill it ran under documents both, and the
   same agent used `[weight = 20]` on its replay helper; it did not connect
   the annotation to the encoding it had given up on.
3. **WP shortened the search.** Both hybrid arms went from WP's "loop has no
   invariant, `aborts_if_is_partial`" straight to a recursion-aligned helper,
   at about 60% of agent-only's cost and with far fewer rewrites. agent-only
   spent its extra turns discovering the swap encoding and the rotation
   counterexample by probing.
4. **Two ways to count.** hybrid-guided counts below-pivot elements over the
   parked vector, hybrid-flexible over the original vector with a correction
   term; both are right and both prove. It is the one place the arms'
   contracts differ in content rather than in name.
