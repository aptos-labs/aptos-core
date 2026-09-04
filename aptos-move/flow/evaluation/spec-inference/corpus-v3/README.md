# Corpus V3

> **This is the benchmark.** The full run is planned on this corpus.
> [V1](../corpus-v1/README.md) is retained as infrastructure, not as a run
> target. See [`DESIGN.md`](../DESIGN.md) for the architecture.

Twenty-three targets: mostly from Etna, the codename for Decibel's private Move
code, plus two public `aptos-experimental` functions that qualify because they
carry no upstream specification, and two authored here. **No Etna source is
committed** — see [Reproducing the package](#reproducing-the-package).

V3 replaced V2 because V2 saturated: its targets were small enough to hold in
your head, so an agent had no reason to reach for a tool, and it scored only
whether a specification verifies — which a vaguer contract passes more easily.
V3 selects against both: targets that resist guessing, and mutants so contract
strength is measurable.

## Targets

A task id is `FAMILY-tag-NNN`: a two-letter code for the module the target
comes from, a short tag distinguishing targets within it, and a corpus-wide
ordinal. `LP-price-021` is `get_liquidation_price` in
`extracted_liquidation_price`, the twenty-first task. The ordinals run 001–025
without gaps and were assigned module by module as the corpus was assembled, so
adjacent numbers share a family — `VS-fees-001` through `VS-redeem-004` are the
four `vault_share_math` targets. The number carries no other meaning: it is not
a difficulty, a rank, or an ordering the round respects. Ids are stable and
appear in every schedule, artifact and metadata file, so they are not renumbered
when a task is held back or excluded.

Every target proves in about a second once specified, which keeps mutation
scoring affordable — it re-proves each target once per mutant. Nineteen are
`hard`; four are kept `guessable` as controls that tell an apparatus failure
apart from a genuinely difficult task.

| task | target | probes | guess |
|---|---|---|---|
| `VS-fees-001` | `vault_share_math::calculate_unrealized_fees` | five guarded returns, a cross-multiplied `u128` ratio test | hard |
| `VS-shares-002` | `vault_share_math::convert_existing_shares_to_asset_amount` | the composition partner, deliberately easy | guessable |
| `VS-contrib-003` | `vault_share_math::convert_new_assets_to_share_count` | tuple result, zero-shares fast path, insolvency assert | hard |
| `VS-redeem-004` | `vault_share_math::calculate_redemption_funds_and_fee` | **composition** — its underflow bound comes only from a callee's contract | hard |
| `OV-order-006` | `extracted_order_validation::validate_order_input` | why it never aborts: `&&` short-circuits past the `%` guards | guessable |
| `UC-credits-008` | `extracted_tier_lookup::credits_for_duration_days` | last-match-wins scan; the "first match" reading is a different function | hard |
| `UC-leverage-009` | `extracted_tier_lookup::leverage_for_tier_rank` | the same scan over an exact rank predicate | hard |
| `TR-order-010` | `extracted_bulk_order_utils::validate_price_ordering` | adjacent-pair scan with an early return, strict in both directions | hard |
| `TR-discard-011` | `extracted_bulk_order_utils::discard_price_crossing_levels` | least non-crossing index — a prefix fact, not a fold | hard |
| `BA-base-012` | `extracted_base_math::compute_base_needed` | **needs an invented lemma** — see below | hard |
| `MM-min-013` | `extracted_minmax::find_min_value` | reads `values[0]` first, so it aborts on empty | hard |
| `MM-max-014` | `extracted_minmax::find_max_value` | the same loop from zero, so it returns on empty — the **paired probe** | hard |
| `MD-median-015` | `extracted_median::get_median_price` | a total function: no loop, no arithmetic, no abort | guessable |
| `BK-bucket-016` | `extracted_bucket_index::get_bucket_index` | early `return`, so the result is the *least* admitting index | hard |
| `TF-taker-017` | `extracted_taker_fee::calculate_min_net_taker_fee` | `100 - pct` underflows above 100 — an abort the source never mentions | hard |
| `DV-dev-018` | `extracted_deviation::calculate_deviation_bps` | a sentinel where an abort is expected | hard |
| `TS-trial-019` | `extracted_trial_size::trial_size_for` | three unstated aborts behind the one range check it does state | hard |
| `TL-lev-020` | `extracted_tier_leverage::checked_max_tier_leverage` | quantified abort and accumulated maximum; neither states the other | hard |
| `LP-price-021` | `extracted_liquidation_price::get_liquidation_price` | zero leverage means a zero divisor, invisible in the arithmetic | hard |
| `SM-select-022` | `selection_machine::select` | **function values** — see below | hard |
| `WU-consume-023` | `extracted_work_units::consume_order_match_work_units` | `&mut` saturating transition; `u32` overflows twice over | hard |
| `WU-limit-024` | `extracted_work_units::get_max_order_placement_limit` | clamped division with a floor of one | guessable |
| `QP-part-025` | `lomuto_partition::partition` | **in-place permutation** — see below | hard |

Four targets carry a capability the rest do not.

`VS-redeem-004` is the composition case: it calls two siblings, and its freedom
from underflow at `shares - shares_for_fee` holds only because the fee is
bounded by the NAV — established in the *callee's* contract and invisible in its
own body.

`BA-base-012` is the auxiliary-reasoning case. An invariant alone cannot state
its contract: naming the accumulated value needs a recursive spec function, and
ruling out a `u128` overflow of the accumulator needs monotonicity of a running
sum, supplied as a lemma proved by recursive `apply`. Its accumulation is linear
on purpose — the reasoning is the difficulty, not the solver time.

`SM-select-022` is the function-value case, and the only target not extracted
from Etna, which has no function-valued code. A bounded selection loop draws
candidates by applying a continuation, accepts the first admissible one, and
after so many failures hands back the position to restart from. Both the
continuation and the test arrive as parameters, so the contract needs
`result_of` and `aborts_of` over them, and the state after `k` draws needs a
recursive specification function over a function value. Because it is ours it
contains no proprietary source.

`QP-part-025` is the quicksort case: Lomuto partition over a `u64` vector, the
only target that rearranges its input in place, and also authored here because
Etna has no sorting code. Where the pivot lands and what lies on each side of
it are ordinary loop-invariant work, with the pivot followed through two swaps.
Saying that the result is a rearrangement and not a rewrite is the difficulty:
the direct `forall`/`exists` containment, a bare recursive count, and even a sum
proxy all exhaust the solver. The reference states it with a counting
specification function under a `[weight = 20]`, a swap lemma proved by
segments, and an entry-point `forall … apply` whose trigger is the double
`update` the prover encodes `vector::swap` as. With that shape every proof and
every refutation takes about a second.

### Blocked

## What `wp_hard` means

`wp_hard` records that WP alone does not reach a verifying contract. It is a
property of the task, not a defect: a target WP already solves would keep the
easy member of every family.

Reading it needs one piece of context. When a loop has no invariant, the havoc
leaves part of the inferred condition unconstrained, so WP drops those clauses
and emits an empty contract carrying `aborts_if_is_partial`. That contract
compiles and verifies. Outside this study the accompanying diagnostic is a
warning, which is right: a person can still use what WP derived. For anything
that only asks whether the prover succeeded it is not, because an empty
contract is indistinguishable from a complete one -- and every loop target was
consequently recorded as `wp_hard: false`, labelling the corpus's hardest
targets as the ones WP handled.

`ProverOptions::uninvariant_loop_is_error` makes that diagnostic an error
instead. `move-flow experiment infer` sets it always, since screening is the
consumer that must not miss it, and an evaluation session sets it through
`EvaluationConfig::uninvariant_loop_is_error`, so an arm cannot mistake an
empty contract for a finished one either. Ordinary Flow use is unchanged.

## A WP artefact on three targets

Unaided WP inference emits an unprovable `sathard` clause on `VS-shares-002`,
`LP-price-021` and `TS-trial-019`: a normal-return `ensures` for a path that
aborts, duplicating an `aborts_if` beside it. See
[#20490](https://github.com/aptos-labs/aptos-core/issues/20490) for the
mechanism and for why the candidate fix was not taken.

What it means here:

- It is not solver difficulty, despite `sathard`, and not a loop -- all three
  targets are loop-free.
- It is not a broken task. All three prove against their references, so
  admission is decided on well-formedness and a verifying reference, with
  WP-hardness recorded as a task property.
- Both hybrid arms receive it, and deleting a `sathard` clause to make a proof
  pass is forbidden, so sessions on these three should be read with that in
  mind.

`VS-redeem-004` also carries `sathard` clauses and is not an instance: its
clauses carry `result_of` over sibling calls, the composition difficulty
described above.

## Provenance

Etna is exported from a pinned commit, but the vendored standard library and
the trading targets are read from the live aptos-core working tree. `build.py`
therefore refuses to regenerate from a tree with tracked modifications: the
corpus would enter under a commit that does not contain them, and the recorded
hashes would describe a source nobody could reconstruct. `--allow-dirty`
overrides it and records `reconstructible: false` in the manifest, so the claim
is never silently false.

The committed manifest carries `tracked_modifications: true` from before that
check existed, so **the current generated sources are not reconstructible from
the recorded commit**. They verify against the manifest, which fixes what they
are; it does not establish where they came from. Regenerating from a clean
checkout is outstanding.

## Round selection

A full round costs one session per task, arm and replicate, so a round may run
a subset of the ready targets. `select_round.py` chooses it from the corpus's
own description of each task and its target source -- never from any arm's
behaviour -- and records the result as `round_selection` on every manifest
record and in `metadata/selection.json`. Nothing is removed: a held-back sample
stays in the corpus for a later round.

The current selection is 16 of the 23 ready targets. It keeps every task that
uniquely carries a feature stratum, one representative of each redundancy
cluster, and at most 2 guessable targets; all 32 strata survive, across
12 modules, 8 of them with loops. The two near-duplicate target pairs the
audit found are `MM-max-014`/`MM-min-013` (0.895 source similarity) and
`UC-credits-008`/`UC-leverage-009` (0.555); `DV-dev-018`, `LP-price-021`,
`TF-taker-017` and `WU-limit-024` carry identical strata, so the largest is
kept and the rest held back.

Not scheduled; the scheduler drops any target whose `screening_status` is not
`ready`, and naming one explicitly is an error rather than an override.

| task | target | why |
|---|---|---|
| `PN-pnl-005` | `extracted_pnl_math::calculate_pnl` | a signed-division disagreement between the specification and the code, reduced in [`blockers/`](blockers/) |
| `QT-quote-007` | `extracted_quote_math::compute_quote_needed` | same lemma requirement as `BA-base-012`, but its per-level `p * s / m` also makes the proof nonlinear and it does not discharge at any budget tried up to 240s |

`QT-quote-007` is kept rather than deleted because it is the evidence for
preferring linear accumulation when selecting a lemma target.

## Mutants

A mutant patches the **implementation**, never the specification. The harness
applies it to a copy of the package and re-runs the prover against the agent's
finished specification: prover fails → killed, the contract was precise enough
to notice; prover succeeds → survived, it verifies against wrong code.

Three per target, four for `QP-part-025`, 70 in all, one per obligation the contract must pin. Abort *codes* are out of scope: a mutant that
changes only which code an abort carries tests error-code pinning rather than
contract strength.

A mutant stores an offset, a length and a SHA-256 into the generated file rather
than the code it rewrites, plus a minimal edit. Mutants and references are
authored **before** a round and without seeing any arm's output.

### Two disjoint sets: refutation and scoring

Refutation feeds surviving mutants back to the agent as a failure, which turns
the mutant set into training material. Scoring an arm on the same set would be
scoring it on what it was told, so there are two sets and they never overlap:

| set | path | role |
|---|---|---|
| refutation | [`mutants/`](mutants/) | shown to the agent, as *categories* only — the extra life |
| scoring | [`mutants-scoring/`](mutants-scoring/) | held out; the strict-success score |

`harness.controller` refuses a run whose two roots resolve equal, and
`author_mutants.py --disjoint-from` refuses a scoring mutant that repeats a
refutation mutant's file, offset and edit. Pass the scoring root at
**schedule** time (`harness.pilot --mutants-root`) so its digest is part of the
recorded apparatus identity, and the refutation root at **launch** time
(`harness.pilot_run --refutation-mutants-root`). Without `--mutants-root` a
round runs `scoring_mode: core`, where `strict_success` is false by
construction and says nothing about the contract.

The scoring set is authored from readable edit descriptions in
[`mutant-specs/scoring.json`](mutant-specs/scoring.json); `author_mutants.py`
computes the offsets and digests and rejects an anchor that does not occur
exactly once in its file.

Two first drafts were dropped for the reason stated above -- abort codes are out
of scope. Both weakened a guard from `x > 0` to `x >= 0`; the zero then reached
a division and the function still aborted, so only the abort *code* changed and
a complete contract could not observe the difference. `validate_mutants`
recorded them `survived`, and they were replaced by observable edits.

## References

A mutant is `essential` only once a hand-authored reference specification kills
it. References are per *module* — the twenty-three targets occupy sixteen — and only the
specification is committed, as a patch under [`references/`](references/) that
adds lines and never removes one.

| module | tasks | mutants killed |
|---|---:|---|
| [`vault_share_math`](references/vault_share_math.patch) | 4 | 12/12 |
| [`extracted_bulk_order_utils`](references/extracted_bulk_order_utils.patch) | 2 | 6/6 |
| [`extracted_minmax`](references/extracted_minmax.patch) | 2 | 6/6 |
| [`extracted_tier_lookup`](references/extracted_tier_lookup.patch) | 2 | 6/6 |
| [`extracted_work_units`](references/extracted_work_units.patch) | 2 | 6/6 |
| [`extracted_base_math`](references/extracted_base_math.patch) | 1 | 3/3 |
| [`extracted_bucket_index`](references/extracted_bucket_index.patch) | 1 | 3/3 |
| [`extracted_deviation`](references/extracted_deviation.patch) | 1 | 3/3 |
| [`extracted_liquidation_price`](references/extracted_liquidation_price.patch) | 1 | 3/3 |
| [`extracted_median`](references/extracted_median.patch) | 1 | 3/3 |
| [`extracted_order_validation`](references/extracted_order_validation.patch) | 1 | 3/3 |
| [`extracted_taker_fee`](references/extracted_taker_fee.patch) | 1 | 3/3 |
| [`extracted_tier_leverage`](references/extracted_tier_leverage.patch) | 1 | 3/3 |
| [`extracted_trial_size`](references/extracted_trial_size.patch) | 1 | 3/3 |
| [`selection_machine`](references/selection_machine.patch) | 1 | 3/3 |
| [`lomuto_partition`](references/lomuto_partition.patch) | 1 | 4/4 |

```text
python3 corpus-v3/build_references.py --verify
.venv/bin/python -m harness.validate_mutants --config config/default.json \
  --reference corpus-v3/references/build/MODULE --baseline corpus-v3/package \
  --target TARGET --mutants corpus-v3/mutants/TASK/mutants.json --timeout 40
```

Adding a module to the package changes every reference package's tree hash, so
`--verify` will flag the others and their tasks need re-validating.

## Reproducing the package

**The Move sources are not committed.** `aptos-core` is public and Etna is not,
so everything generated from it — `package/sources/` and `references/build/` —
is gitignored, and only recipes, specifications, digests and anchors are
tracked.

| | |
|---|---|
| repository | `https://github.com/aptos-labs/etna.git` (private; access required) |
| commit | `dd23678f980266360e050037fb78317b13753068` |

```text
python3 corpus-v3/build.py                 # clones into corpus-v3/.etna
python3 corpus-v3/build.py --etna PATH     # read the pin from an existing checkout
python3 corpus-v3/build.py --verify        # regenerate and compare, writing nothing
```

To read the corpus rather than run it, compose it per task into an untracked
directory: each task gets its module as the agent receives it, the same module
with the reference specification written in, and every mutant as a unified
diff.

```text
python3 corpus-v3/compose.py                  # into corpus-v3/inspect/
python3 corpus-v3/compose.py --task MM-min-013 --output DIR
```

`build.py` exports that commit's tree rather than reading a working directory,
so a dirty checkout cannot change what the corpus is built from. Extraction is
minimal: bodies are copied byte-for-byte, and only two things change, both
recorded in the module header — a carrier struct is reduced to the fields the
target reads, and a global config read becomes a parameter.

Any published artifact built from this corpus needs its own disclosure decision.
Contract shapes can be described without reproducing proprietary source; the
package itself cannot be redistributed without one.

## What is left before the full run

The corpus, its references and both mutant sets are complete and reproducible.
The refutation mechanism made two apparatus changes necessary, and both are in
place:

- A **held-out scoring set** ([`mutants-scoring/`](mutants-scoring/)), because
  refutation turns the set it shows the agent into training material. Schedule
  with `--mutants-root corpus-v3/mutants-scoring`; a round scheduled without it
  runs `scoring_mode: core`, where `strict_success` is false for an apparatus
  reason rather than a specification one.
- A **larger wall budget**. Refutation makes a third controller turn ordinary,
  and at `max_wall_seconds: 2700` a third of the `pilot-qp-ref3` cells were cut
  off mid-repair -- scored as failures, which would have read as refutation
  hurting the arm. `config/default.json` now allows 3600.

What remains is a decision about round size rather than work on the corpus.

Two things are deliberately not on this list. `PN-pnl-005` is permanently
excluded rather than deferred — its blocker is a prover defect, not a property
of the target, so it returns only if that is fixed. And
`harness/review_stage.py` is V1 tooling that V3 does not need: a V3 reference is
a committed specification patch and its mutants are authored directly.
