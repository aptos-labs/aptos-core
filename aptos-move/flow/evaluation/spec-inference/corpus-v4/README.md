# Corpus V4

A public benchmark. Every target is either extracted from public `aptos-core`
code at a pinned commit or authored for the corpus, so the package, the
reference specifications, the mutants and every round built on them can be
published as an artifact.

[`REPRODUCE.md`](REPRODUCE.md) explains how to verify the corpus and a
published round, and how to rerun the experiment.

The selection criterion is **specification absence, not code privacy**. A model
may know public code, but it cannot recall a specification nobody wrote. No
target has a specification upstream; the trading code in `aptos-experimental`
and `aptos-trading` carries exactly one spec block, and that is not on a target.

## Corpus at a glance

Twenty-six tasks from twelve modules. Each asks for the complete contract of
one target function: every task requires both a normal-result and an abort
condition.

| area | source | modules | tasks | with loops | without loops |
| --- | --- | --- | ---: | ---: | ---: |
| Trading | `aptos-experimental` | `bulk_order_utils`, `price_time_index`, `dead_mans_switch_tracker` | 10 | 4 | 6 |
| Accounts and authentication | `aptos-framework` | `multisig_account`, `ethereum_derivable_account`, `sui_derivable_account`, `jwks` | 6 | 4 | 2 |
| Rate and stake limits | `aptos-framework` | `rate_limiter`, `transaction_limits` | 7 | 2 | 5 |
| Standard library | `move-stdlib` | `vector` | 1 | 1 | 0 |
| Authored | this corpus | `selection_machine`, `lomuto_partition` | 2 | 2 | 0 |
| **Total** | | 12 modules | **26** | **13** | **13** |

This document names a task by its target function, which is unique in the
corpus. Data files and run directories key a task by an internal id; the
[Targets](#targets) table gives both.

A loop counts when the agent has to find its invariant.
`find_min_stake_required` searches with the inline `vector::find`, whose loop
carries its own invariants, and is counted without a loop. The program features
the targets exercise, counted per task (a task can have several):

| feature | tasks |
| --- | ---: |
| loop that needs an invariant | 13 |
| calls a helper whose contract the task tree provides | 10 |
| reads global state (4 of them also write it) | 9 |
| quantified contract | 7 |
| composition: the contract is built from callee contracts | 5 |
| table or intrinsic map | 4 |
| mutable reference or in-place update | 4 |
| function values or an inline higher-order function | 2 |
| permutation proved with lemmas | 1 |

Twenty-three targets are rated hard to guess and three serve as guessable
controls (`validate_not_zero_sizes`, `is_taker_order`,
`available_transaction_queue_capacity`).

`partition` is the one target whose complete contract needs lemmas: that the
partitioned vector is a permutation of the input is stated through element
counts, and the reference proves it with `count_swap`, a lemma that a swap
preserves every count, built on a second lemma, `count_agree`. No other
reference uses a lemma.

## Targets

Twenty-six tasks. Each target links to its source: the upstream file in this
repository, where the function is unchanged since the pinned commit, or, for the
two authored targets, `build.py`, which embeds them.

| target | module | probes | guess | id |
|---|---|---|---|---|
| [`validate_price_ordering`](/aptos-move/framework/aptos-experimental/sources/trading/order_book/bulk_order_utils.move#L181) | `bulk_order_utils` | adjacent-pair scan with an early return, strict both ways | hard | `TR-order-010` |
| [`discard_price_crossing_levels`](/aptos-move/framework/aptos-experimental/sources/trading/order_book/bulk_order_utils.move#L209) | `bulk_order_utils` | least non-crossing index -- a prefix fact, not a fold | hard | `TR-discard-011` |
| [`cancel_at_price_level`](/aptos-move/framework/aptos-experimental/sources/trading/order_book/bulk_order_utils.move#L303) | `bulk_order_utils` | removal from two coupled vectors; zero for absent and for an empty level | hard | `TR-cancel-026` |
| [`new_bulk_order_request_with_sanitization`](/aptos-move/framework/aptos-experimental/sources/trading/order_book/bulk_order_utils.move#L38) | `bulk_order_utils` | **composition** -- eleven assertions, four through helpers that need exact contracts | hard | `TR-sanitize-028` |
| [`match_order_and_get_next_from_bulk_order`](/aptos-move/framework/aptos-experimental/sources/trading/order_book/bulk_order_utils.move#L272) | `bulk_order_utils` | tuple of options; three aborts the source never states | hard | `TR-match-029` |
| [`reinsert_order_into_bulk_order`](/aptos-move/framework/aptos-experimental/sources/trading/order_book/bulk_order_utils.move#L237) | `bulk_order_utils` | merge or prepend; unchecked addition, unguarded read | hard | `TR-reinsert-030` |
| [`validate_not_zero_sizes`](/aptos-move/framework/aptos-experimental/sources/trading/order_book/bulk_order_utils.move#L164) | `bulk_order_utils` | linear scan, one quantifier | guessable | `TR-nonzero-031` |
| [`is_taker_order`](/aptos-move/framework/aptos-experimental/sources/trading/order_book/price_time_index.move#L171) | `price_time_index` | option-guarded inclusive comparisons | guessable | `PT-taker-034` |
| [`keep_alive`](/aptos-move/framework/aptos-experimental/sources/trading/market/dead_mans_switch_tracker.move#L295) | `dead_mans_switch_tracker` | map update reading the global clock; strict expiry | hard | `DM-keepalive-035` |
| [`is_order_valid`](/aptos-move/framework/aptos-experimental/sources/trading/market/dead_mans_switch_tracker.move#L207) | `dead_mans_switch_tracker` | strict session start, inclusive expiry, defaulted time | hard | `DM-valid-036` |
| [`refill`](/aptos-move/framework/aptos-framework/sources/account/rate_limiter.move#L43) | `rate_limiter` | token bucket; five unstated aborts, remainder carry | hard | `RL-refill-037` |
| [`validate_tiers`](/aptos-move/framework/aptos-framework/sources/transaction_limits.move#L119) | `transaction_limits` | pairwise order, inclusive in one field and strict in the other | hard | `TL-tiers-038` |
| [`new_tiers`](/aptos-move/framework/aptos-framework/sources/transaction_limits.move#L142) | `transaction_limits` | **composition** of a per-element range check and the pairwise order | hard | `TL-build-039` |
| [`find_min_stake_required`](/aptos-move/framework/aptos-framework/sources/transaction_limits.move#L170) | `transaction_limits` | search through the inline `vector::find` | hard | `TL-find-040` |
| [`validate_scheme`](/aptos-move/framework/aptos-framework/sources/account/common_account_abstractions/ethereum_derivable_account.move#L91) | `ethereum_derivable_account` | a contract that is only an abort condition over a character class | hard | `EA-scheme-041` |
| [`range_with_step`](/aptos-move/framework/move-stdlib/sources/vector.move#L828) | `vector_range` | overflow of an increment whose value is never used | hard | `VR-range-042` |
| [`request`](/aptos-move/framework/aptos-framework/sources/account/rate_limiter.move#L32) | `rate_limiter` | **composition** over `refill` | hard | `RL-request-043` |
| [`validate_enough_stake`](/aptos-move/framework/aptos-framework/sources/transaction_limits.move#L226) | `transaction_limits` | **composition** over a global configuration, only aborts | hard | `TL-enough-044` |
| [`split_signature_bytes`](/aptos-move/framework/aptos-framework/sources/account/common_account_abstractions/sui_derivable_account.move#L136) | `sui_derivable_account` | two loops sharing one index | hard | `SU-split-045` |
| [`get_pending_transactions`](/aptos-move/framework/aptos-framework/sources/multisig_account.move#L446) | `multisig_account` | loop over a table range; overflow and missing-entry aborts | hard | `MS-pending-046` |
| [`can_execute_with_timelock`](/aptos-move/framework/aptos-framework/sources/multisig_account.move#L501) | `multisig_account` | two resources, a table and the clock; aborts the threshold does not avoid | hard | `MS-timelock-047` |
| [`available_transaction_queue_capacity`](/aptos-move/framework/aptos-framework/sources/multisig_account.move#L581) | `multisig_account` | clamp over a subtraction that can underflow | guessable | `MS-capacity-048` |
| [`upsert_provider_jwks`](/aptos-move/framework/aptos-framework/sources/jwks.move#L587) | `jwks` | stop-and-insert scan under an uninterpreted comparator | hard | `JW-upsert-049` |
| [`update_config`](/aptos-move/framework/aptos-framework/sources/transaction_limits.move#L195) | `transaction_limits` | **composition** with a signer check and a create-or-replace global write | hard | `TL-update-050` |
| [`select`](build.py#L704) | `selection_machine` | **function values** | hard | `SM-select-022` |
| [`partition`](build.py#L732) | `lomuto_partition` | **in-place permutation** | hard | `QP-part-025` |

Besides computations over vectors, the corpus covers global state (the clock in
`dead_mans_switch_tracker`, `rate_limiter` and `can_execute_with_timelock`,
account resources in `multisig_account`, a configuration read by
`validate_enough_stake` and written by `update_config`), an intrinsic map
(`dead_mans_switch_tracker`), tables (`multisig_account`), inline higher-order
iteration (`find_min_stake_required`), and composition targets.

### Excluded targets

Two kinds of target are kept out because the prover cannot prove their complete
contracts. Code built on `vector::fold`: `fold`'s lambda both writes its
captured accumulator and forwards to the user's function, and the prover's
`folds_of` derivation cannot split that effect across the forwarding
([#20383](https://github.com/aptos-labs/aptos-core/issues/20383)). And a
function whose result is an `Option<u64>` computed by arithmetic: spec
arithmetic is typed `num`, so the natural contract compares an `Option<u64>`
with an `Option<num>`, which the type checker accepts but the prover's Boogie
translation does not
([#20672](https://github.com/aptos-labs/aptos-core/issues/20672)).
`match_order_and_get_next_from_bulk_order` also returns options, but of values
read from vectors, which keep their `u64` type.

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
  of `is_taker_order`, which upstream holds two ordered maps and is observed
  only through its best bid and ask.
- **Reduced modules.** `timestamp`, `event` and `system_addresses` keep only
  what the targets call.

Headers describe provenance only. They do not say what a target's contract is
or where it is hard; that is what the task asks for.

## References

A reference is the package with one module's complete specification written in.
Only the specification is committed, as an add-only patch under
[`references/`](references/), checked by `build_references.py` to add
specification and no pragma beyond `opaque`. Every helper a target calls in the
corpus code has a complete opaque contract there, proved against its body: the
helpers of its own module, `new_bulk_order_request` of
`extracted_bulk_order_types` in the `extracted_bulk_order_utils` reference, and
`get_transaction`, whose upstream contract the multisig reference only marks
`opaque`.

## Task trees

A task asks for its target's specification and nothing else. It starts from
the package with the reference contracts of every function its target calls in
the module, transitively, together with their loop invariants and the spec
functions they use. The target's own reference -- its contract and the loop
invariants in its body -- is withheld, and so is every other function's: a
caller's contract can restate what the target does. A callee without a
reference contract is read through its body. `prepare_tasks.py` derives these
trees from the references, takes the call closure from `move-flow`'s package
inventory, records each as a preparation patch under [`patches/`](patches/),
pins it in the manifest, and rebases the mutant anchors onto it. It refuses a
module whose reference, removed entirely, does not give back the package.

```text
python3 corpus-v4/build.py                 # generate package/sources from the pinned commit
python3 corpus-v4/build.py --verify        # regenerate in memory and compare with the manifest
python3 corpus-v4/build_references.py      # assemble references/build/
python3 corpus-v4/prepare_tasks.py         # task trees: patches/, manifest pins, mutant anchors
python3 corpus-v4/prepare_tasks.py --verify
python3 corpus-v4/compose.py               # per-task view into corpus-v4/inspect/
```

## Mutants

Two disjoint sets of three mutants per task: a refutation set
([`mutants/`](mutants/)) shown to the agent as obligation categories, and a
held-out scoring set ([`mutants-scoring/`](mutants-scoring/)). Readable
descriptions are in [`mutant-specs/`](mutant-specs/); `author_mutants.py`
anchors them.

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
and survives.

## Screening and round selection

`harness.screen_v3 --all-ready` admits all twenty-six ready tasks (records in
[`screening/`](screening/)): each is well-formed and its reference proves within
the 20-second threshold, in under 12 seconds of wall time per task. Sixteen are
`wp_hard`. For thirteen, unaided WP stops at a loop without an invariant. In the
other three, WP's output carries clauses it flags as untrusted: top-level
quantifiers in `new_bulk_order_request_with_sanitization` and
`find_min_stake_required`, and an existential over the limiter the `refill`
callee updates in `request`.

The round runs all twenty-six: `select_round.py --size 26 --max-guessable 3
--keep-redundant` selects every ready task, covering all 32 feature strata,
thirteen tasks with a loop that needs an invariant, twelve modules and three
guessable controls. The selection reports one near-duplicate pair,
`validate_not_zero_sizes` and `cancel_at_price_level` (source similarity 0.55),
and the modules are uneven -- seven from `bulk_order_utils` and five from
`transaction_limits` -- so a round reports per task as well as pooled.

```text
python3 -m harness.screen_v3 --manifest corpus-v4/manifest.json \
  --experiment-config config/default.json --corpus-config config/corpus.json \
  --results-dir corpus-v4/screening --output corpus-v4/screening/summary.json --all-ready
python3 corpus-v4/select_round.py --size 26 --max-guessable 3 --keep-redundant --write
```
