-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-!
# Global and nested borrows

Port of v0's `Verification/GlobalBorrows.lean`.  This keeps the six verified
functions together: whole-resource global borrows, a nested field borrow,
vector-element borrowing through a mutable parameter, disjoint sibling
borrows, and an element-field borrow of an owned vector.
-/

namespace LeanerLang.Tests.VerificationGlobalBorrows

set_option leaner.route "native"

leaner module 0x42::global_borrows where
  struct Counter has Key where
    value : u64

  public entry fun replace(addr : Address, amount : u64) -> Unit := do
    let counter := &mut Counter[addr]
    *counter := new Counter { value := amount }

  spec replace where
    requires exists<Counter>(addr)
    modifies global<Counter>(addr)
    ensures global<Counter>(addr).value == amount
    aborts_if false

  verify replace

  public fun read_whole(addr : Address) -> u64 := do
    let counter := &Counter[addr]
    let current := *counter
    return current.value

  spec read_whole where
    requires exists<Counter>(addr)
    ensures result == old(global<Counter>(addr).value)
    aborts_if false

  verify read_whole

  public entry fun bump_through(addr : Address) -> Unit := do
    let counter := &mut Counter[addr]
    let value := &mut counter.value
    *value := *value + 1

  spec bump_through where
    requires exists<Counter>(addr)
    modifies global<Counter>(addr)
    ensures global<Counter>(addr).value == old(global<Counter>(addr).value) + 1
    aborts_if old(global<Counter>(addr).value) + 1 > 18446744073709551615

  verify bump_through

set_option leaner.route "native"

leaner module 0x42::local_borrows where
  public fun bump_first(values : &mut Vector<u64>) -> Unit := do
    let first := &mut values[0]
    *first := *first + 1

  spec bump_first where
    requires values.length > 0
    ensures values[0] == old(values)[0] + 1
    aborts_if values[0] + 1 > 18446744073709551615

  verify bump_first

leaner module 0x42::sibling_borrows where
  struct Pair has Copy, Drop, Store where
    left : u64
    right : u64

  public fun read_siblings(pair : &mut Pair) -> u64 := do
    let left := &mut pair.left
    let right := &mut pair.right
    let right_value := *right
    let left_value := *left
    return left_value + right_value

  spec read_siblings where
    ensures result == pair.left + pair.right
    aborts_if pair.left + pair.right > 18446744073709551615

  verify read_siblings

leaner module 0x42::owned_borrows where
  struct Pair has Copy, Drop, Store where
    left : u64
    right : u64

  public fun bump_left() -> u64 := do
    let mut pairs : Vector<Pair> :=
      vector<Pair>[new Pair { left := 1, right := 2 }]
    let left := &mut pairs[0].left
    *left := *left + 5
    let pair := &pairs[0]
    let value := *pair
    return value.left

  spec bump_left where
    ensures result == 6
    aborts_if false

  verify bump_left

open Lean Elab Command in
run_cmd do
  for name in #[``«0x42».global_borrows.replace.verified,
      ``«0x42».global_borrows.read_whole.verified,
      ``«0x42».global_borrows.bump_through.verified,
      ``«0x42».local_borrows.bump_first.verified,
      ``«0x42».sibling_borrows.read_siblings.verified,
      ``«0x42».owned_borrows.bump_left.verified] do
    if (← collectAxioms name).contains ``sorryAx then
      throwError "generated borrow proof contains an admission: {name}"

end LeanerLang.Tests.VerificationGlobalBorrows
