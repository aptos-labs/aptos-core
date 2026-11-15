-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Lean.Elab.Command
import VerexRust.Profile
import VerexSL.Print
import VerexRust.Tests.Programs.VerexSL.Basic
import VerexRust.Tests.Programs.VerexSL.BitwiseNot
import VerexRust.Tests.Programs.VerexSL.Comments
import VerexRust.Tests.Programs.VerexSL.IntegerWidths
import VerexRust.Tests.Programs.VerexSL.Scalar
import VerexRust.Tests.Programs.VerexSL.GenericIdentity
import VerexRust.Tests.Programs.VerexSL.Method
import VerexRust.Tests.Programs.VerexSL.StringLength
import VerexRust.Tests.Programs.VerexSL.StringSlice

/-!
# Rust exporter to VerexSL round-trip unit tests

Each case starts with a checked-in artifact produced from the corresponding
Rust fixture by `verex-rust-export`.  The test validates that artifact,
renders canonical VerexSL, compares it byte-for-byte with an imported
fixture, and renders the freshly elaborated fixture back to the same text.
-/

namespace VerexIR.Rust.Tests.VerexSL

open Lean
open Lean.Elab
open Lean.Elab.Command

private structure Case where
  fixture : String
  source : String
  namespaceName : Name

private structure Run where
  functionName : String
  arguments : Array VerexIR.RuntimeValue
  expected : VerexIR.Outcome

private def runs (case : Case) : Array Run :=
  match case.fixture with
  | "basic.exp.json" => #[{
      functionName := "answer"
      arguments := #[]
      expected := .returned #[.bool true] }]
  | "scalar.exp.json" => #[{
      functionName := "scalar"
      arguments := #[.integer 170, .integer 255]
      expected := (.returned #[.integer 85]) }]
  | "bitwise_not.exp.json" => #[
      { functionName := "invert_signed"
        arguments := #[.integer 15]
        expected := .returned #[.integer (-16)] },
      { functionName := "invert_unsigned"
        arguments := #[.integer 15]
        expected := .returned #[.integer 240] }]
  | "integer_widths.exp.json" => #[{
      functionName := "widths"
      arguments := #[.integer 1, .integer 2, .integer 3, .integer 4, .integer 5,
        .integer (-1), .integer (-2), .integer (-3), .integer (-4), .integer (-5)]
      expected := (.returned #[.integer (-7)]) }]
  | "generic_identity.exp.json" => #[
      { functionName := "choose"
        arguments := #[.integer 7]
        expected := .returned #[.integer 7] },
      { functionName := "round_trip"
        arguments := #[.integer 11]
        expected := .returned #[.integer 11] }]
  | "method.exp.json" => #[
      { functionName := "call_method"
        arguments := #[.integer 40, .integer 2]
        expected := (.returned #[.integer 42]) },
      { functionName := "call_method"
        arguments := #[.integer 4294967295, .integer 1]
        expected := (.threw .panic #[]) }]
  | _ => #[]

private def cases : Array Case := #[
  { fixture := "basic.exp.json", source := "Basic.lean", namespaceName := `basic },
  { fixture := "scalar.exp.json", source := "Scalar.lean", namespaceName := `scalar },
  { fixture := "bitwise_not.exp.json", source := "BitwiseNot.lean",
    namespaceName := `bitwise_not },
  { fixture := "comments.exp.json", source := "Comments.lean", namespaceName := `comments },
  { fixture := "integer_widths.exp.json", source := "IntegerWidths.lean",
    namespaceName := `integer_widths },
  { fixture := "generic_identity.exp.json", source := "GenericIdentity.lean",
    namespaceName := `generic_identity },
  { fixture := "method.exp.json", source := "Method.lean", namespaceName := `method },
  { fixture := "string_length.exp.json", source := "StringLength.lean",
    namespaceName := `string_length },
  { fixture := "string_slice.exp.json", source := "StringSlice.lean",
    namespaceName := `string_slice }
]

private def exporterBaselines : System.FilePath :=
  "rust-exporter/tests/raw-unit"

private def verexSources : System.FilePath :=
  "VerexRust/Tests/Programs/VerexSL"

private def loadOriginal (case : Case) : IO VerexIR.Validation.ValidatedUnit := do
  let json ← IO.FS.readFile (exporterBaselines / case.fixture)
  match VerexIR.Rust.decodeAndValidate json with
    | .ok unit => pure unit
    | .error diagnostics =>
        throw <| IO.userError s!"Rust exporter fixture {case.fixture} does not validate: \
          {repr diagnostics}"

private def renderOriginal (environment : Environment) (case : Case)
    (unit : VerexIR.Validation.ValidatedUnit) : IO String := do
  match VerexSL.Print.render environment unit with
  | .ok source => pure source
  | .error error =>
      throw <| IO.userError s!"Rust exporter fixture {case.fixture} does not render: {error}"

private def runChecked (case : Case) (label : String)
    (unit : VerexIR.Validation.ValidatedUnit) (run : Run) : IO Unit := do
  let executable ← match VerexIR.Validation.prepareExecution
      #[VerexIR.Rust.semantics] unit with
    | .ok executable => pure executable
    | .error diagnostics => throw <| IO.userError s!"{label} {case.fixture} is not executable: \
        {repr diagnostics}"
  let some ns := unit.namespaces[0]?
    | throw <| IO.userError s!"{label} {case.fixture} has no namespace"
  let some (_, functionIndex) := ns.functions.zipIdx.find? fun (declaration, _) =>
      unit.tables.names[declaration.name.index]?.any (·.name == run.functionName)
    | throw <| IO.userError s!"{label} {case.fixture} has no function `{run.functionName}`"
  let handle : VerexIR.FunctionHandle := {
    namespaceId := ⟨0⟩, functionId := ⟨functionIndex⟩ }
  let (_, outcome) ← match VerexIR.Interpreter.run executable 64 handle run.arguments with
    | .ok result => pure result
    | .error error => throw <| IO.userError s!"{label} {case.fixture} execution failed: \
        {repr error}"
  let outcomeMatches := match run.expected, outcome.value with
    | .threw .panic _, .threw .panic _ => true
    | expected, actual => expected == actual
  unless outcomeMatches do
    throw <| IO.userError s!"{label} {case.fixture} returned {repr outcome.value}, expected \
      {repr run.expected}"

private def checkCases (environment : Environment) : IO Unit := do
  for case in cases do
    let original ← loadOriginal case
    let expected ← IO.FS.readFile (verexSources / case.source)
    let rendered ← renderOriginal environment case original
    unless rendered == expected do
      throw <| IO.userError s!"Rust fixture {case.fixture} produced a different VerexSL fixture:\n\
        {rendered}"
    let renderedAgain ← renderOriginal environment case original
    unless renderedAgain == rendered do
      throw <| IO.userError s!"Rust fixture {case.fixture} did not render deterministically"
    let some imported := VerexSL.registeredUnit? environment case.namespaceName
      | throw <| IO.userError s!"VerexSL fixture {case.source} was not freshly elaborated"
    let reprinted ← match VerexSL.Print.render environment imported with
      | .ok source => pure source
      | .error error =>
          throw <| IO.userError s!"freshly elaborated fixture {case.source} does not render: {error}"
    unless reprinted == expected do
      throw <| IO.userError s!"fresh elaboration changed VerexSL fixture {case.source}:\n\
        {reprinted}"
    for run in runs case do
      runChecked case "exported Rust VIR" original run
      runChecked case "fresh VerexSL VIR" imported run

syntax (name := checkRustToVerexSL) "#check_rust_to_verex_sl" : command

@[command_elab checkRustToVerexSL]
private def elaborateCheck : CommandElab := fun _ => do
  liftIO <| checkCases (← getEnv)

#check_rust_to_verex_sl

end VerexIR.Rust.Tests.VerexSL
