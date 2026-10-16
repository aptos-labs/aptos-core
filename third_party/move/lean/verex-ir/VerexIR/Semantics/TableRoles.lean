-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Semantics.TableOperations
import VerexIR.Validation.TableRoles

/-!
# The runtime semantics of table roles

A function bound to an executable role of an intrinsic map whose owner is a
table (`tableRole?`) is run by the runtime, not by its body: the role's
meaning over native storage (`tableRoleCall`), as the verifier reads it
through the role's contract (`designs/intrinsic-maps.md`, "Tables in the
runtime semantics"). The owner's physical value is its handle, with a cached
length where the owner declares one; the entries live at the slot of the
owner's closed type and the handle (`tableSlot`), the slot the denotation's
boundary names (`tableSlot?`).

A table's closed type is the owner's type of the role's parameter, or of
its result for `new`, read at the frame's instantiation and canonicalized to
the first identifier of the type table denoting it. A role at an open type
has no run.
-/

namespace VerexIR.SemanticOperations

open Validation

/-- The executable map role a function plays for a table owner with a handle
of its namespace, with the owner's layout (`Validation.tableRoleOf?`). -/
def tableRole? (unit : ValidatedUnit) (handle : FunctionHandle) : Option (String × TableLayout) :=
  tableRoleOf? unit handle.namespaceId handle.functionId

/-- Whether a type mentions no type parameter. -/
private def closedType (unit : ValidatedUnit) : Nat → TypeId → Bool
  | 0, _ => false
  | fuel + 1, typeId => match unit.tables.types[typeId.index]? with
    | some (.typeParameter _) | none => false
    | some (.vector element _) | some (.typeDomain element) => closedType unit fuel element
    | some (.reference reference) => closedType unit fuel reference.referent
    | some (.tuple elements) => elements.all (closedType unit fuel)
    | some (.function arguments result _) =>
        arguments.all (closedType unit fuel) && closedType unit fuel result
    | some (.nominal _ arguments) => arguments.all fun
        | .typeArg typeUse => closedType unit fuel typeUse.typeId
        | _ => true
    | some _ => true

/-- The first identifier of the type table denoting a closed type, the one
the denotation's boundary names a table's slot by (`tableSlot?`). -/
def canonicalTypeId? (unit : ValidatedUnit) (typeId : TypeId) : Option TypeId := do
  guard (closedType unit (unit.tables.types.size + 1) typeId)
  let type ← unit.tables.types[typeId.index]?
  let index ← unit.tables.types.findIdx? fun candidate => candidate.eraseLocs == type.eraseLocs
  some ⟨index⟩

/-- The handle a table is allocated at: the first of `table-0`, `table-1`,
… the allocation history lacks. It reads the history alone, so that runs
from storages differing in tables they do not reach allocate alike. -/
def freshTableHandle (state : RuntimeState) : String :=
  go 0 (state.tables.allocated.size + 1)
where
  go (candidate : Nat) : Nat → String
    | 0 => s!"table-{candidate}"
    | fuel + 1 =>
        let handle := s!"table-{candidate}"
        if state.tables.allocated.contains handle then go (candidate + 1) fuel else handle

/-- The abort codes of the table natives (`aptos-move/framework/aptos-stdlib`):
a key already present, a key absent, and a table not empty. -/
def tableAlreadyExists : Int := 100
def tableNotFound : Int := 101
def tableNotEmpty : Int := 102

/-- A map argument: its loan, where it is a mutable reference, and the
owner's value. -/
private def mapArgument : RuntimeValue → Option Nat × RuntimeValue
  | .borrow loan current => (some loan, current)
  | value => (none, value)

/-- A table owner's handle: its first field. -/
private def ownerHandle? : RuntimeValue → Option String
  | .nominal _ _ fields => match fields[0]? with
    | some (RuntimeValue.address handle) => some handle
    | _ => none
  | _ => none

/-- The owner's value with its cached length changed, where it caches one. -/
private def withLength (layout : TableLayout) (owner : RuntimeValue) (delta : Int) : RuntimeValue :=
  if layout.length then
    match owner with
    | .nominal source variant fields => match fields[1]? with
      | some (RuntimeValue.integer length) =>
          .nominal source variant (fields.set! 1 (.integer (length + delta)))
      | _ => owner
    | _ => owner
  else owner

/-- The owner's value at a handle with no entries. -/
private def emptyOwner (layout : TableLayout) (handle : String) : RuntimeValue :=
  .nominal ⟨layout.namespaceId, layout.structId⟩ none
    (#[.address handle] ++ if layout.length then #[.integer 0] else #[])

/-- Replace an entry's value in place. -/
def updateTableEntry? (state : RuntimeState) (slot : GlobalKey) (key value : RuntimeValue) :
    Option RuntimeState := do
  let .vector entries ← state.tables.contents.lookup slot | none
  let index ← tableEntryIndex? (.vector entries) key
  some (state.writeLoanValue (.table slot) (.vector (entries.set! index (.tuple #[key, value]))))

/-- The run of a table role at a frame: the role's meaning over native
storage. A mutable map argument is exported at exit as a body exports its
mutable parameters, with its cached length changed where the role changes
the size. A role the runtime does not provide, or at an open owner type,
has no run. -/
def tableRoleCall (unit : ValidatedUnit) (handle : FunctionHandle) (role : String)
    (layout : TableLayout) (typeInstantiation : Array (TypeId × TypeId)) (state : RuntimeState)
    (arguments : Array RuntimeValue) : Option (RuntimeState × Outcome) := do
  let ns ← unit.namespaces[handle.namespaceId.index]?
  let declaration ← ns.functions[handle.functionId.index]?
  guard (arguments.size == declaration.signature.parameters.size)
  let ownerTypeId? (typeId : TypeId) : Option TypeId :=
    let typeId := match unit.tables.types[typeId.index]? with
      | some (.reference reference) => reference.referent
      | _ => typeId
    canonicalTypeId? unit (instantiatedTypeId typeInstantiation typeId)
  -- The map argument of the role, its loan, its handle, and its slot.
  let map : Option (Option Nat × RuntimeValue × String × GlobalKey) := do
    let parameter ← declaration.signature.parameters[0]?
    let argument ← arguments[0]?
    let (loan, owner) := mapArgument argument
    let tableHandle ← ownerHandle? owner
    let typeId ← ownerTypeId? parameter.typeUse.typeId
    some (loan, owner, tableHandle, tableSlot handle.namespaceId typeId tableHandle)
  let abort (state : RuntimeState) (code : Int) : Option (RuntimeState × Outcome) :=
    some (state, .threw .abort #[.integer code])
  -- Exit with a mutable map argument exported.
  let exit (state : RuntimeState) (loan : Option Nat) (owner : RuntimeValue)
      (results : Array RuntimeValue) : Option (RuntimeState × Outcome) :=
    let state := match loan with
      | some loan => { state with pending := state.pending.push (loan, owner) }
      | none => state
    some (state, .returned results)
  let contents (slot : GlobalKey) : Option (Array RuntimeValue) := do
    let .vector entries ← state.tables.contents.lookup slot | none
    some entries
  match role with
  | "map_new" =>
      let result ← declaration.signature.results[0]?
      let typeId ← ownerTypeId? result.typeId
      let tableHandle := freshTableHandle state
      let state ← allocateTableAt? state handle.namespaceId typeId tableHandle
      some (state, .returned #[emptyOwner layout tableHandle])
  | "map_destroy_empty" =>
      let (_, _, _, slot) ← map
      let entries ← contents slot
      if entries.isEmpty then
        let state ← retireEmptyTable? state slot
        some (state, .returned #[])
      else abort state tableNotEmpty
  | "map_len" =>
      let (loan, owner, _, slot) ← map
      let entries ← contents slot
      exit state loan owner #[.integer entries.size]
  | "map_is_empty" =>
      let (loan, owner, _, slot) ← map
      let entries ← contents slot
      exit state loan owner #[.bool entries.isEmpty]
  | "map_has_key" =>
      let (loan, owner, _, slot) ← map
      let entries ← contents slot
      let key ← arguments[1]?
      exit state loan owner #[.bool (tableEntryIndex? (.vector entries) key).isSome]
  | "map_borrow" =>
      let (loan, owner, _, slot) ← map
      let key ← arguments[1]?
      match readTableEntry? state slot key with
      | some value => exit state loan owner #[value]
      | none => abort state tableNotFound
  | "map_borrow_with_default" =>
      let (loan, owner, _, slot) ← map
      let key ← arguments[1]?
      let default ← arguments[2]?
      exit state loan owner #[(readTableEntry? state slot key).getD default]
  | "map_borrow_mut" =>
      let (loan, owner, _, slot) ← map
      let key ← arguments[1]?
      match borrowTableEntry? state slot key with
      | some (state, reference) => exit state loan owner #[reference]
      | none => abort state tableNotFound
  | "map_borrow_mut_with_default" =>
      let (loan, owner, _, slot) ← map
      let key ← arguments[1]?
      let default ← arguments[2]?
      let (state, owner) ← match readTableEntry? state slot key with
        | some _ => some (state, owner)
        | none => do
            let state ← addTableEntry? state slot key default
            some (state, withLength layout owner 1)
      let (state, reference) ← borrowTableEntry? state slot key
      exit state loan owner #[reference]
  | "map_add_no_override" =>
      let (loan, owner, _, slot) ← map
      let key ← arguments[1]?
      let value ← arguments[2]?
      match addTableEntry? state slot key value with
      | some state => exit state loan (withLength layout owner 1) #[]
      | none =>
          if (readTableEntry? state slot key).isSome then abort state tableAlreadyExists
          else none
  | "map_add_override_if_exists" =>
      let (loan, owner, _, slot) ← map
      let key ← arguments[1]?
      let value ← arguments[2]?
      match updateTableEntry? state slot key value with
      | some state => exit state loan owner #[]
      | none =>
          let state ← addTableEntry? state slot key value
          exit state loan (withLength layout owner 1) #[]
  | "map_del_must_exist" =>
      let (loan, owner, _, slot) ← map
      let key ← arguments[1]?
      match removeTableEntry? state slot key with
      | some (state, value) => exit state loan (withLength layout owner (-1)) #[value]
      | none =>
          if (contents slot).isSome then abort state tableNotFound else none
  | _ => none

end VerexIR.SemanticOperations
