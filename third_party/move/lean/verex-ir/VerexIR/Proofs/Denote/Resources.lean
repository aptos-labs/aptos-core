-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.TableMemory
import VerexIR.Semantics.ValueTyping

/-!
# Resource types of a unit

The native types the unit's type tables denote, and the resource type each
runtime storage key names: how typed global memory (`Memory`) relates to the
runtime's (`doc/designs/static-memory.md`).  A runtime key is the declaring
namespace with a type identifier of its table; the memory slot it names is
the resource type that identifier denotes.
-/

namespace VerexIR.Proofs.Denote

open VerexIR.Validation VerexIR.SemanticOperations

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
      some ⟨type, NRow.ofList arguments, .value⟩
  | _ => none

/-- A resource's native type and type arguments come from the unit's tables,
so an instance that covers those types supports them
(`NominalCarriers.covers`). -/
theorem resourceOf_supported [NominalCarriers unit] {namespaceId : NamespaceId} {typeId : TypeId}
    {resource : ResourceType} (named : resourceOf unit namespaceId typeId = some resource) :
    NTy.supported (NominalCarriers.supports unit) resource.type = true ∧
      NRow.supported (NominalCarriers.supports unit) resource.arguments = true := by
  simp only [resourceOf, Option.bind_eq_bind, Option.bind_eq_some_iff] at named
  obtain ⟨ty, -, named⟩ := named
  split at named
  · simp only [Option.bind_eq_bind, Option.bind_eq_some_iff, guard, Option.some.injEq] at named
    obtain ⟨ns, -, handle, -, declaration, -, guarded, -, type, typeRead, args, argsRead,
      same⟩ := named
    cases same
    refine ⟨NTy.supported_ntyOf unit _ _ type typeRead, ?_⟩
    simp only [NRow.supported_ofList]
    refine mapM_supported (NominalCarriers.supports unit) _ ?_ _ args argsRead
    intro argument σ got
    split at got
    · exact NTy.supported_ntyOf unit _ _ σ got
    · exact absurd got (by simp)
  · exact absurd named (by simp)

/-- Ordinary global resources always use the bounded native value carrier. -/
theorem resourceOf_kind {namespaceId : NamespaceId} {typeId : TypeId}
    {resource : ResourceType} (named : resourceOf unit namespaceId typeId = some resource) :
    resource.kind = .value := by
  unfold resourceOf at named
  simp only [bind, Option.bind] at named
  repeat (first | simp only at named | split at named | cases named) <;> try rfl

/-- Only a closed resource type holding no reference names a runtime storage
key. Open generic entries remain templates for frame instantiation
(`resourceOf`), but do not encode independent runtime slots before that
instantiation. -/
def runtimeResourceOf (unit : ValidatedUnit) (namespaceId : NamespaceId) (typeId : TypeId) :
    Option ResourceType := do
  let resource ← resourceOf unit namespaceId typeId
  guard (resource.type.paramFree && resource.type.refFree)
  some resource

theorem runtimeResourceOf_sound {namespaceId : NamespaceId} {typeId : TypeId}
    {resource : ResourceType} (named : runtimeResourceOf unit namespaceId typeId = some resource) :
    resourceOf unit namespaceId typeId = some resource ∧ resource.type.paramFree = true ∧
      resource.type.refFree = true := by
  unfold runtimeResourceOf at named
  obtain ⟨found, source, named⟩ := Option.bind_eq_some_iff.mp named
  by_cases closed : (found.type.paramFree && found.type.refFree) = true
  · simp [guard, closed] at named
    cases named
    exact ⟨source, Bool.and_eq_true_iff.mp closed⟩
  · simp [guard, closed, failure] at named

theorem runtimeResourceOf_kind {namespaceId : NamespaceId} {typeId : TypeId}
    {resource : ResourceType} (named : runtimeResourceOf unit namespaceId typeId = some resource) :
    resource.kind = .value :=
  resourceOf_kind (runtimeResourceOf_sound named).1

theorem runtimeResourceOf_refFree {namespaceId : NamespaceId} {typeId : TypeId}
    {resource : ResourceType} (named : runtimeResourceOf unit namespaceId typeId = some resource) :
    resource.type.refFree = true :=
  (runtimeResourceOf_sound named).2.2

variable [FunctionCarriers unit]

/-- Runtime global memory holding exactly what a typed memory holds: under
every key, the encoding of the value at the resource type its type
identifier denotes, a table's entries included, and nothing under a key
that denotes none.  Its entries are in key order, so the memory determines
it.  Each stored slot carries its folded validity predicate (`NTy.Holds`), so
what its values encode satisfies their data invariants at every depth
(`doc/designs/data-invariants.md`). -/
def Encodes (unit : ValidatedUnit) [FunctionCarriers unit]
    (memory : Memory unit) (globals : GlobalMap) : Prop :=
  globals.Sorted ∧ ∀ namespaceId typeId key, globals.lookup ⟨namespaceId, typeId, key⟩ =
    match runtimeResourceOf unit namespaceId typeId with
    | some resource => (memory resource key).map (resource.encode (unit := unit))
    | none => none

/-- A memory's runtime encoding is unique. -/
theorem Encodes.unique {unit : ValidatedUnit} [FunctionCarriers unit]
    {memory : Memory unit} {left right : GlobalMap}
    (encodes : Encodes unit memory left) (encodes' : Encodes unit memory right) : left = right :=
  GlobalMap.ext encodes.1 encodes'.1 fun ⟨namespaceId, typeId, key⟩ => by
    rw [encodes.2, encodes'.2]

/-- The observable runtime stores encode one typed memory: the globals hold
its values, and the handles allocated to tables are its allocation history,
observable after a table's destruction. Loan registries remain separate.
This is the ledger entry for data invariants: stored slots carry `NTy.Holds`,
and a storage whose value breaks one encodes no memory
(`doc/designs/data-invariants.md`). -/
structure StorageEncodes (unit : ValidatedUnit) [FunctionCarriers unit] (memory : Memory unit)
    (state : RuntimeState) : Prop where
  globals : Encodes unit memory state.globals
  allocated : ∀ key, memory TableMemory.allocationResource key =
    if key = .unit then some (TableMemory.encodeAllocated state.tableHandles) else none

/-- A memory determines both observable stores, independently of loan IDs. -/
theorem StorageEncodes.unique {memory : Memory unit} {left right : RuntimeState}
    (leftEncodes : StorageEncodes unit memory left)
    (rightEncodes : StorageEncodes unit memory right) :
    left.globals = right.globals ∧ left.tableHandles = right.tableHandles :=
  ⟨leftEncodes.globals.unique rightEncodes.globals,
   TableMemory.encodeAllocated_injective _ _ (Option.some.inj (by
     simpa using (leftEncodes.allocated .unit).symm.trans (rightEncodes.allocated .unit)))⟩

/-- A resource type neither runtime store nor the allocation history names. -/
def Unnamed (unit : ValidatedUnit) (resource : ResourceType) : Prop :=
  (∀ namespaceId typeId, runtimeResourceOf unit namespaceId typeId ≠ some resource) ∧
  resource ≠ TableMemory.allocationResource

/-- Two memories that agree on every resource type no runtime key names. -/
def AgreeUnnamed (unit : ValidatedUnit) [FunctionCarriers unit] (left right : Memory unit) : Prop :=
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
resource type, as the Move Prover assumes of the memory a function accesses
(`doc/designs/static-memory.md`, "Contracts"). The invariants read no memory,
so a write keeps those of the values it leaves alone. -/
-- retires with doc/plan/data-and-global-invariants.md, "The stored-invariant layer still exists for the declarations the carriers do not state"
def DataInvariants (invariant : (resource : ResourceType) → resource.carrier unit → Prop)
    (memory : Memory unit) : Prop :=
  ∀ resource key value, memory resource key = some value → invariant resource value

namespace DataInvariants

variable {invariant : (resource : ResourceType) → resource.carrier unit → Prop}
  {memory : Memory unit}

/-- A write keeps the invariants when the value written satisfies its own. -/
theorem set (holds : DataInvariants invariant memory) {resource : ResourceType}
    {key : StorageKey} {value : Option (resource.carrier unit)}
    (stored : ∀ written, value = some written → invariant resource written) :
    DataInvariants invariant (memory.set resource key value) := by
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
theorem set_some (holds : DataInvariants invariant memory) {resource : ResourceType}
    {key : StorageKey} {value : resource.carrier unit} (stored : invariant resource value) :
    DataInvariants invariant (memory.set resource key (some value)) :=
  holds.set fun _ written => by cases written; exact stored

/-- Removing a value keeps the invariants. -/
theorem set_none (holds : DataInvariants invariant memory) {resource : ResourceType}
    {key : StorageKey} : DataInvariants invariant (memory.set resource key none) :=
  holds.set fun _ written => nomatch written

/-- A value read from memory satisfies the invariant of its resource type. -/
theorem read (holds : DataInvariants invariant memory) {resource : ResourceType}
    {key : StorageKey} {value : resource.carrier unit} (read : memory resource key = some value) :
    invariant resource value :=
  holds resource key value read

end DataInvariants

mutual
/-- Whether a type mentions only type parameters below a bound. -/
def NTy.paramsBelow (bound : Nat) : NTy → Bool
  | .param index => index < bound
  | .tuple elements => NRow.paramsBelow bound elements
  | .struct _ arguments fields =>
      NRow.paramsBelow bound arguments && NRow.paramsBelow bound fields
  | .enum _ arguments _ rows _ =>
      NRow.paramsBelow bound arguments && NRows.paramsBelow bound rows
  | .vector element | .slice element => NTy.paramsBelow bound element
  | .array element _ => NTy.paramsBelow bound element
  | .table key value => NTy.paramsBelow bound key && NTy.paramsBelow bound value
  | .ref referent | .sliceRef referent => NTy.paramsBelow bound referent
  | .ghost slot => NTy.paramsBelow bound slot
  | .function parameters _ results =>
      NRow.paramsBelow bound parameters && NRow.paramsBelow bound results
  | .nominal _ arguments _ => NRow.paramsBelow bound arguments
  | .observed referent => NTy.paramsBelow bound referent
  | .unit | .bool | .int _ _ | .num | .address | .signer | .string | .bytes | .char => true

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

/-- A frame's resource type at the runtime family: its arguments are the frame's
reading of them, as the type arguments it induces are (`Skolems.typeArgs`). A
stored argument is no shared reference, so no mark reaches a key. -/
@[reducible] def Skolems.resource (type : NTy) (arguments : NRow) : ResourceType :=
  ⟨Θ.resolve type, arguments.map (NTy.argumentWith Θ.argument), .value⟩

/-- A stored value of a frame's resource, read in the frame's carrier family:
what a leaf reads of global memory. -/
def Skolems.plainAt (τ : NTy) (arguments : NRow) (key : StorageKey) (memory : Memory unit) :
    Option τ.carrier :=
  (memory (Skolems.resource τ arguments) key).map fun stored => Θ.ofRuntime τ stored.val

theorem Skolems.plainAt_def (τ : NTy) (arguments : NRow) (key : StorageKey)
    (memory : Memory unit) :
    Skolems.plainAt τ arguments key memory =
      (memory (Skolems.resource τ arguments) key).map (fun stored => Θ.ofRuntime τ stored.val) := rfl

/-- Pack an optional frame value with its validity witness into a runtime
storage slot. -/
def Skolems.toRuntimeSlot (τ : NTy) : (value : Option τ.carrier) →
    Option.elim value True (NTy.Holds τ) →
    Option (ResourceType.Stored unit (Θ.resolve τ))
  | none, _ => none
  | some v, hv => some ⟨Θ.toRuntime τ v, (Θ.holds_toRuntime τ v).mpr hv⟩

open Classical in
/-- A write into a frame's resource, total by a classical test of the
fact: the read-over-write law takes the fact as a side condition, and in the
denotation the written value's fact is supplied by the step's check
(`Term.globalPublish`, `Term.ghostStore`) or assumption (`Term.globalBorrow`),
while deleting writes (`Term.globalTake`) need none. -/
noncomputable def Skolems.writePlain (τ : NTy) (arguments : NRow) (key : StorageKey)
    (value : Option τ.carrier) (memory : Memory unit) : Memory unit :=
  if holds : Option.elim value True (NTy.Holds τ) then
    memory.set (Skolems.resource τ arguments) key (Skolems.toRuntimeSlot τ value holds)
  else memory

end Frames

section FrameTheorems
variable {Θ : Skolems unit}

theorem Skolems.writePlain_of_holds (τ : NTy) (arguments : NRow) (key : StorageKey)
    (value : Option (@NTy.carrier (Θ.toCarriers).toSorts τ)) (memory : Memory unit)
    (holds : Option.elim value True (@NTy.Holds Θ.toCarriers τ)) :
    Skolems.writePlain (Θ := Θ) τ arguments key value memory =
      memory.set (Skolems.resource (Θ := Θ) τ arguments) key
        (Skolems.toRuntimeSlot (Θ := Θ) τ value holds) :=
  dif_pos holds

theorem Skolems.writePlain_of_not_holds (τ : NTy) (arguments : NRow) (key : StorageKey)
    (value : Option (@NTy.carrier (Θ.toCarriers).toSorts τ)) (memory : Memory unit)
    (holds : ¬Option.elim value True (@NTy.Holds Θ.toCarriers τ)) :
    Skolems.writePlain (Θ := Θ) τ arguments key value memory = memory :=
  dif_neg holds

private theorem Skolems.plainAt_toRuntimeSlot (τ : NTy) :
    (slot : Option (@NTy.carrier (Θ.toCarriers).toSorts τ)) →
    (holds : Option.elim slot True (@NTy.Holds Θ.toCarriers τ)) →
    (Skolems.toRuntimeSlot (Θ := Θ) τ slot holds).map (fun stored => Θ.ofRuntime τ stored.val) =
      slot
  | none, _ => rfl
  | some value, _ => by
      show some (Θ.ofRuntime τ (Θ.toRuntime τ value)) = some value
      rw [Θ.ofRuntime_toRuntime]

theorem Skolems.plainAt_writePlain (τ : NTy) (arguments : NRow) (key otherKey : StorageKey)
    (memory : Memory unit) (value : Option (@NTy.carrier (Θ.toCarriers).toSorts τ))
    (holds : Option.elim value True (@NTy.Holds Θ.toCarriers τ)) :
    Skolems.plainAt (Θ := Θ) τ arguments otherKey
        (Skolems.writePlain (Θ := Θ) τ arguments key value memory) =
      if otherKey = key then value else Skolems.plainAt (Θ := Θ) τ arguments otherKey memory := by
  letI : Skolems unit := Θ
  rw [Skolems.writePlain_of_holds τ arguments key value memory holds, Skolems.plainAt_def,
    Memory.set_same, Skolems.plainAt_def]
  split
  · exact Skolems.plainAt_toRuntimeSlot τ value holds
  · rfl

theorem Skolems.plainAt_writePlain_some (τ : NTy) (arguments : NRow) (key otherKey : StorageKey)
    (memory : Memory unit) (v : @NTy.carrier (Θ.toCarriers).toSorts τ)
    (hv : @NTy.Holds Θ.toCarriers τ v) :
    Skolems.plainAt (Θ := Θ) τ arguments otherKey
        (Skolems.writePlain (Θ := Θ) τ arguments key (some v) memory) =
      if otherKey = key then some v else Skolems.plainAt (Θ := Θ) τ arguments otherKey memory :=
  Skolems.plainAt_writePlain τ arguments key otherKey memory (some v) hv

theorem Skolems.plainAt_writePlain_other (τ σ : NTy) (arguments others : NRow)
    (key otherKey : StorageKey) (memory : Memory unit)
    (value : Option (@NTy.carrier (Θ.toCarriers).toSorts τ))
    (distinct : Skolems.resource (Θ := Θ) σ others ≠ Skolems.resource (Θ := Θ) τ arguments) :
    Skolems.plainAt (Θ := Θ) σ others otherKey
        (Skolems.writePlain (Θ := Θ) τ arguments key value memory) =
      Skolems.plainAt (Θ := Θ) σ others otherKey memory := by
  letI : Skolems unit := Θ
  by_cases holds : Option.elim value True (NTy.Holds τ)
  · rw [Skolems.writePlain_of_holds τ arguments key value memory holds, Skolems.plainAt_def,
      Skolems.plainAt_def, Memory.set_other _ key otherKey _ distinct]
  · rw [Skolems.writePlain_of_not_holds τ arguments key value memory holds]

theorem Skolems.holds_plainAt (τ : NTy) (arguments : NRow) (key : StorageKey)
    (memory : Memory unit) {value : @NTy.carrier (Θ.toCarriers).toSorts τ}
    (read : Skolems.plainAt (Θ := Θ) τ arguments key memory = some value) :
    @NTy.Holds Θ.toCarriers τ value := by
  letI : Skolems unit := Θ
  rw [Skolems.plainAt_def] at read
  rcases stored : memory (Skolems.resource τ arguments) key with _ | source
  · rw [stored] at read
    exact absurd read (by simp)
  · rw [stored, Option.map_some] at read
    exact Option.some.inj read ▸ (Skolems.holds_ofRuntime τ source.val).mpr source.property

theorem Skolems.isSome_plainAt (τ : NTy) (arguments : NRow) (key : StorageKey)
    (memory : Memory unit) :
    (Skolems.plainAt (Θ := Θ) τ arguments key memory).isSome =
      (memory (Skolems.resource (Θ := Θ) τ arguments) key).isSome := by
  letI : Skolems unit := Θ
  rw [Skolems.plainAt_def, Option.isSome_map]

theorem Skolems.plainAt_writePlain_ne (τ : NTy) (arguments : NRow) (key otherKey : StorageKey)
    (value : Option (@NTy.carrier (Θ.toCarriers).toSorts τ)) (memory : Memory unit) (ne : otherKey ≠ key) :
    Skolems.plainAt (Θ := Θ) τ arguments otherKey
        (Skolems.writePlain (Θ := Θ) τ arguments key value memory) =
      Skolems.plainAt (Θ := Θ) τ arguments otherKey memory := by
  letI : Skolems unit := Θ
  by_cases holds : Option.elim value True (NTy.Holds τ)
  · rw [Skolems.plainAt_writePlain τ arguments key otherKey memory value holds, if_neg ne]
  · rw [Skolems.writePlain_of_not_holds τ arguments key value memory holds]

theorem Skolems.plainAt_writePlain_same (τ : NTy) (arguments : NRow) (key : StorageKey)
    (memory : Memory unit) (value : Option (@NTy.carrier (Θ.toCarriers).toSorts τ))
    (holds : Option.elim value True (@NTy.Holds Θ.toCarriers τ)) :
    Skolems.plainAt (Θ := Θ) τ arguments key
        (Skolems.writePlain (Θ := Θ) τ arguments key value memory) = value := by
  rw [Skolems.plainAt_writePlain τ arguments key key memory value holds, if_pos rfl]

theorem Skolems.plainAt_writePlain_some_same (τ : NTy) (arguments : NRow) (key : StorageKey)
    (memory : Memory unit) (v : @NTy.carrier (Θ.toCarriers).toSorts τ)
    (hv : @NTy.Holds Θ.toCarriers τ v) :
    Skolems.plainAt (Θ := Θ) τ arguments key
        (Skolems.writePlain (Θ := Θ) τ arguments key (some v) memory) = some v :=
  Skolems.plainAt_writePlain_same τ arguments key memory (some v) hv

theorem Skolems.plainAt_writePlain_none (τ : NTy) (arguments : NRow) (key otherKey : StorageKey)
    (memory : Memory unit) :
    Skolems.plainAt (Θ := Θ) τ arguments otherKey
        (Skolems.writePlain (Θ := Θ) τ arguments key none memory) =
      if otherKey = key then none else Skolems.plainAt (Θ := Θ) τ arguments otherKey memory :=
  Skolems.plainAt_writePlain τ arguments key otherKey memory none trivial

theorem Skolems.plainAt_writePlain_none_same (τ : NTy) (arguments : NRow) (key : StorageKey)
    (memory : Memory unit) :
    Skolems.plainAt (Θ := Θ) τ arguments key
        (Skolems.writePlain (Θ := Θ) τ arguments key none memory) = none := by
  rw [Skolems.plainAt_writePlain_none, if_pos rfl]

theorem Skolems.plainAt_eq_of_eq (τ : NTy) (arguments : NRow) (key : StorageKey)
    (m1 m2 : Memory unit)
    (same : m1 (Skolems.resource (Θ := Θ) τ arguments) key =
      m2 (Skolems.resource (Θ := Θ) τ arguments) key) :
    Skolems.plainAt (Θ := Θ) τ arguments key m1 =
      Skolems.plainAt (Θ := Θ) τ arguments key m2 := by
  letI : Skolems unit := Θ
  rw [Skolems.plainAt_def, Skolems.plainAt_def, same]

theorem Skolems.writePlain_writePlain_same (τ : NTy) (arguments : NRow) (key : StorageKey)
    (v1 v2 : Option (@NTy.carrier (Θ.toCarriers).toSorts τ)) (memory : Memory unit)
    (holds2 : Option.elim v2 True (@NTy.Holds Θ.toCarriers τ)) :
    Skolems.writePlain (Θ := Θ) τ arguments key v2
        (Skolems.writePlain (Θ := Θ) τ arguments key v1 memory) =
      Skolems.writePlain (Θ := Θ) τ arguments key v2 memory := by
  letI : Skolems unit := Θ
  by_cases holds1 : Option.elim v1 True (NTy.Holds τ)
  · rw [Skolems.writePlain_of_holds τ arguments key v1 memory holds1,
      Skolems.writePlain_of_holds τ arguments key v2 _ holds2,
      Skolems.writePlain_of_holds τ arguments key v2 memory holds2]
    funext r k
    by_cases hr : r = Skolems.resource τ arguments
    · subst hr
      simp only [Memory.set_same]
      by_cases hk : k = key <;> simp [hk]
    · simp only [Memory.set_other _ _ _ _ hr]
  · rw [Skolems.writePlain_of_not_holds τ arguments key v1 memory holds1]

theorem Skolems.writePlain_writePlain_none_same (τ : NTy) (arguments : NRow) (key : StorageKey)
    (v1 : Option (@NTy.carrier (Θ.toCarriers).toSorts τ)) (memory : Memory unit) :
    Skolems.writePlain (Θ := Θ) τ arguments key none
        (Skolems.writePlain (Θ := Θ) τ arguments key v1 memory) =
      Skolems.writePlain (Θ := Θ) τ arguments key none memory :=
  Skolems.writePlain_writePlain_same τ arguments key v1 none memory trivial

theorem Skolems.writePlain_other_key (τ : NTy) (arguments : NRow) (key otherKey : StorageKey)
    (value : Option (@NTy.carrier (Θ.toCarriers).toSorts τ)) (memory : Memory unit) (ne : otherKey ≠ key) :
    Skolems.writePlain (Θ := Θ) τ arguments key value memory
        (Skolems.resource (Θ := Θ) τ arguments) otherKey =
      memory (Skolems.resource (Θ := Θ) τ arguments) otherKey := by
  letI : Skolems unit := Θ
  by_cases holds : Option.elim value True (NTy.Holds τ)
  · rw [Skolems.writePlain_of_holds τ arguments key value memory holds, Memory.set_same, if_neg ne]
  · rw [Skolems.writePlain_of_not_holds τ arguments key value memory holds]

theorem Skolems.writePlain_other_key_of_eq (τ : NTy) (arguments : NRow) {r : ResourceType}
    (key otherKey : StorageKey) (value : Option (@NTy.carrier (Θ.toCarriers).toSorts τ))
    (memory : Memory unit) (same : r = Skolems.resource (Θ := Θ) τ arguments)
    (ne : otherKey ≠ key) :
    Skolems.writePlain (Θ := Θ) τ arguments key value memory r otherKey = memory r otherKey := by
  subst same
  exact Skolems.writePlain_other_key τ arguments key otherKey value memory ne

theorem Skolems.writePlain_other_resource (τ : NTy) (arguments : NRow) (other : ResourceType)
    (key : StorageKey) (value : Option (@NTy.carrier (Θ.toCarriers).toSorts τ)) (memory : Memory unit)
    (ne : other ≠ Skolems.resource (Θ := Θ) τ arguments) :
    Skolems.writePlain (Θ := Θ) τ arguments key value memory other = memory other := by
  letI : Skolems unit := Θ
  funext otherKey
  by_cases holds : Option.elim value True (NTy.Holds τ)
  · rw [Skolems.writePlain_of_holds τ arguments key value memory holds,
      Memory.set_other _ _ _ _ ne]
  · rw [Skolems.writePlain_of_not_holds τ arguments key value memory holds]

theorem Skolems.writePlain_other_resource_apply (τ : NTy) (arguments : NRow) (other : ResourceType)
    (key otherKey : StorageKey) (value : Option (@NTy.carrier (Θ.toCarriers).toSorts τ))
    (memory : Memory unit) (ne : other ≠ Skolems.resource (Θ := Θ) τ arguments) :
    Skolems.writePlain (Θ := Θ) τ arguments key value memory other otherKey =
      memory other otherKey :=
  congrFun (Skolems.writePlain_other_resource τ arguments other key value memory ne) otherKey

theorem Skolems.writePlain_ne_key (τ : NTy) (arguments : NRow) (other : ResourceType)
    (key otherKey : StorageKey) (value : Option (@NTy.carrier (Θ.toCarriers).toSorts τ))
    (memory : Memory unit) (ne : otherKey ≠ key) :
    Skolems.writePlain (Θ := Θ) τ arguments key value memory other otherKey =
      memory other otherKey := by
  by_cases hr : other = Skolems.resource (Θ := Θ) τ arguments
  · exact Skolems.writePlain_other_key_of_eq τ arguments key otherKey value memory hr ne
  · exact Skolems.writePlain_other_resource_apply τ arguments other key otherKey value memory hr

theorem Skolems.plainAt_writePlain_ne_key (τ σ : NTy) (arguments others : NRow)
    (key otherKey : StorageKey) (memory : Memory unit)
    (value : Option (@NTy.carrier (Θ.toCarriers).toSorts τ)) (ne : otherKey ≠ key) :
    Skolems.plainAt (Θ := Θ) σ others otherKey
        (Skolems.writePlain (Θ := Θ) τ arguments key value memory) =
      Skolems.plainAt (Θ := Θ) σ others otherKey memory := by
  letI : Skolems unit := Θ
  rw [Skolems.plainAt_def, Skolems.plainAt_def,
    Skolems.writePlain_ne_key τ arguments (Skolems.resource σ others) key otherKey value memory ne]

@[vir_denote_norm] theorem Skolems.resource_ne_allocationResource (τ : NTy) (arguments : NRow) :
    Skolems.resource (Θ := Θ) τ arguments ≠ TableMemory.allocationResource :=
  TableMemory.value_ne_allocationResource _ _

@[vir_denote_norm] theorem Skolems.allocationResource_ne_resource (τ : NTy) (arguments : NRow) :
    TableMemory.allocationResource ≠ Skolems.resource (Θ := Θ) τ arguments :=
  TableMemory.allocationResource_ne_value _ _

@[simp, vir_denote_norm] theorem Skolems.writePlain_allocationResource (τ : NTy) (arguments : NRow)
    (key otherKey : StorageKey) (value : Option (@NTy.carrier (Θ.toCarriers).toSorts τ))
    (memory : Memory unit) :
    Skolems.writePlain (Θ := Θ) τ arguments key value memory
        TableMemory.allocationResource otherKey =
      memory TableMemory.allocationResource otherKey :=
  Skolems.writePlain_other_resource_apply τ arguments TableMemory.allocationResource key otherKey
    value memory (Skolems.allocationResource_ne_resource τ arguments)

@[simp, vir_denote_norm] theorem Skolems.writePlain_allocationResource_fun (τ : NTy)
    (arguments : NRow) (key : StorageKey) (value : Option (@NTy.carrier (Θ.toCarriers).toSorts τ))
    (memory : Memory unit) :
    Skolems.writePlain (Θ := Θ) τ arguments key value memory TableMemory.allocationResource =
      memory TableMemory.allocationResource :=
  Skolems.writePlain_other_resource τ arguments TableMemory.allocationResource key value memory
    (Skolems.allocationResource_ne_resource τ arguments)

theorem DataInvariants.writePlain {invariant : (r : ResourceType) → r.carrier unit → Prop}
    {memory : Memory unit} (holds : DataInvariants invariant memory)
    {τ : NTy} {arguments : NRow} {key : StorageKey} {value : Option (@NTy.carrier (Θ.toCarriers).toSorts τ)}
    (stored : ∀ (v : @NTy.carrier (Θ.toCarriers).toSorts τ) (hv : @NTy.Holds Θ.toCarriers τ v),
      value = some v →
      invariant (Skolems.resource (Θ := Θ) τ arguments)
        ⟨Θ.toRuntime τ v, (Θ.holds_toRuntime τ v).mpr hv⟩) :
    DataInvariants invariant (Skolems.writePlain (Θ := Θ) τ arguments key value memory) := by
  letI : Skolems unit := Θ
  by_cases h : Option.elim value True (NTy.Holds τ)
  · rw [Skolems.writePlain_of_holds τ arguments key value memory h]
    cases value with
    | none => exact holds.set_none
    | some v => exact holds.set_some (stored v h rfl)
  · rw [Skolems.writePlain_of_not_holds τ arguments key value memory h]
    exact holds

theorem DataInvariants.writePlain_some {invariant : (r : ResourceType) → r.carrier unit → Prop}
    {memory : Memory unit} (holds : DataInvariants invariant memory)
    {τ : NTy} {arguments : NRow} {key : StorageKey} {v : @NTy.carrier (Θ.toCarriers).toSorts τ}
    (stored : ∀ (hv : @NTy.Holds Θ.toCarriers τ v),
      invariant (Skolems.resource (Θ := Θ) τ arguments)
        ⟨Θ.toRuntime τ v, (Θ.holds_toRuntime τ v).mpr hv⟩) :
    DataInvariants invariant (Skolems.writePlain (Θ := Θ) τ arguments key (some v) memory) :=
  DataInvariants.writePlain holds fun _ hv eq => by
    cases eq
    exact stored hv

theorem DataInvariants.writePlain_none {invariant : (r : ResourceType) → r.carrier unit → Prop}
    {memory : Memory unit} (holds : DataInvariants invariant memory)
    {τ : NTy} {arguments : NRow} {key : StorageKey} :
    DataInvariants invariant (Skolems.writePlain (Θ := Θ) τ arguments key none memory) :=
  DataInvariants.writePlain holds fun _ _ eq => nomatch eq

theorem DataInvariants.read_plainAt {invariant : (r : ResourceType) → r.carrier unit → Prop}
    {memory : Memory unit} (holds : DataInvariants invariant memory)
    {τ : NTy} {arguments : NRow} {key : StorageKey} {plainVal : @NTy.carrier (Θ.toCarriers).toSorts τ}
    (read : Skolems.plainAt (Θ := Θ) τ arguments key memory = some plainVal) :
    invariant (Skolems.resource (Θ := Θ) τ arguments)
      ⟨Θ.toRuntime τ plainVal,
        (Θ.holds_toRuntime τ plainVal).mpr
          (Skolems.holds_plainAt τ arguments key memory read)⟩ := by
  letI : Skolems unit := Θ
  have hread := read
  rw [Skolems.plainAt_def] at hread
  rcases hmem : memory (Skolems.resource τ arguments) key with _ | rawVal
  · rw [hmem] at hread; cases hread
  · rw [hmem, Option.map_some, Option.some.injEq] at hread
    have hinv := holds.read hmem
    have hrt : (⟨Θ.toRuntime τ plainVal,
          (Θ.holds_toRuntime τ plainVal).mpr
            (Skolems.holds_plainAt τ arguments key memory read)⟩ :
        (Skolems.resource τ arguments).carrier unit) = rawVal :=
      Subtype.ext (by dsimp only; rw [← hread, Θ.toRuntime_ofRuntime])
    rwa [hrt]

end FrameTheorems

private theorem NTy.unmark_not_isObserved : (σ : NTy) → σ.unmark.isObserved = false
  | .observed r => NTy.unmark_not_isObserved r
  | .unit | .bool | .int _ _ | .num | .address | .signer | .string
  | .bytes | .char | .tuple _ | .struct _ _ _ | .enum _ _ _ _ _ | .slice _
  | .vector _ | .array _ _ | .table _ _ | .ghost _ | .ref _ | .sliceRef _
  | .param _ | .function _ _ _ | .nominal _ _ _ => rfl

private theorem NTy.substWith_not_isObserved (f : Nat → NTy) :
    (τ : NTy) → τ.isObserved = false → (τ.substWith f).isObserved = false
  | .observed _, h => nomatch h
  | .param index, _ => NTy.unmark_not_isObserved (f index)
  | .unit, _ | .bool, _ | .int _ _, _ | .num, _ | .address, _ | .signer, _ | .string, _
  | .bytes, _ | .char, _ | .tuple _, _ | .struct _ _ _, _ | .enum _ _ _ _ _, _ | .slice _, _
  | .vector _, _ | .array _ _, _ | .table _ _, _ | .ghost _, _ | .ref _, _ | .sliceRef _, _
  | .function _ _ _, _ | .nominal _ _ _, _ => rfl

private theorem NTy.unmark_eq_self_of_not_isObserved : (τ : NTy) → τ.isObserved = false →
    τ.unmark = τ
  | .observed _, h => nomatch h
  | .unit, _ | .bool, _ | .int _ _, _ | .num, _ | .address, _ | .signer, _ | .string, _
  | .bytes, _ | .char, _ | .tuple _, _ | .struct _ _ _, _ | .enum _ _ _ _ _, _ | .slice _, _
  | .vector _, _ | .array _ _, _ | .table _ _, _ | .ghost _, _ | .ref _, _ | .sliceRef _, _
  | .param _, _ | .function _ _ _, _ | .nominal _ _ _, _ => rfl

private theorem NTy.substWith_unmark_argumentWith (g : Nat → NTy) (σ : NTy) :
    (σ.unmark).substWith g = (σ.argumentWith g).unmark := by
  cases σ with
  | param _ => rfl
  | observed r => exact NTy.substWith_unmark g r
  | _ => exact (NTy.unmark_eq_self_of_not_isObserved _ (NTy.substWith_not_isObserved g _ rfl)).symm

mutual
private theorem NTy.substWith_substWith (f g : Nat → NTy) : (τ : NTy) →
    (τ.substWith f).substWith g = τ.substWith (fun i => (f i).argumentWith g)
  | .unit | .bool | .int _ _ | .num | .address | .signer | .string | .bytes | .char => rfl
  | .param index => NTy.substWith_unmark_argumentWith g (f index)
  | .tuple elements => congrArg NTy.tuple (NRow.substWith_substWith f g elements)
  | .struct s args fields => by
      simp only [NTy.substWith, NRow.substWith_substWith f g args,
        NRow.substWith_substWith f g fields]
  | .enum s args names rows d => by
      simp only [NTy.substWith, NRow.substWith_substWith f g args,
        NRows.substWith_substWith f g rows]
  | .slice e => congrArg NTy.slice (NTy.substWith_substWith f g e)
  | .vector e => congrArg NTy.vector (NTy.substWith_substWith f g e)
  | .array e n => congrArg (NTy.array · n) (NTy.substWith_substWith f g e)
  | .table k v => by
      simp only [NTy.substWith, NTy.substWith_substWith f g k, NTy.substWith_substWith f g v]
  | .ghost s => congrArg NTy.ghost (NTy.substWith_substWith f g s)
  | .sliceRef r => congrArg NTy.sliceRef (NTy.substWith_substWith f g r)
  | .ref r => congrArg NTy.ref (NTy.substWith_substWith f g r)
  | .observed r => congrArg NTy.observed (NTy.substWith_substWith f g r)
  | .function ps sh rs => by
      simp only [NTy.substWith, NRow.substWith_substWith f g ps, NRow.substWith_substWith f g rs]
  | .nominal s args p => by
      simp only [NTy.substWith, NRow.substWith_substWith f g args]

private theorem NRow.substWith_substWith (f g : Nat → NTy) : (row : NRow) →
    (row.substWith f).substWith g = row.substWith (fun i => (f i).argumentWith g)
  | .nil => rfl
  | .cons τ rest => by
      simp only [NRow.substWith, NTy.substWith_substWith f g τ, NRow.substWith_substWith f g rest]

private theorem NRows.substWith_substWith (f g : Nat → NTy) : (rows : NRows) →
    (rows.substWith f).substWith g = rows.substWith (fun i => (f i).argumentWith g)
  | .nil => rfl
  | .cons row rest => by
      simp only [NRows.substWith, NRow.substWith_substWith f g row,
        NRows.substWith_substWith f g rest]
end

theorem NTy.argumentWith_argumentWith (f g : Nat → NTy) (τ : NTy) :
    (τ.argumentWith f).argumentWith g = τ.argumentWith (fun i => (f i).argumentWith g) := by
  cases τ with
  | param _ => rfl
  | observed r => exact congrArg NTy.observed (NTy.substWith_substWith f g r)
  | unit | bool | int _ _ | num | address | signer | string | bytes | char
  | tuple _ | struct _ _ _ | enum _ _ _ _ _ | slice _ | vector _ | array _ _
  | table _ _ | ghost _ | ref _ | sliceRef _ | function _ _ _ | nominal _ _ _ =>
    exact NTy.substWith_substWith f g _

theorem NRow.map_argumentWith_argumentWith (f g : Nat → NTy) : (row : NRow) →
    (row.map (NTy.argumentWith f)).map (NTy.argumentWith g) =
      row.map (NTy.argumentWith (fun i => (f i).argumentWith g))
  | .nil => rfl
  | .cons τ rest => by
      simp only [NRow.map, NTy.argumentWith_argumentWith f g τ,
        NRow.map_argumentWith_argumentWith f g rest]

theorem Skolems.resource_instantiate (θ : TypeArgs) (outer : Skolems unit) (τ : NTy)
    (arguments : NRow) :
    @Skolems.resource unit _ (Skolems.instantiate θ outer) τ arguments =
      @Skolems.resource unit _ outer (τ.subst θ.1) (arguments.map (NTy.argumentWith θ.1.argument)) := by
  simp only [Skolems.resource, Skolems.resolve_instantiate]
  exact congrArg (ResourceType.mk _ · .value)
    (NRow.map_argumentWith_argumentWith θ.1.argument outer.argument arguments).symm

theorem Skolems.plainAt_instantiate (θ : TypeArgs) (outer : Skolems unit) (τ : NTy)
    (arguments : NRow) (key : StorageKey) (memory : Memory unit) :
    @Skolems.plainAt unit _ (Skolems.instantiate θ outer) τ arguments key memory =
      (@Skolems.plainAt unit _ outer (τ.subst θ.1)
        (arguments.map (NTy.argumentWith θ.1.argument)) key memory).map
        (@NTy.toSkolem (outer.toCarriers).toSorts θ τ) := by
  let rawAt (args : NRow) :
      Option (ResourceType.Stored unit (outer.resolve (τ.subst θ.1))) :=
    memory ⟨outer.resolve (τ.subst θ.1), args, .value⟩ key
  have hraw : rawAt (arguments.map (NTy.argumentWith (Skolems.instantiate θ outer).argument)) =
      rawAt ((arguments.map (NTy.argumentWith θ.1.argument)).map (NTy.argumentWith outer.argument)) :=
    congrArg rawAt (NRow.map_argumentWith_argumentWith θ.1.argument outer.argument arguments).symm
  show (rawAt (arguments.map (NTy.argumentWith (Skolems.instantiate θ outer).argument))).map
      (fun stored => (Skolems.instantiate θ outer).ofRuntime τ stored.val) =
    ((rawAt ((arguments.map (NTy.argumentWith θ.1.argument)).map (NTy.argumentWith outer.argument))).map
      (fun stored => outer.ofRuntime (τ.subst θ.1) stored.val)).map
      (@NTy.toSkolem (outer.toCarriers).toSorts θ τ)
  rw [hraw]
  rcases rawAt ((arguments.map (NTy.argumentWith θ.1.argument)).map (NTy.argumentWith outer.argument)) with _ | raw
  · rfl
  · rfl

theorem Skolems.writePlain_instantiate (θ : TypeArgs) (outer : Skolems unit) (τ : NTy)
    (arguments : NRow) (key : StorageKey)
    (value : Option (@NTy.carrier ((Skolems.instantiate θ outer).toCarriers).toSorts τ))
    (memory : Memory unit) :
    @Skolems.writePlain unit _ (Skolems.instantiate θ outer) τ arguments key value memory =
      @Skolems.writePlain unit _ outer (τ.subst θ.1)
        (arguments.map (NTy.argumentWith θ.1.argument)) key
        (value.map (@NTy.ofSkolem (outer.toCarriers).toSorts θ τ)) memory := by
  let setAt (args : NRow)
      (slot : Option (ResourceType.Stored unit (outer.resolve (τ.subst θ.1)))) :
      Memory unit :=
    memory.set ⟨outer.resolve (τ.subst θ.1), args, .value⟩ key slot
  have hargs : arguments.map (NTy.argumentWith (Skolems.instantiate θ outer).argument) =
      (arguments.map (NTy.argumentWith θ.1.argument)).map (NTy.argumentWith outer.argument) :=
    (NRow.map_argumentWith_argumentWith θ.1.argument outer.argument arguments).symm
  cases value with
  | none =>
      have h1 := @Skolems.writePlain_of_holds unit _ (Skolems.instantiate θ outer) τ arguments key
        none memory trivial
      have h2 := @Skolems.writePlain_of_holds unit _ outer (τ.subst θ.1)
        (arguments.map (NTy.argumentWith θ.1.argument)) key none memory trivial
      rw [Option.map_none, h1, h2]
      exact congrArg (setAt · none) hargs
  | some v =>
      by_cases hv : @NTy.Holds (Skolems.instantiate θ outer).toCarriers τ v
      · have hvOuter : @NTy.Holds outer.toCarriers (τ.subst θ.1)
            (@NTy.ofSkolem (outer.toCarriers).toSorts θ τ v) :=
          (@NTy.holds_ofSkolem_iff outer.toCarriers θ τ v).mpr hv
        have h1 := @Skolems.writePlain_of_holds unit _ (Skolems.instantiate θ outer) τ arguments key
          (some v) memory (hv : Option.elim (some v) True _)
        have h2 := @Skolems.writePlain_of_holds unit _ outer (τ.subst θ.1)
          (arguments.map (NTy.argumentWith θ.1.argument)) key
          (some (@NTy.ofSkolem (outer.toCarriers).toSorts θ τ v)) memory
          (hvOuter : Option.elim (some _) True _)
        rw [Option.map_some, h1, h2]
        have hslot : @Skolems.toRuntimeSlot unit _ (Skolems.instantiate θ outer) τ (some v) hv =
            @Skolems.toRuntimeSlot unit _ outer (τ.subst θ.1)
              (some (@NTy.ofSkolem (outer.toCarriers).toSorts θ τ v)) hvOuter :=
          congrArg some (Subtype.ext rfl)
        show setAt (arguments.map (NTy.argumentWith (Skolems.instantiate θ outer).argument))
            (@Skolems.toRuntimeSlot unit _ (Skolems.instantiate θ outer) τ (some v) hv) =
          setAt ((arguments.map (NTy.argumentWith θ.1.argument)).map (NTy.argumentWith outer.argument))
            (@Skolems.toRuntimeSlot unit _ outer (τ.subst θ.1)
              (some (@NTy.ofSkolem (outer.toCarriers).toSorts θ τ v)) hvOuter)
        rw [hargs, hslot]
      · have hvOuter : ¬@NTy.Holds outer.toCarriers (τ.subst θ.1)
            (@NTy.ofSkolem (outer.toCarriers).toSorts θ τ v) := fun h =>
          hv ((@NTy.holds_ofSkolem_iff outer.toCarriers θ τ v).mp h)
        have h1 := @Skolems.writePlain_of_not_holds unit _ (Skolems.instantiate θ outer) τ arguments
          key (some v) memory hv
        have h2 := @Skolems.writePlain_of_not_holds unit _ outer (τ.subst θ.1)
          (arguments.map (NTy.argumentWith θ.1.argument)) key
          (some (@NTy.ofSkolem (outer.toCarriers).toSorts θ τ v)) memory hvOuter
        rw [Option.map_some, h1, h2]

/-- The data invariant of the stored values of a declaration of native type
`σ` and arguments `arguments`, its type parameters: of a value of the
resource of an instance, `holds` at the instance's type arguments of the
value read at the frame they induce over the runtime frame, as a generic
contract spells its frame; nothing of a value of another resource. -/
-- retires with doc/plan/data-and-global-invariants.md, "The stored-invariant layer still exists for the declarations the carriers do not state"
def StoredCase (σ : NTy) (arguments : NRow)
    (holds : (θ : TypeArgs) →
      @NTy.carrier ((Skolems.instantiate θ (Skolems.runtime unit)).toCarriers).toSorts σ → Prop)
    (resource : ResourceType) (value : resource.carrier unit) : Prop :=
  ∀ (inhabitable : resource.arguments.inhabitable = true)
    (same : resource = @Skolems.resource unit _
      (Skolems.instantiate ⟨resource.arguments, inhabitable⟩ (Skolems.runtime unit)) σ arguments),
    holds ⟨resource.arguments, inhabitable⟩
      ((Skolems.instantiate ⟨resource.arguments, inhabitable⟩ (Skolems.runtime unit)).ofRuntime σ
        (cast (congrArg (ResourceType.carrier unit) same) value).val)

/-! ## A declaration's data invariant over an encoding

`Carriers.own` is a field of the family, so a unit's predicate is generated
at a *base* instance whose `own` is the default and over the decoded carrier
row, which is closed before the unit's real instance exists
(`doc/designs/data-invariants.md`, "Data invariants"). -/

section OwnAtArguments
variable (unit : ValidatedUnit) (functions : FunctionCarriers unit)

/-- The frame a row of type arguments induces over the runtime frame. -/
@[reducible] noncomputable def Skolems.atArguments (arguments : NRow)
    (inhabitable : arguments.inhabitable = true) : Skolems unit :=
  @Skolems.instantiate unit functions ⟨arguments, inhabitable⟩
    (@Skolems.runtime unit functions)

/-- A declaration's data invariant as a predicate of the encoding of a value
of it (`Carriers.own`): whatever value of the declaration's type at the frame
the arguments induce has that encoding satisfies the declaration's clauses.
Over the encoding, which a value keeps across frames
(`Skolems.encode_reframe`), and over the carrier row, so that the predicate is
closed before the unit's instance exists. -/
def OwnAtArguments
    (σ : NTy)
    (holds : (θ : TypeArgs) →
      @NTy.carrier ((@Skolems.instantiate unit functions θ
        (@Skolems.runtime unit functions)).toCarriers).toSorts σ → Prop)
    (arguments : NRow) (value : RuntimeValue) : Prop :=
  ∀ (inhabitable : arguments.inhabitable = true)
    (row : @NTy.carrier
      ((Skolems.atArguments unit functions arguments inhabitable).toCarriers).toSorts σ),
    (@NTy.codec
        ((Skolems.atArguments unit functions arguments inhabitable).toCarriers).toSorts σ).decode?
        value = some row →
      holds ⟨arguments, inhabitable⟩ row

section
variable
  (σ : NTy)
  (holds : (θ : TypeArgs) →
    @NTy.carrier ((@Skolems.instantiate unit functions θ
      (@Skolems.runtime unit functions)).toCarriers).toSorts σ → Prop)
  (arguments : NRow)

/-- A declaration's data invariant holds of an encoding where it holds of one
value with that encoding: the carrier's codec decodes an encoding to at most
one value, and the inhabitability premise is proof-irrelevant. -/
theorem OwnAtArguments.intro {value : RuntimeValue}
    (inhabitable : arguments.inhabitable = true)
    {row : @NTy.carrier
      ((Skolems.atArguments unit functions arguments inhabitable).toCarriers).toSorts σ}
    (decoded : (@NTy.codec
      ((Skolems.atArguments unit functions arguments inhabitable).toCarriers).toSorts σ).decode?
      value = some row)
    (held : holds ⟨arguments, inhabitable⟩ row) :
    OwnAtArguments unit functions σ holds arguments value := by
  intro inhabitable' row' decoded'
  cases Option.some.inj (decoded'.symm.trans decoded)
  exact held

/-- A declaration's data invariant of an encoded value is its clauses of that
value: the premises of `OwnAtArguments` are the decoding, which an encoding
determines (`Codec.decode_encode`). -/
-- Applied explicitly after choosing the value's frame: global simplification
-- otherwise searches expensive dependent conversions between families.
theorem OwnAtArguments.encode
    (inhabitable : arguments.inhabitable = true)
    (row : @NTy.carrier
      ((Skolems.atArguments unit functions arguments inhabitable).toCarriers).toSorts σ) :
    OwnAtArguments unit functions σ holds arguments
        (@NTy.encode
          ((Skolems.atArguments unit functions arguments inhabitable).toCarriers).toSorts σ row)
      ↔ holds ⟨arguments, inhabitable⟩ row := by
  have decoded := (@NTy.codec
    ((Skolems.atArguments unit functions arguments inhabitable).toCarriers).toSorts σ).decode_encode row
  constructor
  · intro own
    exact own inhabitable row decoded
  · intro held
    exact OwnAtArguments.intro unit functions σ holds arguments
      inhabitable decoded held

end

end OwnAtArguments

/-- A stored value satisfies its declaration's invariant where it does at
its resource's own type arguments: the premises' proofs are any. -/
-- retires with doc/plan/data-and-global-invariants.md, "The stored-invariant layer still exists for the declarations the carriers do not state"
theorem StoredCase.intro {σ : NTy} {arguments : NRow}
    {holds : (θ : TypeArgs) →
      @NTy.carrier ((Skolems.instantiate θ (Skolems.runtime unit)).toCarriers).toSorts σ → Prop}
    {resource : ResourceType} {value : resource.carrier unit}
    (inhabitable : resource.arguments.inhabitable = true)
    (same : resource = @Skolems.resource unit _
      (Skolems.instantiate ⟨resource.arguments, inhabitable⟩ (Skolems.runtime unit)) σ arguments)
    (held : holds ⟨resource.arguments, inhabitable⟩
      ((Skolems.instantiate ⟨resource.arguments, inhabitable⟩ (Skolems.runtime unit)).ofRuntime σ
        (cast (congrArg (ResourceType.carrier unit) same) value).val)) :
    StoredCase σ arguments holds resource value :=
  fun _ _ => held

/-! ## The frame of a resource's type arguments

A declaration's invariant is stated of a stored value at its resource's own
type arguments, at the frame they induce over the runtime frame
(`StoredCase`): one statement per row of type values. A frame reading or
writing the value moves its own view of it there (`Skolems.reframe`), which
keeps its encoding (`Skolems.encode_reframe`). -/

/-- Whether a row is the parameters of a declaration, from `start` on. -/
def NRow.isParamsFrom (start : Nat) : NRow → Bool
  | .nil => true
  | .cons τ rest => τ == .param start && rest.isParamsFrom (start + 1)

/-- An entry inside a row, with the entry replaced. Beyond the row the two
differ, as the default is not replaced. -/
theorem NRow.getD_map (g : NTy → NTy) :
    (row : NRow) → (index : Nat) → index < row.length → (default : NTy) →
      (row.map g).getD index default = g (row.getD index default)
  | .nil, _, below, _ => absurd below (Nat.not_lt_zero _)
  | .cons _ _, 0, _, _ => rfl
  | .cons _ rest, index + 1, below, default => by
      simp only [NRow.length] at below
      exact NRow.getD_map g rest index (by omega) default

/-- A declaration's parameters are read as themselves. -/
theorem NRow.getD_params : (params : NRow) → (start : Nat) →
    params.isParamsFrom start = true → (index : Nat) → index < params.length →
      (default : NTy) → params.getD index default = .param (start + index)
  | .nil, _, _, _, below, _ => absurd below (Nat.not_lt_zero _)
  | .cons τ _, start, own, 0, _, _ => by
      simp only [NRow.isParamsFrom, Bool.and_eq_true, beq_iff_eq] at own
      simp only [NRow.getD, own.1, Nat.add_zero]
  | .cons _ rest, start, own, index + 1, below, default => by
      simp only [NRow.isParamsFrom, Bool.and_eq_true] at own
      simp only [NRow.length] at below
      rw [show start + (index + 1) = start + 1 + index from by omega]
      exact NRow.getD_params rest (start + 1) own.2 index (by omega) default

/-- A row's entries with each replaced. -/
theorem NRow.getD_map_params (g : NTy → NTy) :
    (params : NRow) → (start : Nat) → params.isParamsFrom start = true → (index : Nat) →
      index < params.length → (default : NTy) →
      (params.map g).getD index default = g (.param (start + index)) :=
  fun params start own index below default =>
    (NRow.getD_map g params index below default).trans
      (congrArg g (NRow.getD_params params start own index below default))

/-- A declaration's parameters are below their number. -/
theorem NRow.paramsAll_params : (params : NRow) → (start bound : Nat) →
    params.isParamsFrom start = true → start + params.length ≤ bound →
    params.paramsAll (fun index => decide (index < bound)) = true
  | .nil, _, _, _, _ => rfl
  | .cons τ rest, start, bound, own, below => by
      simp only [NRow.isParamsFrom, Bool.and_eq_true, beq_iff_eq] at own
      simp only [NRow.length] at below
      simp only [NRow.paramsAll, own.1, NTy.paramsAll, Bool.and_eq_true, decide_eq_true_eq]
      exact ⟨by omega, NRow.paramsAll_params rest (start + 1) bound own.2 (by omega)⟩

mutual
/-- Substituting a row agreeing with a family on the parameters a type
mentions is resolving it by the family. -/
theorem NTy.subst_eq_substWith (θ : NRow) (f : Nat → NTy) (bound : Nat)
    (agree : ∀ index, index < bound → θ.getD index (.param index) = f index) :
    (τ : NTy) → τ.paramsAll (fun index => decide (index < bound)) = true →
      τ.subst θ = τ.substWith f
  | .param index, bounded => by
      simp only [NTy.paramsAll, decide_eq_true_eq] at bounded
      simp only [NTy.subst, NTy.substWith]
      exact congrArg NTy.unmark (agree index bounded)
  | .unit, _ | .bool, _ | .int _ _, _ | .num, _ | .address, _ | .signer, _ | .string, _
  | .bytes, _ => rfl
  | .char, _ => rfl
  | .tuple elements, bounded => by
      simp only [NTy.paramsAll] at bounded
      simp only [NTy.subst, NTy.substWith, NRow.subst_eq_substWith θ f bound agree elements bounded]
  | .struct _ arguments fields, bounded => by
      simp only [NTy.paramsAll, Bool.and_eq_true] at bounded
      simp only [NTy.subst, NTy.substWith, NRow.subst_eq_substWith θ f bound agree arguments bounded.1,
        NRow.subst_eq_substWith θ f bound agree fields bounded.2]
  | .enum _ arguments _ rows _, bounded => by
      simp only [NTy.paramsAll, Bool.and_eq_true] at bounded
      simp only [NTy.subst, NTy.substWith, NRow.subst_eq_substWith θ f bound agree arguments bounded.1,
        NRows.subst_eq_substWith θ f bound agree rows bounded.2]
  | .vector element, bounded | .slice element, bounded => by
      simp only [NTy.paramsAll] at bounded
      simp only [NTy.subst, NTy.substWith, NTy.subst_eq_substWith θ f bound agree element bounded]
  | .array element _, bounded => by
      simp only [NTy.paramsAll] at bounded
      simp only [NTy.subst, NTy.substWith, NTy.subst_eq_substWith θ f bound agree element bounded]
  | .table key value, bounded => by
      simp only [NTy.paramsAll, Bool.and_eq_true] at bounded
      simp only [NTy.subst, NTy.substWith, NTy.subst_eq_substWith θ f bound agree key bounded.1,
        NTy.subst_eq_substWith θ f bound agree value bounded.2]
  | .ghost slot, bounded => by
      simp only [NTy.paramsAll] at bounded
      simp only [NTy.subst, NTy.substWith, NTy.subst_eq_substWith θ f bound agree slot bounded]
  | .observed referent, bounded => by
      simp only [NTy.paramsAll] at bounded
      simp only [NTy.subst, NTy.substWith, NTy.subst_eq_substWith θ f bound agree referent bounded]
  | .ref referent, bounded | .sliceRef referent, bounded => by
      simp only [NTy.paramsAll] at bounded
      simp only [NTy.subst, NTy.substWith, NTy.subst_eq_substWith θ f bound agree referent bounded]
  | .function parameters shared results, bounded => by
      simp only [NTy.paramsAll, Bool.and_eq_true] at bounded
      simp only [NTy.subst, NTy.substWith,
        NRow.subst_eq_substWith θ f bound agree parameters bounded.1,
        NRow.subst_eq_substWith θ f bound agree results bounded.2, Sharing.at,
        Sharing.atWith_congr (left := θ.argument) (right := f) (shared := shared)
          (fun index mentioned => agree index (of_decide_eq_true mentioned))
          bounded.1 bounded.2]
  | .nominal _ arguments _, bounded => by
      simp only [NTy.paramsAll] at bounded
      simp only [NTy.subst, NTy.substWith,
        NRow.subst_eq_substWith θ f bound agree arguments bounded]

theorem NRow.subst_eq_substWith (θ : NRow) (f : Nat → NTy) (bound : Nat)
    (agree : ∀ index, index < bound → θ.getD index (.param index) = f index) :
    (row : NRow) → row.paramsAll (fun index => decide (index < bound)) = true →
      NRow.subst θ row = NRow.substWith f row
  | .nil, _ => rfl
  | .cons τ rest, bounded => by
      simp only [NRow.paramsAll, Bool.and_eq_true] at bounded
      simp only [NRow.subst, NRow.substWith, NTy.subst_eq_substWith θ f bound agree τ bounded.1,
        NRow.subst_eq_substWith θ f bound agree rest bounded.2]

theorem NRows.subst_eq_substWith (θ : NRow) (f : Nat → NTy) (bound : Nat)
    (agree : ∀ index, index < bound → θ.getD index (.param index) = f index) :
    (rows : NRows) → rows.paramsAll (fun index => decide (index < bound)) = true →
      NRows.subst θ rows = NRows.substWith f rows
  | .nil, _ => rfl
  | .cons fields rest, bounded => by
      simp only [NRows.paramsAll, Bool.and_eq_true] at bounded
      simp only [NRows.subst, NRows.substWith, NRow.subst_eq_substWith θ f bound agree fields bounded.1,
        NRows.subst_eq_substWith θ f bound agree rest bounded.2]
end

/-- A row of types resolved at a frame, as one substitution: resolution
replaces each type parameter by its resolution (`Skolems.resolve_eq`), so
mapping it over a row substitutes the row. -/
theorem NRow.map_resolve (Θ : Skolems unit) : (row : NRow) →
    row.map Θ.resolve = NRow.substWith (fun index => Θ.resolve (.param index)) row
  | .nil => rfl
  | .cons τ rest => by
      simp only [NRow.map, NRow.substWith, Θ.resolve_eq_type τ, NRow.map_resolve Θ rest]

/-- The data invariants a frame states of a declaration: the unit's, of the
same encoded value, at the declaration's arguments as the frame resolves
them (`Skolems.own_resolve`).  This is the normal form at a frame that is
neither the runtime family (`Carriers.own_runtime`) nor an induced one
(`Carriers.own_instantiate`) — every generic function's own frame — where
`Carriers.own` is otherwise opaque and a declaration's clauses never
appear. -/
@[vir_denote_norm] theorem Carriers.own_frame (Θ : Skolems unit) (source : StructHandle)
    (arguments : NRow) (value : RuntimeValue) :
    @Carriers.own Θ.toCarriers source arguments value =
      FunctionCarriers.own unit source (arguments.map Θ.resolve) value := by
  rw [Θ.own_resolve source arguments value, NRow.map_resolve Θ arguments]

/-- A declaration's own parameters substitute a type over them to itself. -/
theorem NTy.subst_params {params : NRow} (own : params.isParamsFrom 0 = true) {τ : NTy}
    (bounded : τ.paramsAll (fun index => decide (index < params.length)) = true) :
    τ.subst params = τ := by
  refine (NTy.subst_eq_substWith params (fun index => .param index) params.length
    (fun index below => ?_) τ bounded).trans (NTy.substWith_param τ)
  simpa only [Nat.zero_add] using NRow.getD_params params 0 own index below (.param index)

/-- A frame's resolutions of a declaration's parameters are inhabitable. -/
theorem NRow.inhabitable_map_resolve (Θ : Skolems unit) :
    (params : NRow) → (start : Nat) → params.isParamsFrom start = true →
      (params.map (NTy.argumentWith Θ.argument)).inhabitable = true
  | .nil, _, _ => rfl
  | .cons _ rest, start, own => by
      simp only [NRow.isParamsFrom, Bool.and_eq_true, beq_iff_eq] at own
      simp only [NRow.map, NRow.inhabitable, own.1,
        NTy.inhabitable_argumentWith Θ.argument_inhabitable (.param start) rfl,
        NRow.inhabitable_map_resolve Θ rest (start + 1) own.2, Bool.and_self]

/-- The type arguments a frame gives a declaration's parameters: its reading of
each, marked where the parameter stands for a shared reference. -/
@[reducible] def Skolems.typeArgs (Θ : Skolems unit) (params : NRow)
    (own : params.isParamsFrom 0 = true) : TypeArgs :=
  ⟨params.map (NTy.argumentWith Θ.argument), NRow.inhabitable_map_resolve Θ params 0 own⟩

/-- The frame those type arguments induce over the runtime frame. -/
@[reducible] noncomputable def Skolems.instance
    (Θ : Skolems unit) (params : NRow)
    (own : params.isParamsFrom 0 = true) : Skolems unit :=
  Skolems.instantiate (Θ.typeArgs params own) (Skolems.runtime unit)

/-- A type over a declaration's parameters resolves alike at a frame and at
the frame the row of its type arguments induces: substituting the row of the
frame's readings is resolving what the row itself substitutes. Stated of any
instance row, so an argument of a declaration may be any type over the
parameters the frame reads, not only one of them. -/
theorem Skolems.resolve_instance (Θ : Skolems unit) (params : NRow) {τ : NTy}
    (bounded : τ.paramsAll (fun index => decide (index < params.length)) = true) :
    τ.subst (params.map (NTy.argumentWith Θ.argument)) = Θ.resolve (τ.subst params) := by
  rw [Θ.resolve_eq (τ.subst params), NTy.substWith_subst]
  refine NTy.subst_eq_substWith _ _ params.length (fun index below => ?_) τ bounded
  exact NRow.getD_map (NTy.argumentWith Θ.argument) params index below (.param index)

/-- The instance row of a declaration's own parameters substitutes a type over
them to itself, so the general resolution reads as it did. -/
theorem Skolems.resolve_instance_params (Θ : Skolems unit) {params : NRow}
    (own : params.isParamsFrom 0 = true) {τ : NTy}
    (bounded : τ.paramsAll (fun index => decide (index < params.length)) = true) :
    τ.subst (params.map (NTy.argumentWith Θ.argument)) = Θ.resolve τ := by
  rw [Θ.resolve_instance params bounded, NTy.subst_params own bounded]

/-- A frame's value of a type over a declaration's parameters, at the frame
of its type arguments: the same runtime value. -/
noncomputable def Skolems.reframe
    (Θ : Skolems unit) {params : NRow} (own : params.isParamsFrom 0 = true)
    {τ : NTy} (bounded : τ.paramsAll (fun index => decide (index < params.length)) = true)
    (value : @NTy.carrier (Θ.toCarriers).toSorts τ) : @NTy.carrier ((Θ.instance params own).toCarriers).toSorts τ :=
  (Θ.instance params own).ofRuntime τ
    (cast (congrArg (@NTy.carrier ((Carriers.runtime unit)).toSorts) (Θ.resolve_instance_params own bounded).symm)
      (Θ.toRuntime τ value))

/-- An encoding does not see a cast between the carriers of equal types. -/
theorem NTy.encode_castCarrier {Φ : Sorts} {ρ ρ' : NTy} (same : ρ = ρ')
    (carriers : @NTy.carrier Φ ρ = @NTy.carrier Φ ρ') (value : @NTy.carrier Φ ρ) :
    @NTy.encode Φ ρ' (cast carriers value) = @NTy.encode Φ ρ value := by
  subst same
  rfl

/-- A row's encoding does not see a cast between the carriers of equal rows. -/
theorem HList.encode_castCarrier {Φ : Sorts} {row row' : NRow} (same : row = row')
    (carriers : @HList Φ row = @HList Φ row') (values : @HList Φ row) :
    @HList.encode Φ row' (cast carriers values) = @HList.encode Φ row values := by
  subst same
  rfl

/-- Moving a value to the frame of its type arguments keeps its encoding. -/
theorem Skolems.encode_reframe
    (Θ : Skolems unit) {params : NRow} (own : params.isParamsFrom 0 = true) {τ : NTy}
    (bounded : τ.paramsAll (fun index => decide (index < params.length)) = true)
    (value : @NTy.carrier (Θ.toCarriers).toSorts τ) :
    @NTy.encode ((Θ.instance params own).toCarriers).toSorts τ (Θ.reframe own bounded value) =
      @NTy.encode (Θ.toCarriers).toSorts τ value := by
  unfold Skolems.reframe
  rw [← (Θ.instance params own).encode_toRuntime, (Θ.instance params own).toRuntime_ofRuntime]
  exact (NTy.encode_castCarrier (Θ.resolve_instance_params own bounded).symm _ _).trans
    (Θ.encode_toRuntime τ value)

attribute [vir_denote_norm] Skolems.encode_reframe

/-- A value's invariants do not see a cast between the carriers of equal types. -/
theorem NTy.holds_castCarrier {Φ : Carriers} {ρ ρ' : NTy} (same : ρ = ρ')
    (carriers : @NTy.carrier (Φ).toSorts ρ = @NTy.carrier (Φ).toSorts ρ') (value : @NTy.carrier (Φ).toSorts ρ) :
    @NTy.Holds Φ ρ' (cast carriers value) ↔ @NTy.Holds Φ ρ value := by
  subst same; rfl

attribute [simp, vir_transport] Skolems.holds_toRuntime Skolems.holds_ofRuntime

/-- Moving a value to the frame of its type arguments preserves its invariants. -/
@[simp, vir_transport] theorem Skolems.holds_reframe_iff
    (Θ : Skolems unit) {params : NRow}
    (own : params.isParamsFrom 0 = true) {τ : NTy}
    (bounded : τ.paramsAll (fun index => decide (index < params.length)) = true)
    (value : @NTy.carrier (Θ.toCarriers).toSorts τ) :
    @NTy.Holds (Θ.instance params own).toCarriers τ (Θ.reframe own bounded value) ↔
      @NTy.Holds Θ.toCarriers τ value :=
  (@Skolems.holds_ofRuntime unit _ (Θ.instance params own) τ _).trans
    ((NTy.holds_castCarrier (Θ.resolve_instance_params own bounded).symm _ _).trans
      (Θ.holds_toRuntime τ value))

/-- Moving values to the frame of their type arguments is injective. -/
@[vir_denote_norm] theorem Skolems.reframe_inj
    (Θ : Skolems unit) {params : NRow} (own : params.isParamsFrom 0 = true) {τ : NTy}
    (bounded : τ.paramsAll (fun index => decide (index < params.length)) = true)
    (left right : @NTy.carrier (Θ.toCarriers).toSorts τ) :
    Θ.reframe own bounded left = Θ.reframe own bounded right ↔ left = right := by
  constructor
  · intro same
    apply @NTy.encode_injective (Θ.toCarriers).toSorts τ
    rw [← Θ.encode_reframe own bounded left, ← Θ.encode_reframe own bounded right, same]
  · intro same
    rw [same]

/-- Equality without handle identity does not see the frame (`NTy.equal_codec`). -/
@[vir_denote_norm] theorem Skolems.equal_reframe
    (Θ : Skolems unit) {params : NRow} (own : params.isParamsFrom 0 = true)
    {τ : NTy} (bounded : τ.paramsAll (fun index => decide (index < params.length)) = true)
    (left right : @NTy.carrier (Θ.toCarriers).toSorts τ) :
    @NTy.equal ((Θ.instance params own).toCarriers).toSorts false τ (Θ.reframe own bounded left)
        (Θ.reframe own bounded right) = @NTy.equal (Θ.toCarriers).toSorts false τ left right := by
  rw [@NTy.equal_codec ((Θ.instance params own).toCarriers).toSorts, @NTy.equal_codec (Θ.toCarriers).toSorts]
  simp only [NTy.codec_encode, Skolems.encode_reframe]
  rfl

/-- Nor does the order (`NTy.order_codec`). -/
@[vir_denote_norm] theorem Skolems.order_reframe
    (Θ : Skolems unit) {params : NRow} (own : params.isParamsFrom 0 = true)
    {τ : NTy} (bounded : τ.paramsAll (fun index => decide (index < params.length)) = true)
    (rank : ValueRanks) (left right : @NTy.carrier (Θ.toCarriers).toSorts τ) :
    @NTy.order ((Θ.instance params own).toCarriers).toSorts rank τ (Θ.reframe own bounded left)
        (Θ.reframe own bounded right) = @NTy.order (Θ.toCarriers).toSorts rank τ left right := by
  rw [← @NTy.order_codec ((Θ.instance params own).toCarriers).toSorts, ← @NTy.order_codec (Θ.toCarriers).toSorts]
  simp only [NTy.codec_encode, Skolems.encode_reframe]

section Reframe
variable (Θ : Skolems unit) {params : NRow} (own : params.isParamsFrom 0 = true)

/-- A value whose carrier and encoding do not read the frame stays itself. -/
theorem Skolems.reframe_of_encode {τ : NTy} (bounded) (value : @NTy.carrier (Θ.toCarriers).toSorts τ)
    (carriers : @NTy.carrier (Θ.toCarriers).toSorts τ = @NTy.carrier ((Θ.instance params own).toCarriers).toSorts τ)
    (encoded : ∀ value, @NTy.encode ((Θ.instance params own).toCarriers).toSorts τ (cast carriers value) =
      @NTy.encode (Θ.toCarriers).toSorts τ value) :
    Θ.reframe own bounded value = cast carriers value := by
  apply @NTy.encode_injective ((Θ.instance params own).toCarriers).toSorts τ
  rw [Θ.encode_reframe, encoded]

@[vir_denote_norm] theorem Skolems.reframe_unit (bounded) (value : Unit) :
    Θ.reframe own (τ := .unit) bounded value = value :=
  Θ.reframe_of_encode own bounded value rfl fun _ => rfl
@[vir_denote_norm] theorem Skolems.reframe_bool (bounded) (value : Bool) :
    Θ.reframe own (τ := .bool) bounded value = value :=
  Θ.reframe_of_encode own bounded value rfl fun _ => rfl
@[vir_denote_norm] theorem Skolems.reframe_int {width : Nat} {signed : Bool} (bounded)
    (value : SpecInt (.bits width) signed) :
    Θ.reframe own (τ := .int width signed) bounded value = value :=
  Θ.reframe_of_encode own bounded value rfl fun _ => rfl
@[vir_denote_norm] theorem Skolems.reframe_num (bounded) (value : SpecInt .unbounded true) :
    Θ.reframe own (τ := .num) bounded value = value :=
  Θ.reframe_of_encode own bounded value rfl fun _ => rfl
@[vir_denote_norm] theorem Skolems.reframe_address (bounded) (value : String) :
    Θ.reframe own (τ := .address) bounded value = value :=
  Θ.reframe_of_encode own bounded value rfl fun _ => rfl
@[vir_denote_norm] theorem Skolems.reframe_signer (bounded) (value : String) :
    Θ.reframe own (τ := .signer) bounded value = value :=
  Θ.reframe_of_encode own bounded value rfl fun _ => rfl
@[vir_denote_norm] theorem Skolems.reframe_string (bounded) (value : String) :
    Θ.reframe own (τ := .string) bounded value = value :=
  Θ.reframe_of_encode own bounded value rfl fun _ => rfl
@[vir_denote_norm] theorem Skolems.reframe_bytes (bounded) (value : Array UInt8) :
    Θ.reframe own (τ := .bytes) bounded value = value :=
  Θ.reframe_of_encode own bounded value rfl fun _ => rfl
@[vir_denote_norm] theorem Skolems.reframe_char (bounded) (value : SpecChar) :
    Θ.reframe own (τ := .char) bounded value = value :=
  Θ.reframe_of_encode own bounded value rfl fun _ => rfl

private theorem paramsAll_head {f : Nat → Bool} {τ : NTy} {rest : NRow}
    (bounded : NRow.paramsAll f (.cons τ rest) = true) : τ.paramsAll f = true := by
  simp only [NRow.paramsAll, Bool.and_eq_true] at bounded
  exact bounded.1

private theorem paramsAll_tail {f : Nat → Bool} {τ : NTy} {rest : NRow}
    (bounded : NRow.paramsAll f (.cons τ rest) = true) : rest.paramsAll f = true := by
  simp only [NRow.paramsAll, Bool.and_eq_true] at bounded
  exact bounded.2

private theorem paramsAll_fields {f : Nat → Bool} {source : StructHandle} {arguments fields : NRow}
    (bounded : (NTy.struct source arguments fields).paramsAll f = true) :
    fields.paramsAll f = true := by
  simp only [NTy.paramsAll, Bool.and_eq_true] at bounded
  exact bounded.2

private theorem paramsAll_struct {f : Nat → Bool} {source : StructHandle} {arguments fields : NRow}
    (arguments' : arguments.paramsAll f = true) (fields' : fields.paramsAll f = true) :
    (NTy.struct source arguments fields).paramsAll f = true := by
  simp only [NTy.paramsAll, Bool.and_eq_true]
  exact ⟨arguments', fields'⟩

/-- The encodings of a struct row's fields, from the row's. -/
private theorem encode_fields {Φ Ψ : Sorts} {source : StructHandle} {arguments : NRow}
    {τ : NTy} {rest : NRow} (left : @HList Φ (.cons τ rest))
    (right : @HList Ψ (.cons τ rest))
    (same : @NTy.encode Φ (.struct source arguments (.cons τ rest)) left =
      @NTy.encode Ψ (.struct source arguments (.cons τ rest)) right) :
    @NTy.encode Φ τ left.1 = @NTy.encode Ψ τ right.1 ∧
      @NTy.encode Φ (.struct source arguments rest) left.2 =
        @NTy.encode Ψ (.struct source arguments rest) right.2 := by
  simp only [NTy.encode_struct, HList.encode_cons, RuntimeValue.nominal.injEq, true_and] at same
  have fields := List.toArray_inj same
  simp only [List.cons.injEq] at fields
  refine ⟨fields.1, ?_⟩
  simp only [NTy.encode_struct, fields.2]

/-- A struct row's head field moves with it. -/
@[vir_denote_norm] theorem Skolems.reframe_struct_fst {source : StructHandle}
    {arguments : NRow} {τ : NTy} {rest : NRow} (bounded)
    (value : @NTy.carrier (Θ.toCarriers).toSorts (.struct source arguments (.cons τ rest))) :
    (Θ.reframe own bounded value).1 =
      Θ.reframe own (paramsAll_head (paramsAll_fields bounded)) value.1 := by
  apply @NTy.encode_injective ((Θ.instance params own).toCarriers).toSorts τ
  rw [(encode_fields _ _ (Θ.encode_reframe own bounded value)).1,
    Θ.encode_reframe]

/-- And so do the rest of its fields. -/
@[vir_denote_norm] theorem Skolems.reframe_struct_snd {source : StructHandle}
    {arguments : NRow} {τ : NTy} {rest : NRow} (bounded)
    (value : @NTy.carrier (Θ.toCarriers).toSorts (.struct source arguments (.cons τ rest))) :
    (Θ.reframe own bounded value).2 =
      Θ.reframe own (τ := .struct source arguments rest)
        (paramsAll_struct
          (by simp only [NTy.paramsAll, Bool.and_eq_true] at bounded; exact bounded.1)
          (paramsAll_tail (paramsAll_fields bounded))) value.2 := by
  apply @NTy.encode_injective ((Θ.instance params own).toCarriers).toSorts
    (.struct source arguments rest)
  rw [(encode_fields _ _ (Θ.encode_reframe own bounded value)).2,
    Θ.encode_reframe]

/-- The encodings of a tuple's head and rest, from the tuple's. -/
private theorem encode_elements {Φ Ψ : Sorts} {τ : NTy} {rest : NRow}
    (left : @HList Φ (.cons τ rest)) (right : @HList Ψ (.cons τ rest))
    (same : @NTy.encode Φ (.tuple (.cons τ rest)) left =
      @NTy.encode Ψ (.tuple (.cons τ rest)) right) :
    @NTy.encode Φ τ left.1 = @NTy.encode Ψ τ right.1 ∧
      @NTy.encode Φ (.tuple rest) left.2 = @NTy.encode Ψ (.tuple rest) right.2 := by
  simp only [NTy.encode_tuple, HList.encode_cons, RuntimeValue.tuple.injEq] at same
  have elements := List.toArray_inj same
  simp only [List.cons.injEq] at elements
  refine ⟨elements.1, ?_⟩
  simp only [NTy.encode_tuple, elements.2]

/-- A row's head component moves with the row: a row is the carrier of a tuple,
which is how a variant's row moves (`Skolems.reframe_enum_inl`), as a struct's
row moves through `Skolems.reframe_struct_fst`. -/
@[vir_denote_norm] theorem Skolems.reframe_tuple_fst {τ : NTy} {rest : NRow} (bounded)
    (value : @NTy.carrier (Θ.toCarriers).toSorts (.tuple (.cons τ rest))) :
    (Θ.reframe own (τ := .tuple (.cons τ rest)) bounded value).1 =
      Θ.reframe own (τ := τ) (paramsAll_head bounded) value.1 := by
  apply @NTy.encode_injective ((Θ.instance params own).toCarriers).toSorts τ
  rw [(encode_elements _ _ (Θ.encode_reframe own bounded value)).1,
    Θ.encode_reframe]

/-- And so does the rest of it. -/
@[vir_denote_norm] theorem Skolems.reframe_tuple_snd {τ : NTy} {rest : NRow} (bounded)
    (value : @NTy.carrier (Θ.toCarriers).toSorts (.tuple (.cons τ rest))) :
    (Θ.reframe own (τ := .tuple (.cons τ rest)) bounded value).2 =
      Θ.reframe own (τ := .tuple rest) (paramsAll_tail bounded) value.2 := by
  apply @NTy.encode_injective ((Θ.instance params own).toCarriers).toSorts (.tuple rest)
  rw [(encode_elements _ _ (Θ.encode_reframe own bounded value)).2,
    Θ.encode_reframe]

private theorem paramsAll_variant {f : Nat → Bool} {source : StructHandle}
    {arguments fields : NRow} {name : String} {names : List String} {rest : NRows}
    {distinct : (name :: names).Nodup}
    (bounded : (NTy.enum source arguments (name :: names) (.cons fields rest) distinct).paramsAll f
      = true) : fields.paramsAll f = true := by
  simp only [NTy.paramsAll, NRows.paramsAll, Bool.and_eq_true] at bounded
  exact bounded.2.1

private theorem paramsAll_variants {f : Nat → Bool} {source : StructHandle}
    {arguments fields : NRow} {name : String} {names : List String} {rest : NRows}
    {distinct : (name :: names).Nodup}
    (bounded : (NTy.enum source arguments (name :: names) (.cons fields rest) distinct).paramsAll f
      = true) :
    (NTy.enum source arguments names rest (List.nodup_cons.mp distinct).2).paramsAll f = true := by
  simp only [NTy.paramsAll, NRows.paramsAll, Bool.and_eq_true] at bounded ⊢
  exact ⟨bounded.1, bounded.2.2⟩

/-- A value under a variant's name moves under that name, its row moving with
it: the encoding of such a value is its row's under the handle and the name
(`NTy.encode_enum_inl`) and moving a value keeps its encoding
(`Skolems.encode_reframe`).  With this and `Skolems.reframe_enum_inr` every
reader of a variant — its name, a projection, a selection — reads a moved value
where it reads a value (`variantName_inl`, `Which.project?`), as a struct's
fields are read through `Skolems.reframe_struct_fst`. -/
@[vir_denote_norm] theorem Skolems.reframe_enum_inl {source : StructHandle}
    {arguments fields : NRow} {name : String} {names : List String} {rest : NRows}
    {distinct : (name :: names).Nodup} (bounded)
    (value : @HList (Θ.toCarriers).toSorts fields) :
    Θ.reframe own
        (τ := .enum source arguments (name :: names) (.cons fields rest) distinct) bounded
        (.inl value) =
      .inl (Θ.reframe own (τ := .tuple fields) (paramsAll_variant bounded) value) := by
  apply @NTy.encode_injective ((Θ.instance params own).toCarriers).toSorts
    (.enum source arguments (name :: names) (.cons fields rest) distinct)
  have row := Θ.encode_reframe own (τ := .tuple fields) (paramsAll_variant bounded) value
  simp only [NTy.encode_tuple, RuntimeValue.tuple.injEq] at row
  simp only [Θ.encode_reframe, NTy.encode_enum_inl, row]

/-- A value under a later variant's name moves under the later names: such a
value encodes as it does at the enum of those names (`NTy.encode_enum_inr`),
and moving it keeps its encoding. -/
@[vir_denote_norm] theorem Skolems.reframe_enum_inr {source : StructHandle}
    {arguments fields : NRow} {name : String} {names : List String} {rest : NRows}
    {distinct : (name :: names).Nodup} (bounded)
    (value : @variantCarrier (Θ.toCarriers).toSorts names rest) :
    Θ.reframe own
        (τ := .enum source arguments (name :: names) (.cons fields rest) distinct) bounded
        (.inr value) =
      .inr (Θ.reframe own
        (τ := .enum source arguments names rest (List.nodup_cons.mp distinct).2)
        (paramsAll_variants bounded) value) := by
  apply @NTy.encode_injective ((Θ.instance params own).toCarriers).toSorts
    (.enum source arguments (name :: names) (.cons fields rest) distinct)
  simp only [Θ.encode_reframe, NTy.encode_enum_inr]

/-- A vector's elements move with it. -/
@[vir_denote_norm] theorem Skolems.reframe_vector_values {element : NTy} (bounded)
    (vector : @NTy.carrier (Θ.toCarriers).toSorts (.vector element)) :
    (Θ.reframe own (τ := .vector element) bounded vector).values =
      vector.values.map (Θ.reframe own (τ := element) (by simpa only [NTy.paramsAll] using bounded)) := by
  have encoded := Θ.encode_reframe own bounded vector
  rw [@NTy.encode_vector ((Θ.instance params own).toCarriers).toSorts, @NTy.encode_vector (Θ.toCarriers).toSorts] at encoded
  simp only [RuntimeValue.vector.injEq] at encoded
  apply Array.map_inj_right (f := @NTy.encode ((Θ.instance params own).toCarriers).toSorts element)
    (fun _ _ same => @NTy.encode_injective ((Θ.instance params own).toCarriers).toSorts element _ _ same) |>.mp
  rw [Array.map_map]
  refine encoded.trans ?_
  apply Array.map_congr_left
  intro value _
  exact (Θ.encode_reframe own _ value).symm

open Lean Meta in
/-- A value of a type without parameters or function values, whose carrier and
encoding do not read the frame, stays itself (`Skolems.reframe_of_encode`). -/
simproc ↓ [vir_denote, vir_denote_norm] reframeClosed (@Skolems.reframe _ ?functions _ _ _ _ _ _) := fun e => do
  let #[unit, functions, frame, params, own, τ, bounded, value] := e.getAppArgs
    | return .continue
  withTransparency .default do
    unless (← whnf (mkApp (mkConst ``NTy.paramFree) τ)).isConstOf ``true do return .continue
    let source ← inferType value
    unless ← isDefEq source (← inferType e) do return .continue
    let carriers ← mkEqRefl source
    let identity := mkAppN (mkConst ``Skolems.reframe_of_encode)
      #[unit, functions, frame, params, own, τ, bounded, value, carriers]
    let .forallE _ encodedType _ _ ← whnf (← inferType identity) | return .continue
    let some encoded ← forallBoundedTelescope encodedType (some 1) fun bound statement => do
        let_expr Eq _ left right := statement | return none
        unless ← isDefEq left right do return none
        return some (← mkLambdaFVars bound (← mkEqRefl right))
      | return .continue
    let proof ← mkExpectedTypeHint (mkApp identity encoded) (← mkEq e value)
    return .visit { expr := value, proof? := some proof }

/-- A ghost slot moves with its value. -/
@[vir_denote_norm] theorem Skolems.reframe_ghost {slot : NTy} (bounded)
    (value : @NTy.carrier (Θ.toCarriers).toSorts (.ghost slot)) :
    Θ.reframe own (τ := .ghost slot) bounded value =
      Θ.reframe own (τ := slot) (by simpa only [NTy.paramsAll] using bounded) value := by
  apply @NTy.encode_injective ((Θ.instance params own).toCarriers).toSorts (.ghost slot)
  rw [Θ.encode_reframe, @NTy.encode_ghost ((Θ.instance params own).toCarriers).toSorts,
    Θ.encode_reframe, @NTy.encode_ghost (Θ.toCarriers).toSorts]

end Reframe

/-- A parameter's reading is the same at a frame and at the frame of its type
arguments: the arguments carry that reading. -/
theorem Skolems.argument_instance (Θ : Skolems unit) {params : NRow}
    (own : params.isParamsFrom 0 = true) (index : Nat) (below : index < params.length) :
    (Θ.instance params own).argument index = Θ.argument index := by
  show NTy.argumentWith (Skolems.runtime unit).argument
      ((Θ.typeArgs params own).1.argument index) = Θ.argument index
  simp only [NRow.argument, Skolems.typeArgs]
  rw [NRow.getD_map_params (NTy.argumentWith Θ.argument) params 0 own index below (.param index)]
  simp only [Nat.zero_add, NTy.argumentWith, Skolems.argument_runtime]
  exact NTy.argumentWith_param _

/-- A type over a declaration's parameters is read alike at a frame and at the
frame of its type arguments. -/
theorem Skolems.argumentWith_instance (Θ : Skolems unit) {params : NRow}
    (own : params.isParamsFrom 0 = true) (τ : NTy)
    (bounded : τ.paramsAll (fun index => decide (index < params.length)) = true) :
    NTy.argumentWith (Θ.instance params own).argument τ = NTy.argumentWith Θ.argument τ := by
  have agree : ∀ index, decide (index < params.length) = true →
      (Θ.instance params own).argument index = Θ.argument index :=
    fun index mentioned => Θ.argument_instance own index (of_decide_eq_true mentioned)
  cases τ <;>
    first
      | exact agree _ (by simpa only [NTy.paramsAll] using bounded)
      | (simp only [NTy.argumentWith]
         exact congrArg NTy.observed
           (NTy.substWith_congr agree _ (by simpa only [NTy.paramsAll] using bounded)))
      | exact NTy.substWith_congr agree _ bounded

/-- A row over a declaration's parameters resolves alike at a frame and at
the frame of its type arguments. -/
theorem Skolems.map_resolve_instance
    (Θ : Skolems unit) {params : NRow}
    (own : params.isParamsFrom 0 = true) :
    (row : NRow) → row.paramsAll (fun index => decide (index < params.length)) = true →
      row.map (NTy.argumentWith (Θ.instance params own).argument) =
        row.map (NTy.argumentWith Θ.argument)
  | .nil, _ => rfl
  | .cons τ rest, bounded => by
      simp only [NRow.paramsAll, Bool.and_eq_true] at bounded
      show NRow.cons (NTy.argumentWith (Θ.instance params own).argument τ)
          (rest.map (NTy.argumentWith (Θ.instance params own).argument)) =
        .cons (NTy.argumentWith Θ.argument τ) (rest.map (NTy.argumentWith Θ.argument))
      exact congr (congrArg NRow.cons (Θ.argumentWith_instance own τ bounded.1))
        (Θ.map_resolve_instance own rest bounded.2)

private theorem stored_val_cast {τ₁ τ₂ : NTy} {args₁ args₂ : NRow}
    (same : ResourceType.mk τ₁ args₁ .value = ResourceType.mk τ₂ args₂ .value)
    (sameType : τ₁ = τ₂)
    (value : ResourceType.Stored unit τ₁) :
    (cast (congrArg (ResourceType.carrier unit) same) value).val =
      cast (congrArg (@NTy.carrier ((Carriers.runtime unit)).toSorts) sameType) value.val := by
  obtain ⟨hτ, hargs, -⟩ := ResourceType.mk.inj same
  subst hτ
  subst hargs
  have h1 : same = rfl := Subsingleton.elim _ _
  have h2 : sameType = rfl := Subsingleton.elim _ _
  rw [h1, h2]
  rfl

/-- A declaration's invariant at a frame: of the value the frame reads at its
resource, the invariant at the resource's type arguments of the value moved
to the frame they induce (`Skolems.reframe`). -/
-- retires with doc/plan/data-and-global-invariants.md, "The stored-invariant layer still exists for the declarations the carriers do not state"
theorem StoredCase.atFrame (Θ : Skolems unit) {σ : NTy} {params : NRow}
    {holds : (θ : TypeArgs) →
      @NTy.carrier ((Skolems.instantiate θ (Skolems.runtime unit)).toCarriers).toSorts σ → Prop}
    (own : params.isParamsFrom 0 = true)
    (bounded : σ.paramsAll (fun index => decide (index < params.length)) = true)
    (value : (@Skolems.resource unit _ Θ σ params).carrier unit) :
    StoredCase σ params holds (@Skolems.resource unit _ Θ σ params) value ↔
      holds (Θ.typeArgs params own) (Θ.reframe own bounded (Θ.ofRuntime σ value.val)) := by
  have same : @Skolems.resource unit _ Θ σ params =
      @Skolems.resource unit _ (Θ.instance params own) σ params := by
    show ResourceType.mk (Θ.resolve σ) (params.map (NTy.argumentWith Θ.argument)) .value =
      ResourceType.mk (σ.subst (params.map (NTy.argumentWith Θ.argument)))
        (params.map (NTy.argumentWith (Θ.instance params own).argument)) .value
    rw [Θ.map_resolve_instance own params
      (NRow.paramsAll_params params 0 params.length own (by omega)),
      Θ.resolve_instance_params own bounded]
  constructor
  · intro stored
    have held := stored (Θ.typeArgs params own).2 same
    rw [stored_val_cast same (Θ.resolve_instance_params own bounded).symm value] at held
    unfold Skolems.reframe
    rw [Θ.toRuntime_ofRuntime]
    exact held
  · intro held _ same'
    rw [stored_val_cast same' (Θ.resolve_instance_params own bounded).symm value]
    unfold Skolems.reframe at held
    rw [Θ.toRuntime_ofRuntime] at held
    exact held

/-- The entries and substitutions of type arguments are inhabitable: what a
stored value's resource arguments are, spelled as normalization leaves them. -/
@[simp] theorem TypeArgs.getD_inhabitable (θ : TypeArgs) (index : Nat) (default : NTy)
    (held : default.inhabitable = true) : (θ.1.getD index default).inhabitable = true :=
  NRow.getD_inhabitable θ.1 θ.2 index default held

@[simp] theorem TypeArgs.subst_inhabitable (θ : TypeArgs) (τ : NTy) (held : τ.inhabitable = true) :
    (τ.subst θ.1).inhabitable = true :=
  NTy.inhabitable_subst θ.1 θ.2 τ held

section Frames
variable [Θ : Skolems unit]

/-- The native type a frame gives a type identifier of a function: read at the frame's
types for its type parameters, so that a view of one erases it (`ntyOfFuel`). -/
def frameNty (unit : ValidatedUnit) [FunctionCarriers unit] [Skolems unit]
    (handle : FunctionHandle) (typeId : TypeId) : Option NTy :=
  ntyOfFuel (unitTypes unit) unit
    (NRow.ofList ((List.range (typeArity unit handle)).map fun index =>
      Skolems.resolve (.param index)))
    (typeFuel unit) handle.namespaceId typeId

/-- A function's frame agrees with its runtime type instantiation on the
types it reads, as the runtime reads them: each type it requires
(`requiredAt`), instantiated as the runtime does, is the frame's reading
of it, as a native type and, for a resource type, with its arguments; and
each of its type parameters, read at its node through the instantiation, is
the frame's type for it.  The runtime instantiation maps only interned
types, and static typing interns the required ones. -/
structure Coherent (unit : ValidatedUnit) [FunctionCarriers unit]
    [Skolems unit] (handle : FunctionHandle)
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
      frameNty unit handle typeId
  params : ∀ index < typeArity unit handle, ∀ node,
    paramNodeIn? unit handle.namespaceId index = some node →
    ntyOf unit handle.namespaceId (instantiatedTypeId typeInstantiation node) =
      some (Skolems.resolve (.param index))

/-- Whether a frame agrees with a runtime type instantiation on a function's
required types and type parameters, decided by evaluation. -/
def coherentCheck (unit : ValidatedUnit) [FunctionCarriers unit]
    [Skolems unit] (handle : FunctionHandle)
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
              frameNty unit handle typeId))) &&
  (List.range (typeArity unit handle)).all fun index =>
    match paramNodeIn? unit handle.namespaceId index with
    | none => true
    | some node =>
        decide (ntyOf unit handle.namespaceId (instantiatedTypeId typeInstantiation node) =
          some (Skolems.resolve (.param index)))

theorem Coherent.ofCheck {unit : ValidatedUnit} [FunctionCarriers unit]
    [Skolems unit] {handle : FunctionHandle}
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
    (value : @NTy.carrier ((Carriers.runtime unit)).toSorts (Θ.resolve τ)) :
    @NTy.encode ((Carriers.runtime unit)).toSorts (Θ.resolve τ) value =
      @NTy.encode (Θ.toCarriers).toSorts τ (Θ.ofRuntime τ value) := by
  rw [← Θ.encode_toRuntime, Θ.toRuntime_ofRuntime]

/-- The identity leaves a row as it is. -/
theorem NRow.map_id : (row : NRow) → row.map (fun τ => τ) = row
  | .nil => rfl
  | .cons τ rest => congrArg (NRow.cons τ) (NRow.map_id rest)

/-- The runtime frame's row for a function's type parameters leaves each parameter. -/
theorem NRow.getD_ofList_params (arity index : Nat) :
    (NRow.ofList ((List.range arity).map fun index => NTy.param index)).getD index (.param index) =
      .param index := by
  have read : ∀ (types : List NTy) (position : Nat) (fallback : NTy),
      (NRow.ofList types).getD position fallback = types.getD position fallback := by
    intro types
    induction types with
    | nil => intro _ _; rfl
    | cons head tail ih =>
        intro position fallback
        cases position with
        | zero => rfl
        | succ position => simpa [NRow.ofList, NRow.getD] using ih position fallback
  rw [read]
  by_cases below : index < arity <;> simp [List.getD, below]

/-- At the runtime frame, the empty instantiation is coherent: every type
identifier denotes its own type, and every parameter stays. -/
theorem coherent_runtime (unit : ValidatedUnit) [FunctionCarriers unit]
    (handle : FunctionHandle) :
    @Coherent unit _ (Skolems.runtime unit) handle #[] := by
  refine @Coherent.mk unit _ (Skolems.runtime unit) handle #[]
    (fun typeId _ resource named _ _ => ?_) (fun typeId _ τ named _ => ?_)
    (fun index _ node found => ?_)
  · rw [instantiatedTypeId_empty, named]
    show some resource = some ⟨resource.type,
      resource.arguments.map (NTy.argumentWith fun index => NTy.param index), .value⟩
    rw [funext NTy.argumentWith_param, NRow.map_id]
    rw [← resourceOf_kind named]
  · rw [instantiatedTypeId_empty]
    show ntyOf unit handle.namespaceId typeId = ntyOfFuel (unitTypes unit) unit
      (NRow.ofList ((List.range (typeArity unit handle)).map fun index => NTy.param index))
      (typeFuel unit) handle.namespaceId typeId
    rw [ntyOfFuel_identity _ _ _ (NRow.getD_ofList_params (typeArity unit handle))]
    rfl
  · rw [instantiatedTypeId_empty, ntyOf_paramNode found]
    rfl

end VerexIR.Proofs.Denote
