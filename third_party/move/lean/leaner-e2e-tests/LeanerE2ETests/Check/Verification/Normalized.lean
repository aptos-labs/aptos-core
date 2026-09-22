-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-!
# Storage verification on the generic route

The storage contracts of `Storage.lean`, verified by normalization: the
generated body tree computed by `leaner_normalize`, the resources exposed
and the writes represented from the context, and the certified closing
over the normal form — one theorem shape for every body, with no plan and
no per-shape law.  The module is the same; only the route differs.
-/

set_option leaner.route "native"

namespace LeanerLang.Tests.VerificationNormalized

leaner module 0x42::storage_normalized where
  struct Amount has Copy, Drop, Store where
    value : u64

  struct Coin has Key where
    amount : Amount

  struct Flag has Key where
    on : Bool

  public fun balance_of(addr : Address) -> u64 := Coin[addr].amount.value

  spec balance_of where
    aborts_if !exists<Coin>(addr)
    ensures result == global<Coin>(addr).amount.value
    ensures result == old(global<Coin>(addr).amount.value)

  public fun is_published(addr : Address) -> Bool := exists<Coin>(addr)

  spec is_published where
    ensures result == exists<Coin>(addr)

  public fun flag_of(addr : Address) -> Bool := Flag[addr].on

  spec flag_of where
    aborts_if !exists<Flag>(addr)
    ensures result == global<Flag>(addr).on

  public fun remove(addr : Address) -> u64 := do
    let Coin { amount := amount } := move_from<Coin>(addr)
    return amount.value

  spec remove where
    aborts_if !exists<Coin>(addr)
    ensures result == old(global<Coin>(addr).amount.value)
    ensures !exists<Coin>(addr)
    modifies global<Coin>(addr)

  /-- A whole-resource mutable borrow: the loan's write-back is keyed, so
  the post-state read of the same key meets what the body wrote. -/
  public entry fun deposit(addr : Address, value : u64) -> Unit := do
    let coin := &mut Coin[addr]
    *coin := new Coin { amount := new Amount { value := (*coin).amount.value + value } }

  spec deposit where
    requires exists<Coin>(addr)
    ensures global<Coin>(addr).amount.value
        == old(global<Coin>(addr).amount.value) + value
    aborts_if global<Coin>(addr).amount.value + value > 18446744073709551615
    modifies global<Coin>(addr)

  const E_INSUFFICIENT : u64 := 3

  /-- A field bracket guarded by an `assert!`: the throw sits on the guard's
  else-arm and retires the loans itself. -/
  public entry fun withdraw(addr : Address, amount : u64) -> Unit := do
    let value := &mut Coin[addr].amount.value
    let current := *value
    assert!(current >= amount, E_INSUFFICIENT)
    *value := *value - amount

  spec withdraw where
    requires exists<Coin>(addr)
    modifies global<Coin>(addr)
    ensures global<Coin>(addr).amount.value
        == old(global<Coin>(addr).amount.value) - amount
    aborts_if old(global<Coin>(addr).amount.value) < amount with E_INSUFFICIENT

  struct Pair has Key where
    left : u64
    right : u64

  /-- A field bracket with a sibling beside the focus: the write lands on
  `right`, and `left` is carried through the bracket untouched. -/
  public entry fun bump_right(addr : Address, amount : u64) -> Unit := do
    let value := &mut Pair[addr].right
    *value := *value + amount

  spec bump_right where
    requires exists<Pair>(addr)
    modifies global<Pair>(addr)
    ensures global<Pair>(addr).right == old(global<Pair>(addr).right) + amount
    ensures global<Pair>(addr).left == old(global<Pair>(addr).left)
    aborts_if old(global<Pair>(addr).right) + amount > 18446744073709551615

  verify balance_of
  verify is_published
  verify flag_of
  verify remove
  verify deposit
  verify withdraw
  verify bump_right

end LeanerLang.Tests.VerificationNormalized
