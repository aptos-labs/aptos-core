-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Lean
import VerexSL.Profile
import VerexIR.Validation.Link
import VerexIR.Validation.RequiredTypes
import VerexIR.Validation.ClosureTemplates

/-!
# The registry of validated namespaces

A file's registered namespace persists as its relocatable namespace, the
validation results included, and its interface. Its linked unit, which holds
every namespace it reaches, is kept only by the file that elaborated it; an
importer that asks for the unit of an imported namespace links it from the
imported namespaces when it first does. A unit a theorem was stated over stays
whole: an importer verifying that namespace again reads the same definition.
-/

namespace VerexSL

open Lean Elab Command
open VerexIR.Validation

/-- What a namespace's `.olean` holds of it, independent of its importers. -/
structure RegisteredNamespace where
  /-- The namespace with its validation results. -/
  object : RelocatableNamespace
  /-- The declarations another namespace is lowered against, or why they do
  not elaborate. -/
  interface : Except String Namespace
  /-- The namespaces of its unit, itself first, in the unit's order. -/
  unitPaths : Array (Array String) := #[]
  /-- The calls and closures of its bodies, in its own identifiers, which
  the closure sites of every unit holding it join (`closureSites`). -/
  closureTemplates : ClosureTemplates := {}

private structure Registered where
  entry : RegisteredNamespace
  /-- The namespace's unit, or why the imported namespaces do not link. -/
  unit : Thunk (Except String ValidatedUnit)

instance : Inhabited Registered :=
  ⟨⟨⟨default, .error "", #[], {}⟩, Thunk.pure (.error "")⟩⟩

private structure RegistryState where
  /-- The namespaces this file registers, the last first. -/
  own : List (Name × Registered) := []
  all : NameMap Registered := {}
  deriving Inhabited

private def pathName (segments : Array String) : Name :=
  segments.foldl (fun name segment => Name.str name segment) .anonymous

/-- The unit of namespaces `paths`, each as `object` finds it, in that order: a
function of the relocatable namespaces, so a file and its importers get the same
unit. -/
def linkRegistered (profiles : Array VerexIR.ProfileConfig) (paths : Array (Array String))
    (object : Array String → Option RelocatableNamespace) : Except String ValidatedUnit := do
  let objects ← paths.mapM fun path => match object path with
    | some found => pure found
    | none => throw s!"the namespace `{"::".intercalate path.toList}` is not registered"
  return (← assemble profiles objects).internRequiredTypes

/-- The unit of an imported namespace; or why it does not link, as where an
`.olean` was made against other namespaces than those imported. -/
private def linkImported (imported : NameMap RegisteredNamespace) (start : RegisteredNamespace) :
    Except String ValidatedUnit :=
  match linkRegistered start.object.profiles start.unitPaths fun path =>
      (imported.find? (pathName path)).map (·.object) with
  | .ok unit => .ok unit
  | .error message =>
      .error s!"the imported namespace `{"::".intercalate start.object.path.toList}` does not \
        link: {message}"

private initialize unitExtension :
    PersistentEnvExtension (Name × RegisteredNamespace) (Name × Registered) RegistryState ←
  registerPersistentEnvExtension {
    mkInitial := pure {}
    addImportedFn := fun modules => do
      let imported := modules.foldl (init := ({} : NameMap RegisteredNamespace)) fun all entries =>
        entries.foldl (init := all) fun all (name, entry) => all.insert name entry
      let all := imported.foldl (init := ({} : NameMap Registered)) fun all name entry =>
        all.insert name { entry, unit := Thunk.mk fun _ => linkImported imported entry }
      pure { all }
    addEntryFn := fun state (name, registered) =>
      { own := (name, registered) :: state.own, all := state.all.insert name registered }
    exportEntriesFn := fun state =>
      (state.own.reverse.map fun (name, registered) => (name, registered.entry)).toArray }

/-- The units a theorem has been stated over, by namespace: the value of the
unit's definition, which an importer verifying the namespace again must read. -/
private initialize definedUnitExtension :
    SimplePersistentEnvExtension (Name × ValidatedUnit) (NameMap ValidatedUnit) ←
  registerSimplePersistentEnvExtension {
    addEntryFn := fun units (name, unit) => units.insert name unit
    addImportedFn := fun modules =>
      modules.foldl (init := {}) fun units entries =>
        entries.foldl (init := units) fun units (name, unit) => units.insert name unit }

/-- Record `unit` as the value of the definition of the unit of namespace `name`. -/
def recordDefinedUnit (environment : Environment) (name : Name) (unit : ValidatedUnit) :
    Environment :=
  definedUnitExtension.addEntry environment (name, unit)

/-- The namespace registered under `name` in this module or an imported one. -/
def registeredNamespace? (environment : Environment) (name : Name) :
    Option RegisteredNamespace :=
  ((unitExtension.getState environment).all.find? name).map (·.entry)

/-- The relocatable namespace registered at a path. -/
def registeredObject? (environment : Environment) (path : Array String) :
    Option RelocatableNamespace :=
  (registeredNamespace? environment (pathName path)).map (·.object)

/-- The closure templates registered with the namespace at a path. -/
def registeredClosureTemplates? (environment : Environment) (path : Array String) :
    Option ClosureTemplates := do
  let entry ← registeredNamespace? environment (pathName path)
  guard (entry.object.path == path)
  pure entry.closureTemplates

/-- The validated unit of a namespace registered in this module or an imported
module, an imported namespace's linked when first asked for; `none` where the
name is not registered, an error where its namespaces do not link. -/
def registeredUnitResult (environment : Environment) (name : Name) :
    Except String (Option ValidatedUnit) :=
  match (definedUnitExtension.getState environment).find? name with
  | some unit => .ok (some unit)
  | none =>
    match (unitExtension.getState environment).all.find? name with
    | some registered => registered.unit.get.map some
    | none => .ok none

/-- `registeredUnitResult` where only a unit that links is of interest. -/
def registeredUnit? (environment : Environment) (name : Name) : Option ValidatedUnit :=
  (registeredUnitResult environment name).toOption.join

/-- The unit registered under `name`, reporting at `ref` a name not registered,
or namespaces that do not link. -/
def registeredUnitAt (ref : Syntax) (name : Name) : CommandElabM ValidatedUnit := do
  match registeredUnitResult (← getEnv) name with
  | .ok (some unit) => pure unit
  | .ok none => throwErrorAt ref s!"unknown Verex namespace `{name}`"
  | .error message => throwErrorAt ref message

/-- Every namespace registered in this module or an imported module. -/
def registeredNamespaces (environment : Environment) : List (Name × RegisteredNamespace) :=
  (unitExtension.getState environment).all.toList.map fun (name, registered) =>
    (name, registered.entry)

/-- Every validated unit registered in this module or an imported module, or why
some do not link. -/
def registeredUnits (environment : Environment) : Except String (List (Name × ValidatedUnit)) :=
  (unitExtension.getState environment).all.toList.mapM fun (name, registered) =>
    registered.unit.get.map (name, ·)

/-- The validated Verex namespaces this module registers, in order. -/
def moduleUnits (environment : Environment) : List (Name × ValidatedUnit) :=
  ((unitExtension.getState environment).own.reverse).filterMap fun (name, registered) =>
    registered.unit.get.toOption.map (name, ·)

/-- The namespaces of the compilation this file belongs to: those it registers
and those registered by the files its header imports. A rendering verifying one
namespace imports the namespaces rendered before it. -/
def compilationNamespaces (environment : Environment) : List Name :=
  let imported := environment.header.imports.toList.flatMap fun import_ =>
    match environment.getModuleIdx? import_.module with
    | some index => (unitExtension.getModuleEntries environment index).toList.map (·.1)
    | none => []
  imported ++ ((unitExtension.getState environment).own.reverse.map (·.1))

/-- Persist a checked source unit. Repeating an identical declaration is
idempotent; changing the meaning of an existing name is rejected. -/
def registerUnit (environment : Environment) (name : Name) (unit : ValidatedUnit)
    (entry : RegisteredNamespace) : Except String Environment :=
  match registeredNamespace? environment name with
  | none => .ok (unitExtension.addEntry environment (name, { entry, unit := Thunk.pure (.ok unit) }))
  | some previous =>
      if previous.object == entry.object then .ok environment
      else .error s!"Verex namespace `{name}` is already registered with a different unit"

end VerexSL
