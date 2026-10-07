-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.Denote.TableMemory
import LeanerIR.Semantics.TableOperations

/-!
# Typed native Table operations

The typed contents operations below agree with execution at a resolved native
slot. They preserve the unbounded typed carrier and expose the exact missing-key
and duplicate-key cases, without decoding arbitrary runtime values into a junk
typed value. Owner/handle resolution, whole-memory agreement, and ownership
frames remain obligations of the native-call adapter.
-/

namespace LeanerIR.Proofs.Denote.TableMemory

open SemanticOperations Validation

variable {unit : ValidatedUnit}

local notation "runtimeEncode" => @NTy.encode (Carriers.runtime unit)

/-- Find a typed key by the equality used by the runtime Table primitive. -/
noncomputable def index? (key value : NTy) (contents : Contents unit key value)
    (query : @NTy.carrier (Carriers.runtime unit) key) : Option Nat :=
  contents.findIdx? fun entry =>
    runtimeEncode key entry.1 == runtimeEncode key query

/-- Shared lookup retains the typed value, including any nested Table handle. -/
noncomputable def lookup? (key value : NTy) (contents : Contents unit key value)
    (query : @NTy.carrier (Carriers.runtime unit) key) :
    Option (@NTy.carrier (Carriers.runtime unit) value) := do
  let index ← index? key value contents query
  let entry ← contents[index]?
  some entry.2.1

noncomputable def add? (key value : NTy) (contents : Contents unit key value)
    (newKey : @NTy.carrier (Carriers.runtime unit) key)
    (newValue : @NTy.carrier (Carriers.runtime unit) value) : Option (Contents unit key value) := do
  guard (index? key value contents newKey).isNone
  some (contents.push (newKey, newValue, ()))

noncomputable def remove? (key value : NTy) (contents : Contents unit key value)
    (query : @NTy.carrier (Carriers.runtime unit) key) :
    Option (Contents unit key value × @NTy.carrier (Carriers.runtime unit) value) := do
  let index ← index? key value contents query
  let entry ← contents[index]?
  some (contents.eraseIdxIfInBounds index, entry.2.1)

theorem tableEntry?_encode (key value : NTy)
    (entry : @NTy.carrier (Carriers.runtime unit) (.tuple (.cons key (.cons value .nil)))) :
    tableEntry? (runtimeEncode (.tuple (.cons key (.cons value .nil))) entry) =
      some (runtimeEncode key entry.1, runtimeEncode value entry.2.1) := by
  simp [NTy.encode_tuple, HList.encode_cons, HList.encode_nil, tableEntry?]

theorem tableEntryIndex?_encode (owner key value : NTy) (contents : Contents unit key value)
    (query : @NTy.carrier (Carriers.runtime unit) key) :
    tableEntryIndex? ((resource owner key value).encode contents) (runtimeEncode key query) =
      index? key value contents query := by
  simp [resource, ResourceType.encode, tableEntryIndex?, Array.findIdx?_map,
    Function.comp_def, tableEntry?, index?]

private theorem eraseIdxIfInBounds_map (f : α → β) (values : Array α) (index : Nat) :
    (values.map f).eraseIdxIfInBounds index = (values.eraseIdxIfInBounds index).map f := by
  apply Array.toList_inj.mp
  simp [List.eraseIdx_eq_take_drop_succ, List.map_take, List.map_drop]

/-- Shared lookup agrees with execution for both present and absent keys.
The encoded slot is plain, so this observation does not require resolving an
outstanding mutable loan. -/
theorem lookup?_agrees (owner key value : NTy) (contents : Contents unit key value)
    (query : @NTy.carrier (Carriers.runtime unit) key)
    (state : RuntimeState) (slot : GlobalKey)
    (stored : state.tables.contents.lookup slot = some ((resource owner key value).encode contents)) :
    readTableEntry? state slot (runtimeEncode key query) =
      (lookup? key value contents query).map (runtimeEncode value) := by
  simp only [readTableEntry?, stored]
  change (do
    let index ← tableEntryIndex? ((resource owner key value).encode contents) (runtimeEncode key query)
    readProjections? ((resource owner key value).encode contents) [.index index, .index 1]) = _
  rw [tableEntryIndex?_encode]
  cases found : index? key value contents query with
  | none => simp [lookup?, found]
  | some index =>
      cases selected : contents[index]? <;>
        simp [lookup?, found, resource, ResourceType.encode, readProjections?, selected]

/-- Typed insertion and runtime insertion agree, including rejection of a
duplicate key. The result is still a typed contents array of unrestricted size. -/
theorem add?_agrees (owner key value : NTy) (contents : Contents unit key value)
    (newKey : @NTy.carrier (Carriers.runtime unit) key)
    (newValue : @NTy.carrier (Carriers.runtime unit) value)
    (state : RuntimeState) (slot : GlobalKey)
    (stored : state.tables.contents.lookup slot = some ((resource owner key value).encode contents)) :
    addTableEntry? state slot (runtimeEncode key newKey) (runtimeEncode value newValue) =
      (add? key value contents newKey newValue).map fun updated =>
        state.writeLoanValue (.table slot) ((resource owner key value).encode updated) := by
  simp only [addTableEntry?, stored, resource, ResourceType.encode]
  change (do
    guard (tableEntryIndex? ((resource owner key value).encode contents) (runtimeEncode key newKey)).isNone
    some (state.writeLoanValue (.table slot)
      (.vector ((contents.map (runtimeEncode (.tuple (.cons key (.cons value .nil))))).push
        (.tuple #[runtimeEncode key newKey, runtimeEncode value newValue]))))) = _
  rw [tableEntryIndex?_encode]
  by_cases missing : index? key value contents newKey = none
  · simp [add?, missing, guard, NTy.encode_tuple,
      HList.encode_cons, HList.encode_nil]
  · simp [add?, missing, guard, failure]

/-- Typed removal agrees with execution and returns the typed value from
the selected binding. It also agrees when the key is absent. -/
theorem remove?_agrees (owner key value : NTy) (contents : Contents unit key value)
    (query : @NTy.carrier (Carriers.runtime unit) key)
    (state : RuntimeState) (slot : GlobalKey)
    (stored : state.tables.contents.lookup slot = some ((resource owner key value).encode contents)) :
    removeTableEntry? state slot (runtimeEncode key query) =
      (remove? key value contents query).map fun (updated, removed) =>
        (state.writeLoanValue (.table slot) ((resource owner key value).encode updated),
          runtimeEncode value removed) := by
  simp only [removeTableEntry?, stored, resource, ResourceType.encode]
  change (do
    let index ← tableEntryIndex? ((resource owner key value).encode contents) (runtimeEncode key query)
    let entry ← (contents.map (runtimeEncode (.tuple (.cons key (.cons value .nil)))))[index]?
    let (_, removed) ← tableEntry? entry
    some (state.writeLoanValue (.table slot)
      (.vector ((contents.map (runtimeEncode (.tuple (.cons key (.cons value .nil))))).eraseIdxIfInBounds index)),
      removed)) = _
  rw [tableEntryIndex?_encode]
  cases found : index? key value contents query with
  | none => simp [remove?, found]
  | some index =>
      cases selected : contents[index]? <;>
        simp [remove?, found, selected, tableEntry?, eraseIdxIfInBounds_map]

theorem containsB_of_index?_none (key value : NTy) (contents : Contents unit key value)
    (query : @NTy.carrier (Carriers.runtime unit) key)
    (missing : index? key value contents query = none) :
    Maps.containsB (entries key value contents) (runtimeEncode key query) = false := by
  apply Bool.eq_false_iff.mpr
  intro present
  obtain ⟨encodedValue, member⟩ := (Maps.containsB_iff _ _).mp present
  obtain ⟨entry, member, equal⟩ := List.mem_map.mp member
  have absent := Array.findIdx?_eq_none_iff.mp missing entry (Array.mem_toList_iff.mp member)
  have same := congrArg Prod.fst equal
  dsimp only at same
  simp [same] at absent

/-- The typed insertion implements the existing pure map update on a
missing key, so its lookup and distinct-key laws apply without new axioms. -/
theorem add?_entries (key value : NTy) (contents updated : Contents unit key value)
    (newKey : @NTy.carrier (Carriers.runtime unit) key)
    (newValue : @NTy.carrier (Carriers.runtime unit) value)
    (added : add? key value contents newKey newValue = some updated) :
    entries key value updated =
      Maps.seqSet (entries key value contents) (runtimeEncode key newKey) (runtimeEncode value newValue) := by
  by_cases missing : index? key value contents newKey = none
  · simp [add?, missing, guard] at added
    cases added
    rw [Maps.seqSet, containsB_of_index?_none key value contents newKey missing]
    simp [entries]
  · simp [add?, missing, guard, failure] at added

theorem add?_distinct (key value : NTy) (contents updated : Contents unit key value)
    (newKey : @NTy.carrier (Carriers.runtime unit) key)
    (newValue : @NTy.carrier (Carriers.runtime unit) value)
    (added : add? key value contents newKey newValue = some updated)
    (distinct : Maps.Distinct (entries key value contents)) :
    Maps.Distinct (entries key value updated) := by
  rw [add?_entries key value contents updated newKey newValue added]
  exact Maps.distinct_seqSet distinct _ _

theorem add?_size (key value : NTy) (contents updated : Contents unit key value)
    (newKey : @NTy.carrier (Carriers.runtime unit) key)
    (newValue : @NTy.carrier (Carriers.runtime unit) value)
    (added : add? key value contents newKey newValue = some updated) :
    updated.size = contents.size + 1 := by
  by_cases missing : index? key value contents newKey = none
  · simp [add?, missing, guard] at added
    cases added
    simp
  · simp [add?, missing, guard, failure] at added

theorem entries_eraseIdx (key value : NTy) (contents : Contents unit key value) (index : Nat) :
    entries key value (contents.eraseIdxIfInBounds index) = (entries key value contents).eraseIdx index := by
  simp [entries, List.eraseIdx_eq_take_drop_succ, List.map_take, List.map_drop]

theorem remove?_distinct (key value : NTy) (contents updated : Contents unit key value)
    (query : @NTy.carrier (Carriers.runtime unit) key)
    (result : @NTy.carrier (Carriers.runtime unit) value)
    (removed : remove? key value contents query = some (updated, result))
    (distinct : Maps.Distinct (entries key value contents)) :
    Maps.Distinct (entries key value updated) := by
  simp only [remove?, Option.bind_eq_bind, Option.bind_eq_some_iff, Option.some.injEq,
    Prod.mk.injEq] at removed
  obtain ⟨index, _, entry, _, rfl, _⟩ := removed
  rw [entries_eraseIdx]
  exact List.Nodup.sublist ((List.eraseIdx_sublist _ _).map Prod.fst) distinct

theorem remove?_size (key value : NTy) (contents updated : Contents unit key value)
    (query : @NTy.carrier (Carriers.runtime unit) key)
    (result : @NTy.carrier (Carriers.runtime unit) value)
    (removed : remove? key value contents query = some (updated, result)) :
    updated.size + 1 = contents.size := by
  simp only [remove?, Option.bind_eq_bind, Option.bind_eq_some_iff, Option.some.injEq,
    Prod.mk.injEq] at removed
  obtain ⟨index, _, entry, selected, rfl, _⟩ := removed
  obtain ⟨bound, _⟩ := Array.getElem?_eq_some_iff.mp selected
  simp [Array.eraseIdxIfInBounds, bound]
  omega

/-- The removed entry and the remaining entries, in the pure map vocabulary. -/
theorem remove?_entries (key value : NTy) (contents updated : Contents unit key value)
    (query : @NTy.carrier (Carriers.runtime unit) key)
    (result : @NTy.carrier (Carriers.runtime unit) value)
    (removed : remove? key value contents query = some (updated, result)) :
    ∃ index, (entries key value contents)[index]? =
        some (runtimeEncode key query, runtimeEncode value result) ∧
      entries key value updated = (entries key value contents).eraseIdx index := by
  simp only [remove?, Option.bind_eq_bind, Option.bind_eq_some_iff, Option.some.injEq,
    Prod.mk.injEq] at removed
  obtain ⟨index, found, entry, selected, rfl, returned⟩ := removed
  have matched := Array.of_findIdx?_eq_some found
  simp only [selected] at matched
  have sameKey : runtimeEncode key entry.1 = runtimeEncode key query := by simpa using matched
  refine ⟨index, ?_, entries_eraseIdx key value contents index⟩
  simp [entries, selected, sameKey, returned]

theorem remove?_value (key value : NTy) (contents updated : Contents unit key value)
    (query : @NTy.carrier (Carriers.runtime unit) key)
    (result : @NTy.carrier (Carriers.runtime unit) value)
    (removed : remove? key value contents query = some (updated, result))
    (distinct : Maps.Distinct (entries key value contents)) :
    Maps.get (entries key value contents) (runtimeEncode key query) = runtimeEncode value result := by
  obtain ⟨index, selected, _⟩ := remove?_entries key value contents updated query result removed
  exact Maps.get_of_mem distinct (List.mem_of_getElem? selected)

/-- Removal changes membership at exactly the queried key. No other binding
is lost, including when its value contains another owned Table. -/
theorem remove?_member (key value : NTy) (contents updated : Contents unit key value)
    (query : @NTy.carrier (Carriers.runtime unit) key)
    (result : @NTy.carrier (Carriers.runtime unit) value)
    (removed : remove? key value contents query = some (updated, result))
    (distinct : Maps.Distinct (entries key value contents)) (entry : RuntimeValue × RuntimeValue) :
    entry ∈ entries key value updated ↔
      entry ∈ entries key value contents ∧ entry.1 ≠ runtimeEncode key query := by
  obtain ⟨index, selected, remaining⟩ := remove?_entries key value contents updated query result removed
  obtain ⟨bound, selected⟩ := List.getElem?_eq_some_iff.mp selected
  rw [remaining, Maps.mem_eraseIdx_iff_of_distinct distinct index bound entry, selected]

/-- A typed entry and the surrounding contents retained by a mutable borrow.
It carries no loan identifier; execution supplies and registers that identifier. -/
structure EntryFocus (unit : ValidatedUnit) (key value : NTy) where
  before : Contents unit key value
  after : Contents unit key value
  entryKey : @NTy.carrier (Carriers.runtime unit) key

namespace EntryFocus

def fill (focus : EntryFocus unit key value)
    (replacement : @NTy.carrier (Carriers.runtime unit) value) : Contents unit key value :=
  focus.before.push (focus.entryKey, replacement, ()) ++ focus.after

noncomputable def encoded (focus : EntryFocus unit key value) : TableEntryFocus :=
  ⟨⟨focus.before.map (runtimeEncode (.tuple (.cons key (.cons value .nil)))),
    focus.after.map (runtimeEncode (.tuple (.cons key (.cons value .nil))))⟩,
    runtimeEncode key focus.entryKey⟩

theorem encode_fill (focus : EntryFocus unit key value) (owner : NTy)
    (replacement : @NTy.carrier (Carriers.runtime unit) value) :
    (resource owner key value).encode (focus.fill replacement) =
      focus.encoded.fill (runtimeEncode value replacement) := by
  simp [resource, ResourceType.encode, fill, encoded, TableEntryFocus.fill, VectorFocus.fill]

theorem encoded_plain (focus : EntryFocus unit key value) : focus.encoded.Plain := by
  refine ⟨⟨?_, ?_⟩, @NTy.encode_plain (Carriers.runtime unit) _ _⟩
  all_goals
    intro entry member
    obtain ⟨typed, _, rfl⟩ := Array.mem_map.mp member
    exact @NTy.encode_plain (Carriers.runtime unit) _ typed

/-- Filling a typed entry loan preserves the number and identity of its keys. -/
theorem keys_fill (focus : EntryFocus unit key value)
    (before after : @NTy.carrier (Carriers.runtime unit) value) :
    (entries key value (focus.fill before)).map Prod.fst =
      (entries key value (focus.fill after)).map Prod.fst := by
  simp [fill, entries]

theorem distinct_fill (focus : EntryFocus unit key value)
    (before after : @NTy.carrier (Carriers.runtime unit) value)
    (distinct : Maps.Distinct (entries key value (focus.fill before))) :
    Maps.Distinct (entries key value (focus.fill after)) := by
  unfold Maps.Distinct at *
  rwa [← keys_fill focus before after]

/-- Settling the entry loan restores an encoding of typed contents. The
surrounding Table values stay intact, even when a value owns nested Tables. -/
theorem reconcile (focus : EntryFocus unit key value) (owner : NTy)
    (state : RuntimeState) (slot : GlobalKey) (loan : Nat)
    (replacement : @NTy.carrier (Carriers.runtime unit) value) :
    applyWriteBack {} { state with
        tables.contents := state.tables.contents.insert slot (focus.encoded.fill (.loanHole loan))
        storageLoans := (loan, .table slot) :: state.storageLoans } loan (runtimeEncode value replacement) =
      ({}, { state with
        tables.contents :=
          (state.tables.contents.insert slot (focus.encoded.fill (.loanHole loan))).insert slot
            ((resource owner key value).encode (focus.fill replacement)) }) := by
  rw [encode_fill]
  exact TableEntryFocus.reconcile state slot loan focus.encoded_plain
    (@NTy.encode_plain (Carriers.runtime unit) _ _)

end EntryFocus

/-- Successful typed lookup yields a typed focus for the actual native borrow.
This connects key selection, extraction and subsequent typed reconciliation. -/
theorem lookup?_borrow (owner key value : NTy) (contents : Contents unit key value)
    (query : @NTy.carrier (Carriers.runtime unit) key)
    (current : @NTy.carrier (Carriers.runtime unit) value)
    (state : RuntimeState) (slot : GlobalKey)
    (stored : state.tables.contents.lookup slot = some ((resource owner key value).encode contents))
    (found : lookup? key value contents query = some current) :
    ∃ focus : EntryFocus unit key value, focus.entryKey = query ∧ contents = focus.fill current ∧
      borrowTableEntry? state slot (runtimeEncode key query) =
        some ({ state with
          tables.contents := state.tables.contents.insert slot (focus.encoded.fill (.loanHole state.nextLoan))
          storageLoans := (state.nextLoan, .table slot) :: state.storageLoans
          nextLoan := state.nextLoan + 1 }, .borrow state.nextLoan (runtimeEncode value current)) := by
  simp only [lookup?, Option.bind_eq_bind, Option.bind_eq_some_iff, Option.some.injEq] at found
  obtain ⟨index, indexed, entry, selected, returned⟩ := found
  have matched := Array.of_findIdx?_eq_some indexed
  simp only [selected] at matched
  have sameKey : entry.1 = query :=
    @NTy.encode_injective (Carriers.runtime unit) key _ _ (by simpa using matched)
  obtain ⟨bound, equal⟩ := Array.getElem?_eq_some_iff.mp selected
  let focus : EntryFocus unit key value :=
    ⟨contents.extract 0 index, contents.extract (index + 1) contents.size, query⟩
  have restored : focus.fill current = contents := by
    have entryEqual : ((query, current, ()) :
        @NTy.carrier (Carriers.runtime unit) (.tuple (.cons key (.cons value .nil)))) =
        contents[index] := by
      rw [equal, ← sameKey, ← returned]
      rcases entry with ⟨k, v, ⟨⟩⟩
      rfl
    simp only [EntryFocus.fill, focus, entryEqual]
    rw [Array.push_extract_getElem bound, Array.extract_append_extract]
    simp [Nat.max_eq_right (Nat.succ_le_of_lt bound)]
  have focusedIndex : focus.encoded.entries.index = index := by
    simp [EntryFocus.encoded, VectorFocus.index, focus, Nat.min_eq_left (Nat.le_of_lt bound)]
  refine ⟨focus, rfl, restored.symm, ?_⟩
  have borrowed := TableEntryFocus.borrow (state := state) (slot := slot)
    (focus := focus.encoded) (value := runtimeEncode value current) (by
      rw [← EntryFocus.encode_fill focus owner, restored]
      exact stored)
  rw [focusedIndex] at borrowed
  simpa only [borrowTableEntry?, stored, Option.bind_eq_bind, Option.bind_some,
    tableEntryIndex?_encode, indexed] using borrowed

end LeanerIR.Proofs.Denote.TableMemory
