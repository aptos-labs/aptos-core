-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.Denote.TableMemory

/-!
# Values observed by specifications

Logical values retain Table contents at each occurrence. They are a separate
type from executable values: a functional update of a Table snapshot cannot be
passed to execution merely by discarding its contents and keeping its handle.
Observation takes an explicit memory, including for old and labeled states.

Contract translation uses this carrier for Table specification observations.
Native call adapters must separately establish agreement with execution memory.
-/

namespace LeanerIR.Proofs.Denote.SnapshotValue

/-- The shape of an aggregate, independent of its observed children. -/
inductive Shape where
  | vector
  | tuple
  | nominal (source : StructHandle) (variant : Option String)
  deriving DecidableEq

/-- A Table node holds its physical handle/metadata and an independently
observed contents tree. Missing storage remains explicit rather than becoming
an empty allocated Table. Entries may themselves contain Table snapshots. -/
inductive Value where
  | scalar (value : RuntimeValue)
  | aggregate (shape : Shape) (fields : List Value)
  | table (physical : RuntimeValue) (contents : Option (List (RuntimeValue × Value)))
  deriving Inhabited

namespace Value

/-- Recover only the physical representation. This is an observation theorem's
projection, not a conversion licensing execution of hypothetical snapshots. -/
def physical : Value → RuntimeValue
  | .scalar value => value
  | .aggregate shape fields =>
      let fields := (fields.map physical).toArray
      match shape with
      | .vector => .vector fields
      | .tuple => .tuple fields
      | .nominal source variant => .nominal source variant fields
  | .table value _ => value

/-- Equality's observation: Table nodes compare allocation identity, including
when nested in aggregates. Content snapshots and cached lengths are not part
of a Table's identity. This is deliberately distinct from Lean equality. -/
def identity : Value → RuntimeValue
  | .scalar value => value
  | .aggregate shape fields =>
      let fields := (fields.map identity).toArray
      match shape with
      | .vector => .vector fields
      | .tuple => .tuple fields
      | .nominal source variant => .nominal source variant fields
  | .table (.nominal source variant fields) _ =>
      .nominal source variant #[fields[0]?.getD .unit]
  | .table value _ => value

def SameIdentity (left right : Value) : Prop := left.identity = right.identity

@[simp, grind ←] theorem sameIdentity_refl (value : Value) : SameIdentity value value := rfl
theorem sameIdentity_symm {left right : Value} :
    SameIdentity left right → SameIdentity right left := Eq.symm
theorem sameIdentity_trans {left middle right : Value} :
    SameIdentity left middle → SameIdentity middle right → SameIdentity left right := Eq.trans

/-- Projection retains the selected child's own snapshot. -/
def field (value : Value) (index : Nat) : Value :=
  match value with
  | .aggregate _ fields => fields[index]?.getD (.scalar .unit)
  | .table physical _ | .scalar physical => .scalar (physical.field index)

/-- Logical vectors retain each element's independently selected observation. -/
def elements : Value → List Value
  | .aggregate .vector values => values
  | .scalar (.vector values) => values.toList.map .scalar
  | _ => []

def vectorLength (value : Value) : Int := value.elements.length
def vectorContains (value element : Value) : Prop :=
  ∃ member ∈ value.elements, SameIdentity member element
def vectorUpdate (value : Value) (index : Nat) (replacement : Value) : Value :=
  .aggregate .vector (value.elements.set index replacement)
def vectorPush (value replacement : Value) : Value :=
  .aggregate .vector (value.elements ++ [replacement])
def vectorConcat (left right : Value) : Value :=
  .aggregate .vector (left.elements ++ right.elements)
def vectorSlice (value : Value) (lower upper : Nat) : Value :=
  .aggregate .vector ((value.elements.drop lower).take (upper - lower))

def testVariants (value : Value) (source : StructHandle) (variants : Array String) : Bool :=
  match value with
  | .aggregate (.nominal actual (some variant)) _ => actual == source && variants.contains variant
  | .scalar (.nominal actual (some variant) _) => actual == source && variants.contains variant
  | _ => false

def selectVariantField (value : Value) (source : StructHandle)
    (choices : Array (String × Nat)) : Value :=
  match value with
  | .aggregate (.nominal actual (some variant)) _
  | .scalar (.nominal actual (some variant) _) =>
      if actual == source then
        ((choices.find? (·.1 == variant)).map fun choice => value.field choice.2).getD (.scalar .unit)
      else .scalar .unit
  | _ => .scalar .unit

def updateNominalField (value : Value) (source : StructHandle)
    (choices : List (Option String × Nat)) (replacement : Value) : Value :=
  match value with
  | .aggregate (.nominal actual variant) fields =>
      if actual == source then
        ((choices.find? (·.1 == variant)).map fun choice =>
          Value.aggregate (.nominal actual variant) (fields.set choice.2 replacement)).getD (.scalar .unit)
      else .scalar .unit
  | .scalar (.nominal actual variant fields) =>
      if actual == source then
        ((choices.find? (·.1 == variant)).map fun choice =>
          Value.aggregate (.nominal actual variant)
            ((fields.toList.map .scalar).set choice.2 replacement)).getD (.scalar .unit)
      else .scalar .unit
  | _ => .scalar .unit

/-- A Table lookup returns the stored logical child, not a handle interpreted
again at the surrounding expression's memory. -/
def lookup? (value : Value) (key : RuntimeValue) : Option Value := do
  let .table _ (some entries) := value | none
  let entry ← entries.find? (·.1 == key)
  some entry.2

def size : Value → Int
  | .table _ (some entries) => entries.length
  | _ => 0

def hasKey (value : Value) (key : RuntimeValue) : Bool := (value.lookup? key).isSome

def hasContents : Value → Bool
  | .table _ (some _) => true
  | _ => false

@[grind →] theorem hasContents_of_hasKey (value : Value) (key : RuntimeValue)
    (present : value.hasKey key = true) : value.hasContents = true := by
  cases value <;> simp only [hasKey, lookup?] at present <;> try contradiction
  case table physical contents =>
    cases contents with
    | none => contradiction
    | some _ => rfl

/-- The first blocked Table registry fixture needs this implication on both
old and current observations; it is a property of each snapshot independently. -/
@[grind →] theorem size_pos_of_hasKey (value : Value) (key : RuntimeValue)
    (present : value.hasKey key = true) : 0 < value.size := by
  cases value <;> simp only [hasKey, lookup?] at present <;> try contradiction
  case table physical contents =>
    cases contents with
    | none => contradiction
    | some entries =>
        cases entries with
        | nil => simp at present
        | cons entry rest => simp [size]

/-- Functional replacement, preserving the entire logical value at the
selected occurrence. Other occurrences of the same Table identity do not change. -/
def setEntries : List (RuntimeValue × Value) → RuntimeValue → Value → List (RuntimeValue × Value)
  | [], key, value => [(key, value)]
  | entry :: rest, key, value =>
      if entry.1 == key then (key, value) :: rest else entry :: setEntries rest key value

/-- A cached length, when present, describes the new hypothetical contents.
Handle-only Tables have no field to change. -/
def refreshLength (physical : RuntimeValue) (length : Nat) : RuntimeValue :=
  match physical with
  | .nominal source none fields =>
      match fields.toList with
      | [handle, .integer _] => .nominal source none #[handle, .integer length]
      | _ => physical
  | _ => physical

/-- A total specification adapter can choose its junk result for `none`.
There is no allocation or implicit read of the current memory here. -/
def set? (table : Value) (key : RuntimeValue) (replacement : Value) : Option Value := do
  let .table physical (some entries) := table | none
  let updated := setEntries entries key replacement
  some (.table (refreshLength physical updated.length) (some updated))

def remove? (table : Value) (key : RuntimeValue) : Option Value := do
  let .table physical (some entries) := table | none
  let updated := entries.filter (fun entry => !(entry.1 == key))
  some (.table (refreshLength physical updated.length) (some updated))

def getValue (table : Value) (key : RuntimeValue) : Value :=
  (table.lookup? key).getD (.scalar .unit)
def setValue (table : Value) (key : RuntimeValue) (replacement : Value) : Value :=
  (table.set? key replacement).getD (.scalar .unit)
def removeValue (table : Value) (key : RuntimeValue) : Value :=
  (table.remove? key).getD (.scalar .unit)

theorem find_setEntries (entries : List (RuntimeValue × Value)) (key : RuntimeValue)
    (replacement : Value) :
    (setEntries entries key replacement).find? (·.1 == key) = some (key, replacement) := by
  induction entries with
  | nil => simp [setEntries]
  | cons entry rest ih =>
      cases matched : entry.1 == key <;> simp [setEntries, matched, ih]

theorem find_setEntries_other (entries : List (RuntimeValue × Value))
    (key probe : RuntimeValue) (replacement : Value) (distinct : probe ≠ key) :
    (setEntries entries key replacement).find? (·.1 == probe) = entries.find? (·.1 == probe) := by
  have different : (key == probe) = false := beq_eq_false_iff_ne.mpr distinct.symm
  induction entries with
  | nil => simp [setEntries, different]
  | cons entry rest ih =>
      cases changed : entry.1 == key <;> cases queried : entry.1 == probe <;>
        simp_all [setEntries]

theorem lookup_set (physical : RuntimeValue) (entries : List (RuntimeValue × Value))
    (key : RuntimeValue) (replacement : Value) :
    (set? (.table physical (some entries)) key replacement).bind (·.lookup? key) = some replacement := by
  simp [set?, lookup?, find_setEntries]

theorem lookup_set_other (physical : RuntimeValue) (entries : List (RuntimeValue × Value))
    (key probe : RuntimeValue) (replacement : Value) (distinct : probe ≠ key) :
    (set? (.table physical (some entries)) key replacement).bind (·.lookup? probe) =
      (Value.table physical (some entries)).lookup? probe := by
  simp [set?, lookup?, find_setEntries_other entries key probe replacement distinct]

@[grind =] theorem get_setValue (table : Value) (key : RuntimeValue) (replacement : Value)
    (allocated : table.hasContents = true) :
    (table.setValue key replacement).getValue key = replacement := by
  cases table <;> simp only [hasContents] at allocated <;> try contradiction
  case table physical contents =>
    cases contents with
    | none => contradiction
    | some entries => simp [getValue, setValue, set?, lookup?, find_setEntries]

theorem lookup_remove (physical : RuntimeValue) (entries : List (RuntimeValue × Value))
    (key : RuntimeValue) :
    (remove? (.table physical (some entries)) key).bind (·.lookup? key) = none := by
  simp only [remove?, Option.bind_eq_bind, Option.bind_some, lookup?]
  have absent : (entries.filter (fun entry => !(entry.1 == key))).find? (·.1 == key) = none := by
    apply List.find?_eq_none.mpr
    intro entry member
    simpa using (List.mem_filter.mp member).2
  simp [absent]

@[simp] theorem physical_table (physical : RuntimeValue)
    (contents : Option (List (RuntimeValue × Value))) :
    (Value.table physical contents).physical = physical := by simp [Value.physical]

@[simp, grind =] theorem physical_scalar (value : RuntimeValue) :
    (Value.scalar value).physical = value := by simp [Value.physical]

@[simp, grind =] theorem identity_scalar (value : RuntimeValue) :
    (Value.scalar value).identity = value := by simp [Value.identity]

@[simp] theorem field_aggregate (shape : Shape) (fields : List Value) (index : Nat) :
    (Value.aggregate shape fields).field index = fields[index]?.getD (.scalar .unit) := rfl

theorem sameIdentity_table (physical : RuntimeValue)
    (before after : Option (List (RuntimeValue × Value))) :
    SameIdentity (.table physical before) (.table physical after) := by
  cases physical <;> simp [SameIdentity, identity]

theorem identity_refreshLength (physical : RuntimeValue) (length : Nat)
    (before after : Option (List (RuntimeValue × Value))) :
    (Value.table (refreshLength physical length) after).identity =
      (Value.table physical before).identity := by
  cases physical <;> try (simp [refreshLength, identity]; done)
  case nominal source variant fields =>
    cases variant with
    | some name => simp [refreshLength, identity]
    | none =>
        simp only [refreshLength]
        split
        · rename_i handle cached row
          have equal : fields = #[handle, .integer cached] := Array.toList_inj.mp row
          subst fields
          simp [identity]
        · simp [identity]

theorem sameIdentity_set {original updated : Value} {key : RuntimeValue} {replacement : Value}
    (changed : original.set? key replacement = some updated) : SameIdentity original updated := by
  cases original <;> simp only [set?] at changed <;> try contradiction
  case table physical contents =>
    cases contents with
    | none => contradiction
    | some entries =>
        simp only [Option.some.injEq] at changed
        subst updated
        exact (identity_refreshLength physical _ _ _).symm

theorem sameIdentity_remove {original updated : Value} {key : RuntimeValue}
    (changed : original.remove? key = some updated) : SameIdentity original updated := by
  cases original <;> simp only [remove?] at changed <;> try contradiction
  case table physical contents =>
    cases contents with
    | none => contradiction
    | some entries =>
        simp only [Option.some.injEq] at changed
        subst updated
        exact (identity_refreshLength physical _ _ _).symm

@[grind ←] theorem sameIdentity_setValue (table : Value) (key : RuntimeValue)
    (replacement : Value) (allocated : table.hasContents = true) :
    SameIdentity table (table.setValue key replacement) := by
  cases table <;> simp only [hasContents] at allocated <;> try contradiction
  case table physical contents =>
    cases contents with
    | none => contradiction
    | some entries =>
        apply sameIdentity_set
        rfl

end Value

/-- Native Table ownership is declared by the intrinsic registry. Shape checks
are separate, so unrelated structs with an address field do not become Tables. -/
def tableOwner (unit : Validation.ValidatedUnit) (source : StructHandle) : Bool :=
  ((unit.namespaces[source.namespaceId.index]?).bind fun ns =>
    (ns.structs[source.structId]?).map fun declaration =>
      ns.intrinsics.any fun intrinsic =>
        intrinsic.model == "map" && intrinsic.owner == declaration.name).getD false

variable {unit : Validation.ValidatedUnit}

local notation "runtimeEncode" => @NTy.encode (Carriers.runtime unit)
local notation "runtimeCarrier" => @NTy.carrier (Carriers.runtime unit)
local notation "runtimeRow" => @HList (Carriers.runtime unit)
local notation "runtimeVariant" => @variantCarrier (Carriers.runtime unit)

mutual
/-- Observe typed values at one memory. Recursion follows the finite native
type, including the value type in a Table's phantom arguments. Opaque generic
parameters retain their runtime encoding; a caller must instantiate the native
type before asking to observe Tables hidden behind such a parameter. -/
noncomputable def observe (memory : Memory unit) : (type : NTy) → runtimeCarrier type → Value
  | .tuple elements, value => .aggregate .tuple (observeRow memory elements value)
  | .struct source arguments fields, value =>
      match arguments with
      | .cons key (.cons element .nil) =>
          if tableOwner unit source && TableMemory.handleFields fields then
            let physical := runtimeEncode (.struct source arguments fields) value
            let contents := TableMemory.read memory (.struct source arguments fields) key element
              (physical.field 0).asString
            .table physical (contents.map fun entries => entries.toList.map fun entry =>
              (runtimeEncode key entry.1, observe memory element entry.2.1))
          else .aggregate (.nominal source none) (observeRow memory fields value)
      | _ => .aggregate (.nominal source none) (observeRow memory fields value)
  | .enum source _ names rows _, value => observeVariant memory source names rows value
  | .vector element, value =>
      .aggregate .vector (value.values.toList.map (observe memory element))
  | .ref referent, value =>
      .aggregate .tuple [observe memory referent value.1, observe memory referent value.2]
  | .unit, value => .scalar (runtimeEncode .unit value)
  | .bool, value => .scalar (runtimeEncode .bool value)
  | .int width signed, value => .scalar (runtimeEncode (.int width signed) value)
  | .address, value => .scalar (runtimeEncode .address value)
  | .signer, value => .scalar (runtimeEncode .signer value)
  | .string, value => .scalar (runtimeEncode .string value)
  | .bytes, value => .scalar (runtimeEncode .bytes value)
  | .param index, value => .scalar (runtimeEncode (.param index) value)
  | .function parameters shared results, value =>
      .scalar (runtimeEncode (.function parameters shared results) value)

noncomputable def observeRow (memory : Memory unit) : (row : NRow) → runtimeRow row → List Value
  | .nil, _ => []
  | .cons type rest, (value, values) => observe memory type value :: observeRow memory rest values

noncomputable def observeVariant (memory : Memory unit) (source : StructHandle) :
    (names : List String) → (rows : NRows) → runtimeVariant names rows → Value
  | _, .nil, value => nomatch value
  | [], .cons _ _, value => nomatch value
  | name :: _, .cons fields _, .inl value =>
      .aggregate (.nominal source (some name)) (observeRow memory fields value)
  | _ :: names, .cons _ rest, .inr value => observeVariant memory source names rest value
end

theorem observe_table (memory : Memory unit) (source : StructHandle) (key element : NTy)
    (fields : NRow) (value : runtimeRow fields)
    (registered : tableOwner unit source = true) (layout : TableMemory.handleFields fields = true) :
    observe memory (.struct source (.cons key (.cons element .nil)) fields) value =
      let owner := NTy.struct source (.cons key (.cons element .nil)) fields
      let physical := runtimeEncode owner value
      .table physical ((TableMemory.read memory owner key element (physical.field 0).asString).map
        fun entries => entries.toList.map fun entry =>
          (runtimeEncode key entry.1, observe memory element entry.2.1)) := by
  simp [observe, registered, layout]

mutual
/-- Observing storage enriches values without changing their physical fields.
This holds even for missing Table slots; existence is a separate invariant. -/
theorem physical_observe (memory : Memory unit) : (type : NTy) →
    (value : runtimeCarrier type) → (observe memory type value).physical = runtimeEncode type value
  | .tuple elements, value => by
      simp [observe, Value.physical, physical_observeRow]
  | .struct source arguments fields, value => by
      cases arguments with
      | nil => simp [observe, Value.physical, physical_observeRow]
      | cons key rest =>
          cases rest with
          | nil => simp [observe, Value.physical, physical_observeRow]
          | cons element rest =>
              cases rest with
              | nil =>
                  simp only [observe]
                  split <;> simp [Value.physical, physical_observeRow]
              | cons _ _ => simp [observe, Value.physical, physical_observeRow]
  | .enum source arguments names rows distinct, value =>
      physical_observeVariant memory source names rows value
  | .vector element, value => by
      simp [observe, Value.physical, List.map_map, Function.comp_def, physical_observe]
      apply Array.toList_inj.mp
      simp only [Array.toList_map]
      rfl
  | .ref referent, value => by
      simp [observe, Value.physical, physical_observe]
  | .unit, value | .bool, value | .int _ _, value | .address, value | .signer, value
  | .string, value | .bytes, value | .param _, value | .function .., value => by
      simp [observe]

theorem physical_observeRow (memory : Memory unit) : (row : NRow) → (values : runtimeRow row) →
    (observeRow memory row values).map Value.physical = @HList.encode (Carriers.runtime unit) row values
  | .nil, _ => by simp [observeRow]
  | .cons type rest, (value, values) => by
      simp [observeRow, physical_observe, physical_observeRow]

theorem physical_observeVariant (memory : Memory unit) (source : StructHandle) :
    (names : List String) → (rows : NRows) → (value : runtimeVariant names rows) →
    (observeVariant memory source names rows value).physical =
      @variantEncode (Carriers.runtime unit) source names rows
        (@rowCodecs (Carriers.runtime unit) rows) value
  | _, .nil, value => nomatch value
  | [], .cons _ _, value => nomatch value
  | name :: _, .cons fields _, .inl value => by
      simp [observeVariant, Value.physical, physical_observeRow, variantEncode, rowCodecs,
        Codec.nominalRow, HList.encode]
  | _ :: names, .cons _ rest, .inr value => physical_observeVariant memory source names rest value
end

/-- At a fixed observation memory no distinction between typed physical
values is lost. Equality of snapshots is stronger than identity comparison. -/
theorem observe_injective (memory : Memory unit) (type : NTy)
    {left right : runtimeCarrier type} (equal : observe memory type left = observe memory type right) :
    left = right := by
  apply @NTy.encode_injective (Carriers.runtime unit) type
  simpa only [physical_observe] using congrArg Value.physical equal

/-- Specification arguments are observed after the generic frame resolves
their types. This discovers Tables even behind a callee's opaque parameter. -/
noncomputable def observeInFrame (frame : Skolems unit) (memory : Memory unit) (type : NTy)
    (value : @NTy.carrier frame.toCarriers type) : Value :=
  observe memory (frame.resolve type) (frame.toRuntime type value)

theorem physical_observeInFrame (frame : Skolems unit) (memory : Memory unit) (type : NTy)
    (value : @NTy.carrier frame.toCarriers type) :
    (observeInFrame frame memory type value).physical = @NTy.encode frame.toCarriers type value := by
  rw [observeInFrame, physical_observe, frame.encode_toRuntime]

/-- A caller and the instantiated callee observe precisely the same snapshot.
The shared memory may be any bound state label; no program point is involved. -/
theorem observeInFrame_toSkolem (frame : Skolems unit) (arguments : TypeArgs)
    (memory : Memory unit) (type : NTy)
    (value : @NTy.carrier frame.toCarriers (type.subst arguments.1)) :
    observeInFrame (Skolems.instantiate arguments frame) memory type
        (@NTy.toSkolem frame.toCarriers arguments type value) =
      observeInFrame frame memory (type.subst arguments.1) value := by
  simp only [observeInFrame, Skolems.resolve_instantiate, Skolems.toRuntime_instantiate,
    NTy.ofSkolem_toSkolem]

/-- Bridge for contract inputs already expressed through their runtime codec.
Malformed input remains absent. Logical values never pass through this bridge
again: a spec-local value already carries the observation it denotes. -/
noncomputable def observeRuntime? (frame : Skolems unit) (memory : Memory unit)
    (type : NTy) (raw : RuntimeValue) : Option Value :=
  (@NTy.codec frame.toCarriers type).decode? raw |>.map (observeInFrame frame memory type)

/-- At the runtime frame, decoding an aggregate exposes its typed observation
directly. Keep other frames opaque until instantiation transport applies; eager
unfolding there hides the caller/callee correspondence behind codec casts. -/
@[lir_denote_norm] theorem observeRuntime?_runtime (memory : Memory unit)
    (type : NTy) (raw : RuntimeValue) :
    observeRuntime? (Skolems.runtime unit) memory type raw =
      ((@NTy.codec (Carriers.runtime unit) type).decode? raw).map (observe memory type) := rfl

-- The denotation normalizer already reduces runtime codecs. Continue through
-- successful observations so a field read through a Table contract meets the
-- same field read in the caller's specification. TableMemory.read stays opaque:
-- normalization retains the selected memory instead of erasing snapshots.
attribute [lir_denote_norm] observeRow Value.field_aggregate Value.physical_scalar
  Value.identity_scalar Value.physical_table

/-- Reduce known aggregate snapshots without splitting an unknown lookup
result. Stored data invariants read these physical fields after the lookup's
postcondition has identified its returned value. -/
@[lir_denote_norm] theorem Value.physical_aggregate (shape : Shape) (values : List Value) :
    (Value.aggregate shape values).physical =
      (match shape with
       | .vector => .vector (values.map Value.physical).toArray
       | .tuple => .tuple (values.map Value.physical).toArray
       | .nominal source variant => .nominal source variant (values.map Value.physical).toArray) := by
  cases shape <;> simp only [Value.physical]

-- Do not unfold projections of unknown snapshots into matches. The call's
-- postcondition may identify the snapshot only after its guard is discharged.
@[lir_denote_norm] theorem Value.vectorLength_aggregate (values : List Value) :
    (Value.aggregate .vector values).vectorLength = (values.length : Int) := rfl

@[lir_denote_norm] theorem Value.sameIdentity_scalar (left right : RuntimeValue) :
    Value.SameIdentity (.scalar left) (.scalar right) ↔ left = right := by
  simp [Value.SameIdentity]

@[lir_denote_norm] theorem Value.identity_vector (values : List Value) :
    (Value.aggregate .vector values).identity = .vector (values.map Value.identity).toArray := by
  simp [Value.identity]

@[lir_denote_norm] theorem Value.sameIdentity_scalar_vector
    (left : RuntimeValue) (values : List Value) :
    Value.SameIdentity (.scalar left) (.aggregate .vector values) ↔
      left = .vector (values.map Value.identity).toArray := by
  simp [Value.SameIdentity, Value.identity]

/-- Observations traverse lists, whereas executable vector codecs traverse
arrays. Give both paths the same form without expanding either traversal. -/
@[lir_denote_norm] theorem toArray_map_toList {α β : Type} (values : Array α) (f : α → β) :
    (values.toList.map f).toArray = values.map f := by
  rw [← Array.toList_map, Array.toArray_toList]

@[lir_denote_norm] theorem observe_struct_nil (memory : Memory unit)
    (source : StructHandle) (fields : NRow) (value : runtimeRow fields) :
    observe memory (.struct source .nil fields) value =
      .aggregate (.nominal source none) (observeRow memory fields value) := rfl

@[lir_denote_norm] theorem observe_struct_singleton (memory : Memory unit)
    (source : StructHandle) (argument : NTy) (fields : NRow) (value : runtimeRow fields) :
    observe memory (.struct source (.cons argument .nil) fields) value =
      .aggregate (.nominal source none) (observeRow memory fields value) := rfl

@[lir_denote_norm] theorem observe_vector (memory : Memory unit)
    (element : NTy) (value : runtimeCarrier (.vector element)) :
    observe memory (.vector element) value =
      .aggregate .vector (value.values.toList.map (observe memory element)) := rfl

@[lir_denote_norm] theorem observe_int (memory : Memory unit)
    (width : Nat) (signed : Bool) (value : SpecInt (.bits width) signed) :
    observe memory (.int width signed) value = .scalar (.integer value.val) := rfl

theorem observeRuntime?_encode (frame : Skolems unit) (memory : Memory unit)
    (type : NTy) (value : @NTy.carrier frame.toCarriers type) :
    observeRuntime? frame memory type (@NTy.encode frame.toCarriers type value) =
      some (observeInFrame frame memory type value) := by
  simp [observeRuntime?]

/-- Instantiating a callee's type parameters changes its carrier, but not the
observation of a runtime value at the caller's selected memory. This includes
malformed inputs, which remain absent in both views. -/
@[simp, lir_denote] theorem observeRuntime?_instantiate (frame : Skolems unit)
    (arguments : TypeArgs) (memory : Memory unit) (type : NTy) (raw : RuntimeValue) :
    observeRuntime? (Skolems.instantiate arguments frame) memory type raw =
      observeRuntime? frame memory (type.subst arguments.1) raw := by
  cases decoded : (@NTy.codec frame.toCarriers (type.subst arguments.1)).decode? raw with
  | some value =>
      have physical := @NTy.codec_tight frame.toCarriers (type.subst arguments.1) raw value decoded
      change @NTy.encode frame.toCarriers (type.subst arguments.1) value = raw at physical
      rw [← physical]
      have transported := @NTy.encode_toSkolem frame.toCarriers arguments type value
      rw [← transported, observeRuntime?_encode, observeInFrame_toSkolem,
        transported, observeRuntime?_encode]
  | none =>
      cases inner : (@NTy.codec (Skolems.instantiate arguments frame).toCarriers type).decode? raw with
      | none => simp only [observeRuntime?, decoded, inner, Option.map_none]
      | some value =>
          have physical := @NTy.codec_tight (Skolems.instantiate arguments frame).toCarriers
            type raw value inner
          change @NTy.encode (Skolems.instantiate arguments frame).toCarriers type value = raw at physical
          have outer := @NTy.encode_ofSkolem frame.toCarriers arguments type value
          rw [physical] at outer
          have decoded' := @NTy.decode?_of_encode frame.toCarriers (type.subst arguments.1)
            (NTy.ofSkolem arguments type value) raw outer
          rw [decoded] at decoded'
          contradiction

@[simp, lir_denote] theorem observeRuntime?_int (frame : Skolems unit) (memory : Memory unit)
    (width : Nat) (signed : Bool) (value : SpecInt (.bits width) signed) :
    observeRuntime? frame memory (.int width signed) (.integer value.val) =
      some (.scalar (.integer value.val)) := by
  change observeRuntime? frame memory (.int width signed)
    (@NTy.encode frame.toCarriers (.int width signed) value) = _
  rw [observeRuntime?_encode]
  have scalar (resolved : NTy) (held : @NTy.carrier (Carriers.runtime unit) resolved)
      (equal : resolved = (.int width signed)) :
      observe memory resolved held = .scalar (@NTy.encode (Carriers.runtime unit) resolved held) := by
    cases equal
    rfl
  rw [observeInFrame, scalar _ _ (Skolems.resolve_paramFree (Θ := frame) (τ := (.int width signed)) rfl),
    frame.encode_toRuntime]
  rfl

@[simp, lir_denote] theorem observeRuntime?_bool (frame : Skolems unit) (memory : Memory unit)
    (value : Bool) : observeRuntime? frame memory .bool (.bool value) = some (.scalar (.bool value)) := by
  change observeRuntime? frame memory .bool (@NTy.encode frame.toCarriers .bool value) = _
  rw [observeRuntime?_encode]
  have scalar (resolved : NTy) (held : @NTy.carrier (Carriers.runtime unit) resolved)
      (equal : resolved = .bool) :
      observe memory resolved held = .scalar (@NTy.encode (Carriers.runtime unit) resolved held) := by
    cases equal
    rfl
  rw [observeInFrame, scalar _ _ (Skolems.resolve_paramFree (Θ := frame) (τ := .bool) rfl),
    frame.encode_toRuntime]
  rfl

/-- The successful boundary conversion still denotes the very physical value
supplied to it. Content observations are additional information, not a cast. -/
theorem observeRuntime?_physical (frame : Skolems unit) (memory : Memory unit)
    (type : NTy) (raw : RuntimeValue) (view : Value)
    (observed : observeRuntime? frame memory type raw = some view) : view.physical = raw := by
  obtain ⟨value, decoded, rfl⟩ := Option.map_eq_some_iff.mp observed
  rw [physical_observeInFrame]
  exact @NTy.codec_tight frame.toCarriers type raw value decoded

/-- Physical projection commutes with field selection, including an absent
field. It never observes the external contents of a Table. -/
@[lir_denote_norm] theorem Value.physical_field (value : Value) (index : Nat) :
    (value.field index).physical = value.physical.field index := by
  cases value with
  | scalar value => simp [Value.field]
  | table value contents => simp [Value.field]
  | aggregate shape fields =>
    cases shape <;>
      simp only [Value.field, Value.physical, RuntimeValue.field, List.getElem?_toArray,
        List.getElem?_map]
    all_goals cases fields[index]? <;> simp

/-- Select a known aggregate's field before encoding its siblings. Unknown
observations keep the usual physical projection form, which also works for
generic carriers whose observation still needs decoding. -/
@[lir_denote_norm↓] theorem Value.physical_aggregate_field
    (shape : Shape) (fields : List Value) (index : Nat) :
    (Value.aggregate shape fields).physical.field index =
      (fields[index]?.getD (.scalar .unit)).physical := by
  rw [← Value.physical_field, Value.field_aggregate]

theorem observeRuntime?_resolve {unit : Validation.ValidatedUnit}
    (frame : Skolems unit) (memory : Memory unit) (type : NTy) (raw : RuntimeValue) :
    observeRuntime? frame memory type raw =
      ((@NTy.codec (Carriers.runtime unit) (frame.resolve type)).decode? raw).map
        (observe memory (frame.resolve type)) := by
  cases decoded : (@NTy.codec (Carriers.runtime unit) (frame.resolve type)).decode? raw with
  | some value =>
      have physical := @NTy.codec_tight (Carriers.runtime unit) (frame.resolve type) raw value decoded
      change @NTy.encode (Carriers.runtime unit) (frame.resolve type) value = raw at physical
      have transported := frame.encode_toRuntime type (frame.ofRuntime type value)
      simp only [frame.toRuntime_ofRuntime] at transported
      calc
        observeRuntime? frame memory type raw =
            some (observeInFrame frame memory type (frame.ofRuntime type value)) := by
          rw [← physical, transported, observeRuntime?_encode]
        _ = Option.map (observe memory (frame.resolve type)) (some value) := by
          simp only [Option.map_some, observeInFrame, frame.toRuntime_ofRuntime]
  | none =>
      cases inner : (@NTy.codec frame.toCarriers type).decode? raw with
      | none => simp only [observeRuntime?, inner, Option.map_none]
      | some value =>
          have physical := @NTy.codec_tight frame.toCarriers type raw value inner
          change @NTy.encode frame.toCarriers type value = raw at physical
          have transported := frame.encode_toRuntime type value
          rw [physical] at transported
          have decoded' := @NTy.decode?_of_encode (Carriers.runtime unit) (frame.resolve type)
            (frame.toRuntime type value) raw transported
          rw [decoded] at decoded'
          contradiction

/-- Decode the physical single-handle layout without expanding its codec. -/
@[lir_denote_norm] theorem observeRuntime?_handle {unit : Validation.ValidatedUnit}
    (frame : Skolems unit) (memory : Memory unit) (source : StructHandle)
    (arguments : NRow) (handle : String) :
    observeRuntime? frame memory (.struct source arguments (.cons .address .nil))
        (.nominal source none #[.address handle]) =
      some (observe memory (.struct source (NRow.substWith frame.type arguments)
        (.cons .address .nil)) (handle, ())) := by
  rw [observeRuntime?_resolve, frame.resolve_eq]
  change Option.map _ ((@NTy.codec (Carriers.runtime unit)
    (.struct source (NRow.substWith frame.type arguments) (.cons .address .nil))).decode?
      (@NTy.encode (Carriers.runtime unit)
        (.struct source (NRow.substWith frame.type arguments) (.cons .address .nil)) (handle, ()))) = _
  simp only [NTy.decode?_encode, Option.map_some]
  rfl

/-- The other recognized physical Table layout includes its cached length. -/
@[lir_denote_norm] theorem observeRuntime?_cached_handle {unit : Validation.ValidatedUnit}
    (frame : Skolems unit) (memory : Memory unit) (source : StructHandle)
    (arguments : NRow) (handle : String) (length : SpecInt (.bits 64) false) :
    observeRuntime? frame memory
        (.struct source arguments (.cons .address (.cons (.int 64 false) .nil)))
        (.nominal source none #[.address handle, .integer length.val]) =
      some (observe memory (.struct source (NRow.substWith frame.type arguments)
        (.cons .address (.cons (.int 64 false) .nil))) (handle, length, ())) := by
  rw [observeRuntime?_resolve, frame.resolve_eq]
  change Option.map _ ((@NTy.codec (Carriers.runtime unit)
    (.struct source (NRow.substWith frame.type arguments)
      (.cons .address (.cons (.int 64 false) .nil)))).decode?
      (@NTy.encode (Carriers.runtime unit)
        (.struct source (NRow.substWith frame.type arguments)
          (.cons .address (.cons (.int 64 false) .nil))) (handle, length, ()))) = _
  simp only [NTy.decode?_encode, Option.map_some]
  rfl

/-- Resolve concrete constructors without unfolding an opaque parameter. -/
@[lir_denote_norm] theorem resolve_int {unit : Validation.ValidatedUnit}
    (frame : Skolems unit) (width : Nat) (signed : Bool) :
    frame.resolve (.int width signed) = .int width signed := frame.resolve_eq _
@[lir_denote_norm] theorem resolve_struct {unit : Validation.ValidatedUnit}
    (frame : Skolems unit) (owner : StructHandle) (arguments fields : NRow) :
    frame.resolve (.struct owner arguments fields) =
      .struct owner (NRow.substWith frame.type arguments) (NRow.substWith frame.type fields) :=
  frame.resolve_eq _
/-- Normalize an observation's phantom arguments despite its dependent value
argument. The physical carrier depends only on the fields. -/
@[congr] theorem observe_struct_arguments {unit : Validation.ValidatedUnit}
    (memory : Memory unit) (source : StructHandle) (fields : NRow)
    (left right : NRow) (same : left = right)
    (value : @HList (Carriers.runtime unit) fields) :
    SnapshotValue.observe memory (.struct source left fields) value =
      SnapshotValue.observe memory (.struct source right fields) value := by
  cases same
  rfl

@[lir_denote_norm] theorem physical_observeRuntime?_getD {unit : Validation.ValidatedUnit}
    (frame : Skolems unit) (memory : Memory unit) (type : NTy) (raw : RuntimeValue) :
    ((SnapshotValue.observeRuntime? frame memory type raw).getD
      (.scalar .unit)).physical =
      (((@NTy.codec frame.toCarriers type).decode? raw).map
        (@NTy.encode frame.toCarriers type)).getD .unit := by
  cases decoded : (@NTy.codec frame.toCarriers type).decode? raw <;>
    simp [SnapshotValue.observeRuntime?, decoded, SnapshotValue.physical_observeInFrame]
end LeanerIR.Proofs.Denote.SnapshotValue
