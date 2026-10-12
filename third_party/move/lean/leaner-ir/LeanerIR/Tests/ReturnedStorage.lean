-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.Denote.ReturnedStorage

namespace LeanerIR.Tests.ReturnedStorage

open SemanticOperations Proofs

-- A native entry loan resolves its value only; another entry and another
-- Table allocation remain unaffected. The physical Table is still just a handle.
example :
    let key : GlobalKey := ⟨⟨0⟩, ⟨0⟩, .address "table-a"⟩
    let heap : GlobalMap := ⟨#[⟨key,
      .vector #[.tuple #[.integer 1, .loanHole 42], .tuple #[.integer 2, .integer 3]]⟩]⟩
    (heap.resolveReturned #[.borrow 42 (.integer 9)]).lookup key =
      some (.vector #[.tuple #[.integer 1, .integer 9], .tuple #[.integer 2, .integer 3]]) := by
  simp [GlobalMap.resolveReturned, GlobalMap.lookup, resolveReturnedBorrows,
    outermostBorrows, collectPruned, borrowEntry?, fillHole?, rewriteFirst, rewriteFirstList]

example (state : RuntimeState) (results : Array RuntimeValue) :
    (state.resolveReturned results).tables.allocated = state.tables.allocated := rfl

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

end LeanerIR.Tests.ReturnedStorage

namespace LeanerIR.Tests.ReturnedStorage

open SemanticOperations

-- The same key in the two stores names different owners. Returning the
-- native entry must preserve a live global loan and allocation history.
private def collisionKey : GlobalKey := ⟨⟨0⟩, ⟨0⟩, .address "table-a"⟩
private def collisionState : RuntimeState := {
  globals := ⟨#[⟨collisionKey, .loanHole 1⟩]⟩
  tables := {
    contents := ⟨#[⟨collisionKey, .vector #[.tuple #[.integer 7, .loanHole 0], .tuple #[.integer 8, .integer 17]]⟩]⟩
    allocated := #["destroyed-table", "table-a"] }
  storageLoans := [(0, .table collisionKey), (1, .global collisionKey)]
  nextLoan := 2 }

-- Observing a returned row resolves the global loan, but cannot overwrite
-- a Table slot already replaced by a plain value, even if that row still
-- contains the Table's former loan. Observation also cannot resurrect an
-- erased slot. Both checks use the same physical key in the two stores.
#guard
  let written := { collisionState with tables.contents :=
    (collisionState.tables.contents.insert collisionKey
      (.vector #[.tuple #[.integer 7, .integer 23]])) }
  let observed := written.resolveReturned #[.borrow 0 (.integer 9), .borrow 1 (.integer 11)]
  observed.tables.contents.lookup collisionKey ==
      some (.vector #[.tuple #[.integer 7, .integer 23]]) &&
    observed.globals.lookup collisionKey == some (.integer 11) &&
    observed.tables.allocated == collisionState.tables.allocated

#guard
  let erased := { collisionState with tables.contents :=
    collisionState.tables.contents.erase collisionKey }
  let observed := erased.resolveReturned #[.borrow 0 (.integer 9), .borrow 1 (.integer 11)]
  (observed.tables.contents.lookup collisionKey).isNone &&
    observed.globals.lookup collisionKey == some (.integer 11) &&
    observed.tables.allocated == collisionState.tables.allocated

example :
    exportFrameLoans { locals := #[some (.borrow 0 (.integer 9))] } collisionState =
      { collisionState with
        tables.contents := ⟨#[⟨collisionKey, .vector #[.tuple #[.integer 7, .integer 9], .tuple #[.integer 8, .integer 17]]⟩]⟩
        storageLoans := [(1, .global collisionKey)] } := by
  simp [collisionState, exportFrameLoans, exportSettledLoans, frameBorrows,
    outermostBorrows, borrowEntry?, collectPruned,
    holeInFrame, holeWithin, findFirst, applyWriteBack,
    fillVisibleHole, storageLoanTarget?, storageLoanTargetIn?,
    RuntimeState.loanValue?, RuntimeState.writeLoanValue, GlobalMap.lookup,
    GlobalMap.insert, GlobalMap.erase, GlobalMap.insertSlot,
    transferStorageLoan, removeStorageLoan, transferredLoan?, fillHole?,
    rewriteFirst, rewriteFirstList]

example :
    let afterTable := exportFrameLoans { locals := #[some (.borrow 0 (.integer 9))] }
      collisionState
    exportFrameLoans { locals := #[some (.borrow 1 (.integer 11))] } afterTable =
      { collisionState with
        globals := ⟨#[⟨collisionKey, .integer 11⟩]⟩
        tables.contents := ⟨#[⟨collisionKey, .vector #[.tuple #[.integer 7, .integer 9], .tuple #[.integer 8, .integer 17]]⟩]⟩
        storageLoans := [] } := by
  simp [collisionState, exportFrameLoans, exportSettledLoans, frameBorrows,
    outermostBorrows, borrowEntry?, collectPruned,
    holeInFrame, holeWithin, findFirst, applyWriteBack,
    fillVisibleHole, storageLoanTarget?, storageLoanTargetIn?,
    RuntimeState.loanValue?, RuntimeState.writeLoanValue, GlobalMap.lookup,
    GlobalMap.insert, GlobalMap.erase, GlobalMap.insertSlot,
    transferStorageLoan, removeStorageLoan, transferredLoan?, fillHole?,
    rewriteFirst, rewriteFirstList]

-- A returned subloan keeps the native owner, including its storage domain.
example :
    let initial := { collisionState with nextLoan := 3 }
    let transferred := (applyWriteBack {} initial 0 (.tuple #[.loanHole 2])).2
    (applyWriteBack {} transferred 2 (.integer 13)).2 =
      { initial with
        tables.contents := ⟨#[⟨collisionKey,
          .vector #[.tuple #[.integer 7, .tuple #[.integer 13]], .tuple #[.integer 8, .integer 17]]⟩]⟩
        storageLoans := [(1, .global collisionKey)] } := by
  simp [collisionState, holeInFrame, findFirst, findFirstList, applyWriteBack,
    fillVisibleHole, storageLoanTarget?, storageLoanTargetIn?,
    RuntimeState.loanValue?, RuntimeState.writeLoanValue, GlobalMap.lookup,
    GlobalMap.insert, GlobalMap.erase, GlobalMap.insertSlot,
    transferStorageLoan, removeStorageLoan, transferredLoan?, fillHole?,
    rewriteFirst, rewriteFirstList]

end LeanerIR.Tests.ReturnedStorage
