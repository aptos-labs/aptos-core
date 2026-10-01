-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerMove.SourceVerify
import LeanerRust.SourceVerify
import LeanerIR.TestInfra

/-!
# Verifying Move and Rust sources

Every `.move` file and every `.rs` file of this directory is verified as
`leaner-move verify` and `leaner-rust verify` do it: rendered as LeanerLang
(a Rust file with the `.spec.lean` items beside it) and elaborated. The
messages, in the coordinates of the Move, Rust, or specification file, are
the baseline `<name>.exp`; a source that verifies has none. The Move
standard library the corpus renders is verified as a package, its modules
linking the ones they use; its baseline records what does not verify yet.
-/

namespace LeanerE2ETests.SourceVerify

open LeanerIR.TestInfra

private def featureDir : System.FilePath :=
  "LeanerE2ETests/SourceVerify"

/-- Move packages verified whole, each with its baseline in this directory. -/
private def packages : Array System.FilePath :=
  #["LeanerE2ETests/MoveToLeanerLang/MoveStdlib"]

/-- A package's own files: its manifest, Move sources, and proof files,
without the baselines another suite keeps beside them. -/
private def packageInput (path : System.FilePath) : Bool :=
  path.fileName.any fun name =>
    name == "Move.toml" || name == "Move.lock" || name.endsWith ".move" ||
      name.endsWith ".proof.lean"

/-- The reports of one source, with the temporary rendering's directory
removed so the baseline is stable. -/
private def reportsOf (environment : Lean.Environment) (directory source : System.FilePath) :
    IO String := do
  let output := directory / s!"{source.fileName.getD "source"}.lean"
  let reports ← if source.extension == some "move" || (← source.isDir) then
      LeanerMove.SourceVerify.verifySource environment source output
    else LeanerIR.Rust.SourceVerify.verifyFile environment source none output
  let text := "\n".intercalate (reports.map (·.render)).toList
  pure <| text.replace s!"{directory}/" ""

def testBaselines : IO Unit := do
  let sources := (← Baseline.sourceFiles featureDir ".move") ++
    (← Baseline.sourceFiles featureDir ".rs")
  if sources.isEmpty then
    throw <| IO.userError s!"no sources found under {featureDir}"
  let environment ← LeanerLang.SourceVerify.importLeanerLang
  IO.FS.withTempDir fun directory => do
    for source in sources do
      Baseline.checkOutput (source.withExtension "exp") (← reportsOf environment directory source)
    for package in packages do
      let name := package.fileName.getD "package"
      let reports ← Baseline.withStagedDirectory package (directory / name) packageInput
        fun staged => reportsOf environment directory staged
      Baseline.checkOutput (featureDir / s!"{name}.exp") reports

end LeanerE2ETests.SourceVerify
