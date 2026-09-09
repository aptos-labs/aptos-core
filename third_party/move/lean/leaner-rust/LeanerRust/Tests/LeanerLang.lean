-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Lean.Elab.Command
import LeanerRust.Profile
import LeanerLang.Print
import LeanerRust.Tests.Programs.LeanerLang.Basic
import LeanerRust.Tests.Programs.LeanerLang.BitwiseNot
import LeanerRust.Tests.Programs.LeanerLang.Comments
import LeanerRust.Tests.Programs.LeanerLang.IntegerWidths
import LeanerRust.Tests.Programs.LeanerLang.Scalar
import LeanerRust.Tests.Programs.LeanerLang.GenericIdentity
import LeanerRust.Tests.Programs.LeanerLang.Method
import LeanerRust.Tests.Programs.LeanerLang.StringLength
import LeanerRust.Tests.Programs.LeanerLang.StringSlice

/-!
# Rust exporter to LeanerLang round-trip unit tests

Each case starts with a checked-in artifact produced from the corresponding
Rust fixture by `leaner-rust-export`.  The test validates that artifact,
renders canonical LeanerLang, compares it byte-for-byte with an imported
fixture, and renders the freshly elaborated fixture back to the same text.
-/

namespace LeanerIR.Rust.Tests.LeanerLang

open Lean
open Lean.Elab
open Lean.Elab.Command

private structure Case where
  fixture : String
  source : String
  namespaceName : Name

private structure Run where
  functionName : String
  arguments : Array LeanerIR.RuntimeValue
  expected : LeanerIR.Outcome

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

private def leanerSources : System.FilePath :=
  "LeanerRust/Tests/Programs/LeanerLang"

private def loadOriginal (case : Case) : IO LeanerIR.Validation.ValidatedUnit := do
  let json ← IO.FS.readFile (exporterBaselines / case.fixture)
  match LeanerIR.Rust.decodeAndValidate json with
    | .ok unit => pure unit
    | .error diagnostics =>
        throw <| IO.userError s!"Rust exporter fixture {case.fixture} does not validate: \
          {repr diagnostics}"

private def renderOriginal (environment : Environment) (case : Case)
    (unit : LeanerIR.Validation.ValidatedUnit) : IO String := do
  match LeanerLang.Print.render environment unit with
  | .ok source => pure source
  | .error error =>
      throw <| IO.userError s!"Rust exporter fixture {case.fixture} does not render: {error}"

private def runChecked (case : Case) (label : String)
    (unit : LeanerIR.Validation.ValidatedUnit) (run : Run) : IO Unit := do
  let executable ← match LeanerIR.Validation.prepareExecution
      #[LeanerIR.Rust.semantics] unit with
    | .ok executable => pure executable
    | .error diagnostics => throw <| IO.userError s!"{label} {case.fixture} is not executable: \
        {repr diagnostics}"
  let some ns := unit.namespaces[0]?
    | throw <| IO.userError s!"{label} {case.fixture} has no namespace"
  let some (_, functionIndex) := ns.functions.zipIdx.find? fun (declaration, _) =>
      unit.tables.names[declaration.name.index]?.any (·.name == run.functionName)
    | throw <| IO.userError s!"{label} {case.fixture} has no function `{run.functionName}`"
  let handle : LeanerIR.FunctionHandle := {
    namespaceId := ⟨0⟩, functionId := ⟨functionIndex⟩ }
  let (_, outcome) ← match LeanerIR.Interpreter.run executable 64 handle run.arguments with
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
    let expected ← IO.FS.readFile (leanerSources / case.source)
    let rendered ← renderOriginal environment case original
    unless rendered == expected do
      throw <| IO.userError s!"Rust fixture {case.fixture} produced a different LeanerLang fixture:\n\
        {rendered}"
    let renderedAgain ← renderOriginal environment case original
    unless renderedAgain == rendered do
      throw <| IO.userError s!"Rust fixture {case.fixture} did not render deterministically"
    let some imported := LeanerLang.registeredUnit? environment case.namespaceName
      | throw <| IO.userError s!"LeanerLang fixture {case.source} was not freshly elaborated"
    let reprinted ← match LeanerLang.Print.render environment imported with
      | .ok source => pure source
      | .error error =>
          throw <| IO.userError s!"freshly elaborated fixture {case.source} does not render: {error}"
    unless reprinted == expected do
      throw <| IO.userError s!"fresh elaboration changed LeanerLang fixture {case.source}:\n\
        {reprinted}"
    for run in runs case do
      runChecked case "exported Rust LIR" original run
      runChecked case "fresh LeanerLang LIR" imported run

syntax (name := checkRustToLeanerLang) "#check_rust_to_leaner_lang" : command

@[command_elab checkRustToLeanerLang]
private def elaborateCheck : CommandElab := fun _ => do
  liftIO <| checkCases (← getEnv)

#check_rust_to_leaner_lang

end LeanerIR.Rust.Tests.LeanerLang
