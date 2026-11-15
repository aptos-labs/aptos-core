-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL.Print
import VerexSL.Options
import VerexSL.Perf
import VerexSL.Quote
import VerexIR.Validation.Link
import VerexIR.Validation.RequiredTypes

/-!
# Modules that use modules

A namespace command naming registered modules is lowered against their
interfaces, then links their registered namespaces into its unit in place of
those interfaces, so its unit holds every namespace it reaches, with bodies.
A linked namespace is relocated, not validated again. Further registered
modules a caller names, those whose invariants concern the unit's memory,
are linked with the modules they use.
-/

namespace VerexSL

open Lean Elab Command VerexIR VerexIR.Validation

private def pathName (segments : Array String) : Name :=
  segments.foldl (fun name segment => Name.str name segment) .anonymous

/-- The namespace registered as the module at a path. -/
private def registeredModule? (environment : Environment) (path : Array String) :
    Option RegisteredNamespace := do
  let entry ← registeredNamespace? environment (pathName path)
  guard <| entry.object.path == path
  pure entry

/-- The registered modules a command reaches: every registered module a
prefix of one of its paths names, and the modules those use. -/
private def moduleClosure (environment : Environment) (own : Array String)
    (paths : Array (Array String)) : Except String (Array (Array String × RegisteredNamespace)) := do
  let mut worklist : Array (Array String) := #[]
  for path in paths do
    let path := canonicalMovePath environment path
    for length in [1:path.size + 1] do
      let prefix_ := path.extract 0 length
      if prefix_ != own && (registeredModule? environment prefix_).isSome &&
          !worklist.contains prefix_ then
        worklist := worklist.push prefix_
  let mut found : Array (Array String × RegisteredNamespace) := #[]
  while let some path := worklist.back? do
    worklist := worklist.pop
    if path == own || found.any (·.1 == path) then continue
    let some entry := registeredModule? environment path
      | throw s!"`{"::".intercalate path.toList}` is used but not declared"
    found := found.push (path, entry)
    -- A namespace no module declares, such as a builtin, resolves as it did.
    worklist := worklist ++ (entry.object.references.map (·.segments)).filter fun path =>
      (registeredModule? environment path).isSome
  return found

/-- Lower and validate a namespace command, linking it and the registered
modules at the paths `related` names for the unit the command's own uses give:
the unit, the namespace as relocatable namespace, which is the unit's first,
the namespace's path, and the path's syntax. -/
def compileNamespaceWith (related : Environment → ValidatedUnit → Array (Array String))
    (stx : Syntax) (compile : Syntax → CompilationUnit → CommandElabM ValidatedUnit := compileAt) :
    CommandElabM (ValidatedUnit × RelocatableNamespace × Array String × Syntax) := do
  let (stx, unit) ← namespaceUnitOf stx
  let pathSyntax := (pathChildren stx)[0]!
  stageLog s!"{pathSyntax.reprint.getD ""}: unit"
  let some sourceNs := unit.namespaces[0]? | throwErrorAt stx "a Verex namespace requires a path"
  let env ← getEnv
  let modules ← match moduleClosure env sourceNs.path (referencedPaths stx) with
    | .ok modules => pure modules
    | .error message => throwErrorAt pathSyntax message
  let mut interfaces := #[]
  for (path, entry) in modules do
    match entry.interface with
    | .ok interface => interfaces := interfaces.push interface
    | .error message =>
        throwErrorAt pathSyntax s!"the interface of `{"::".intercalate path.toList}` does not \
          elaborate: {message}"
  stageLog s!"interfaces ({interfaces.size})"
  let validated ← compile stx { unit with namespaces := unit.namespaces ++ interfaces }
  stageLog "validated"
  let object ← match extract validated ⟨0⟩ with
    | .ok object => pure object
    | .error message => throwErrorAt pathSyntax message
  let linked ← match link validated (#[object] ++ modules.map (·.2.object)) with
    | .ok linked => pure linked
    | .error message => throwErrorAt pathSyntax message
  let extraPaths := (related env linked).filter fun path => !modules.any (·.1 == path)
  let linked ← if extraPaths.isEmpty then pure linked else do
    let extras ← match moduleClosure env sourceNs.path extraPaths with
      | .ok extras => pure (extras.filter fun (path, _) => !modules.any (·.1 == path))
      | .error message => throwErrorAt pathSyntax message
    match link validated (#[object] ++ (modules ++ extras).map (·.2.object)) with
    | .ok linked => pure linked
    | .error message => throwErrorAt pathSyntax message
  stageLog "linked"
  -- The unit holds every type its generic frames require.
  return (linked.internRequiredTypes, object, sourceNs.path, pathSyntax)

/-- Elaborate a namespace command and register its unit
(`compileNamespaceWith`), with the definition of its relocatable namespace
(`Quote.ensureObjectDefinition`), which the units that hold it quote. -/
def elaborateNamespaceWith (related : Environment → ValidatedUnit → Array (Array String))
    (stx : Syntax) : CommandElabM Unit := Perf.withPhase .lowering do
  let (unit, object, path, pathSyntax) ← compileNamespaceWith related stx
  let env ← getEnv
  let interface := (Print.interfaceNamespace env unit ⟨0⟩).mapError (·.message)
  let unitPaths := unit.namespaces.filterMap fun ns =>
    (unit.tables.namespaces[ns.unitIdentity.index]?).map (·.segments)
  let entry := { object, interface, unitPaths, closureTemplates := closureTemplates object.body }
  match registerUnit env (pathName path) unit entry with
  | .ok environment => setEnv environment
  | .error message => throwErrorAt pathSyntax message
  liftCoreM <| Quote.ensureObjectDefinition object

@[command_elab verexNamespaceCommand, command_elab verexMoveModuleCommand,
  command_elab verexRustNamespaceCommand]
def elaborateNamespace : CommandElab := elaborateNamespaceWith fun _ _ => #[]

end VerexSL
