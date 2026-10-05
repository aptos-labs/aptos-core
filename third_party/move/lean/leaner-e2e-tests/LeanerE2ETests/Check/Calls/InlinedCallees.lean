-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-!
# Callees reasoned about through their bodies

A caller reasons about a callee through its body, specified or not, unless
the callee is `opaque`: its specification then stands for it. An inlined
callee brings its loops along with their invariants, and an invariant's
`old` reads the callee's arguments at the call.
-/

namespace LeanerLang.Tests.Check.Calls.InlinedCallees

leaner module 0x45::inlined_callees where
  public fun count_to(limit : u64) -> u64 := do
    let mut current : u64 := 0
    loop if current < limit then do
      current := current + 1
    else break
    spec do
      invariant current <= limit
    current
  spec count_to where
    ensures result == limit
    aborts_if false

  -- The invariant's `old` reads the argument `start` of each call.
  public fun raise_to(start : u64, limit : u64) -> u64 := do
    let mut value := start
    loop if value < limit then do
      value := value + 1
    else break
    spec do
      invariant old(start) <= value && (value <= limit || value == old(start))
    value
  spec raise_to where
    ensures result >= start && result >= limit
    aborts_if false

  -- Both loops are met inlined, each with its invariant.
  public fun count_then_raise(limit : u64) -> u64 := do
    let counted := count_to(limit)
    raise_to(counted, limit)
  spec count_then_raise where
    ensures result == limit
    aborts_if false

  -- Two calls of one callee in sequence, each starting from its arguments.
  public fun raise_twice(first : u64, second : u64) -> u64 := do
    let low := raise_to(first, second)
    raise_to(low, first)
  spec raise_twice where
    ensures result >= first && result >= second
    aborts_if false

  -- Nested calls of one callee: each loop's `old(start)` is its own call's
  -- argument, the inner call's `a` and the outer call's `a + 1`.
  public fun add_steps(start : u64, steps : u64) -> u64 := do
    let mut value := start
    let mut i : u64 := 0
    loop if i < steps then do
      value := value + 1
      i := i + 1
    else break
    spec do
      invariant i <= steps && value == old(start) + i
    value
  spec add_steps where
    ensures result == start + steps
    aborts_if start + steps > MAX_U64

  public fun add_three(a : u64) -> u64 := add_steps(add_steps(a, 1), 2)
  spec add_three where
    ensures result == a + 3
    aborts_if a + 3 > MAX_U64

  -- An opaque callee stands for its specification: `twice` claims only
  -- that the result is even, so the caller cannot learn the value.
  public fun twice(value : u64) -> u64 := value * 2
  spec twice where
    pragma opaque
    ensures result % 2 == 0
    aborts_if value * 2 > MAX_U64

  public fun twice_of_one() -> u64 := twice(1)
  spec twice_of_one where
    ensures result == 2 -- error: an opaque callee's body is not seen
    aborts_if false

end LeanerLang.Tests.Check.Calls.InlinedCallees
