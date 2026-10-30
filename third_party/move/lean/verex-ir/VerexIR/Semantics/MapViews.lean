-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Semantics.TableRoles
import VerexIR.Validation.MapOwners

/-!
# The abstract value of an intrinsic map owner

The runtime's values are read up to a normalization: every value of an
intrinsic map owner is replaced by the canonical value of its abstract
layout, its entries, ghost slots, and handle read off the concrete shape
(`doc/designs/intrinsic-maps.md`, "Abstract representation"). The agreement
relates runtime values to the denotation's through it.
-/

namespace VerexIR.SemanticOperations

open Validation

/-- The canonical layout of an owner's abstract value: a table's own, else
its entries, its ghost slots, and its handle where its concrete value holds
one. -/
def _root_.VerexIR.Validation.OwnerLayout.owner (layout : OwnerLayout) : StructHandle :=
  ⟨layout.namespaceId, layout.structId⟩

def _root_.VerexIR.Validation.OwnerLayout.canonical (layout : OwnerLayout) : Maps.Layout :=
  match layout.shape with
  | .table table => table.model
  | .entries =>
      { owner := layout.owner, variant := none, entry := none, handle := false, length := false,
        dummy := false, ghosts := layout.ghosts }
  | .wrapped =>
      { owner := layout.owner, variant := none, entry := none, handle := true, length := false,
        dummy := false, ghosts := layout.ghosts }
  | .tree _ _ _ _ _ _ handlePath =>
      { owner := layout.owner, variant := none, entry := none, handle := handlePath.isSome,
        length := false, dummy := false, ghosts := layout.ghosts }

/-- How an owner keeps its entries: in key order where it binds an ordering
role, by the order of its namespace, and in insertion order otherwise. -/
def _root_.VerexIR.Validation.OwnerLayout.discipline (unit : ValidatedUnit) (layout : OwnerLayout) :
    Maps.Discipline :=
  if layout.ordered then
    .ordered (valueRanks (match unit.namespaces[layout.namespaceId.index]? with
      | some ns => ns.orders
      | none => default))
  else .sequence

/-- The ghost slots of a concrete owner value: after its declared fields. -/
def _root_.VerexIR.Validation.OwnerLayout.slotsOf (layout : OwnerLayout) (value : RuntimeValue) :
    List RuntimeValue :=
  (List.range layout.ghosts).map fun k => value.field (layout.declared + k)

/-- The entries a vector of two-field entry values holds. -/
def entriesAt (value : RuntimeValue) (index : Nat) : Maps.Entries :=
  match value.field index with
  | .vector elements => elements.toList.map fun entry => (entry.field 0, entry.field 1)
  | _ => []

/-- The value at a path of field indices. -/
def followPath (value : RuntimeValue) : List Nat → RuntimeValue
  | [] => value
  | index :: rest => followPath (value.field index) rest

/-- Entries in key order, a later entry replacing an earlier one at its key. -/
def _root_.VerexIR.Maps.sortEntries (rank : ValueRanks) (entries : Maps.Entries) : Maps.Entries :=
  entries.foldl (fun entries entry => Maps.ordSet rank entry.1 entry.2 entries) []

/-- Whether a value has the shape of an owner's canonical value: no variant,
its handle where the layout holds one, its slots, and entries as pairs. The
concrete shapes differ from it in a variant, a field count, a field's kind,
or an entry's kind, so the view leaves a canonical value as it is. -/
def _root_.VerexIR.Validation.OwnerLayout.isCanonical (layout : OwnerLayout) (value : RuntimeValue) :
    Bool :=
  let canonical := layout.canonical
  match value with
  | .nominal _ none fields =>
      fields.size == canonical.leading + canonical.ghosts + 1 &&
        (!canonical.handle || match fields[0]? with | some (RuntimeValue.address _) => true | _ => false) &&
        match fields.back? with
        | some (.vector entries) => entries.all fun
            | .tuple pair => pair.size == 2
            | _ => false
        | _ => false
  | _ => false

/-- The abstract value of a concrete owner value: the canonical value of its
layout, holding its entries, its ghost slots, and its handle. -/
def _root_.VerexIR.Validation.OwnerLayout.view (unit : ValidatedUnit) (layout : OwnerLayout)
    (value : RuntimeValue) : RuntimeValue :=
  let canonical := layout.canonical
  if layout.isCanonical value then value else
  match layout.shape with
  | .table _ => value
  | .entries => canonical.build .unit (entriesAt value (layout.declared - 1)) (layout.slotsOf value)
  | .wrapped =>
      let inner := value.field 0
      canonical.build (inner.field 0) (Maps.entriesOf inner) (layout.slotsOf value)
  | .tree nodeNamespace nodeStruct root nodes children leaf handlePath =>
      let handle := match handlePath with
        | some path => followPath value path
        | none => .unit
      let entries := Maps.treeEntries ⟨nodeNamespace, nodeStruct⟩ root nodes children leaf value
      let rank := match layout.discipline unit with
        | .ordered rank => rank
        | .sequence => valueRanks default
      canonical.build handle (Maps.sortEntries rank entries) (layout.slotsOf value)

mutual
/-- A runtime value with every intrinsic map owner it holds whose values the
denotation holds abstractly (`abstractLayout?`) read as its abstract value,
innermost first. -/
def normalize (unit : ValidatedUnit) : RuntimeValue → RuntimeValue
  | .vector elements => .vector (normalizeList unit elements.toList).toArray
  | .tuple elements => .tuple (normalizeList unit elements.toList).toArray
  | .nominal source variant fields =>
      let value := RuntimeValue.nominal source variant (normalizeList unit fields.toList).toArray
      match abstractLayout? unit source.namespaceId source.structId with
      | some layout => layout.view unit value
      | none => value
  | .closure function mask typeInstantiation captures =>
      .closure function mask typeInstantiation (normalizeList unit captures.toList).toArray
  | .borrow loan current => .borrow loan (normalize unit current)
  | .loanHole loan => .loanHole loan
  | .ghost contents => .ghost (normalize unit contents)
  | .unit => .unit
  | .bool value => .bool value
  | .character value => .character value
  | .integer value => .integer value
  | .address value => .address value
  | .signer value => .signer value
  | .string value => .string value
  | .bytes value => .bytes value
termination_by value => sizeOf value
decreasing_by all_goals (simp_wf; (try simp only [Array.sizeOf_eq_toList]); try omega)

/-- Each value of a list normalized. -/
def normalizeList (unit : ValidatedUnit) : List RuntimeValue → List RuntimeValue
  | [] => []
  | value :: values => normalize unit value :: normalizeList unit values
termination_by values => sizeOf values
end

/-- A global store with every value normalized; its keys and their order are
unchanged. -/
def _root_.VerexIR.GlobalMap.normalize (unit : ValidatedUnit) (globals : GlobalMap) : GlobalMap :=
  ⟨globals.entries.map fun slot => { slot with value := SemanticOperations.normalize unit slot.value }⟩

/-- A runtime state read up to normalization: its stores' and pending
exports' values normalized, its loan bookkeeping as it is. -/
def _root_.VerexIR.RuntimeState.normalize (unit : ValidatedUnit) (state : RuntimeState) :
    RuntimeState :=
  { state with
    globals := state.globals.normalize unit
    pending := state.pending.map fun entry => (entry.1, SemanticOperations.normalize unit entry.2) }

/-- An outcome with its values normalized. -/
def _root_.VerexIR.Outcome.normalize (unit : ValidatedUnit) : Outcome → Outcome
  | .returned values => .returned (values.map (SemanticOperations.normalize unit))
  | .threw kind arguments => .threw kind (arguments.map (SemanticOperations.normalize unit))

@[simp] theorem _root_.VerexIR.RuntimeState.normalize_globals (unit : ValidatedUnit)
    (state : RuntimeState) : (state.normalize unit).globals = state.globals.normalize unit := rfl
@[simp] theorem _root_.VerexIR.RuntimeState.normalize_globalLoans (unit : ValidatedUnit)
    (state : RuntimeState) : (state.normalize unit).globalLoans = state.globalLoans := rfl
@[simp] theorem _root_.VerexIR.RuntimeState.normalize_nextLoan (unit : ValidatedUnit)
    (state : RuntimeState) : (state.normalize unit).nextLoan = state.nextLoan := rfl
@[simp] theorem _root_.VerexIR.RuntimeState.normalize_tableHandles (unit : ValidatedUnit)
    (state : RuntimeState) : (state.normalize unit).tableHandles = state.tableHandles := rfl
@[simp] theorem _root_.VerexIR.RuntimeState.normalize_pending (unit : ValidatedUnit)
    (state : RuntimeState) :
    (state.normalize unit).pending =
      state.pending.map fun entry => (entry.1, normalize unit entry.2) := rfl

@[simp] theorem _root_.VerexIR.GlobalMap.normalize_lookup (unit : ValidatedUnit) (globals : GlobalMap)
    (key : GlobalKey) :
    (globals.normalize unit).lookup key = (globals.lookup key).map (normalize unit) := by
  simp only [GlobalMap.lookup, GlobalMap.normalize, Array.find?_map, Option.map_map]
  rfl

/-! Normalization is structural. -/

private theorem map_toList_toArray {α β : Type} (f : α → β) (values : Array α) :
    (values.toList.map f).toArray = values.map f := by
  rw [← Array.toList_map, Array.toArray_toList]

theorem normalizeList_eq_map (unit : ValidatedUnit) (values : List RuntimeValue) :
    normalizeList unit values = values.map (normalize unit) := by
  induction values with
  | nil => rw [normalizeList]; rfl
  | cons value values ih => rw [normalizeList, ih]; rfl

@[simp] theorem normalize_tuple (unit : ValidatedUnit) (elements : Array RuntimeValue) :
    normalize unit (.tuple elements) = .tuple (elements.map (normalize unit)) := by
  rw [normalize, normalizeList_eq_map, map_toList_toArray]
@[simp] theorem normalize_vector (unit : ValidatedUnit) (elements : Array RuntimeValue) :
    normalize unit (.vector elements) = .vector (elements.map (normalize unit)) := by
  rw [normalize, normalizeList_eq_map, map_toList_toArray]
@[simp] theorem normalize_closure (unit : ValidatedUnit) (function : FunctionHandle) (mask : Nat)
    (typeInstantiation : Array (TypeId × TypeId)) (captures : Array RuntimeValue) :
    normalize unit (.closure function mask typeInstantiation captures) =
      .closure function mask typeInstantiation (captures.map (normalize unit)) := by
  rw [normalize, normalizeList_eq_map, map_toList_toArray]
@[simp] theorem normalize_borrow (unit : ValidatedUnit) (loan : Nat) (current : RuntimeValue) :
    normalize unit (.borrow loan current) = .borrow loan (normalize unit current) := by
  rw [normalize]
@[simp] theorem normalize_loanHole (unit : ValidatedUnit) (loan : Nat) :
    normalize unit (.loanHole loan) = .loanHole loan := by rw [normalize]
@[simp] theorem normalize_ghost (unit : ValidatedUnit) (contents : RuntimeValue) :
    normalize unit (.ghost contents) = .ghost (normalize unit contents) := by rw [normalize]
/-- A nominal value normalizes to its fields normalized, read as its owner's
abstract value where its declaration is an owner. -/
theorem normalize_nominal (unit : ValidatedUnit) (source : StructHandle) (variant : Option String)
    (fields : Array RuntimeValue) :
    normalize unit (.nominal source variant fields) =
      match abstractLayout? unit source.namespaceId source.structId with
      | some layout => layout.view unit (.nominal source variant (fields.map (normalize unit)))
      | none => .nominal source variant (fields.map (normalize unit)) := by
  rw [normalize, normalizeList_eq_map, map_toList_toArray]

/-- The semantic types of an abstract owner's leading fields: its handle or
placeholder, and its cached length. -/
def _root_.VerexIR.Validation.OwnerLayout.leadingTypes (layout : OwnerLayout) : List SemTy :=
  (if layout.canonical.handle then [SemTy.address]
    else if layout.canonical.dummy then [SemTy.bool] else []) ++
  (if layout.canonical.length then [SemTy.integer (.bits 64) false] else [])

/-- The semantic types of the fields of an owner's canonical value: its
leading fields, its ghost slots, and its entries at its key and value
types. -/
def _root_.VerexIR.Validation.OwnerLayout.canonicalTypes (layout : OwnerLayout)
    (ghostTypes : List SemTy) (key value : SemTy) : List SemTy :=
  layout.leadingTypes ++ (ghostTypes.map .ghost ++ [.vector (.tuple [key, value]) none])

/-! Scalar values normalize to themselves. -/

@[simp] theorem normalize_unit (unit : ValidatedUnit) : normalize unit .unit = .unit := by
  rw [normalize]
@[simp] theorem normalize_bool (unit : ValidatedUnit) (value : Bool) :
    normalize unit (.bool value) = .bool value := by rw [normalize]
@[simp] theorem normalize_character (unit : ValidatedUnit) (value : Nat) :
    normalize unit (.character value) = .character value := by rw [normalize]
@[simp] theorem normalize_integer (unit : ValidatedUnit) (value : Int) :
    normalize unit (.integer value) = .integer value := by rw [normalize]
@[simp] theorem normalize_address (unit : ValidatedUnit) (value : String) :
    normalize unit (.address value) = .address value := by rw [normalize]
@[simp] theorem normalize_signer (unit : ValidatedUnit) (value : String) :
    normalize unit (.signer value) = .signer value := by rw [normalize]
@[simp] theorem normalize_string (unit : ValidatedUnit) (value : String) :
    normalize unit (.string value) = .string value := by rw [normalize]
@[simp] theorem normalize_bytes (unit : ValidatedUnit) (value : Array UInt8) :
    normalize unit (.bytes value) = .bytes value := by rw [normalize]

@[simp] theorem _root_.VerexIR.Outcome.normalize_returned (unit : ValidatedUnit)
    (values : Array RuntimeValue) :
    (Outcome.returned values).normalize unit = .returned (values.map (normalize unit)) := rfl
@[simp] theorem _root_.VerexIR.Outcome.normalize_threw (unit : ValidatedUnit) (kind : ThrowKind)
    (arguments : Array RuntimeValue) :
    (Outcome.threw kind arguments).normalize unit = .threw kind (arguments.map (normalize unit)) :=
  rfl

/-- An outcome normalizing to a return returned the values it normalizes from. -/
theorem _root_.VerexIR.Outcome.normalize_eq_returned {unit : ValidatedUnit} {outcome : Outcome}
    {values : Array RuntimeValue} (normalized : outcome.normalize unit = .returned values) :
    ∃ returned, outcome = .returned returned ∧ returned.map (normalize unit) = values := by
  cases outcome with
  | returned returned => exact ⟨returned, rfl, Outcome.returned.inj normalized⟩
  | threw _ _ => cases normalized

/-- An outcome normalizing to a throw threw the payload it normalizes from. -/
theorem _root_.VerexIR.Outcome.normalize_eq_threw {unit : ValidatedUnit} {outcome : Outcome}
    {kind : ThrowKind} {arguments : Array RuntimeValue}
    (normalized : outcome.normalize unit = .threw kind arguments) :
    ∃ payload, outcome = .threw kind payload ∧ payload.map (normalize unit) = arguments := by
  cases outcome with
  | returned _ => cases normalized
  | threw kind' payload =>
      obtain ⟨rfl, rfl⟩ := Outcome.threw.inj normalized
      exact ⟨payload, rfl, rfl⟩

end VerexIR.SemanticOperations
