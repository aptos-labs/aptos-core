# Verifying the Aptos framework packages

Goal: most verification obligations of `move-stdlib`, `aptos-stdlib`,
`aptos-framework` and `aptos-trading` pass under Verex. This file is the
inventory: what passes, what does not, and why, in the order to work on it.

A *target* is a function `verex-move verify` proves, one theorem with a budget
(`source-verification.md`, "Budgets"). A module with no targets has nothing of
its own to prove (`pragma verify = false`, natives only).

## How to reproduce

```bash
scripts/verify-modules.py <package> --jobs 7 --timeout 300 --logs <dir>
```

The driver first compiles the package's namespaces to `.olean` files under
`build/verex/namespaces` of the package (`relocatable-namespaces.md`, "Compiled
namespaces"). `<dir>/<module>.stages` holds one `theorem done` line per target with its
heartbeats, its outcome (`verified`, `failed`, `budget`) and whether its proof is
authored or automatic; `<module>.out` holds the messages. A module that hit the
timeout keeps the stage lines of what finished.

## Inventory

Do not cap `LEAN_NUM_THREADS` for a sweep: at 8 processes a cap of 4 took the framework from
717 s to 1,001 s on the same code.

Measured with `--heartbeats 300 --timeout 600` on an otherwise idle machine, so a
target over 300M heartbeats counts as over budget here even where the default
budget of 1000M would prove it. Over-budget targets are therefore an upper bound on
what needs work, and a module's count is complete unless it timed out.

| Package | Modules | Fully verified | Timed out | Targets verified | Failed | Over 300M | Verified |
|---|---:|---:|---:|---:|---:|---:|---:|
| `move-stdlib` | 13 | 13 | 0 | 71 | 0 | 0 | 100% |
| `aptos-stdlib` | 38 | 32 | 0 | 224 | 3 | 31 | 87% |
| `aptos-framework` | 78 | 34 | 0 | 521 | 49 | 211 | 67% |
| `aptos-trading` | 3 | 1 | 0 | 7 | 0 | 3 | 70% |
| Total | 132 | 80 | 0 | 823 | 52 | 245 | 73% |

No module stops at the timeout. Several targets sit just above the 300M cap, such as
`big_ordered_map::test_verify_iter_next`, so a small cost change moves them.

No module is blocked by another's rendering any more: each runs its own targets.

### Regressions on this base

Verified targets cost about 3% more heartbeats than at the checkpoint before the Rust stack;
not traced. `pool_u64::buy_in`, `big_ordered_map::test_verify_iter_next`,
`hot_state_config::on_new_epoch` and `order_match_types::validate_single_order_reinsertion_request`
sit just above the 300M cap. `confidential_balance::add_mut_base` fails with a kernel type
mismatch: a proof reads `named.val` where the statement has the value it names.
`staking_contract::update_distribution_pool` went from over budget to failed, not diagnosed.

### Over budget

Of the over-300M targets rerun at the default 1000M, in `big_vector`, `pool_u64_unbound`,
`smart_table`, `smart_vector`, `storage_slots_allocator`, `block`, `coin`,
`multisig_account`, `stake` and `staking_contract`, 4 of 93 verify, 34 fail and 55 stay
over budget. A larger budget is not the lever. All but 2 over-budget targets, and all
failed ones, are automatic attempts, so they have not shown a proof is out of reach:
authored proofs are their next tier (`proof-strategy.md`), largest modules first, and a
solver change is for a cause that repeats across modules.

### 1. Causes across modules

Failure messages by cause, most frequent first, in `aptos-framework`. A count is of
messages, several to a target. A target over budget leaves messages for the leaves
it had not reached, so a count here need not be a missing fact.

| Messages | Cause | Seen first in |
|---:|---|---|
| 7 | Continuation after a call of `emit` | `account` |
| 7 | Data invariant `intrinsic_map` for a value written (`simple_map.move:22`) | `vesting` |
| 5 | Continuation after a call of `some` | `account` |

Elaborating a statement with routed callees was quadratic in their number: each inlined
callee's hypothesis spelled out the whole chain of meanings. The chain is now one
definition, whose elaboration falls outside the target's budget. `genesis::initialize`,
`genesis::create_accounts` and `staking_contract::add_stake` used to stop at their statement
with no heartbeats spent; they now reach the walk and stay over budget.

### Where over-budget targets spend

By `verex.solverProfile` at 300M, walking paths is the main cost: of eight profiled targets,
five never reach a leaf. `coin::coin_supply` and `smart_vector::swap` spend most in leaves.
Open there, by what the profile shows:

- `split` at 0.4–0.8M each and folded continuations walked again on each arrival.
- `inline` walks the continuations, which a leaf's normal form depends on: 165M on
  `primary_fungible_store::deposit`, 30M on `multisig_account::can_execute`. Walking around them
  gives `inline` 119M on `deposit`, but `call` and `split` then cost 28M more, and it changes what
  a leaf receives ([`performance.md`](../designs/performance.md), "Calls").
- `call-contract` still does on the default path (`wp_call_normal`): 328M on `can_execute`.
  Holding the continuation there, abstracted over its reads of its parameters and with the
  call's premises named after the callee, gives 20M and 27 leaves instead of 18, but is not
  landed. The eager walk also normalized the rest of the program with the call's facts in
  context: without it 3 smoke fixtures and 13 `big_ordered_map` targets fail
  ([`performance.md`](../designs/performance.md), "Calls").
- A conditional now splits by `wp_ite`, not by `split`: on `stake::leave_validator_set` the split
  stage fell from 590M to 277M, with 14 leaves reached instead of 3, and it stays over 1G.
  `deposit` and `transfer` still reach no leaf at 1G: `bind` (172M, 195M), `split` (175M, 231M),
  `inline` and `call-cases` (205M on `deposit`) remain. On the path before this change, two
  `verex_split` failures on `Not`-headed goals cost about 100M each outside its instrumented
  steps, not located.
- `setup unfold` is 89M on `multisig_account::can_execute`, not diagnosed.
- `verex.sharedContinuations` reached 26 leaves on `primary_fungible_store::deposit` where the
  default reached none. One run, before the statement fix.

### What authored proofs need from the language

Reported by authored proofs, not yet designed:

- `result` is unavailable for the value written through a returned `&mut` (`big_vector::borrow_mut`).
- A `lemma` item's map parameter has no validity fact.
- The index a vector search returns has no name; `spec.indexOfVector` does not translate
  (`big_vector::index_of`).
- A read in bounds does not bridge `values[k]? != some v` to `b[k] != v`, even with `solve`.
- `contains` resolves to a module's own function of that name; `val in b` does not parse in a step.
- `global<…>` at `at entry` gives `internal: a clause reads storage without a frame or a unit`; inside
  `if` the facts are silently dropped (`staking_config::reward_rate`, whose repeated facts cost most
  of its 780.9M).
- `when if 99 then` raises no selector error.
- Points cannot reach inside inlined callees, and their results have no name.
- A `let` inside an `if` branch is not in scope for that branch's `assert`s.
- `q == n - 1 ==> l == last` is rewritten to `True → l == last`, which `decide` does not read.
- `decide` does not unpack stored invariants.
- `split` on a value whose certificates mention it fails: its generalization abstracts some
  occurrences only (`split seed` in `randomness::next_32_bytes`).
- `assert` at `at entry` of a fact reading a generic spec function's cut hits maximum recursion
  depth; deep, not infinite. Suspect: a carrier reduced through `toSkolem`.

Each of these forces `lean =>` or a detour in a proof written so far:

- A stated equation can act as a rewrite rule on later facts.
- Keys for `leaf` obligations.

The clauses reached through `system_addresses` are not one cause. An `aborts_if` leaf
lists every clause of its disjunction, so that line is only the first named. The config
modules' targets split into three groups:

- Over budget: `version`, `consensus_config`, `execution_config`, `gas_schedule`,
  `randomness_api_v0_config::on_new_epoch`.
- Dependent on the unit: `gas_schedule::initialize` and
  `randomness_api_v0_config::initialize` verify when the rendering is cut to what they
  reach (11.0M, 30.6M) and fail in the full unit (46.6M, 70.4M). Which invariants a function
  assumes matches the Prover's memory usage: `initialize` reaches `GenesisEndMarker` through
  `assert_genesis`, and the 20 invariants reading it are ones the Prover assumes too. What
  scales with them is leaf work: added invariants raise the second's cost, 30.6M to 142.2M at
  80 of them. `verex_split_hypotheses` rescans the whole context after each split, and
  `verex_call_observations`'s `simp_all` covers it.
- The leaf budget: twelve automatic targets failed by default and verified with
  `verex.leafHeartbeats=0`, so they are cost and no missing fact. Six verify by default now
  (`proof-strategy.md`, "One leaf cap", "Facts a decider reads"):
  `account::verify_signed_message`, `fungible_asset::remove_store`,
  `gas_schedule::initialize`, `gas_schedule::set_for_next_epoch`,
  `reconfiguration_state::start_time_secs`, `stake::get_validator_state`. The other six
  exhaust an attempt's allowance in the deciders: `code::get_module_names`,
  `confidential_range_proofs::verify_batch_range_proof`, `object::create_object_from_guid`,
  `reconfiguration::initialize`, `reconfiguration_state::on_reconfig_start` and
  `staking_proxy::set_stake_pool_operator`, whose `pipeline` takes 98M. Each gets a real step
  or a cheaper decider, not a larger allowance.
- Preparation: a leaf's bounds now follow its obligation (`proof-strategy.md`, "Bounds follow
  the obligation"), which removed the bounds of stored terms from `coin::name`, `symbol` and
  `decimals`. What is left of their `prepare` is mostly `normalize` over the stored reads, 41M,
  where the block normal form of an empty authored proof costs less. Stored reads themselves are
  not deferred: `on_reconfig_start` and the stored-invariant fixtures need them.

A stored invariant is stated at the frame of the resource's type arguments, a leaf at its
own. Stated in the leaf's spelling and with sequence maps moved to its frame, maps at
struct keys and generic resources verify. Open there:

- A parameter as a map key is rejected (`struct Keyed<K> { values: Table<K, u64> }`).
- Ordered maps at a parametric frame stay stuck.
- `voting::{register, is_voting_closed, get_proposal_metadata,
  get_proposal_metadata_value}`, not diagnosed.
- `contains(&R[a].f, k)` with a local receiver renders as `«f.contains»`.

The typing facts a theorem assuming typing takes (`ResourcesTyped`, `TypesAgree`) are decided
by the kernel over the whole unit, once per unit, at the first such target, outside any budget.
Their statements must cover the whole unit, since an unseen invocation may run any function.
`TypesAgree` is proved once and `ResourcesTyped` takes its verdict; a literal read through its
contract no longer needs either, which leaves function-typed parameters, `result_of`, and
non-generic `aborts_of` in executable units. The checks read a once-per-unit view of the tables
(`TypingView`, `VerexIR/Proofs/IndexedTyping.lean`); each reader is proved equal to the
array-reading original, and the array-reading checks stay the statement.
Five functions returning a tuple with a shared reference (`borrow_front` and `borrow_back` of
`ordered_map` and `big_ordered_map`, and `internal_leaf_iter_borrow_entries_and_next_leaf_index`)
stopped `TypesAgree` on every unit holding those modules; they are not closure targets now
(`higher-order-functions.md`), and no rendering compiled for the runs above constructs a function
value of one.

Measured on `version`'s unit (70 namespaces sharing tables, 988 types, 2854 names, 1369
functions), one run at moderate load, through an interpreted harness, so these overstate the
kernel: `TypesAgree` 39.5 s and `ResourcesTyped` 8.7 s. Over the arrays, the resource loop over
the first table alone did not end in 840 s (RSS about 18 GB), a signature cost about 1 s, and 20
type identifiers cost 1.2 s near the start of the table and 12.5 s near its end. On
`chunky_dkg_config_seqnum`'s unit (14 namespaces, 109 types, 526 names, 303 functions) the array
checks took 4.1 s for resources, 4.4 s for signatures and 4.6 s for types.

The stage gap to `unit facts` before and after, one run each, under different load: `chunky_dkg_config_seqnum`
15.8 s to 3.8 s (46% to 10% of the module), `randomness_config` 18.3 s to 2.7 s (47% to 8%),
`stake::initialize_owner` 9.2 s both (2.5% before), not attributed. `version`'s targets assume no
typing, so only the harness above reaches its unit's proof.

`decryption::on_new_epoch` no longer leaves clauses open, but stops at `maximum recursion
depth has been reached`, not diagnosed.

Every `canonical` message, and every `OptionalAggregator` message outside its own
module, comes from a target over its 300M budget: the leaves left open when it ran
out, not a missing fact.

Clauses not established, by module, each to be diagnosed: `account` 28, `gas_schedule` 23,
`staking_config` 20, `staking_contract` 18, `object` 16, `randomness` 16, `block` 15,
`storage_gas` 14, `randomness_api_v0_config` 12. `staking_config`'s `initialize` needs
nonlinear division in `create_from_rational`, so an authored proof.

### 2. Targets by module

Failed, then over 300M.

| Package | Module | Verified | Failed | Over 300M |
|---|---|---:|---:|---:|
| `aptos-stdlib` | `smart_vector` | 6 | 0 | 14 |
|  | `big_vector` | 9 | 2 | 7 |
|  | `smart_table` | 8 | 0 | 5 |
|  | `pool_u64_unbound` | 19 | 0 | 3 |
|  | `storage_slots_allocator` | 15 | 0 | 2 |
| `aptos-framework` | `multisig_account` (timed out) | 12 | 6 | 36 |
|  | `stake` | 32 | 9 | 21 |
|  | `coin` | 27 | 4 | 13 |
|  | `primary_fungible_store` | 5 | 0 | 14 |
|  | `staking_contract` | 7 | 1 | 12 |
|  | `jwks` | 9 | 0 | 13 |
|  | `voting` | 12 | 5 | 7 |
|  | `genesis` | 1 | 1 | 10 |
|  | `account` | 28 | 2 | 8 |
|  | `fungible_asset` | 40 | 0 | 10 |
|  | `vesting` | 19 | 0 | 8 |
|  | `randomness` | 10 | 3 | 4 |
|  | `code` | 3 | 1 | 5 |
|  | `block` | 4 | 1 | 4 |
|  | `init` | 0 | 2 | 3 |
|  | `staking_config` | 14 | 2 | 2 |
|  | `storage_gas` | 9 | 2 | 2 |
|  | `reconfiguration_with_dkg` | 0 | 0 | 4 |
|  | `gas_schedule` | 3 | 1 | 3 |
|  | `object` | 39 | 1 | 2 |
|  | `staking_proxy` | 0 | 2 | 1 |
|  | `big_ordered_map` | 43 | 0 | 2 |
|  | `transaction_validation` | 3 | 1 | 1 |
|  | `aggregator_v2` | 14 | 2 | 0 |
|  | `confidential_balance` | 6 | 0 | 2 |
|  | `aptos_coin` | 2 | 1 | 1 |
|  | `confidential_amount` | 2 | 0 | 2 |
|  | `dkg` | 5 | 0 | 2 |
|  | `version` | 2 | 0 | 2 |
|  | `consensus_config` | 3 | 0 | 2 |
|  | `execution_config` | 1 | 0 | 2 |
| `aptos-trading` | `single_order_types` | 4 | 0 | 2 |

`big_ordered_map`'s three targets over 300M verify at the default budget, with authored proofs
(329M, 357M, 653M).

`aggregator_v2`'s `verify_aggregator_generic_add` and `_sub` state what the Prover's
prelude models for the `add`/`sub` wrappers, which never abort there; the Move bodies
abort, so these specifications look false of the code. `pool_u64_unbound::transfer_shares`
inlines `deduct_shares` and `add_shares` into 366 leaves, with `verex_obligations`
alone about 400M: it needs cheaper walking or a continuation shared across paths, not a
longer script. `big_vector` needs authored proofs: nonlinear arithmetic over
`i / bucket_size` with its data invariants.

### 3. Cross-cutting

- **Address representation.** Addresses are strings in the runtime and the
  denotation (`RuntimeValue.address`, `NTy.address`); the Move Prover's are integers
  in `0 .. 2^256`. Integers remove the second spelling of an address key's
  `DecidableEq` instance (`roadmap.md`, "Key-instance spelling") and bring addresses
  within reach of the arithmetic deciders.
- **Ghost memory through `modifies *`.** A callee with frame `modifies *` leaves
  every spec variable unconstrained, where the Prover never havocs ghost memory, so
  `ensures supply<T> == old(supply<T>)` fails across such a call. The loose and open
  frames gain one conjunct: every ghost resource instantiation other than the
  function's targets reads the same, owed by the callee and discharged by the
  `ghostWrites` fixed point. A callee of unknown body gets no such conjunct. It adds
  an obligation to every memory-writing function; size it first.
- **Frames of loops.** A loop writing the store keeps no frame, where the Prover
  havocs only what the loop modifies; one that only reads it keeps it whole. A function
  updating a spec variable in a loop therefore has the frame `modifies *`, and any
  function's closed frame over a loop's real writes fails. A frame per loop, from what
  its body writes, removes both.
- **Inlined callees' ghost reads.** `frameUses` walks callee contracts, not inlined
  bodies. It no longer matters for existence, and a read of an unlisted instance
  stays sound.

## Debt: what a module is verified against

A target's rendering imports every namespace rendered before it in its own export,
because those decide which modules' invariants its unit links
(`invariantRelatedModules`). So what a module is verified against depends on the
export's module set and its dependency order, not on what the module reaches: a
module rendered after it, whose invariant reads memory it writes, is not linked, and
verifies the function against that invariant in its own command instead, which the
rendering must therefore elaborate. Compiled namespaces keep this as it was. The fix is a package-level memory
summary, computed before any target, that makes the related set a function of the
package; a separate change.
