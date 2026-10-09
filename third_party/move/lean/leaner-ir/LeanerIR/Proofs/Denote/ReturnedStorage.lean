-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.Denote.Resources
import LeanerIR.Semantics.LoanRenamingFunctions
import LeanerIR.Semantics.Focus

/-!
# Storage observations at a returned reference

An escaping reference leaves its lender's hole in runtime storage. The
current or prophecy row supplied by a call resolves those holes to observe
the corresponding typed memory. This is an observation, not an execution
write-back: loan registries and allocation identities remain unchanged.
-/

namespace LeanerIR
open SemanticOperations

private theorem fillReturned_shift (offset : Nat) (loans : Array (Nat × RuntimeValue))
    (value : RuntimeValue) :
    (loans.map fun entry => (entry.1 + offset, entry.2.shift offset)).foldl
        (fun resolved returned => (fillHole? returned.1 returned.2 resolved).getD resolved)
        (value.shift offset) =
      (loans.foldl
        (fun resolved returned => (fillHole? returned.1 returned.2 resolved).getD resolved)
        value).shift offset := by
  rw [Array.foldl_map, ← Array.foldl_toList, ← Array.foldl_toList]
  induction loans.toList generalizing value with
  | nil => rfl
  | cons head tail ih =>
      simp only [List.foldl_cons, fillHole?_shift]
      rw [Option.getD_map]
      exact ih _

theorem resolveReturnedBorrows_shift (offset : Nat) (results : Array RuntimeValue)
    (value : RuntimeValue) :
    resolveReturnedBorrows (results.map (·.shift offset)) (value.shift offset) =
      (resolveReturnedBorrows results value).shift offset := by
  unfold resolveReturnedBorrows
  rw [Array.foldl_map, ← Array.foldl_toList, ← Array.foldl_toList]
  induction results.toList generalizing value with
  | nil => rfl
  | cons head tail ih =>
      simp only [List.foldl_cons, outermostBorrows_shift, fillReturned_shift]
      simp only [outermostBorrows_shift] at ih
      exact ih _

theorem resolveReturnedBorrows_plain (results : Array RuntimeValue)
    (plain : ∀ result ∈ results, Plain result) (value : RuntimeValue) :
    resolveReturnedBorrows results value = value := by
  unfold resolveReturnedBorrows
  rw [← Array.foldl_toList]
  have all : ∀ result ∈ results.toList, Plain result := by simpa using plain
  generalize results.toList = list at all ⊢
  induction list generalizing value with
  | nil => rfl
  | cons head tail ih =>
      simp only [List.foldl_cons,
        Plain.outermostBorrows_eq_empty (all head (by simp)), Array.foldl_empty]
      exact ih value (fun result member => all result (by simp [member]))

/-- Returned references cannot alter a stored value that contains no loan
holes. The returned row itself may contain arbitrary references. -/
theorem resolveReturnedBorrows_of_plain (results : Array RuntimeValue)
    {value : RuntimeValue} (plain : Plain value) :
    resolveReturnedBorrows results value = value := by
  have fill (loans : Array (Nat × RuntimeValue)) :
      loans.foldl
        (fun resolved returned => (fillHole? returned.1 returned.2 resolved).getD resolved)
        value = value := by
    rw [← Array.foldl_toList]
    induction loans.toList with
    | nil => rfl
    | cons head tail ih =>
        simpa only [List.foldl_cons, fillHole?,
          rewriteFirst_eq_none_of_plain (LoanMatcher.holeFill? head.1 head.2) plain,
          Option.getD_none] using ih
  unfold resolveReturnedBorrows
  rw [← Array.foldl_toList]
  induction results.toList with
  | nil => rfl
  | cons head tail ih => simpa only [List.foldl_cons, fill] using ih

/-- Observe heap values after resolving the returned loans at their supplied view. -/
def GlobalMap.resolveReturned (results : Array RuntimeValue) (heap : GlobalMap) : GlobalMap :=
  ⟨heap.entries.map fun slot =>
    { slot with value := resolveReturnedBorrows results slot.value }⟩

theorem GlobalMap.lookup_resolveReturned (results : Array RuntimeValue)
    (heap : GlobalMap) (key : GlobalKey) :
    (heap.resolveReturned results).lookup key =
      (heap.lookup key).map (resolveReturnedBorrows results) := by
  unfold GlobalMap.lookup GlobalMap.resolveReturned
  simp only [Array.find?_map, Option.map_map]
  rfl

theorem GlobalMap.sorted_resolveReturned (results : Array RuntimeValue)
    {heap : GlobalMap} (sorted : heap.Sorted) : (heap.resolveReturned results).Sorted := by
  simpa [GlobalMap.Sorted, GlobalMap.resolveReturned, List.map_map, Function.comp_def] using sorted

theorem GlobalMap.sorted_resolveReturned_iff (results : Array RuntimeValue)
    (heap : GlobalMap) : (heap.resolveReturned results).Sorted ↔ heap.Sorted := by
  simp [GlobalMap.Sorted, GlobalMap.resolveReturned, List.map_map, Function.comp_def]

/-- Resolving loans commutes with replacing a slot, resolving its new value
in the same observation. No assumption on the returned row is needed. -/
theorem GlobalMap.resolveReturned_insert (results : Array RuntimeValue)
    {heap : GlobalMap} (sorted : heap.Sorted) (key : GlobalKey) (value : RuntimeValue) :
    (heap.insert key value).resolveReturned results =
      (heap.resolveReturned results).insert key (resolveReturnedBorrows results value) := by
  apply GlobalMap.ext (sorted_resolveReturned results (sorted.insert _ _))
    ((sorted_resolveReturned results sorted).insert _ _)
  intro query
  by_cases equal : query = key
  · subst query
    simp [lookup_resolveReturned]
  · simp [lookup_resolveReturned, equal]

theorem GlobalMap.resolveReturned_erase (results : Array RuntimeValue)
    {heap : GlobalMap} (sorted : heap.Sorted) (key : GlobalKey) :
    (heap.erase key).resolveReturned results = (heap.resolveReturned results).erase key := by
  apply GlobalMap.ext (sorted_resolveReturned results (sorted.erase _))
    ((sorted_resolveReturned results sorted).erase _)
  intro query
  by_cases equal : query = key
  · subst query
    simp [lookup_resolveReturned]
  · simp [lookup_resolveReturned, equal]

theorem GlobalMap.resolveReturned_shift (offset : Nat) (results : Array RuntimeValue)
    (heap : GlobalMap) :
    (heap.shift offset).resolveReturned (results.map (·.shift offset)) =
      (heap.resolveReturned results).shift offset := by
  simp only [GlobalMap.resolveReturned, GlobalMap.shift, Array.map_map, GlobalMap.mk.injEq]
  apply Array.map_congr_left
  intro slot _
  simp only [Function.comp_apply, resolveReturnedBorrows_shift]

theorem GlobalMap.resolveReturned_empty (heap : GlobalMap) :
    heap.resolveReturned #[] = heap := by
  cases heap with
  | mk entries =>
    simp only [GlobalMap.resolveReturned, resolveReturnedBorrows_empty]
    change GlobalMap.mk (entries.map id) = GlobalMap.mk entries
    rw [Array.map_id]

theorem GlobalMap.resolveReturned_plain (results : Array RuntimeValue)
    (plain : ∀ result ∈ results, Plain result) (heap : GlobalMap) :
    heap.resolveReturned results = heap := by
  cases heap with
  | mk entries =>
    simp only [GlobalMap.resolveReturned, resolveReturnedBorrows_plain results plain]
    change GlobalMap.mk (entries.map id) = GlobalMap.mk entries
    rw [Array.map_id]

/-- Observe returned loan holes at the current or prophecy row supplied by a call. -/
def RuntimeState.resolveReturned (results : Array RuntimeValue) (state : RuntimeState) :
    RuntimeState :=
  { state with
    globals := state.globals.resolveReturned results
    tables := { state.tables with contents := state.tables.contents.resolveReturned results } }

def Proofs.Denote.StorageEncodesReturned (unit : Validation.ValidatedUnit)
    (memory : Proofs.Denote.Memory unit) (state : RuntimeState)
    (results : Array RuntimeValue) : Prop :=
  Proofs.Denote.StorageEncodes unit memory (state.resolveReturned results)

theorem RuntimeState.resolveReturned_plain (results : Array RuntimeValue)
    (plain : ∀ result ∈ results, Plain result) (state : RuntimeState) :
    state.resolveReturned results = state := by
  simp only [RuntimeState.resolveReturned, GlobalMap.resolveReturned_plain results plain]

theorem StateShifted.resolveReturned_stores {offset frontier inert inert' : Nat}
    {state state' : RuntimeState} (shifted : StateShifted offset frontier inert inert' state state')
    (results : Array RuntimeValue) :
    (state'.resolveReturned (results.map (·.shift offset))).globals =
        (state.resolveReturned results).globals.shift offset ∧
    (state'.resolveReturned (results.map (·.shift offset))).tables =
        (state.resolveReturned results).tables.shift offset := by
  constructor
  · simp only [RuntimeState.resolveReturned, shifted.globals, GlobalMap.resolveReturned_shift]
  · simp only [RuntimeState.resolveReturned, shifted.tables, NativeTableStorage.shift,
      GlobalMap.resolveReturned_shift]

end LeanerIR
