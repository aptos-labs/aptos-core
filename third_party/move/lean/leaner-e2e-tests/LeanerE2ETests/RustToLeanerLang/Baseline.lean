-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerRust.Driver
import LeanerLang.Print
import LeanerIR.TestInfra

/-!
# Rust source to LeanerLang end-to-end baselines

Every `.rs` file in this feature directory is discovered automatically and
crosses the pinned rustc MIR exporter and validated Rust LIR before its
canonical LeanerLang source, or its current concrete error, is compared with
the side-by-side `.exp.lean`.
-/

namespace LeanerE2ETests.RustToLeanerLang

open LeanerIR.TestInfra

private def featureDir : System.FilePath :=
  "LeanerE2ETests/RustToLeanerLang"

private def loadOriginal (artifact source : System.FilePath) :
    IO LeanerIR.Validation.ValidatedUnit := do
  let result ← LeanerIR.Rust.Benchmark.measure "e2e.rust_import" source.toString <|
    LeanerIR.Rust.Driver.importRustFile {
      source
      output := artifact }
  pure result.unit

private def render (environment : Lean.Environment) (source : System.FilePath)
    (unit : LeanerIR.Validation.ValidatedUnit) : IO String := do
  let printed ← LeanerIR.Rust.Benchmark.measureExcept "e2e.leaner_print" source.toString fun _ =>
    LeanerLang.Print.render environment unit
  match printed with
  | .ok output =>
      let formatted ← LeanerIR.Rust.Benchmark.measureExcept "e2e.leaner_reimport"
          source.toString fun _ =>
        LeanerLang.Print.formatSource environment output 80 "<generated Rust LeanerLang>"
      match formatted with
      | .ok reprinted => if reprinted == output then pure output else
          throw <| IO.userError s!"generated Rust LeanerLang is not a canonical fixed point:\n\
            {Baseline.formatDiff "<first print>" output reprinted}"
      | .error error =>
          throw <| IO.userError s!"generated Rust LeanerLang source does not re-import: {error}\n\
            --- generated source ---\n{output}"
  | .error error => do
      let message ← match error.location with
        | some (_, range) => Baseline.locatedMessage source range.startByte error.message
        | none => pure error.message
      pure <| Baseline.errorOutput message

private def normalizeError (source : System.FilePath) (message : String) : IO String := do
  let canonical ← IO.FS.realPath source
  pure <| message.replace canonical.toString (source.fileName.getD source.toString)

private def captureErrors (source : System.FilePath) (action : IO String) : IO String :=
  try action catch error =>
    let message ← normalizeError source (toString error)
    pure <| Baseline.errorOutput (← Baseline.locatedMessage source 0 message)

private def importOrError (source artifact : System.FilePath) :
    IO (Except String LeanerIR.Validation.ValidatedUnit) := do
  try
    return .ok (← loadOriginal artifact source)
  catch error =>
    if ← artifact.pathExists then
      throw <| IO.userError s!"failed Rust import left a partial artifact at {artifact}"
    let message ← normalizeError source (toString error)
    return .error <| Baseline.errorOutput (← Baseline.locatedMessage source 0 message)

private def testSource (environment : Lean.Environment)
    (temporary source : System.FilePath) : IO Unit := do
  LeanerIR.Rust.Benchmark.measure "e2e.case_total" source.toString do
    let fileName := source.fileName.getD "source.rs"
    let artifact := temporary / s!"{fileName}.raw.json"
    let actual ← match ← importOrError source artifact with
      | .error output => pure output
      | .ok unit => captureErrors source (render environment source unit)
    let expectation := Baseline.expectationPath source "lean"
    Baseline.check expectation actual

def testBaselines (environment : Lean.Environment) : IO Unit :=
  LeanerIR.Rust.Benchmark.measure "e2e.suite_total" "RustToLeanerLang" <|
  IO.FS.withTempDir fun temporary => do
    let sources ← Baseline.sourceFiles featureDir ".rs"
    unless !sources.isEmpty do
      throw <| IO.userError s!"no Rust-to-LeanerLang baselines found under {featureDir}"
    for source in sources do
      testSource environment temporary source

end LeanerE2ETests.RustToLeanerLang
