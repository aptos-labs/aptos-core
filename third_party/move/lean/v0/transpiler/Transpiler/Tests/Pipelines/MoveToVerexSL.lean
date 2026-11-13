-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Lean.Elab.Command
import Transpiler
import VerexSL.Print
import Transpiler.Tests.Programs.VerexSL.MoveScalar

/-!
# Move compiler to VerexSL round-trip unit test

The checked-in Move source enters through compiler-v2 XAST and the shared
Move VIR validator. Its canonical VerexSL source is compared byte-for-byte
with a freshly elaborated fixture. Both validated units are then prepared and
executed under the Move semantic profile.
-/

namespace Transpiler.Tests.Pipelines.MoveToVerexSL

open Lean
open Lean.Elab
open Lean.Elab.Command

private def programsDir : System.FilePath := "Transpiler/Tests/Programs/VerexSL"

private def namespaceName : Name :=
  #["0x42", "move_scalar"].foldl Name.str .anonymous

private def render (environment : Environment)
    (unit : VerexIR.Validation.ValidatedUnit) : IO String :=
  match VerexSL.Print.render environment unit with
  | .ok source => pure source
  | .error error => throw <| IO.userError (toString error)

private def runInvert (unit : VerexIR.Validation.ValidatedUnit)
    (value : Bool) : IO VerexIR.RuntimeValue := do
  let executable ← match VerexIR.Validation.prepareExecution
      #[VerexIR.Move.semantics] unit with
    | .ok executable => pure executable
    | .error diagnostics =>
        throw <| IO.userError s!"Move VIR is not executable: {repr diagnostics}"
  let handle : VerexIR.FunctionHandle := {
    namespaceId := ⟨0⟩, functionId := ⟨0⟩ }
  let (_, outcome) ← match VerexIR.Interpreter.run executable 32 handle #[.bool value] with
    | .ok result => pure result
    | .error error => throw <| IO.userError s!"Move VIR execution failed: {repr error}"
  match outcome.value with
  | .returned #[result] => pure result
  | other => throw <| IO.userError s!"Move VIR returned an unexpected outcome: {repr other}"

private def checkRoundtrip (environment : Environment) : IO Unit := do
  let package ← Transpiler.Cli.exportMoveFiles [programsDir / "move_scalar.move"]
  let original ← match Transpiler.VIR.Backend.fromXast package with
    | .ok unit => pure unit
    | .error message => throw <| IO.userError message
  let some originalNamespace := original.namespaces[0]?
    | throw <| IO.userError "Move-to-VIR produced no namespace"
  unless originalNamespace.constants.size == 1 && originalNamespace.structs.size == 1 &&
      originalNamespace.functions.size == 1 && originalNamespace.specFunctions.size == 1 do
    throw <| IO.userError "Move-to-VIR lost a declaration or compiler-derived spec function"

  let expected ← IO.FS.readFile (programsDir / "MoveScalar.lean")
  let rendered ← render environment original
  unless rendered == expected do
    throw <| IO.userError s!"Move fixture produced a different VerexSL fixture:\n{rendered}"

  let some fresh := VerexSL.registeredUnit? environment namespaceName
    | throw <| IO.userError "the Move VerexSL fixture was not freshly elaborated"
  let reprinted ← render environment fresh
  unless reprinted == expected do
    throw <| IO.userError s!"fresh elaboration changed the Move VerexSL fixture:\n{reprinted}"

  for input in #[false, true] do
    let originalResult ← runInvert original input
    let freshResult ← runInvert fresh input
    let expectedResult := VerexIR.RuntimeValue.bool (!input)
    unless originalResult == expectedResult && freshResult == expectedResult do
      throw <| IO.userError s!"Move executable behavior changed for input {input}"

syntax (name := checkMoveToVerexSL) "#check_move_to_verex_sl" : command

@[command_elab checkMoveToVerexSL]
private def elaborateCheck : CommandElab := fun _ => do
  liftIO <| checkRoundtrip (← getEnv)

#check_move_to_verex_sl

end Transpiler.Tests.Pipelines.MoveToVerexSL
