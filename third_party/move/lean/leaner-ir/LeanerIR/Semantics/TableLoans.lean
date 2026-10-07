-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Semantics.Focus
import LeanerIR.Semantics.LoanRenamingOps

/-!
# Native Table entry loans

The native resolves a Table's closed owner/type and handle to its contents
slot, then resolves the key to an entry index. Borrowing the value leaves a
hole in that entry and registers its Table destination. The existing frame
export/write-back operations consume the returned borrow.

These are runtime primitives, not a replacement for the native contract or
Move's ownership checks. The native adapter must establish that the supplied
slot belongs to the borrowed Table and has a valid typed map representation.
-/

namespace LeanerIR.SemanticOperations

/-- Borrow a resolved part of a stored value. Only the native's temporary
selection uses the path: the returned reference and registry carry the loan
identity and owning slot, without retaining a projection path. -/
def borrowStoredAt? (state : RuntimeState) (target : LoanTarget)
    (path : List RuntimeProjection) : Option (RuntimeState × RuntimeValue) := do
  let owner ← state.loanValue? target
  let current ← readProjections? owner path
  let holed ← writeProjections? owner path (.loanHole state.nextLoan)
  some ({ state.writeLoanValue target holed with
    storageLoans := (state.nextLoan, target) :: state.storageLoans
    nextLoan := state.nextLoan + 1 }, .borrow state.nextLoan current)

/-- Borrow the value component of a resolved native Table entry. -/
def borrowTableEntryAt? (state : RuntimeState) (slot : GlobalKey) (index : Nat) :
    Option (RuntimeState × RuntimeValue) :=
  borrowStoredAt? state (.table slot) [.index index, .index 1]

/-- Decode a native entry, rejecting rows other than a key/value pair. -/
def tableEntry? : RuntimeValue → Option (RuntimeValue × RuntimeValue)
  | .tuple fields => match fields.toList with
    | [key, value] => some (key, value)
    | _ => none
  | _ => none

theorem tableEntry?_sound {entry key value : RuntimeValue}
    (found : tableEntry? entry = some (key, value)) : entry = .tuple #[key, value] := by
  cases entry <;> simp only [tableEntry?, reduceCtorEq] at found
  case tuple fields =>
    split at found
    · rename_i storedKey storedValue row
      simp only [Option.some.injEq, Prod.mk.injEq] at found
      obtain ⟨rfl, rfl⟩ := found
      congr 1
      exact Array.toList_inj.mp row
    · contradiction

/-- Find an entry by structural key equality in the native contents vector.
The native contract supplies distinct, typed keys. Malformed rows do not
match. The array position is an execution detail, not part of Table identity. -/
def tableEntryIndex? (contents key : RuntimeValue) : Option Nat := do
  let .vector entries := contents | none
  entries.findIdx? fun entry => (tableEntry? entry).any (·.1 == key)

/-- A successful lookup selects a complete entry with the queried key. -/
theorem tableEntryIndex?_sound {contents key : RuntimeValue} {index : Nat}
    (found : tableEntryIndex? contents key = some index) :
    ∃ entries value, contents = .vector entries ∧ entries[index]? = some (.tuple #[key, value]) := by
  cases contents <;> simp only [tableEntryIndex?, reduceCtorEq] at found
  case vector entries =>
    have matched := Array.of_findIdx?_eq_some found
    cases entry_eq : entries[index]? with
    | none => simp [entry_eq] at matched
    | some entry =>
        simp only [entry_eq] at matched
        obtain ⟨⟨storedKey, value⟩, decoded, same⟩ := (Option.any_eq_true _ _).mp matched
        have equal : storedKey = key := eq_of_beq same
        subst storedKey
        exact ⟨entries, value, rfl, entry_eq.trans (congrArg some (tableEntry?_sound decoded))⟩

/-- Mutable Table lookup followed by entry-loan registration. -/
def borrowTableEntry? (state : RuntimeState) (slot : GlobalKey) (key : RuntimeValue) :
    Option (RuntimeState × RuntimeValue) := do
  let contents ← state.tables.contents.lookup slot
  let index ← tableEntryIndex? contents key
  borrowTableEntryAt? state slot index

/-- Shared Table lookup observes a value without minting or registering a
loan, as ordinary shared references do after certified ownership checking. -/
def readTableEntry? (state : RuntimeState) (slot : GlobalKey) (key : RuntimeValue) :
    Option RuntimeValue := do
  let contents ← state.tables.contents.lookup slot
  let index ← tableEntryIndex? contents key
  readProjections? contents [.index index, .index 1]

/-- A successful stored borrow registers exactly the fresh identifier and
advances the frontier; earlier registrations remain unchanged. -/
theorem borrowStoredAt?_discipline {state final : RuntimeState} {target : LoanTarget}
    {path : List RuntimeProjection} {result : RuntimeValue}
    (borrowed : borrowStoredAt? state target path = some (final, result)) :
    LoanDiscipline state final := by
  simp only [borrowStoredAt?, Option.bind_eq_bind, Option.bind_eq_some_iff, Option.some.injEq] at borrowed
  obtain ⟨owner, _, current, _, holed, _, equal⟩ := borrowed
  cases equal
  exact LoanDiscipline.of_registered rfl (Nat.le_refl _) (Nat.lt_succ_self _)

/-- A Table borrow leaves ordinary globals and allocation history unchanged. -/
theorem borrowTableEntryAt?_frame {state final : RuntimeState} {slot : GlobalKey}
    {index : Nat} {result : RuntimeValue}
    (borrowed : borrowTableEntryAt? state slot index = some (final, result)) :
    final.globals = state.globals ∧ final.tables.allocated = state.tables.allocated ∧
      final.pending = state.pending := by
  simp only [borrowTableEntryAt?, borrowStoredAt?, Option.bind_eq_bind, Option.bind_eq_some_iff,
    Option.some.injEq] at borrowed
  obtain ⟨owner, _, current, _, holed, _, equal⟩ := borrowed
  cases equal
  exact ⟨rfl, rfl, rfl⟩

private theorem readProjections?_above {value current : RuntimeValue}
    (above : value.Above frontier) (read : readProjections? value path = some current) :
    current.Above frontier := by
  rw [← RuntimeValue.shift_unshift frontier value above, readProjections?_shift] at read
  obtain ⟨base, _, rfl⟩ := Option.map_eq_some_iff.mp read
  exact RuntimeValue.above_shift_self frontier base

private theorem writeProjections?_above {value replacement updated : RuntimeValue}
    (above : value.Above frontier) (replacementAbove : replacement.Above frontier)
    (write : writeProjections? value path replacement = some updated) :
    updated.Above frontier := by
  rw [← RuntimeValue.shift_unshift frontier value above,
    ← RuntimeValue.shift_unshift frontier replacement replacementAbove,
    writeProjections?_shift] at write
  obtain ⟨base, _, rfl⟩ := Option.map_eq_some_iff.mp write
  exact RuntimeValue.above_shift_self frontier base

/-- Stored borrowing commutes with loan renaming. This is the registration
part of the native's loan-independence obligation, including Table slots. -/
theorem borrowStoredAt?_mirror
    (shifted : StateShifted offset frontier inert inert' state state₂)
    {target : LoanTarget} {path : List RuntimeProjection} {final : RuntimeState}
    {result : RuntimeValue}
    (borrowed : borrowStoredAt? state target path = some (final, result)) :
    ∃ final₂, borrowStoredAt? state₂ target path =
        some (final₂, result.shift offset) ∧
      StateShifted offset frontier inert inert' final final₂ ∧ result.Above frontier := by
  simp only [borrowStoredAt?, Option.bind_eq_bind, Option.bind_eq_some_iff,
    Option.some.injEq] at borrowed
  obtain ⟨owner, owner_eq, current, read_eq, holed, write_eq, equal⟩ := borrowed
  cases equal
  have ownerAbove := shifted.loanValue_above owner_eq
  have currentAbove := readProjections?_above ownerAbove read_eq
  have holeAbove : (RuntimeValue.loanHole state.nextLoan).Above frontier := by
    simpa using shifted.frontier_le
  have holedAbove := writeProjections?_above ownerAbove holeAbove write_eq
  have written := shifted.writeLoanValue target holedAbove
  have read₂ : readProjections? (owner.shift offset) path = some (current.shift offset) := by
    rw [readProjections?_shift, read_eq]; rfl
  have write₂ : writeProjections? (owner.shift offset) path (.loanHole state₂.nextLoan) =
      some (holed.shift offset) := by
    rw [shifted.nextLoan, ← RuntimeValue.shift_loanHole, writeProjections?_shift, write_eq]
    rfl
  refine ⟨{ state₂.writeLoanValue target (holed.shift offset) with
      storageLoans := (state₂.nextLoan, target) :: state₂.storageLoans
      nextLoan := state₂.nextLoan + 1 }, ?_, ?_, ?_⟩
  · simp only [borrowStoredAt?, Option.bind_eq_bind, shifted.loanValue target, owner_eq,
      Option.map_some, Option.bind_some, read₂, write₂]
    simp [shifted.nextLoan]
  · exact { written with
      nextLoan := by simp [shifted.nextLoan, Nat.add_right_comm]
      frontier_le := Nat.le_succ_of_le shifted.frontier_le
      registry := by
        simpa [shifted.nextLoan] using shifted.registry.cons shifted.frontier_le target }
  · simpa using And.intro shifted.frontier_le currentAbove

/-- A selected Table value with the other entries and its own key retained.
The decomposition is proof data; runtime lookup uses the key/index helpers. -/
structure TableEntryFocus where
  entries : VectorFocus
  key : RuntimeValue

namespace TableEntryFocus

def fill (focus : TableEntryFocus) (value : RuntimeValue) : RuntimeValue :=
  focus.entries.fill (.tuple #[focus.key, value])

def Plain (focus : TableEntryFocus) : Prop :=
  focus.entries.Plain ∧ SemanticOperations.Plain focus.key

theorem read (focus : TableEntryFocus) (value : RuntimeValue) :
    readProjections? (focus.fill value) [.index focus.entries.index, .index 1] =
      some value := by
  simp [fill, VectorFocus.fill, readProjections?]

theorem write (focus : TableEntryFocus) (value replacement : RuntimeValue) :
    writeProjections? (focus.fill value) [.index focus.entries.index, .index 1] replacement =
      some (focus.fill replacement) := by
  simp [fill, VectorFocus.fill, writeProjections?]
  simpa only [Array.set!_eq_setIfInBounds] using focus.entries.set!_fill
    (.tuple #[focus.key, value]) (.tuple #[focus.key, replacement])

/-- Reconciliation updates precisely the selected value, retaining its key
and every other entry. -/
theorem fillHole {focus : TableEntryFocus} (plain : focus.Plain)
    (loan : Nat) (replacement : RuntimeValue) :
    fillHole? loan replacement (focus.fill (.loanHole loan)) =
      some (focus.fill replacement) := by
  unfold fill fillHole?
  rw [VectorFocus.rewriteFirst_fill (LoanMatcher.holeFill? loan replacement) plain.1]
  simp [rewriteFirst, rewriteFirstList,
    rewriteFirst_eq_none_of_plain (LoanMatcher.holeFill? loan replacement) plain.2]

/-- The registered native borrow extracts the selected value and leaves its
hole, without placing a contents vector in the Table's physical value. -/
theorem borrow {state : RuntimeState} {slot : GlobalKey} {focus : TableEntryFocus}
    {value : RuntimeValue} (stored : state.tables.contents.lookup slot = some (focus.fill value)) :
    borrowTableEntryAt? state slot focus.entries.index =
      some ({ state with
        tables.contents := state.tables.contents.insert slot (focus.fill (.loanHole state.nextLoan))
        storageLoans := (state.nextLoan, .table slot) :: state.storageLoans
        nextLoan := state.nextLoan + 1 }, .borrow state.nextLoan value) := by
  simp [borrowTableEntryAt?, borrowStoredAt?, stored, read, write]

/-- Returning a settled native entry restores its value and retires exactly
its registration. Other slots, keys, entries, pending loans, and allocation
history remain those of the initial state. -/
theorem reconcile (state : RuntimeState) (slot : GlobalKey) (loan : Nat)
    {focus : TableEntryFocus} (plain : focus.Plain)
    {value : RuntimeValue} (valuePlain : SemanticOperations.Plain value) :
    applyWriteBack {} { state with
        tables.contents := state.tables.contents.insert slot (focus.fill (.loanHole loan))
        storageLoans := (loan, .table slot) :: state.storageLoans } loan value =
      ({}, { state with
        tables.contents :=
          (state.tables.contents.insert slot (focus.fill (.loanHole loan))).insert slot
            (focus.fill value) }) := by
  simp [applyWriteBack_empty, storageLoanTarget?, storageLoanTargetIn?,
    fillHole plain, transferStorageLoan, removeStorageLoan, transferredLoan?,
    findFirst_eq_none_of_plain LoanMatcher.anyHole? valuePlain]

end TableEntryFocus

end LeanerIR.SemanticOperations
