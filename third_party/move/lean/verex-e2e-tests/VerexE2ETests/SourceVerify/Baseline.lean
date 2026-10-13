-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexMove.SourceVerify
import VerexRust.SourceVerify
import VerexIR.TestInfra

/-!
# Verifying Move and Rust sources

Every `.move` file and every `.rs` file of this directory is verified as
`verex-move verify` and `verex-rust verify` do it: rendered as VerexSL
(a Rust file with the `.spec.lean` items beside it) and elaborated, a Move
file with its proof file. A Move file's rendering is written where its proof
file's header finds it, `.verex/` beside it. The messages, in the
coordinates of the Move, Rust, specification, or proof file, are the
baseline `<name>.exp`; a source that verifies has none. A proof file is then
elaborated on its own, as the editor elaborates it, its header loading the
rendering just written: its messages are the baseline `<name>.proof.exp`.
The framework's Move standard library is verified in place as a package, its
modules linking the ones they use; its baseline records what does not verify
yet.
-/

namespace VerexE2ETests.SourceVerify

open VerexIR.TestInfra

private def featureDir : System.FilePath :=
  "VerexE2ETests/SourceVerify"

/-- Move packages verified whole in place, each named by its baseline in this
directory: the framework's standard library. -/
private def packages : Array (String × System.FilePath) :=
  #[("MoveStdlib", "../../../../aptos-move/framework/move-stdlib")]

/-- A package's own files: its manifest, and the Move sources and proof files
under `sources`, without its build output and tests. -/
private def packageInput (package path : System.FilePath) : Bool :=
  let relative := path.toString.drop (package.toString.length + 1)
  relative == "Move.toml" || relative == "Move.lock" ||
    relative.startsWith "sources/" &&
      (relative.endsWith ".move" || relative.endsWith ".proof.lean")

/-- The reports of one source, with the temporary rendering's directory
removed so the baseline is stable. -/
private def reportsOf (environment : Lean.Environment) (directory source : System.FilePath) :
    IO String := do
  let reports ← if source.extension == some "move" || (← source.isDir) then
      VerexMove.SourceVerify.verifySource environment source
        (← VerexMove.SourceVerify.renderingOf source)
    else
      VerexIR.Rust.SourceVerify.verifyFile environment source none
        (directory / s!"{source.fileName.getD "source"}.lean")
  let text := "\n".intercalate (reports.map (·.render)).toList
  pure <| text.replace s!"{directory}/" ""

/-- The messages of a proof file elaborated on its own, as the editor
elaborates it, but verifying every item, as the command line does. -/
private def editorReportsOf (environment : Lean.Environment) (proofFile : System.FilePath) :
    IO String := do
  let inputCtx := Lean.Parser.mkInputContext (← IO.FS.readFile proofFile) proofFile.toString
  let (_, parserState, messages) ← Lean.Parser.parseHeader inputCtx
  let options := ({} : Lean.Options).setBool `verex.skip_verify false
  let state ← Lean.Elab.IO.processCommands inputCtx parserState
    (Lean.Elab.Command.mkState environment messages options)
  let reports ← state.commandState.messages.toList.filterMapM fun message => do
    if message.isSilent then return none
    return some ({ file := message.fileName, line := message.pos.line
                   column := message.pos.column + 1, severity := message.severity
                   text := ← message.data.toString } : VerexSL.SourceVerify.Report).render
  pure ("\n".intercalate reports)

/-- A proof file whose Move file changed since its rendering warns at its
header. -/
private def testStaleRendering (environment : Lean.Environment) (directory : System.FilePath) :
    IO Unit := do
  let staged := directory / "stale"
  IO.FS.createDirAll staged
  for name in ["proofs.move", "proofs.proof.lean"] do
    IO.FS.writeFile (staged / name) (← IO.FS.readFile (featureDir / name))
  let source := staged / "proofs.move"
  discard <| VerexMove.SourceVerify.verifySource environment source
    (← VerexMove.SourceVerify.renderingOf source)
  IO.FS.writeFile source ((← IO.FS.readFile source) ++ "\n// changed\n")
  let reports ← editorReportsOf environment (staged / "proofs.proof.lean")
  unless (reports.splitOn "is older than `").length > 1 do
    throw <| IO.userError s!"a stale rendering does not warn:\n{reports}"

def testBaselines : IO Unit := do
  let sources := (← Baseline.sourceFiles featureDir ".move") ++
    (← Baseline.sourceFiles featureDir ".rs")
  if sources.isEmpty then
    throw <| IO.userError s!"no sources found under {featureDir}"
  let environment ← VerexMove.SourceVerify.importEnvironment
  IO.FS.withTempDir fun directory => do
    for source in sources do
      Baseline.checkOutput (source.withExtension "exp") (← reportsOf environment directory source)
      let proofFile := VerexMove.Proofs.proofFile source
      if source.extension == some "move" && (← proofFile.pathExists) then
        Baseline.checkOutput (source.withExtension "proof.exp")
          (← editorReportsOf environment proofFile)
    testStaleRendering environment directory
    for (name, package) in packages do
      let reports ← Baseline.withStagedDirectory package (directory / name)
        (packageInput package) fun staged => reportsOf environment directory staged
      Baseline.checkOutput (featureDir / s!"{name}.exp") reports

end VerexE2ETests.SourceVerify
