-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.Denote.TableOperations

namespace LeanerIR.Tests.TableOperations

open SemanticOperations

private def slot : GlobalKey := tableSlot ⟨0⟩ ⟨0⟩ "table-a"
private def otherSlot : GlobalKey := tableSlot ⟨1⟩ ⟨2⟩ "table-b"
private def initial : RuntimeState := {
  globals := ⟨#[⟨slot, .loanHole 3⟩]⟩
  storageLoans := [(3, .global slot)]
  nextLoan := 42
  tables := {
    contents := ⟨#[⟨otherSlot, .vector #[.tuple #[.integer 1, .integer 80]]⟩]⟩
    allocated := #["retired-table", "table-b"] } }

private def populated? : Option RuntimeState := do
  let created ← allocateTableAt? initial ⟨0⟩ ⟨0⟩ "table-a"
  let first ← addTableEntry? created slot (.integer 7) (.integer 5)
  addTableEntry? first slot (.integer 8) (.integer 17)

#guard allocateTableAt? initial ⟨0⟩ ⟨0⟩ "retired-table" == none
#guard allocateTableAt? initial ⟨9⟩ ⟨9⟩ "table-b" == none

-- Even imported live contents with no history entry cannot be allocated a
-- second time through another owner/type instance.
#guard allocateTableAt? { initial with tables.allocated := #[] } ⟨9⟩ ⟨9⟩ "table-b" == none

#guard populated?.bind (fun state => readTableEntry? state slot (.integer 7)) == some (.integer 5)
#guard populated?.bind (fun state => addTableEntry? state slot (.integer 7) (.integer 99)) == none
#guard populated?.bind (fun state => removeTableEntry? state slot (.integer 99)) == none
#guard populated?.bind (fun state => retireEmptyTable? state slot) == none
#guard addTableEntry? initial slot (.integer 7) (.integer 5) == none

-- Real allocation, insertion, mutable borrowing, frame write-back, removal,
-- and retirement compose while retaining a colliding global loan and another
-- owner's Table contents. Retirement retains the allocation history.
private def roundTrip? : Option (RuntimeState × RuntimeValue × RuntimeValue) := do
  let state ← populated?
  let (borrowed, .borrow loan _) ← borrowTableEntry? state slot (.integer 7) | none
  let written := exportFrameLoans { locals := #[some (.borrow loan (.integer 9))] } borrowed
  let (removed, first) ← removeTableEntry? written slot (.integer 7)
  let (emptied, second) ← removeTableEntry? removed slot (.integer 8)
  let retired ← retireEmptyTable? emptied slot
  some (retired, first, second)

#guard roundTrip? == some ({ initial with
  tables.allocated := #["retired-table", "table-b", "table-a"]
  nextLoan := 43 }, .integer 9, .integer 17)

#guard roundTrip?.bind (fun (state, _, _) => allocateTableAt? state ⟨9⟩ ⟨9⟩ "table-a") == none

#guard populated?.bind (fun state => (removeTableEntry? state slot (.integer 7)).bind
  (fun (removed, _) => readTableEntry? removed slot (.integer 8))) == some (.integer 17)

-- Typed-model boundary cases agree with the executable duplicate/missing-key
-- checks. These proofs work for an arbitrary unit, without evaluating a unit.
open Proofs.Denote in
example (unit : Validation.ValidatedUnit) :
    TableMemory.add? (unit := unit) .bool .bool #[(true, false, ())] true true = none := by
  simp [TableMemory.add?, TableMemory.index?, guard, failure]

open Proofs.Denote in
example (unit : Validation.ValidatedUnit) :
    TableMemory.remove? (unit := unit) .bool .bool #[(true, false, ())] false = none := by
  simp [TableMemory.remove?, TableMemory.index?]

open Proofs.Denote in
example (unit : Validation.ValidatedUnit) :
    TableMemory.remove? (unit := unit) .bool .bool #[(true, false, ())] true =
      some (#[], false) := by
  simp [TableMemory.remove?, TableMemory.index?, Array.eraseIdxIfInBounds]

end LeanerIR.Tests.TableOperations
