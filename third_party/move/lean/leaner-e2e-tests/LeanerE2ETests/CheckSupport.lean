-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang
import LeanerMove.Profile
import LeanerIR.Interpreter.Interpreter

namespace LeanerE2ETests.CheckSupport

open Lean Elab Command LeanerIR

/-- Interpreter assertions retained from the v0 language corpus. These are
execution checks, independent of the separately generated contract proofs. -/
structure RunCase where
  function : String
  arguments : Array RuntimeValue
  expected : Outcome
  initial : RuntimeState := {}

/-- An execution assertion that also pins the final global/loan state. -/
structure StateRunCase where
  function : String
  arguments : Array RuntimeValue
  expected : Outcome
  initial : RuntimeState := {}
  final : RuntimeState := {}

/-- Build a one-resource state from registered module metadata, without
hard-coding the nominal's type-table index in individual source ports. -/
def singleResourceState (namespace_ : Name) (resource : String) (address : String)
    (fields : Array RuntimeValue) (nextLoan : Nat := 0) : CommandElabM RuntimeState := do
  let some unit := LeanerLang.registeredUnit? (← getEnv) namespace_
    | throwError "missing source module {namespace_}"
  let some ns := unit.namespaces[0]?
    | throwError "source module {namespace_} has no namespace"
  let some structIndex := ns.structs.findIdx? fun declaration =>
      (ns.tables.names[declaration.name.index]?.map (·.name) == some resource)
    | throwError "missing source resource {resource}"
  let declaration := ns.structs[structIndex]!
  let some typeIndex := ns.tables.types.findIdx? fun
      | .nominal name arguments => name == declaration.name && arguments.isEmpty
      | _ => false
    | throwError "missing concrete source resource type {resource}"
  let handle : StructHandle := { namespaceId := ⟨0⟩, structId := structIndex }
  let key : GlobalKey := ⟨⟨0⟩, ⟨typeIndex⟩, .address address⟩
  return { globals := ({} : GlobalMap).insert key (.nominal handle none fields), nextLoan }

def assertRuns (namespace_ : Name) (cases : Array RunCase) : CommandElabM Unit := do
  let some unit := LeanerLang.registeredUnit? (← getEnv) namespace_
    | throwError "missing source module {namespace_}"
  let executable ← match Validation.prepareExecution #[LeanerIR.Move.semantics] unit with
    | .ok executable => pure executable
    | .error diagnostics => throwError "source execution preparation failed: {repr diagnostics}"
  for test in cases do
    let some (namespaceIndex, _, index, _) := LeanerLang.Contract.findFunction? unit test.function
      | throwError "missing source function {test.function}"
    let handle : FunctionHandle := { namespaceId := ⟨namespaceIndex⟩, functionId := ⟨index⟩ }
    match Interpreter.run executable 256 handle test.arguments test.initial with
    | .error failure => throwError "source execution {test.function} failed: {repr failure}"
    | .ok (_, outcome) =>
        unless outcome.value == test.expected do
          throwError "source execution {test.function}: expected {repr test.expected}, got {repr outcome.value}"

def assertRunsState (namespace_ : Name) (cases : Array StateRunCase) : CommandElabM Unit := do
  let some unit := LeanerLang.registeredUnit? (← getEnv) namespace_
    | throwError "missing source module {namespace_}"
  let executable ← match Validation.prepareExecution #[LeanerIR.Move.semantics] unit with
    | .ok executable => pure executable
    | .error diagnostics => throwError "source execution preparation failed: {repr diagnostics}"
  for test in cases do
    let some (namespaceIndex, _, index, _) := LeanerLang.Contract.findFunction? unit test.function
      | throwError "missing source function {test.function}"
    let handle : FunctionHandle := { namespaceId := ⟨namespaceIndex⟩, functionId := ⟨index⟩ }
    match Interpreter.run executable 256 handle test.arguments test.initial with
    | .error failure => throwError "source execution {test.function} failed: {repr failure}"
    | .ok (state, outcome) =>
        unless outcome.value == test.expected do
          throwError "source execution {test.function}: expected {repr test.expected}, got {repr outcome.value}"
        unless state == test.final do
          throwError "source execution {test.function}: expected state {repr test.final}, got {repr state}"

end LeanerE2ETests.CheckSupport
