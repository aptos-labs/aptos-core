-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerMove.Frontend
import LeanerLang.Print
import LeanerIR.TestInfra

/-!
# Move compiler to LeanerLang end-to-end baselines

Every direct `.move` file and every immediate child package with a `Move.toml`
is discovered automatically. Each owned module crosses compiler-v2 XAST and
validated Move LIR before its canonical LeanerLang source, or its current
concrete error, is compared with the side-by-side `.exp.lean`.
-/

namespace LeanerE2ETests.MoveToLeanerLang

open LeanerIR.TestInfra

private def featureDir : System.FilePath :=
  "LeanerE2ETests/MoveToLeanerLang"

private def formatPrintError (remap : String → IO System.FilePath)
    (error : LeanerLang.Print.Error) : IO String := do
  let message ← match error.location with
    | some (source, range) =>
        Baseline.locatedMessage (← remap source.name) range.startByte error.message
    | none => pure error.message
  pure <| Baseline.errorOutput message

private def render (environment : Lean.Environment)
    (unit : LeanerIR.Validation.ValidatedUnit)
    (context : LeanerIR.Validation.ValidatedUnit)
    (identity : LeanerIR.NamespaceId)
    (remap : String → IO System.FilePath) : IO String :=
  match LeanerLang.Print.renderNamespace environment unit identity with
  | .ok output =>
      match LeanerLang.Print.formatSourceInContext environment output context identity 80
          "<generated Move LeanerLang>" with
      | .ok reprinted => if reprinted == output then pure output else
          -- A non-canonical fixed point is recorded as the module's concrete
          -- outcome: re-elaboration inserts specification coercions the
          -- transpiled unit does not carry, and printer canonicalization of
          -- those coercions is tracked as deferred work.
          pure <| Baseline.errorOutput s!"generated Move LeanerLang is not a canonical fixed point:\n\
            {Baseline.formatDiff "<first print>" output reprinted}"
      | .error error =>
          throw <| IO.userError s!"generated Move LeanerLang source does not re-import: {error}\n\
            --- generated source ---\n{output}"
  | .error error => formatPrintError remap error

private def captureErrors (source : System.FilePath) (action : IO String) : IO String :=
  try action catch error =>
    pure <| Baseline.errorOutput
      (← Baseline.locatedMessage source 0 (toString error))

/-- Package staging keeps every Move source, specification files included:
`type_info.move`-style inline specifications reference declarations from
their sibling `.spec.move`. Staging exists to exclude the side-by-side
`.exp.lean` baselines, which would otherwise be collected as package
sources. -/
private def packageInput (path : System.FilePath) : Bool :=
  path.fileName.any fun name =>
    name == "Move.toml" || name == "Move.lock" || name.endsWith ".move"


/-- Stage every discovered package as `.{name}.dep-input` next to itself for
the duration of `action`. Checked-in manifests reference these staged copies
for local dependencies, so a dependency package whose sources carry
side-by-side baselines still compiles. -/
private partial def withDependencyInputs (packages : List System.FilePath)
    (action : IO α) : IO α :=
  match packages with
  | [] => action
  | directory :: rest => do
      match directory.parent with
      | none => withDependencyInputs rest action
      | some parent =>
          let name := directory.fileName.getD "package"
          Baseline.withStagedDirectory directory (parent / s!".{name}.dep-input")
            packageInput fun _ => withDependencyInputs rest action

private def normalizePackageError (directory staged : System.FilePath)
    (message : String) : IO String := do
  let directoryPath ← IO.FS.realPath directory
  let normalized := message
    |>.replace staged.toString directory.toString
    |>.replace (← IO.FS.realPath staged).toString directoryPath.toString
  let parts := normalized.splitOn "` failed:\n"
  pure <| match parts with
    | _command :: detail :: details =>
        String.intercalate "` failed:\n" (detail :: details)
    | _ => normalized

private def originalSourcePath (directory staged : System.FilePath)
    (sourceName : String) : IO System.FilePath := do
  let directoryPath ← IO.FS.realPath directory
  let stagedPath ← IO.FS.realPath staged
  let directoryName := directory.fileName.getD "package"
  let stagedName := staged.fileName.getD ""
  let intrinsicsName := s!".{directoryName}.intrinsics-input"
  let suffix := (sourceName : System.FilePath).components.dropWhile fun component =>
    component != stagedName && component != intrinsicsName
  if let _stagingRoot :: relative := suffix then
    return relative.foldl (fun path component =>
      System.FilePath.join path (System.FilePath.mk component)) directory
  let remapped := sourceName
    |>.replace stagedPath.toString directoryPath.toString
    |>.replace staged.toString directory.toString
    |>.replace stagedName directoryName
    |>.replace intrinsicsName directoryName
  pure remapped

private def testSource (environment : Lean.Environment) (source : System.FilePath) : IO Unit := do
  let actual ← captureErrors source do
    let package ← LeanerMove.Frontend.Cli.exportMoveFiles [source]
    match LeanerMove.Frontend.LIR.Backend.fromXast package with
    | .ok unit => render environment unit unit ⟨0⟩ (fun name => pure name)
    | .error message => do
        let startByte := package.modules[0]?.map (·.loc.start) |>.getD 0
        let message ← Baseline.locatedMessage source startByte message
        pure <| Baseline.errorOutput message
  let expectation := Baseline.expectationPath source "lean"
  Baseline.check expectation actual

private def testPackage (environment : Lean.Environment) (directory : System.FilePath) : IO Nat := do
  let some parent := directory.parent
    | throw <| IO.userError s!"Move package {directory} has no parent directory"
  let name := directory.fileName.getD "package"
  let nonce ← IO.monoNanosNow
  let staged := parent / s!".{name}.{nonce}.baseline-input"
  Baseline.withStagedDirectory directory staged packageInput fun staged => do
    let package? ← try
      -- Export the dependency modules too: a package's own modules construct,
      -- select, and match values of the types they import, which needs those
      -- declarations as dependency interfaces.
      pure <| some (← LeanerMove.Frontend.Cli.exportPackage staged (includeDeps := true))
    catch error =>
      let marker := directory / "Move.toml"
      let message ← normalizePackageError directory staged (toString error)
      let message ← Baseline.locatedMessage marker 0 message
      Baseline.check (Baseline.expectationPath marker "lean")
        (Baseline.errorOutput message)
      pure none
    let some package := package?
      | return 1
    let mut count := 0
    for sourceModule in package.modules do
      let some sourceName := sourceModule.sources[sourceModule.loc.file]?
        | throw <| IO.userError s!"{sourceModule.name}: source path is missing"
      let source ← originalSourcePath directory staged sourceName
      -- Keep every owned module available as authoritative declaration
      -- context, but put the source under test first so its namespace id is
      -- stable and its diagnostics remain attributable to this baseline.
      let otherModules := package.modules.filter fun candidate =>
        candidate.ref != sourceModule.ref
      let contextual := { package with modules := sourceModule :: otherModules }
      let actual ← match LeanerMove.Frontend.LIR.Backend.fromXast contextual with
        | .ok context => try
            render environment context context ⟨0⟩ (originalSourcePath directory staged)
          catch error =>
            -- A render or re-import boundary is this module's concrete
            -- outcome; recording it keeps the rest of the package's
            -- baselines running. The staged path carries a per-run nonce,
            -- so it is normalized back to the package directory.
            let message := (toString error)
              |>.replace staged.toString directory.toString
            pure <| Baseline.errorOutput
              (← Baseline.locatedMessage source 0 message)
        | .error message => do
            let message ← Baseline.locatedMessage source sourceModule.loc.start message
            pure <| Baseline.errorOutput message
      Baseline.check (Baseline.expectationPath source "lean") actual
      count := count + 1
    pure count

def testBaselines (environment : Lean.Environment) : IO Unit := do
  let sources ← Baseline.sourceFiles featureDir ".move"
  let packages ← Baseline.markedDirectories featureDir "Move.toml"
  unless !sources.isEmpty || !packages.isEmpty do
    throw <| IO.userError s!"no Move-to-LeanerLang baselines found under {featureDir}"
  for source in sources do
    testSource environment source
  withDependencyInputs packages.toList do
    for directory in packages do
      unless (← testPackage environment directory) > 0 do
        throw <| IO.userError s!"Move package {directory} contains no owned modules"

end LeanerE2ETests.MoveToLeanerLang
