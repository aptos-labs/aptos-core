-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-!
# Callees, verified from LIR

Ported from v0's `Verification/Callees.lean`: callees with a mutable
reference parameter consumed through their contracts, pure callees in
value, condition, and nested positions, and the recursive callees v0
verified by hand.  v0's `#test` execution assertions are left to the
differential suite.
-/

leaner module 0x99::callees where
  struct Counter has Key where
    value : u64

  struct PairValues has Copy, Drop, Store where
    left : u64
    right : u64

  -- ## Callees with a mutable-reference parameter

  public fun bump(slot : &mut u64) -> Unit := do
    *slot := *slot + 1
  spec bump where
    ensures *slot == old(*slot) + 1
    aborts_if old(*slot) + 1 > 18446744073709551615
  verify bump

  -- The live `&mut` parameter is passed on twice.
  public fun bump_twice(slot : &mut u64) -> Unit := do
    bump(slot)
    bump(slot)
  spec bump_twice where
    ensures *slot == old(*slot) + 2
    aborts_if old(*slot) + 2 > 18446744073709551615
  verify bump_twice

  -- A callee whose exit value is not the entry plus one: the caller's
  -- proof takes the slot's new value from the callee's clause.
  public fun add_two(slot : &mut u64) -> Unit := do
    *slot := *slot + 2
  spec add_two where
    ensures *slot == old(*slot) + 2
    aborts_if old(*slot) + 2 > 18446744073709551615
  verify add_two

  public fun bump_then_add_two(slot : &mut u64) -> Unit := do
    bump(slot)
    add_two(slot)
  spec bump_then_add_two where
    ensures *slot == old(*slot) + 3
    aborts_if old(*slot) + 3 > 18446744073709551615
  verify bump_then_add_two

  -- A global field borrow is passed to the callee.
  public entry fun bump_counter(addr : Address) -> Unit := do
    let value := &mut Counter[addr].value
    bump(value)
  spec bump_counter where
    requires exists<Counter>(addr)
    modifies global<Counter>(addr)
    ensures global<Counter>(addr).value == old(global<Counter>(addr).value) + 1
    aborts_if old(global<Counter>(addr).value) + 1 > 18446744073709551615
  verify bump_counter

  -- The caller observes the reference before and after the call.
  public fun take_and_bump(slot : &mut u64) -> u64 := do
    let before := *slot
    bump(slot)
    before
  spec take_and_bump where
    ensures result == old(*slot) && *slot == old(*slot) + 1
    aborts_if old(*slot) + 1 > 18446744073709551615
  verify take_and_bump

  -- Multiple mutable parameters have independent prophecies.
  public fun set_pair(left : &mut u64, right : &mut u64) -> Unit := do
    *left := 10
    *right := 20
  spec set_pair where
    ensures *left == 10 && *right == 20
    aborts_if false
  verify set_pair

  public fun forward_set_pair(left : &mut u64, right : &mut u64) -> Unit := do
    set_pair(left, right)
  spec forward_set_pair where
    ensures *left == 10 && *right == 20
    aborts_if false
  verify forward_set_pair

  -- ## Pure callees

  public fun plus_one(value : u64) -> u64 := value + 1

  public fun calls_pure_helper(value : u64) -> u64 := plus_one(value)
  spec calls_pure_helper where
    ensures result == value + 1
    aborts_if value + 1 > 18446744073709551615
  verify calls_pure_helper

  public fun pure_predicate(value : u64) -> Bool := value == 0

  -- A pure callee in a condition.
  public fun helper_condition(value : u64) -> u64 :=
    if pure_predicate(value) then 1 else 2
  spec helper_condition where
    ensures result == (if value == 0 then 1 else 2)
    aborts_if false
  verify helper_condition

  -- Nested pure calls in an embedded position.
  public fun embedded_helper(value : u64) -> u64 := do
    let doubled := plus_one(plus_one(value))
    doubled
  spec embedded_helper where
    ensures result == value + 2
    aborts_if value + 2 > 18446744073709551615
  verify embedded_helper

/-! ## Recursive callees

v0 declared these `partial` and verified them by hand through the
recursive contract; a caller of a verified recursive callee was then
automatic. -/

leaner module 0x99::callees_recursive where
  public fun drain(slot : &mut u64) -> Unit := do
    let current := *slot
    if current > 0 then
      *slot := current - 1
      drain(slot)
  spec drain where
    ensures *slot == 0
    aborts_if false
  verify drain

  public fun call_drain(slot : &mut u64) -> Unit := do
    drain(slot)
  spec call_drain where
    ensures *slot == 0
    aborts_if false
  verify call_drain

  public fun sum_down(value : u64) -> u64 :=
    if value < 1 then 0 else value + sum_down(value - 1)
