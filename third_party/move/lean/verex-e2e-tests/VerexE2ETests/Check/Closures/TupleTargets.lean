-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

/-!
# Function values of tuple-returning functions

A function returning a tuple is not a closure target: a function type states
its results as a row, and a result shape holds one type
(`doc/designs/higher-order-functions.md`). A function value of one is reported
where it is constructed, naming a shared reference the tuple holds, and the
functions around it verify. A direct call of such a function stays supported.
-/

namespace VerexSL.Tests.Check.Closures.TupleTargets

verex module 0x99::tuple_targets where
  public fun pair(x : u64) -> (u64, u64) := (x, x)
  spec pair where
    aborts_if false

  public fun with_reference(x : &u64) -> (u64, &u64) := (*x, x)
  spec with_reference where
    aborts_if false

  fun of_pair() -> Unit := do
    let _f := function[Fn(u64) -> (u64, u64) has Copy, Drop](pair)
    ()
  spec of_pair where
    aborts_if false

  fun of_reference() -> Unit := do
    let _f := function[Fn(&u64) -> (u64, &u64) has Copy, Drop](with_reference)
    ()
  spec of_reference where
    aborts_if false

  fun direct(x : u64) -> u64 := do
    let (first, _second) := with_reference(&x)
    first
  spec direct where
    aborts_if false

end VerexSL.Tests.Check.Closures.TupleTargets
