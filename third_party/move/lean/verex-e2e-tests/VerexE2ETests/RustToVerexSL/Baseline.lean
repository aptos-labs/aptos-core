-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexRust.Driver
import VerexSL.Print
import VerexIR.TestInfra

/-!
# Rust source to VerexSL end-to-end baselines

Every `.rs` file in this feature directory is discovered automatically and
crosses the pinned rustc MIR exporter and validated Rust VIR before its
canonical VerexSL source, or its current concrete error, is compared with
the side-by-side `.exp.lean`.
-/

namespace VerexE2ETests.RustToVerexSL

open VerexIR.TestInfra

private def featureDir : System.FilePath :=
  "VerexE2ETests/RustToVerexSL"

private def loadOriginal (artifact source : System.FilePath) :
    IO VerexIR.Validation.ValidatedUnit := do
  let result ← VerexIR.Rust.Benchmark.measure "e2e.rust_import" source.toString <|
    VerexIR.Rust.Driver.importRustFile {
      source
      output := artifact }
  pure result.unit

private def render (environment : Lean.Environment) (source : System.FilePath)
    (unit : VerexIR.Validation.ValidatedUnit) : IO String := do
  let printed ← VerexIR.Rust.Benchmark.measureExcept "e2e.verex_print" source.toString fun _ =>
    VerexSL.Print.render environment unit
  match printed with
  | .ok output =>
      let formatted ← VerexIR.Rust.Benchmark.measureExcept "e2e.verex_reimport"
          source.toString fun _ =>
        VerexSL.Print.formatSource environment output 80 "<generated Rust VerexSL>"
      match formatted with
      | .ok reprinted => if reprinted == output then pure output else
          throw <| IO.userError s!"generated Rust VerexSL is not a canonical fixed point:\n\
            {Baseline.formatDiff "<first print>" output reprinted}"
      | .error error =>
          throw <| IO.userError s!"generated Rust VerexSL source does not re-import: {error}\n\
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
    IO (Except String VerexIR.Validation.ValidatedUnit) := do
  try
    return .ok (← loadOriginal artifact source)
  catch error =>
    if ← artifact.pathExists then
      throw <| IO.userError s!"failed Rust import left a partial artifact at {artifact}"
    let message ← normalizeError source (toString error)
    return .error <| Baseline.errorOutput (← Baseline.locatedMessage source 0 message)

private def testSource (environment : Lean.Environment)
    (temporary source : System.FilePath) : IO Unit := do
  VerexIR.Rust.Benchmark.measure "e2e.case_total" source.toString do
    let fileName := source.fileName.getD "source.rs"
    let artifact := temporary / s!"{fileName}.raw.json"
    let actual ← match ← importOrError source artifact with
      | .error output => pure output
      | .ok unit => captureErrors source (render environment source unit)
    let expectation := Baseline.expectationPath source "lean"
    Baseline.check expectation actual

def testBaselines (environment : Lean.Environment) : IO Unit :=
  VerexIR.Rust.Benchmark.measure "e2e.suite_total" "RustToVerexSL" <|
  IO.FS.withTempDir fun temporary => do
    let sources ← Baseline.sourceFiles featureDir ".rs"
    unless !sources.isEmpty do
      throw <| IO.userError s!"no Rust-to-VerexSL baselines found under {featureDir}"
    for source in sources do
      testSource environment temporary source

end VerexE2ETests.RustToVerexSL
