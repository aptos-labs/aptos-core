# Verifying the Aptos framework packages

What passes under Verex across `move-stdlib`, `aptos-stdlib`,
`aptos-framework` and `aptos-trading`, what does not, and the causes found.
Open problems that came out of it are in
[`roadmap.md`](../plan/roadmap.md) (the language gaps authored proofs
reported: [`proof-language.md`](../plan/proof-language.md), "Gaps authored
proofs of the framework reported").

A *target* is a function `verex-move verify` proves, one theorem with a budget
([`source-verification.md`](../designs/source-verification.md), "Budgets"). A
module with no targets has nothing of its own to prove (`pragma verify =
false`, natives only).

## How to reproduce

```bash
scripts/verify-modules.py <package> --jobs 7 --timeout 300 --logs <dir>
```

The driver first compiles the package's namespaces to `.olean` files under
`build/verex/namespaces` of the package
([`relocatable-namespaces.md`](../designs/relocatable-namespaces.md),
"Compiled namespaces"). `<dir>/<module>.stages` holds one `theorem done` line
per target with its heartbeats, its outcome (`verified`, `failed`, `budget`)
and whether its proof is authored or automatic; `<module>.out` holds the
messages. A module that hit the timeout keeps the stage lines of what
finished. Do not cap `LEAN_NUM_THREADS` for a sweep: at 8 processes a cap of
4 made the framework about 40% slower on the same code.

## Inventory

Last measured at `621f3630829`, with `--heartbeats 300 --timeout 600` on an
otherwise idle machine, so a target over 300M heartbeats counts as over
budget here even where the default budget of 1000M would prove it.
Over-budget targets are therefore an upper bound on what needs work, and a
module's count is complete unless it timed out.

| Package | Modules | Fully verified | Timed out | Targets verified | Failed | Over 300M | Verified |
|---|---:|---:|---:|---:|---:|---:|---:|
| `move-stdlib` | 13 | 13 | 0 | 71 | 0 | 0 | 100% |
| `aptos-stdlib` | 38 | 32 | 0 | 224 | 3 | 31 | 87% |
| `aptos-framework` | 78 | 34 | 0 | 521 | 49 | 211 | 67% |
| `aptos-trading` | 3 | 1 | 0 | 7 | 0 | 3 | 70% |
| Total | 132 | 80 | 0 | 823 | 52 | 245 | 73% |

Several targets sit just above the 300M cap (`pool_u64::buy_in`,
`big_ordered_map::test_verify_iter_next`, `hot_state_config::on_new_epoch`,
`order_match_types::validate_single_order_reinsertion_request`), so a small
cost change moves them.

Of the over-300M targets rerun at the default 1000M, in `big_vector`,
`pool_u64_unbound`, `smart_table`, `smart_vector`, `storage_slots_allocator`,
`block`, `coin`, `multisig_account`, `stake` and `staking_contract`, 4 of 93
verify, 34 fail and 55 stay over budget: a larger budget is not the lever.
All but 2 over-budget targets, and all failed ones, are automatic attempts,
so they have not shown a proof is out of reach: authored proofs are their
next tier ([`proof-strategy.md`](../designs/proof-strategy.md)), largest
modules first, and a solver change is for a cause that repeats across
modules.

## Targets by module

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

`big_ordered_map`'s three targets over 300M verify at the default budget, with
authored proofs.

## Causes across modules

Failure messages by cause, most frequent first, in `aptos-framework`. A count
is of messages, several to a target. A target over budget leaves messages for
the leaves it had not reached, so a count here need not be a missing fact.
Every `canonical` message, and every `OptionalAggregator` message outside its
own module, comes from a target over its 300M budget.

| Messages | Cause | Seen first in |
|---:|---|---|
| 7 | Continuation after a call of `emit` | `account` |
| 7 | Data invariant `intrinsic_map` of `simple_map` for a value written | `vesting` |
| 5 | Continuation after a call of `some` | `account` |

Clauses not established, by module: `account` 28, `gas_schedule` 23,
`staking_config` 20, `staking_contract` 18, `object` 16, `randomness` 16,
`block` 15, `storage_gas` 14, `randomness_api_v0_config` 12.

Over-budget targets spend mainly in walking paths: of eight profiled
targets, five never reach a leaf; `coin::coin_supply` and
`smart_vector::swap` spend most in leaves. The walk's open levers are
[`performance.md`](../designs/performance.md), "Open levers".

The config modules' targets fall into groups:

- Over budget: `version`, `consensus_config`, `execution_config`,
  `gas_schedule`, `randomness_api_v0_config::on_new_epoch`.
- Dependent on the unit: `gas_schedule::initialize` and
  `randomness_api_v0_config::initialize` verify when the rendering is cut to
  what they reach and fail in the full unit. Which invariants a function
  assumes matches the Prover's memory usage (`initialize` reaches
  `GenesisEndMarker` through `assert_genesis`, and the invariants reading it
  are ones the Prover assumes too); what scales with them is leaf work.
- The leaf budget: `code::get_module_names`,
  `confidential_range_proofs::verify_batch_range_proof`,
  `object::create_object_from_guid`, `reconfiguration::initialize`,
  `reconfiguration_state::on_reconfig_start` and
  `staking_proxy::set_stake_pool_operator` exhaust an attempt's allowance in
  the deciders and verify with `verex.leafHeartbeats=0`. Each needs a real
  step or a cheaper decider, not a larger allowance.

## Not diagnosed

- `confidential_balance::add_mut_base` fails with a kernel type mismatch: a
  proof reads `named.val` where the statement has the value it names.
- `staking_contract::update_distribution_pool` fails.
- `decryption::on_new_epoch` stops at `maximum recursion depth has been
  reached`.
- `setup unfold` is the largest stage of `multisig_account::can_execute`.
- `voting::{register, is_voting_closed, get_proposal_metadata,
  get_proposal_metadata_value}`.
- `aggregator_v2`'s `verify_aggregator_generic_add` and `_sub` state what the
  Prover's prelude models for the `add`/`sub` wrappers, which never abort
  there; the Move bodies abort, so these specifications look false of the
  code.
- Stored invariants at generic frames: a parameter as a map key is rejected
  (`struct Keyed<K> { values: Table<K, u64> }`); ordered maps at a parametric
  frame stay stuck; `contains(&R[a].f, k)` with a local receiver renders as
  `«f.contains»`.

## Needing authored proofs

- `staking_config::initialize`: nonlinear division in `create_from_rational`.
- `big_vector`: nonlinear arithmetic over `i / bucket_size` with its data
  invariants.
- `pool_u64_unbound::transfer_shares` inlines `deduct_shares` and
  `add_shares` into 366 leaves; it needs cheaper walking or a continuation
  shared across paths, not a longer script.
