-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexE2ETests.CheckSupport

/-!
# The frame of a function value whose result type states an invariant

A function-valued parameter's frame is a fact of its runs on valid arguments
to a valid result.  Where the result type states an invariant, here the
caller's own type parameter, the invocation's rule reads the frame at the
run with the result's fact, which the value's own assumption gives
(`ValidResultsAt`, `FramedAt.at_run`): the state the parameter keeps is kept
over the invocation.  A parameter whose frame is open keeps nothing.
-/

namespace VerexSL.Tests.Check.Closures.FrameResultValidity

verex module 0x42::frame_result_validity where
  struct Counter has Key where
    value : u64

  -- The parameter keeps global memory, and its result is of a type that may
  -- state an invariant.
  public fun keeps {R}(f : Fn(u64) -> R, x : u64, addr : Address) -> R := invoke(f, x)
  spec keeps where
    pragma aborts_if_is_partial
    ensures exists<Counter>(addr) == old(exists<Counter>(addr))
    ensures ensures_of<f>(x, result)

  -- The same invocation of a parameter whose frame is open.
  public fun keeps_open {R}(f : Fn(u64) -> R, x : u64, addr : Address) -> R := invoke(f, x)
  spec keeps_open where
    pragma aborts_if_is_partial
    modifies_of<f> *
    ensures exists<Counter>(addr) == old(exists<Counter>(addr)) -- error: `f` may change any memory
    modifies *

end VerexSL.Tests.Check.Closures.FrameResultValidity
