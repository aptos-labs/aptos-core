-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-! Reject assertions that do not hold where they are stated: in a function
with or without a specification, in a loop body, in an inlined callee at a
call, in a generic function, and over the state a state anchor saved. -/

leaner module 0x42::negative_assertions where
  struct Counter has Key where
    value : u64

  fun unspecified(x : u64, y : u64) -> Unit := do
    if !(x > y) then abort(1)
    spec assert x == y

  fun specified(x : u64, y : u64) -> Unit := do
    if !(x > y) then abort(1)
    spec assert x == y
  spec specified where
    ensures true

  fun count(n : u64) -> u64 := do
    let mut i := 0
    while i < n do
      spec assert i + 1 < n
      i := i + 1
    where
      invariant i <= n
    i
  spec count where
    ensures result == n

  fun positive(x : u64) -> u64 := do
    spec assert x > 0
    x
  spec positive where
    pragma verify = false

  fun calls_positive(x : u64) -> u64 := positive(x)
  spec calls_positive where
    ensures result == x

  fun same {T has Copy, Drop}(value : T, other : T) -> T := do
    spec assert value == other
    value
  spec same where
    ensures result == value

  fun bump_once(a : Address) -> Unit := do
    spec assume save_state_anchor!(0)
    let _ := a
    spec assert with_state_anchor!(0, global<Counter>(a).value == old(global<Counter>(a).value) + 1)
  spec bump_once where
    requires exists<Counter>(a)
