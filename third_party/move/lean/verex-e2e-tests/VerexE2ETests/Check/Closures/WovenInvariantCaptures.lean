-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexE2ETests.CheckSupport

/-!
# Behaviors of a closure that captures a value with a data invariant

A closure behaves as the function value naming its target at the woven
arguments (`WovenBehaviors`).  A behavior at the woven arguments is stated of
valid values, the captured one among them, so the equation holds of a capture
that holds its data invariant: the caller's own parameter, or a value packed
where its invariant is established.  A capture packed where the invariant is
not established fails at the pack, which is the obligation the capture owes.

The equation of a site whose captures state no invariant takes no premise
and is a normalization rule; the one of a site capturing a declared type
takes the captures' fact, and the leaf that holds the fact reads it.
-/

namespace VerexSL.Tests.Check.Closures.WovenInvariantCaptures

verex module 0x42::woven_invariant_captures where
  struct Percent has Copy, Drop where
    value : u64
  spec Percent where
    invariant value <= 100

  fun scale(p : Percent, x : u64) -> u64 := p.value * x
  spec scale where
    aborts_if p.value * x > MAX_U64
    ensures result == p.value * x

  fun add(x : u64, y : u64) -> u64 := x + y
  spec add where
    aborts_if x + y > MAX_U64
    ensures result == x + y

  -- The capture is the caller's parameter, whose invariant it holds.
  fun scaling(p : Percent) -> Fn(u64) -> u64 has Copy, Drop :=
    function[Fn(u64) -> u64 has Copy, Drop](scale, p)
  spec scaling where
    aborts_if false
    ensures ∀ (x : u64),
      aborts_of<result>(x) == aborts_of<function[Fn(Percent, u64) -> u64](scale)>(p, x)
    ensures ∀ (x : u64), ∀ (r : u64),
      ensures_of<result>(x, r) == ensures_of<function[Fn(Percent, u64) -> u64](scale)>(p, x, r)

  -- The capture is packed under the condition that establishes its invariant.
  fun scaling_by(v : u64) -> Fn(u64) -> u64 has Copy, Drop :=
    function[Fn(u64) -> u64 has Copy, Drop](scale, new Percent { value := v })
  spec scaling_by where
    requires v <= 100
    aborts_if false
    ensures ∀ (x : u64),
      aborts_of<result>(x) ==
        aborts_of<function[Fn(Percent, u64) -> u64](scale)>(new Percent { value := v }, x)

  -- A capture that states no invariant: the equation is unconditional.
  fun adding(x : u64) -> Fn(u64) -> u64 has Copy, Drop :=
    function[Fn(u64) -> u64 has Copy, Drop](add, x)
  spec adding where
    aborts_if false
    ensures ∀ (y : u64),
      aborts_of<result>(y) == aborts_of<function[Fn(u64, u64) -> u64](add)>(x, y)

  -- The captured value's invariant is not established where it is packed.
  fun scaling_unchecked(v : u64) -> Fn(u64) -> u64 has Copy, Drop :=
    function[Fn(u64) -> u64 has Copy, Drop](scale, new Percent { value := v }) -- error: the invariant of the captured `Percent`
  spec scaling_unchecked where
    aborts_if false

  -- The capture is not the target's first argument of the equation.
  fun scaling_other(p : Percent, q : Percent) -> Fn(u64) -> u64 has Copy, Drop :=
    function[Fn(u64) -> u64 has Copy, Drop](scale, p)
  spec scaling_other where
    aborts_if false
    ensures ∀ (x : u64),
      aborts_of<result>(x) == aborts_of<function[Fn(Percent, u64) -> u64](scale)>(q, x) -- error: another capture

end VerexSL.Tests.Check.Closures.WovenInvariantCaptures

/-! The generated equations of the two capturing sites: the closures of
`scale` share one site, whose equation takes the captures' validity as a
premise and is read by the leaf that holds it (`guardedWovenAt`); the site of
`add` captures an integer, and its equation takes none. -/

open Lean Meta in
run_meta do
  let environment ← getEnv
  let mut guarded : Nat := 0
  let mut plain : Nat := 0
  for (name, info) in environment.constants.map₂.toList do
    let .str family last := name | continue
    unless last.startsWith "wovenAborts" do continue
    let premised ← forallTelescope info.type fun binders _ =>
      binders.anyM fun binder => do
        return (← inferType binder).isAppOf ``VerexIR.Proofs.Denote.HList.Holds
    let site := Name.str family ("site" ++ (last.drop "wovenAborts".length).toString)
    let registered :=
      (VerexIR.Proofs.Denote.guardedWovenAt environment site).contains name
    unless premised == registered do
      throwError "the equation {name} takes the captures' fact: {premised};         it is the site's to read under that fact: {registered}"
    if premised then guarded := guarded + 1 else plain := plain + 1
  unless guarded == 1 && plain == 1 do
    throwError "expected one guarded and one unconditional woven equation, got       {guarded} and {plain}"
