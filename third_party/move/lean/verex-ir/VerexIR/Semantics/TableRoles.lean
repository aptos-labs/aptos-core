-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Semantics.Operations
import VerexIR.Semantics.Focus
import VerexIR.Proofs.Maps
import VerexIR.Validation.TableRoles

/-!
# The runtime semantics of table roles

A table is a value: its owner's value holds its identity, a handle or the
placeholder field of an owner declared without fields, its cached length
where the owner declares one, and its entries in a hidden last field
(`doc/designs/intrinsic-maps.md`, "Tables in the runtime semantics"). A function
bound to an executable role of a table owner (`tableRole?`) is provided by
the runtime, not run from its body: the role's meaning on that value by the
map model (`Maps`), the meaning the verifier reads the role by. Allocation
takes a handle no table had (`RuntimeState.tableHandles`).
-/

namespace VerexIR.SemanticOperations

open Validation

attribute [local instance] Maps.decEq

/-- The executable map role a function plays for a table owner of its
namespace, with the owner's layout (`Validation.tableRoleOf?`). -/
def tableRole? (unit : ValidatedUnit) (handle : FunctionHandle) : Option (String × TableLayout) :=
  tableRoleOf? unit handle.namespaceId handle.functionId

/-- The map model's layout of a table owner. -/
def _root_.VerexIR.Validation.TableLayout.model (layout : TableLayout) : Maps.Layout where
  owner := ⟨layout.namespaceId, layout.structId⟩
  variant := none
  entry := none
  handle := layout.handle
  length := layout.length
  dummy := layout.dummy
  ghosts := layout.ghosts

/-- How a table owner keeps its entries: in key order where it binds an
ordering role, by the order of its namespace, and in insertion order
otherwise. -/
def _root_.VerexIR.Validation.TableLayout.discipline (unit : ValidatedUnit) (layout : TableLayout) :
    Maps.Discipline :=
  if layout.ordered then
    .ordered (valueRanks (match unit.namespaces[layout.namespaceId.index]? with
      | some ns => ns.orders
      | none => default))
  else .sequence

/-- The handle a table is allocated at: the first of `table-0`, `table-1`,
… no table had. -/
def freshTableHandle (handles : Array String) : String :=
  go 0 (handles.size + 1)
where
  go (candidate : Nat) : Nat → String
    | 0 => s!"table-{candidate}"
    | fuel + 1 =>
        let handle := s!"table-{candidate}"
        if handles.contains handle then go (candidate + 1) fuel else handle

/-- The abort codes of the table owners (`aptos-move/framework/aptos-stdlib`):
the natives abort on a key already present or absent in their legacy
encoding, `reason * 256 + category` with the invalid-argument category 7;
no native checks emptiness, so checked destruction of a nonempty table is
`table_with_length::destroy_empty`'s `error::invalid_state(102)`,
`(category << 16) + reason`. The bulk roles use their own
argument-rejection code. -/
def tableAlreadyExists : Int := (100 * 256) + 7
def tableNotFound : Int := (101 * 256) + 7
def tableNotEmpty : Int := (3 * 65536) + 102
def tableInvalidArguments : Int := 103
/-- The code `big_ordered_map::new_with_config` rejects a degree with,
`error::invalid_argument(EINVALID_CONFIG_PARAMETER)`. -/
def invalidConfiguration : Int := (1 * 65536) + 11

/-- A map argument: its loan, where it is a mutable reference, and the value
it reads. -/
def mapArgument : RuntimeValue → Option Nat × RuntimeValue
  | .borrow loan current => (some loan, current)
  | value => (none, value)

/-- The key of a nonempty map's first or last entry. -/
def endKey (map : RuntimeValue) (front : Bool) : RuntimeValue :=
  Maps.keyAt map (if front then 0 else Maps.size map - 1)

/-- The owner with a loan's hole in place of an entry's value. -/
def withEntryHole (map key : RuntimeValue) (loan : Nat) : Option RuntimeValue := do
  let .nominal _ _ fields := map | none
  guard (0 < fields.size)
  writeProjections? map
    [.field (fields.size - 1), .index (Maps.indexOf (Maps.entriesOf map) key), .index 1]
    (.loanHole loan)

/-- Whether adding several entries aborts (`Maps.AbortsAddAll`). -/
def abortsAddAll (map keys values : RuntimeValue) : Bool :=
  (Maps.elementsOf keys).length != (Maps.elementsOf values).length ||
    (Maps.elementsOf keys).any (Maps.hasKey map ·) || !decide (Maps.elementsOf keys).Nodup

/-- Whether building a map aborts (`Maps.AbortsNewFrom`). -/
def abortsNewFrom (keys values : RuntimeValue) : Bool :=
  (Maps.elementsOf keys).length != (Maps.elementsOf values).length ||
    !decide (Maps.elementsOf keys).Nodup

/-- Whether appending a map aborts on a key both hold (`Maps.AbortsAppendDisjoint`). -/
def abortsAppendDisjoint (map other : RuntimeValue) : Bool :=
  (Maps.entriesOf other).any fun entry => Maps.hasKey map entry.1

/-- Whether a map's entries keep its discipline's invariant (`Maps.Valid`). -/
def validEntries : Maps.Discipline → Maps.Entries → Bool
  | .sequence, entries => decide (entries.map Prod.fst).Nodup
  | .ordered rank, entries =>
      decide (entries.Pairwise fun earlier later => RuntimeValue.order rank earlier.1 later.1 = .lt)

/-- What a table role does with the values of its arguments: abort with a
code; return a result, with the map's new value and a handle it allocates;
or lend the value of a map's entry. -/
inductive RoleStep where
  | abort (code : Int)
  | returned (result : RuntimeValue) (updated : Option RuntimeValue) (allocated : Option String)
  | lends (map key value : RuntimeValue)

/-- Both representations used by Move's `Option`: the original vector
wrapper and an enum with an empty and a single-field variant. -/
inductive OptionalLayout where
  | vector
  | variants (empty present : String)
  deriving Repr, BEq

/-- Read the optional result's representation from its declaration. The
runtime and the generated role contract must construct the same value. -/
def optionalLayout? (ns : ValidatedNamespace) (declaration : StructDecl) : Option OptionalLayout := do
  guard declaration.ghosts.isEmpty
  if declaration.variants.isEmpty then
    let #[field] := declaration.fields | none
    let some (.vector _) := ns.tables.types[field.type.typeId.index]? | none
    return .vector
  guard (declaration.variants.size == 2)
  let empty ← declaration.variants.find? (·.fields.isEmpty)
  let present ← declaration.variants.find? (·.fields.size == 1)
  return .variants (← ns.tables.names[empty.name.index]?).name (← ns.tables.names[present.name.index]?).name

def OptionalLayout.encode (layout : OptionalLayout) (owner : StructHandle)
    (value : Option RuntimeValue) : RuntimeValue :=
  match layout with
  | .vector => .nominal owner none #[.vector value.toArray]
  | .variants empty present =>
      .nominal owner (some (if value.isSome then present else empty)) value.toArray

/-- An optional result of a role, in the representation its declaration uses. -/
def roleOptional (unit : ValidatedUnit) (ns : ValidatedNamespace)
    (declaration : FunctionDecl FunctionBody) (value : Option RuntimeValue) :
    Option RuntimeValue := do
  let result ← declaration.signature.results[0]?
  let .nominal name _ ← ns.tables.types[result.typeId.index]? | none
  let owner ← resolveNominal? unit ns name
  let target ← unit.namespaces[owner.namespaceId.index]?
  let declaration ← target.structs[owner.structId]?
  let layout ← optionalLayout? target declaration
  some (layout.encode owner value)

/-- The iterator an entry role (`map_iter_borrow_mut`) indexes a map by: the
declaration of its first parameter's type, whose variant with one field
holds a key, where the field is the iterator's type parameter, or a position. -/
structure IteratorShape where
  variant : String
  keyed : Bool

def iteratorShape? (unit : ValidatedUnit) (ns : ValidatedNamespace)
    (declaration : FunctionDecl FunctionBody) : Option IteratorShape := do
  let parameter ← declaration.signature.parameters[0]?
  let .nominal name _ ← ns.tables.types[parameter.typeUse.typeId.index]? | none
  let (namespaceId, structId) ← nominalDeclarationId? unit ns name
  let target ← unit.namespaces[namespaceId.index]?
  let iterator ← target.structs[structId]?
  let variant ← iterator.variants.find? (·.fields.size == 1)
  let named ← target.tables.names[variant.name.index]?
  let field ← variant.fields[0]?
  let keyed := target.tables.types[field.type.typeId.index]? == some (.typeParameter 0)
  pure { variant := named.name, keyed }

/-- The key an iterator's payload names in a map: the payload itself, or the
key at the position it holds, which is none out of range. -/
def iteratorKey? (shape : IteratorShape) (map payload : RuntimeValue) : Option RuntimeValue :=
  if shape.keyed then some payload
  else match payload with
    | .integer position =>
        if 0 ≤ position && position < Maps.size map then some (Maps.keyAt map position) else none
    | _ => none

/-- The step lending the entry an iterator names in a map: of the iterator's
payload variant, holding a key the map holds or a position below its size;
an end iterator, an absent key, and a position out of range are a missing
entry. -/
def iteratorEntryStep (shape : IteratorShape) (map iterator : RuntimeValue) : Option RoleStep := do
  let .nominal _ (some variant) fields := iterator | none
  if variant != shape.variant then some (.abort tableNotFound) else
  let payload ← fields[0]?
  match iteratorKey? shape map payload with
  | some key =>
      if Maps.hasKey map key then some (.lends map key (Maps.valueAt map key))
      else some (.abort tableNotFound)
  | none => some (.abort tableNotFound)

/-- The step building a map's entries in another table layout
(`Validation.resultLayout?`), at a fresh handle where it has one. -/
def conversionStep (unit : ValidatedUnit) (result : TableLayout) (map : RuntimeValue)
    (handles : Array String) : RoleStep :=
  let built (empty : RuntimeValue) : RuntimeValue :=
    Maps.updateAll result.model (result.discipline unit) empty (Maps.keysOf map) (Maps.valuesOf map)
  if result.handle then
    let tableHandle := freshTableHandle handles
    .returned (built (result.model.build (.address tableHandle) [])) none (some tableHandle)
  else .returned (built (Maps.empty result.model)) none none

/-- A table role's meaning on the values of its arguments, a mutable
reference's current value among them, by the map model, `handles` the
handles allocated so far. Invalid arguments have no step. -/
def tableOperationStep (unit : ValidatedUnit) (ns : ValidatedNamespace)
    (declaration : FunctionDecl FunctionBody) (role : MapOperation) (layout : TableLayout)
    (handles : Array String) (values : Array RuntimeValue) : Option RoleStep := do
  let model := layout.model
  let discipline := layout.discipline unit
  let argument (index : Nat) : Option RuntimeValue := values[index]?
  let map := argument role.mapArgument
  let abort (code : Int) : Option RoleStep := some (.abort code)
  let exit (result : RuntimeValue) : Option RoleStep := some (.returned result none none)
  let exitUpdated (updated result : RuntimeValue) : Option RoleStep :=
    some (.returned result (some updated) none)
  let optional := roleOptional unit ns declaration
  let pair (key value : RuntimeValue) : RuntimeValue := .tuple #[key, value]
  -- A fresh map: of a table layout, at a handle no table had.
  let fresh : Option RoleStep :=
    if layout.handle then
      let tableHandle := freshTableHandle handles
      some (.returned (model.build (.address tableHandle) []) none (some tableHandle))
    else exit (Maps.empty model)
  match role with
  | .new_ => fresh
  | .newWithConfig =>
      let .integer inner ← argument 0 | none
      let .integer leaf ← argument 1 | none
      if (inner != 0 && (inner < 4 || 4096 < inner)) ||
          (leaf != 0 && (leaf < 3 || 4096 < leaf)) then
        abort invalidConfiguration
      else fresh
  | .newFrom =>
      let keys ← argument 0
      let values ← argument 1
      if (Maps.elementsOf keys).length != (Maps.elementsOf values).length then
        abort tableInvalidArguments
      else if abortsNewFrom keys values then abort tableAlreadyExists
      else exit (Maps.updateAll model discipline (Maps.empty model) keys values)
  | .len => exit (.integer (Maps.size (← map)))
  | .isEmpty => exit (.bool (Maps.size (← map) == 0))
  | .hasKey => exit (.bool (Maps.hasKey (← map) (← argument 1)))
  | .borrow =>
      let current ← map
      let key ← argument 1
      if Maps.hasKey current key then exit (Maps.valueAt current key) else abort tableNotFound
  | .borrowWithDefault =>
      let current ← map
      let key ← argument 1
      let default ← argument 2
      exit (if Maps.hasKey current key then Maps.valueAt current key else default)
  | .get =>
      let current ← map
      let key ← argument 1
      exit (← optional (if Maps.hasKey current key then some (Maps.valueAt current key) else none))
  | .borrowMut | .borrowMutWithDefault =>
      let current ← map
      let key ← argument 1
      let current ← if Maps.hasKey current key then some current
        else if role == .borrowMut then some current
        else do some (Maps.update model discipline current key (← argument 2))
      if !Maps.hasKey current key then abort tableNotFound
      else some (.lends current key (Maps.valueAt current key))
  | .iterBorrowMut => iteratorEntryStep (← iteratorShape? unit ns declaration) (← map) (← argument 0)
  | .addNoOverride =>
      let current ← map
      let key ← argument 1
      let value ← argument 2
      if Maps.hasKey current key then abort tableAlreadyExists
      else exitUpdated (Maps.update model discipline current key value) .unit
  | .addOverrideIfExists =>
      let current ← map
      exitUpdated (Maps.update model discipline current (← argument 1) (← argument 2)) .unit
  | .upsert =>
      let current ← map
      let key ← argument 1
      let value ← argument 2
      let previous := if Maps.hasKey current key then some (Maps.valueAt current key) else none
      exitUpdated (Maps.update model discipline current key value) (← optional previous)
  | .delMustExist =>
      let current ← map
      let key ← argument 1
      if Maps.hasKey current key then
        exitUpdated (Maps.remove model discipline current key) (Maps.valueAt current key)
      else abort tableNotFound
  | .removeOrNone =>
      let current ← map
      let key ← argument 1
      let previous := if Maps.hasKey current key then some (Maps.valueAt current key) else none
      exitUpdated (Maps.remove model discipline current key) (← optional previous)
  | .delReturnKey =>
      let current ← map
      let key ← argument 1
      if Maps.hasKey current key then
        exitUpdated (Maps.remove model discipline current key) (pair key (Maps.valueAt current key))
      else abort tableNotFound
  | .destroyUnchecked => exit .unit
  | .destroyEmpty =>
      if Maps.size (← map) == 0 then exit .unit else abort tableNotEmpty
  | .addAll =>
      let current ← map
      let keys ← argument 1
      let values ← argument 2
      if (Maps.elementsOf keys).length != (Maps.elementsOf values).length then
        abort tableInvalidArguments
      else if abortsAddAll current keys values then abort tableAlreadyExists
      else exitUpdated (Maps.updateAll model discipline current keys values) .unit
  | .upsertAll =>
      let current ← map
      let keys ← argument 1
      let values ← argument 2
      if (Maps.elementsOf keys).length != (Maps.elementsOf values).length then
        abort tableInvalidArguments
      else exitUpdated (Maps.updateAll model discipline current keys values) .unit
  | .append | .appendDisjoint =>
      let current ← map
      let other ← argument 1
      if role == .appendDisjoint && abortsAppendDisjoint current other then
        abort tableAlreadyExists
      else exitUpdated (Maps.updateAll model discipline current (Maps.keysOf other)
        (Maps.valuesOf other)) .unit
  | .trim =>
      let current ← map
      let .integer count ← argument 1 | none
      if Maps.size current < count then abort tableInvalidArguments
      else exitUpdated (Maps.takeEntries model current count) (Maps.dropEntries model current count)
  | .replaceKeyInplace =>
      let current ← map
      let old ← argument 1
      let new ← argument 2
      let replaced := Maps.replaceKey model current old new
      if !Maps.hasKey current old then abort tableNotFound
      else if !validEntries discipline (Maps.entriesOf replaced) then abort tableInvalidArguments
      else exitUpdated replaced .unit
  | .borrowFront | .borrowBack =>
      let current ← map
      if Maps.size current == 0 then abort tableNotFound else
      let key := endKey current (role == .borrowFront)
      exit (pair key (Maps.valueAt current key))
  | .frontKey | .backKey =>
      let current ← map
      if Maps.size current == 0 then abort tableNotFound
      else exit (endKey current (role == .frontKey))
  | .popFront | .popBack =>
      let current ← map
      if Maps.size current == 0 then abort tableNotFound else
      let key := endKey current (role == .popFront)
      exitUpdated (Maps.remove model discipline current key) (pair key (Maps.valueAt current key))
  | .prevKey | .nextKey =>
      let .ordered rank := discipline | none
      let current ← map
      let key ← argument 1
      exit (← optional (if role == .prevKey then Maps.prevKey? rank current key
        else Maps.nextKey? rank current key))
  | .keys => exit (Maps.keysOf (← map))
  | .values => exit (Maps.valuesOf (← map))
  | .toVecPair =>
      let current ← map
      exit (pair (Maps.keysOf current) (Maps.valuesOf current))
  | .toOrderedMap => some (conversionStep unit (← resultLayout? unit ns declaration) (← map) handles)

/-- A declared role's step: of the executable role its name denotes, which
preparation resolves the same way. -/
def roleStep (unit : ValidatedUnit) (ns : ValidatedNamespace)
    (declaration : FunctionDecl FunctionBody) (role : String) (layout : TableLayout)
    (handles : Array String) (values : Array RuntimeValue) : Option RoleStep := do
  tableOperationStep unit ns declaration (← MapOperation.ofName? role) layout handles values

/-- The position of the map among a declared role's arguments. -/
def roleMapArgument (role : String) : Nat :=
  ((MapOperation.ofName? role).map (·.mapArgument)).getD 0

/-- Whether a value of the owner's type holds fewer than 2^64 entries, so
that its size, and a cached length, reads as a `u64`, as a vector's length
does. A value of another type fits. -/
def _root_.VerexIR.Validation.TableLayout.fits (layout : TableLayout) : RuntimeValue → Bool
  | .nominal source _ fields =>
      source != ⟨layout.namespaceId, layout.structId⟩ ||
        decide (Maps.size (.nominal source none fields) < 18446744073709551616)
  | _ => true

/-- The values a step yields. -/
def RoleStep.values : RoleStep → List RuntimeValue
  | .abort _ => []
  | .returned result updated _ => result :: updated.toList
  | .lends map _ value => [map, value]

/-- The mutable references among a role's arguments, from position `index`
on, exported at its exit as a body exports its mutable parameters: the
map's, at position `mapIndex`, with its new value, any other with its
current one. -/
def roleExportsFrom (updated : Option RuntimeValue) (mapIndex : Nat) :
    List RuntimeValue → Nat → List (Nat × RuntimeValue)
  | [], _ => []
  | .borrow loan current :: rest, index =>
      (loan, if index == mapIndex then updated.getD current else current) ::
        roleExportsFrom updated mapIndex rest (index + 1)
  | _ :: rest, index => roleExportsFrom updated mapIndex rest (index + 1)

/-- The mutable references among a role's arguments exported at its exit. -/
def roleExports (arguments : Array RuntimeValue) (updated : Option RuntimeValue)
    (mapIndex : Nat) : Array (Nat × RuntimeValue) :=
  (roleExportsFrom updated mapIndex arguments.toList 0).toArray

/-- The run of a table role: its step (`roleStep`) on the values of its
arguments, which hold no loan, as a role reads none, and no table of 2^64
entries or more. A mutable reference
argument is exported at exit, the map's with its new value; an entry lent
is a loan into the map the caller lent, whose hole the exported map holds,
so the entry's final value returns into the table when the loan dies. -/
def tableRoleCall (unit : ValidatedUnit) (handle : FunctionHandle) (role : String)
    (layout : TableLayout) (state : RuntimeState) (arguments : Array RuntimeValue) :
    Option (RuntimeState × Outcome) := do
  let ns ← unit.namespaces[handle.namespaceId.index]?
  let declaration ← ns.functions[handle.functionId.index]?
  guard (arguments.size == declaration.signature.parameters.size)
  let values := arguments.map fun argument => (mapArgument argument).2
  guard (values.all plain?)
  -- A table holds fewer than 2^64 entries: a role has no run on a larger
  -- one, nor one making it.
  guard (values.all layout.fits)
  let exit (state : RuntimeState) (updated : Option RuntimeValue) (result : RuntimeValue) :
      Option (RuntimeState × Outcome) := do
    let results ← unpackFallthrough declaration.signature.results.size result
    some ({ state with
      pending := state.pending ++ roleExports arguments updated (roleMapArgument role) },
      .returned results)
  let step ← roleStep unit ns declaration role layout state.tableHandles values
  guard (step.values.all layout.fits)
  match step with
  | .abort code => some (state, .threw .abort #[.integer code])
  | .returned result updated allocated =>
      exit { state with tableHandles := allocated.elim state.tableHandles state.tableHandles.push }
        updated result
  | .lends map key value =>
      let some (.borrow _ _) := arguments[roleMapArgument role]? | none
      let loan := state.nextLoan
      let holed ← withEntryHole map key loan
      exit { state with nextLoan := loan + 1 } (some holed) (.borrow loan value)

end VerexIR.SemanticOperations
