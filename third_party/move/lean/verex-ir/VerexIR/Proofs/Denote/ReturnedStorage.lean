-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.Resources
import VerexIR.Semantics.LoanRenamingFunctions
import VerexIR.Semantics.Focus

/-!
# Storage observations at a returned reference

An escaping reference leaves its lender's hole in runtime storage. The
current or prophecy row supplied by a call resolves those holes to observe
the corresponding typed memory. This is an observation, not an execution
write-back: loan registries and allocation identities remain unchanged.
-/

namespace VerexIR
open SemanticOperations

private theorem fillReturned_rename {ρ : Nat → Nat} (injective : Function.Injective ρ)
    (loans : Array (Nat × RuntimeValue)) (value : RuntimeValue) :
    (loans.map fun entry => (ρ entry.1, entry.2.rename ρ)).foldl
        (fun resolved returned => (fillHole? returned.1 returned.2 resolved).getD resolved)
        (value.rename ρ) =
      (loans.foldl
        (fun resolved returned => (fillHole? returned.1 returned.2 resolved).getD resolved)
        value).rename ρ := by
  rw [Array.foldl_map, ← Array.foldl_toList, ← Array.foldl_toList]
  induction loans.toList generalizing value with
  | nil => rfl
  | cons head tail ih =>
      simp only [List.foldl_cons, fillHole?_rename injective]
      rw [Option.getD_map]
      exact ih _

theorem resolveReturnedBorrows_rename {ρ : Nat → Nat} (injective : Function.Injective ρ)
    (results : Array RuntimeValue) (value : RuntimeValue) :
    resolveReturnedBorrows (results.map (·.rename ρ)) (value.rename ρ) =
      (resolveReturnedBorrows results value).rename ρ := by
  unfold resolveReturnedBorrows
  rw [Array.foldl_map, ← Array.foldl_toList, ← Array.foldl_toList]
  induction results.toList generalizing value with
  | nil => rfl
  | cons head tail ih =>
      simp only [List.foldl_cons, outermostBorrows_rename, fillReturned_rename injective]
      simp only [outermostBorrows_rename] at ih
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

theorem GlobalMap.resolveReturned_rename {ρ : Nat → Nat} (injective : Function.Injective ρ)
    (results : Array RuntimeValue) (heap : GlobalMap) :
    (heap.rename ρ).resolveReturned (results.map (·.rename ρ)) =
      (heap.resolveReturned results).rename ρ := by
  simp only [GlobalMap.resolveReturned, GlobalMap.rename, Array.map_map, GlobalMap.mk.injEq]
  apply Array.map_congr_left
  intro slot _
  simp only [Function.comp_apply, resolveReturnedBorrows_rename injective]

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

/-- A run's exit, its returned loans resolved at the row supplied: stores
holding the final memory, and possibly tables of other frames. -/
def Proofs.Denote.StorageHoldsReturned (unit : Validation.ValidatedUnit)
    (memory : Proofs.Denote.Memory unit) (state : RuntimeState)
    (results : Array RuntimeValue) : Prop :=
  Proofs.Denote.StorageHolds unit memory (state.resolveReturned results)

theorem Proofs.Denote.StorageEncodesReturned.holds {unit : Validation.ValidatedUnit}
    {memory : Proofs.Denote.Memory unit} {state : RuntimeState} {results : Array RuntimeValue}
    (encodes : Proofs.Denote.StorageEncodesReturned unit memory state results) :
    Proofs.Denote.StorageHoldsReturned unit memory state results :=
  Proofs.Denote.StorageEncodes.holds encodes

/-- A run's exit, its returned loans resolved at the row supplied: stores
holding the final memory, and the tables the results and the arguments'
prophecies hold in native storage. -/
def Proofs.Denote.ReturnedEncloses (unit : Validation.ValidatedUnit) [Proofs.Denote.Skolems unit]
    (memory : Proofs.Denote.Memory unit) (state : RuntimeState) (results : Array RuntimeValue)
    (tables : List (Proofs.Denote.NTy × String × RuntimeValue)) : Prop :=
  Proofs.Denote.Encloses unit memory (state.resolveReturned results) tables

theorem Proofs.Denote.ReturnedEncloses.stores {unit : Validation.ValidatedUnit}
    [Proofs.Denote.Skolems unit] {memory : Proofs.Denote.Memory unit} {state : RuntimeState}
    {results : Array RuntimeValue} {tables : List (Proofs.Denote.NTy × String × RuntimeValue)}
    (held : Proofs.Denote.ReturnedEncloses unit memory state results tables) :
    Proofs.Denote.StorageHoldsReturned unit memory state results :=
  Proofs.Denote.Encloses.stores held

theorem Proofs.Denote.ReturnedEncloses.of_nil {unit : Validation.ValidatedUnit}
    [Proofs.Denote.Skolems unit] {memory : Proofs.Denote.Memory unit} {state : RuntimeState}
    {results : Array RuntimeValue}
    (stores : Proofs.Denote.StorageHoldsReturned unit memory state results)
    {tables : List (Proofs.Denote.NTy × String × RuntimeValue)} (nil : tables = []) :
    Proofs.Denote.ReturnedEncloses unit memory state results tables :=
  Proofs.Denote.Encloses.of_nil stores nil

theorem Proofs.Denote.ReturnedEncloses.instantiate {unit : Validation.ValidatedUnit}
    (θ : Proofs.Denote.TypeArgs) (Θ : Proofs.Denote.Skolems unit) (memory : Proofs.Denote.Memory unit)
    (state : RuntimeState) (results : Array RuntimeValue)
    (tables : List (Proofs.Denote.NTy × String × RuntimeValue)) :
    @Proofs.Denote.ReturnedEncloses unit (Proofs.Denote.Skolems.instantiate θ Θ) memory state results
        tables ↔
      @Proofs.Denote.ReturnedEncloses unit Θ memory state results
        (tables.map (Proofs.Denote.tableSubst θ)) :=
  Proofs.Denote.Encloses.instantiate θ Θ memory (state.resolveReturned results) tables

theorem RuntimeState.resolveReturned_plain (results : Array RuntimeValue)
    (plain : ∀ result ∈ results, Plain result) (state : RuntimeState) :
    state.resolveReturned results = state := by
  simp only [RuntimeState.resolveReturned, GlobalMap.resolveReturned_plain results plain]

/-- Resolving returned loans commutes with renaming them. -/
theorem RuntimeState.resolveReturned_rename {ρ : Nat → Nat} (injective : Function.Injective ρ)
    {state state' : RuntimeState} (globals : state'.globals = state.globals.rename ρ)
    (tables : state'.tables = state.tables.rename ρ) (results : Array RuntimeValue) :
    (state'.resolveReturned (results.map (·.rename ρ))).globals =
        (state.resolveReturned results).globals.rename ρ ∧
    (state'.resolveReturned (results.map (·.rename ρ))).tables =
        (state.resolveReturned results).tables.rename ρ := by
  constructor
  · simp only [RuntimeState.resolveReturned, globals, GlobalMap.resolveReturned_rename injective]
  · simp only [RuntimeState.resolveReturned, tables, NativeTableStorage.rename,
      GlobalMap.resolveReturned_rename injective]

end VerexIR
