-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-!
# Callees, verified from LIR

Callees with a mutable reference parameter consumed through their
contracts, pure callees in value, condition, and nested positions, and
recursive callees.
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
    aborts_if old(*slot) + 1 > MAX_U64

  -- The live `&mut` parameter is passed on twice.
  public fun bump_twice(slot : &mut u64) -> Unit := do
    bump(slot)
    bump(slot)
  spec bump_twice where
    ensures *slot == old(*slot) + 2
    aborts_if old(*slot) + 2 > MAX_U64

  -- A callee whose exit value is not the entry plus one: the caller's
  -- proof takes the slot's new value from the callee's clause.
  public fun add_two(slot : &mut u64) -> Unit := do
    *slot := *slot + 2
  spec add_two where
    ensures *slot == old(*slot) + 2
    aborts_if old(*slot) + 2 > MAX_U64

  public fun bump_then_add_two(slot : &mut u64) -> Unit := do
    bump(slot)
    add_two(slot)
  spec bump_then_add_two where
    ensures *slot == old(*slot) + 3
    aborts_if old(*slot) + 3 > MAX_U64

  -- A global field borrow is passed to the callee.
  public entry fun bump_counter(addr : Address) -> Unit := do
    let value := &mut Counter[addr].value
    bump(value)
  spec bump_counter where
    requires exists<Counter>(addr)
    modifies global<Counter>(addr)
    ensures global<Counter>(addr).value == old(global<Counter>(addr).value) + 1
    aborts_if old(global<Counter>(addr).value) + 1 > MAX_U64

  -- The caller observes the reference before and after the call.
  public fun take_and_bump(slot : &mut u64) -> u64 := do
    let before := *slot
    bump(slot)
    before
  spec take_and_bump where
    ensures result == old(*slot) && *slot == old(*slot) + 1
    aborts_if old(*slot) + 1 > MAX_U64

  -- Multiple mutable parameters have independent prophecies.
  public fun set_pair(left : &mut u64, right : &mut u64) -> Unit := do
    *left := 10
    *right := 20
  spec set_pair where
    ensures *left == 10 && *right == 20
    aborts_if false

  public fun forward_set_pair(left : &mut u64, right : &mut u64) -> Unit := do
    set_pair(left, right)
  spec forward_set_pair where
    ensures *left == 10 && *right == 20
    aborts_if false

  -- ## Pure callees

  public fun plus_one(value : u64) -> u64 := value + 1

  public fun calls_pure_helper(value : u64) -> u64 := plus_one(value)
  spec calls_pure_helper where
    ensures result == value + 1
    aborts_if value + 1 > MAX_U64

  public fun pure_predicate(value : u64) -> Bool := value == 0

  -- A pure callee in a condition.
  public fun helper_condition(value : u64) -> u64 :=
    if pure_predicate(value) then 1 else 2
  spec helper_condition where
    ensures result == (if value == 0 then 1 else 2)
    aborts_if false

  -- Nested pure calls in an embedded position.
  public fun embedded_helper(value : u64) -> u64 := do
    let doubled := plus_one(plus_one(value))
    doubled
  spec embedded_helper where
    ensures result == value + 2
    aborts_if value + 2 > MAX_U64

/-! ## Recursive callees

A recursive callee verifies by induction; its callers use its contract. -/

leaner module 0x99::callees_recursive where
  public fun drain(slot : &mut u64) -> Unit := do
    let current := *slot
    if current > 0 then
      *slot := current - 1
      drain(slot)
  spec drain where
    ensures *slot == 0
    aborts_if false

  public fun call_drain(slot : &mut u64) -> Unit := do
    drain(slot)
  spec call_drain where
    ensures *slot == 0
    aborts_if false

  public fun sum_down(value : u64) -> u64 :=
    if value < 1 then 0 else value + sum_down(value - 1)

  public fun call_sum_down(value : u64) -> u64 := sum_down(value)
  spec call_sum_down where
    pragma verify = false
    ensures result >= value
    aborts_if false

-- An unspecified callee calling itself has no finite inlining.
/--
error: `call_sum_down` reaches `sum_down` again through callees that are inlined; only a function calling itself directly is verified by induction, so specify and verify the callees on the cycle
-/
#guard_msgs in
verify 0x99::callees_recursive::call_sum_down
