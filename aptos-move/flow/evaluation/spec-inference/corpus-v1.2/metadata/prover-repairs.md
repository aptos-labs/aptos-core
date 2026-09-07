# Shared-framework prover repairs (corpus-v1.2)

The corpus keeps one editable framework package (`framework/`), produced by
`harness.prepare` and then patched with `source-repairs.patch`. This ledger
records the behavior-preserving source changes and the study-authored reference
contracts needed to make every selected target's reference prove. Evidence for
each is the `reference_proof` block of `screening/results/<task>.json`.

## Repairs to upstream sources and contracts

| Module/function | Repair |
| --- | --- |
| `0x1::code::freeze_code_object` | Rebuilt the `frozen` vector with an explicit loop and an `update_field` invariant instead of a higher-order traversal. |
| `0x1::code::check_dependencies` | Explicit loops with first-match (`spec_first_package_named`), dependency-list (`spec_module_deps`), abort-suffix (`spec_deps_abort_from`) and allowed-deps (`spec_allowed_deps`) invariants; opaque exact contract. Recursive spec funs carry `[weight = 20]`; a forall-apply lemma form timed out and was replaced by the suffix predicate. |
| `0x1::code::get_module_names` | Indexed loop with a length/prefix invariant. |
| `0x1::multisig_account::validate_owners` | Indexed loop with invariants over the processed prefix. |
| `0x1::aptos_coin::find_delegation` | Loop invariants for the first-match search. |
| `0x1::fungible_asset::is_address_balance_at_least` | Opaque with `spec_is_address_balance_at_least`. |
| `0x1::object::is_owner` | Opaque. |
| `0x1::cmp::{is_eq, is_ne, is_lt, is_le, is_gt, is_ge}` | Complete opaque contracts (`aborts_if false; ensures result == (self is Ordering::X)`); a partial callee otherwise forces every caller partial. Also applied to the real `move-stdlib` on request. |
| `0x7::pending_order_book_index` (loop bodies) | Loop invariants plus an `index_before` length snapshot. |
| `0x7::bulk_order_book::get_remaining_size` | Reads the order through `orders.borrow(&account)` after the existing `contains` assert (same abort code) instead of `get(...).destroy_some()`: the opaque option copy is only extensionally equal to the stored value, so a recursive sum over its `sizes` vector never matches the contract's. |
| `0x5::bulk_order_types::get_total_remaining_size` | Explicit loop with a conservation invariant (`total + spec_sum_from(sizes, i) == spec_sum_from(sizes, 0)`), `sum_from_nonneg` lemma, opaque exact contract. |
| `0x1::aptos_governance::update_governance_config` | `pragma verify = true` (the module disables verification) and removal of the stale `GovernanceEvents` abort condition: `event::emit` no longer needs the resource. |
| `0x1::transaction_fee::store_aptos_coin_mint_cap` | Verification enabled (`pragma verify = true`; the module disables it) and the paired mint-ref abort conditions stated. |
| `0x1::account::revoke_any_signer_capability`, `0x1::vesting::vesting_schedule`, `0x1::aptos_coin::find_delegation` | Post-conditions added to abort-only upstream references (offer cleared; result is the stored schedule; result is the first delegation index). |
| `0x1::multisig_account::validate_owners`, `0x1::stake::append` | `pragma verify = false` and `[abstract]` removed; the loop invariants (in the source) make the contracts verify. |
| `0x1::coin::get_paired_mint_copy_ref` | Exact opaque contract so `store_aptos_coin_mint_cap` verifies. |
| `0x1::code::get_module_names` | `[abstract]` removed: the loop invariant lets the contract be verified rather than assumed. |
| `0x1::dispatchable_fungible_asset::derived_balance` | Opaque partial summary over an uninterpreted `spec_derived_balance_at(store_addr)`: the balance hook is user code, so only its stability is promised. |
| `0x1::coin::balance` | Replaced the pre-FA-migration `CoinStore`-only reference (under `verify = false`) with the exact sum of the legacy store balance and the paired primary-store balance, `pragma verify = true`. |

## Study-authored references (aptos-experimental / aptos-trading)

`native_position_types`, `trading_native_capability`, `price_time_index`,
`order_book`, `pending_order_book_index`, `dead_mans_switch_operations`
(partial), `bulk_order_types`, `bulk_order_book` — all in `*.spec.move` files
created by the patch. Selection policy marks these
`study_authored_reference`.

## Tool findings recorded as task properties

- WP emits `aborts_of`/`result_of` over intrinsic-mapped callees
  (`big_ordered_map::borrow`), which the prover rejects ("no specification but
  referenced by a behavioral predicate"). Recorded as `wp_hard` /
  `implementation_failure` for `AX-bulk-order-book-009`; a WP gap to close in
  `aptos-move/flow`, not a corpus defect.
- `result_of` over transparent std/helper callees (`error::out_of_range`,
  `order_book_utils::new_default_big_ordered_map`) yields `sathard` clauses:
  the agent must rewrite them (trust rule, see `spec_inf_rules.md`).
- Uninvariant loops (`stake::append`) make WP decline; the loop invariant is
  the task.

## Task packages and inline specification blocks

`harness.prepare` blanks the inline `spec { ... }` blocks (loop invariants,
inline assertions) of a target's body in the task package, so a loop invariant
the reference carries is part of the reference rather than a hint. The one
exception is an `inline fun` target (`AX-pending-order-book-index-006`): its
body is expanded into callers, so the checker's bytecode comparison would count
the blanking as an implementation change; its invariants stay in the task.
