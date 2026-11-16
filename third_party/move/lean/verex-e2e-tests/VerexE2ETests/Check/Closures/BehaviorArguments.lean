-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

/-!
# The arguments of a behavior a leaf reads

Reading the behavior of a known function value through its target's theorem
owes that theorem's condition on the arguments of the invocation, the
captured ones among them: the target's contract states the data invariants of
the values it is applied to, and the leaf pays them from the facts it holds
of those values. What stays open is a residual of the argument it belongs
to, named after the callee the behavior belongs to.
-/

namespace VerexSL.Tests.Check.Closures.BehaviorArguments

verex module 0x99::behavior_arguments where
  struct Percent has Copy, Drop where
    value : u64
  spec Percent where
    invariant value <= 100

  -- A target whose contract states its captured argument's invariant, so
  -- that the condition on the behavior's arguments is not vacuous. The
  -- invocation's own rows stay scalar, which the rule for an unseen
  -- invocation needs.
  fun scale(p : Percent, x : u64) -> u64 := p.value * x
  spec scale where
    aborts_if p.value * x > MAX_U64
    ensures result == p.value * x

  fun apply(f : Fn(u64) -> u64, x : u64) -> u64 := invoke(f, x)
  spec apply where
    ensures ensures_of<f>(x, result)

  -- The leaf reads the closure's behavior through `scale`'s theorem: the
  -- captured argument is the caller's own parameter, whose invariant it
  -- holds, so the condition is paid and the reading gives the result.
  fun apply_scale(p : Percent, x : u64) -> u64 :=
    apply(function[Fn(u64) -> u64](scale, p), x)
  spec apply_scale where
    pragma aborts_if_is_partial
    ensures result == p.value * x

end VerexSL.Tests.Check.Closures.BehaviorArguments
