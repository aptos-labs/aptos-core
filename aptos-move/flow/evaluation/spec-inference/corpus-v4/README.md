# Corpus V4

A public benchmark. Every target is either extracted from public `aptos-core`
code at a pinned commit or authored for the corpus, so the package, the
reference specifications, the mutants and every round built on them can be
published as an artifact. V3.2 measured the same kind of task on private Etna
code that cannot be redistributed.

[`REPRODUCE.md`](REPRODUCE.md) explains how to verify the corpus and a
published round, and how to rerun the experiment.

The selection criterion is the one V3.2 already stated: **specification
absence, not code privacy**. A model may know public code, but it cannot recall
a specification nobody wrote. No target has a specification upstream; the
trading code in `aptos-experimental` and `aptos-trading` carries exactly one
spec block, and that is not on a target.

## Targets

Twenty-six tasks. Five are carried over unchanged from V3.2 (`TR-order-010`,
`TR-discard-011`, `TR-cancel-026` from the public `bulk_order_utils`, and the
two authored targets `SM-select-022`, `QP-part-025`), keeping their ids. New
tasks continue V3.2's numbering from 028, so an id never names two targets.

| task | target | probes | guess |
|---|---|---|---|
| `TR-order-010` | `extracted_bulk_order_utils::validate_price_ordering` | adjacent-pair scan with an early return, strict both ways | hard |
| `TR-discard-011` | `extracted_bulk_order_utils::discard_price_crossing_levels` | least non-crossing index -- a prefix fact, not a fold | hard |
| `TR-cancel-026` | `extracted_bulk_order_utils::cancel_at_price_level` | removal from two coupled vectors; zero for absent and for an empty level | hard |
| `TR-sanitize-028` | `extracted_bulk_order_utils::new_bulk_order_request_with_sanitization` | **composition** -- eleven assertions, four through helpers that need exact contracts | hard |
| `TR-match-029` | `extracted_bulk_order_utils::match_order_and_get_next_from_bulk_order` | tuple of options; three aborts the source never states | hard |
| `TR-reinsert-030` | `extracted_bulk_order_utils::reinsert_order_into_bulk_order` | merge or prepend; unchecked addition, unguarded read | hard |
| `TR-nonzero-031` | `extracted_bulk_order_utils::validate_not_zero_sizes` | linear scan, one quantifier | guessable |
| `PT-taker-034` | `extracted_price_time_index::is_taker_order` | option-guarded inclusive comparisons | guessable |
| `DM-keepalive-035` | `extracted_dead_mans_switch_tracker::keep_alive` | map update reading the global clock; strict expiry | hard |
| `DM-valid-036` | `extracted_dead_mans_switch_tracker::is_order_valid` | strict session start, inclusive expiry, defaulted time | hard |
| `RL-refill-037` | `extracted_rate_limiter::refill` | token bucket; five unstated aborts, remainder carry | hard |
| `TL-tiers-038` | `extracted_transaction_limits::validate_tiers` | pairwise order, inclusive in one field and strict in the other | hard |
| `TL-build-039` | `extracted_transaction_limits::new_tiers` | **composition** of a per-element range check and the pairwise order | hard |
| `TL-find-040` | `extracted_transaction_limits::find_min_stake_required` | search through the inline `vector::find` | hard |
| `EA-scheme-041` | `extracted_ethereum_derivable_account::validate_scheme` | a contract that is only an abort condition over a character class | hard |
| `VR-range-042` | `extracted_vector_range::range_with_step` | overflow of an increment whose value is never used | hard |
| `RL-request-043` | `extracted_rate_limiter::request` | **composition** over `refill` | hard |
| `TL-enough-044` | `extracted_transaction_limits::validate_enough_stake` | **composition** over a global configuration, only aborts | hard |
| `SU-split-045` | `extracted_sui_derivable_account::split_signature_bytes` | two loops sharing one index | hard |
| `MS-pending-046` | `extracted_multisig_account::get_pending_transactions` | loop over a table range; overflow and missing-entry aborts | hard |
| `MS-timelock-047` | `extracted_multisig_account::can_execute_with_timelock` | two resources, a table and the clock; aborts the threshold does not avoid | hard |
| `MS-capacity-048` | `extracted_multisig_account::available_transaction_queue_capacity` | clamp over a subtraction that can underflow | guessable |
| `JW-upsert-049` | `extracted_jwks::upsert_provider_jwks` | stop-and-insert scan under an uninterpreted comparator | hard |
| `TL-update-050` | `extracted_transaction_limits::update_config` | **composition** with a signer check and a create-or-replace global write | hard |
| `SM-select-022` | `selection_machine::select` | **function values** | hard |
| `QP-part-025` | `lomuto_partition::partition` | **in-place permutation** | hard |

Families: `TR` `bulk_order_utils`, `PT` `price_time_index`, `DM`
`dead_mans_switch_tracker` (all trading code); `RL` `rate_limiter`, `TL`
`transaction_limits`, `EA` `ethereum_derivable_account`, `SU`
`sui_derivable_account`, `MS` `multisig_account`, `JW` `jwks` (framework); `VR`
`vector` (standard library); `SM`, `QP` authored.

Compared with V3.2 the corpus adds what its private pool structurally lacked:
global state (the clock in `DM-*`, `RL-*` and `MS-timelock-047`, account
resources in `MS-*`, a configuration read by `TL-enough-044` and written by
`TL-update-050`), an intrinsic map
(`DM-*`), tables (`MS-*`), inline higher-order iteration (`TL-find-040`), and
more composition targets.

### Excluded targets

Code built on `vector::fold` is kept out. `fold`'s lambda both writes its
captured accumulator and forwards to the user's function, and the prover's
`folds_of` derivation cannot split that effect across the forwarding
([#20383](https://github.com/aptos-labs/aptos-core/issues/20383)), so no
complete contract for such a target can be proved. Id 032, its one
candidate, is unused for that reason.

Id 033 was `price_time_index::get_slippage_price`, whose result is an
`Option<u64>` computed by arithmetic. Spec arithmetic is typed `num`, so the
natural contract `result == option::spec_some(mid + slippage)` compares an
`Option<u64>` with an `Option<num>`. The type checker accepts that, but the
prover's Boogie translation does not
([#20672](https://github.com/aptos-labs/aptos-core/issues/20672)), and Flow
reports the failure as infrastructure, which invalidated a pilot cell. The
target returns when that is fixed. `TR-match-029` also returns options, but of
values read from vectors, which keep their `u64` type.

## Extraction

`build.py` generates `package/sources/` from aptos-core at the commit pinned in
the manifest, reading each file with `git show`, so neither a checkout on
another branch nor local edits change what is built. The package declares no
dependencies: the standard library, `math64`, the comparator, the two ordered
maps and the tables they use are vendored whole, and `timestamp`, `event` and
`system_addresses` are reduced to the functions the targets call, with their
upstream specifications.

Bodies are copied byte for byte. Each module header states what was left out
and every change, and the changes are of three kinds only:

- **Lifted vectors.** Upstream, the `bulk_order_utils` functions that edit one
  side of a bulk order reach its price and size vectors through an accessor
  returning a pair of mutable references. Here the two vectors are parameters.
- **Reduced carriers.** A struct is reduced to the fields a target reads: the
  matched order passed to `reinsert_order_into_bulk_order`, and the price index
  of `get_slippage_price` and `is_taker_order`, which upstream holds two
  ordered maps and is observed only through its best bid and ask.
- **Reduced modules.** `timestamp`, `event` and `system_addresses` keep only
  what the targets call.

Headers describe provenance only. They do not say what a target's contract is
or where it is hard; that is what the task asks for.

## References

A reference is the package with one module's complete specification written in.
As in V3.2 only the specification is committed, as an add-only patch under
[`references/`](references/), checked by `build_references.py` to add
specification and no pragma beyond `opaque`.

```text
python3 corpus-v4/build.py                 # generate package/sources from the pinned commit
python3 corpus-v4/build.py --verify        # regenerate in memory and compare with the manifest
python3 corpus-v4/build_references.py      # assemble references/build/
python3 corpus-v4/compose.py               # per-task view into corpus-v4/inspect/
```

## Mutants

Two disjoint sets of three mutants per task, as in V3.2: a refutation set
([`mutants/`](mutants/)) shown to the agent as obligation categories, and a
held-out scoring set ([`mutants-scoring/`](mutants-scoring/)). Readable
descriptions are in [`mutant-specs/`](mutant-specs/); `author_mutants.py`
anchors them. The five carried-over tasks keep V3.2's mutants, re-anchored to
the V4 sources.

```text
python3 corpus-v4/author_mutants.py --role refutation --spec corpus-v4/mutant-specs/refutation.json \
  --package corpus-v4/package --out corpus-v4/mutants
python3 corpus-v4/author_mutants.py --role scoring --spec corpus-v4/mutant-specs/scoring.json \
  --package corpus-v4/package --out corpus-v4/mutants-scoring --disjoint-from corpus-v4/mutants
python3 -m harness.validate_mutants --config config/default.json \
  --reference corpus-v4/references/build/MODULE --baseline corpus-v4/package \
  --target TARGET --mutants corpus-v4/mutants/TASK/mutants.json --timeout 20
```

Every mutant of the ready tasks is essential: the reference of its
module kills it, after the reference itself has proved, passed the
inconsistency check, and been confirmed to carry the corpus implementation
unchanged. A mutant must rewrite the target's own body. One placed in an opaque
callee, or in a function the reference specification calls, is not observed
and survives; three first drafts were replaced for that reason.

## Screening and round selection

`harness.screen_v3 --all-ready` admits all twenty-six ready tasks (records in
[`screening/`](screening/)): each is well-formed and its reference proves
within the 20-second threshold, in under 11 seconds of wall time per task.
Nineteen are `wp_hard`. For thirteen, unaided WP stops at a loop without an
invariant. For five, it stops at a helper with a loop or global memory access
and no contract yet, so the hybrid arms have to specify the helper before the
target, the composition step V3.2 asked of `VS-redeem-004`. In `TL-find-040`,
WP's output carries clauses it flags as untrusted.

The round runs all twenty-six: `select_round.py --size 26 --max-guessable 3
--keep-redundant` selects every ready task, covering all 32 feature strata,
fourteen tasks with loops, twelve modules and three guessable controls. The
selection still reports the one near-duplicate pair, `TR-nonzero-031` and
`TR-cancel-026` (source similarity 0.55), and the families are uneven --
seven `TR` and five `TL` tasks -- so a round reports per task as well as
pooled.

```text
python3 -m harness.screen_v3 --manifest corpus-v4/manifest.json \
  --experiment-config config/default.json --corpus-config config/corpus.json \
  --results-dir corpus-v4/screening --output corpus-v4/screening/summary.json --all-ready
python3 corpus-v4/select_round.py --size 26 --max-guessable 3 --keep-redundant --write
```

## Prover requirements

`VR-range-042` needs `old(p)` of a value parameter `p` in a loop invariant: the
loop advances the parameter, and only its entry value relates the elements
pushed so far. Inline properties now accept it; a function-level `old(p)` is
still rejected, since there a parameter already denotes its entry value. The
screen and the references must run with a `move-flow` that includes this, and
so must any round that includes `VR-range-042`.
