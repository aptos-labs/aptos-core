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
every key, the encoding of the value at the resource type its type
identifier denotes, a table's entries included, and nothing under a key
that denotes none.  Its entries are in key order, so the memory determines
it. -/
def Encodes (unit : ValidatedUnit) (memory : Memory unit) (globals : GlobalMap) : Prop :=
  globals.Sorted ∧ ∀ namespaceId typeId key, globals.lookup ⟨namespaceId, typeId, key⟩ =
    match runtimeResourceOf unit namespaceId typeId with
    | some resource => (memory resource key).map (resource.encode (unit := unit))
    | none => none

/-- A memory's runtime encoding is unique. -/
theorem Encodes.unique {unit : ValidatedUnit} {memory : Memory unit} {left right : GlobalMap}
    (encodes : Encodes unit memory left) (encodes' : Encodes unit memory right) : left = right :=
  GlobalMap.ext encodes.1 encodes'.1 fun ⟨namespaceId, typeId, key⟩ => by
    rw [encodes.2, encodes'.2]

/-- The observable runtime stores encode one typed memory: the globals hold
its values, and the handles allocated to tables are its allocation history,
observable after a table's destruction. Loan registries remain separate. -/
structure StorageEncodes (unit : ValidatedUnit) (memory : Memory unit)
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
resource type, as the Move Prover assumes of the memory a function accesses
(`doc/designs/static-memory.md`, "Contracts"). The invariants read no memory,
so a write keeps those of the values it leaves alone. -/
def MemoryInvariants (invariant : (resource : ResourceType) → resource.carrier unit → Prop)
    (memory : Memory unit) : Prop :=
  ∀ resource key value, memory resource key = some value → invariant resource value

namespace MemoryInvariants

variable {invariant : (resource : ResourceType) → resource.carrier unit → Prop}
  {memory : Memory unit}

/-- A write keeps the invariants when the value written satisfies its own. -/
theorem set (holds : MemoryInvariants invariant memory) {resource : ResourceType}
    {key : StorageKey} {value : Option (resource.carrier unit)}
    (stored : ∀ written, value = some written → invariant resource written) :
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
    {key : StorageKey} {value : resource.carrier unit} (stored : invariant resource value) :
    MemoryInvariants invariant (memory.set resource key (some value)) :=
  holds.set fun _ written => by cases written; exact stored

/-- Removing a value keeps the invariants. -/
theorem set_none (holds : MemoryInvariants invariant memory) {resource : ResourceType}
    {key : StorageKey} : MemoryInvariants invariant (memory.set resource key none) :=
  holds.set fun _ written => nomatch written

/-- A value read from memory satisfies the invariant of its resource type. -/
theorem read (holds : MemoryInvariants invariant memory) {resource : ResourceType}
    {key : StorageKey} {value : resource.carrier unit} (read : memory resource key = some value) :
    invariant resource value :=
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
  | .ghost slot => NTy.paramsBelow bound slot
  | .function parameters _ results =>
      NRow.paramsBelow bound parameters && NRow.paramsBelow bound results
  | .unit | .bool | .int _ _ | .num | .address | .signer | .string | .bytes => true

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


end Frames

/-- The data invariant of the stored values of a declaration of native type
`σ` and arguments `arguments`, its type parameters: of a value of the
resource of an instance, `holds` at the instance's type arguments of the
value read at the frame they induce over the runtime frame, as a generic
contract spells its frame; nothing of a value of another resource. -/
def StoredCase (σ : NTy) (arguments : NRow)
    (holds : (θ : TypeArgs) →
      @NTy.carrier (Skolems.instantiate θ (Skolems.runtime unit)).toCarriers σ → Prop)
    (resource : ResourceType) (value : resource.carrier unit) : Prop :=
  ∀ (inhabitable : resource.arguments.inhabitable = true)
    (same : resource = @Skolems.resource unit
      (Skolems.instantiate ⟨resource.arguments, inhabitable⟩ (Skolems.runtime unit)) σ arguments),
    holds ⟨resource.arguments, inhabitable⟩
      ((Skolems.instantiate ⟨resource.arguments, inhabitable⟩ (Skolems.runtime unit)).ofRuntime σ
        (cast (congrArg (ResourceType.carrier unit) same) value))

/-- A stored value satisfies its declaration's invariant where it does at
its resource's own type arguments: the premises' proofs are any. -/
theorem StoredCase.intro {σ : NTy} {arguments : NRow}
    {holds : (θ : TypeArgs) →
      @NTy.carrier (Skolems.instantiate θ (Skolems.runtime unit)).toCarriers σ → Prop}
    {resource : ResourceType} {value : resource.carrier unit}
    (inhabitable : resource.arguments.inhabitable = true)
    (same : resource = @Skolems.resource unit
      (Skolems.instantiate ⟨resource.arguments, inhabitable⟩ (Skolems.runtime unit)) σ arguments)
    (held : holds ⟨resource.arguments, inhabitable⟩
      ((Skolems.instantiate ⟨resource.arguments, inhabitable⟩ (Skolems.runtime unit)).ofRuntime σ
        (cast (congrArg (ResourceType.carrier unit) same) value))) :
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

/-- A row's entries with each replaced. -/
theorem NRow.getD_map_params (g : NTy → NTy) :
    (params : NRow) → (start : Nat) → params.isParamsFrom start = true → (index : Nat) →
      index < params.length → (default : NTy) →
      (params.map g).getD index default = g (.param (start + index))
  | .nil, _, _, _, below, _ => absurd below (Nat.not_lt_zero _)
  | .cons τ rest, start, own, 0, _, _ => by
      simp only [NRow.isParamsFrom, Bool.and_eq_true, beq_iff_eq] at own
      simp only [NRow.map, NRow.getD, own.1, Nat.add_zero]
  | .cons _ rest, start, own, index + 1, below, default => by
      simp only [NRow.isParamsFrom, Bool.and_eq_true] at own
      simp only [NRow.map, NRow.getD]
      rw [NRow.getD_map_params g rest (start + 1) own.2 index
        (by simp only [NRow.length] at below; omega) default]
      congr 2
      omega

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
      exact agree index bounded
  | .unit, _ | .bool, _ | .int _ _, _ | .num, _ | .address, _ | .signer, _ | .string, _
  | .bytes, _ => rfl
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
  | .vector element, bounded => by
      simp only [NTy.paramsAll] at bounded
      simp only [NTy.subst, NTy.substWith, NTy.subst_eq_substWith θ f bound agree element bounded]
  | .table key value, bounded => by
      simp only [NTy.paramsAll, Bool.and_eq_true] at bounded
      simp only [NTy.subst, NTy.substWith, NTy.subst_eq_substWith θ f bound agree key bounded.1,
        NTy.subst_eq_substWith θ f bound agree value bounded.2]
  | .ghost slot, bounded => by
      simp only [NTy.paramsAll] at bounded
      simp only [NTy.subst, NTy.substWith, NTy.subst_eq_substWith θ f bound agree slot bounded]
  | .ref referent, bounded => by
      simp only [NTy.paramsAll] at bounded
      simp only [NTy.subst, NTy.substWith, NTy.subst_eq_substWith θ f bound agree referent bounded]
  | .function parameters _ results, bounded => by
      simp only [NTy.paramsAll, Bool.and_eq_true] at bounded
      simp only [NTy.subst, NTy.substWith,
        NRow.subst_eq_substWith θ f bound agree parameters bounded.1,
        NRow.subst_eq_substWith θ f bound agree results bounded.2]

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

/-- A frame's resolutions of a declaration's parameters are inhabitable. -/
theorem NRow.inhabitable_map_resolve (Θ : Skolems unit) :
    (params : NRow) → (start : Nat) → params.isParamsFrom start = true →
      (params.map Θ.resolve).inhabitable = true
  | .nil, _, _ => rfl
  | .cons _ rest, start, own => by
      simp only [NRow.isParamsFrom, Bool.and_eq_true, beq_iff_eq] at own
      simp only [NRow.map, NRow.inhabitable, own.1, Θ.type_inhabitable,
        NRow.inhabitable_map_resolve Θ rest (start + 1) own.2, Bool.and_self]

/-- The type arguments a frame gives a declaration's parameters. -/
@[reducible] def Skolems.typeArgs (Θ : Skolems unit) (params : NRow)
    (own : params.isParamsFrom 0 = true) : TypeArgs :=
  ⟨params.map Θ.resolve, NRow.inhabitable_map_resolve Θ params 0 own⟩

/-- The frame those type arguments induce over the runtime frame. -/
@[reducible] noncomputable def Skolems.instance (Θ : Skolems unit) (params : NRow)
    (own : params.isParamsFrom 0 = true) : Skolems unit :=
  Skolems.instantiate (Θ.typeArgs params own) (Skolems.runtime unit)

/-- A type over a declaration's parameters resolves alike at a frame and at
the frame of its type arguments. -/
theorem Skolems.resolve_instance (Θ : Skolems unit) {params : NRow}
    (own : params.isParamsFrom 0 = true) {τ : NTy}
    (bounded : τ.paramsAll (fun index => decide (index < params.length)) = true) :
    τ.subst (params.map Θ.resolve) = Θ.resolve τ := by
  rw [Θ.resolve_eq τ]
  refine NTy.subst_eq_substWith _ _ params.length (fun index below => ?_) τ bounded
  simpa only [Nat.zero_add] using
    NRow.getD_map_params Θ.resolve params 0 own index below (.param index)

/-- A frame's value of a type over a declaration's parameters, at the frame
of its type arguments: the same runtime value. -/
noncomputable def Skolems.reframe (Θ : Skolems unit) {params : NRow} (own : params.isParamsFrom 0 = true)
    {τ : NTy} (bounded : τ.paramsAll (fun index => decide (index < params.length)) = true)
    (value : @NTy.carrier Θ.toCarriers τ) : @NTy.carrier (Θ.instance params own).toCarriers τ :=
  (Θ.instance params own).ofRuntime τ
    (cast (congrArg (@NTy.carrier (Carriers.runtime unit)) (Θ.resolve_instance own bounded).symm)
      (Θ.toRuntime τ value))

/-- An encoding does not see a cast between the carriers of equal types. -/
theorem NTy.encode_castCarrier {Φ : Carriers} {ρ ρ' : NTy} (same : ρ = ρ')
    (carriers : @NTy.carrier Φ ρ = @NTy.carrier Φ ρ') (value : @NTy.carrier Φ ρ) :
    @NTy.encode Φ ρ' (cast carriers value) = @NTy.encode Φ ρ value := by
  subst same
  rfl

/-- Moving a value to the frame of its type arguments keeps its encoding. -/
theorem Skolems.encode_reframe (Θ : Skolems unit) {params : NRow} (own : params.isParamsFrom 0 = true) {τ : NTy} (bounded : τ.paramsAll (fun index => decide (index < params.length)) = true)
    (value : @NTy.carrier Θ.toCarriers τ) :
    @NTy.encode (Θ.instance params own).toCarriers τ (Θ.reframe own bounded value) =
      @NTy.encode Θ.toCarriers τ value := by
  unfold Skolems.reframe
  rw [← (Θ.instance params own).encode_toRuntime, (Θ.instance params own).toRuntime_ofRuntime]
  exact (NTy.encode_castCarrier (Θ.resolve_instance own bounded).symm _ _).trans
    (Θ.encode_toRuntime τ value)

/-- Moving values to the frame of their type arguments is injective. -/
@[vir_denote_norm] theorem Skolems.reframe_inj (Θ : Skolems unit) {params : NRow} (own : params.isParamsFrom 0 = true) {τ : NTy}
    (bounded : τ.paramsAll (fun index => decide (index < params.length)) = true) (left right : @NTy.carrier Θ.toCarriers τ) :
    Θ.reframe own bounded left = Θ.reframe own bounded right ↔ left = right := by
  constructor
  · intro same
    apply @NTy.encode_injective Θ.toCarriers τ
    rw [← Θ.encode_reframe own bounded left, ← Θ.encode_reframe own bounded right, same]
  · intro same
    rw [same]

/-- Equality without handle identity does not see the frame (`NTy.equal_codec`). -/
@[vir_denote_norm] theorem Skolems.equal_reframe (Θ : Skolems unit) {params : NRow} (own : params.isParamsFrom 0 = true)
    {τ : NTy} (bounded : τ.paramsAll (fun index => decide (index < params.length)) = true) (left right : @NTy.carrier Θ.toCarriers τ) :
    @NTy.equal (Θ.instance params own).toCarriers false τ (Θ.reframe own bounded left)
        (Θ.reframe own bounded right) = @NTy.equal Θ.toCarriers false τ left right := by
  rw [@NTy.equal_codec (Θ.instance params own).toCarriers, @NTy.equal_codec Θ.toCarriers]
  simp only [NTy.codec_encode, Skolems.encode_reframe]
  rfl

/-- Nor does the order (`NTy.order_codec`). -/
@[vir_denote_norm] theorem Skolems.order_reframe (Θ : Skolems unit) {params : NRow} (own : params.isParamsFrom 0 = true)
    {τ : NTy} (bounded : τ.paramsAll (fun index => decide (index < params.length)) = true) (rank : ValueRanks) (left right : @NTy.carrier Θ.toCarriers τ) :
    @NTy.order (Θ.instance params own).toCarriers rank τ (Θ.reframe own bounded left)
        (Θ.reframe own bounded right) = @NTy.order Θ.toCarriers rank τ left right := by
  rw [← @NTy.order_codec (Θ.instance params own).toCarriers, ← @NTy.order_codec Θ.toCarriers]
  simp only [NTy.codec_encode, Skolems.encode_reframe]

section Reframe
variable (Θ : Skolems unit) {params : NRow} (own : params.isParamsFrom 0 = true)

/-- A scalar is the same value at every frame. -/
private theorem Skolems.reframe_scalar {τ : NTy} (bounded) (value : @NTy.carrier Θ.toCarriers τ)
    (carriers : @NTy.carrier Θ.toCarriers τ = @NTy.carrier (Θ.instance params own).toCarriers τ)
    (encoded : ∀ value, @NTy.encode (Θ.instance params own).toCarriers τ (cast carriers value) =
      @NTy.encode Θ.toCarriers τ value) :
    Θ.reframe own bounded value = cast carriers value := by
  apply @NTy.encode_injective (Θ.instance params own).toCarriers τ
  rw [Θ.encode_reframe, encoded]

@[vir_denote_norm] theorem Skolems.reframe_unit (bounded) (value : Unit) :
    Θ.reframe own (τ := .unit) bounded value = value :=
  Θ.reframe_scalar own bounded value rfl fun _ => rfl
@[vir_denote_norm] theorem Skolems.reframe_bool (bounded) (value : Bool) :
    Θ.reframe own (τ := .bool) bounded value = value :=
  Θ.reframe_scalar own bounded value rfl fun _ => rfl
@[vir_denote_norm] theorem Skolems.reframe_int {width : Nat} {signed : Bool} (bounded)
    (value : SpecInt (.bits width) signed) :
    Θ.reframe own (τ := .int width signed) bounded value = value :=
  Θ.reframe_scalar own bounded value rfl fun _ => rfl
@[vir_denote_norm] theorem Skolems.reframe_num (bounded) (value : SpecInt .unbounded true) :
    Θ.reframe own (τ := .num) bounded value = value :=
  Θ.reframe_scalar own bounded value rfl fun _ => rfl
@[vir_denote_norm] theorem Skolems.reframe_address (bounded) (value : String) :
    Θ.reframe own (τ := .address) bounded value = value :=
  Θ.reframe_scalar own bounded value rfl fun _ => rfl
@[vir_denote_norm] theorem Skolems.reframe_signer (bounded) (value : String) :
    Θ.reframe own (τ := .signer) bounded value = value :=
  Θ.reframe_scalar own bounded value rfl fun _ => rfl
@[vir_denote_norm] theorem Skolems.reframe_string (bounded) (value : String) :
    Θ.reframe own (τ := .string) bounded value = value :=
  Θ.reframe_scalar own bounded value rfl fun _ => rfl
@[vir_denote_norm] theorem Skolems.reframe_bytes (bounded) (value : Array UInt8) :
    Θ.reframe own (τ := .bytes) bounded value = value :=
  Θ.reframe_scalar own bounded value rfl fun _ => rfl

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

/-- The encodings of a struct value's fields, from the value's. -/
private theorem encode_fields {Φ Ψ : Carriers} {source : StructHandle} {arguments : NRow}
    {τ : NTy} {rest : NRow} (left : @NTy.carrier Φ (.struct source arguments (.cons τ rest)))
    (right : @NTy.carrier Ψ (.struct source arguments (.cons τ rest)))
    (same : @NTy.encode Φ (.struct source arguments (.cons τ rest)) left =
      @NTy.encode Ψ (.struct source arguments (.cons τ rest)) right) :
    @NTy.encode Φ τ left.1 = @NTy.encode Ψ τ right.1 ∧
      @NTy.encode Φ (.struct source arguments rest) left.2 =
        @NTy.encode Ψ (.struct source arguments rest) right.2 := by
  rw [@NTy.encode_struct Φ, @NTy.encode_struct Ψ, @HList.encode_cons Φ, @HList.encode_cons Ψ] at same
  simp only [RuntimeValue.nominal.injEq, true_and] at same
  have fields := List.toArray_inj same
  simp only [List.cons.injEq] at fields
  refine ⟨fields.1, ?_⟩
  rw [@NTy.encode_struct Φ, @NTy.encode_struct Ψ, fields.2]

/-- A struct value's fields move with it. -/
@[vir_denote_norm] theorem Skolems.reframe_struct_fst {source : StructHandle} {arguments : NRow}
    {τ : NTy} {rest : NRow} (bounded)
    (value : @NTy.carrier Θ.toCarriers (.struct source arguments (.cons τ rest))) :
    (Θ.reframe own bounded value).1 =
      Θ.reframe own (paramsAll_head (paramsAll_fields bounded)) value.1 := by
  apply @NTy.encode_injective (Θ.instance params own).toCarriers τ
  rw [(encode_fields _ _ (Θ.encode_reframe own bounded value)).1, Θ.encode_reframe]

@[vir_denote_norm] theorem Skolems.reframe_struct_snd {source : StructHandle} {arguments : NRow}
    {τ : NTy} {rest : NRow} (bounded)
    (value : @NTy.carrier Θ.toCarriers (.struct source arguments (.cons τ rest))) :
    (Θ.reframe own bounded value).2 =
      Θ.reframe own (τ := .struct source arguments rest)
        (paramsAll_struct (by simp only [NTy.paramsAll, Bool.and_eq_true] at bounded; exact bounded.1)
          (paramsAll_tail (paramsAll_fields bounded))) value.2 := by
  apply @NTy.encode_injective (Θ.instance params own).toCarriers (.struct source arguments rest)
  rw [(encode_fields _ _ (Θ.encode_reframe own bounded value)).2, Θ.encode_reframe]

/-- A vector's elements move with it. -/
@[vir_denote_norm] theorem Skolems.reframe_vector_values {element : NTy} (bounded)
    (vector : @NTy.carrier Θ.toCarriers (.vector element)) :
    (Θ.reframe own (τ := .vector element) bounded vector).values =
      vector.values.map (Θ.reframe own (τ := element) (by simpa only [NTy.paramsAll] using bounded)) := by
  have encoded := Θ.encode_reframe own bounded vector
  rw [@NTy.encode_vector (Θ.instance params own).toCarriers, @NTy.encode_vector Θ.toCarriers] at encoded
  simp only [RuntimeValue.vector.injEq] at encoded
  apply Array.map_inj_right (f := @NTy.encode (Θ.instance params own).toCarriers element)
    (fun _ _ same => @NTy.encode_injective (Θ.instance params own).toCarriers element _ _ same) |>.mp
  rw [Array.map_map]
  refine encoded.trans ?_
  apply Array.map_congr_left
  intro value _
  exact (Θ.encode_reframe own _ value).symm

/-- A ghost slot moves with its value. -/
@[vir_denote_norm] theorem Skolems.reframe_ghost {slot : NTy} (bounded)
    (value : @NTy.carrier Θ.toCarriers (.ghost slot)) :
    Θ.reframe own (τ := .ghost slot) bounded value =
      Θ.reframe own (τ := slot) (by simpa only [NTy.paramsAll] using bounded) value := by
  apply @NTy.encode_injective (Θ.instance params own).toCarriers (.ghost slot)
  rw [Θ.encode_reframe, @NTy.encode_ghost (Θ.instance params own).toCarriers,
    Θ.encode_reframe, @NTy.encode_ghost Θ.toCarriers]

end Reframe

/-- A row over a declaration's parameters resolves alike at a frame and at
the frame of its type arguments. -/
theorem Skolems.map_resolve_instance (Θ : Skolems unit) {params : NRow}
    (own : params.isParamsFrom 0 = true) :
    (row : NRow) → row.paramsAll (fun index => decide (index < params.length)) = true →
      row.map (Θ.instance params own).resolve = row.map Θ.resolve
  | .nil, _ => rfl
  | .cons τ rest, bounded => by
      simp only [NRow.paramsAll, Bool.and_eq_true] at bounded
      show NRow.cons (τ.subst (params.map Θ.resolve)) (rest.map (Θ.instance params own).resolve) =
        .cons (Θ.resolve τ) (rest.map Θ.resolve)
      rw [Θ.map_resolve_instance own rest bounded.2, Θ.resolve_instance own bounded.1]

/-- A declaration's invariant at a frame: of the value the frame reads at its
resource, the invariant at the resource's type arguments of the value moved
to the frame they induce (`Skolems.reframe`). -/
theorem StoredCase.atFrame (Θ : Skolems unit) {σ : NTy} {params : NRow}
    {holds : (θ : TypeArgs) →
      @NTy.carrier (Skolems.instantiate θ (Skolems.runtime unit)).toCarriers σ → Prop}
    (own : params.isParamsFrom 0 = true)
    (bounded : σ.paramsAll (fun index => decide (index < params.length)) = true)
    (value : @NTy.carrier (Carriers.runtime unit) (Θ.resolve σ)) :
    StoredCase σ params holds (@Skolems.resource unit Θ σ params) value ↔
      holds (Θ.typeArgs params own) (Θ.reframe own bounded (Θ.ofRuntime σ value)) := by
  have same : @Skolems.resource unit Θ σ params =
      @Skolems.resource unit (Θ.instance params own) σ params := by
    show ResourceType.mk (Θ.resolve σ) (params.map Θ.resolve) .value =
      ResourceType.mk (σ.subst (params.map Θ.resolve)) (params.map (Θ.instance params own).resolve) .value
    rw [Θ.map_resolve_instance own params (NRow.paramsAll_params params 0 params.length own (by omega)),
      Θ.resolve_instance own bounded]
  constructor
  · intro stored
    have held := stored (Θ.typeArgs params own).2 same
    unfold Skolems.reframe
    rw [Θ.toRuntime_ofRuntime]
    exact held
  · intro held _ _
    unfold Skolems.reframe at held
    rw [Θ.toRuntime_ofRuntime] at held
    exact held

mutual
/-- A substitution by inhabitable types keeps a type inhabitable. -/
theorem NTy.inhabitable_subst (θ : NRow) (fits : θ.inhabitable = true) :
    (τ : NTy) → τ.inhabitable = true → (τ.subst θ).inhabitable = true
  | .tuple elements, held => by
      simp only [NTy.subst, NTy.inhabitable] at held ⊢
      exact NRow.inhabitable_subst θ fits elements held
  | .struct _ _ fields, held => by
      simp only [NTy.subst, NTy.inhabitable] at held ⊢
      exact NRow.inhabitable_subst θ fits fields held
  | .enum _ _ names rows _, held => by
      simp only [NTy.subst, NTy.inhabitable] at held ⊢
      exact NRows.firstInhabitable_subst θ fits names rows held
  | .ref referent, held => by
      simp only [NTy.subst, NTy.inhabitable] at held ⊢
      exact NTy.inhabitable_subst θ fits referent held
  | .ghost slot, held => by
      simp only [NTy.subst, NTy.inhabitable] at held ⊢
      exact NTy.inhabitable_subst θ fits slot held
  | .param index, _ => by
      simp only [NTy.subst]
      exact NRow.getD_inhabitable θ fits index (.param index) rfl
  | .function _ _ _, held => by simp [NTy.inhabitable] at held
  | .vector _, _ => rfl
  | .table _ _, _ => rfl
  | .unit, _ => rfl
  | .bool, _ => rfl
  | .int _ _, held => held
  | .num, _ => rfl
  | .address, _ => rfl
  | .signer, _ => rfl
  | .string, _ => rfl
  | .bytes, _ => rfl

theorem NRow.inhabitable_subst (θ : NRow) (fits : θ.inhabitable = true) :
    (row : NRow) → row.inhabitable = true → (NRow.subst θ row).inhabitable = true
  | .nil, _ => rfl
  | .cons τ rest, held => by
      simp only [NRow.subst, NRow.inhabitable, Bool.and_eq_true] at held ⊢
      exact ⟨NTy.inhabitable_subst θ fits τ held.1, NRow.inhabitable_subst θ fits rest held.2⟩

theorem NRows.firstInhabitable_subst (θ : NRow) (fits : θ.inhabitable = true) :
    (names : List String) → (rows : NRows) → NRows.firstInhabitable names rows = true →
      NRows.firstInhabitable names (rows.subst θ) = true
  | _ :: _, .cons fields _, held => by
      simp only [NRows.subst, NRows.firstInhabitable] at held ⊢
      exact NRow.inhabitable_subst θ fits fields held
  | [], _, held => by simp [NRows.firstInhabitable] at held
  | _ :: _, .nil, held => by simp [NRows.firstInhabitable] at held
end

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

end VerexIR.Proofs.Denote
