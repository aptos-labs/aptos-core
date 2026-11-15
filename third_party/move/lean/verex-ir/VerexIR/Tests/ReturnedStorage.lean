-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.ReturnedStorage

namespace VerexIR.Tests.ReturnedStorage

open SemanticOperations Proofs

-- An entry loan of a table a global holds resolves that entry's value only;
-- another entry remains unaffected.
example :
    let key : GlobalKey := ⟨⟨0⟩, ⟨0⟩, .address "table-a"⟩
    let heap : GlobalMap := ⟨#[⟨key,
      .vector #[.tuple #[.integer 1, .loanHole 42], .tuple #[.integer 2, .integer 3]]⟩]⟩
    (heap.resolveReturned #[.borrow 42 (.integer 9)]).lookup key =
      some (.vector #[.tuple #[.integer 1, .integer 9], .tuple #[.integer 2, .integer 3]]) := by
  simp [GlobalMap.resolveReturned, GlobalMap.lookup, resolveReturnedBorrows,
    outermostBorrows, collectPruned, borrowEntry?, fillHole?, rewriteFirst, rewriteFirstList]

example (state : RuntimeState) (results : Array RuntimeValue) :
    (state.resolveReturned results).tableHandles = state.tableHandles := rfl

-- Even when failure conditions excuse ensures, the frame must constrain the
-- actual returned value. An unrelated result cannot satisfy it existentially.
private def nextContract : Contract Nat Unit Unit Nat where
  requires := fun _ _ => True
  ensures := fun _ _ _ _ => False
  aborts := fun _ _ _ => False
  mayAbort := fun _ _ => True
  frame := fun _ initial result final => final = initial ∧ result = initial + 1

private def next : Unit → Spec Nat Unit Nat := fun _ => {
  ok := fun initial result final => final = initial ∧ result = initial + 1
  aborts := fun _ _ => False
  undefined := fun _ => False }

example : Satisfies next nextContract := by
  simp [Satisfies, next, nextContract]

example : ¬ Satisfies (fun _ => Spec.pure 0) nextContract := by
  intro satisfies
  have wrong := (satisfies () 0 trivial trivial).1 0 0 ⟨rfl, rfl⟩
  have impossible : (0 : Nat) = 1 := wrong.2.1.2
  omega

end VerexIR.Tests.ReturnedStorage
