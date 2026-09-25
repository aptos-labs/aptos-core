-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-!
# Global and nested borrows

Six verified functions: whole-resource global borrows, a nested field borrow,
vector-element borrowing through a mutable parameter, disjoint sibling
borrows, and an element-field borrow of an owned vector.
-/

namespace LeanerLang.Tests.Check.Storage.GlobalBorrows

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

  public fun read_whole(addr : Address) -> u64 := do
    let counter := &Counter[addr]
    let current := *counter
    current.value
  spec read_whole where
    requires exists<Counter>(addr)
    ensures result == old(global<Counter>(addr).value)
    aborts_if false

  public entry fun bump_through(addr : Address) -> Unit := do
    let counter := &mut Counter[addr]
    let value := &mut counter.value
    *value := *value + 1
  spec bump_through where
    requires exists<Counter>(addr)
    modifies global<Counter>(addr)
    ensures global<Counter>(addr).value == old(global<Counter>(addr).value) + 1
    aborts_if old(global<Counter>(addr).value) + 1 > MAX_U64

leaner module 0x42::local_borrows where
  public fun bump_first(values : &mut Vector<u64>) -> Unit := do
    let first := &mut values[0]
    *first := *first + 1
  spec bump_first where
    requires values.length > 0
    ensures values[0] == old(values)[0] + 1
    aborts_if values[0] + 1 > MAX_U64

leaner module 0x42::sibling_borrows where
  struct Pair has Copy, Drop, Store where
    left : u64
    right : u64

  public fun read_siblings(pair : &mut Pair) -> u64 := do
    let left := &mut pair.left
    let right := &mut pair.right
    let right_value := *right
    let left_value := *left
    left_value + right_value
  spec read_siblings where
    ensures result == pair.left + pair.right
    aborts_if pair.left + pair.right > MAX_U64

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
    value.left
  spec bump_left where
    ensures result == 6
    aborts_if false

end LeanerLang.Tests.Check.Storage.GlobalBorrows
