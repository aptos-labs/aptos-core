-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang.Print
import LeanerLang.Options
import LeanerLang.Perf
import LeanerIR.Validation.Link
import LeanerIR.Validation.RequiredTypes

/-!
# Modules that use modules

A namespace command naming registered modules is lowered against their
interfaces, then links their registered namespaces into its unit in place of
those interfaces, so its unit holds every namespace it reaches, with bodies.
A linked namespace is relocated, not validated again. Further registered
modules a caller names, those whose invariants concern the unit's memory,
are linked with the modules they use.
-/

namespace LeanerLang

open Lean Elab Command LeanerIR LeanerIR.Validation

private def pathName (segments : Array String) : Name :=
  segments.foldl (fun name segment => Name.str name segment) .anonymous

/-- The registered unit declaring the module at a path, and that module. -/
private def registeredModule? (environment : Environment) (path : Array String) :
    Option (ValidatedUnit × NamespaceId) := do
  let unit ← registeredUnit? environment (pathName path)
  let ns ← unit.namespaces[0]?
  guard <| (unit.tables.namespaces[ns.identity.index]?.map (·.segments)) == some path
  pure (unit, ns.identity)

/-- The registered modules a command reaches: every registered module a
prefix of one of its paths names, and the modules those use. -/
private def moduleClosure (environment : Environment) (own : Array String)
    (paths : Array (Array String)) : Except String (Array (Array String × RelocatableNamespace)) := do
  let mut worklist : Array (Array String) := #[]
  for path in paths do
    let path := canonicalMovePath environment path
    for length in [1:path.size + 1] do
      let prefix_ := path.extract 0 length
      if prefix_ != own && (registeredModule? environment prefix_).isSome &&
          !worklist.contains prefix_ then
        worklist := worklist.push prefix_
  let mut found : Array (Array String × RelocatableNamespace) := #[]
  while let some path := worklist.back? do
    worklist := worklist.pop
    if path == own || found.any (·.1 == path) then continue
    let some (unit, identity) := registeredModule? environment path
      | throw s!"`{"::".intercalate path.toList}` is used but not declared"
    let object ← extract unit identity
    found := found.push (path, object)
    -- A namespace no module declares, such as a builtin, resolves as it did.
    worklist := worklist ++ (object.references.map (·.segments)).filter fun path =>
      (registeredModule? environment path).isSome
  return found

/-- Elaborate a namespace command and register its unit, linking the
registered modules at the paths `related` names for the unit the command's
own uses give. -/
def elaborateNamespaceWith (related : Environment → ValidatedUnit → Array (Array String))
    (stx : Syntax) : CommandElabM Unit := Perf.withPhase .lowering do
  let (stx, unit) ← namespaceUnitOf stx
  let pathSyntax := (pathChildren stx)[0]!
  stageLog s!"{pathSyntax.reprint.getD ""}: unit"
  let some sourceNs := unit.namespaces[0]? | throwErrorAt stx "a Leaner namespace requires a path"
  let env ← getEnv
  let modules ← match moduleClosure env sourceNs.path (referencedPaths stx) with
    | .ok modules => pure modules
    | .error message => throwErrorAt pathSyntax message
  let mut interfaces := #[]
  for (path, _) in modules do
    let some (owner, identity) := registeredModule? env path | continue
    match Print.interfaceNamespace env owner identity with
    | .ok interface => interfaces := interfaces.push interface
    | .error error =>
        throwErrorAt pathSyntax s!"the interface of `{"::".intercalate path.toList}` does not \
          elaborate: {error.message}"
  stageLog s!"interfaces ({interfaces.size})"
  let validated ← compileAt stx { unit with namespaces := unit.namespaces ++ interfaces }
  stageLog "validated"
  let linked ← if modules.isEmpty then pure validated else
    match link validated (modules.map (·.2)) with
    | .ok linked => pure linked
    | .error message => throwErrorAt pathSyntax message
  let extraPaths := (related env linked).filter fun path => !modules.any (·.1 == path)
  let linked ← if extraPaths.isEmpty then pure linked else do
    let extras ← match moduleClosure env sourceNs.path extraPaths with
      | .ok extras => pure (extras.filter fun (path, _) => !modules.any (·.1 == path))
      | .error message => throwErrorAt pathSyntax message
    match link validated ((modules ++ extras).map (·.2)) with
    | .ok linked => pure linked
    | .error message => throwErrorAt pathSyntax message
  stageLog "linked"
  -- The unit holds every type its generic frames require.
  match registerUnit (← getEnv) (pathName sourceNs.path) linked.internRequiredTypes with
  | .ok environment => setEnv environment
  | .error message => throwErrorAt pathSyntax message

@[command_elab leanerNamespaceCommand, command_elab leanerMoveModuleCommand,
  command_elab leanerRustNamespaceCommand]
def elaborateNamespace : CommandElab := elaborateNamespaceWith fun _ _ => #[]

end LeanerLang
