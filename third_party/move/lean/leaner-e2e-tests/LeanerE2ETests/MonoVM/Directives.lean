-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerE2ETests.MonoVM.Payload

/-!
# MonoVM differential fixture directives

Executable Move fixtures are ordinary Move source whose directives are Move
comments in the grammar the mono-move differential suite already consumes:
`// RUN: publish` starts a source block and
`// RUN: execute <addr>::<module>::<function> [--args <literal>, …]` runs a
call. That suite's parser rejects unknown modifiers, so Lean-side knobs live
in the namespaced `// LEANER: fuel <n>` and `// LEANER: gas <n>` directives,
which apply to every subsequent execute step. A fixture that runs in both
harnesses is the mismatch-triage path made concrete.
-/

namespace LeanerE2ETests.MonoVM

/-- One execute step with its resolved limits. -/
structure ExecuteStep where
  address : String
  module : String
  function : String
  args : List String
  fuel : Nat
  gas : Nat
  deriving Inhabited

/-- One parsed fixture: the concatenated publish sources and its steps. -/
structure Fixture where
  source : String
  steps : Array ExecuteStep
  deriving Inhabited

/-- Default interpreter fuel per call. -/
def defaultFuel : Nat := 100000

/-- Default adapter gas budget per call. -/
def defaultGas : Nat := 10000000000

/-- The `// RUN:` directive prefix. -/
private def runPrefix : String := "// RUN: "

/-- The `// LEANER:` directive prefix. -/
private def leanerPrefix : String := "// LEANER: "

private def parseExecute (rest : String) (fuel gas : Nat) : Except String ExecuteStep := do
  let parts := rest.splitOn " --args "
  let head := (parts.getD 0 "").trimAscii.toString
  let args := match parts.getD 1 "" with
    | "" => []
    | args => (args.splitOn ",")|>.map (fun s => s.trimAscii.toString) |>.filter (fun s => !s.isEmpty)
  let pieces := head.splitOn "::"
  unless pieces.length == 3 do
    throw s!"execute directive needs `0x…::module::function`, got {head}"
  let [address, module, function] := pieces
    | throw "unreachable: length checked above"
  unless address.startsWith "0x" do
    throw s!"execute directive address {address} must be a numeric 0x… literal"
  return { address, module, function, args, fuel, gas }

/-- Parses a fixture file's text into its publish source and execute steps. -/
def parseFixture (text : String) : Except String Fixture :=
  let rec go (lines : List String) (source : List String) (steps : Array ExecuteStep)
      (fuel gas : Nat) : Except String Fixture :=
    match lines with
    | [] =>
        if steps.isEmpty then
          .error "fixture contains no execute steps"
        else
          .ok { source := "\n".intercalate source.reverse, steps }
    | line :: rest =>
        let trimmed := line.trimAscii
        if trimmed.startsWith runPrefix then
          let directive := (trimmed.drop runPrefix.length).toString.trimAscii
          if directive.startsWith "publish" then
            go rest source steps fuel gas
          else if directive.startsWith "execute" then
            match parseExecute (directive.drop "execute ".length).toString fuel gas with
            | .ok step => go rest source (steps.push step) fuel gas
            | .error error => .error error
          else
            .error s!"unknown RUN directive: {directive}"
        else if trimmed.startsWith leanerPrefix then
          let directive := (trimmed.drop leanerPrefix.length).toString.trimAscii
          if directive.startsWith "fuel " then
            match (directive.drop "fuel ".length).toString.trimAscii.toNat? with
            | some fuel => go rest source steps fuel gas
            | none => .error s!"invalid fuel directive: {directive}"
          else if directive.startsWith "gas " then
            match (directive.drop "gas ".length).toString.trimAscii.toNat? with
            | some gas => go rest source steps fuel gas
            | none => .error s!"invalid gas directive: {directive}"
          else
            .error s!"unknown LEANER directive: {directive}"
        else
          go rest (line :: source) steps fuel gas
  go (text.splitOn "\n") ([] : List String) #[] defaultFuel defaultGas

end LeanerE2ETests.MonoVM
