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

Nothing. The corpus, its references and its mutants are complete and
reproducible; what remains is a decision about round size rather than work on
the corpus.

Two things are deliberately not on this list. `PN-pnl-005` is permanently
excluded rather than deferred — its blocker is a prover defect, not a property
of the target, so it returns only if that is fixed. And
`harness/review_stage.py` is V1 tooling that V3 does not need: a V3 reference is
a committed specification patch and its mutants are authored directly.
