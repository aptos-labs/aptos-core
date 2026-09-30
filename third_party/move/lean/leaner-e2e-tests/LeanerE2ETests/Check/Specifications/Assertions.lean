-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-! In-body assertions are checked where they are stated, and what follows
may assume them: in straight-line code, under `old`, in a loop body, in an
inlined callee at each call, in a generic function, and over the state a
state anchor saved. -/

leaner module 0x42::assertions where
  struct Box {T has Copy, Drop} has Copy, Drop where
    value : T

  struct Counter has Key where
    value : u64

  fun difference(x : u64, y : u64) -> u64 := do
    if !(x > y) then abort(1)
    let z := x - y
    spec assert z > 0
    z
  spec difference where
    ensures result > 0

  fun successor(x : u64) -> u64 := do
    let y := x + 1
    spec assert y == old(x) + 1
    y
  spec successor where
    ensures result == x + 1

  fun count(n : u64) -> u64 := do
    let mut i := 0
    while i < n do
      spec assert i < n
      i := i + 1
    where
      invariant i <= n
    i
  spec count where
    ensures result == n

  fun checked(x : u64) -> u64 := do
    if x == 0 then abort(1)
    spec assert x > 0
    x

  fun calls_checked(x : u64) -> u64 := checked(x)
  spec calls_checked where
    ensures result == x

  fun unbox {T has Copy, Drop}(b : Box<T>) -> T := do
    spec assert b.value == b.value
    b.value
  spec unbox where
    ensures result == b.value

  fun bump_twice(a : Address) -> Unit := do
    let first := &mut Counter[a]
    first.value := first.value + 1
    spec assume save_state_anchor!(0)
    let second := &mut Counter[a]
    second.value := second.value + 1
    spec assert with_state_anchor!(0, global<Counter>(a).value == old(global<Counter>(a).value) + 1)
  spec bump_twice where
    requires exists<Counter>(a) && global<Counter>(a).value < 100
    ensures global<Counter>(a).value == old(global<Counter>(a).value) + 2
    modifies global<Counter>(a)
