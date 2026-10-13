-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.TableMemory
import VerexIR.Semantics.ValueTyping

/-!
# Resource types of a unit

The native types the unit's type tables denote, and the resource type each
runtime storage key names: how typed global memory (`Memory`) relates to the
runtime's (`designs/static-memory.md`).  A runtime key is the declaring
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

/-- Runtime global memory holding exactly what a typed memory holds: under
every key, the physical value of the value at the resource type its type
identifier denotes, and nothing under a key that denotes none.  Its entries
are in key order, so the memory determines it. -/
def Encodes (unit : ValidatedUnit) (memory : Memory unit) (globals : GlobalMap) : Prop :=
  globals.Sorted ∧ ∀ namespaceId typeId key, globals.lookup ⟨namespaceId, typeId, key⟩ =
    match runtimeResourceOf unit namespaceId typeId with
    | some resource => (memory resource key).map (resource.erase (unit := unit))
    | none => none

/-- A memory's runtime encoding is unique. -/
theorem Encodes.unique {unit : ValidatedUnit} {memory : Memory unit} {left right : GlobalMap}
    (encodes : Encodes unit memory left) (encodes' : Encodes unit memory right) : left = right :=
  GlobalMap.ext encodes.1 encodes'.1 fun ⟨namespaceId, typeId, key⟩ => by
    rw [encodes.2, encodes'.2]

/-- A table some value stored in a memory holds. -/
def MemoryTable (unit : ValidatedUnit) (memory : Memory unit)
    (table : NTy × String × RuntimeValue) : Prop :=
  ∃ namespaceId typeId resource key value,
    runtimeResourceOf unit namespaceId typeId = some resource ∧
    memory resource key = some value ∧ table ∈ resource.tables (unit := unit) value

/-- Native table storage holding the tables the values of a typed memory
hold, each at the slot of its owner's type and handle (`tableSlot?`), in
slot order and without loans, and the allocation history at its resource.
It may hold further tables, which no global reaches: those the frames of a
run hold (`designs/intrinsic-maps.md`, "Tables in the runtime semantics").
The storage determines the memory, not the memory the storage. -/
structure TablesHeld (unit : ValidatedUnit) (memory : Memory unit)
    (storage : NativeTableStorage) : Prop where
  sorted : storage.contents.Sorted
  held : ∀ table, MemoryTable unit memory table →
    ∃ key, tableSlot? unit table.1 table.2.1 = some key ∧
      storage.contents.lookup key = some table.2.2
  plain : ∀ slot ∈ storage.contents.entries, SemanticOperations.Plain slot.value
  allocated : ∀ key, memory TableMemory.allocationResource key =
    if key = .unit then some (TableMemory.encodeAllocated storage.allocated) else none

/-- Native table storage holding exactly the tables the values of a typed
memory hold: held as above, and nothing else. The memory determines it. -/
structure TablesEncode (unit : ValidatedUnit) (memory : Memory unit)
    (storage : NativeTableStorage) : Prop extends TablesHeld unit memory storage where
  stored : ∀ slot ∈ storage.contents.entries, ∃ table, MemoryTable unit memory table ∧
    tableSlot? unit table.1 table.2.1 = some slot.key ∧ slot.value = table.2.2

/-- A key some slot of a store publishes is a stored slot. -/
theorem _root_.VerexIR.GlobalMap.mem_of_lookup {globals : GlobalMap} {key : GlobalKey}
    {value : RuntimeValue} (found : globals.lookup key = some value) :
    (⟨key, value⟩ : GlobalSlot) ∈ globals.entries := by
  unfold GlobalMap.lookup at found
  obtain ⟨slot, slotFound, slotValue⟩ := Option.map_eq_some_iff.mp found
  have member := Array.mem_of_find?_eq_some slotFound
  have keyed := Array.find?_some slotFound
  simp only [decide_eq_true_eq] at keyed
  cases slot
  cases keyed
  cases slotValue
  exact member

theorem TablesEncode.unique {memory : Memory unit} {left right : NativeTableStorage}
    (leftEncodes : TablesEncode unit memory left) (rightEncodes : TablesEncode unit memory right) :
    left = right := by
  have lookups : ∀ key value, left.contents.lookup key = some value →
      right.contents.lookup key = some value := by
    intro key value found
    obtain ⟨table, held, slotted, stored⟩ :=
      leftEncodes.stored ⟨key, value⟩ (GlobalMap.mem_of_lookup found)
    obtain ⟨key', slotted', found'⟩ := rightEncodes.held table held
    simp only at slotted stored
    rw [slotted] at slotted'
    cases slotted'
    rw [found', stored]
  have lookups' : ∀ key value, right.contents.lookup key = some value →
      left.contents.lookup key = some value := by
    intro key value found
    obtain ⟨table, held, slotted, stored⟩ :=
      rightEncodes.stored ⟨key, value⟩ (GlobalMap.mem_of_lookup found)
    obtain ⟨key', slotted', found'⟩ := leftEncodes.held table held
    simp only at slotted stored
    rw [slotted] at slotted'
    cases slotted'
    rw [found', stored]
  have contents : left.contents = right.contents :=
    GlobalMap.ext leftEncodes.sorted rightEncodes.sorted fun key => by
      cases foundLeft : left.contents.lookup key with
      | some value => exact (lookups key value foundLeft).symm
      | none =>
          cases foundRight : right.contents.lookup key with
          | some value => rw [lookups' key value foundRight] at foundLeft; cases foundLeft
          | none => rfl
  have allocated := TableMemory.encodeAllocated_injective left.allocated right.allocated
    (Option.some.inj (by
      simpa using (leftEncodes.allocated .unit).symm.trans (rightEncodes.allocated .unit)))
  cases left
  cases right
  cases contents
  cases allocated
  rfl

/-- The observable runtime stores hold a typed memory: the globals encode it
and the Table storage holds its tables, and possibly tables of other frames.
Loan registries remain separate. -/
structure StorageHolds (unit : ValidatedUnit) (memory : Memory unit)
    (state : RuntimeState) : Prop where
  globals : Encodes unit memory state.globals
  tables : TablesHeld unit memory state.tables

/-- The observable runtime stores encode one typed memory. Loan registries
remain separate; Table allocation history is observable even after destruction. -/
structure StorageEncodes (unit : ValidatedUnit) (memory : Memory unit)
    (state : RuntimeState) : Prop where
  globals : Encodes unit memory state.globals
  tables : TablesEncode unit memory state.tables

/-- Stores encoding a memory hold it. -/
theorem StorageEncodes.holds {memory : Memory unit} {state : RuntimeState}
    (encodes : StorageEncodes unit memory state) : StorageHolds unit memory state :=
  ⟨encodes.globals, encodes.tables.toTablesHeld⟩

/-- A memory determines both observable stores, independently of loan IDs. -/
theorem StorageEncodes.unique {memory : Memory unit} {left right : RuntimeState}
    (leftEncodes : StorageEncodes unit memory left)
    (rightEncodes : StorageEncodes unit memory right) :
    left.globals = right.globals ∧ left.tables = right.tables :=
  ⟨leftEncodes.globals.unique rightEncodes.globals,
   leftEncodes.tables.unique rightEncodes.tables⟩

/-- A run's start: stores holding the memory, and the tables the arguments
hold, at the frame, in native storage. The storage may hold more: the
tables the caller's and its ancestors' other values hold. -/
structure Encloses (unit : ValidatedUnit) [Skolems unit] (memory : Memory unit)
    (state : RuntimeState) (tables : List (NTy × String × RuntimeValue)) : Prop where
  stores : StorageHolds unit memory state
  held : TableMemory.TablesIn unit tables state.tables

/-- Stores holding a memory enclose no table. -/
theorem Encloses.of_nil [Skolems unit] {memory : Memory unit} {state : RuntimeState}
    (stores : StorageHolds unit memory state) {tables : List (NTy × String × RuntimeValue)}
    (nil : tables = []) : Encloses unit memory state tables :=
  ⟨stores, nil ▸ TableMemory.TablesIn.nil state.tables⟩

/-- A start enclosing tables read at an induced frame encloses them read at
the caller's frame, their owners at the type arguments. -/
theorem Encloses.instantiate (θ : TypeArgs) (Θ : Skolems unit) (memory : Memory unit)
    (state : RuntimeState) (tables : List (NTy × String × RuntimeValue)) :
    @Encloses unit (Skolems.instantiate θ Θ) memory state tables ↔
      @Encloses unit Θ memory state (tables.map (tableSubst θ)) :=
  ⟨fun held => @Encloses.mk unit Θ memory state _
      (@Encloses.stores unit (Skolems.instantiate θ Θ) memory state tables held)
      ((@TableMemory.TablesIn.instantiate unit θ Θ tables state.tables).mp
        (@Encloses.held unit (Skolems.instantiate θ Θ) memory state tables held)),
   fun held => @Encloses.mk unit (Skolems.instantiate θ Θ) memory state tables
      (@Encloses.stores unit Θ memory state _ held)
      ((@TableMemory.TablesIn.instantiate unit θ Θ tables state.tables).mpr
        (@Encloses.held unit Θ memory state _ held))⟩

/-- A resource type neither runtime store nor the allocation history names. -/
def Unnamed (unit : ValidatedUnit) (resource : ResourceType) : Prop :=
  (∀ namespaceId typeId, runtimeResourceOf unit namespaceId typeId ≠ some resource) ∧
  resource ≠ TableMemory.allocationResource

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
    invariant resource (resource.encode (unit := unit) value)

namespace MemoryInvariants

variable {invariant : ResourceType → RuntimeValue → Prop} {memory : Memory unit}

/-- A write keeps the invariants when the value written satisfies its own. -/
theorem set (holds : MemoryInvariants invariant memory) {resource : ResourceType}
    {key : StorageKey} {value : Option (resource.carrier unit)}
    (stored : ∀ written, value = some written →
      invariant resource (resource.encode (unit := unit) written)) :
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
    (stored : invariant resource (resource.encode (unit := unit) value)) :
    MemoryInvariants invariant (memory.set resource key (some value)) :=
  holds.set fun _ written => by cases written; exact stored

/-- Removing a value keeps the invariants. -/
theorem set_none (holds : MemoryInvariants invariant memory) {resource : ResourceType}
    {key : StorageKey} : MemoryInvariants invariant (memory.set resource key none) :=
  holds.set fun _ written => nomatch written

/-- A value read from memory satisfies the invariant of its resource type. -/
theorem read (holds : MemoryInvariants invariant memory) {resource : ResourceType}
    {key : StorageKey} {value : resource.carrier unit} (read : memory resource key = some value) :
    invariant resource (resource.encode (unit := unit) value) :=
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
  | .table key value => NTy.paramsBelow bound key && NTy.paramsBelow bound value
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
  ⟨Θ.resolve type, arguments.map Θ.resolve, .value⟩


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
    show some resource = some ⟨resource.type, resource.arguments.map fun τ => τ, .value⟩
    rw [NRow.map_id]
    rw [← resourceOf_kind named]
  · rw [instantiatedTypeId_empty, named]
    rfl
  · rw [instantiatedTypeId_empty, ntyOf_paramNode found]
    rfl

/-! ## Data invariants of stored values

The declaration predicate states the authored invariant of a nominal value at
its resolved type arguments. The arguments matter even for phantom
parameters: two values with identical physical encodings can have different
invariants. Traversal follows the native type: fields, variants' payloads,
vector elements, and the entries of a map's contents. The predicate reads no
memory. -/

namespace DataInvariant

def elements : RuntimeValue → List RuntimeValue
  | .vector values => values.toList
  | _ => []

mutual
def value (declared : StructHandle → NRow → RuntimeValue → Prop) : NTy → RuntimeValue → Prop
  | .struct source arguments fields, raw =>
      declared source arguments raw ∧ row declared fields raw 0
  | .enum source arguments names rows _, raw =>
      declared source arguments raw ∧ variant declared names rows raw
  | .tuple fields, raw => row declared fields raw 0
  | .vector element, raw => ∀ held ∈ elements raw, value declared element held
  -- A map's contents: each entry's key and value.
  | .table key element, raw => ∀ entry ∈ elements raw,
      value declared key (entry.field 0) ∧ value declared element (entry.field 1)
  | .ref element, raw => value declared element (raw.field 0) ∧ value declared element (raw.field 1)
  | _, _ => True

def row (declared : StructHandle → NRow → RuntimeValue → Prop) : NRow → RuntimeValue → Nat → Prop
  | .nil, _, _ => True
  | .cons type rest, raw, index =>
      value declared type (raw.field index) ∧ row declared rest raw (index + 1)

def variant (declared : StructHandle → NRow → RuntimeValue → Prop) :
    List String → NRows → RuntimeValue → Prop
  | name :: names, .cons fields rest, raw =>
      (raw.variant? = some name → row declared fields raw 0) ∧
        variant declared names rest raw
  | _, _, _ => True
end

/-- Primitive integer elements carry no declared nominal invariant. Resolve
this before traversing a potentially large stored vector expression. -/
@[vir_denote_norm↓] theorem integer_vector
    (declared : StructHandle → NRow → RuntimeValue → Prop) (width : Nat) (signed : Bool)
    (raw : RuntimeValue) : value declared (.vector (.int width signed)) raw := by
  intro held member
  trivial

/-- Nested primitive vectors likewise need no nominal-invariant traversal. -/
@[vir_denote_norm↓] theorem integer_vector_vector
    (declared : StructHandle → NRow → RuntimeValue → Prop) (width : Nat) (signed : Bool)
    (raw : RuntimeValue) : value declared (.vector (.vector (.int width signed))) raw := by
  intro held member
  exact integer_vector declared width signed held

end DataInvariant

end VerexIR.Proofs.Denote
