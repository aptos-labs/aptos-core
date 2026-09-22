-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Lean
import LeanerE2ETests.MonoVM.Link
import LeanerE2ETests.MonoVM.Directives
import LeanerE2ETests.MonoVM.Normalize
import LeanerIR.TestInfra
import LeanerMove
import LeanerLang.Print
import LeanerMove.Frontend

/-!
# MonoVM differential baselines

Executable Move fixtures under `LeanerE2ETests/MonoDifferential/` compare
three interpretations of the same call: the linked MonoVM adapter, the
original validated Move LIR, and the LIR obtained after the canonical
LeanerLang round trip. Neither side is the baseline — the first comparison
checks the Move-to-LIR semantic boundary, the second checks that printing
and re-elaboration preserve it. Exhaustion on either side is inconclusive;
abort locations, messages, and diagnostics are carried for triage but never
compared.

Each fixture records what all three engines produced in its side-by-side
`.exp` file, and that baseline is the only pass/fail mechanism: agreement
alone would not show *what* the engines agreed on, and a divergence shows up
as a diff of the outcome that changed. A divergence additionally records an
explicit `ERROR:` line naming the comparison that failed, so it reads as a
finding rather than as one more changed value. Agreement records nothing
extra — the equal outcome lines already say it. Update with `UB=1 lake test`
and review the diff.
-/

namespace LeanerE2ETests.MonoDifferential

open LeanerIR LeanerIR.TestInfra LeanerE2ETests.MonoVM

private def featureDir : System.FilePath :=
  "LeanerE2ETests/MonoDifferential"

/-- One resolved execute step: the fixture step plus its typed arguments in
both runtime representations. -/
private structure Prepared where
  step : ExecuteStep
  runtimeArgs : Array RuntimeValue
  requestArgs : Array Value

/-- Renders a parameter's bit width for the adapter value schema. -/
private def intWidth : LeanerIR.IntWidth → Nat
  | .bits width => width
  | .pointer => 64
  | .unbounded => 64

/-- Parses one directive literal against one parameter type. -/
private def parseArg (ty : LeanerIR.Ty) (literal : String) :
    Except String (RuntimeValue × Value) :=
  match ty with
  | .integer width signed =>
      match literal.toInt? with
      | some value =>
          if signed || value ≥ 0 then
            .ok (.integer value, .integer (intWidth width) signed (toString value))
          else
            .error s!"literal {literal} is negative but the parameter is unsigned"
      | none => .error s!"literal {literal} is not an integer"
  | .bool =>
      match literal with
      | "true" => .ok (.bool true, .bool true)
      | "false" => .ok (.bool false, .bool false)
      | other => .error s!"literal {other} is not a boolean"
  | other => .error s!"parameter type {repr other} is not yet a fixture argument type"

/-- Resolves the callee's typed arguments from its declaration signature. -/
private def prepareStep (ns : Validation.ValidatedNamespace)
    (declaration : LeanerIR.FunctionDecl Validation.FunctionBody) (step : ExecuteStep) :
    Except String Prepared := do
  let mut runtimeArgs := #[]
  let mut requestArgs := #[]
  unless step.args.length == declaration.signature.parameters.size do
    throw s!"{step.function} takes {declaration.signature.parameters.size} \
      arguments but the directive supplies {step.args.length}"
  for (parameter, literal) in declaration.signature.parameters.zip step.args.toArray do
    let some ty := ns.tables.types[parameter.typeUse.typeId.index]?
      | throw s!"parameter type {parameter.name} did not resolve"
    let (runtimeArg, requestArg) ← parseArg ty literal
    runtimeArgs := runtimeArgs.push runtimeArg
    requestArgs := requestArgs.push requestArg
  return ⟨step, runtimeArgs, requestArgs⟩

/-- Finds a namespace by its `address::module` path segments. -/
private def namespaceByPath (unit : Validation.ValidatedUnit)
    (address module : String) : Option Validation.ValidatedNamespace :=
  unit.namespaces.find? fun ns =>
    (unit.tables.namespaces[ns.identity.index]?.map (·.segments)) == some #[address, module]

/-- Resolves one step's callee in a validated unit, by name. Every unit is
resolved on its own: the LeanerLang round trip renders declarations in
canonical order, so a function's position is not stable across the round trip
and only its name identifies it. -/
private def resolveStep (unit : Validation.ValidatedUnit) (step : ExecuteStep) :
    Except String
      (Validation.ValidatedNamespace × LeanerIR.FunctionDecl Validation.FunctionBody × Nat) := do
  let some ns := namespaceByPath unit step.address step.module
    | throw s!"namespace {step.address}::{step.module} not found"
  let some (declaration, index) := ns.functions.zipIdx.find? fun (declaration, _) =>
      (ns.tables.names[declaration.name.index]?.map (·.name)) == some step.function
    | throw s!"function {step.function} not found in {step.module}"
  return (ns, declaration, index)

/-- Runs one prepared step through a prepared executable unit. -/
private def runLean (executable : Validation.ExecutableUnit)
    (namespaceId : NamespaceId) (functionId : Nat) (prepared : Prepared) :
    Except LocatedInterpreterError (RuntimeState × LocatedOutcome) :=
  Interpreter.run executable prepared.step.fuel
    { namespaceId, functionId := ⟨functionId⟩ } prepared.runtimeArgs

/-- Renders one step's recorded result: what each engine produced, and an
explicit `ERROR:` line for each comparison that diverges. Agreement is not
recorded — it is what the equal outcome lines already say — but a divergence
is called out by name so it cannot be read past in a baseline diff. -/
private def renderStep (index : Nat) (step : ExecuteStep)
    (mono original roundTrip : NormOutcome) : String :=
  let call := s!"{step.address}::{step.module}::{step.function}\
    ({String.intercalate ", " step.args})"
  let errors :=
    (if divergent mono original then #["  ERROR: mono and LIR diverge"] else #[]) ++
    (if divergent original roundTrip then #["  ERROR: LIR and round trip diverge"] else #[])
  String.intercalate "\n" <|
    [ s!"step {index}: {call}",
      s!"  mono:       {mono.render}",
      s!"  LIR:        {original.render}",
      s!"  round trip: {roundTrip.render}" ] ++ errors.toList

/-- Runs one differential fixture. The adapter's build identity is logged once
per suite run for triage; it never enters a baseline. -/
private def testFixture (environment : Lean.Environment)
    (identityLogged : IO.Ref Bool) (fixturePath : System.FilePath) : IO Unit := do
  let text ← IO.FS.readFile fixturePath
  let fixture ← match parseFixture text with
    | .ok fixture => pure fixture
    | .error error => throw <| IO.userError s!"{fixturePath}: {error}"
  -- The original validated LIR from the Move source. The fixture's directive
  -- comments are stripped before the frontend sees it, so the publish source
  -- is staged to a build-directory file: staging it beside the fixture would
  -- both write into the source tree and leave a file the next run discovers
  -- as a fixture whenever the export fails.
  let stagingDirectory : System.FilePath := ".lake" / "build" / "monodiff"
  IO.FS.createDirAll stagingDirectory
  let stagedSource := stagingDirectory / (fixturePath.fileName.getD "fixture.move")
  IO.FS.writeFile stagedSource fixture.source
  let package ← try
      LeanerMove.Frontend.Cli.exportMoveFiles [stagedSource]
    finally
      IO.FS.removeFile stagedSource
  let unit ← match LeanerMove.Frontend.LIR.Backend.fromXast package with
    | .ok unit => pure unit
    | .error message => throw <| IO.userError s!"{fixturePath}: {message}"
  -- Preparing either Lean unit can fail on a frontend or semantic gap the
  -- fixture is reaching. That is an outcome of this engine, not a reason to
  -- abandon the fixture: MonoVM still runs the call, and recording what the
  -- Lean side could not do keeps the gap visible in the baseline instead of
  -- taking the whole suite down. Only the distinct diagnostic codes are
  -- recorded — their count and wording churn as the frontend moves, while
  -- the kind of gap is the reviewable fact.
  let preparationError (diagnostics : Array Validation.Diagnostic) : String :=
    let codes := diagnostics.foldl (init := #[]) fun codes diagnostic =>
      if codes.contains diagnostic.code then codes else codes.push diagnostic.code
    s!"the Move LIR is not executable: {String.intercalate ", " codes.qsort.toList}"
  let executable? : Except String Validation.ExecutableUnit :=
    (Validation.prepareExecution #[LeanerIR.Move.semantics] unit).mapError preparationError
  let roundTripExecutable? : Except String Validation.ExecutableUnit :=
    match LeanerLang.Print.reimportUnit environment unit with
    | .error error => .error s!"the round trip did not re-import: {toString error}"
    | .ok roundTrip =>
        (Validation.prepareExecution #[LeanerIR.Move.semantics] roundTrip).mapError
          fun diagnostics => s!"round trip: {preparationError diagnostics}"
  -- Resolve every step in the original unit; the round-trip resolution
  -- reuses the original indices when that leg is skipped.
  let mut prepared := #[]
  for step in fixture.steps do
    let (ns, declaration, index) ← match resolveStep unit step with
      | .ok resolved => pure resolved
      | .error error => throw <| IO.userError s!"{fixturePath}: {error}"
    let p ← match prepareStep ns declaration step with
      | .ok p => pure p
      | .error error => throw <| IO.userError s!"{fixturePath}: {error}"
    prepared := prepared.push (ns.identity, index, p)
  -- One linked adapter request per distinct limits bucket: calls sharing
  -- limits batch into one compile-and-run request, and a directive that
  -- changes gas or heap splits its steps into their own request.
  let mut outcomes : Array (Option MonoVM.Outcome) :=
    (List.replicate prepared.size none).toArray
  let mut buckets : Array Limits := #[]
  for (entry, _) in prepared.zipIdx do
    let (_, _, p) := entry
    let limits := Limits.mk p.step.gas none
    unless buckets.contains limits do
      buckets := buckets.push limits
  for limits in buckets do
    let members := prepared.zipIdx.filterMap fun (entry, stepIndex) =>
      let (_, _, p) := entry
      if Limits.mk p.step.gas none == limits then some (entry, stepIndex) else none
    let request : Request :=
      Request.mk payloadVersion
        (CompileSpec.mk #[SourceFile.mk "fixture.move" fixture.source] #[] 2)
        limits
        (members.map fun (member, _) =>
          let (_, _, p) := member
          Call.mk s!"{p.step.address}::{p.step.module}::{p.step.function}" #[]
            p.requestArgs)
    let response ← MonoVM.run request
    unless response.outcomes.size == members.size do
      throw <| IO.userError
        s!"{fixturePath}: the adapter returned {response.outcomes.size} outcomes \
          for {members.size} calls"
    unless (← identityLogged.get) do
      identityLogged.set true
      IO.println
        s!"mono-move adapter identity: abi {response.identity.abi}, \
          profile {response.identity.profile}, {response.identity.rustc}"
    for ((_, stepIndex), outcome) in members.zip response.outcomes do
      outcomes := outcomes.set! stepIndex outcome
  -- Compare each step three ways and record what every engine produced.
  let mut recorded : Array String := #[]
  for ((entry, outcome?), stepIndex) in prepared.zip outcomes |>.zipIdx do
    let some outcome := outcome?
      | throw <| IO.userError s!"{fixturePath}: step {stepIndex} was never run"
    let (identity, index, p) := entry
    let mono := normalizeOutcome outcome
    let original := match executable? with
      | .error reason => .error reason
      | .ok executable => normalizeLeanOutcome (runLean executable identity index p)
    let roundTrip ← match roundTripExecutable? with
      | .error reason => pure (.error reason)
      | .ok roundTripExecutable =>
          match resolveStep roundTripExecutable.unit p.step with
          | .ok (roundTripNs, _, roundTripIndex) =>
              pure <| normalizeLeanOutcome
                (runLean roundTripExecutable roundTripNs.identity roundTripIndex p)
          | .error error => throw <| IO.userError s!"{fixturePath} round trip: {error}"
    recorded := recorded.push (renderStep stepIndex p.step mono original roundTrip)
  Baseline.check (fixturePath.withExtension "exp")
    (String.intercalate "\n\n" recorded.toList ++ "\n")

/-- Runs every differential fixture. -/
def testBaselines (environment : Lean.Environment) : IO Unit := do
  let fixtures ← Baseline.sourceFiles featureDir ".move"
  if fixtures.isEmpty then
    throw <| IO.userError s!"no MonoVM differential fixtures found under {featureDir}"
  let identityLogged ← IO.mkRef false
  for fixture in fixtures do
    testFixture environment identityLogged fixture

end LeanerE2ETests.MonoDifferential
