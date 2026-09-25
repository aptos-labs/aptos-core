-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-!
# Storage-parametric V1 denotations

A type parameter which does not determine a global resource key is represented
by one abstract inhabited carrier.  It may still occur in values, signatures,
aggregates, and calls.  The generic bodies below are therefore denoted and
proved once; concrete callers reach the same denotation at distinct type
arguments.
-/

namespace LeanerLang.Tests.Check.Generics.GenericFunctions

open Lean Elab Command

leaner module 0x42::verification_generics where
  public fun carry {T}(value : T) -> T := value
  spec carry where
    pragma aborts_if_is_strict
    ensures result == value

  public fun carry_u64(value : u64) -> u64 :=
    core.call carry::<u64>(value)
  spec carry_u64 where
    pragma aborts_if_is_strict
    ensures result == value

  public fun carry_bool(value : Bool) -> Bool :=
    core.call carry::<Bool>(value)
  spec carry_bool where
    pragma aborts_if_is_strict
    ensures result == value

  -- Passing `T` to another generic function does not make it non-parametric.
  public fun forward {T}(value : T) -> T :=
    core.call carry::<T>(value)
  spec forward where
    pragma aborts_if_is_strict
    ensures result == value

  -- In contrast, `T` itself is the global resource family.  This function
  -- must be specialized for the reachable concrete instantiations.
  public fun has_resource {T has Key}(addr : Address) -> Bool :=
    exists<T>(addr)

  -- Storage-key dependence propagates through direct generic calls.
  public fun calls_has_resource {T has Key}(addr : Address) -> Bool :=
    core.call has_resource::<T>(addr)

  struct Items {T has Copy, Drop, Store} has Copy, Drop, Store where
    items : Vector<T>

  -- The loan on a generic field ends when its holder dies.
  public fun take {T has Copy, Drop, Store}(w : &mut Items<T>, i : u64) -> T := do
    let items := &mut w.items
    let (removed, rest) := core.prim.removeVector(*items, i)
    *items := rest
    removed
  spec take where
    requires i < w.items.length
    ensures w.items.length + 1 == old(w).items.length
    aborts_if false

end LeanerLang.Tests.Check.Generics.GenericFunctions
