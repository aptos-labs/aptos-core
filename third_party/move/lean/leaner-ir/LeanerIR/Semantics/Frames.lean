-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Semantics.Operations

/-!
# Frames of generic calls

A call's frame is the type instantiation the runtime computes from the
caller's frame and the call's type arguments (`callTypeInstantiation`). The
substitution reads its arguments only through the types and lifetimes they
give the parameters, and a frame gives each parameter its argument; so a
call passing its caller's own type parameters, in order, from the caller's
namespace, runs in the caller's frame.
-/

namespace LeanerIR.SemanticOperations
open Validation

/-- The type a list of generic arguments gives a type parameter. -/
def argumentTypeId? (arguments : Array GenericArgument) (index : Nat) : Option TypeId :=
  match arguments[index]? with
  | some (.typeArg value) => some value.typeId
  | _ => none

/-- The lifetime a list of generic arguments gives a lifetime parameter. -/
def argumentLifetime? (arguments : Array GenericArgument) (index : Nat) : Option LifetimeId :=
  match arguments[index]? with
  | some (.lifetime value) => some value
  | _ => none

/-- Lifetime substitution reads its arguments only through their lifetimes. -/
theorem instantiateLifetime?_congr {ns : ValidatedNamespace} {a b : Array GenericArgument}
    (lifetimes : ∀ index, argumentLifetime? a index = argumentLifetime? b index) :
    instantiateLifetime? ns a = instantiateLifetime? ns b := by
  funext lifetime
  unfold instantiateLifetime?
  cases ns.tables.lifetimes[lifetime.index]? with
  | none => rfl
  | some declaration =>
    simp only [Option.bind_eq_bind, Option.bind_some]
    cases declaration.kind with
    | parameter index =>
      have := lifetimes index
      simp only [argumentLifetime?] at this
      exact this
    | _ => rfl

/-- Type substitution reads its arguments only through the types and
lifetimes they give the parameters. -/
theorem instantiatePlaceFieldTypeFuel?_congr {ns : ValidatedNamespace} {a b : Array GenericArgument}
    (types : ∀ index, argumentTypeId? a index = argumentTypeId? b index)
    (lifetimes : ∀ index, argumentLifetime? a index = argumentLifetime? b index) :
    ∀ fuel, instantiatePlaceFieldTypeFuel? ns a fuel = instantiatePlaceFieldTypeFuel? ns b fuel
  | 0 => by funext typeId; simp [instantiatePlaceFieldTypeFuel?]
  | fuel + 1 => by
    have recursive := instantiatePlaceFieldTypeFuel?_congr (ns := ns) types lifetimes fuel
    funext typeId
    simp only [instantiatePlaceFieldTypeFuel?, recursive, instantiateLifetime?_congr lifetimes]
    cases ns.tables.types[typeId.index]? with
    | none => rfl
    | some type =>
      simp only [Option.bind_eq_bind, Option.bind_some]
      cases type with
      | typeParameter index =>
        have := types index
        simp only [argumentTypeId?] at this
        exact this
      | _ => rfl

/-- A frame's entry for one type: its instance, where that differs. -/
def frameEntry (ns : ValidatedNamespace) (arguments : Array GenericArgument) (symbolic : TypeId) :
    Option (TypeId × TypeId) :=
  match instantiatePlaceFieldType? ns arguments symbolic with
  | some concrete => if concrete == symbolic then none else some (symbolic, concrete)
  | none => none

/-- One step of building a frame: the entry of one type, if any. -/
def frameStep (ns : ValidatedNamespace) (arguments : Array GenericArgument)
    (result : Array (TypeId × TypeId)) (index : Nat) : Array (TypeId × TypeId) :=
  match frameEntry ns arguments ⟨index⟩ with
  | some entry => result.push entry
  | none => result

theorem frameEntry_key {ns : ValidatedNamespace} {arguments : Array GenericArgument}
    {symbolic : TypeId} {entry : TypeId × TypeId}
    (found : frameEntry ns arguments symbolic = some entry) : entry.1 = symbolic := by
  unfold frameEntry at found
  split at found
  · split at found
    · cases found
    · cases found; rfl
  · cases found

theorem invocationTypeInstantiation_eq_frameStep (ns : ValidatedNamespace)
    (outer : Array (TypeId × TypeId)) (arguments : Array GenericArgument) :
    invocationTypeInstantiation ns outer arguments =
      (List.range ns.tables.types.size).foldl
        (frameStep ns (instantiateGenericArguments outer arguments)) #[] := by
  unfold invocationTypeInstantiation
  rw [← Array.foldl_toList, Array.toList_range]
  congr 1
  funext result index
  unfold frameStep frameEntry
  cases found : instantiatePlaceFieldType? ns (instantiateGenericArguments outer arguments)
      ⟨index⟩ with
  | none => simp [found]
  | some concrete => by_cases same : concrete = ⟨index⟩ <;> simp [found, same]

theorem frameStep_of_entry {ns : ValidatedNamespace} {arguments : Array GenericArgument}
    {result : Array (TypeId × TypeId)} {index : Nat} {entry : TypeId × TypeId}
    (found : frameEntry ns arguments ⟨index⟩ = some entry) :
    frameStep ns arguments result index = result.push entry := by
  unfold frameStep; rw [found]

theorem frameStep_of_none {ns : ValidatedNamespace} {arguments : Array GenericArgument}
    {result : Array (TypeId × TypeId)} {index : Nat}
    (found : frameEntry ns arguments ⟨index⟩ = none) :
    frameStep ns arguments result index = result := by
  unfold frameStep; rw [found]

/-- A frame built over the first `count` types has the entry of each of them. -/
theorem find?_frame (ns : ValidatedNamespace) (arguments : Array GenericArgument) (typeId : TypeId) :
    ∀ count, ((List.range count).foldl (frameStep ns arguments) #[]).find?
        (fun entry => entry.1 == typeId) =
      if typeId.index < count then frameEntry ns arguments typeId else none
  | 0 => by simp
  | count + 1 => by
    rw [List.range_succ, List.foldl_append, List.foldl_cons, List.foldl_nil]
    have previous := find?_frame ns arguments typeId count
    by_cases same : typeId.index = count
    · have : typeId = ⟨count⟩ := by cases typeId; simp_all
      subst this
      rw [if_neg (Nat.lt_irrefl _)] at previous
      rw [if_pos (Nat.lt_succ_self _)]
      cases found : frameEntry ns arguments ⟨count⟩ with
      | some entry =>
        rw [frameStep_of_entry found, Array.find?_push, previous, frameEntry_key found]
        simp
      | none => rw [frameStep_of_none found, previous]
    · have below : (typeId.index < count + 1) ↔ (typeId.index < count) := by omega
      rw [show (if typeId.index < count + 1 then frameEntry ns arguments typeId else none) =
          (if typeId.index < count then frameEntry ns arguments typeId else none) by
        by_cases lt : typeId.index < count
        · rw [if_pos lt, if_pos (below.mpr lt)]
        · rw [if_neg lt, if_neg (fun h => lt (below.mp h))], ← previous]
      cases found : frameEntry ns arguments ⟨count⟩ with
      | some entry =>
        rw [frameStep_of_entry found, Array.find?_push]
        have key : (entry.1 == typeId) = false := by
          rw [frameEntry_key found]
          simp only [beq_eq_false_iff_ne]; intro h; subst h; exact same rfl
        simp [key]
      | none => rw [frameStep_of_none found]

/-- A frame reads each type of its namespace at its instance. -/
theorem instantiatedTypeId_invocation (ns : ValidatedNamespace) (outer : Array (TypeId × TypeId))
    (arguments : Array GenericArgument) (typeId : TypeId)
    (bound : typeId.index < ns.tables.types.size) :
    instantiatedTypeId (invocationTypeInstantiation ns outer arguments) typeId =
      (instantiatePlaceFieldType? ns (instantiateGenericArguments outer arguments) typeId).getD
        typeId := by
  unfold instantiatedTypeId
  rw [invocationTypeInstantiation_eq_frameStep, find?_frame, if_pos bound]
  unfold frameEntry
  split
  · rename_i concrete found
    rw [found]
    split
    · rename_i equal
      simp only [Option.map_none, Option.getD_none, Option.getD_some]
      exact (beq_iff_eq.mp equal).symm
    · simp
  · rename_i found
    rw [found]
    simp

theorem instantiateGenericArguments_typeArgs (outer : Array (TypeId × TypeId))
    (uses : Array TypeUse) :
    instantiateGenericArguments outer (uses.map .typeArg) =
      uses.map fun use => .typeArg { use with typeId := instantiatedTypeId outer use.typeId } := by
  simp [instantiateGenericArguments, Array.map_map, Function.comp_def]

/-- A frame depends on its arguments only through the types and lifetimes
they give the parameters. -/
theorem invocationTypeInstantiation_congr (ns : ValidatedNamespace)
    (outer outer' : Array (TypeId × TypeId)) (arguments arguments' : Array GenericArgument)
    (types : ∀ index, argumentTypeId? (instantiateGenericArguments outer arguments) index =
      argumentTypeId? (instantiateGenericArguments outer' arguments') index)
    (lifetimes : ∀ index, argumentLifetime? (instantiateGenericArguments outer arguments) index =
      argumentLifetime? (instantiateGenericArguments outer' arguments') index) :
    invocationTypeInstantiation ns outer arguments =
      invocationTypeInstantiation ns outer' arguments' := by
  rw [invocationTypeInstantiation_eq_frameStep, invocationTypeInstantiation_eq_frameStep]
  have same : instantiatePlaceFieldType? ns (instantiateGenericArguments outer arguments) =
      instantiatePlaceFieldType? ns (instantiateGenericArguments outer' arguments') := by
    funext typeId
    unfold instantiatePlaceFieldType?
    rw [instantiatePlaceFieldTypeFuel?_congr types lifetimes]
  unfold frameStep frameEntry
  simp only [same]

/-- A call of a function with the type parameters of its caller, in order,
from the caller's namespace, runs in the caller's frame: that frame gives
each parameter the caller's argument, so the call's arguments read through
it are the caller's own. -/
theorem callTypeInstantiation_own_parameters (unit : ValidatedUnit) (ns : ValidatedNamespace)
    (caller callee : FunctionHandle) (outer : Array (TypeId × TypeId))
    (arguments own : Array TypeUse)
    (namespaceOf : unit.namespaces[caller.namespaceId.index]? = some ns)
    (same : callee.namespaceId = caller.namespaceId)
    (sizes : own.size = arguments.size)
    (parameters : ∀ (index : Nat) (bound : index < own.size),
      ns.tables.types[own[index].typeId.index]? = some (.typeParameter index)) :
    callTypeInstantiation unit callee (callTypeInstantiation unit caller outer
        (arguments.map .typeArg)) (own.map .typeArg) =
      callTypeInstantiation unit caller outer (arguments.map .typeArg) := by
  by_cases empty : arguments.size = 0
  · have ownEmpty : own.size = 0 := by omega
    simp [callTypeInstantiation, Array.isEmpty, empty, ownEmpty]
  have ownNonempty : own.size ≠ 0 := by omega
  simp only [callTypeInstantiation, Array.isEmpty_iff_size_eq_zero, Array.size_map, empty,
    ownNonempty, if_false, same, namespaceOf]
  apply invocationTypeInstantiation_congr
  · intro index
    rw [instantiateGenericArguments_typeArgs, instantiateGenericArguments_typeArgs]
    simp only [argumentTypeId?, Array.getElem?_map]
    by_cases bound : index < own.size
    · have parameter := parameters index bound
      have argumentBound : index < arguments.size := sizes ▸ bound
      rw [Array.getElem?_eq_getElem bound, Array.getElem?_eq_getElem argumentBound]
      simp only [Option.map_some, Option.some.injEq]
      have typeBound : own[index].typeId.index < ns.tables.types.size := by
        have := Array.getElem?_eq_some_iff.mp parameter
        exact this.1
      rw [instantiatedTypeId_invocation ns outer _ _ typeBound, instantiateGenericArguments_typeArgs]
      unfold instantiatePlaceFieldType?
      simp only [instantiatePlaceFieldTypeFuel?, parameter, Option.bind_eq_bind, Option.bind_some,
        Array.getElem?_map, Array.getElem?_eq_getElem argumentBound, Option.map_some,
        Option.getD_some]
    · have argumentBound : ¬ index < arguments.size := sizes ▸ bound
      rw [Array.getElem?_eq_none (by omega), Array.getElem?_eq_none (by omega)]
      rfl
  · intro index
    rw [instantiateGenericArguments_typeArgs, instantiateGenericArguments_typeArgs]
    simp only [argumentLifetime?, Array.getElem?_map]
    cases own[index]? <;> cases arguments[index]? <;> rfl

/-- Whether `own` are the type parameters of a namespace, in order. -/
def ownParameters (ns : ValidatedNamespace) (own : Array TypeUse) : Bool :=
  (List.range own.size).all fun index =>
    match own[index]? with
    | some use => ns.tables.types[use.typeId.index]? == some (.typeParameter index)
    | none => false

theorem ownParameters_spec {ns : ValidatedNamespace} {own : Array TypeUse}
    (checked : ownParameters ns own = true) (index : Nat) (bound : index < own.size) :
    ns.tables.types[own[index].typeId.index]? = some (.typeParameter index) := by
  unfold ownParameters at checked
  have := List.all_eq_true.mp checked index (List.mem_range.mpr bound)
  simp only [Array.getElem?_eq_getElem bound] at this
  exact beq_iff_eq.mp this

/-- Whether `own` are the type parameters of a function's namespace, in order. -/
def ownParametersIn (unit : ValidatedUnit) (handle : FunctionHandle) (own : Array TypeUse) : Bool :=
  match unit.namespaces[handle.namespaceId.index]? with
  | some ns => ownParameters ns own
  | none => false

end LeanerIR.SemanticOperations
