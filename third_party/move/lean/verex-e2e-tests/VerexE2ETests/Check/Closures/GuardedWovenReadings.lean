-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexE2ETests.CheckSupport

/-!
# Several guarded behaviors in one clause

A closure capturing a value with a data invariant states its behaviors at
its target's woven arguments under the captures' fact, which the leaf that
holds the fact reads (`WovenInvariantCaptures`).  A clause may state the
behaviors of several such closures in one statement, an equivalence or a
disjunction, which reaches the leaf whole: every one of them is read, each
at its own captures, and a closure whose captures the leaf holds no fact of
keeps its behavior without keeping the others from being read.

A site states these behaviors at the value naming its target, so each
target is named by a clause.

Negatives first: two closures of one target that fix different arguments
behave differently at the same supplied argument.
-/

namespace VerexSL.Tests.Check.Closures.GuardedWovenReadings

verex module 0x42::guarded_woven_readings where
  struct Percent has Copy, Drop where
    value : u64
  spec Percent where
    invariant value <= 100

  fun scale(p : Percent, x : u64) -> u64 := p.value * x
  spec scale where
    aborts_if p.value * x > MAX_U64
    ensures result == p.value * x

  fun combine(a : Percent, b : Percent, x : u64) -> u64 := a.value + b.value + x
  spec combine where
    aborts_if a.value + b.value + x > MAX_U64
    ensures result == a.value + b.value + x

  struct Pair has Copy, Drop where
    first : Fn(Percent, u64) -> u64 has Copy, Drop
    second : Fn(Percent, u64) -> u64 has Copy, Drop

  -- Both closures are read, and their targets' arguments differ.
  fun fixing_other(p : Percent, q : Percent) -> Pair :=
    new Pair {
      first := function[Fn(Percent, u64) -> u64 has Copy, Drop](combine, p, _, _),
      second := function[Fn(Percent, u64) -> u64 has Copy, Drop](combine, _, q, _) }
  spec fixing_other where
    aborts_if false
    ensures ∀ (x : u64),
      aborts_of<result.first>(p, x) == aborts_of<result.second>(p, x) -- error: other arguments

  -- Two sites of one target, each fixing one of the caller's parameters: the
  -- equivalence states both behaviors, and both are the target's at the same
  -- arguments.
  fun fixing(p : Percent, q : Percent) -> Pair :=
    new Pair {
      first := function[Fn(Percent, u64) -> u64 has Copy, Drop](combine, p, _, _),
      second := function[Fn(Percent, u64) -> u64 has Copy, Drop](combine, _, q, _) }
  spec fixing where
    aborts_if false
    ensures ∀ (x : u64),
      aborts_of<result.first>(q, x) == aborts_of<result.second>(p, x)
    ensures ∀ (x : u64),
      aborts_of<result.first>(q, x) ==
        aborts_of<function[Fn(Percent, Percent, u64) -> u64](combine)>(p, q, x)

  -- The first closure the clause states captures a value the leaf holds no
  -- fact of; the second captures the caller's parameter and is read.
  fun scaling_beside(v : u64, q : Percent) -> Fn(u64) -> u64 has Copy, Drop :=
    function[Fn(u64) -> u64 has Copy, Drop](scale, q)
  spec scaling_beside where
    aborts_if false
    ensures ∀ (x : u64),
      aborts_of<function[Fn(u64) -> u64 has Copy, Drop](scale, new Percent { value := v })>(x) ||
        (aborts_of<result>(x) == aborts_of<function[Fn(Percent, u64) -> u64](scale)>(q, x))

end VerexSL.Tests.Check.Closures.GuardedWovenReadings

/-! Each of the three capturing sites states its behaviors under its
captures' fact, so none is passed over as a site with nothing to read. -/

open Lean Meta in
run_meta do
  let environment ← getEnv
  let mut guarded : Nat := 0
  for (name, _) in environment.constants.map₂.toList do
    let .str family last := name | continue
    unless last.startsWith "wovenAborts" do continue
    let site := Name.str family ("site" ++ (last.drop "wovenAborts".length).toString)
    if (VerexIR.Proofs.Denote.guardedWovenAt environment site).contains name then
      guarded := guarded + 1
  unless guarded == 3 do
    throwError "expected three sites stating their behaviors under their captures' fact,       got {guarded}"
