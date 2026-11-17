-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL
import VerexSL.Tests.LoanDeathSupport

/-! ## The checks of a generic callee inlined on the generic route

A generic call is read at the callee's body compiled at the call's row where
the callee's conditions are carried there (`doc/plan/one-spelling.md`, D2).
A callee they are not carried for keeps the generic route: its body runs in
the caller's walk at the carriers the call's type arguments induce, and so
its checks are emitted at those carriers, while every fact of the caller's
context is stated at the caller's own. The normal form states such a check at
the outer carriers where the body's steps are exposed
(`checkingAtOuterFamily`), so the fact of the vector the caller passed, and
of that vector swapped and with an element removed, is the fact in hand.

The callee here is on the generic route as the Move standard library's
`vector::swap_remove` is: intrinsic in a module the prelude implements. The
controls call it at an opaque and at a declared element type, and hand it a
vector the caller made invalid.
-/

namespace VerexSL.Tests.InstantiatedCalleeFacts

set_option Elab.async false

open Lean Meta Elab VerexIR VerexSL

verex module 0x1::vector where
  pragma verify = false

  -- Intrinsic in a module the prelude implements: inlined, but on the generic
  -- route, at the carriers `T` induces (`carriesConditionsAtRow` is false).
  public fun swap_take {T}(items : &mut Vector<T>, index : u64) -> T := do
    let last := (*items).length - 1
    *items := core.prim.swapVector(*items, index, last)
    let (removed, rest) := core.prim.removeVector(*items, last)
    *items := rest
    removed
  spec swap_take where
    pragma intrinsic

verex module 0x42::instantiated_callee_facts where
  pragma verify = false

  struct Percent has Copy, Drop, Store where
    value : u64
  spec Percent where
    invariant value <= 100

  -- At an opaque element type of the caller.
  fun take_generic_ok {E}(v : &mut Vector<E>, index : u64) -> E :=
    0x1::vector::swap_take::<E>(v, index)
  spec take_generic_ok where
    requires index < v.length
    ensures v.length == old(v).length - 1

  -- At an element type that declares an invariant.
  fun take_percent_ok(v : &mut Vector<Percent>, index : u64) -> Percent :=
    0x1::vector::swap_take::<Percent>(v, index)
  spec take_percent_ok where
    requires index < v.length
    ensures v.length == old(v).length - 1
    ensures result.value <= 100

  -- The caller hands over a vector holding an element its type refuses.
  fun take_invalid_bad(v : &mut Vector<Percent>, index : u64) -> Percent := do
    (*v)[index].value := 101u64
    0x1::vector::swap_take::<Percent>(v, index)
  spec take_invalid_bad where
    requires index < v.length

/-! ### Negative control -/

open LoanDeathControls in
run_cmd do
  expectCuts `«0x42».instantiated_callee_facts.take_invalid_bad
    [("invariant value <= 100", handOver)]
    (← `(command| verify 0x42::instantiated_callee_facts::take_invalid_bad)) (some [])

/-! ### Positive controls -/

verify 0x42::instantiated_callee_facts::take_generic_ok
verify 0x42::instantiated_callee_facts::take_percent_ok

end VerexSL.Tests.InstantiatedCalleeFacts
