-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL
import VerexSL.Tests.LoanDeathSupport

/-! ## A program's reads of a literal vector

A clause reads a vector at a position as the total read
`v[k]?.getD fallback`, the fallback unspecified. A program's successful read
of a literal vector is spelled the same way where the walk splits it
(`literalReadElement`), so the element a loop body read and the element an
invariant states under its binder are one term at the instance, and the
step of a range invariant is decided at its bound without search. The
controls keep an invariant under its binder against a read through a place
and against an indexed read, read a literal at a symbolic position within it
and at a concrete one, and check what the spelling must not give: a value
at an unknown or missing position, the agreement of two fallbacks, a bound
the element's own facts do not state.
-/

namespace VerexSL.Tests.LiteralVectorReads

set_option Elab.async false

open Lean Meta Elab VerexIR VerexSL

/-! ### The spelling of a successful read, at the lookup's own carrier

The helper's scope is a typed literal lookup, an array's or a list's, whose
element type spells `NTy.carrier C τ`; there is no test of where the
container came from. A lookup of any other container is left as it is, with
the element's name and the read's equation. -/

section Spelling
open VerexIR.Proofs.Denote Lean.Elab.Tactic

/-- The goal `literalReadElement` leaves for the read a hypothesis states. -/
elab "spell_read " equation:ident : tactic => do
  let goal ← getMainGoal
  let read ← goal.withContext (getFVarId equation)
  replaceMainGoal [← literalReadElement goal (.fvar read)]

-- A literal array at an explicit carrier: the element is the clause's read
-- of that position, and the position within the array stays as a fact.
example [Carriers] (position : Nat) (a b c element : (NTy.int 64 false).carrier)
    (heq : (#[a, b, c])[position]? = some element) :
    (#[a, b, c])[position]?.getD ((NTy.int 64 false).plainFallback (by decide)) = element := by
  spell_read heq
  guard_hyp verexWithin : position < (#[a, b, c] : Array (NTy.int 64 false).carrier).size
  rfl

-- The list a literal lookup is normalized to, at the same carrier.
example [Carriers] (position : Nat) (a b c element : (NTy.int 64 false).carrier)
    (heq : [a, b, c][position]? = some element) :
    [a, b, c][position]?.getD ((NTy.int 64 false).plainFallback (by decide)) = element := by
  spell_read heq
  guard_hyp verexWithin : position < [a, b, c].length
  rfl

-- An ordinary array is no literal: the read keeps the element's name and the
-- equation that names it.
set_option linter.unusedVariables false in
example [Carriers] (position : Nat) (xs : Array (NTy.int 64 false).carrier)
    (element : (NTy.int 64 false).carrier) (heq : xs[position]? = some element) : True := by
  spell_read heq
  guard_hyp heq : xs[position]? = some element
  trivial

end Spelling

/-! ### Two fallbacks are not one off the domain -/

section Fallbacks
open VerexIR.Proofs.Denote

-- Negative: nothing states the position, and the two reads are not
-- identified.
set_option linter.unusedVariables false in
example (k : Nat) (first second : Int) : True := by
  fail_if_success
    have : (#[1, 4, 2] : Array Int)[k]?.getD first = (#[1, 4, 2] : Array Int)[k]?.getD second := by
      verex_normalize_context
      first | rfl | assumption | omega
  trivial

-- Negative: at a position the literal does not hold, the two defaults stay
-- apart.
set_option linter.unusedVariables false in
example (first second : Int) (distinct : first ≠ second) : True := by
  fail_if_success
    have : (#[1, 4, 2] : Array Int)[3]?.getD first = (#[1, 4, 2] : Array Int)[3]?.getD second := by
      verex_normalize_context
      first | rfl | assumption | omega
  trivial

-- Negative: at a position not known to be within the list, the reads with
-- two fallbacks stay two terms.
set_option linter.unusedVariables false in
example (k : Nat) (first second : Int) (xs : Array Int)
    (same : (#[1, 4, 2] : Array Int)[k]?.getD first = (#[1, 4, 2] : Array Int)[k]?.getD second) : True := by
  fail_if_success
    have : first = second := by
      verex_normalize_context
      first | assumption | omega
  trivial

-- A concrete position evaluates whatever the fallback is.
example (fallback : Int) : (#[1, 4, 2] : Array Int)[1]?.getD fallback = 4 := by
  simp only [vir_denote_norm]

-- A position past the literal reads the fallback, as stated.
example (fallback : Int) : (#[1, 4, 2] : Array Int)[3]?.getD fallback = fallback := by
  simp only [vir_denote_norm]

end Fallbacks

verex module 0x42::literal_vector_reads where
  pragma verify = false

  struct Percent has Copy, Drop, Store where
    value : u64
  spec Percent where
    invariant value <= 100

  -- The sum of the elements below `end`.
  spec fun total(v : Vector<u64>, end : u64) : Int :=
    if end == 0 then 0 else total(v, end - 1) + v[end - 1]

  -- The invariant states more of the elements than the body tested.
  fun all_small_bad() -> Bool := do
    let v := vector<u64>[1, 4, 2]
    let i := 0
    while i < v.length do
      let x := &v[i]
      if *x > 4 then return false;
      i := i + 1
    where
      invariant i <= v.length
      invariant ∀ (j in 0 .. i), v[j] <= 3
    true
  spec all_small_bad where
    aborts_if false

  -- A specification's read at a position nothing bounds states no value.
  fun unknown_position_bad(i : u64) -> u64 := 0
  spec unknown_position_bad where
    ensures vector<u64>[1, 4, 2][i] <= 4

  -- The element's facts are its own: nothing below what the literal holds.
  fun percent_bad(i : u64) -> u64 := do
    let v := vector<Percent>[new Percent { value := 50 }, new Percent { value := 70 }]
    v[i].value
  spec percent_bad where
    requires i < 2
    ensures result <= 60

  -- The invariant stays under its binder; the body reads through a place.
  fun all_small() -> Bool := do
    let v := vector<u64>[1, 4, 2]
    let i := 0
    while i < v.length do
      let x := &v[i]
      if *x > 4 then return false;
      i := i + 1
    where
      invariant i <= v.length
      invariant ∀ (j in 0 .. i), v[j] <= 4
    true
  spec all_small where
    aborts_if false

  -- The same against an indexed read, with a recursive specification of
  -- the accumulator.
  fun sum() -> u64 := do
    let v := vector<u64>[1, 2, 3]
    let acc := 0
    let i := 0
    while i < v.length do
      acc := acc + v[i]
      i := i + 1
    where
      invariant i <= v.length
      invariant acc == total(v, i)
      invariant ∀ (j in 0 .. i), !(total(v, j) + v[j] > MAX_U64)
    acc
  spec sum where
    aborts_if false
    ensures result == 6

  -- A symbolic position within the literal: the program's read is the
  -- specification's.
  fun at_position(i : u64) -> u64 := do
    let v := vector<u64>[1, 4, 2]
    v[i]
  spec at_position where
    aborts_if i >= 3
    ensures result == vector<u64>[1, 4, 2][i]
    ensures result <= 4

  -- A concrete position evaluates.
  fun at_one() -> u64 := do
    let v := vector<u64>[1, 4, 2]
    v[1]
  spec at_one where
    aborts_if false
    ensures result == 4

  -- An element carrying an invariant keeps it.
  fun percent(i : u64) -> u64 := do
    let v := vector<Percent>[new Percent { value := 50 }, new Percent { value := 70 }]
    v[i].value
  spec percent where
    requires i < 2
    aborts_if false
    ensures result <= 100

/-! ### Negative controls -/

open LoanDeathControls in
run_cmd do
  let diagnostics ← rejectedDiagnostics `«0x42».literal_vector_reads.all_small_bad
    (← `(command| verify 0x42::literal_vector_reads::all_small_bad))
  unless diagnostics.contains
      "the loop invariant `∀ (j in 0 .. i), v[j] <= 3` is not preserved by an iteration" do
    throwError "all_small_bad is rejected otherwise: {diagnostics}"
  expectUnestablished `«0x42».literal_vector_reads.unknown_position_bad
    ["ensures vector<u64>[1, 4, 2][i] <= 4"]
    (← `(command| verify 0x42::literal_vector_reads::unknown_position_bad))
  expectUnestablished `«0x42».literal_vector_reads.percent_bad ["ensures result <= 60"]
    (← `(command| verify 0x42::literal_vector_reads::percent_bad))

/-! ### Positive controls -/

verify 0x42::literal_vector_reads::all_small
verify 0x42::literal_vector_reads::sum
verify 0x42::literal_vector_reads::at_position
verify 0x42::literal_vector_reads::at_one
verify 0x42::literal_vector_reads::percent

end VerexSL.Tests.LiteralVectorReads
