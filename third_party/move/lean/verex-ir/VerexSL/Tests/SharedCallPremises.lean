-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

/-! A call in a shared suffix owes what the callee's theorem assumes, such as
a module axiom. The context states the axiom whole; under a summary it closes
at the call as on the tree walk, instead of being introduced and reaching a
leaf that opens the entry relations to prove an arrival-independent fact. -/

set_option Elab.async false

-- The same fact can reach a leaf from a different source clause. Its
-- location, including nested markers, does not change its proposition.
open VerexIR.Proofs in
example (p : Prop) (h : Obligation "callee" 1 2 (Obligation "inner" 3 4 p)) :
    Obligation "caller" 5 6 p := by
  verex_assumption

-- Different propositions still do not match, and a failed attempt leaves
-- the original marker intact for diagnostics.
open Lean Meta Elab Tactic VerexIR.Proofs in
example (p q : Prop) (_h : Obligation "callee" 1 2 p) (hq : q) :
    Obligation "caller" 5 6 q := by
  fail_if_success (clear hq; verex_assumption)
  run_tac
    unless (← (← getMainGoal).getType).isAppOfArity ``Obligation 4 do
      throwError "failed assumption matching removed the source marker"
  exact Obligation.intro hq

open Lean Elab Command in
/-- The target verifies with the given number of leaves and no opened entry
relation, its join sharing one summary. -/
elab "#guard_held " leaves:num " in " command:command : command => do
  let environment ← getEnv
  let (output, ()) ← IO.FS.withIsolatedStreams (isolateStderr := false) <|
    elabCommand (← `(command|
      set_option verex.sharedContinuations true in
      set_option verex.solverProfile true in $command))
  setEnv environment
  let lines := output.splitOn "\n"
  let profiles := lines.filter (·.startsWith "profile ")
  unless profiles.length == 1 &&
      profiles.all (·.endsWith s!" {leaves.getNat} leaves; stages") do
    throwError "unexpected leaves under the summary: {profiles}"
  let exposures := lines.filter (·.startsWith "  step join exposure")
  unless exposures.isEmpty do
    throwError "a leaf opened the entry relations: {exposures}"
  unless (lines.filter (·.startsWith "  joins:")).any (·.endsWith "summaries 1, proofs 1") do
    throwError "the join did not share its suffix: {lines.filter (·.startsWith "  joins:")}"

verex module 0x42::shared_call_premises where
  pragma verify = false

  opaque spec fun rank(i : Int) : Int

  spec module where
    axiom ∀ (i : Int; j : Int), i < j ==> rank(i) < rank(j)

  -- Verified and opaque: a call applies its theorem, which assumes the
  -- module's axiom.
  fun ordered(i : u64, j : u64) -> Bool := i < j
  spec ordered where
    pragma opaque
    aborts_if false
    ensures result ==> rank(i) < rank(j)

  -- Both arrivals at the join assign a parameter: one class, one summary.
  -- The shared suffix calls `ordered`, owing the axiom at the call.
  fun after_join(b : Bool, a : u64, c : u64) -> Bool := do
    let mut x : u64 := 0
    if b then x := a else x := c
    ordered(x, 200)

  -- The same, owing its abort clause on the callee's abort path, whose
  -- context states `False` under the callee's clause marker.
  fun after_join_aborts(b : Bool, a : u64, c : u64) -> Bool := do
    let mut x : u64 := 0
    if b then x := a else x := c
    ordered(x, 200)
  spec after_join_aborts where
    aborts_if false

verify 0x42::shared_call_premises::ordered

-- The axiom closes at the call: no leaf.
#guard_held 0 in
verify 0x42::shared_call_premises::after_join

-- The abort clause closes from the stated `False` through its source marker,
-- before reaching the leaf pipeline or opening an arrival.
#guard_held 0 in
verify 0x42::shared_call_premises::after_join_aborts


-- A direct interface call and a verified callee share the same native
-- dependency. The solver protects the direct callee's hypothesis as an
-- implementation detail; the verified callee must carry it explicitly.

verex module 0x1::call_premise_error where
  pragma verify = false
  public fun canonical(category : u64, reason : u64) -> u64 := (category << 16) + reason
  spec canonical where
    pragma opaque
    ensures [concrete] result == (category << 16) + reason
    aborts_if [abstract] false
    ensures [abstract] result == category
  public fun wrapped(x : u64) -> u64 := canonical(1, x)

verex module 0x2::call_premise_dependencies where
  use 0x1::call_premise_error::wrapped
  public fun first(x : u64) -> u64 := do
    assert!(x > 0, wrapped(1))
    x
  spec first where
    pragma opaque
    aborts_if x == 0
    ensures result == x
  public fun caller(x : u64) -> u64 := do
    assert!(x > 0, wrapped(2))
    first(x)
  spec caller where
    aborts_if x == 0
    ensures result == x
