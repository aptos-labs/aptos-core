-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-!
# Core primitives under verification

Port of v0's `Verification/CorePrimitives.lean`. The Leaner surface lowers
these borrow, read, write, abort, and checked vector-index forms to the same
core LIR operations exercised by the original test.
-/

namespace LeanerLang.Tests.VerificationCorePrimitives

leaner module 0x42::core_primitives where
  struct Counter has Key where
    value : u64

  public entry fun explicit_read_write(addr : Address, amount : u64) -> Unit := do
    let counter := &mut Counter[addr]
    let slot := &mut counter.value
    let previous := *slot
    *slot := previous + amount

  spec explicit_read_write where
    requires exists<Counter>(addr)
    modifies global<Counter>(addr)
    ensures global<Counter>(addr).value == old(global<Counter>(addr).value) + amount
    aborts_if old(global<Counter>(addr).value) + amount > 18446744073709551615

  public fun explicit_local(mut value : u64) -> u64 := do
    let slot := &mut value
    *slot := 7
    let view := &*slot
    return *view

  spec explicit_local where
    ensures result == 7
    aborts_if false

  public fun explicit_abort(value : u64) -> u64 := do
    if value < 10 then abort(4)
    return value

  spec explicit_abort where
    ensures result == value
    aborts_if value < 10 with 4

  public fun vector_get(index : u64) -> u64 := do
    let values : Vector<u64> := vector<u64>[10, 20, 30]
    return values[index]

  spec vector_get where
    ensures index == 0 ==> result == 10
    ensures index == 1 ==> result == 20
    ensures index == 2 ==> result == 30
    aborts_if index >= 3

  public fun vector_set(index : u64) -> Vector<u64> := do
    let mut values : Vector<u64> := vector<u64>[10, 20, 30]
    values[index] := 7
    return values

  spec vector_set where
    ensures index == 0 ==> result == vector<u64>[7, 20, 30]
    ensures index == 1 ==> result == vector<u64>[10, 7, 30]
    ensures index == 2 ==> result == vector<u64>[10, 20, 7]
    aborts_if index >= 3

  verify explicit_read_write
  verify explicit_local
  verify explicit_abort
  verify vector_get
  verify vector_set

end LeanerLang.Tests.VerificationCorePrimitives
