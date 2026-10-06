-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.Denote.Types
import LeanerIR.Semantics.ValueTyping

/-!
# Resource types of a unit

The native types the unit's type tables denote, and the resource type each
runtime storage key names: how typed global memory (`Memory`) relates to the
runtime's (`designs/static-memory.md`).  A runtime key is the declaring
namespace with a type identifier of its table; the memory slot it names is
the resource type that identifier denotes.
-/

namespace LeanerIR.Proofs.Denote

open LeanerIR.Validation LeanerIR.SemanticOperations

variable {unit : ValidatedUnit}

/-- The resource type a runtime key's type identifier denotes in its
namespace: a `key` declaration's native type and its type arguments. -/
def resourceOf (unit : ValidatedUnit) (namespaceId : NamespaceId) (typeId : TypeId) :
    Option ResourceType := do
  let ty ← unitTypes unit namespaceId typeId
  match ty with
  | .nominal name arguments =>
      let ns ← unit.namespaces[namespaceId.index]?
      let handle ← resolveNominal? unit ns name
      let declaration ← unit.namespaces[handle.namespaceId.index]?.bind
        (·.structs[handle.structId]?)
      guard (declaration.abilities.contains .key)
      let type ← ntyOf unit namespaceId typeId
      let arguments ← arguments.toList.mapM fun argument => match argument with
        | .typeArg value => ntyOf unit namespaceId value.typeId
        | _ => none
      some ⟨type, NRow.ofList arguments⟩
  | _ => none

/-- Runtime global memory holding exactly what a typed memory holds: under
every key, the encoding of the value at the resource type its type
identifier denotes, and nothing under a key that denotes none.  Its entries
are in key order, so the memory determines it. -/
def Encodes (unit : ValidatedUnit) (memory : Memory unit) (globals : GlobalMap) : Prop :=
  globals.Sorted ∧ ∀ namespaceId typeId key, globals.lookup ⟨namespaceId, typeId, key⟩ =
    match resourceOf unit namespaceId typeId with
    | some resource => (memory resource key).map (@NTy.encode (Carriers.runtime unit) resource.type)
    | none => none

/-- A memory's runtime encoding is unique. -/
theorem Encodes.unique {unit : ValidatedUnit} {memory : Memory unit} {left right : GlobalMap}
    (encodes : Encodes unit memory left) (encodes' : Encodes unit memory right) : left = right :=
  GlobalMap.ext encodes.1 encodes'.1 fun ⟨namespaceId, typeId, key⟩ => by
    rw [encodes.2, encodes'.2]

/-- A resource type no runtime key of the unit names. -/
def Unnamed (unit : ValidatedUnit) (resource : ResourceType) : Prop :=
  ∀ namespaceId typeId, resourceOf unit namespaceId typeId ≠ some resource

/-- Two memories that agree on every resource type no runtime key names. -/
def AgreeUnnamed (unit : ValidatedUnit) (left right : Memory unit) : Prop :=
  ∀ resource, Unnamed unit resource → left resource = right resource

/-! ## Data invariants of stored resources -/

/-- The declaration a native type is an instance of. -/
def NTy.handle? : NTy → Option StructHandle
  | .struct source _ _ => some source
  | .enum source _ _ _ _ => some source
  | _ => none

@[simp] theorem NTy.handle?_struct (source : StructHandle) (arguments fields : NRow) :
    (NTy.struct source arguments fields).handle? = some source := rfl

@[simp] theorem NTy.handle?_enum (source : StructHandle) (arguments : NRow) (names : List String)
    (rows : NRows) (distinct : names.Nodup) :
    (NTy.enum source arguments names rows distinct).handle? = some source := rfl

/-- Every value stored in memory satisfies the data invariant of its
resource type, read over its encoding, as the Move Prover assumes of the
memory a function accesses (`designs/static-memory.md`, "Contracts"). The
invariants read no memory, so a write keeps those of the values it leaves
alone. -/
def MemoryInvariants (invariant : ResourceType → RuntimeValue → Prop)
    (memory : Memory unit) : Prop :=
  ∀ resource key value, memory resource key = some value →
    invariant resource (@NTy.encode (Carriers.runtime unit) resource.type value)

namespace MemoryInvariants

variable {invariant : ResourceType → RuntimeValue → Prop} {memory : Memory unit}

/-- A write keeps the invariants when the value written satisfies its own. -/
theorem set (holds : MemoryInvariants invariant memory) {resource : ResourceType}
    {key : StorageKey} {value : Option (resource.carrier unit)}
    (stored : ∀ written, value = some written →
      invariant resource (@NTy.encode (Carriers.runtime unit) resource.type written)) :
    MemoryInvariants invariant (memory.set resource key value) := by
  intro other otherKey held read
  by_cases same : other = resource
  · subst same
    rw [Memory.set_same] at read
    split at read
    · exact stored held read
    · exact holds _ _ _ read
  · rw [Memory.set_other _ _ _ _ same] at read
    exact holds _ _ _ read

/-- Publishing or writing a value keeps the invariants when it satisfies
its own. -/
theorem set_some (holds : MemoryInvariants invariant memory) {resource : ResourceType}
    {key : StorageKey} {value : resource.carrier unit}
    (stored : invariant resource (@NTy.encode (Carriers.runtime unit) resource.type value)) :
    MemoryInvariants invariant (memory.set resource key (some value)) :=
  holds.set fun _ written => by cases written; exact stored

/-- Removing a value keeps the invariants. -/
theorem set_none (holds : MemoryInvariants invariant memory) {resource : ResourceType}
    {key : StorageKey} : MemoryInvariants invariant (memory.set resource key none) :=
  holds.set fun _ written => nomatch written

/-- A value read from memory satisfies the invariant of its resource type. -/
theorem read (holds : MemoryInvariants invariant memory) {resource : ResourceType}
    {key : StorageKey} {value : resource.carrier unit} (read : memory resource key = some value) :
    invariant resource (@NTy.encode (Carriers.runtime unit) resource.type value) :=
  holds resource key value read

end MemoryInvariants

mutual
/-- Whether a type mentions only type parameters below a bound. -/
def NTy.paramsBelow (bound : Nat) : NTy → Bool
  | .param index => index < bound
  | .tuple elements => NRow.paramsBelow bound elements
  | .struct _ arguments fields =>
      NRow.paramsBelow bound arguments && NRow.paramsBelow bound fields
  | .enum _ arguments _ rows _ =>
      NRow.paramsBelow bound arguments && NRows.paramsBelow bound rows
  | .vector element => NTy.paramsBelow bound element
  | .ref referent => NTy.paramsBelow bound referent
  | .function parameters _ results =>
      NRow.paramsBelow bound parameters && NRow.paramsBelow bound results
  | .unit | .bool | .int _ _ | .address | .signer | .string | .bytes => true

def NRow.paramsBelow (bound : Nat) : NRow → Bool
  | .nil => true
  | .cons τ rest => NTy.paramsBelow bound τ && NRow.paramsBelow bound rest

def NRows.paramsBelow (bound : Nat) : NRows → Bool
  | .nil => true
  | .cons fields rest => NRow.paramsBelow bound fields && NRows.paramsBelow bound rest
end

/-- The number of generic binders a function declares: its type parameters
are numbered below it. -/
def typeArity (unit : ValidatedUnit) (handle : FunctionHandle) : Nat :=
  match unit.namespaces[handle.namespaceId.index]? with
  | some ns => match ns.functions[handle.functionId.index]? with
    | some declaration => declaration.signature.generics.size
    | none => 0
  | none => 0

/-- A row with each type replaced. -/
@[reducible] def NRow.map (f : NTy → NTy) : NRow → NRow
  | .nil => .nil
  | .cons τ rest => .cons (f τ) (NRow.map f rest)

section Frames
variable [Θ : Skolems unit]

/-- A frame's resource type at the runtime family. -/
@[reducible] def Skolems.resource (type : NTy) (arguments : NRow) : ResourceType :=
  ⟨Θ.resolve type, arguments.map Θ.resolve⟩


/-- A function's frame agrees with its runtime type instantiation on the
types it reads, as the runtime reads them: each type it requires
(`requiredAt`), instantiated as the runtime does, is the frame's resolution
of it, as a native type and, for a resource type, with its arguments; and
each of its type parameters, read at its node through the instantiation, is
the frame's type for it.  The runtime instantiation maps only interned
types, and static typing interns the required ones. -/
structure Coherent (unit : ValidatedUnit) [Skolems unit] (handle : FunctionHandle)
    (typeInstantiation : Array (TypeId × TypeId)) : Prop where
  resources : ∀ typeId ∈ requiredAt unit handle, ∀ resource,
    resourceOf unit handle.namespaceId typeId = some resource →
    resource.type.paramsBelow (typeArity unit handle) = true →
    resource.arguments.paramsBelow (typeArity unit handle) = true →
    resourceOf unit handle.namespaceId (instantiatedTypeId typeInstantiation typeId) =
      some (Skolems.resource resource.type resource.arguments)
  types : ∀ typeId ∈ requiredAt unit handle, ∀ τ,
    ntyOf unit handle.namespaceId typeId = some τ →
    τ.paramsBelow (typeArity unit handle) = true →
    ntyOf unit handle.namespaceId (instantiatedTypeId typeInstantiation typeId) =
      some (Skolems.resolve τ)
  params : ∀ index < typeArity unit handle, ∀ node,
    paramNodeIn? unit handle.namespaceId index = some node →
    ntyOf unit handle.namespaceId (instantiatedTypeId typeInstantiation node) =
      some (Skolems.resolve (.param index))

/-- Whether a frame agrees with a runtime type instantiation on a function's
required types and type parameters, decided by evaluation. -/
def coherentCheck (unit : ValidatedUnit) [Skolems unit] (handle : FunctionHandle)
    (typeInstantiation : Array (TypeId × TypeId)) : Bool :=
  (requiredAt unit handle).toList.all (fun typeId =>
    (match resourceOf unit handle.namespaceId typeId with
      | none => true
      | some resource =>
          !(resource.type.paramsBelow (typeArity unit handle) &&
              resource.arguments.paramsBelow (typeArity unit handle)) ||
            decide (resourceOf unit handle.namespaceId
                (instantiatedTypeId typeInstantiation typeId) =
              some (Skolems.resource resource.type resource.arguments))) &&
    (match ntyOf unit handle.namespaceId typeId with
      | none => true
      | some τ =>
          !τ.paramsBelow (typeArity unit handle) ||
            decide (ntyOf unit handle.namespaceId (instantiatedTypeId typeInstantiation typeId) =
              some (Skolems.resolve τ)))) &&
  (List.range (typeArity unit handle)).all fun index =>
    match paramNodeIn? unit handle.namespaceId index with
    | none => true
    | some node =>
        decide (ntyOf unit handle.namespaceId (instantiatedTypeId typeInstantiation node) =
          some (Skolems.resolve (.param index)))

theorem Coherent.ofCheck {unit : ValidatedUnit} [Skolems unit] {handle : FunctionHandle}
    {typeInstantiation : Array (TypeId × TypeId)}
    (check : coherentCheck unit handle typeInstantiation = true) :
    Coherent unit handle typeInstantiation := by
  unfold coherentCheck at check
  simp only [Bool.and_eq_true, List.all_eq_true] at check
  obtain ⟨required, params⟩ := check
  constructor
  · intro typeId member resource named typeBelow argumentsBelow
    have checked := (required typeId (Array.mem_toList_iff.mpr member)).1
    rw [named] at checked
    simpa only [typeBelow, argumentsBelow, Bool.and_self, Bool.not_true, Bool.false_or,
      decide_eq_true_eq] using checked
  · intro typeId member τ named below
    have checked := (required typeId (Array.mem_toList_iff.mpr member)).2
    rw [named] at checked
    simpa only [below, Bool.not_true, Bool.false_or, decide_eq_true_eq] using checked
  · intro index bounded node found
    have checked := params index (List.mem_range.mpr bounded)
    rw [found] at checked
    simpa only [decide_eq_true_eq] using checked

end Frames

/-- A frame resolves an instance of a declaration to an instance of it. -/
@[simp] theorem NTy.handle?_resolve_struct (Θ : Skolems unit) (source : StructHandle)
    (arguments fields : NRow) :
    (Θ.resolve (.struct source arguments fields)).handle? = some source := by
  rw [Θ.resolve_eq]; rfl

@[simp] theorem NTy.handle?_resolve_enum (Θ : Skolems unit) (source : StructHandle)
    (arguments : NRow) (names : List String) (rows : NRows) (distinct : names.Nodup) :
    (Θ.resolve (.enum source arguments names rows distinct)).handle? = some source := by
  rw [Θ.resolve_eq]; rfl

/-- A stored value's encoding is the encoding of the frame's value it
carries. -/
theorem Skolems.encode_runtime (Θ : Skolems unit) (τ : NTy)
    (value : @NTy.carrier (Carriers.runtime unit) (Θ.resolve τ)) :
    @NTy.encode (Carriers.runtime unit) (Θ.resolve τ) value =
      @NTy.encode Θ.toCarriers τ (Θ.ofRuntime τ value) := by
  rw [← Θ.encode_toRuntime, Θ.toRuntime_ofRuntime]

/-- The identity leaves a row as it is. -/
theorem NRow.map_id : (row : NRow) → row.map (fun τ => τ) = row
  | .nil => rfl
  | .cons τ rest => congrArg (NRow.cons τ) (NRow.map_id rest)

/-- At the runtime frame, the empty instantiation is coherent: every type
identifier denotes its own type, and every parameter stays. -/
theorem coherent_runtime (unit : ValidatedUnit) (handle : FunctionHandle) :
    @Coherent unit (Skolems.runtime unit) handle #[] := by
  refine @Coherent.mk unit (Skolems.runtime unit) handle #[]
    (fun typeId _ resource named _ _ => ?_) (fun typeId _ τ named _ => ?_)
    (fun index _ node found => ?_)
  · rw [instantiatedTypeId_empty, named]
    show some resource = some ⟨resource.type, resource.arguments.map fun τ => τ⟩
    rw [NRow.map_id]
  · rw [instantiatedTypeId_empty, named]
    rfl
  · rw [instantiatedTypeId_empty, ntyOf_paramNode found]
    rfl

end LeanerIR.Proofs.Denote
