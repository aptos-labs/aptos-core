-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexE2ETests.CheckSupport

/-!
# Behaviors of a target returning a mutable reference

A behavioral predicate reads a returned mutable reference as its referent's
value at return. A call whose returned reference dies at once is a run of
the function value naming the target: its `ensures_of`, its `result_of`,
and the absence of its aborts follow.
-/

namespace VerexSL.Tests.Check.Closures.MutableResults

verex module 0x42::mutable_results where
  struct Cell has Copy, Drop where
    value : u64

  fun increment(c : &mut Cell) -> &mut Cell := do
    c.value := c.value + 1
    c
  spec increment where
    aborts_if c.value == MAX_U64
    ensures c.value == old(c).value + 1

  fun bump(c : &mut Cell) -> Unit := do
    increment(c)
  spec bump where
    aborts_if c.value == MAX_U64
    ensures ensures_of<function[Fn(&mut Cell) -> &mut Cell](increment)>(
      old(c), result_of<function[Fn(&mut Cell) -> &mut Cell](increment)>(old(c)), c)
    ensures result_of<function[Fn(&mut Cell) -> &mut Cell](increment)>(old(c)) == c
    ensures !aborts_of<function[Fn(&mut Cell) -> &mut Cell](increment)>(old(c))

  fun bump_unchanged(c : &mut Cell) -> Unit := do
    increment(c)
  spec bump_unchanged where
    ensures result_of<function[Fn(&mut Cell) -> &mut Cell](increment)>(old(c)) == old(c) -- error: the result is the incremented cell

end VerexSL.Tests.Check.Closures.MutableResults
