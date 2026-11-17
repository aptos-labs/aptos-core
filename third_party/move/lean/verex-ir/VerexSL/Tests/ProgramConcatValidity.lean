-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL
import VerexSL.Tests.LoanDeathSupport

/-! ## The fact of a program's concatenation

A program's concatenation of two vectors denotes `concatBounded`, which keeps
both vectors until its result is asked for its fact
(`denote_concat_direct`). The fact of the result is the facts of both
(`NTy.Holds.concatBounded`): the opener selects it where the fact of one of
the two is reached and asks the other vector for its fact whole, so a literal
is read by its elements and a vector the context says nothing of stays owed.
Past the representable length the program has no result, as before.

The controls run programs through the walk: two vectors the context holds, at
an opaque and at a declared element type; a literal on either side; two
literals; a concatenation of a concatenation; and vectors or elements whose
fact is missing. The tactic controls state the selection itself.
-/

namespace VerexSL.Tests.ProgramConcatValidity

set_option Elab.async false

open Lean Meta Elab VerexIR VerexSL VerexIR.Proofs.Denote

verex module 0x42::program_concat_validity where
  pragma verify = false

  struct Percent has Copy, Drop, Store where
    value : u64
  spec Percent where
    invariant value <= 100

  -- Two vectors the context holds, at an opaque element type.
  fun append_generic_ok {T}(lhs : &mut Vector<T>, other : Vector<T>) -> Unit :=
    *lhs := core.prim.concatVector(*lhs, other)
  spec append_generic_ok where
    ensures lhs.length == old(lhs).length + other.length

  -- The same at an element type that declares an invariant.
  fun append_percents_ok(lhs : &mut Vector<Percent>, other : Vector<Percent>) -> Unit :=
    *lhs := core.prim.concatVector(*lhs, other)
  spec append_percents_ok where
    ensures lhs.length == old(lhs).length + other.length

  -- A literal of one element on the right, and on the left.
  fun append_singleton_ok(lhs : &mut Vector<Percent>, p : Percent) -> Unit :=
    *lhs := core.prim.concatVector(*lhs, vector<Percent>[p])
  spec append_singleton_ok where
    ensures lhs.length == old(lhs).length + 1

  fun prepend_singleton_ok(lhs : &mut Vector<Percent>, p : Percent) -> Unit :=
    *lhs := core.prim.concatVector(vector<Percent>[p], *lhs)
  spec prepend_singleton_ok where
    ensures lhs.length == old(lhs).length + 1

  -- Two literals.
  fun literals_ok(lhs : &mut Vector<Percent>, p : Percent, q : Percent) -> Unit :=
    *lhs := core.prim.concatVector(vector<Percent>[p], vector<Percent>[q, p])
  spec literals_ok where
    ensures lhs.length == 3

  -- A concatenation of a concatenation, returned.
  fun nested_ok(a : Vector<Percent>, b : Vector<Percent>, p : Percent) -> Vector<Percent> :=
    core.prim.concatVector(core.prim.concatVector(a, vector<Percent>[p]), b)
  spec nested_ok where
    ensures result.length == a.length + 1 + b.length

  -- The concatenated vector holds an element its type's invariant refuses.
  fun append_invalid_bad(lhs : &mut Vector<Percent>, other : &mut Vector<Percent>,
      idx : u64) -> Unit := do
    (*other)[idx].value := 101u64
    *lhs := core.prim.concatVector(*lhs, *other)
  spec append_invalid_bad where
    requires idx < other.length

/-! ### Negative control

The element the write made invalid is refused where the vector that holds it
is copied out of its reference, on its way into the concatenation: the
concatenation is given no fact of that vector by the vector beside it. -/

open LoanDeathControls in
run_cmd do
  expectCuts `«0x42».program_concat_validity.append_invalid_bad
    [("invariant value <= 100", "where a value is copied out of a mutable reference")]
    (← `(command| verify 0x42::program_concat_validity::append_invalid_bad)) (some [])

/-! ### Positive controls -/

verify 0x42::program_concat_validity::append_generic_ok
verify 0x42::program_concat_validity::append_percents_ok
verify 0x42::program_concat_validity::append_singleton_ok
verify 0x42::program_concat_validity::prepend_singleton_ok
verify 0x42::program_concat_validity::literals_ok
verify 0x42::program_concat_validity::nested_ok

end VerexSL.Tests.ProgramConcatValidity
