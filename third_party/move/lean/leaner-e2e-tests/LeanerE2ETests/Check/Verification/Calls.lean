-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-!
# Calls between functions, verified from LIR

Ported from v0's `Verification/Calls.lean`: pure and effectful helpers,
callers consuming their callees' contracts, a `Bool`-driven choice, a
shared read, and the recursive helpers.  v0 also ran its `#test`
execution assertions over the lowered module; execution is left to the
differential suite.
-/

leaner module 0x99::calls where
  struct Counter has Key where
    value : u64

  public fun twice(value : u64) -> u64 := value + value
  spec twice where
    ensures result == value + value
    aborts_if value + value > 18446744073709551615

  public fun increment(value : u64) -> u64 := value + 1
  spec increment where
    ensures result == value + 1
    aborts_if value + 1 > 18446744073709551615

  -- Declaring no abort condition leaves abort behavior uninterpreted: any
  -- abort code is permitted, but the postcondition still has to hold for
  -- every successful execution.
  public fun increment_unspecified(value : u64) -> u64 := value + 1
  spec increment_unspecified where
    requires value + 1 <= 18446744073709551615
    ensures result == value + 1

  public fun pure_caller(value : u64) -> u64 := twice(value)

  public fun effect_caller(value : u64) -> u64 := increment(value)
  spec effect_caller where
    ensures result == value + 1
    aborts_if value + 1 > 18446744073709551615

  public fun composed(value : u64) -> u64 := do
    let doubled := twice(value)
    increment(doubled)

  public fun bound_caller(value : u64) -> u64 := do
    let incremented := increment(value)
    twice(incremented)

  /-- Minimal examples for compositional calls, independent of arithmetic so
  a failure points at call semantics. -/
  public fun choose(flag : Bool) -> u64 := if flag then 7 else 8
  spec choose where
    ensures result == (if flag then 7 else 8)
    aborts_if false

  public fun call_choose(flag : Bool) -> u64 := choose(flag)
  spec call_choose where
    ensures result == (if flag then 7 else 8)
    aborts_if false

  friend fun add_to(addr : Address, amount : u64) -> Unit := do
    let value := &mut Counter[addr].value
    *value := *value + amount
  spec add_to where
    requires exists<Counter>(addr)
    modifies global<Counter>(addr)
    ensures global<Counter>(addr).value == old(global<Counter>(addr).value) + amount
    aborts_if old(global<Counter>(addr).value) + amount > 18446744073709551615

  public entry fun add_twice(addr : Address, amount : u64) -> Unit :=
    add_to(addr, twice(amount))

  public entry fun add_twice_then_one(addr : Address, amount : u64) -> Unit := do
    add_twice(addr, amount)
    add_to(addr, 1)

  public fun read_counter(addr : Address) -> u64 := Counter[addr].value
  spec read_counter where
    requires exists<Counter>(addr)
    ensures result == old(global<Counter>(addr).value)
    aborts_if false

  public fun forwarded_read(addr : Address) -> u64 := read_counter(addr)

  verify twice
  verify increment
  verify increment_unspecified
  verify add_to
  verify effect_caller
  verify choose
  verify call_choose
  verify read_counter

/-! ## Recursive helpers

v0 declared these `partial` and verified them by hand through the
recursive contract. -/

leaner module 0x99::calls_recursive where
  public fun recursive_choose(done : Bool) -> u64 :=
    if done then 7 else recursive_choose(true)
  spec recursive_choose where
    ensures result == 7
    aborts_if false

  verify recursive_choose
