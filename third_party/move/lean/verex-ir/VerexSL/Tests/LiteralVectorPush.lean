-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL
import VerexSL.Tests.LoanDeathSupport

/-! ## An element appended to a literal vector

A program that builds a vector by appending to a literal holds, in the normal
form, the literal of its elements (`pushOfLiteral`), as a clause's
concatenation of literals does: a specification function applied to that
vector is applied to a literal, whose length and elements its unfolding
reads. The controls normalize appends to an empty and to a nonempty literal
with symbolic elements and a chain of them, to a literal that a second
normalization leaves; leave an append to a vector that is no literal; keep
the append to a full vector and the program's append without a result
apart; and verify a sum over a vector built by three appends through the
recursive specification of the sum.
-/

namespace VerexSL.Tests.LiteralVectorPush

set_option Elab.async false

open Lean Meta Elab Tactic VerexIR VerexSL

/-- The hypothesis states, on its left, a literal vector of that many
elements with no append left in it. -/
elab "assert_literal " hypothesis:ident count:num : tactic => withMainContext do
  let declaration ← getLocalDeclFromUserName hypothesis.getId
  let some (_, vector, _) := (← instantiateMVars declaration.type).eq?
    | throwError "{hypothesis} is no equation"
  let_expr VerexIR.SpecVector.mk _ values _ := vector
    | throwError "no literal vector: {vector}"
  let some (_, elements) := values.arrayLit? | throwError "no literal array: {values}"
  unless elements.length == count.getNat do
    throwError "a literal of {elements.length} elements: {vector}"
  if (vector.find? (·.isConstOf ``VerexIR.SpecVector.push)).isSome then
    throwError "an append is left: {vector}"

section Literals
open VerexIR.Proofs.Denote

variable {α : Type} (x y z : α)

-- An empty literal.
example (v : SpecVector α) (same : (⟨#[], by simp⟩ : SpecVector α).push x = v) :
    v.values = #[x] := by
  simp only [vir_denote_norm] at same
  assert_literal same 1
  subst same
  rfl

-- A literal with elements: the appended one is last, the others as they were.
example (v : SpecVector α) (same : (⟨#[x, y], by simp⟩ : SpecVector α).push z = v) :
    v.values = #[x, y, z] := by
  simp only [vir_denote_norm] at same
  assert_literal same 3
  subst same
  rfl

-- A chain of appends is one literal, which a second normalization leaves.
example (v : SpecVector α)
    (same : (((⟨#[], by simp⟩ : SpecVector α).push x).push y).push z = v) :
    v.values = #[x, y, z] := by
  simp only [vir_denote_norm] at same
  assert_literal same 3
  fail_if_success simp only [vir_denote_norm] at same
  subst same
  rfl

-- Negative: a vector that is no literal keeps its append, a variable and one
-- built from an array that is no literal alike.
set_option linter.unusedVariables false in
example (u v : SpecVector α) (same : u.push x = v) : True := by
  fail_if_success simp only [vir_denote_norm] at same
  trivial

set_option linter.unusedVariables false in
example (values : Array α) (bounded : values.size < 2 ^ 64) (v : SpecVector α)
    (same : (SpecVector.mk values bounded).push x = v) : True := by
  fail_if_success simp only [vir_denote_norm] at same
  trivial

-- The append to a full vector leaves it, and no grown vector exists there
-- for the evaluation to state.
example (values : Array α) (bounded : values.size < 2 ^ 64)
    (full : ¬values.size + 1 < 2 ^ 64) :
    (SpecVector.mk values bounded).push x = ⟨values, bounded⟩ ∧
      ¬(values.push x).size < 2 ^ 64 := by
  refine ⟨?_, by simpa only [Array.size_push] using full⟩
  simp only [SpecVector.push, dif_neg full]

end Literals

-- The program's append keeps its branch without a result: the evaluation is
-- of the vector a specification states, not of the program's step.
run_cmd do
  let some denotation := (← getEnv).find? ``VerexIR.Proofs.Denote.denote_push_direct
    | throwError "the denotation of a program's append is not stated"
  let states (name : Name) := (denotation.type.find? (·.isConstOf name)).isSome
  unless states ``SpecVector.push && states ``VerexIR.Proofs.Spec.bottom do
    throwError "a program's append denotes otherwise:\n{denotation.type}"

verex module 0x42::literal_vector_push where
  pragma verify = false

  -- The sum of the elements below `end`.
  spec fun total(v : Vector<u64>, end : Int) : Int :=
    if end <= 0 || v.length < end then 0 else total(v, end - 1) + v[end - 1]

  fun sum(v : &Vector<u64>) -> u64 := do
    let i := 0
    let sum := 0
    while v.length > i do
      let x := &v[i]
      sum := sum + *x
      i := i + 1
    where
      invariant i >= 0 && v.length >= i
      invariant sum == total(v, i)
    sum
  spec sum where
    ensures result == total(v, v.length)

  -- Another sum than the three appended elements'.
  fun sum_of_three_bad(x : u64, y : u64, z : u64) -> u64 := do
    let mut v := vector<u64>[]
    v := core.prim.pushVector(v, x)
    v := core.prim.pushVector(v, y)
    v := core.prim.pushVector(v, z)
    sum(&v)
  spec sum_of_three_bad where
    ensures result == x + y + y

  fun sum_of_three(x : u64, y : u64, z : u64) -> u64 := do
    let mut v := vector<u64>[]
    v := core.prim.pushVector(v, x)
    v := core.prim.pushVector(v, y)
    v := core.prim.pushVector(v, z)
    sum(&v)
  spec sum_of_three where
    ensures result == x + y + z

  -- A literal with elements as the base.
  fun sum_of_grown(x : u64, y : u64, z : u64) -> u64 := do
    let mut v := vector<u64>[x, y]
    v := core.prim.pushVector(v, z)
    sum(&v)
  spec sum_of_grown where
    ensures result == x + y + z

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».literal_vector_push.sum_of_three_bad ["ensures result == x + y + y"]
    (← `(command| verify 0x42::literal_vector_push::sum_of_three_bad))

verify 0x42::literal_vector_push::sum
verify 0x42::literal_vector_push::sum_of_three
verify 0x42::literal_vector_push::sum_of_grown

end VerexSL.Tests.LiteralVectorPush
