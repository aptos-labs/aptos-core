-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL
import VerexSL.Tests.LoanDeathSupport

/-! ## A clause's concatenation with the literal of one element

A clause states the append of one element as `concat(x, vec(e))`. Its
translation (`nativeVectorConcat`) is the vector a program's append of that
element denotes, `SpecVector.push x e`, so a recursive specification
function that collects by concatenation unfolds to the term a loop body's
append produces. The controls check the equation the choice rests on in
both of its branches, the translated form of each kind of concatenation,
and the agreement of a program's appends with the clauses stating them:
with a symbolic accumulator, an element carrying an invariant, a chain of
appends, an append read back, and a loop against the recursive collection.

Such an append may stand inside a nest of concatenations. The values of a
nest of both are read as one concatenation under the one bound on the sum of
its steps' lengths (`valuesOfNest`), an appended element counting one; the
controls state that bound in its spelling, withhold it, and read a single
append by its own rule.
-/

namespace VerexSL.Tests.ClauseSingletonConcat

set_option Elab.async false

open Lean Meta Elab VerexIR VerexSL

/-! ### The equation, by branch -/

-- An empty vector: the literal of the element.
example (element : Nat) (bounded : #[element].size < 2 ^ 64) :
    ((⟨#[], by decide⟩ : SpecVector Nat).append ⟨#[element], bounded⟩).values = #[element] := by
  rw [VerexSL.Contract.append_singleton_eq_push]
  rfl

-- A vector with room: one element more, the appended one last.
example (vector : SpecVector Nat) (element : Nat) (bounded : #[element].size < 2 ^ 64)
    (room : vector.values.size + 1 < 2 ^ 64) :
    (vector.append ⟨#[element], bounded⟩).values = vector.values.push element := by
  rw [VerexSL.Contract.append_singleton_eq_push, SpecVector.values_push element room]

-- A full vector: the concatenation and the append both leave it as it is.
example (vector : SpecVector Nat) (element : Nat) (bounded : #[element].size < 2 ^ 64)
    (full : ¬vector.values.size + 1 < 2 ^ 64) :
    vector.append ⟨#[element], bounded⟩ = vector ∧ vector.push element = vector := by
  rw [VerexSL.Contract.append_singleton_eq_push]
  simp only [SpecVector.push, dif_neg full, and_self]

-- The program's append keeps both of its branches: the vector with the
-- element where the length is representable, and no result where it is not,
-- which the equation of the two pure operations does not replace.
run_cmd do
  let some denotation := (← getEnv).find? ``VerexIR.Proofs.Denote.denote_push_direct
    | throwError "the denotation of a program's append is not stated"
  let states (name : Name) := (denotation.type.find? (·.isConstOf name)).isSome
  unless states ``SpecVector.push && states ``VerexIR.Proofs.Spec.bottom &&
      !states ``SpecVector.append do
    throwError "a program's append denotes otherwise:\n{denotation.type}"

/-! ### The values of a nest

The premise of a nest is the sum of its steps' lengths in their order, as
integers, an appended element as `1`: the discharger here is `assumption`,
so each hypothesis is that premise as spelled. -/

section Nests
open VerexIR.Proofs.Denote

variable (a b c : SpecVector Nat) (e f : Nat) (xs : Array Nat)

-- An appended element inside a nest.
example (bound : (a.values.size : Int) + 1 + (b.values.size : Int) ≤ 18446744073709551615) :
    ((a.push e).append b).values = a.values.push e ++ b.values := by
  simp (disch := assumption) only [vir_denote_norm]

-- An appended element on top of a nest.
example (bound : (a.values.size : Int) + (b.values.size : Int) + 1 ≤ 18446744073709551615) :
    ((a.append b).push e).values = (a.values ++ b.values).push e := by
  simp (disch := assumption) only [vir_denote_norm]

-- Concatenations alone, and appended elements alone.
example (bound : (a.values.size : Int) + (b.values.size : Int) + (c.values.size : Int) ≤
      18446744073709551615) :
    ((a.append b).append c).values = a.values ++ b.values ++ c.values := by
  simp (disch := assumption) only [vir_denote_norm]

example (bound : (a.values.size : Int) + 1 + 1 ≤ 18446744073709551615) :
    ((a.push e).push f).values = (a.values.push e).push f := by
  simp (disch := assumption) only [vir_denote_norm]

-- The steps in another order.
example (bound : (a.values.size : Int) + 1 + (b.values.size : Int) + 1 ≤ 18446744073709551615) :
    (((a.push e).append b).push f).values = (a.values.push e ++ b.values).push f := by
  simp (disch := assumption) only [vir_denote_norm]

-- Two appended elements whose rooms are known one by one and whose sum is
-- not stated: each by its own rule, to the same values.
example (first : a.values.size + 1 < 2 ^ 64) (second : (a.push e).values.size + 1 < 2 ^ 64) :
    ((a.push e).push f).values = (a.values.push e).push f := by
  simp (disch := assumption) only [vir_denote_norm]

-- Negative: without the bound the nest stays, also where its first append
-- alone has room.
set_option linter.unusedVariables false in
example (same : ((a.push e).append b).values = xs) : True := by
  fail_if_success simp (disch := omega) only [vir_denote_norm] at same
  trivial

set_option linter.unusedVariables false in
example (room : a.values.size + 1 < 2 ^ 64) (same : ((a.push e).append b).values = xs) :
    True := by
  fail_if_success simp (disch := omega) only [vir_denote_norm] at same
  trivial

-- Negative: a single append is no nest. The sum in the nest's spelling does
-- not read it; its own room does.
set_option linter.unusedVariables false in
example (bound : (a.values.size : Int) + 1 ≤ 18446744073709551615)
    (same : (a.push e).values = xs) : True := by
  fail_if_success simp (disch := assumption) only [vir_denote_norm] at same
  trivial

example (room : a.values.size + 1 < 2 ^ 64) : (a.push e).values = a.values.push e := by
  simp (disch := assumption) only [vir_denote_norm]

-- At a full vector the appended element is not among the values of the
-- nest, which is why the nest is read only under its bound.
example (full : ¬a.values.size + 1 < 2 ^ 64) :
    ((a.push e).append b).values = (a.append b).values := by
  simp only [SpecVector.push, dif_neg full]

end Nests

verex module 0x42::clause_singleton_concat where
  pragma verify = false

  struct Percent has Copy, Drop, Store where
    value : u64
  spec Percent where
    invariant value <= 100

  -- The prefix of `v` below `end`, collected element by element.
  spec fun collected(v : Vector<u64>, end : Int) : Vector<u64> decreases end :=
    if end <= 0 then spec.emptyVector::<u64>()
    else concat(collected(v, end - 1), vec(v[end - 1]))

  -- The other concatenations, each as the step of a recursion, whose
  -- unfolding shows its translation.
  spec fun repeated(x : Vector<u64>, y : Vector<u64>, n : Int) : Vector<u64> decreases n :=
    if n <= 0 then x else concat(repeated(x, y, n - 1), y)
  spec fun paired(x : Vector<u64>, a : u64, b : u64, n : Int) : Vector<u64> decreases n :=
    if n <= 0 then x else concat(paired(x, a, b, n - 1), vector<u64>[a, b])
  spec fun padded(x : Vector<u64>, n : Int) : Vector<u64> decreases n :=
    if n <= 0 then x else concat(padded(x, n - 1), spec.emptyVector::<u64>())
  spec fun prepended(e : u64, x : Vector<u64>, n : Int) : Vector<u64> decreases n :=
    if n <= 0 then x else concat(vec(e), prepended(e, x, n - 1))
  spec fun literal(a : u64, b : u64, n : Int) : Vector<u64> decreases n :=
    if n <= 0 then concat(vec(a), vec(b)) else literal(a, b, n - 1)

  -- Another element than the appended one.
  fun append_other_bad(acc : Vector<u64>, x : u64, y : u64) -> Vector<u64> :=
    core.prim.pushVector(acc, x)
  spec append_other_bad where
    ensures result == concat(acc, vec(y))

  -- The appended element's invariant is not more than it states.
  fun grow_percent_bad(acc : Vector<Percent>, p : Percent) -> Vector<Percent> :=
    core.prim.pushVector(acc, p)
  spec grow_percent_bad where
    ensures result == concat(acc, vec(p))
    ensures result[acc.length].value <= 99

  -- An element its type's invariant refuses, beside a clause that
  -- concatenates a valid one.
  fun grow_invalid_bad(acc : Vector<Percent>, p : Percent) -> Vector<Percent> :=
    core.prim.pushVector(acc, new Percent { value := 101 })
  spec grow_invalid_bad where
    aborts_if false
    ensures result.length == concat(acc, vec(p)).length

  -- Two appends in the other order.
  fun chain_swapped_bad(acc : Vector<u64>, x : u64, y : u64) -> Vector<u64> := do
    let v := core.prim.pushVector(acc, x)
    core.prim.pushVector(v, y)
  spec chain_swapped_bad where
    ensures result == concat(concat(acc, vec(y)), vec(x))

  -- The collection is exactly the input.
  fun collect_bad(v : Vector<u64>) -> Vector<u64> := do
    let r := vector<u64>[]
    let i := 0u64
    while i < v.length do
      r := core.prim.pushVector(r, v[i])
      i := i + 1
      continue
    r
  spec collect_bad where
    ensures result.length == v.length + 1
    loop 1 where
      invariant i <= v.length
      invariant r == collected(v, i)

  -- A nest holding a one-element literal, without the bound on its length.
  fun nest_unbounded_bad(a : Vector<u64>, e : u64, b : Vector<u64>) -> Vector<u64> := a
  spec nest_unbounded_bad where
    ensures concat(concat(a, vec(e)), b).length == a.length + 1 + b.length

  fun append_ok(acc : Vector<u64>, x : u64) -> Vector<u64> := core.prim.pushVector(acc, x)
  spec append_ok where
    aborts_if false
    ensures result == concat(acc, vec(x))

  fun grow_percent_ok(acc : Vector<Percent>, p : Percent) -> Vector<Percent> :=
    core.prim.pushVector(acc, p)
  spec grow_percent_ok where
    aborts_if false
    ensures result == concat(acc, vec(p))
    ensures result[acc.length].value <= 100

  fun chain_ok(acc : Vector<u64>, x : u64, y : u64) -> Vector<u64> := do
    let v := core.prim.pushVector(acc, x)
    core.prim.pushVector(v, y)
  spec chain_ok where
    ensures result == concat(concat(acc, vec(x)), vec(y))
    ensures result.length == acc.length + 2
    ensures result[acc.length + 1] == y

  -- A chain from the empty literal against the literal a clause folds.
  fun literal_chain_ok(x : u64, y : u64, z : u64) -> Vector<u64> := do
    let v := vector<u64>[]
    v := core.prim.pushVector(v, x)
    v := core.prim.pushVector(v, y)
    v := core.prim.pushVector(v, z)
    v
  spec literal_chain_ok where
    ensures result == vector<u64>[x, y, z]
    ensures result == concat(concat(vec(x), vec(y)), vec(z))
    ensures result == concat(vector<u64>[x, y], vec(z))

  -- An append, then the removal of the element appended.
  fun push_then_pop_ok(acc : Vector<u64>, x : u64) -> u64 := do
    let v := core.prim.pushVector(acc, x)
    let (removed, rest) := core.prim.removeVector(v, v.length - 1)
    removed
  spec push_then_pop_ok where
    ensures result == concat(acc, vec(x))[acc.length]
    ensures result == x

  -- A one-element literal inside a nest of concatenations, and on top of one.
  fun nest_middle_ok(a : Vector<u64>, e : u64, b : Vector<u64>) -> Vector<u64> := a
  spec nest_middle_ok where
    requires a.length + 1 + b.length <= 18446744073709551615
    ensures concat(concat(a, vec(e)), b).length == a.length + 1 + b.length

  fun nest_tail_ok(a : Vector<u64>, b : Vector<u64>, e : u64) -> Vector<u64> := a
  spec nest_tail_ok where
    requires a.length + b.length + 1 <= 18446744073709551615
    ensures concat(concat(a, b), vec(e)).length == a.length + b.length + 1

  fun nest_mixed_ok(a : Vector<u64>, e : u64, b : Vector<u64>, f : u64) -> Vector<u64> := a
  spec nest_mixed_ok where
    requires a.length + 1 + b.length + 1 <= 18446744073709551615
    ensures concat(concat(concat(a, vec(e)), b), vec(f)).length == a.length + b.length + 2
    ensures concat(concat(concat(a, vec(e)), b), vec(f))[a.length] == e

  fun collect(v : Vector<u64>) -> Vector<u64> := do
    let r := vector<u64>[]
    let i := 0u64
    while i < v.length do
      r := core.prim.pushVector(r, v[i])
      i := i + 1
      continue
    r
  spec collect where
    aborts_if false
    ensures result == collected(v, v.length)
    loop 1 where
      invariant i <= v.length
      invariant r == collected(v, i)

/-! ### The translated forms

Only the concatenation with the literal of one element on the right is an
append of that element; a vector that is no literal, a literal of two or of
no elements, and a one-element literal on the left stay concatenations, and
two literals are the literal of their elements. -/

run_cmd do
  let env ← getEnv
  for (function, push, append) in [("collected", true, false), ("repeated", false, true),
      ("paired", false, true), ("padded", false, true), ("prepended", false, true),
      ("literal", false, false)] do
    let name := (`«0x42».clause_singleton_concat).str function ++ `spec ++ `unfold
    let some unfolding := env.find? name | throwError "no unfolding of {function}"
    let states (operation : Name) := (unfolding.type.find? (·.isConstOf operation)).isSome
    unless states ``SpecVector.push == push && states ``SpecVector.append == append do
      throwError "{function} unfolds with push {states ``SpecVector.push} and append \
        {states ``SpecVector.append}:\n{unfolding.type}"

/-! ### Negative controls -/

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».clause_singleton_concat.append_other_bad
    ["ensures result == concat(acc, vec(y))"]
    (← `(command| verify 0x42::clause_singleton_concat::append_other_bad))
  expectUnestablished `«0x42».clause_singleton_concat.grow_percent_bad
    ["ensures result[acc.length].value <= 99"]
    (← `(command| verify 0x42::clause_singleton_concat::grow_percent_bad))
  expectCuts `«0x42».clause_singleton_concat.grow_invalid_bad
    [("invariant value <= 100", construction)]
    (← `(command| verify 0x42::clause_singleton_concat::grow_invalid_bad)) (some [])
  expectUnestablished `«0x42».clause_singleton_concat.chain_swapped_bad
    ["ensures result == concat(concat(acc, vec(y)), vec(x))"]
    (← `(command| verify 0x42::clause_singleton_concat::chain_swapped_bad))
  expectUnestablished `«0x42».clause_singleton_concat.nest_unbounded_bad
    ["ensures concat(concat(a, vec(e)), b).length == a.length + 1 + b.length"]
    (← `(command| verify 0x42::clause_singleton_concat::nest_unbounded_bad))
  expectUnestablished `«0x42».clause_singleton_concat.collect_bad
    ["ensures result.length == v.length + 1"]
    (← `(command| verify 0x42::clause_singleton_concat::collect_bad))

/-! ### Positive controls -/

verify 0x42::clause_singleton_concat::append_ok
verify 0x42::clause_singleton_concat::grow_percent_ok
verify 0x42::clause_singleton_concat::chain_ok
verify 0x42::clause_singleton_concat::literal_chain_ok
verify 0x42::clause_singleton_concat::push_then_pop_ok
verify 0x42::clause_singleton_concat::nest_middle_ok
verify 0x42::clause_singleton_concat::nest_tail_ok
verify 0x42::clause_singleton_concat::nest_mixed_ok
verify 0x42::clause_singleton_concat::collect

end VerexSL.Tests.ClauseSingletonConcat
