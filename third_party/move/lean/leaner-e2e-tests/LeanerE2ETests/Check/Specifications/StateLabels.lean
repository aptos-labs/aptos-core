-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-!
# State labels

A quantified state label binds a memory and a copy of every mutable
reference parameter: an existential split of a two-state specification
function over two opaque calls, witnessed at the state after the first call;
a one-state read at a bound label; the same split over two inlined calls
through a `&mut` parameter, witnessed by the parameter's value at the point
between them.
-/

namespace LeanerLang.Tests.Check.Specifications.StateLabels

leaner module 0x42::state_labels where
  struct Counter has Key where
    value : u64

  fun bump(addr : Address) -> Unit := do
    let counter := &mut Counter[addr]
    counter.value := counter.value + 1
  spec bump where
    pragma opaque
    requires exists<Counter>(addr)
    requires global<Counter>(addr).value < 1000
    ensures exists<Counter>(addr)
    ensures global<Counter>(addr).value == old(global<Counter>(addr).value) + 1
    aborts_if false
    modifies global<Counter>(addr)

  spec fun increased(addr : Address) : Bool :=
    old(global<Counter>(addr).value) < global<Counter>(addr).value

  fun bump_twice(addr : Address) -> Unit := do
    bump(addr)
    bump(addr)
  spec bump_twice where
    requires exists<Counter>(addr)
    requires global<Counter>(addr).value < 500
    ensures ∃ (S : StateDomain), (..S |~ increased(addr)) && (S.. |~ increased(addr))
    ensures ∃ (S : StateDomain), (S |~ exists<Counter>(addr)) && (S.. |~ increased(addr))
    modifies global<Counter>(addr)

  fun bump_once(addr : Address) -> Unit := do
    bump(addr)
  spec bump_once where
    requires exists<Counter>(addr)
    requires global<Counter>(addr).value < 500
    ensures ∃ (S : StateDomain), (..S |~ increased(addr)) && (S.. |~ increased(addr)) -- error: one call, no split
    modifies global<Counter>(addr)

  fun inc(c : &mut Counter) -> Unit := do
    c.value := c.value + 1

  spec fun grew(before : Counter, after : Counter) : Bool := before.value < after.value

  fun inc_twice(c : &mut Counter) -> Unit := do
    inc(c)
    inc(c)
  spec inc_twice where
    requires c.value < 1000
    ensures ∃ (S : StateDomain), (..S |~ grew(old(c), c)) && (S.. |~ grew(old(c), c))

  fun inc_once(c : &mut Counter) -> Unit := do
    inc(c)
  spec inc_once where
    requires c.value < 1000
    ensures ∃ (S : StateDomain), (..S |~ grew(old(c), c)) && (S.. |~ grew(old(c), c)) -- error: one call, no split

end LeanerLang.Tests.Check.Specifications.StateLabels
